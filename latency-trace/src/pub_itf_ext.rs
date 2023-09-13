//! Main public interface extension to the core library, including latency measurement methods.

use crate::{default_span_grouper, Latencies, LatencyTraceCfg, LatencyTracePriv};
use std::{future::Future, sync::Arc};
use tracing::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

/// Provides the ability to measure latencies for code (both sync and async) instrumented with the
/// [`tracing`](https://crates.io/crates/tracing) library.
///
/// Its configuration encapsulates a span grouper function
/// (`impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static`)
/// to define [SpanGroup](crate::SpanGroup)s, as well as the histogram configuration parameters
/// [hdrhistogram::Histogram::high] and [hdrhistogram::Histogram::sigfig].
pub struct LatencyTrace(LatencyTraceCfg);

impl LatencyTrace {
    /// Instantiates a [LatencyTrace] with default configuration. The defaults are:
    /// - Grouping of spans using the [`default_span_grouper`], which simply groups by the span's
    /// callsite information (see [`CallsiteInfo`](crate::CallsiteInfo), which distills [tracing::Callsite]).
    /// - Histograms use a `hist_high` of `20,000,000` (20 seconds) and a `hist_sigfig` of 2.
    ///
    /// See [hdrhistogram::Histogram::high] and [hdrhistogram::Histogram::sigfig] for an explanation of these
    /// histogram configuration parameters.
    ///
    /// Once an instance with the default configuration is created, it can be modified with other methods provided
    /// by this struct.
    pub fn new() -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: Arc::new(default_span_grouper),
            hist_high: 20 * 1000 * 1000,
            hist_sigfig: 2,
        };
        Self(cfg)
    }

    /// Creates a new [`LatencyTrace`] configured the same as `self` but with the given `span_grouper`.
    pub fn with_span_grouper(
        &self,
        span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    ) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: Arc::new(span_grouper),
            hist_high: self.0.hist_high,
            hist_sigfig: self.0.hist_sigfig,
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
    /// Will panic if this function or [Self::measure_latencies_tokio] have been previously called in the same process.
    pub fn measure_latencies(self, f: impl FnOnce() + Send + 'static) -> Latencies {
        let ltp = LatencyTracePriv::new(self.0);
        Registry::default().with(ltp.clone()).init();
        f();
        ltp.control.ensure_tls_dropped();
        ltp.generate_latencies()
    }

    /// Measures latencies of spans in async function `f` running on the [tokio] runtime.
    /// Will panic if this function or [Self::measure_latencies] have been previously called in the same process.
    pub fn measure_latencies_tokio<F>(self, f: impl FnOnce() -> F + Send + 'static) -> Latencies
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
}
