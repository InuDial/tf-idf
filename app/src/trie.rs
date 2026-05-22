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
