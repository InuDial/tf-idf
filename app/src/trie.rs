use crate::map::TMapFactory;
use crate::map::{MapInsert, MapQuery};
use std::borrow::Borrow;
use std::sync::LazyLock;

type TrieMapFactory = crate::map::IndexMapFactory<16>;
type TrieNode = Node<TrieMapFactory>;

pub static TRIE: LazyLock<TrieNode> = LazyLock::new(|| {
    let mut root = TrieNode::default();
    for (string, value) in dict_data::DICT_PAIRS {
        root.insert(string.as_bytes().iter(), *value);
    }
    root
});

pub struct Iter<It: Iterator> {
    inner: It,
    last: Option<u8>,
}

impl<It: Iterator> Iter<It>
where
    It::Item: Borrow<u8>,
{
    pub fn new(iter: impl IntoIterator<IntoIter = It>) -> Self {
        Self {
            inner: iter.into_iter().into(),
            last: None,
        }
    }
}

impl<It: Iterator> Iterator for Iter<It>
where
    It::Item: Borrow<u8>,
{
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.last.is_some() {
            return std::mem::replace(&mut self.last, None);
        }
        let c = *self.inner.next()?.borrow();
        let low = c & 0x0F;
        let high = (c >> 4) & 0x0F;
        self.last = Some(high);
        return Some(low);
    }
}

/// Trie implement. Note that partial seek is not supported.
pub struct Node<F: TMapFactory<u8>> {
    value: Option<f64>,
    next: F::MapKind<Box<Self>>,
}

impl<F: TMapFactory<u8>> Default for Node<F>
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

impl<F: TMapFactory<u8>> Node<F> {
    pub fn value(&self) -> Option<f64> {
        self.value
    }
}

impl<F: TMapFactory<u8>> Node<F>
where
    F::MapKind<Box<Self>>: MapInsert<u8, Box<Self>, u8> + Default,
{
    pub fn insert(&mut self, path: impl IntoIterator<Item = impl Borrow<u8>>, value: f64) {
        let mut p = self;
        for c in Iter::new(path) {
            p = p.next.get_mut_or_insert_with(c.borrow(), Box::default);
        }
        p.value = Some(value);
    }

    pub fn seek(&self, path: impl IntoIterator<Item = impl Borrow<u8>>) -> Option<&Self> {
        let mut p = self;
        for c in Iter::new(path) {
            p = p.next.get(c.borrow())?;
        }
        Some(p)
    }
}

impl<F: TMapFactory<u8>> Node<F>
where
    F::MapKind<Box<Self>>: MapInsert<u8, Box<Self>, u8> + Default,
{
    pub fn seek_char(&self, path: char) -> Option<&Self> {
        let mut buf = [0u8; 4];
        let bytes = path.encode_utf8(&mut buf).as_bytes();
        self.seek(bytes)
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
