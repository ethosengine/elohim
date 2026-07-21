//! Dogfood parity (plan Task 2): for the sealed-edges spec's OWN `cites:` envelopes, the
//! Rust seal-aware verdict on an unchanged target must be `Ok` for every sealed cite. This
//! mirrors `flow_parity.rs` (the cite-fingerprint parity) one layer up — proving the verdict
//! rule, not just the digest, matches the live corpus. Read-only; skips gracefully when run
//! outside the repo (the spec file absent).

use std::path::PathBuf;

use elohim_epr_cli::flow::edges::{derive_verdict, EdgePlane, IndexedEdge, SealForm, Verdict};
use elohim_epr_cli::flow::{canonical_body, cite_fingerprint, cite_path, parse_frontmatter};
use elohim_epr_rea::Governor;
use eprfs_core::BlobCid;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root resolves")
}

const SEALED_EDGES_SPEC: &str =
    "genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md";

#[test]
fn sealed_cites_in_the_spec_derive_ok_verdicts() {
    let root = repo_root();
    let spec_path = root.join(SEALED_EDGES_SPEC);
    let Ok(text) = std::fs::read_to_string(&spec_path) else {
        eprintln!("sealed-edges spec absent — skipping dogfood parity (outside repo context)");
        return;
    };
    let fm = parse_frontmatter(&text);

    let mut checked = 0usize;
    for entry in fm.list("cites") {
        // Only sealed envelopes (a fingerprint present) participate; bare/legacy path cites
        // are unsealed references, out of the seal-aware graph.
        let Some(fingerprint) = cite_fingerprint(entry) else {
            continue;
        };
        let Some(path) = cite_path(entry) else {
            continue;
        };
        let target = root.join(&path);
        let body = std::fs::read_to_string(&target)
            .unwrap_or_else(|e| panic!("cite target {path} readable: {e}"));
        let current = BlobCid::compute_raw(canonical_body(&body).as_bytes());

        let edge = IndexedEdge {
            from: SEALED_EDGES_SPEC.to_string(),
            to: path.clone(),
            desc: None,
            governor: Governor::CiteSeal,
            seal: SealForm::Short(fingerprint),
            held: None,
            plane: EdgePlane::Doc,
            target_exists: true,
        };
        assert_eq!(
            derive_verdict(&edge, Some(&current)),
            Verdict::Ok,
            "sealed cite to {path} must derive Ok (unchanged target)"
        );
        checked += 1;
    }

    assert!(
        checked >= 6,
        "expected at least 6 sealed cites verified, got {checked}"
    );
}
