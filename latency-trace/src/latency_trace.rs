//! Main public interface extension to the core library, including latency measurement methods.

use crate::{
    default_span_grouper, LatencyTraceCfg, LatencyTracePriv, PausableMode, PausableTrace, Timings,
};
use std::{future::Future, sync::Arc, thread};
use tracing::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

/// Provides the ability to measure latencies for code (both sync and async) instrumented with the
/// [tracing](https://crates.io/crates/tracing) library.
///
/// Its configuration encapsulates a span grouper function
/// (`impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static`)
/// to define [SpanGroup](crate::SpanGroup)s, as well as the histogram configuration parameters
/// [hdrhistogram::Histogram::high] and [hdrhistogram::Histogram::sigfig].
pub struct LatencyTrace(LatencyTraceCfg);

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
    pub fn with_hist_sigfig(&self, hist_sigfig: u8) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: self.0.span_grouper.clone(),
            hist_high: self.0.hist_high,
            hist_sigfig,
        };
        Self(cfg)
    }

    /// Measures latencies of spans in `f`.
    /// Will panic if this function or any of the other `Self::measure_latencies*` functions have been
    /// previously called in the same process.
    pub fn measure_latencies(self, f: impl FnOnce() + Send + 'static) -> Timings {
        let ltp = LatencyTracePriv::new(self.0);
        Registry::default().with(ltp.clone()).init();
        f();
        let acc = ltp.take_acc_timings();
        ltp.reduce_acc_timings(acc)
    }

    /// Measures latencies of spans in async function `f` running on the *tokio* runtime.
    /// Will panic if this function or any of the other `Self::measure_latencies*` functions have been
    /// previously called in the same process.
    pub fn measure_latencies_tokio<F>(self, f: impl FnOnce() -> F + Send + 'static) -> Timings
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    f().await;
                });
        })
    }

    /// Measures latencies of spans in `f`, returning a [`PausableTrace`] that allows measurements to be
    /// paused and reported before `f` completes.
    /// Will panic if this function or any of the other `Self::measure_latencies*` functions have been
    /// previously called in the same process.
    pub fn measure_latencies_pausable(
        self,
        mode: PausableMode,
        f: impl FnOnce() + Send + 'static,
    ) -> PausableTrace {
        let ltp = LatencyTracePriv::new(self.0);
        let pt = PausableTrace::new(ltp, mode);
        Registry::default().with(pt.clone()).init();
        let jh = thread::spawn(f);
        pt.set_join_handle(jh);
        pt
    }

    /// Measures latencies of spans in `f`, returning a [`PausableTrace`] that allows measurements to be
    /// paused and reported before `f` completes.
    /// Will panic if this function or any of the other `Self::measure_latencies*` functions have been
    /// previously called in the same process.
    pub fn measure_latencies_pausable_tokio<F>(
        self,
        mode: PausableMode,
        f: impl FnOnce() -> F + Send + 'static,
    ) -> PausableTrace
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies_pausable(mode, || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    f().await;
                });
        })
    }
}
