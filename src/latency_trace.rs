//! supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented wth the Rust [tracing](https://crates.io/crates/tracing) library, this library
//! uses the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture both total and active
//! span timings, where:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use hdrhistogram::Histogram;
use log;
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use thread_local_drop::{self, Control, Holder};
use tracing::{callsite::Identifier, Id, Subscriber};
use tracing_core::span::Attributes;
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer, Registry,
};

use crate::map::{BTreeMapExt, HashMapExt};

//=================
// SpanGroup

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    callsite: Identifier,
    code_line: String,
    name: String,
    props: Vec<(String, String)>,
}

impl SpanGroup {
    pub fn new(attrs: &Attributes, props: Vec<(String, String)>) -> Self {
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

//=================
// Timing

#[derive(Clone)]
pub struct Timing {
    pub total_time: Histogram<u64>,
    pub active_time: Histogram<u64>,
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

//=================
// Info

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

    // pub fn name_to_span_group_timing_pairs(&self) -> BTreeMapExt<String, Vec<(SpanGroup, Timing)>> {
    //     let mut outer_map: BTreeMap<String, Vec<(SpanGroup, Timing)>> = BTreeMap::new();

    //     self.timings.iter().for_each(|(span_group, timing)| {
    //         let name = span_group.name.clone();
    //         outer_map
    //             .entry(name)
    //             .or_insert_with(|| Vec::new())
    //             .push((span_group.clone(), timing.clone()));
    //     });

    //     outer_map.into()
    // }

    // pub fn callsite_to_span_group_timing_pairs(
    //     &self,
    // ) -> HashMapExt<Identifier, Vec<(SpanGroup, Timing)>> {
    //     let mut outer_map: HashMap<Identifier, Vec<(SpanGroup, Timing)>> = HashMap::new();

    //     self.timings.iter().for_each(|(span_group, timing)| {
    //         let callsite = span_group.callsite.clone();
    //         outer_map
    //             .entry(callsite)
    //             .or_insert_with(|| Vec::new())
    //             .push((span_group.clone(), timing.clone()));
    //     });

    //     outer_map.into()
    // }
}

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    span_group: SpanGroup,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
    parent_callsite: Option<Identifier>,
}

//=================
// Latencies

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub struct Latencies {
    control: Control<Info, Info>,
    span_grouper: Option<Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>>,
}

impl Latencies {
    fn new(
        span_grouper: Option<
            Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>,
        >,
    ) -> Latencies {
        Latencies {
            control: Control::new(Info::new(), op),
            span_grouper,
        }
    }

    pub fn with<V>(&self, f: impl FnOnce(&Info) -> V) -> V {
        self.control.with_acc(f).unwrap()
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
        let span_group = SpanGroup::new(
            attrs,
            self.span_grouper
                .as_ref()
                .map(|f| f(attrs))
                .unwrap_or(vec![]),
        );

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
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<Info, Info> = Holder::new(|| Info::new());
}

//=================
// functions

/// Used to accumulate results on [`Control`].
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

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
fn measure_latencies_priv(
    span_grouper: Option<Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>>,
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
    span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    f: impl FnOnce() -> () + Send + 'static,
) -> Latencies {
    measure_latencies_priv(Some(Arc::new(span_grouper)), f)
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
    span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;
    use tracing::{info, instrument, warn, Instrument};

    #[instrument(level = "trace")]
    pub async fn f() {
        let mut foo: u64 = 1;

        for i in 0..8 {
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
                .instrument(tracing::trace_span!("my_other_span", foo = i % 2))
                .await;
            }
            .instrument(tracing::trace_span!(
                "my_great_span",
                foo = i % 2,
                bar = i % 4
            ))
            .await
        }
    }

    fn are_close(left: f64, right: f64, pct: f64) -> bool {
        let avg_abs = (left.abs() + right.abs()) / 2.0;
        (left - right).abs() <= avg_abs * pct
    }

