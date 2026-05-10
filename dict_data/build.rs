use std::collections::HashSet;
// build.rs
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() {
    // 告诉 Cargo 如果 dict.txt 变了就重新运行 build.rs
    println!("cargo:rerun-if-changed=dict.txt");

    let dict_path = Path::new("dict.txt");
    let file = File::open(dict_path).expect("无法打开 dict.txt");
    let reader = BufReader::new(file);

    let mut pairs = Vec::new();
    for line in reader.lines() {
        let line = line.expect("读行失败");
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace().take(2).map(ToString::to_string);

        match (parts.next(), parts.next()) {
            (Some(key), Some(value)) => {
                pairs.push((key, value));
            }
            _ => {
                panic!("dict.txt 每行必须包含两个由空白分隔的字段，实际: {:?}", trimmed);
            }
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut seen = HashSet::new();
    let pairs: Vec<_> = pairs
        .into_iter()
        .filter(|x| seen.insert(x.0.clone()))
        .collect();

    // 生成 Rust 源码文件
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_dict.rs");
    let mut f = File::create(&dest_path).unwrap();

    // 写入常量数组定义
    writeln!(
        f,
        "phf::phf_map! {{\n"
    )
    .unwrap();

    for (key, value) in pairs {
        writeln!(f, "    \"{}\" => {},", key, value).unwrap();
    }
    writeln!(f, "}}").unwrap();
}