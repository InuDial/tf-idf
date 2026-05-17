use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use rkyv::Archive;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Fallible;
use rkyv::ser::{Allocator, Writer};
use rkyv::with::{ArchiveWith, DeserializeWith, Map, SerializeWith};
use rkyv::{Deserialize, Serialize};

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
        let os_str = unsafe { OsString::from_encoded_bytes_unchecked(field.as_slice().to_vec()) };
        Ok(PathBuf::from(os_str))
    }
}

#[derive(Archive, Serialize, Deserialize)]
#[repr(C)]
pub struct Metadata {
    #[rkyv(with = PathBytes)]
    pub path: PathBuf,
    pub content: String,
    pub split: Vec<usize>,
}

impl Metadata {
    pub fn term_freq(&self) -> HashMap<String, f64> {
        let inv_term_count = 1f64 / (self.split.len() + 1) as f64;

        let mut ret = HashMap::new();
        let mut l = 0;

        let mut view = &self.content[..];
        for &r in &self.split {
            let entry = ret.entry(view[..r - l].to_string());
            view = &view[r - l..];
            *entry.or_insert(0.) += inv_term_count;
            l = r;
        }
        ret
    }
}

#[derive(Archive, Serialize, Deserialize, CheckBytes)]
#[repr(C)]
pub struct Library {
    #[rkyv(with = Map<PathBytes>)]
    pub articles: Vec<PathBuf>,
    /// term -> [(article_id, value)], value = reletive_freq * idf
    pub tf_idf: HashMap<String, Vec<(u64, f64)>>,
}

impl Library {
    pub fn new(articles: impl IntoIterator<Item = Metadata>) -> Self {
        let articles: Vec<_> = articles.into_iter().collect();
        let n = articles.len() as f64;
        let mut occurrences: HashMap<String, Vec<(u64, f64)>> = HashMap::new();

        for (id, article) in articles.iter().enumerate() {
            let tf_map = article.term_freq();

            for (term, freq) in tf_map {
                occurrences
                    .entry(term.to_ascii_uppercase())
                    .or_default()
                    .push((id as u64, freq));
            }
        }

        for occ in occurrences.values_mut() {
            let freq_sum: f64 = occ.iter().map(|x| x.1).sum();
            let idf = (n / occ.len() as f64).ln();

            for (_i, f) in occ {
                *f *= idf / freq_sum;
            }
        }

        Self {
            articles: articles.into_iter().map(|meta| meta.path).collect(),
            tf_idf: occurrences,
        }
    }
}

pub unsafe fn path_from_bytes(field: &rkyv::vec::ArchivedVec<u8>) -> &Path {
    let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(field) };
    Path::new(os_str)
}