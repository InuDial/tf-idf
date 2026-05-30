use std::borrow::Borrow;

use super::{Init, Map, MapFactory, MapInsert, MapQuery};

pub struct SmallMap<K, V>(Vec<(K, V)>);

impl<K, V> Default for SmallMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K, V> Init for SmallMap<K, V> {
    fn new() -> Self {
        Self::default()
    }
}

impl<K: Ord, V> Map<K, V> for SmallMap<K, V> {
    fn insert(&mut self, k: K, value: V) -> Option<V> {
        match self.0.binary_search_by(|cur| cur.0.cmp(&k)) {
            Ok(index) => Some(std::mem::replace(&mut (self.0)[index].1, value)),
            Err(index) => {
                self.0.insert(index, (k, value));
                None
            }
        }
    }
}

impl<K: Ord, V, Q: Ord> MapQuery<K, V, Q> for SmallMap<K, V>
where
    K: Borrow<Q>,
{
    fn get(&self, q: &Q) -> Option<&V> {
        let index = &self.0.binary_search_by(|cur| cur.0.borrow().cmp(q)).ok()?;
        Some(&self.0[*index].1)
    }
}

impl<K: Ord, V, Q: Ord> MapInsert<K, V, Q> for SmallMap<K, V>
where
    K: Borrow<Q>,
    Q: ToOwned<Owned = K>,
{
    fn get_mut_or_insert_with(&mut self, q: &Q, f: impl FnOnce() -> V) -> &mut V {
        match self.0.binary_search_by(|cur| cur.0.borrow().cmp(q)) {
            Ok(index) => &mut (self.0)[index].1,
            Err(index) => {
                self.0.insert(index, (q.to_owned(), f()));
                &mut self.0[index].1
            }
        }
    }
}

pub struct SmallMapFactory;

impl MapFactory for SmallMapFactory {
    type MapKind<K, V> = SmallMap<K, V>;
}
