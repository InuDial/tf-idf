pub static DICT: phf::Map<&'static str, usize> =
    include!(concat!(env!("OUT_DIR"), "/generated_dict.rs"));