    #[test]
    fn test_default_grouping() {
        let latencies = measure_latencies_tokio(|| async {
            let h1 = tokio::spawn(f());
            let h2 = tokio::spawn(f());
            _ = h1.await;
            _ = h2.await;
        });

        latencies.with(|info| {
            let parents = &info.parents;

            let name_to_timing: BTreeMap<String, Timing> = HashMapExt(&info.timings)
                .map_to_btree_map(|k, v| (k.name.clone(), v.clone()))
                .into();

            let name_to_callsite: BTreeMap<String, Identifier> = HashMapExt(&info.timings)
                .map_to_btree_map(|k, _| (k.name.clone(), k.callsite.clone()))
                .into();

            for name in ["f", "my_great_span", "my_other_span"] {
                let parent = parents
                    .get(name_to_callsite.get(name).unwrap())
                    .unwrap()
                    .as_ref();
                let timing = name_to_timing.get(name).unwrap();
                let total_time_mean = timing.total_time.mean();
                let total_time_count = timing.total_time.len();
                let active_time_mean = timing.active_time.mean();
                let active_time_count = timing.active_time.len();

                match name {
                    "f" => {
                        let expected_parent = None;
                        let expected_total_time_mean = 130.0 * 8.0 * 1000.0;
                        let expected_active_time_mean = 5.0 * 8.0 * 1000.0;
                        let expected_total_time_count = 2;
                        let expected_active_time_count = 2;

                        assert_eq!(parent, expected_parent, "{name} parent");

                        println!(
                            "** {name} total_time_mean: {total_time_mean}, {}",
                            expected_total_time_mean
                        );
                        assert!(
                            are_close(total_time_mean, expected_total_time_mean, 0.1),
                            "{name} total_time mean"
                        );

                        println!(
                            "** {name} total_time_count: {total_time_count}, {}",
                            expected_total_time_count
                        );
                        assert_eq!(
                            total_time_count, expected_total_time_count,
                            "{name} total_time count"
                        );

                        println!(
                            "** {name} active_time_mean: {active_time_mean}, {}",
                            expected_active_time_mean
                        );
                        assert!(
                            are_close(active_time_mean, expected_active_time_mean, 0.2),
                            "{name} active_time mean"
                        );

                        println!(
                            "** {name} active_time_count: {active_time_count}, {}",
                            expected_active_time_count
                        );
                        assert_eq!(
                            active_time_count, expected_active_time_count,
                            "{name} active_time count"
                        );
                    }

                    "my_great_span" => {
                        let expected_parent = Some(name_to_callsite.get("f").unwrap());
                        let expected_total_time_mean = 130.0 * 1000.0;
                        let expected_active_time_mean = 5.0 * 1000.0;
                        let expected_total_time_count = 16;
                        let expected_active_time_count = 16;

                        assert_eq!(parent, expected_parent, "{name} parent");

                        println!(
                            "** {name} total_time_mean: {total_time_mean}, {}",
                            expected_total_time_mean
                        );
                        assert!(
                            are_close(total_time_mean, expected_total_time_mean, 0.1),
                            "{name} total_time mean"
                        );

                        println!(
                            "** {name} total_time_count: {total_time_count}, {}",
                            expected_total_time_count
                        );
                        assert_eq!(
                            total_time_count, expected_total_time_count,
                            "{name} total_time count"
                        );

                        println!(
                            "** {name} active_time_mean: {active_time_mean}, {}",
                            expected_active_time_mean
                        );
                        assert!(
                            are_close(active_time_mean, expected_active_time_mean, 0.2),
                            "{name} active_time mean"
                        );

                        println!(
                            "** {name} active_time_count: {active_time_count}, {}",
                            expected_active_time_count
                        );
                        assert_eq!(
                            active_time_count, expected_active_time_count,
                            "{name} active_time count"
                        );
                    }

                    "my_other_span" => {
                        let expected_parent = Some(name_to_callsite.get("my_great_span").unwrap());
                        let expected_total_time_mean = 27.0 * 1000.0;
                        let expected_active_time_mean = 2.0 * 1000.0;
                        let expected_total_time_count = 16;
                        let expected_active_time_count = 16;

                        assert_eq!(parent, expected_parent, "{name} parent");

                        println!(
                            "** {name} total_time_mean: {total_time_mean}, {}",
                            expected_total_time_mean
                        );
                        assert!(
                            are_close(total_time_mean, expected_total_time_mean, 0.1),
                            "{name} total_time mean"
                        );

                        println!(
                            "** {name} total_time_count: {total_time_count}, {}",
                            expected_total_time_count
                        );
                        assert_eq!(
                            total_time_count, expected_total_time_count,
                            "{name} total_time count"
                        );

                        println!(
                            "** {name} active_time_mean: {active_time_mean}, {}",
                            expected_active_time_mean
                        );
                        assert!(
                            are_close(active_time_mean, expected_active_time_mean, 0.2),
                            "{name} active_time mean"
                        );

                        println!(
                            "** {name} active_time_count: {active_time_count}, {}",
                            expected_active_time_count
                        );
                        assert_eq!(
                            active_time_count, expected_active_time_count,
                            "{name} active_time count"
                        );
                    }

                    _ => {}
                }
            }
        });
    }
}
