//! Main public interface of library.

use crate::{
    core_internals_post::{report_timings, Timings},
    core_internals_pre::{LatencyTraceCfg, LatencyTracePriv},
    default_span_grouper, ProbedTrace,
};
use std::{future::Future, sync::Arc, thread};
use tracing::span::Attributes;

/// Provides the ability to measure latencies for code (both sync and async) instrumented with the
/// [tracing](https://crates.io/crates/tracing) library.
///
/// Its configuration encapsulates a span grouper function
/// (`impl Fn(&`[`Attributes`]`) -> Vec<(String, String)> + Send + Sync + 'static`)
/// to define [`SpanGroup`](crate::SpanGroup)s, as well as the histogram configuration parameters
/// [`hdrhistogram::Histogram::high`] and [`hdrhistogram::Histogram::sigfig`].
pub struct LatencyTrace(pub(crate) LatencyTraceCfg);

impl Default for LatencyTrace {
    /// Instantiates a [LatencyTrace] with default configuration. The defaults are:
    /// - Grouping of spans using the [`default_span_grouper`], which simply groups by the span's
    /// callsite information, which distills the *tracing* framework's Callsite concept
    /// (see [Metadata and Callsite](https://docs.rs/tracing-core/0.1.31/tracing_core/)).
    /// - Histograms use a `hist_high` of `20,000,000` (20 seconds) and a `hist_sigfig` of 2.
    ///
    /// See [hdrhistogram::Histogram::high] and [hdrhistogram::Histogram::sigfig] for an explanation of these
    /// histogram configuration parameters.
    ///
    /// Once an instance with the default configuration is created, it can be modified with other methods provided
    /// by this struct.
    fn default() -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: Arc::new(default_span_grouper),
            hist_high: 20 * 1000 * 1000,
            hist_sigfig: 2,
        };
        Self(cfg)
    }
}

impl LatencyTrace {
    /// Creates a new [`LatencyTrace`] configured the same as `self` but with the given `span_grouper`.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    pub fn with_span_grouper(
        &self,
        span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    ) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: Arc::new(span_grouper),
            ..self.0
        };
        Self(cfg)
    }

    /// Creates a new [`LatencyTrace`] configured the same as `self` but with the given `hist_high`
    /// (see [hdrhistogram::Histogram::high]).
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` then
    /// this instance will panic when it is used.
    pub fn with_hist_high(&self, hist_high: u64) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: self.0.span_grouper.clone(),
            hist_high,
            hist_sigfig: self.0.hist_sigfig,
        };
        Self(cfg)
    }

    /// Creates a new [`LatencyTrace`] configured the same as `self` but with the given `hist_sigfig`
    /// (see [hdrhistogram::Histogram::sigfig]).
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_sigfic` then
    /// this instance will panic when it is used.
    pub fn with_hist_sigfig(&self, hist_sigfig: u8) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: self.0.span_grouper.clone(),
            hist_high: self.0.hist_high,
            hist_sigfig,
        };
        Self(cfg)
    }

    /// Executes the instrumented function `f` and, after `f` completes, returns the observed latencies.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies(self, f: impl FnOnce()) -> Timings {
        let ltp = LatencyTracePriv::initialized(self.0);
        f();
        let acc = ltp.take_acc_timings();
        report_timings(&ltp, acc)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; after `f` completes,
    /// returns the observed latencies.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies_tokio<F>(self, f: impl FnOnce() -> F) -> Timings
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(f());
        })
    }

    /// Executes the instrumented function `f`, returning a [`ProbedTrace`] that allows partial latencies to be
    /// reported before `f` completes.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies_probed(self, f: impl FnOnce() + Send + 'static) -> ProbedTrace {
        let ltp = LatencyTracePriv::initialized(self.0);
        let pt = ProbedTrace::new(ltp);
        let jh = thread::spawn(f);
        pt.set_join_handle(jh);
        pt
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; returns a [`ProbedTrace`]
    /// that allows partial latencies to be reported before `f` completes.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies_probed_tokio<F>(
        self,
        f: impl FnOnce() -> F + Send + 'static,
    ) -> ProbedTrace
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies_probed(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(f());
        })
    }
}
