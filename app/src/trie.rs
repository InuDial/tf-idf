use std::sync::LazyLock;

pub struct Node {
    value: Option<f64>,
    next: [Option<Box<Node>>; 256],
}

impl Default for Node {
    fn default() -> Self {
        Self {
            value: None,
            next: std::array::from_fn(|_| None),
        }
    }
}

impl Node {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn value(&self) -> Option<f64> {
        self.value
    }

    pub fn insert(&mut self, path: &[u8], value: f64) {
        match path.first() {
            None => {
                self.value = Some(value);
            }
            Some(c) => {
                self.next[*c as usize]
                    .get_or_insert_with(Box::default)
                    .insert(&path[1..], value);
            }
        }
    }

    pub fn seek(&self, path: &[u8]) -> Option<&Node> {
        match path.first() {
            Some(c) => self.next[*c as usize].as_ref()?.seek(&path[1..]),
            None => Some(self),
        }
    }

    pub fn seek_char(&self, path: char) -> Option<&Node> {
        let mut buf = [0u8; 4];
        let bytes = path.encode_utf8(&mut buf).as_bytes();
        self.seek(bytes)
    }
}

pub static TRIE: LazyLock<Node> = LazyLock::new(|| {
    let mut root = Node::new();
    for (string, value) in dict_data::DICT_PAIRS {
        root.insert(string.as_bytes(), *value);
    }
    root
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_insert_and_seek() {
        let mut node = Node::new();
        node.insert(b"hello", 1.0);
        let found = node.seek(b"hello").unwrap();
        assert_eq!(found.value(), Some(1.0));
    }

    #[test]
    fn node_seek_partial() {
        let mut node = Node::new();
        node.insert(b"hello", 1.0);
        let found = node.seek(b"hel");
        assert!(found.is_some());
        assert_eq!(found.unwrap().value(), None);
    }

    #[test]
    fn node_seek_missing() {
        let mut node = Node::new();
        node.insert(b"hello", 1.0);
        assert!(node.seek(b"world").is_none());
    }

    #[test]
    fn node_overwrite_value() {
        let mut node = Node::new();
        node.insert(b"a", 1.0);
        node.insert(b"a", 2.0);
        assert_eq!(node.seek(b"a").unwrap().value(), Some(2.0));
    }

    #[test]
    fn node_prefix_shared() {
        let mut node = Node::new();
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

    #[test]
    fn seek_char_matches_seek() {
        let mut node = Node::new();
        node.insert(b"abc", 3.0);
        assert_eq!(
            node.seek_char('a')
                .unwrap()
                .seek_char('b')
                .unwrap()
                .seek_char('c')
                .unwrap()
                .value(),
            Some(3.0)
        );
    }
}
