//! Main public interface extension to the core library, including latency measurement methods.

use crate::{default_span_grouper, Latencies, LatencyTraceCfg, LatencyTracePriv};
use std::{future::Future, sync::Arc};
use tracing::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

pub struct LatencyTrace(LatencyTraceCfg);

impl LatencyTrace {
    pub fn new() -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: Arc::new(default_span_grouper),
            hist_high: 20 * 1000 * 1000,
            hist_sigfig: 1,
        };
        Self(cfg)
    }

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

    pub fn with_hist_high(&self, hist_high: u64) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: self.0.span_grouper.clone(),
            hist_high,
            hist_sigfig: self.0.hist_sigfig,
        };
        Self(cfg)
    }

    pub fn with_hist_sigfig(&self, hist_sigfig: u8) -> Self {
        let cfg = LatencyTraceCfg {
            span_grouper: self.0.span_grouper.clone(),
            hist_high: self.0.hist_high,
            hist_sigfig,
        };
        Self(cfg)
    }

    /// Measures latencies of spans in `f`.
    /// May only be called once per process and will panic if called more than once.
    pub fn measure_latencies(self, f: impl FnOnce() + Send + 'static) -> Latencies {
        let ltp = LatencyTracePriv::new(self.0);
        Registry::default().with(ltp.clone()).init();
        f();
        ltp.control.ensure_tls_dropped();
        ltp.generate_latencies()
    }

    /// Measures latencies of spans in async function `f` running on the [tokio] runtime.
    /// May only be called once per process and will panic if called more than once.
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
