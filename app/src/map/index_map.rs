use super::{Map, MapInsert, MapQuery, TMapFactory};
use num_traits::AsPrimitive;

pub struct Slice<T, const N: usize>([Option<T>; N]);

impl<V, const N: usize> Default for Slice<V, N> {
    fn default() -> Self {
        let ret = std::array::from_fn(|_| None);
        Self(ret)
    }
}

impl<V, const N: usize> std::ops::Index<usize> for Slice<V, N> {
    type Output = Option<V>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T, const N: usize> std::ops::IndexMut<usize> for Slice<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<K: AsPrimitive<usize>, V, const N: usize> Map<K, V> for Slice<V, N> {
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self[k.as_()].replace(v)
    }
}

impl<K: AsPrimitive<usize>, V, const N: usize> MapQuery<K, V, K> for Slice<V, N> {
    fn get(&self, q: &K) -> Option<&V> {
        self[q.as_()].as_ref()
    }
}

impl<K: AsPrimitive<usize>, V, const N: usize> MapInsert<K, V, K> for Slice<V, N> {
    fn get_mut_or_insert_with(&mut self, q: &K, f: impl FnOnce() -> V) -> &mut V {
        self[q.as_()].get_or_insert_with(f)
    }
}

pub struct IndexMapFactory<const N: usize>;

impl<K: AsPrimitive<usize>, const N: usize> TMapFactory<K> for IndexMapFactory<N> {
    type MapKind<V> = Slice<V, N>;
}
