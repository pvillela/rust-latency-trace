use crate::{BTreeMapExt, SpanGroup};
use hdrhistogram::Histogram;
use std::collections::BTreeMap;

/// Type of timing information recorded for span groups.
pub type Timings = BTreeMapExt<SpanGroup, Histogram<u64>>;

impl Timings {
    /// Aggregates span group histograms by sets of span groups that yield the same value when `f` is applied.
    pub fn aggregate<G>(&self, f: impl Fn(&SpanGroup) -> G) -> BTreeMapExt<G, Histogram<u64>>
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Histogram<u64>> = BTreeMap::new();
        for (k, v) in self {
            let g = f(k);
            let hist = match res.get_mut(&g) {
                Some(hist) => hist,
                None => {
                    res.insert(g.clone(), Histogram::new_from(v));
                    res.get_mut(&g).unwrap()
                }
            };
            hist.add(v).unwrap();
        }
        res.into()
    }
}
