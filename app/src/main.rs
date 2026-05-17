// todo: use Trie for DICT
use dict_data::DICT;

mod data;

use memmap2::Mmap;
use rkyv::tuple::ArchivedTuple2;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::data::{ArchivedLibrary, Library, Metadata, path_from_bytes};

const MAX_TERM_LENGTH: usize = 50;

/// Return split points of content
fn tokenize(content: impl AsRef<str>) -> Vec<usize> {
    let content: Vec<_> = content.as_ref().char_indices().collect();
    let n = content.len();
    let mut f = vec![0f64; n + 1];
    let mut next = vec![n; n + 1];
    let mut cur = String::with_capacity(n);
    for i in (0..n).rev() {
        cur.clear();
        for j in i..n.min(i + MAX_TERM_LENGTH) {
            cur.push(content[j].1);
            let nf = f[j + 1] + get_term_value_log(&cur);

            if f[i] <= nf {
                f[i] = nf;
                next[i] = j + 1;
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

fn get_term_value_log(term: &str) -> f64 {
    DICT.get(term).map(|x| (*x as f64).ln()).unwrap_or_else(|| {
        let alphabetic: usize = term.chars().map(|c| c.is_alphabetic() as usize).sum();

        if alphabetic == 0 || alphabetic == term.len() {
            0.
        } else {
            -100.
        }
    })
}

const INDEX_NAME: &str = ".tf-idf.bin";

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

fn construct_index(folder: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let folder = folder.as_ref();

    if !folder.is_dir() {
        return Err(format!(
            "Path should be a directory, found {path}.",
            path = folder.to_string_lossy()
        )
        .into());
    }

    let mut metas = Vec::new();

    let count = fs::read_dir(folder)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file()) // 仅统计文件，排除文件夹
        .count();
    let mut cur_count = 0;
    let period = (count / 100).max(1);

    for entry in fs::read_dir(folder)? {
        if period < 10 {
            cur_count += 1;
            println!("{}/{}...", cur_count, count);
        } else {
            if cur_count % period == 0 {
                println!("{}%...", cur_count / period);
            }
            cur_count += 1;
        }
        let entry = entry?;
        let path = entry.path();
        let rpath = pathdiff::diff_paths(&path, folder).unwrap();
        // println!("{rpath:?}");
        if path.file_name().and_then(|x| x.to_str()) == Some(INDEX_NAME) {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        let split = tokenize(&content);

        metas.push(Metadata {
            path: rpath,
            content,
            split,
        })
    }

    let mut file = fs::File::create(folder.join(INDEX_NAME)).unwrap();

    let lib = Library::new(metas);

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&lib)?;

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

    let file = fs::File::open(index_path).unwrap();

    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let archived = unsafe {
        let root_pos = mmap.len() - std::mem::size_of::<ArchivedLibrary>();
        let ptr = mmap.as_ptr().add(root_pos) as *const <Library as rkyv::Archive>::Archived;
        &*ptr
    };

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
            if let Some(vec) = archived.tf_idf.get(&keyword[start..end]) {
                for ArchivedTuple2(i, value) in vec.iter() {
                    let i = i.to_native() as usize;
                    let value = value.to_native();
                    *file_value.entry(i).or_insert(0.) += value * (end - start) as f64;
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
