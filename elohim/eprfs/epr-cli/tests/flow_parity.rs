//! Cite-oracle parity: the Rust canonical-body encoder must reproduce the sealed cite
//! fingerprints on real repository docs. This proves `epr flow`'s resource CIDs match the
//! live Python cite oracle (`_lib/cite_graph.fingerprint_text`) byte-for-byte.
//!
//! Dynamic by construction: it parses the fabric spec's own `cites:` lines, takes each
//! entry's `sha256:hex16` + `path:` locator, computes the target file's body CID in Rust,
//! and asserts the short form equals the sealed hex16.

mod common;

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
    // The spec and the seven cite targets its frontmatter names; a new cite must be declared.
    // gate-reads: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
    // gate-reads: genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md
    // gate-reads: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
    // gate-reads: genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
    // gate-reads: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
    // gate-reads: genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md
    // gate-reads: genesis/docs/superpowers/specs/2026-07-12-epr-meta-kinship-lineage-reconciliation-design.md
    // gate-reads: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
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

    let read_paths = cites
        .iter()
        .filter_map(|entry| cite_fingerprint(entry).and_then(|_| cite_path(entry)))
        .collect::<Vec<_>>();
    common::assert_declared_reads(
        include_str!("flow_parity.rs"),
        read_paths.iter().map(String::as_str),
        "fabric parity cite targets",
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
