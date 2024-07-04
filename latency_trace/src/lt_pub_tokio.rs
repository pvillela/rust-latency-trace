//! This module is supported on **`feature="tokio"`** only.
//! Publicly exported `tokio`-related methods of [`LatencyTrace`].

use crate::{
    lt_refine_g::Timings, lt_report_g::ActivationError, probed_trace::ProbedTrace, LatencyTrace,
};
use std::future::Future;

impl LatencyTrace {
    /// Executes the instrumented async function `f`, running on the `tokio` runtime; after `f` completes,
    /// returns the observed latencies.
    /// Requires **`feature="tokio"`**.
    pub fn measure_latencies_tokio<F>(&self, f: impl FnOnce() -> F) -> Timings
    where
        F: Future<Output = ()> + Send,
    {
        self.0.measure_latencies_tokio(f)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; returns a [`ProbedTrace`]
    /// that allows partial latencies to be reported before `f` completes.
    /// Requires **`feature="tokio"`**.
    pub fn measure_latencies_probed_tokio<F>(
        self,
        f: impl FnOnce() -> F + Send + 'static,
    ) -> Result<ProbedTrace, ActivationError>
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies_probed(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Tokio runtime error")
                .block_on(f());
        })
    }
}
