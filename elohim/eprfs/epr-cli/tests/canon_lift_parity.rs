//! Parity, freshness, and anti-mirror gates for `epr canon-lift`.
//!
//! # What each half proves, and why the halves are computed differently
//!
//! A contract test that computes both sides through one helper measures the helper (`9e9c9d023`,
//! 2026-07-24: `BlobCid::verifies` hardcoded the wrong codec and stayed green for every real
//! fingerprint because both sides were computed the same wrong way). So the checks here reach for
//! three sources that do not share a derivation:
//!
//! 1. **The generator's own pipeline** — `canon_lift::project`, YAML -> canonical JSON -> CID.
//! 2. **The registry file's literal text** — `contentHash:` lines lifted by a raw line scan, with
//!    no YAML parse and no canonicalization in the path. This is the anti-mirror half: if the
//!    Rust canonicalization ever drifts from `_lib.epr_meta.policy_content_hash`, the computed
//!    digest stops matching the pin the file literally declares, on every affected row.
//! 3. **The frozen Python fixture** `elohim/epr/tests/vectors/concern_canon_row_c0.json`, written
//!    by `.claude/scripts/_lib/__tests__/concern_canon_liftability_test.py --write-fixture` and
//!    kept honest by that same test, which regenerates the body from the live YAML and asserts
//!    byte-equality. That test guards the reverse direction (fixture vs live canon); this one
//!    asserts the generator reproduces the fixture byte for byte. Neither can drift alone.
//!
//! The freshness half then byte-compares a fresh render (into a tempdir) against the committed
//! artifacts, so a canon edit that leaves the projection behind fails here rather than shipping a
//! brit lift of bytes that no longer exist.
//!
//! # When the registries are not reachable
//!
//! `epr-cli` lives in the `eprfs` workspace and is built in-tree, but a checkout without
//! `.claude/epr-meta` is a legitimate state. C5 governs: evidence that cannot be evaluated
//! confers nothing in either direction, so these report themselves UNEVALUATED rather than
//! pretending to pass. Set `CANON_LIFT_REQUIRE_REGISTRIES=1` to make unreachable fatal.

use std::path::{Path, PathBuf};

use elohim_epr_cli::canon_lift::{self, canonical};

const CONCERNS_REL: &str = ".claude/epr-meta/concerns.yaml";
const POLICIES_REL: &str = ".claude/epr-meta/policies.yaml";
const FIXTURE_REL: &str = "elohim/epr/tests/vectors/concern_canon_row_c0.json";
const FIXTURE_ROW: (&str, u64) = ("c0-plane-location", 1);

