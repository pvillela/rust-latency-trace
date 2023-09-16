use crate::{histogram_summary, SpanGroup, Wrapper};
use hdrhistogram::Histogram;
use std::{borrow::Borrow, collections::BTreeMap};

//=================
// BTreeMapExt

/// Wrapper of [BTreeMap] that provides an additional [`map_values`](Self::map_values) method.
/// As this type [Deref](std::ops::Deref)s to [BTreeMap] and implements [IntoIterator]s with the same results as
/// those of [BTreeMap], it supports `for` loops and all immutable [BTreeMap] methods.
pub type BTreeMapExt<K, V> = Wrapper<BTreeMap<K, V>>;

impl<K, V> IntoIterator for BTreeMapExt<K, V> {
    type Item = (K, V);
    type IntoIter = <BTreeMap<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a BTreeMapExt<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = <&'a BTreeMap<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut BTreeMapExt<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = <&'a mut BTreeMap<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<K, V> BTreeMapExt<K, V> {
    /// Returns a new [BTreeMapExt] with the same keys as `self` and values corresponding to the
    /// invocation of function `f` on the original values.
    pub fn map_values<'a, V1, RV>(&'a self, f: impl Fn(&RV) -> V1) -> BTreeMapExt<K, V1>
    where
        K: Ord + Clone,
        V: 'static,
        V: Borrow<RV>,
    {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), f(v.borrow())))
            .collect::<BTreeMap<_, _>>()
            .into()
    }
}

fn _type_check(b: BTreeMapExt<SpanGroup, Histogram<u64>>) {
    _ = b.map_values(histogram_summary);
}
