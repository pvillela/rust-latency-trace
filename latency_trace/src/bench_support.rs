//! Undocumented functions to support benchmarks in `dev_support` crate.]

use crate::LatencyTrace;
use std::{future::Future, hint::black_box};

/// Set-up for measurement of latencies.
pub fn measure_latencies1(lt: LatencyTrace) {
    black_box(measure_latencies2(lt, || black_box(())));
}

/// Executes tracing up to completion of instrumnted function, before final collection and aggregation.
pub fn measure_latencies2(lt: LatencyTrace, f: impl FnOnce()) -> usize {
    f();
    black_box(lt.0.take_acc_timings().len())
}

/// Executes tracing up to completion of instrumnted async function, before final collection and aggregation.
pub fn measure_latencies2_tokio<F>(lt: LatencyTrace, f: impl FnOnce() -> F) -> usize
where
    F: Future<Output = ()> + Send,
{
    measure_latencies2(lt, move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime error on bench_support::measure_latencies_2")
            .block_on(f());
    })
}
