#![allow(dead_code)]

use std::borrow::Borrow;

mod hash_map;
mod index_map;
mod small_map;
#[allow(unused_imports)]
pub use hash_map::*;
#[allow(unused_imports)]
pub use index_map::*;
#[allow(unused_imports)]
pub use small_map::*;

pub trait Map<K, V> {
    fn insert(&mut self, k: K, v: V) -> Option<V>;
}

pub trait MapQuery<K, V, Q>: Map<K, V>
where
    K: Borrow<Q>,
{
    fn get(&self, q: &Q) -> Option<&V>;
}

pub trait MapInsert<K, V, Q>: MapQuery<K, V, Q>
where
    K: Borrow<Q>,
    Q: ToOwned<Owned = K>,
{
    fn get_mut_or_insert_with(&mut self, q: &Q, f: impl FnOnce() -> V) -> &mut V;
}

pub trait MapFactory {
    type MapKind<K, V>;
}

pub trait TMapFactory<K> {
    type MapKind<V>;
}

impl<T, K> TMapFactory<K> for T
where
    T: MapFactory,
{
    type MapKind<V> = <T as MapFactory>::MapKind<K, V>;
}
