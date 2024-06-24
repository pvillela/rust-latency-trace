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
    measure_latencies2(lt, || ())
}

/// Executes tracing up to completion of instrumnted function, before final collection and aggregation.
pub fn measure_latencies2(lt: LatencyTrace, f: impl Fn() + Send + 'static) {
    let default_dispatch_exists =
        tracing::dispatcher::get_default(|d| d.is::<Layered<LatencyTracePriv, Registry>>());
    let ltp_new = LatencyTracePriv::new(lt.0);
    let new_config = (ltp_new.hist_high, ltp_new.hist_sigfig);
    if !default_dispatch_exists {
        let reg: tracing_subscriber::layer::Layered<LatencyTracePriv, Registry> =
            Registry::default().with(ltp_new);
        reg.init();
    }
    tracing::dispatcher::get_default(|disp| {
        let ltp: &LatencyTracePriv = disp
            .downcast_ref()
            .expect("existing dispatcher must be of type `LatencyTracePriv`");
        let curr_config = (ltp.hist_high, ltp.hist_sigfig);
        // Note: below assertion does not cover the `LatencyTracePriv` `span_grouper` field as it is not
        // possible to check equality of functions.
        assert_eq!(
            curr_config, new_config,
            "New and existing LatencyTrace configuration settings must be identical."
        );
        f();
        black_box(ltp);
    });
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
