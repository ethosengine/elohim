//! Bootstrap module — fetch, verify, and unzip an SSR bundle from the substrate.
//!
//! `BundleSource` is the trait that resolves a deployment slug to a content-addressed
//! blob hash and then fetches the raw bytes. `materialize_server_bundle` does the
//! integrity check and unzips the bundle into a target directory, returning the path to
//! `main.server.mjs` inside it.
//!
//! The SSR runtime always materializes the Angular **server** bundle, so the
//! materialize path resolves via [`BundleSource::resolve_server_blob_hash`] (the
//! deploy-time `serverBlobHash` pointer), not the browser `blobHash`. The trait
//! retains `resolve_blob_hash` for the browser-bundle resolve used elsewhere
//! (e.g. doorway's per-app file cache), but the boot materialize is server-only.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read as IoRead;
use std::path::{Path, PathBuf};

use crate::error::{RenderError, Result};

/// Resolves a deployment slug to a content-addressed hash and fetches the blob bytes.
///
/// Two resolve methods, by bundle kind:
/// - `resolve_blob_hash` reads the **browser** bundle pointer (`blobHash`).
/// - `resolve_server_blob_hash` reads the **server** bundle pointer (`serverBlobHash`).
///
/// Both are required (no default impl): an implementor must read a genuinely
/// different field for each — a default that delegated server→browser would
/// silently materialize the wrong bundle.
pub trait BundleSource {
    fn resolve_blob_hash(&self, slug: &str) -> Result<String>;
    fn resolve_server_blob_hash(&self, slug: &str) -> Result<String>;
    fn fetch_blob(&self, hash: &str) -> Result<Vec<u8>>;
}

