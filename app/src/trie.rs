use num_traits::AsPrimitive;

use crate::map::TMapFactory;
use crate::map::{MapInsert, MapQuery};
use std::borrow::Borrow;
use std::sync::LazyLock;

type TrieMapFactory = crate::map::IndexMapFactory<16>;
type TrieNode = Node<u8, TrieMapFactory>;

pub static TRIE: LazyLock<TrieNode> = LazyLock::new(|| {
    let mut root = TrieNode::default();
    for (string, value) in dict_data::DICT_PAIRS {
        root.insert(string.as_bytes().iter(), *value);
    }
    root
});

pub struct Iter<'a> {
    inner: &'a [u8],
    first: bool,
}

impl<'a> Iter<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self {
            inner: slice,
            first: false,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        self.first ^= true;
        if !self.first {
            // actually first

            let first = *self.inner.first()?;
            Some(first & 0x0F)
        } else {
            let (c, rest) = self.inner.split_first()?;
            self.inner = rest;
            Some((*c >> 4) & 0x0F)
        }
    }
}

/// Trie implement. Note that partial seek is not supported.
pub struct Node<Idx: AsPrimitive<usize>, F: TMapFactory<Idx>> {
    value: Option<f64>,
    next: F::MapKind<Box<Self>>,
}

impl<Idx: AsPrimitive<usize>, F: TMapFactory<Idx>> Default for Node<Idx, F>
where
    F::MapKind<Box<Self>>: Default,
{
    fn default() -> Self {
        Self {
            value: None,
            next: Default::default(),
        }
    }
}

impl<Idx: AsPrimitive<usize>, F: TMapFactory<Idx>> Node<Idx, F> {
    pub fn value(&self) -> Option<f64> {
        self.value
    }
}

impl<Idx: AsPrimitive<usize>, F: TMapFactory<Idx>> Node<Idx, F>
where
    F::MapKind<Box<Self>>: MapInsert<Idx, Box<Self>, Idx> + Default,
{
    pub fn insert(&mut self, path: impl IntoIterator<Item = impl Borrow<Idx>>, value: f64) {
        let mut path = path.into_iter();
        let mut p = self;
        while let Some(c) = path.next() {
            p = p.next.get_mut_or_insert_with(c.borrow(), Box::default);
        }
        p.value = Some(value);
    }

    pub fn seek(&self, path: impl IntoIterator<Item = impl Borrow<Idx>>) -> Option<&Self> {
        let mut path = path.into_iter();
        let mut p = self;
        while let Some(c) = path.next() {
            p = p.next.get(c.borrow())?;
        }
        Some(p)
    }
}

impl<Idx: AsPrimitive<usize>, F: TMapFactory<Idx>> Node<Idx, F>
where
    u8: AsPrimitive<Idx>,
    F::MapKind<Box<Self>>: MapInsert<Idx, Box<Self>, Idx> + Default,
{
    pub fn seek_char(&self, path: char) -> Option<&Self> {
        let mut buf = [0u8; 4];
        let bytes = path.encode_utf8(&mut buf).as_bytes();
        self.seek(Iter::new(&bytes).map(|c| c.as_()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_insert_and_seek() {
        let mut node = TrieNode::default();
        node.insert(b"hello", 1.0);
        let found = node.seek(b"hello").unwrap();
        assert_eq!(found.value(), Some(1.0));
    }

    #[test]
    fn node_seek_missing() {
        let mut node = TrieNode::default();
        node.insert(b"hello", 1.0);
        assert!(node.seek(b"world").is_none());
    }

    #[test]
    fn node_overwrite_value() {
        let mut node = TrieNode::default();
        node.insert(b"a", 1.0);
        node.insert(b"a", 2.0);
        assert_eq!(node.seek(b"a").unwrap().value(), Some(2.0));
    }

    #[test]
    fn node_prefix_shared() {
        let mut node = TrieNode::default();
        node.insert(b"ab", 1.0);
        node.insert(b"ac", 2.0);
        assert_eq!(node.seek(b"ab").unwrap().value(), Some(1.0));
        assert_eq!(node.seek(b"ac").unwrap().value(), Some(2.0));
    }

    #[test]
    fn static_trie_exists() {
        // 验证静态 TRIE 已构建，且至少存入了 dict 中的词条
        assert!(TRIE.seek_char('的').is_some());
    }
}
