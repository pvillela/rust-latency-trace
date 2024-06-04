//! Undocumented functions to support benchmarks.]

use crate::{core_internals_pre::LatencyTracePriv, latency_trace::LatencyTrace};
use std::{future::Future, hint::black_box};
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    util::SubscriberInitExt,
    Registry,
};

/// Set-up for measurement of latencies.
pub fn measure_latencies1(lt: LatencyTrace) {
    let ltp = LatencyTracePriv::new(lt.0);
    let reg: tracing_subscriber::layer::Layered<LatencyTracePriv, Registry> =
        Registry::default().with(ltp.clone());
    let default_dispatch_exists =
        tracing::dispatcher::get_default(|d| d.is::<Layered<LatencyTracePriv, Registry>>());
    if !default_dispatch_exists {
        reg.init();
    }
    black_box(ltp);
}

/// Executes tracing up to completion of instrumnted function, before final collection and aggregation.
pub fn measure_latencies2(lt: LatencyTrace, f: impl FnOnce() + Send + 'static) {
    let ltp = LatencyTracePriv::new(lt.0);
    let reg: tracing_subscriber::layer::Layered<LatencyTracePriv, Registry> =
        Registry::default().with(ltp.clone());
    let default_dispatch_exists =
        tracing::dispatcher::get_default(|d| d.is::<Layered<LatencyTracePriv, Registry>>());
    if !default_dispatch_exists {
        reg.init();
    }
    f();
    black_box(ltp);
}

/// Executes tracing up to completion of instrumnted async function, before final collection and aggregation.
pub fn measure_latencies2_tokio<F>(lt: LatencyTrace, f: impl FnOnce() -> F + Send + 'static)
where
    F: Future<Output = ()> + Send,
{
    measure_latencies2(lt, || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f());
    })
}
