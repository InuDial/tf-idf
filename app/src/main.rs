use dict_data::DICT;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const MAX_TERM_LENGTH: usize = 50;

/// Return split points of content
fn tokenize(content: impl AsRef<str>) -> Vec<usize> {
    let content: Vec<char> = content.as_ref().chars().collect();
    let n = content.len();
    let mut f = vec![0f64; n + 1];
    let mut next = vec![n; n + 1];
    let mut cur = String::with_capacity(n);
    for i in (0..n).rev() {
        cur.clear();
        for j in i..n.min(i + MAX_TERM_LENGTH) {
            cur.push(content[j]);
            let nf = f[j + 1] + get_term_value_log(&cur);

            if f[i] < nf {
                f[i] = nf;
                next[i] = j + 1;
            }
        }
    }
    let mut ret = Vec::new();

    let mut p = next[0];
    while p < n {
        ret.push(p);
        p = next[p];
    }

    ret
}

fn get_term_value_log(term: &str) -> f64 {
    DICT.get(term).map(|x| (*x as f64).ln()).unwrap_or(0.)
}

pub struct Metadata {
    pub path: PathBuf,
    pub content: String,
    pub split: Vec<usize>,
}

impl Metadata {
    pub fn term_freq(&self) -> HashMap<String, f64> {
        let inv_term_count = 1f64 / (self.split.len() + 1) as f64;

        let mut ret = HashMap::new();
        let mut l = 0;
        for &r in &self.split {
            let entry = ret.entry(self.content[l..r].to_string());
            *entry.or_insert(0.) += inv_term_count;
            l = r;
        }
        ret
    }
}

pub struct Library {
    articles: Vec<Metadata>,
    /// term -> [(article_id, reletive_freq * idf)]
    tf_idf: HashMap<String, Vec<(usize, f64)>>,
}

impl Library {
    pub fn new(articles: impl IntoIterator<Item = Metadata>) -> Self {
        let articles: Vec<_> = articles.into_iter().collect();
        let n = articles.len() as f64;
        let mut occurrences: HashMap<String, Vec<(usize, f64)>> = HashMap::new();

        for (id, article) in articles.iter().enumerate() {
            let tf_map = article.term_freq();

            for (term, freq) in tf_map {
                occurrences.entry(term).or_default().push((id, freq));
            }
        }

        for (_term, occ) in &mut occurrences {
            let freq_sum: f64 = occ.iter().map(|x| x.1).sum();
            let idf = (n / occ.len() as f64).ln();

            for (_i, f) in occ {
                *f *= idf / freq_sum;
            }
        }

        Self {
            articles,
            tf_idf: occurrences,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();

    if args.len() == 1 {
        println!("usage: {program} <path>", program = args.first().unwrap());
        return Ok(());
    }

    let folder = std::path::Path::new(&args[1]);

    if !folder.is_dir() {
        return Err("ERROR: Path should be a directory.".into());
    }

    let mut metas = Vec::new();

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let content = fs::read_to_string(&path)?;
        let split = tokenize(&content);

        metas.push(Metadata {
            path,
            content,
            split,
        })
    }

    let _lib = Library::new(metas);

    Ok(())
}
