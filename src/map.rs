use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

fn map_entries<'a, K, V, K1, V1>(
    iter: impl Iterator<Item = (&'a K, &'a V)> + 'a,
    mut f: impl FnMut(&K, &V) -> (K1, V1) + 'a,
) -> impl Iterator<Item = (K1, V1)> + 'a
where
    K: 'a,
    V: 'a,
{
    iter.map(move |(k, v)| f(k, v))
}

fn map_values<'a, K, V, V1>(
    iter: impl Iterator<Item = (&'a K, &'a V)> + 'a,
    mut f: impl FnMut(&V) -> V1 + 'a,
) -> impl Iterator<Item = (K, V1)> + 'a
where
    K: Clone + 'a,
    V: 'a,
{
    iter.map(move |(k, v)| (k.clone(), f(v)))
}

fn filter<'a, K, V>(
    iter: impl Iterator<Item = (&'a K, &'a V)> + 'a,
    mut f: impl FnMut(&K, &V) -> bool + 'a,
) -> impl Iterator<Item = (K, V)> + 'a
where
    K: 'a + Clone,
    V: 'a + Clone,
{
    iter.filter(move |(k, v)| f(k, v))
        .map(|(k, v)| (k.clone(), v.clone()))
}

pub struct BTreeMapExt<K, V>(pub BTreeMap<K, V>);

// impl<K, V> From<BTreeMap<K, V>> for BTreeMapExt<K, V> {
//     fn from(value: BTreeMap<K, V>) -> Self {
//         BTreeMapExt(value)
//     }
// }

// impl<K, V> Into<BTreeMap<K, V>> for BTreeMapExt<K, V> {
//     fn into(self) -> BTreeMap<K, V> {
//         self.0
//     }
// }

impl<K, V> BTreeMapExt<K, V> {
    pub fn map_to_btree_map<K1, V1>(&self, f: impl FnMut(&K, &V) -> (K1, V1)) -> BTreeMap<K1, V1>
    where
        K1: Ord,
    {
        map_entries(self.0.iter(), f).collect::<BTreeMap<_, _>>()
        // .into()
    }

    pub fn map_to_hash_map<K1, V1>(&self, f: impl FnMut(&K, &V) -> (K1, V1)) -> HashMap<K1, V1>
    where
        K1: Eq + Hash,
    {
        map_entries(self.0.iter(), f).collect::<HashMap<_, _>>()
        // .into()
    }

    pub fn map_values<V1>(&self, f: impl FnMut(&V) -> V1) -> BTreeMap<K, V1>
    where
        K: Ord + Clone,
    {
        map_values(self.0.iter(), f).collect::<BTreeMap<_, _>>()
        // .into()
    }

    pub fn filter(&self, f: impl FnMut(&K, &V) -> bool) -> BTreeMap<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        filter(self.0.iter(), f).collect::<BTreeMap<_, _>>()
        // .into()
    }
}

pub struct HashMapExt<'a, K, V>(pub &'a HashMap<K, V>);

// impl<'a, K, V> From<&'a HashMap<K, V>> for HashMapExt<'a, K, V> {
//     fn from(value: &'a HashMap<K, V>) -> Self {
//         HashMapExt(value)
//     }
// }

// impl<'a, K, V> Into<&'a HashMap<K, V>> for HashMapExt<'a, K, V> {
//     fn into(self) -> &'a HashMap<K, V> {
//         self.0
//     }
// }

impl<'a, K, V> HashMapExt<'a, K, V> {
    pub fn map_to_btree_map<K1, V1>(&self, f: impl FnMut(&K, &V) -> (K1, V1)) -> BTreeMap<K1, V1>
    where
        K1: Ord,
    {
        map_entries(self.0.iter(), f).collect::<BTreeMap<_, _>>()
        // .into()
    }

    pub fn map_to_hash_map<K1, V1>(&self, f: impl FnMut(&K, &V) -> (K1, V1)) -> HashMap<K1, V1>
    where
        K1: Eq + Hash,
    {
        map_entries(self.0.iter(), f).collect::<HashMap<_, _>>()
    }

    pub fn map_values<V1>(&self, f: impl FnMut(&V) -> V1) -> HashMap<K, V1>
    where
        K: Eq + Hash + Clone,
    {
        map_values(self.0.iter(), f)
            .collect::<HashMap<_, _>>()
            .into()
    }

    pub fn filter(&self, f: impl FnMut(&K, &V) -> bool) -> HashMap<K, V>
    where
        K: Eq + Hash + Clone,
        V: Clone,
    {
        filter(self.0.iter(), f).collect::<HashMap<_, _>>()
        // .into()
    }
}
