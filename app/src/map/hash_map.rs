use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use super::{Init, Map, MapFactory, MapInsert, MapQuery};

impl<K, V> Init for HashMap<K, V> {
    fn new() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V> Map<K, V> for HashMap<K, V> {
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        HashMap::insert(self, k, v)
    }
}

impl<K: Hash + Eq, V, Q: Hash + Eq> MapQuery<K, V, Q> for HashMap<K, V>
where
    K: Borrow<Q>,
{
    fn get(&self, q: &Q) -> Option<&V> {
        HashMap::get(self, q)
    }
}

impl<K: Hash + Eq, V, Q: Hash + Eq> MapInsert<K, V, Q> for HashMap<K, V>
where
    K: Borrow<Q>,
    Q: ToOwned<Owned = K>,
{
    fn get_mut_or_insert_with(&mut self, q: &Q, f: impl FnOnce() -> V) -> &mut V {
        self.entry(q.to_owned()).or_insert_with(f)
    }
}

pub struct HashMapFactory;

impl MapFactory for HashMapFactory {
    type MapKind<K, V> = HashMap<K, V>;
}
