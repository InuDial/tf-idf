#![feature(gen_blocks)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use rayon::prelude::*;

mod data;
mod lexer;
mod map;
mod small_str;
mod trie;

use crate::lexer::MAX_TERM_LENGTH;
use crate::small_str::ArchivedSmallString;
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
use std::time::Instant;

use crate::data::{ArchivedLibrary, Library, path_from_bytes, term_freq};

struct Timing {
    label: &'static str,
    start: Instant,
}

impl Timing {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            start: Instant::now(),
        }
    }
}

impl Drop for Timing {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        eprintln!("[TIMING] {}: {:.3}s", self.label, elapsed.as_secs_f64());
    }
}

const INDEX_NAME: &str = ".tf-idf.bin";
pub const INDEX_VERSION: u64 = include!(concat!(env!("OUT_DIR"), "/version.rs"));

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

        println!(
            "{}",
            search(folder, &keyword)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| String::from("None"))
        );
        Ok(())
    }
}

fn get_library(folder: impl AsRef<Path>) -> Result<Library, Box<dyn Error>> {
    let _tracy = tracy_client::span!("get_library");
    let _timer = Timing::new("get_library (total)");
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

    let read_dir: Vec<_> = {
        let _t = Timing::new("  walk_dir_tree");
        let generator = gen {
            let mut stack = vec![folder.to_owned()];
            while let Some(top) = stack.pop() {
                let Ok(dir) = fs::read_dir(top).inspect_err(|e| {
                    eprintln!("{e:?}");
                }) else {
                    continue;
                };
                for sub in dir {
                    let Ok(sub) = sub.inspect_err(|e| {
                        eprintln!("{e:?}");
                    }) else {
                        continue;
                    };
                    if sub.file_type().is_ok_and(|d| d.is_dir()) {
                        stack.push(sub.path());
                        continue;
                    }
                    if sub.file_name().to_str() != Some(INDEX_NAME) {
                        yield sub;
                    }
                }
            }
        };
        generator.collect()
    };

    let (names, metas): (Vec<_>, Vec<_>) = {
        let _t = Timing::new("  parallel_read_and_tokenize");
        read_dir
            .into_par_iter()
            .filter_map(|entry| {
                let prev_count = cur_count.fetch_add(1, Ordering::Relaxed);
                if period < 10 {
                    println!("{}/{}...", prev_count + 1, count);
                } else if prev_count.is_multiple_of(period) {
                    println!("{}%...", prev_count / period);
                }
                let path = entry.path();
                let rpath = pathdiff::diff_paths(&path, folder).unwrap();
                let content = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Warning: 跳过文件 {:?}, 错误原因: {}", path, e);
                        return None;
                    }
                };

                Some((rpath, term_freq(content)))
            })
            .unzip()
    };

    let lib = {
        let _t = Timing::new("  build_library");
        Library::new(names, metas)
    };
    Ok(lib)
}

fn construct_index(folder: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let _tracy = tracy_client::span!("construct_index");
    let _timer = Timing::new("construct_index (total)");
    let folder = folder.as_ref();

    let lib = get_library(folder)?;

    let bytes = {
        let _t = Timing::new("  serialize_to_bytes");
        rkyv::to_bytes::<rkyv::rancor::Error>(&lib)?
    };

    let mut file = fs::File::create(folder.join(INDEX_NAME)).unwrap();

    file.write_all(&INDEX_VERSION.to_le_bytes()).unwrap();
    file.write_all(&bytes).unwrap();

    println!("Finished indexing for {}.", folder.to_string_lossy());

    Ok(())
}

fn search(folder: impl AsRef<Path>, keyword: &str) -> Option<PathBuf> {
    let _tracy = tracy_client::span!("search");
    let _timer = Timing::new("search (total)");
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

    // SAFETY: mmap is an unsafe operation
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let archived = access::<ArchivedLibrary, rkyv::rancor::Error>(&mmap[8..]).unwrap();

    let mut file_value: HashMap<usize, f32> = HashMap::new();

    let boundaries: Vec<usize> = keyword.char_indices().map(|(i, _)| i).collect();
    let total = keyword.len();

    // TODO: split keywords instead of traversing substrings of keyword
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
            if let Some(vec) = archived.tf_idf.get(term) {
                for ArchivedTuple2(i, value) in vec.iter() {
                    let i = i.to_native() as usize;
                    let value = value.to_native();
                    let t = file_value.entry(i).or_insert(0.);
                    *t += value * (end - start) as f32;
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
