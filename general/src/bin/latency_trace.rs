//! This captures both total and sync timings:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use hdrhistogram::{
    sync::{Recorder, SyncHistogram},
    Histogram,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};
use tracing::{
    callsite::Identifier,
    info, instrument,
    subscriber::{Interest, Subscriber},
    warn, Id, Instrument, Metadata,
};
use tracing_core::span::Attributes;
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer, Registry,
};

/// Globally collected information for a callsite.
#[derive(Debug)]
pub struct CallsiteTiming {
    pub callsite_str: String,
    pub span_name: String,
    pub total_time: SyncHistogram<u64>,
    pub active_time: SyncHistogram<u64>,
}

#[derive(Debug)]
pub struct CallsiteParent {
    knows_parent: bool,
    pub parent: Option<Identifier>,
}

/// Timings by callsite.
type Timings = RwLock<HashMap<Identifier, CallsiteTiming>>;

/// Callsite parents.
/// Separate from [Timings] to avoid locking issues caused by [SyncHistogram].refresh.
type Parents = RwLock<HashMap<Identifier, CallsiteParent>>;

/// Thread-local information collected for a callsite.
struct LocalCallsiteInfo {
    total_time: Recorder<u64>,
    active_time: Recorder<u64>,
    knows_parent_callsite: bool,
    parent_callsite: Option<Identifier>,
}

struct LocalHolder {
    local_state: RefCell<HashMap<Identifier, LocalCallsiteInfo>>,
    parents_ref: RefCell<Option<Arc<Parents>>>,
}

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
    parent_callsite: Option<Identifier>,
}

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub struct Latencies(Arc<Timings>, Arc<Parents>);

impl Drop for LocalHolder {
    fn drop(&mut self) {
        println!(
            ">>>>>>> drop called for thread {:?}",
            thread::current().id()
        );
        let parents = self.parents_ref.borrow();
        let parents = parents.as_ref().unwrap();
        let mut parents = parents.write().unwrap();
        println!(">>>>>>> lock obtained");
        for (callsite, local_info) in self.local_state.borrow().iter() {
            parents.entry(callsite.clone()).or_insert(CallsiteParent {
                knows_parent: true,
                parent: local_info.parent_callsite.clone(),
            });
            let mut parent = parents.get_mut(callsite).unwrap();
            println!("parent={:?}", parent);
            if !parent.knows_parent {
                parent.knows_parent = true;
                parent.parent = local_info.parent_callsite.clone();
            }
            println!("parent={:?}", parent);
        }
        println!(
            ">>>>>>> drop completed for thread {:?}",
            thread::current().id()
        );
    }
}

thread_local! {
    static LOCAL_HOLDER: LocalHolder = LocalHolder { local_state: RefCell::new(HashMap::new()), parents_ref: RefCell::new(None) };
}

impl Latencies {
    pub fn new() -> Latencies {
        let timings = RwLock::new(HashMap::new());
        let parents = RwLock::new(HashMap::new());
        Latencies(Arc::new(timings), Arc::new(parents))
    }

    pub fn read(
        &self,
        f: impl FnOnce(&HashMap<Identifier, CallsiteTiming>, &HashMap<Identifier, CallsiteParent>),
    ) {
        for (_, v) in self.0.write().unwrap().iter_mut() {
            v.total_time.refresh_timeout(Duration::from_millis(60000));
            v.active_time.refresh_timeout(Duration::from_millis(60000));
        }
        f(
            self.0.read().unwrap().deref(),
            self.1.read().unwrap().deref(),
        );
    }

    pub fn print_mean_timings(&self) {
        self.read(|timings, parents| {
            println!("\nMean timing values by span:");

            for (callsite, v) in timings.iter() {
                let mean_total_time = v.total_time.mean();
                let mean_active_time = v.active_time.mean();
                let total_time_count = v.total_time.len();
                let active_time_count = v.active_time.len();
                let parent = &parents.get(callsite).unwrap().parent;
                println!(
                    "  callsite={:?}, parent={:?}, callsite_str={}, span_name={}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                    callsite, parent, v.callsite_str, v.span_name, mean_total_time, total_time_count, mean_active_time,active_time_count
                );
            }
        });
    }

    fn ensure_local_parents(&self) {
        LOCAL_HOLDER.with(|lh| {
            let mut x = lh.parents_ref.borrow_mut();
            if x.is_none() {
                *x = Some(self.1.clone());
            }
        });
    }

    fn with_recorder(&self, callsite: &Identifier, f: impl Fn(&mut LocalCallsiteInfo) -> ()) {
        LOCAL_HOLDER.with(|lh| {
            let local_state = &lh.local_state;
            let mut callsite_recorders = local_state.borrow_mut();
            let mut local_info = callsite_recorders
                .entry(callsite.clone())
                .or_insert_with(|| {
                    println!(
                    "***** thread-loacal CallsiteRecorder created for callsite={:?} on thread={:?}",
                    callsite,
                    thread::current().id()
                );

                    let callsite_timings = self.0.read().unwrap();
                    let callsite_timing = callsite_timings.get(&callsite).unwrap();

                    LocalCallsiteInfo {
                        total_time: callsite_timing.total_time.recorder(),
                        active_time: callsite_timing.active_time.recorder(),
                        knows_parent_callsite: false,
                        parent_callsite: None,
                    }
                });

            f(&mut local_info);
        });
    }
}

