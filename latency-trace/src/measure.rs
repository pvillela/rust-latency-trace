//! Latency measurement methods and other public impls.

use crate::{default_span_grouper, Latencies, LatencyTrace, LatencyTracePriv, SpanGroup, Timing};
use std::{collections::BTreeMap, future::Future, sync::Arc};
use tracing::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

impl LatencyTrace {
    pub fn new() -> Self {
        Self {
            span_grouper: Arc::new(default_span_grouper),
            hist_high: 20 * 1000 * 1000,
            hist_sigfig: 1,
        }
    }

    pub fn with_span_grouper(
        &self,
        span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    ) -> Self {
        Self {
            span_grouper: Arc::new(span_grouper),
            hist_high: self.hist_high,
            hist_sigfig: self.hist_sigfig,
        }
    }

    pub fn with_hist_high(&self, hist_high: u64) -> Self {
        Self {
            span_grouper: self.span_grouper.clone(),
            hist_high,
            hist_sigfig: self.hist_sigfig,
        }
    }

    pub fn with_hist_sigfig(&self, hist_sigfig: u8) -> Self {
        Self {
            span_grouper: self.span_grouper.clone(),
            hist_high: self.hist_high,
            hist_sigfig,
        }
    }

    /// Measures latencies of spans in `f`.
    /// May only be called once per process and will panic if called more than once.
    pub fn measure_latencies(self, f: impl FnOnce() + Send + 'static) -> Latencies {
        let ltp = LatencyTracePriv::new(self);
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

impl Latencies {
    pub fn span_groups(&self) -> &Vec<SpanGroup> {
        &self.span_groups
    }

    pub fn timings(&self) -> &BTreeMap<SpanGroup, Timing> {
        &self.timings
    }

    /// Aggregate timings by sets of [`crate::SpanGroup`]s that have the same value when `f` is applied.
    pub fn aggregate_timings<G>(&self, f: impl Fn(&SpanGroup) -> G) -> BTreeMap<G, Timing>
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Timing> = BTreeMap::new();
        for (k, v) in &self.timings {
            let g = f(k);
            let timing = match res.get_mut(&g) {
                Some(timing) => timing,
                None => {
                    res.insert(g.clone(), Timing::new(self.hist_high, self.hist_sigfig));
                    res.get_mut(&g).unwrap()
                }
            };
            timing.total_time.add(v.total_time()).unwrap();
            timing.active_time.add(v.active_time()).unwrap();
        }
        res
    }
}
