//! This captures both total and sync timings:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use env_logger;
use hdrhistogram::Histogram;
use log;
use std::{
    collections::HashMap,
    env::set_var,
    future::Future,
    hash::Hash,
    thread::{self, ThreadId},
    time::{Duration, Instant},
};
use thread_local_drop::{self, Control, Holder};
use tracing::{callsite::Identifier, info, instrument, warn, Id, Instrument, Subscriber};
use tracing_core::span::Attributes;
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer, Registry,
};

//=================
// Types

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    callsite: Identifier,
    code_line: String,
    name: String,
    props: Vec<(String, String)>,
}

impl SpanGroup {
    fn new(attrs: &Attributes, props: Vec<(String, String)>) -> Self {
        let meta = attrs.metadata();
        SpanGroup {
            callsite: meta.callsite(),
            code_line: format!("{}:{}", meta.module_path().unwrap(), meta.line().unwrap()),
            name: meta.name().to_owned(),
            props,
        }
    }

    pub fn callsite_id(&self) -> &Identifier {
        &self.callsite
    }

    pub fn code_line(&self) -> &str {
        &self.code_line
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn props(&self) -> &Vec<(String, String)> {
        &self.props
    }
}

pub struct Timing {
    total_time: Histogram<u64>,
    active_time: Histogram<u64>,
}

impl Timing {
    fn new() -> Self {
        let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 1000, 1).unwrap();
        hist.auto(true);
        let hist2 = hist.clone();

        Self {
            total_time: hist,
            active_time: hist2,
        }
    }
}

pub struct Info {
    pub parents: HashMap<Identifier, Option<Identifier>>,
    pub timings: HashMap<SpanGroup, Timing>,
}

impl Info {
    fn new() -> Self {
        Self {
            parents: HashMap::new(),
            timings: HashMap::new(),
        }
    }
}

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    span_group: SpanGroup,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
    parent_callsite: Option<Identifier>,
}

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub struct Latencies {
    control: Control<Info, Info>,
    span_grouper: Option<fn(&Attributes) -> Vec<(String, String)>>,
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<Info, Info> = Holder::new(|| Info::new());
}

//=================
// impls

fn op(data: &Info, acc: &mut Info, tid: &ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    for (k, v) in data.parents.iter() {
        acc.parents.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in data.timings.iter() {
        let timing = acc
            .timings
            .entry(k.clone())
            .or_insert_with(|| Timing::new());
        timing.total_time.add(v.total_time.clone()).unwrap();
        timing.active_time.add(v.active_time.clone()).unwrap();
    }
}

impl Latencies {
    fn new(span_grouper: Option<fn(&Attributes) -> Vec<(String, String)>>) -> Latencies {
        Latencies {
            control: Control::new(Info::new(), op),
            span_grouper,
        }
    }

    pub fn with<V>(&self, f: impl FnOnce(&Info) -> V) -> V {
        let acc = self.control.accumulator().unwrap();
        let info = &acc.acc;
        f(info)
    }

    pub fn print_mean_timings(&self) {
        self.with(|info| {
            println!("\nMean timing values by span group:");

            let parents = &info.parents;

            for (span_group, v) in info.timings.iter() {
                let mean_total_time = v.total_time.mean();
                let mean_active_time = v.active_time.mean();
                let total_time_count = v.total_time.len();
                let active_time_count = v.active_time.len();
                let parent = parents.get(span_group.callsite_id()).unwrap();
                println!(
                    "  span_group={:?}, parent={:?}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                    span_group, parent, mean_total_time, total_time_count, mean_active_time,active_time_count
                );
            }
        });
    }

    fn update_parents(&self, callsite: &Identifier, parent: &Option<Identifier>) {
        log::trace!(
            "entered `update_parent_info`for callsite id {:?} on thread {:?}",
            callsite,
            thread::current().id(),
        );
        self.control.with_tl_mut(&LOCAL_INFO, |info| {
            let parents = &mut info.parents;
            if parents.contains_key(callsite) {
                // Both local and global parents info are good for this callsite.
                return;
            }

            // Update local parents
            {
                parents.insert(callsite.clone(), parent.clone());
            }
        });
    }

    fn update_timings(&self, span_group: &SpanGroup, f: impl Fn(&mut Timing)) {
        self.control.with_tl_mut(&LOCAL_INFO,|info| {
            let  timings = &mut info.timings;
            let mut timing = timings
                .entry(span_group.clone())
                .or_insert_with(|| {
                    log::trace!(
                        "***** thread-loacal LocalCallsiteTiming created for callsite={:?} on thread={:?}",
                        span_group,
                        thread::current().id()
                    );
                    Timing::new()
                });

            f(&mut timing);
            log::trace!(
                "***** exiting with_local_callsite_info for callsite={:?} on thread={:?}",
                span_group,
                thread::current().id()
            );
});
    }
}

impl<S> Layer<S> for Latencies
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_new_span`");
        let span = ctx.span(id).unwrap();
        let parent_span = span.parent();
        let parent_callsite = parent_span.map(|span_ref| span_ref.metadata().callsite());
        let span_group =
            SpanGroup::new(attrs, self.span_grouper.map(|f| f(attrs)).unwrap_or(vec![]));

        span.extensions_mut().insert(SpanTiming {
            span_group,
            created_at: Instant::now(),
            entered_at: Instant::now(),
            acc_active_time: 0,
            parent_callsite,
        });
        log::trace!("`on_new_span` executed with id={:?}", id);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_enter` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.entered_at = Instant::now();
        log::trace!("`on_enter` executed with id={:?}", id);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_exit` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.acc_active_time += (Instant::now() - span_timing.entered_at).as_micros() as u64;
        log::trace!("`on_exit` executed for span id {:?}", id);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_close` wth span Id {:?}", id);

        let span = ctx.span(&id).unwrap();
        let callsite = span.metadata().callsite();
        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        self.update_timings(&span_timing.span_group, |r| {
            r.total_time
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
            r.active_time.record(span_timing.acc_active_time).unwrap();
        });

        log::trace!(
            "`on_close` completed call to with_local_callsite_info for span id {:?}",
            id
        );

        self.update_parents(&callsite, &span_timing.parent_callsite);

        log::trace!("`on_close` executed for span id {:?}", id);
    }
}

