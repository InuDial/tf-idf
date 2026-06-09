use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use rkyv::Archive;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Fallible;
use rkyv::ser::{Allocator, Writer};
use rkyv::with::{ArchiveWith, DeserializeWith, Map, SerializeWith};
use rkyv::{Deserialize, Serialize};

use crate::Term;
use crate::lexer::tokenize;

pub struct PathBytes;

impl ArchiveWith<PathBuf> for PathBytes {
    type Archived = rkyv::vec::ArchivedVec<u8>;
    type Resolver = rkyv::vec::VecResolver;

    fn resolve_with(field: &PathBuf, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let bytes = field.as_os_str().as_encoded_bytes();
        rkyv::vec::ArchivedVec::resolve_from_slice(bytes, resolver, out);
    }
}

impl<S: Fallible + Allocator + Writer + ?Sized> SerializeWith<PathBuf, S> for PathBytes {
    fn serialize_with(field: &PathBuf, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let bytes = field.as_os_str().as_encoded_bytes();
        rkyv::vec::ArchivedVec::serialize_from_slice(bytes, serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<rkyv::vec::ArchivedVec<u8>, PathBuf, D> for PathBytes {
    fn deserialize_with(
        field: &rkyv::vec::ArchivedVec<u8>,
        _deserializer: &mut D,
    ) -> Result<PathBuf, D::Error> {
        // SAFETY: should be read from valid, trustable source
        let os_str = unsafe { OsString::from_encoded_bytes_unchecked(field.as_slice().to_vec()) };
        Ok(PathBuf::from(os_str))
    }
}

pub fn term_freq(content: String) -> Vec<(String, f64)> {
    let _tracy = tracy_client::span!("term_freq");
    let split = tokenize(&content);
    let inv_term_count = 1.0 / (split.len() + 1) as f64;

    let mut ret = HashMap::with_capacity(split.len() + 1);
    let mut l = 0;

    let mut view = &content[..];
    for &r in &split {
        let entry = ret.entry(&view[..r - l]);
        *entry.or_insert(0) += 1;
        view = &view[r - l..];
        l = r;
    }
    if !view.is_empty() {
        *ret.entry(view).or_insert(0) += 1;
    }
    ret.into_iter()
        .map(move |(i, v)| (i.to_owned(), v as f64 * inv_term_count))
        .collect()
}

#[derive(Archive, Serialize, Deserialize, CheckBytes)]
#[repr(C)]
pub struct Library {
    #[rkyv(with = Map<PathBytes>)]
    pub articles: Vec<PathBuf>,
    /// term -> [(article_id, value)], value = reletive_freq * idf
    pub tf_idf: HashMap<Term, Vec<(u32, f32)>>,
}

impl Library {
    /// Creates a new [`Library`].
    ///
    /// # Arguments
    ///
    /// * `names` - The path to each document.
    /// * `metas` - The term frequency data for each document, where each inner iterator
    ///   yields `(term, frequency)` pairs.
    pub fn new(
        names: Vec<PathBuf>,
        metas: impl IntoIterator<Item = impl IntoIterator<Item = (String, f64)>>,
    ) -> Self {
        let _tracy = tracy_client::span!("Library::new");
        let n = names.len();

        // Term -> (doc-id, tf-idf)
        let mut occurrences: HashMap<Term, Vec<(u32, f64)>> = HashMap::new();

        for (id, article) in metas.into_iter().enumerate() {
            for (term, freq) in article {
                let upper = term.to_ascii_uppercase();
                occurrences
                    .entry(upper.as_str().try_into().unwrap())
                    .or_default()
                    .push((id as u32, freq));
            }
        }

        let tf_idf: HashMap<_, _> = occurrences
            .into_iter()
            .map(|(term, occ)| {
                // Sum freq of this term in all docs
                let freq_sum: f64 = occ.iter().map(|x| x.1).sum();
                let idf = (n as f64 / occ.len() as f64).ln();

                let values: Vec<_> = occ
                    .into_iter()
                    .map(|(doc, freq)| (doc, (freq * idf / freq_sum) as f32))
                    .collect();

                (term, values)
            })
            .collect();

        Self {
            articles: names,
            tf_idf,
        }
    }
}

// SAFETY: should be valid bytes
pub unsafe fn path_from_bytes(field: &rkyv::vec::ArchivedVec<u8>) -> &Path {
    let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(field) };
    Path::new(os_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_freq_basic() {
        let tf = term_freq("hello world hello".into());
        // "hello" 出现 2 次，"world" 出现 1 次
        let hello = tf.iter().find(|(k, _)| k == "hello").unwrap();
        let world = tf.iter().find(|(k, _)| k == "world").unwrap();
        // hello 的频率应为 world 的两倍
        assert!((hello.1 / world.1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn term_freq_empty() {
        let tf = term_freq(String::new());
        assert!(tf.is_empty());
    }

    #[test]
    fn term_freq_single_term() {
        let tf = term_freq("test".into());
        assert_eq!(tf.len(), 1);
        assert_eq!(tf[0].0, "test");
    }

    #[test]
    fn library_new_empty() {
        let names: Vec<PathBuf> = vec![];
        let metas: Vec<Vec<(String, f64)>> = vec![];
        let lib = Library::new(names, metas);
        assert!(lib.articles.is_empty());
        assert!(lib.tf_idf.is_empty());
    }

    #[test]
    fn library_new_single_document() {
        let names = vec![PathBuf::from("file1.txt")];
        let metas = vec![vec![("hello".into(), 0.5), ("world".into(), 0.5)]];
        let lib = Library::new(names, metas);
        assert_eq!(lib.articles.len(), 1);
        assert_eq!(lib.tf_idf.len(), 2);
        // 单文档时 idf = ln(1/1) = 0，所有 tf-idf 值应为 0
        for entries in lib.tf_idf.values() {
            for &(_, v) in entries {
                assert_eq!(v, 0.0);
            }
        }
    }

    #[test]
    fn library_new_multiple_documents() {
        let names = vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("c.txt"),
        ];
        // "common" 出现在所有文档 → idf = ln(3/3) = 0，权重 0
        // "rare" 只出现在 a.txt → idf = ln(3/1) > 0
        let metas = vec![
            vec![("common".into(), 0.5), ("rare".into(), 0.5)],
            vec![("common".into(), 1.0)],
            vec![("common".into(), 1.0)],
        ];
        let lib = Library::new(names, metas);

        // common 出现在所有文档，idf 为 0
        let common_key: Term = "COMMON".try_into().unwrap();
        for &(_, v) in lib.tf_idf.get(&common_key).unwrap() {
            assert_eq!(v, 0.0);
        }
        // rare 只出现在第一个文档
        let rare_key: Term = "RARE".try_into().unwrap();
        let rare_entries = lib.tf_idf.get(&rare_key).unwrap();
        assert_eq!(rare_entries.len(), 1);
        assert_eq!(rare_entries[0].0, 0);
        assert!(rare_entries[0].1 > 0.0);
    }
}
