//! Cite-oracle parity: the Rust canonical-body encoder must reproduce the sealed cite
//! fingerprints on real repository docs. This proves `epr flow`'s resource CIDs match the
//! live Python cite oracle (`_lib/cite_graph.fingerprint_text`) byte-for-byte.
//!
//! Dynamic by construction: it parses the fabric spec's own `cites:` lines, takes each
//! entry's `sha256:hex16` + `path:` locator, computes the target file's body CID in Rust,
//! and asserts the short form equals the sealed hex16.

use std::path::PathBuf;

use elohim_epr_cli::flow::{canonical_body, cite_fingerprint, cite_path, parse_frontmatter};
use eprfs_core::BlobCid;

fn repo_root() -> PathBuf {
    // tests run from the crate dir: <repo>/elohim/eprfs/epr-cli → repo root is three up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root resolves")
}

const FABRIC_SPEC: &str =
    "genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md";

#[test]
fn cite_fingerprints_match_the_rust_body_encoder() {
    let root = repo_root();
    let spec_path = root.join(FABRIC_SPEC);
    let text = std::fs::read_to_string(&spec_path).expect("fabric spec is present");
    let fm = parse_frontmatter(&text);

    let cites = fm.list("cites");
    assert!(
        cites.len() >= 7,
        "fabric spec should declare at least 7 cites, found {}",
        cites.len()
    );

    let mut checked = 0usize;
    for entry in cites {
        let Some(sealed) = cite_fingerprint(entry) else {
            panic!("cite entry missing a sha256: fingerprint:\n{entry}");
        };
        let Some(path) = cite_path(entry) else {
            // A cite without a path locator can't be resolved to a file — skip it, but
            // the fabric spec's cites all carry one, so this should not fire.
            continue;
        };
        let target = root.join(&path);
        let body = std::fs::read_to_string(&target)
            .unwrap_or_else(|e| panic!("cite target {path} readable: {e}"));
        let computed = BlobCid::compute_raw(canonical_body(&body).as_bytes()).short_fingerprint();
        assert_eq!(
            computed, sealed,
            "fingerprint drift for {path}: rust {computed} != sealed {sealed}"
        );
        checked += 1;
    }

    assert!(
        checked >= 7,
        "expected to verify at least 7 resolvable cites, verified {checked}"
    );
}