//=================
// functions

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
fn measure_latencies_priv(
    span_grouper: Option<fn(&Attributes) -> Vec<(String, String)>>,
    f: impl FnOnce() + Send + 'static,
) -> Latencies {
    let latencies = Latencies::new(span_grouper);
    Registry::default().with(latencies.clone()).init();
    f();
    latencies.control.ensure_tls_dropped();
    latencies
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send + 'static) -> Latencies {
    measure_latencies_priv(None, f)
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_with_custom_grouping(
    span_grouper: fn(&Attributes) -> Vec<(String, String)>,
    f: impl FnOnce() -> () + Send + 'static,
) -> Latencies {
    measure_latencies_priv(Some(span_grouper), f)
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

/// Measures latencies of spans in async function `f` running on the [tokio] runtime.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_with_custom_grouping_tokio<F>(
    span_grouper: fn(&Attributes) -> Vec<(String, String)>,
    f: impl FnOnce() -> F + Send + 'static,
) -> Latencies
where
    F: Future<Output = ()> + Send,
{
    measure_latencies_with_custom_grouping(span_grouper, || {
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
            log::trace!("Before my_great_span");

            async {
                thread::sleep(Duration::from_millis(3));
                tokio::time::sleep(Duration::from_millis(100)).await;
                foo += 1;
                info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
                log::trace!("Before my_other_span");
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

    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "trace");

    let latencies = measure_latencies_tokio(|| async {
        // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
        // Otherwise, setting a second logger would panic.
        _ = env_logger::try_init();

        let h1 = tokio::spawn(f());
        let h2 = tokio::spawn(f());
        _ = h1.await;
        _ = h2.await;
    });

    latencies.print_mean_timings();

    latencies.with(|info| {
        println!("\nMedian timings by span group:");

        let parents = &info.parents;

        for (span_group, v) in info.timings.iter() {
            let median_total_time = v.total_time.value_at_percentile(50.0);
            let median_active_time = v.active_time.value_at_percentile(50.0);
            let total_time_count = v.total_time.len();
            let active_time_count = v.active_time.len();
            let parent = parents.get(span_group.callsite_id()).unwrap();
            println!(
                "  span_group={:?}, parent={:?}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
                span_group, parent, median_total_time, total_time_count, median_active_time,active_time_count
            );
        }
    });
}
