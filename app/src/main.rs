#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use rayon::prelude::*;

mod data;
mod small_str;
mod trie;

use crate::small_str::ArchivedSmallString;
// use dict_data::DICT;
use crate::trie::TRIE;
const MAX_TERM_LENGTH: usize = 50;
type Term = crate::small_str::SmallString<MAX_TERM_LENGTH>;

use memmap2::Mmap;
use rkyv::access;
use rkyv::tuple::ArchivedTuple2;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::data::{ArchivedLibrary, Library, path_from_bytes, term_freq};

/// Return split points of content
fn tokenize(content: impl AsRef<str>) -> Vec<usize> {
    let content: Vec<_> = content.as_ref().char_indices().collect();
    let n = content.len();
    let mut f = vec![0f64; n + 1];
    let mut next = vec![n; n + 1];
    // let mut cur = String::with_capacity(n);
    for i in (0..n).rev() {
        // cur.clear();
        let mut all_alphabet = true;
        let mut none_alphabet = true;
        let mut empty_term_value = 0.0;
        let mut node = &*TRIE;
        for j in i..n.min(i + MAX_TERM_LENGTH - 1) {
            // cur.push(content[j].1);
            if content[j].1.is_ascii_alphabetic() {
                none_alphabet = false;
                if !all_alphabet {
                    empty_term_value = -100.0;
                }
            } else {
                all_alphabet = false;
                if !none_alphabet {
                    empty_term_value = -100.0;
                }
            }
            let next_node = node.seek_char(content[j].1);
            let nf = f[j + 1]
                + next_node
                    .and_then(|n| n.value())
                    .unwrap_or(empty_term_value);

            if f[i] <= nf {
                f[i] = nf;
                next[i] = j + 1;
            }
            match next_node {
                Some(n) => node = n,
                None => {
                    break;
                }
            }
        }
    }
    let mut ret = Vec::new();

    let mut p = next[0];
    while p < n {
        ret.push(content[p].0);
        p = next[p];
    }

    ret
}

const INDEX_NAME: &str = ".tf-idf.bin";
/// Should be greater than 0
const INDEX_VERSION: u64 = 91;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();

    if args.len() == 1 {
        println!(
            "usage: {program} <path> or {program} <path> <keyword>",
            program = args.first().unwrap()
        );
        return Ok(());
    }

    if args.len() == 2 {
        let folder = std::path::Path::new(&args[1]);
        construct_index(folder)
    } else {
        let folder = PathBuf::from(&args[1]);
        let keyword = args.into_iter().nth(2).unwrap();

        println!("{:?}", search(folder, &keyword));
        Ok(())
    }
}

fn get_library(folder: impl AsRef<Path>) -> Result<Library, Box<dyn Error>> {
    let folder = folder.as_ref();

    if !folder.is_dir() {
        return Err(format!(
            "Path should be a directory, found {path}.",
            path = folder.to_string_lossy()
        )
        .into());
    }

    let count = fs::read_dir(folder)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|f| f.is_file())) // 仅统计文件，排除文件夹
        .count();
    let cur_count = AtomicUsize::new(0);
    let period = (count / 100).max(1);

    let read_dir: Vec<_> = fs::read_dir(folder)?
        .filter_map(|res| match res {
            Ok(entry) if entry.file_name().to_str() != Some(INDEX_NAME) => Some(entry),
            _ => None,
        })
        .collect();
    let (names, metas): (Vec<_>, Vec<_>) = read_dir
        .into_par_iter()
        .filter_map(|entry| {
            let prev_count = cur_count.fetch_add(1, Ordering::Relaxed);
            if period < 10 {
                println!("{}/{}...", prev_count + 1, count);
            } else {
                if prev_count % period == 0 {
                    println!("{}%...", prev_count / period);
                }
            }
            let path = entry.path();
            let rpath = pathdiff::diff_paths(&path, folder).unwrap();
            // println!("{rpath:?}");
            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Warning: 跳过文件 {:?}, 错误原因: {}", path, e);
                    return None; // 这里的 return 只退出当前闭包，不影响其他并行的线程
                }
            };
            let split = tokenize(&content);

            Some((rpath, term_freq(content, split)))
        })
        .unzip();

    Ok(Library::new(names, metas))
}

fn construct_index(folder: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let folder = folder.as_ref();

    let lib = get_library(folder)?;

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&lib)?;

    let mut file = fs::File::create(folder.join(INDEX_NAME)).unwrap();

    file.write_all(&INDEX_VERSION.to_le_bytes()).unwrap();
    file.write_all(&bytes).unwrap();

    println!("Finished indexing for {}.", folder.to_string_lossy());

    Ok(())
}

fn search(folder: impl AsRef<Path>, keyword: &str) -> Option<PathBuf> {
    let folder = folder.as_ref();
    let keyword = keyword.to_ascii_uppercase();

    let index_path = folder.join(INDEX_NAME);

    if !index_path.exists() {
        println!("Index doesn't exist, creating...");
        construct_index(folder).unwrap();
    }

    let mut file = fs::File::open(&index_path).unwrap();
    let mut version_slice = 0u64.to_le_bytes();
    file.read_exact(&mut version_slice).unwrap();

    let version = u64::from_le_bytes(version_slice);

    if version != INDEX_VERSION {
        println!("Index out of date, recreating...");
        drop(file);
        construct_index(folder).unwrap();
        file = fs::File::open(&index_path).unwrap();
    }

    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let archived = access::<ArchivedLibrary, rkyv::rancor::Error>(&mmap[8..]).unwrap();
    // let archived = unsafe {
    //     rkyv::access_unchecked::<ArchivedLibrary>(&mmap)
    // };
    // let archived = unsafe {
    //     let root_pos = mmap.len() - std::mem::size_of::<ArchivedLibrary>();
    //     let ptr = mmap.as_ptr().add(root_pos) as *const <Library as rkyv::Archive>::Archived;
    //     &*ptr
    // };

    let mut file_value: HashMap<usize, f64> = HashMap::new();

    let boundaries: Vec<usize> = keyword.char_indices().map(|(i, _)| i).collect();
    let total = keyword.len();

    // Traversal substrings of keyword
    for &start in &boundaries {
        for &end in boundaries
            .iter()
            .skip_while(|&&b| b <= start)
            .chain(std::iter::once(&total))
        {
            let Ok(term): &Result<ArchivedSmallString<_>, _> = &keyword[start..end].try_into()
            else {
                break;
            };
            dbg!(term.as_ref());
            if let Some(vec) = archived.tf_idf.get(term) {
                for ArchivedTuple2(i, value) in vec.iter() {
                    let i = i.to_native() as usize;
                    let value = value.to_native();
                    let t = file_value.entry(i).or_insert(0.);
                    *t += value * (end - start) as f64;
                }
            }
        }
    }

    let best_match = file_value.into_iter().max_by(|a, b| a.1.total_cmp(&b.1))?;

    let rpath = &archived.articles[best_match.0];
    let rpath = unsafe { path_from_bytes(rpath) };
    let path = folder.join(rpath);

    Some(path)
}