impl<S> Layer<S> for Latencies
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn register_callsite(&self, meta: &Metadata<'_>) -> Interest {
        //println!("`register_callsite` entered");
        if !meta.is_span() {
            return Interest::never();
        }

        self.ensure_local_parents();

        let meta_name = meta.name();
        let callsite = meta.callsite();
        let callsite_str = format!("{}-{}", meta.module_path().unwrap(), meta.line().unwrap());
        let interest = Interest::always();

        let mut map = self.0.write().unwrap();

        let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 1000, 1).unwrap();
        hist.auto(true);
        let hist2 = hist.clone();
        let hist: SyncHistogram<u64> = hist.into();
        let hist2: SyncHistogram<u64> = hist2.into();

        map.insert(
            callsite.clone(),
            CallsiteTiming {
                callsite_str: callsite_str.to_owned(),
                span_name: meta_name.to_owned(),
                total_time: hist,
                active_time: hist2,
            },
        );

        //println!(
        //     "`register_callsite` executed with id={:?}, meta_name={}",
        //     callsite, meta_name
        // );

        interest
    }

    fn on_new_span(&self, _attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        //println!("`new_span` entered");
        let span = ctx.span(id).unwrap();
        let parent_span = span.parent();
        let parent_callsite = parent_span.map(|span_ref| span_ref.metadata().callsite());

        span.extensions_mut().insert(SpanTiming {
            created_at: Instant::now(),
            entered_at: Instant::now(),
            acc_active_time: 0,
            parent_callsite,
        });
        //println!("`new_span` executed with id={:?}", id);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        //println!("entered `enter` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.entered_at = Instant::now();
        //println!("`enter` executed with id={:?}", id);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        //println!("entered `exit` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.acc_active_time += (Instant::now() - span_timing.entered_at).as_micros() as u64;
        //println!("`try_close` executed for span id {:?}", id);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        //println!("entered `try_close` wth span Id {:?}", id);

        let span = ctx.span(&id).unwrap();
        let callsite = span.metadata().callsite();
        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        self.with_recorder(&callsite, |r| {
            r.total_time
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
            r.active_time.record(span_timing.acc_active_time).unwrap();
            if !r.knows_parent_callsite {
                r.knows_parent_callsite = true;
                r.parent_callsite = span_timing.parent_callsite.clone();
            }
        });

        //println!("`try_close` executed for span id {:?}", id);
    }
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send) -> Latencies {
    let latencies = Latencies::new();

    Registry::default().with(latencies.clone()).init();

    thread::scope(|s| {
        s.spawn(f);
    });

    latencies
}

/// Measures latencies of spans in async function `f` running on the [tokio] runtime.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_tokio<F>(f: impl FnOnce() -> F + Send) -> Latencies
where
    F: Future<Output = ()> + Send,
{
    measure_latencies(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                f().await;
            });
    })
}

mod example {
    use super::*;

    #[instrument(level = "trace")]
    pub async fn f() {
        let mut foo: u64 = 1;

        for _ in 0..4 {
            println!("Before my_great_span");

            async {
                thread::sleep(Duration::from_millis(3));
                tokio::time::sleep(Duration::from_millis(100)).await;
                foo += 1;
                info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
                println!("Before my_other_span");
                async {
                    thread::sleep(Duration::from_millis(2));
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    warn!(yak_shaved = false, yak_count = -1, "failed to shave yak");
                }
                .instrument(tracing::trace_span!("my_other_span"))
                .await;
            }
            .instrument(tracing::trace_span!("my_great_span"))
            .await
        }
    }
}

fn main() {
    use example::f;

    let latencies = measure_latencies_tokio(|| async {
        f().await;
        f().await;
    });

    latencies.print_mean_timings();

    // let timings = latencies.read();
    // println!("\nMedian timings by span:");
    // for (callsite, v) in timings.iter() {
    //     let median_total_time = v.total_time.value_at_quantile(0.5);
    //     let median_active_time = v.active_time.value_at_quantile(0.5);
    //     let total_time_count = v.total_time.len();
    //     let active_time_count = v.active_time.len();
    //     println!(
    //         "  callsite_id={:?}, parent_callsite={:?}, callsite_str={}, span_name={}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
    //         callsite, v.parent, v.callsite_str, v.span_name, median_total_time, total_time_count, median_active_time,active_time_count
    //         // "  callsite_str={}, span_name={}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
    //         //  v.callsite_str, v.span_name, median_total_time, total_time_count, median_active_time,active_time_count
    //     );
    // }
}
