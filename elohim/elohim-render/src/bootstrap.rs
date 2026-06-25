//! Bootstrap module — fetch, verify, and unzip an SSR bundle from the substrate.
//!
//! `BundleSource` is the trait that resolves a deployment slug to a content-addressed
//! blob hash and then fetches the raw bytes. `materialize_bundle` does the integrity
//! check and unzips the bundle into a target directory, returning the path to
//! `main.server.mjs` inside it.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read as IoRead;
use std::path::{Path, PathBuf};

use crate::error::{RenderError, Result};

/// Resolves a deployment slug to a content-addressed hash and fetches the blob bytes.
pub trait BundleSource {
    fn resolve_blob_hash(&self, slug: &str) -> Result<String>;
    fn fetch_blob(&self, hash: &str) -> Result<Vec<u8>>;
}

/// Fetch the bundle for `slug`, verify its integrity, unzip into `target_dir`,
/// and return the path to `main.server.mjs`.
///
/// Errors (never panics) on:
/// - `resolve_blob_hash` failure
/// - `fetch_blob` failure
/// - SHA-256 mismatch
/// - ZIP extraction failure
pub fn materialize_bundle<S: BundleSource>(
    src: &S,
    slug: &str,
    target_dir: &Path,
) -> Result<PathBuf> {
    let expected_hash = src.resolve_blob_hash(slug)?;
    let bytes = src.fetch_blob(&expected_hash)?;

    // Verify integrity: format!("sha256-{:x}", Sha256::digest(&bytes))
    let actual_hash = format!("sha256-{:x}", Sha256::digest(&bytes));
    if actual_hash != expected_hash {
        return Err(RenderError::Bootstrap(format!(
            "hash mismatch for slug `{}`: expected `{}`, got `{}`",
            slug, expected_hash, actual_hash
        )));
    }

    // Unzip into target_dir
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| RenderError::Bootstrap(format!("failed to open zip archive: {}", e)))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            RenderError::Bootstrap(format!("failed to read zip entry {}: {}", i, e))
        })?;

        let entry_path = entry
            .enclosed_name()
            .ok_or_else(|| RenderError::Bootstrap(format!("zip entry {} has unsafe path", i)))?
            .to_owned();

        let out_path = target_dir.join(&entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(&out_path)?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| RenderError::Bootstrap(format!("failed to read zip entry: {}", e)))?;
            use std::io::Write;
            file.write_all(&buf)?;
        }
    }

    Ok(target_dir.join("main.server.mjs"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

    // A simple mock source that returns whatever hash/bytes we configure
    struct MockSource {
        hash: Result<String>,
        blob: Result<Vec<u8>>,
    }

    impl MockSource {
        fn ok(hash: impl Into<String>, blob: Vec<u8>) -> Self {
            Self {
                hash: Ok(hash.into()),
                blob: Ok(blob),
            }
        }
        fn resolve_err(msg: impl Into<String>) -> Self {
            Self {
                hash: Err(RenderError::Bootstrap(msg.into())),
                blob: Ok(vec![]),
            }
        }
    }

    impl BundleSource for MockSource {
        fn resolve_blob_hash(&self, _slug: &str) -> Result<String> {
            match &self.hash {
                Ok(h) => Ok(h.clone()),
                Err(e) => Err(RenderError::Bootstrap(e.to_string())),
            }
        }
        fn fetch_blob(&self, _hash: &str) -> Result<Vec<u8>> {
            match &self.blob {
                Ok(b) => Ok(b.clone()),
                Err(e) => Err(RenderError::Bootstrap(e.to_string())),
            }
        }
    }

    /// Build an in-memory zip containing one file at `name` with body `body`.
    fn make_zip(name: &str, body: &[u8]) -> Vec<u8> {
        let buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(buf);
        let opts = zip::write::SimpleFileOptions::default();
        zw.start_file(name, opts).unwrap();
        zw.write_all(body).unwrap();
        zw.finish().unwrap().into_inner()
    }

    #[test]
    fn materialize_rejects_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        // Source returns a hash that won't match the actual bytes
        let src = MockSource::ok(
            "sha256-deadbeef00000000000000000000000000000000000000000000000000000000",
            b"wrong bytes".to_vec(),
        );
        let result = materialize_bundle(&src, "my-bundle", dir.path());
        assert!(result.is_err(), "expected Err on hash mismatch, got Ok");
    }

    #[test]
    fn materialize_unzips_and_returns_entry() {
        let dir = tempfile::tempdir().unwrap();
        // Build in-memory zip with main.server.mjs containing "X"
        let zip_bytes = make_zip("main.server.mjs", b"X");
        let hash = format!("sha256-{:x}", Sha256::digest(&zip_bytes));
        let src = MockSource::ok(&hash, zip_bytes);
        let path = materialize_bundle(&src, "my-bundle", dir.path()).unwrap();
        assert!(path.exists(), "main.server.mjs should exist at {:?}", path);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "X");
    }

    #[test]
    fn materialize_propagates_resolve_error() {
        let dir = tempfile::tempdir().unwrap();
        let src = MockSource::resolve_err("not found");
        let result = materialize_bundle(&src, "missing-bundle", dir.path());
        assert!(result.is_err(), "expected Err when resolve_blob_hash fails");
    }
}
