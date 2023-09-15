//! Wrapper for [`BTreeMap`] that provides additional methods.

use std::{collections::BTreeMap, ops::Deref};

/// Wrapper of [BTreeMap] that provides an additional [`map_values`](Self::map_values) method.
/// As this type [Deref]s to [BTreeMap] and implements [IntoIterator]s with the same results as
/// those of [BTreeMap], it supports `for` loops and all immutable [BTreeMap] methods.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BTreeMapExt<K, V>(pub BTreeMap<K, V>);

impl<K, V> From<BTreeMap<K, V>> for BTreeMapExt<K, V> {
    fn from(value: BTreeMap<K, V>) -> Self {
        Self(value)
    }
}

impl<K, V> Deref for BTreeMapExt<K, V> {
    type Target = BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    /// Returns a new [BTreeMapExt] with the same keys as `self` but with values corresponding to the
    /// invocation of function `f` on the original values.
    pub fn map_values<V1>(&self, mut f: impl FnMut(&V) -> V1) -> BTreeMapExt<K, V1>
    where
        K: Ord + Clone,
    {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), f(v)))
            .collect::<BTreeMap<_, _>>()
            .into()
    }
}
