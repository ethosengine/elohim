//! Embed first-party dependency source identity without executing a generator at install time.
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

fn walk(root: &Path, at: &Path, output: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(at)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "method dependency source must not be a symlink: {}",
                path.display()
            )
            .into());
        }
        if metadata.is_dir() {
            walk(root, &path, output)?;
        } else if metadata.is_file() {
            output.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // The three method-dependency crates below are embedded whole (src/ and Cargo.toml):
    // gate-reads: elohim/eprfs/eprfs-core
    // gate-reads: elohim/eprfs/eprfs-meta
    // gate-reads: crates/seam-contracts
    let package = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let mut generated = String::from("pub const DEPENDENCY_SOURCES: &[(&str, &[u8])] = &[\n");
    for (label, relative) in [
        ("eprfs-core", "../../../elohim/eprfs/eprfs-core"),
        ("eprfs-meta", "../../../elohim/eprfs/eprfs-meta"),
        ("seam-contracts", "../../../crates/seam-contracts"),
    ] {
        let root = package.join(relative).canonicalize()?;
        println!("cargo:rerun-if-changed={}", root.join("src").display());
        println!(
            "cargo:rerun-if-changed={}",
            root.join("Cargo.toml").display()
        );
        let mut files = vec![PathBuf::from("Cargo.toml")];
        walk(&root, &root.join("src"), &mut files)?;
        files.sort();
        for file in files {
            generated.push_str(&format!(
                "({:?}, include_bytes!({:?})),\n",
                format!("{label}/{}", file.display()),
                root.join(file)
            ));
        }
    }
    generated.push_str("];\n");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../../elohim/eprfs/Cargo.toml");
    fs::write(
        PathBuf::from(env::var("OUT_DIR")?).join("method-dependencies.rs"),
        generated,
    )?;
    Ok(())
}
