//! Based on `tracing_counter_by_span_name_naive` and `tracing_timing_original`.
//! Naive because it does not use [tracing_subscriber::Registry] and instead uses a naive storage
//! approach based on [std::sync::RwLock].
//!
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
    instrument,
    subscriber::{Interest, Subscriber},
    Id, Instrument, Metadata,
};
use tracing_core::span::Attributes;
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
    Layer,
};

#[derive(Debug)]
pub struct CallsiteTiming {
    span_name: String,
    total_time: SyncHistogram<u64>,
    active_time: SyncHistogram<u64>,
}

struct CallsiteRecorder {
    total_time: Recorder<u64>,
    active_time: Recorder<u64>,
}

#[derive(Debug)]
struct SpanTime {
    callsite: Identifier,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
}

/// Collects counts emitted by application spans and events.
#[derive(Debug)]
struct Timings {
    callsite_timings: RwLock<HashMap<Identifier, CallsiteTiming>>,
    span_times: RwLock<HashMap<Id, SpanTime>>,
}

pub struct Latencies(Arc<Timings>);

impl Clone for Latencies {
    fn clone(&self) -> Self {
        Latencies(self.0.clone())
    }
}

impl Latencies {
    pub fn new() -> Latencies {
        let timing_by_span = RwLock::new(HashMap::new());
        let span_start_times = RwLock::new(HashMap::new());
        let timings = Timings {
            callsite_timings: timing_by_span,
            span_times: span_start_times,
        };
        Latencies(Arc::new(timings))
    }

    pub fn read<'a>(&'a self) -> impl Deref<Target = HashMap<Identifier, CallsiteTiming>> + 'a {
        for (_, v) in self.0.callsite_timings.write().unwrap().iter_mut() {
            v.total_time.refresh();
            v.active_time.refresh();
        }
        self.0.callsite_timings.read().unwrap()
    }

    pub fn print_mean_timings(&self) {
        println!("\nMean timing values by span:");
        for (_, v) in self.0.callsite_timings.write().unwrap().iter_mut() {
            v.total_time.refresh();
            v.active_time.refresh();
            let mean_total_time = v.total_time.mean();
            let mean_active_time = v.active_time.mean();
            let total_time_count = v.total_time.len();
            let active_time_count = v.active_time.len();
            println!(
                "  span_name={}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                v.span_name, mean_total_time, total_time_count, mean_active_time,active_time_count
            );
        }
    }
}

fn with_recorder(timings: &Timings, id: &Identifier, f: impl Fn(&mut CallsiteRecorder) -> ()) {
    thread_local! {
        static RECORDERS: RefCell<HashMap<Identifier, CallsiteRecorder>> = RefCell::new(HashMap::new());
    }

    RECORDERS.with(|m| {
        let mut callsite_recorders = m.borrow_mut();
        let mut recorder = callsite_recorders.entry(id.clone()).or_insert_with(|| {
            println!(
                "***** thread-loacal CallsiteRecorder created for callsite={:?} on thread={:?}",
                id,
                thread::current().id()
            );

            let callsite_timings = timings.callsite_timings.read().unwrap();
            let callsite_timing = callsite_timings.get(&id).unwrap();

            CallsiteRecorder {
                total_time: callsite_timing.total_time.recorder(),
                active_time: callsite_timing.active_time.recorder(),
            }
        });

        f(&mut recorder);
    });
}

impl<S: Subscriber> Layer<S> for Latencies {
    fn register_callsite(&self, meta: &Metadata<'_>) -> Interest {
        //println!("`register_callsite` entered");
        if !meta.is_span() {
            return Interest::never();
        }

        let meta_name = meta.name();
        let callsite = meta.callsite();
        let interest = Interest::always();

        let mut map = self.0.callsite_timings.write().unwrap();

        let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 1000, 1).unwrap();
        hist.auto(true);
        let hist2 = hist.clone();
        let hist: SyncHistogram<u64> = hist.into();
        let hist2: SyncHistogram<u64> = hist2.into();

        map.insert(
            callsite.clone(),
            CallsiteTiming {
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

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
        //println!("`new_span` entered");
        let callsite = attrs.metadata().callsite();

        let mut start_times = self.0.span_times.write().unwrap();
        start_times.insert(
            id.clone(),
            SpanTime {
                callsite: callsite.clone(),
                created_at: Instant::now(),
                entered_at: Instant::now(),
                acc_active_time: 0,
            },
        );

        //println!("`new_span` executed with id={:?}", id);
    }

    fn on_enter(&self, id: &Id, _ctx: Context<'_, S>) {
        //println!("entered `enter` wth span Id {:?}", id);
        let mut span_times = self.0.span_times.write().unwrap();
        let span_time = span_times.get_mut(id).unwrap();
        span_time.entered_at = Instant::now();
        //println!("`enter` executed with id={:?}", id);
    }

    fn on_exit(&self, id: &Id, _ctx: Context<'_, S>) {
        //println!("entered `exit` wth span Id {:?}", id);
        let mut span_times = self.0.span_times.write().unwrap();
        let span_time = span_times.get_mut(id).unwrap();
        span_time.acc_active_time += (Instant::now() - span_time.entered_at).as_micros() as u64;

        //println!("`try_close` executed for span id {:?}", id);
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        //println!("entered `try_close` wth span Id {:?}", id);
        let mut start_times = self.0.span_times.write().unwrap();
        let SpanTime {
            callsite,
            created_at,
            entered_at: _,
            acc_active_time,
        } = start_times.remove(&id).unwrap();

        with_recorder(&self.0, &callsite, |r| {
            r.total_time
                .record((Instant::now() - created_at).as_micros() as u64)
                .unwrap()
        });

        with_recorder(&self.0, &callsite, |r| {
            r.active_time.record(acc_active_time).unwrap()
        });

        //println!("`try_close` executed for span id {:?}", id);
    }
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send) -> Latencies {
    let latencies = Latencies::new();

    tracing_subscriber::registry::Registry::default()
        .with(latencies.clone())
        .init();

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

#[instrument(level = "trace")]
async fn f() {
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

fn main() {
    let latencies = measure_latencies_tokio(|| async {
        f().await;
        f().await;
    });

    latencies.print_mean_timings();

    let timings = latencies.read();
    let timings = timings.deref();
    println!("\nMedian timings by span:");
    for (_, v) in timings.iter() {
        let median_total_time = v.total_time.value_at_quantile(0.5);
        let median_active_time = v.active_time.value_at_quantile(0.5);
        let total_time_count = v.total_time.len();
        let active_time_count = v.active_time.len();
        println!(
            "  span_name={}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
            v.span_name, median_total_time, total_time_count, median_active_time,active_time_count
        );
    }
}
