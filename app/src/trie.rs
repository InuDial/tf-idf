use crate::map::{Init, TMapFactory};
use crate::map::{MapInsert, MapQuery};
use std::sync::LazyLock;

type TrieMapFactory = crate::map::IndexMapFactory;
type TrieNode = Node<TrieMapFactory>;
type IndexType = u8;

pub static TRIE: LazyLock<TrieNode> = LazyLock::new(|| {
    let mut root = TrieNode::default();
    for (string, value) in dict_data::DICT_PAIRS {
        root.insert(string.as_bytes(), *value);
    }
    root
});

/// Take out the first IndexType in `bytes`, fill with 0xff in native endian
fn take_first_index(bytes: &[u8]) -> (IndexType, &[u8]) {
    const N: usize = size_of::<IndexType>();
    match bytes.len() {
        n @ ..=N => {
            let mut buf = [0xffu8; N];
            buf[..n].copy_from_slice(bytes);
            (IndexType::from_ne_bytes(buf), &[])
        }
        _ => {
            let (cur, rest) = bytes.split_at(N);
            let buf = cur.try_into().expect("This should not panic.");
            (IndexType::from_ne_bytes(buf), rest)
        }
    }
}

/// Trie implement. Note that partial seek is not supported.
pub struct Node<F: TMapFactory<IndexType>> {
    value: Option<f64>,
    next: F::MapKind<Box<Self>>,
}

impl<F: TMapFactory<IndexType>> Default for Node<F>
where
    F::MapKind<Box<Self>>: Init,
{
    fn default() -> Self {
        Self {
            value: None,
            next: Init::new(),
        }
    }
}

impl<F: TMapFactory<IndexType>> Node<F> {
    pub fn value(&self) -> Option<f64> {
        self.value
    }
}

impl<F: TMapFactory<IndexType>> Node<F>
where
    F::MapKind<Box<Self>>: MapInsert<IndexType, Box<Self>, IndexType> + Init,
{
    pub fn insert(&mut self, path: &[u8], value: f64) {
        if path.is_empty() {
            self.value = Some(value);
            return;
        }
        let (cur, rest) = take_first_index(path);
        self.next
            .get_mut_or_insert_with(&cur, Box::default)
            .insert(rest, value);
    }

    pub fn seek(&self, path: &[u8]) -> Option<&Self> {
        if path.is_empty() {
            return Some(self);
        }
        let (cur, rest) = take_first_index(path);
        self.next.get(&cur)?.seek(rest)
    }

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
