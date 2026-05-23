#[allow(clippy::approx_constant)]
pub static DICT_PAIRS: &[(&str, f64)] = &include!(concat!(env!("OUT_DIR"), "/trie.rs"));
