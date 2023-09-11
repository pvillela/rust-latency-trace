use hdrhistogram::Histogram;

#[derive(Debug, Clone)]
pub struct HistogramSummary {
    pub count: u64,
    pub mean: f64,
    pub stdev: f64,
    pub min: u64,
    pub p1: u64,
    pub p5: u64,
    pub p10: u64,
    pub p25: u64,
    pub median: u64,
    pub p75: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
}

impl HistogramSummary {
    pub fn new(hist: &Histogram<u64>) -> Self {
        Self {
            count: hist.len(),
            mean: hist.mean(),
            stdev: hist.stdev(),
            min: hist.value_at_quantile(0.0),
            p1: hist.value_at_quantile(0.01),
            p5: hist.value_at_quantile(0.05),
            p10: hist.value_at_quantile(0.10),
            p25: hist.value_at_quantile(0.25),
            median: hist.value_at_quantile(0.50),
            p75: hist.value_at_quantile(0.75),
            p90: hist.value_at_quantile(0.90),
            p95: hist.value_at_quantile(0.95),
            p99: hist.value_at_quantile(0.99),
            max: hist.value_at_quantile(1.0),
        }
    }
}

pub fn histogram_summary(hist: &Histogram<u64>) -> HistogramSummary {
    HistogramSummary::new(hist)
}