fn repo_root() -> Option<PathBuf> {
    let mut dir: &Path = Path::new(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        if dir.join(POLICIES_REL).is_file() && dir.join(CONCERNS_REL).is_file() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

/// `Some(root)` or an UNEVALUATED note (C5).
fn root_or_skip(what: &str) -> Option<PathBuf> {
    match repo_root() {
        Some(root) => Some(root),
        None => {
            assert!(
                std::env::var("CANON_LIFT_REQUIRE_REGISTRIES").is_err(),
                "CANON_LIFT_REQUIRE_REGISTRIES is set but .claude/epr-meta was not found above {}",
                env!("CARGO_MANIFEST_DIR")
            );
            eprintln!(
                "{what}: UNEVALUATED — .claude/epr-meta/{{policies,concerns}}.yaml not reachable \
                 from {}. This check confers NOTHING in either direction (C5). Set \
                 CANON_LIFT_REQUIRE_REGISTRIES=1 to make it fatal.",
                env!("CARGO_MANIFEST_DIR")
            );
            None
        }
    }
}

/// Every `contentHash:` pin in a registry, keyed by the `- id:`/`version:` block it sits in —
/// lifted by a LITERAL line scan. No YAML parser, no canonicalization: this side must not share a
/// single line of derivation with the generator, or it is a mirror.
fn pins_by_literal_scan(text: &str) -> Vec<((String, u64), String)> {
    let mut out = Vec::new();
    let mut id: Option<String> = None;
    let mut version: Option<u64> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("  - id: ") {
            id = Some(rest.trim().to_string());
            version = None;
        } else if let Some(rest) = line.strip_prefix("    version: ") {
            version = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("    contentHash: ") {
            if let (Some(i), Some(v)) = (id.clone(), version) {
                out.push(((i, v), rest.trim().to_string()));
            }
        }
    }
    out
}

#[test]
fn regenerating_reproduces_the_committed_artifacts_byte_for_byte() {
    let Some(root) = root_or_skip("canon_lift freshness") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");

    // Regenerate into a tempdir through the REAL write path (atomic rename and all), then read
    // the files back off disk — a render that only ever lives in memory would not catch a
    // serialization/IO asymmetry.
    let tmp = tempfile::tempdir().expect("tempdir");
    canon_lift::write_to(&projection, tmp.path()).expect("write projection");

    let committed = root.join(canon_lift::OUT_DIR_REL);
    for (name, _) in &projection.files {
        let fresh = std::fs::read(tmp.path().join(name)).expect("fresh artifact");
        let have = std::fs::read(committed.join(name)).unwrap_or_else(|e| {
            panic!(
                "{}/{name} missing ({e}) — run `epr canon-lift --write`",
                canon_lift::OUT_DIR_REL
            )
        });
        assert_eq!(
            have.len(),
            fresh.len(),
            "{}/{name} is STALE ({}B committed vs {}B regenerated) — the canon moved and its \
             brit-lift projection did not. Run `epr canon-lift --write`.",
            canon_lift::OUT_DIR_REL,
            have.len(),
            fresh.len()
        );
        assert!(
            have == fresh,
            "{}/{name} differs from a fresh render at the same length — run \
             `epr canon-lift --write`.",
            canon_lift::OUT_DIR_REL
        );
    }
}

#[test]
fn the_projection_reports_no_findings() {
    let Some(root) = root_or_skip("canon_lift findings") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");
    assert!(
        projection.errors.is_empty(),
        "canon-lift findings (pin mismatch / unpinned row / dangling lineage): {:#?}",
        projection.errors
    );
}

/// ANTI-MIRROR (a): every computed digest against the pin the registry file LITERALLY declares.
#[test]
fn every_computed_digest_matches_the_pin_the_registry_text_declares() {
    let Some(root) = root_or_skip("canon_lift pin parity") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");

    let mut literal = Vec::new();
    for (rel, home) in [
        (POLICIES_REL, "policies.yaml"),
        (CONCERNS_REL, "concerns.yaml"),
    ] {
        let text = std::fs::read_to_string(root.join(rel)).expect("registry readable");
        for (key, pin) in pins_by_literal_scan(&text) {
            literal.push((home, key, pin));
        }
    }
    assert!(
        !literal.is_empty(),
        "the literal scan found no `contentHash:` lines — the independent side of this check is \
         dead, and a dead instrument reads as 'nothing to report'"
    );
    assert_eq!(
        literal.len(),
        projection.atoms.len(),
        "the literal scan sees {} pinned rows but the projection minted {} atoms — one side is \
         missing rows",
        literal.len(),
        projection.atoms.len()
    );

    for (home, (id, version), pin) in &literal {
        let atom = projection
            .atoms
            .iter()
            .find(|a| a.registry == *home && a.id == *id && a.version == *version)
            .unwrap_or_else(|| panic!("{home}#{id}@{version} minted no atom"));
        assert_eq!(
            &atom.sha256_computed, pin,
            "{home}#{id}@{version}: the canonical body hashes to {} but the registry literally \
             pins {pin}. The Rust canonicalization has drifted from \
             `_lib.epr_meta.policy_content_hash` — the brit lift would be a REWRITE, not a move.",
            atom.sha256_computed
        );
        assert_eq!(atom.pin_state, "verified");
    }
}

/// The P0.4 guarantee, generalized: for EVERY row (not just the fixture), the CID is
/// `CIDv1(dag-cbor, sha2-256)` whose multihash digest is the sha256 pin — one hash, two
/// renderings, so the lift is a codec wrap of bytes that already exist.
#[test]
fn every_atom_is_cidv1_dag_cbor_over_the_pinned_bytes() {
    let Some(root) = root_or_skip("canon_lift cid shape") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");
    for atom in &projection.atoms {
        let cid: cid::Cid = atom.cid.parse().expect("atom cid parses");
        assert_eq!(cid.version(), cid::Version::V1, "{} not CIDv1", atom.id);
        assert_eq!(cid.codec(), 0x71, "{} not dag-cbor", atom.id);
        assert_eq!(cid.hash().code(), 0x12, "{} not sha2-256", atom.id);
        assert_eq!(
            format!("sha256:{}", canonical::hex_lower(cid.hash().digest())),
            atom.sha256_pin.clone().unwrap_or_default(),
            "{}@{}: the CID digest and the registry pin are not the same 32 bytes",
            atom.id,
            atom.version
        );
        // CIDv1 + dag-cbor + sha2-256 renders in base32 with the `bafyrei` prefix.
        assert!(atom.cid.starts_with("bafyrei"), "{}", atom.cid);
    }
}

/// ANTI-MIRROR (b): the generator's canonical body for the fixture row must equal the frozen
/// Python fixture byte for byte. The Python sibling test guards the reverse direction (fixture vs
/// live canon), so the two together pin all three sources to one another.
#[test]
fn the_fixture_row_canonicalizes_to_the_frozen_python_fixture() {
    let Some(root) = root_or_skip("canon_lift fixture parity") else {
        return;
    };
    let fixture_path = root.join(FIXTURE_REL);
    let Ok(fixture) = std::fs::read(&fixture_path) else {
        eprintln!(
            "canon_lift fixture parity: UNEVALUATED — {FIXTURE_REL} not present; confers nothing \
             (C5)."
        );
        return;
    };

    // Re-derive the row's canonical body straight from the live YAML through the same
    // canonicalizer the generator uses, then compare against bytes written by the PYTHON tool.
    let text = std::fs::read_to_string(root.join(CONCERNS_REL)).expect("concerns.yaml");
    let doc: serde_yaml::Value = serde_yaml::from_str(&text).expect("concerns.yaml parses");
    let rows = doc
        .get("concerns")
        .and_then(|v| v.as_sequence())
        .expect("concerns list");
    let row = rows
        .iter()
        .find_map(|r| {
            let m = r.as_mapping()?;
            let id = m.get(serde_yaml::Value::String("id".into()))?.as_str()?;
            let version = m
                .get(serde_yaml::Value::String("version".into()))?
                .as_u64()?;
            (id == FIXTURE_ROW.0 && version == FIXTURE_ROW.1).then_some(m)
        })
        .unwrap_or_else(|| {
            panic!(
                "{}@{} absent from concerns.yaml",
                FIXTURE_ROW.0, FIXTURE_ROW.1
            )
        });

    let body = canonical::canonical_body(row).expect("canonical body");
    assert_eq!(
        String::from_utf8_lossy(&body),
        String::from_utf8_lossy(&fixture),
        "the Rust canonical body for {}@{} differs from the frozen Python fixture \
         ({FIXTURE_REL}). Either this canonicalization drifted, or the canon row moved and the \
         fixture was not regenerated — run \
         `python3 .claude/scripts/_lib/__tests__/concern_canon_liftability_test.py --write-fixture` \
         after re-reading the row.",
        FIXTURE_ROW.0,
        FIXTURE_ROW.1
    );
}

/// The two-plane split is structural, not decorative: every standing record must cite a content
/// CID that an atom actually minted, and must not embed the content it cites.
#[test]
fn every_standing_cites_a_minted_content_cid() {
    let Some(root) = root_or_skip("canon_lift two-plane split") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");
    assert_eq!(projection.standings.len(), projection.atoms.len());
    for standing in &projection.standings {
        let atom = projection
            .atoms
            .iter()
            .find(|a| a.cid == standing.cites_content_cid)
            .unwrap_or_else(|| {
                panic!(
                    "standing `{}` cites {} — no atom carries that address",
                    standing.precedent.id, standing.cites_content_cid
                )
            });
        assert_eq!(standing.cites_content_sha256, atom.sha256_computed);
        assert!(
            !standing
                .precedent
                .metadata_json
                .contains(&standing.cites_content_cid),
            "standing `{}` duplicates its content CID into metadata_json — two homes for one \
             fact is exactly what the two-plane split exists to prevent",
            standing.precedent.id
        );
        // `citations` and the timestamps are minted at DHT residency, never fabricated here.
        assert!(standing.precedent.citations.is_none());
        assert!(standing.precedent.established_at.is_none());
    }
}

/// Every declared head resolves `pinned`, names a version that exists, and — the tip semantics
/// the registries themselves declare — is the ACTIVE version whenever exactly one exists.
#[test]
fn declared_heads_resolve_pinned_over_a_real_version() {
    let Some(root) = root_or_skip("canon_lift declared heads") else {
        return;
    };
    let projection = canon_lift::project(&root).expect("canon-lift projection");
    assert!(!projection.heads.is_empty());
    for head in &projection.heads {
        assert_eq!(
            head.resolution_mode, "pinned",
            "`{}` resolves `{}` — a governing artifact resolves by pin, never by election",
            head.id, head.resolution_mode
        );
        assert!(
            head.versions.iter().any(|v| v.version == head.head.version
                && v.registry == head.head.registry
                && v.cid == head.head.cid),
            "`{}` head points at a version absent from its own DAG",
            head.id
        );
        if head.tip_basis == "sole-active" {
            let tip = head
                .versions
                .iter()
                .find(|v| v.version == head.head.version && v.registry == head.head.registry)
                .unwrap();
            assert_ne!(tip.status, "superseded", "`{}` head is superseded", head.id);
        }
        assert!(
            !head.competing,
            "`{}` has competing active versions",
            head.id
        );
    }
}
