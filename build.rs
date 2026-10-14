use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn collect(path: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            collect(&entry.unwrap().path(), files);
        }
    } else {
        files.push(path.to_owned());
    }
}
fn main() {
    let mut files = Vec::new();
    for path in ["src", "web", "Cargo.toml", "Cargo.lock", "build.rs"] {
        collect(Path::new(path), &mut files);
    }
    files.sort();
    let mut digest = Sha256::new();
    for file in files {
        let name = file.to_str().unwrap().as_bytes();
        let content = fs::read(&file).unwrap();
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name);
        digest.update((content.len() as u64).to_be_bytes());
        digest.update(content);
    }
    println!(
        "cargo:rustc-env=FRONTIER_SOURCE_SHA256={:x}",
        digest.finalize()
    );
}
