// 一次性诊断：测量 I/O vs 分词的时间占比
// 运行: cargo run --profile profiling --bin diagnose -- <folder>

use std::env;
use std::fs;
use std::io::Read;
use std::time::Instant;

use tf_idf::data::term_freq;
use tf_idf::lexer::tokenize;

fn main() {
    let args: Vec<_> = env::args().collect();
    let folder = &args[1];

    let mut all_files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(folder)];
    while let Some(top) = stack.pop() {
        if let Ok(dir) = fs::read_dir(&top) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().map(|e| e == "txt").unwrap_or(false) {
                    all_files.push(path);
                }
            }
        }
    }

    // Sample ~500 files evenly distributed
    let step = (all_files.len() / 500).max(1);
    let sample: Vec<_> = all_files.iter().step_by(step).take(500).collect();

    let n = sample.len();
    eprintln!("Sampling {n} files...");

    // Phase 1: pure I/O (read to string, no processing)
    let t0 = Instant::now();
    let mut total_bytes = 0u64;
    let mut total_chars = 0u64;
    let mut contents = Vec::with_capacity(n);
    for path in &sample {
        let mut s = String::new();
        if let Ok(mut f) = fs::File::open(path) {
            if f.read_to_string(&mut s).is_ok() {
                total_bytes += s.len() as u64;
                total_chars += s.chars().count() as u64;
                contents.push(s);
            }
        }
    }
    let io_time = t0.elapsed();
    eprintln!(
        "[IO only]  {n} files, {total_bytes} bytes, {total_chars} chars: {:.3}s",
        io_time.as_secs_f64()
    );

    // Phase 2: tokenize only
    let t0 = Instant::now();
    let mut total_splits = 0u64;
    for content in &contents {
        let split = tokenize(content);
        total_splits += split.len() as u64;
    }
    let tok_time = t0.elapsed();
    eprintln!(
        "[Tokenize] {n} files, {total_splits} splits:       {:.3}s",
        tok_time.as_secs_f64()
    );

    // Phase 3: full term_freq (tokenize + hashmap)
    let t0 = Instant::now();
    let mut total_terms = 0u64;
    for content in &contents {
        let tf = term_freq(content.clone());
        total_terms += tf.len() as u64;
    }
    let tf_time = t0.elapsed();
    eprintln!(
        "[TermFreq] {n} files, {total_terms} unique terms:  {:.3}s",
        tf_time.as_secs_f64()
    );

    // Phase 4: read + term_freq combined (what actually runs in parallel)
    let t0 = Instant::now();
    let mut combined_bytes = 0u64;
    for path in &sample {
        let mut s = String::new();
        if let Ok(mut f) = fs::File::open(path) {
            if f.read_to_string(&mut s).is_ok() {
                combined_bytes += s.len() as u64;
                let _tf = term_freq(s);
            }
        }
    }
    let combined_time = t0.elapsed();
    eprintln!(
        "[I/O+TF]   {n} files, {combined_bytes} bytes:         {:.3}s",
        combined_time.as_secs_f64()
    );

    eprintln!("\n=== Summary ({n} sampled files) ===");
    let total = combined_time.as_secs_f64();
    let io_pct = io_time.as_secs_f64() / total * 100.0;
    let compute_pct = (total - io_time.as_secs_f64()) / total * 100.0;
    eprintln!("Combined I/O+TF:  {:.3}s (100%)", total);
    eprintln!(
        "  Pure I/O:       {:.3}s ({:.0}%)",
        io_time.as_secs_f64(),
        io_pct
    );
    eprintln!(
        "  Compute (TF):   {:.3}s ({:.0}%)",
        total - io_time.as_secs_f64(),
        compute_pct
    );
    eprintln!("    - Tokenize:   {:.3}s", tok_time.as_secs_f64());
    eprintln!("    - TermFreq:   {:.3}s", tf_time.as_secs_f64());
    eprintln!(
        "Avg file size: {:.1} KB",
        total_bytes as f64 / n as f64 / 1024.0
    );
}
