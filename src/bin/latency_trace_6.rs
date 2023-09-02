//! This captures both total and sync timings:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.
//!
//! WIP latency_trace: refactored for cleaner implementation with separate thread-local for parents, but
//! use of thread-local Drop to synchronize with the global state is not reliable and
//! race conditions between execution of code being measured and SyncHistogram refresh continues to be an
//! issue.

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
    instrument,
    subscriber::{Interest, Subscriber},
    Id, Instrument, Metadata,
};
use tracing_core::span::Attributes;
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer, Registry,
};

//=================
// Types

/// Globally collected information for a callsite.
#[derive(Debug)]
pub struct CallsiteTiming {
    pub callsite_str: String,
    pub span_name: String,
    pub total_time: SyncHistogram<u64>,
    pub active_time: SyncHistogram<u64>,
}

/// Timings by callsite.
type Timings = RwLock<HashMap<Identifier, CallsiteTiming>>;

/// Callsite parents.
/// Separate from [Timings] to avoid locking issues caused by [SyncHistogram].refresh.
type Parents = RwLock<HashMap<Identifier, Option<Identifier>>>;

/// Thread-local information collected for a callsite.
struct LocalCallsiteTiming {
    total_time: Recorder<u64>,
    active_time: Recorder<u64>,
}

struct LocalHolderOfParentInfo {
    global_ref: RefCell<Option<Arc<Parents>>>,
    local_info: RefCell<HashMap<Identifier, Option<Identifier>>>,
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

//=================
// Thread-locals

thread_local! {
    static LOCAL_HOLDER_OF_PARENT_INFO: LocalHolderOfParentInfo = LocalHolderOfParentInfo {
        global_ref: RefCell::new(None),
        local_info: RefCell::new(HashMap::new()),
    };
}

thread_local! {
    static LOCAL_CALLSITE_INFO: RefCell<HashMap<Identifier, LocalCallsiteTiming>> = RefCell::new(HashMap::new());
}

//=================
// impls

impl Drop for LocalHolderOfParentInfo {
    fn drop(&mut self) {
        println!(
            ">>>>>>> drop called for thread {:?}",
            thread::current().id()
        );
        let global_parents = self.global_ref.borrow();
        let global_parents = global_parents.as_ref();
        if global_parents.is_none() {
            return;
        }
        let mut global_parents = global_parents.unwrap().write().unwrap();
        println!(
            ">>>>>>> lock obtained on thread {:?}",
            thread::current().id()
        );
        for (callsite, parent) in self.local_info.borrow().iter() {
            global_parents
                .entry(callsite.clone())
                .or_insert_with(|| parent.clone());
        }

        // TODO: remove this experiment
        let sleep_millis = 5_000;
        println!(
            "thread {:?} will sleep for {} millis",
            thread::current().id(),
            sleep_millis
        );
        thread::sleep(Duration::from_millis(sleep_millis));

        println!(
            ">>>>>>> drop completed for thread {:?}",
            thread::current().id()
        );
    }
}

impl Latencies {
    pub fn new() -> Latencies {
        let timings = RwLock::new(HashMap::new());
        let parents = RwLock::new(HashMap::new());
        Latencies(Arc::new(timings), Arc::new(parents))
    }

    fn refresh(&self) {
        for (_, v) in self.0.write().unwrap().iter_mut() {
            v.total_time.refresh();
            v.active_time.refresh();
        }
    }

    pub fn with(
        &self,
        f: impl FnOnce(&HashMap<Identifier, CallsiteTiming>, &HashMap<Identifier, Option<Identifier>>),
    ) {
        f(
            self.0.read().unwrap().deref(),
            self.1.read().unwrap().deref(),
        );
    }

    pub fn print_mean_timings(&self) {
        self.with(|timings, parents| {
            println!("\nMean timing values by span:");

            for (callsite, v) in timings.iter() {
                let mean_total_time = v.total_time.mean();
                let mean_active_time = v.active_time.mean();
                let total_time_count = v.total_time.len();
                let active_time_count = v.active_time.len();
                let parent = parents.get(callsite).unwrap();
                println!(
                    "  callsite={:?}, parent={:?}, callsite_str={}, span_name={}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                    callsite, parent, v.callsite_str, v.span_name, mean_total_time, total_time_count, mean_active_time,active_time_count
                );
            }
        });
    }

    fn ensure_globacl_parents_ref(&self) {
        LOCAL_HOLDER_OF_PARENT_INFO.with(|lh| {
            let mut x = lh.global_ref.borrow_mut();
            if x.is_none() {
                *x = Some(self.1.clone());
            }
        });
    }

    fn update_local_parent_info(&self, callsite: &Identifier, parent: &Option<Identifier>) {
        LOCAL_HOLDER_OF_PARENT_INFO.with(|lh| {
            let mut x = lh.local_info.borrow_mut();
            x.entry(callsite.clone()).or_insert(parent.clone());
        });
    }

    fn with_local_callsite_info(
        &self,
        callsite: &Identifier,
        f: impl Fn(&mut LocalCallsiteTiming) -> (),
    ) {
        LOCAL_CALLSITE_INFO.with(|local_info| {
            let mut callsite_recorders = local_info.borrow_mut();
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

                    LocalCallsiteTiming {
                        total_time: callsite_timing.total_time.recorder(),
                        active_time: callsite_timing.active_time.recorder(),
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

        self.ensure_globacl_parents_ref();

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

        self.with_local_callsite_info(&callsite, |r| {
            r.total_time
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
            r.active_time.record(span_timing.acc_active_time).unwrap();
        });

        self.update_local_parent_info(&callsite, &span_timing.parent_callsite);

        //println!("`try_close` executed for span id {:?}", id);
    }
}

//=================
// functions

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send + 'static) -> Latencies {
    let latencies = Latencies::new();

    Registry::default().with(latencies.clone()).init();

    // thread::scope(|s| {
    //     s.spawn(f);
    // });

    thread::spawn(f).join().unwrap();

    latencies.refresh();

    latencies
}

/// Measures latencies of spans in async function `f` running on the [tokio] runtime.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_tokio<F>(f: impl FnOnce() -> F + Send + 'static) -> Latencies
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

//=================
// Examples

mod example {
    use super::*;

    #[instrument(level = "trace")]
    pub async fn f() {
        let mut foo: u64 = 1;

        for _ in 0..4 {
            println!("Before outer_async_span");

            async {
                thread::sleep(Duration::from_millis(3));
                tokio::time::sleep(Duration::from_millis(100)).await;
                foo += 1;
                println!("Before inner_async_span");
                async {
                    thread::sleep(Duration::from_millis(2));
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                .instrument(tracing::trace_span!("inner_async_span"))
                .await;
            }
            .instrument(tracing::trace_span!("outer_async_span"))
            .await
        }
    }
}

fn main() {
    use example::f;

    let latencies = measure_latencies_tokio(|| async {
        let h1 = tokio::spawn(f());
        let h2 = tokio::spawn(f());
        _ = h1.await;
        _ = h2.await;
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
