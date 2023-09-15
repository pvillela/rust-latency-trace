use crate::{BTreeMapExt, CallsiteInfo, SpanGroup};
use hdrhistogram::Histogram;
use std::{collections::BTreeMap, sync::Arc};

/// Type of timing information recorded for span groups.
pub type Timings = BTreeMapExt<SpanGroup, Histogram<u64>>;

impl Timings {
    /// Combines histograms of span group according to sets of span groups that yield the same value when `f`
    /// is applied. The values resulting from applying `f` to span groups are called ***aggregate key***s and
    /// the sets of span groups corresponding to each *aggregate key* are called ***aggregates***.
    ///
    /// An aggregation is consistent if and only if, for each *aggregate*, all the span groups in the *aggregate*
    /// have the same callsite.
    ///
    /// This function returns a pair with the following components:
    /// - a [BTreeMapExt] that associates each *aggregate key* to its aggregated histogram;
    /// - a boolean that is `true` if the aggregation is consistent, `false` otherwise.
    pub fn aggregate<G>(
        &self,
        f: impl Fn(&SpanGroup) -> G,
    ) -> (BTreeMapExt<G, Histogram<u64>>, bool)
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Histogram<u64>> = BTreeMap::new();
        let mut aggregates: BTreeMap<G, Arc<CallsiteInfo>> = BTreeMap::new();
        let mut aggregates_are_consistent = true;
        for (k, v) in self {
            // Construct aggregation map.
            let g = f(k);
            let hist = match res.get_mut(&g) {
                Some(hist) => hist,
                None => {
                    res.insert(g.clone(), Histogram::new_from(v));
                    res.get_mut(&g).unwrap()
                }
            };
            hist.add(v).unwrap();

            // Check aggregation consistency.
            if aggregates_are_consistent {
                let callsite = match aggregates.get(&g) {
                    Some(callsite) => callsite,
                    None => {
                        aggregates.insert(g.clone(), k.callsite.clone());
                        aggregates.get(&g).unwrap()
                    }
                };
                aggregates_are_consistent = callsite.as_ref() == k.callsite();
            }
        }

        (res.into(), aggregates_are_consistent)
    }
}
