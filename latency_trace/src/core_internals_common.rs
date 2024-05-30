use hdrhistogram::Histogram;
use std::sync::Arc;
use tracing::span::Attributes;

//=================
// Timing

/// Alias of [`Histogram<u64>`].
pub type Timing = Histogram<u64>;

//=================
// LatencyTraceCfg

//=================
// SpanGrouper

/// Internal type of span groupers.
pub(crate) type SpanGrouper =
    Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>;

/// Configuration information used by both [`LatencyTracePriv`] and [`LatencyTrace`](super::LatencyTrace).
pub(crate) struct LatencyTraceCfg {
    pub(crate) span_grouper: SpanGrouper,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}
