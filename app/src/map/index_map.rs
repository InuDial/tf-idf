use std::borrow::Borrow;

use super::{Init, Map, MapInsert, MapQuery, TMapFactory};

macro_rules! impl_init {
    ($key: ty) => {
        impl<V> Init for [Option<V>; <$key>::MAX as usize + 1] {
            fn new() -> Self {
                std::array::from_fn(|_| None)
            }
        }
    };
}

impl_init!(u8);
impl_init!(u16);

/// Implement [`Map`], [`MapQuery`], and [`MapInsert`] on `[Option<V>; MAX+1]`
/// for the given integer key type.
///
/// # Example
///
/// ```ignore
/// use app::map::{Map, MapQuery, MapInsert};
///
/// let mut m: [Option<&str>; 256] = [None; 256];
/// m.insert(42u8, "hello");
/// assert_eq!(m.get(&42u8), Some(&"hello"));
/// ```
macro_rules! impl_index_map {
    ($key:ty) => {
        impl<V> Map<$key, V> for [Option<V>; <$key>::MAX as usize + 1] {
            fn insert(&mut self, k: $key, v: V) -> Option<V> {
                std::mem::replace(&mut self[k as usize], Some(v))
            }
        }

        impl<V> MapQuery<$key, V, $key> for [Option<V>; <$key>::MAX as usize + 1]
        where
            $key: Borrow<$key>,
        {
            fn get(&self, q: &$key) -> Option<&V> {
                self[*q as usize].as_ref()
            }
        }

        impl<V> MapInsert<$key, V, $key> for [Option<V>; <$key>::MAX as usize + 1]
        where
            $key: Borrow<$key>,
            $key: ToOwned<Owned = $key>,
        {
            fn get_mut_or_insert_with(&mut self, q: &$key, f: impl FnOnce() -> V) -> &mut V {
                self[*q as usize].get_or_insert_with(f)
            }
        }
    };
}

impl_index_map!(u8);
impl_index_map!(u16);

/// Factory that produces `[Option<V>; I::MAX + 1]` arrays for integer keys.
pub struct IndexMapFactory;

macro_rules! impl_tmap_factory {
    ($key:ty) => {
        impl TMapFactory<$key> for IndexMapFactory {
            type MapKind<V> = [Option<V>; <$key>::MAX as usize + 1];
        }
    };
}

impl_tmap_factory!(u8);
impl_tmap_factory!(u16);
