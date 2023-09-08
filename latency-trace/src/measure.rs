use crate::{default_span_grouper, Latencies, LatencyTrace, SpanGroup, Timing};
use std::{collections::BTreeMap, future::Future, sync::Arc};
use tracing_core::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
fn measure_latencies_priv(
    span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    f: impl FnOnce() + Send + 'static,
) -> Latencies {
    let lt = LatencyTrace::new(Arc::new(span_grouper));
    Registry::default().with(lt.clone()).init();
    f();
    lt.control.ensure_tls_dropped();
    lt.generate_latencies()
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send + 'static) -> Latencies {
    measure_latencies_priv(default_span_grouper, f)
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_with_custom_grouping(
    span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    f: impl FnOnce() -> () + Send + 'static,
) -> Latencies {
    measure_latencies_priv(span_grouper, f)
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

/// Aggregate timings by sets of [`crate::SpanGroup`]s that have the same value when `f` is applied.
pub fn aggregate_timings<G>(
    latencies: &Latencies,
    f: impl Fn(&SpanGroup) -> G,
) -> BTreeMap<G, Timing>
where
    G: Ord + Clone,
{
    let mut res: BTreeMap<G, Timing> = BTreeMap::new();
    for (k, v) in &latencies.timings {
        let g = f(k);
        let timing = match res.get_mut(&g) {
            Some(timing) => timing,
            None => {
                res.insert(g.clone(), Timing::new());
                res.get_mut(&g).unwrap()
            }
        };
        timing.total_time.add(v.total_time()).unwrap();
        timing.active_time.add(v.active_time()).unwrap();
    }
    res
}
