use std::{borrow::Borrow, collections::BTreeMap};

/// Extends [BTreeMap] to provide an additional [`map_values`](Self::map_values) method.
pub trait BTreeMapExt<K, V> {
    fn map_values<V1, BV>(&self, f: impl FnMut(&BV) -> V1) -> BTreeMap<K, V1>
    where
        K: Ord + Clone,
        V: Borrow<BV>;
}

impl<K, V> BTreeMapExt<K, V> for BTreeMap<K, V> {
    /// Returns a new [BTreeMap] with the same keys as `self` and values corresponding to the
    /// invocation of function `f` on the original values.
    fn map_values<V1, BV>(&self, mut f: impl FnMut(&BV) -> V1) -> BTreeMap<K, V1>
    where
        K: Ord + Clone,
        V: Borrow<BV>,
    {
        self.iter()
            .map(|(k, v)| (k.clone(), f(v.borrow())))
            .collect::<BTreeMap<_, _>>()
    }
}
