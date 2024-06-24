//! Undocumented functions to support benchmarks.]

use crate::{core_internals_pre::LatencyTracePriv, latency_trace::LatencyTrace};
use std::{future::Future, hint::black_box};

/// Set-up for measurement of latencies.
pub fn measure_latencies1(lt: LatencyTrace) {
    measure_latencies2(lt, || ())
}

/// Executes tracing up to completion of instrumnted function, before final collection and aggregation.
pub fn measure_latencies2(lt: LatencyTrace, f: impl Fn() + Send + 'static) {
    let g = move |ltp: &LatencyTracePriv| {
        f();
        black_box(ltp);
    };
    lt.init_and_run(g);
}

/// Executes tracing up to completion of instrumnted async function, before final collection and aggregation.
pub fn measure_latencies2_tokio<F>(lt: LatencyTrace, f: impl Fn() -> F + Send + Sync + 'static)
where
    F: Future<Output = ()> + Send,
{
    measure_latencies2(lt, move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f());
    })
}
