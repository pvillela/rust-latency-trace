//! `tokio`-related latency reporting methods of [`crate::lt_collect_g::LatencyTraceG`].
//! Present only when the **"tokio"** feature flag is enabled.

use crate::{
    lt_collect_g::LatencyTraceG,
    lt_refine_g::Timings,
    tlc_param::{TlcDirect, TlcParam},
};
use std::future::Future;

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcDirect,
{
    /// Executes the instrumented async function `f`, running on the `tokio` runtime; after `f` completes,
    /// returns the observed latencies.
    /// Present only when the **"tokio"** feature flag is enabled.
    pub fn measure_latencies_tokio<F>(&self, f: impl FnOnce() -> F) -> Timings
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Tokio runtime error")
                .block_on(f());
        })
    }
}
