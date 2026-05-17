// pub static DICT: phf::Map<&'static str, f64> =
//     include!(concat!(env!("OUT_DIR"), "/generated_dict.rs"));

#[allow(clippy::approx_constant)]
pub static DICT_PAIRS: &[(&str, f64)] = &include!(concat!(env!("OUT_DIR"), "/trie.rs"));

