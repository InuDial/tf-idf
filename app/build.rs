fn main() -> Result<(), Box<dyn std::error::Error>> {
    let version = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let out_dir = std::env::var("OUT_DIR")?;
    let dest = std::path::Path::new(&out_dir).join("version.rs");
    std::fs::write(&dest, version.to_string())?;

    Ok(())
}