/// Fetch the **server** bundle for `slug`, verify its integrity, unzip into
/// `target_dir`, and return the path to `main.server.mjs`.
///
/// Resolves the content hash via [`BundleSource::resolve_server_blob_hash`]
/// (the deploy-time `serverBlobHash` pointer on the one EPR node), then performs
/// the shared verify + zip-slip-guarded unzip.
///
/// Errors (never panics) on:
/// - `resolve_server_blob_hash` failure
/// - `fetch_blob` failure
/// - SHA-256 mismatch
/// - ZIP extraction failure
pub fn materialize_server_bundle<S: BundleSource>(
    src: &S,
    slug: &str,
    target_dir: &Path,
) -> Result<PathBuf> {
    let expected_hash = src.resolve_server_blob_hash(slug)?;
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

    // A simple mock source that returns whatever hash/bytes we configure.
    //
    // `server_hash` is the value returned by `resolve_server_blob_hash`. When
    // `None`, it mirrors `hash` (the browser hash) — but the dedicated
    // distinct-hash test sets it explicitly so we can prove the materialize
    // path resolves the SERVER pointer, not the browser one.
    struct MockSource {
        hash: Result<String>,
        server_hash: Option<Result<String>>,
        blob: Result<Vec<u8>>,
    }

    impl MockSource {
        fn ok(hash: impl Into<String>, blob: Vec<u8>) -> Self {
            Self {
                hash: Ok(hash.into()),
                server_hash: None,
                blob: Ok(blob),
            }
        }
        fn resolve_err(msg: impl Into<String>) -> Self {
            Self {
                hash: Err(RenderError::Bootstrap(msg.into())),
                server_hash: None,
                blob: Ok(vec![]),
            }
        }
        /// Set a server hash distinct from the browser hash.
        fn with_server_hash(mut self, server_hash: impl Into<String>) -> Self {
            self.server_hash = Some(Ok(server_hash.into()));
            self
        }
    }

    impl BundleSource for MockSource {
        fn resolve_blob_hash(&self, _slug: &str) -> Result<String> {
            match &self.hash {
                Ok(h) => Ok(h.clone()),
                Err(e) => Err(RenderError::Bootstrap(e.to_string())),
            }
        }
        fn resolve_server_blob_hash(&self, slug: &str) -> Result<String> {
            match &self.server_hash {
                Some(Ok(h)) => Ok(h.clone()),
                Some(Err(e)) => Err(RenderError::Bootstrap(e.to_string())),
                // Default: mirror the browser hash so existing fixtures that
                // don't distinguish still work; the distinct-hash test overrides.
                None => self.resolve_blob_hash(slug),
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
        let result = materialize_server_bundle(&src, "my-bundle", dir.path());
        assert!(result.is_err(), "expected Err on hash mismatch, got Ok");
    }

    #[test]
    fn materialize_unzips_and_returns_entry() {
        let dir = tempfile::tempdir().unwrap();
        // Build in-memory zip with main.server.mjs containing "X"
        let zip_bytes = make_zip("main.server.mjs", b"X");
        let hash = format!("sha256-{:x}", Sha256::digest(&zip_bytes));
        // server hash mirrors the browser hash here (None) — fine for the
        // unzip path because the bytes match either way.
        let src = MockSource::ok(&hash, zip_bytes);
        let path = materialize_server_bundle(&src, "my-bundle", dir.path()).unwrap();
        assert!(path.exists(), "main.server.mjs should exist at {:?}", path);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "X");
    }

    #[test]
    fn materialize_propagates_resolve_error() {
        let dir = tempfile::tempdir().unwrap();
        let src = MockSource::resolve_err("not found");
        let result = materialize_server_bundle(&src, "missing-bundle", dir.path());
        assert!(
            result.is_err(),
            "expected Err when resolve_server_blob_hash fails"
        );
    }

    // A source that records which hash `fetch_blob` was asked for, so we can
    // prove the materialize path resolves the SERVER hash, not the browser one.
    struct RecordingSource {
        browser_hash: String,
        server_hash: String,
        zip_bytes: Vec<u8>,
        fetched: std::cell::RefCell<Option<String>>,
    }

    impl BundleSource for RecordingSource {
        fn resolve_blob_hash(&self, _slug: &str) -> Result<String> {
            Ok(self.browser_hash.clone())
        }
        fn resolve_server_blob_hash(&self, _slug: &str) -> Result<String> {
            Ok(self.server_hash.clone())
        }
        fn fetch_blob(&self, hash: &str) -> Result<Vec<u8>> {
            *self.fetched.borrow_mut() = Some(hash.to_string());
            Ok(self.zip_bytes.clone())
        }
    }

    #[test]
    fn materialize_server_bundle_fetches_the_server_hash() {
        let dir = tempfile::tempdir().unwrap();
        let zip_bytes = make_zip("main.server.mjs", b"SERVER");
        // The server hash is the real (correct) hash of the bytes; the browser
        // hash is a DISTINCT bogus value. If the materialize path mistakenly
        // resolved the browser hash, the integrity check would fail.
        let server_hash = format!("sha256-{:x}", Sha256::digest(&zip_bytes));
        let browser_hash =
            "sha256-0000000000000000000000000000000000000000000000000000000000000000".to_string();
        assert_ne!(server_hash, browser_hash, "test setup: hashes must differ");

        let src = RecordingSource {
            browser_hash: browser_hash.clone(),
            server_hash: server_hash.clone(),
            zip_bytes,
            fetched: std::cell::RefCell::new(None),
        };

        let path = materialize_server_bundle(&src, "elohim-host-landing", dir.path())
            .expect("server materialize should succeed with the server hash");
        assert!(path.exists(), "main.server.mjs should exist at {:?}", path);

        // The blob actually fetched was keyed on the SERVER hash, not the browser one.
        let fetched = src.fetched.borrow().clone();
        assert_eq!(
            fetched,
            Some(server_hash),
            "materialize_server_bundle must fetch the SERVER blob hash"
        );
    }

    #[test]
    fn materialize_distinct_hashes_via_mock_with_server_hash() {
        // Mirror of the above using MockSource's `with_server_hash` builder, to
        // exercise the trait method's distinct-field contract directly.
        let zip_bytes = make_zip("main.server.mjs", b"Y");
        let server_hash = format!("sha256-{:x}", Sha256::digest(&zip_bytes));
        // Browser hash is distinct and bogus; the materialize path must NOT use it.
        let src = MockSource::ok(
            "sha256-1111111111111111111111111111111111111111111111111111111111111111",
            zip_bytes,
        )
        .with_server_hash(server_hash);
        let dir = tempfile::tempdir().unwrap();
        let path = materialize_server_bundle(&src, "slug", dir.path())
            .expect("server materialize must succeed using the server hash");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Y");
    }
}
