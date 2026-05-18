use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use rkyv::Archive;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Fallible;
use rkyv::ser::{Allocator, Writer};
use rkyv::with::{ArchiveWith, DeserializeWith, Map, SerializeWith};
use rkyv::{Deserialize, Serialize};

use crate::{Term, tokenize};

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

pub fn term_freq(content: String) -> Vec<(String, f64)> {
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
    pub tf_idf: HashMap<Term, Vec<(u64, f64)>>,
}

impl Library {
    pub fn new(
        names: Vec<PathBuf>,
        metas: impl IntoIterator<Item = impl IntoIterator<Item = (String, f64)>>,
    ) -> Self {
        // let n = articles.len() as f64;
        let n = names.len();
        let mut occurrences: HashMap<Term, Vec<(u64, f64)>> = HashMap::new();

        for (id, article) in metas.into_iter().enumerate() {
            for (term, freq) in article {
                let upper = term.to_ascii_uppercase();
                occurrences
                    .entry(upper.as_str().try_into().unwrap())
                    .or_default()
                    .push((id as u64, freq));
            }
        }

        for occ in occurrences.values_mut() {
            let freq_sum: f64 = occ.iter().map(|x| x.1).sum();
            let idf = (n as f64 / occ.len() as f64).ln();

            for (_i, f) in occ {
                *f *= idf / freq_sum;
            }
        }

        Self {
            articles: names,
            tf_idf: occurrences,
        }
    }
}

pub unsafe fn path_from_bytes(field: &rkyv::vec::ArchivedVec<u8>) -> &Path {
    let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(field) };
    Path::new(os_str)
}
