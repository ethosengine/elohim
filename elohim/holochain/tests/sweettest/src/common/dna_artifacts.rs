//! Runtime artifact lookup for a prebuilt test binary moved off its build host.

use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// An explicit artifact root is authoritative: never fall back to the build checkout.
/// Kept independent of conductor dependencies so relocation can be tested cheaply.
pub fn supplied_dna(root: Option<&Path>, name: &str) -> Result<Option<PathBuf>> {
    let Some(root) = root else { return Ok(None) };
    if !root.is_absolute()
        || name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "SWEETTEST_DNA_DIR must be absolute and DNA name must be a filename stem",
        ));
    }
    let root = root.canonicalize()?;
    let artifact = root.join(format!("{name}.dna")).canonicalize()?;
    if !artifact.starts_with(&root) || !artifact.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "supplied DNA must be a file inside SWEETTEST_DNA_DIR",
        ));
    }
    Ok(Some(artifact))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocated_bundle_is_found_without_the_original_checkout() {
        let root =
            std::env::temp_dir().join(format!("sweettest-relocation-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("lamad.dna"), b"shipped bundle").unwrap();
        let path = supplied_dna(Some(&root), "lamad").unwrap().unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"shipped bundle");
        assert!(supplied_dna(Some(&root), "mishpat").is_err());
        assert!(supplied_dna(Some(&root), "../lamad").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_an_absent_override_allows_legacy_resolution() {
        assert_eq!(supplied_dna(None, "lamad").unwrap(), None);
        assert!(supplied_dna(Some(Path::new("relative")), "lamad").is_err());
        assert!(supplied_dna(Some(Path::new("")), "lamad").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn supplied_root_cannot_escape_through_symlinks() {
        let root = std::env::temp_dir().join(format!("sweettest-symlink-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", root.join("lamad.dna")).unwrap();
        assert!(supplied_dna(Some(&root), "lamad").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
