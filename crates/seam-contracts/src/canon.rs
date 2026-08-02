//! Cross-repo canon pins — the concern canon as compile-time constants.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (Design surface 7 "SDK inheritance surface"; task P4.6).
//!
//! ## What this module is for
//!
//! Inside this repo a policy version bump becomes a **fingerprinted finding**: the census diffs
//! `policy: <id>@<version>` pins in every `.epr-meta` binding and each stale pin dispatches
//! (Design surface 5, the cascade). An **external** peer runtime has no `.epr-meta`, no census,
//! and no Claude tooling — so the same cascade needs a shape its own `cargo build` can deliver.
//! That shape is here:
//!
//! - [`CONCERN_CANON_VERSION`] — the canon's own version, so a consumer can gate on it;
//! - one [`PolicyPin`] const per class — `{id, version, content_hash}`, so a consumer *declares*
//!   which version it conformed to instead of tracking recency (mode 2, **pinned**: which version
//!   binds a consumer is a declared dependency, and a new version moves nothing until each
//!   binding re-declares);
//! - `#[deprecated]` on every superseded pin — the version bump's compile-time analog of a
//!   fingerprinted finding. The external build emits a warning **naming the exact policy that
//!   moved** and where to re-pin.
//!
//! ## The semver rule (never silently redefine)
//!
//! | Change | Crate semver | What ships |
//! |---|---|---|
//! | Tightening a policy (new version, same guarantee) | **minor** | the new `_V<n>` pin **plus** `#[deprecated]` on the old — both remain referenceable |
//! | Removing a pin entirely | **major** | the class left the canon; a consumer's reference stops compiling, which is the point |
//! | Redefining a pin's `content_hash` or semantics in place | **forbidden at any version** | — |
//!
//! The third row is the load-bearing one. `id` is identity; `version` is that identity's
//! monotonic life; `content_hash` is the seal over the bytes that version means. Re-pointing an
//! existing const at new bytes would make every downstream conformance claim retroactively false
//! while every build stayed green — the exact silent-corruption class C2 exists to forbid, turned
//! on the canon itself. A landed version is immutable; a change is a NEW version with supersession
//! lineage, recorded in `elohim/sdk/schemas/v1/registries/concern-canon-changelog.json`.
//!
//! ## Anti-mirror: these consts are hand-written, and the test does not trust them
//!
//! The pins below are authored by hand and [`canon_conformance`] verifies them against the
//! **live YAML registries** (`.claude/epr-meta/policies.yaml` + `.claude/epr-meta/concerns.yaml`),
//! parsed independently — *not* against `concern-classes.json`, which is generated from the same
//! registries by the same rule and would therefore be a mirror. A contract test that computes
//! both sides through one helper measures the helper (`9e9c9d023`, 2026-07-24: `BlobCid::verifies`
//! hardcoded the wrong codec and the test computed both sides the same wrong way, staying green
//! for every real fingerprint).
//!
//! A **generated** `include!` was the alternative and was declined for two reasons: it would put
//! the registry-to-Rust derivation in exactly one place — so nothing would be left to check — and
//! it would make this crate unbuildable outside the monorepo, which is the one property an SDK
//! inheritance surface may not lose.
//!
//! ## When the registries are not reachable
//!
//! An external consumer running `cargo test` on the published crate has no `.claude/epr-meta`.
//! C5 governs: **evidence that cannot be evaluated confers nothing, in either direction** — so the
//! conformance test reports itself UNEVALUATED and does not pretend to pass a check it never ran
//! (`4d90bf276`, 2026-07-25: an unresolvable validator may neither harden nor soften a rule).
//! In-tree CI always has the files; set `SEAM_CONTRACTS_REQUIRE_REGISTRIES=1` to turn
//! "unreachable" into a hard failure wherever that must be guaranteed.

/// A declared dependency on one version of one concern-canon policy.
///
/// **Concerns:** C5 (evidence-not-authority — `content_hash` is what a consumer re-derives, so
/// the pin confers only what the bytes prove); C2 (monotonic authority — a pin is how a consumer
/// refuses to be moved by recency); C7 (advertise/serve symmetry — [`canon_conformance`] is the
/// check that what this module advertises is what the registries serve).
///
/// **Forcing incident:** `a83b35079` (2026-07-29) — the epr-cli/REA sidecar re-minted a cured
/// ordering defect through replay because nothing in the consumer *declared* which version of the
/// rule it had conformed to; the cure had no way to announce that it had moved.
///
/// **Contract test:** [`canon_conformance`].
///
/// # Example
///
/// ```
/// use seam_contracts::canon::{PIN_C4_HONEST_ABSENCE, CONCERN_CANON_VERSION};
///
/// // An external peer runtime records what it conformed to.
/// assert_eq!(PIN_C4_HONEST_ABSENCE.id, "c4-honest-absence");
/// assert_eq!(PIN_C4_HONEST_ABSENCE.version, 1);
/// assert_eq!(CONCERN_CANON_VERSION, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PolicyPin {
    /// Registry row id, e.g. `"c2-monotonic-authority"`. Identity — never reused for a different
    /// guarantee.
    pub id: &'static str,
    /// The row version this pin declares. Monotonic; a bump is a new const, never an edit.
    pub version: u32,
    /// `sha256:<hex>` over the row's canonical-JSON body (the same seal `epr-meta-pin.py --write`
    /// computes). A consumer that holds the registry can re-derive it; a consumer that does not
    /// can still compare two checkouts' pins and see that the bytes moved.
    pub content_hash: &'static str,
    /// The canon class id (`"C0"`..`"C14"`). Derived from the row, verified by the conformance
    /// test — carried here so a consumer can index by class without shipping the projection.
    pub concern: &'static str,
    /// Which registry home holds the row: `"policies.yaml"` (an evaluable predicate exists) or
    /// `"concerns.yaml"` (Precedent-shaped, no predicate). A class lives in exactly one home, and
    /// a *graduation across homes* is itself a version bump (C6a `@1` -> `@2`).
    pub home: &'static str,
}

/// The concern canon's own version, as declared by `.claude/epr-meta/concerns.yaml`'s
/// `concern-canon-version`.
///
/// **Concerns:** C10 (contract-evolution honesty — a consumer gates on this instead of guessing
/// which classes exist).
///
/// **Forcing incident:** `270dbafac` (2026-07-11) — `ice_servers`/`iceServers`: a contract that
/// changed shape with nothing to gate on ran the whole fleet with zero ICE servers since
/// inception, silently.
///
/// **Contract test:** [`canon_conformance`].
///
/// Bumped when the canon's *structure* changes (a class admitted or retired), not when an
/// individual row's version moves — row moves travel through the per-class pins and the changelog.
pub const CONCERN_CANON_VERSION: u32 = 1;

/// C0 — plane location. Name the plane before binding any concern.
pub const PIN_C0_PLANE_LOCATION: PolicyPin = PolicyPin {
    id: "c0-plane-location",
    version: 1,
    content_hash: "sha256:df38bb561769ff552e76d63391996e41093a70621a47fae47ac25fcfa3500505",
    concern: "C0",
    home: "concerns.yaml",
};

/// C1 — anti-self-election. No component crowns what it authored (except the guarded arm).
pub const PIN_C1_ANTI_SELF_ELECTION: PolicyPin = PolicyPin {
    id: "c1-anti-self-election",
    version: 1,
    content_hash: "sha256:6786f77a5deca569a81ab88d317575f2d5b61cf68c4aaf2bbb9fe9354754039f",
    concern: "C1",
    home: "concerns.yaml",
};

/// C2 v1 — **superseded**. The live pin is [`PIN_C2_MONOTONIC_AUTHORITY_V2`].
///
/// Retained (not deleted) so an external consumer that pinned `@1` keeps compiling *and* gets
/// told, by name, that the policy moved — the compile-time analog of the in-repo cascade's
/// fingerprinted finding. Deleting it would be a major bump and would break the build without
/// explaining anything.
///
/// Migration: `elohim/sdk/schemas/v1/registries/concern-canon-changelog.json`, row
/// `c2-monotonic-authority 1 -> 2`. The guarantee is unchanged; only the edit-time predicate
/// graduated (`dedupe-of` advisory -> `epr:validator-heal-fills-never-moves`), so re-pinning is a
/// one-line edit with no code change.
#[deprecated(
    since = "0.1.0",
    note = "concern-canon policy `c2-monotonic-authority` moved @1 -> @2 (2026-08-02): the \
            bind-don't-redefine `dedupe-of` advisory graduated to the real detector \
            `epr:validator-heal-fills-never-moves`. Same guarantee, sharper predicate. Re-pin to \
            PIN_C2_MONOTONIC_AUTHORITY_V2; migration note in \
            elohim/sdk/schemas/v1/registries/concern-canon-changelog.json"
)]
pub const PIN_C2_MONOTONIC_AUTHORITY_V1: PolicyPin = PolicyPin {
    id: "c2-monotonic-authority",
    version: 1,
    content_hash: "sha256:a5a4d533a03eac9915d0b132ff374a834954446bad25ffd32e6cc9e8290ca1d9",
    concern: "C2",
    home: "policies.yaml",
};

/// C2 — monotonic authority. Never backwards; name the notarized clock; break ties deterministically.
pub const PIN_C2_MONOTONIC_AUTHORITY_V2: PolicyPin = PolicyPin {
    id: "c2-monotonic-authority",
    version: 2,
    content_hash: "sha256:373c0f803c0e7f2ffb2385dcb1278e2cb59d138b0500e4ab63e9ae578d9b9a34",
    concern: "C2",
    home: "policies.yaml",
};

/// C3 — liveness. Every reachable non-terminal state has an enabled *automated* transition.
pub const PIN_C3_LIVENESS: PolicyPin = PolicyPin {
    id: "c3-liveness",
    version: 1,
    content_hash: "sha256:8bb97f24fde37f07e62e05dfb11c12bce47b35e6ac846dfd217588a869055746",
    concern: "C3",
    home: "concerns.yaml",
};

/// C4 — honest absence. Absent, unreachable, refused and unverifiable are distinct answers.
pub const PIN_C4_HONEST_ABSENCE: PolicyPin = PolicyPin {
    id: "c4-honest-absence",
    version: 1,
    content_hash: "sha256:caffa2cb3e1ae7858458aa8c5cfb6d8e8515daeb9a0668e3b1c8ba455d1b6cb5",
    concern: "C4",
    home: "concerns.yaml",
};

/// C5 — evidence-not-authority. A transferred claim confers only what the receiver re-derives.
pub const PIN_C5_EVIDENCE_NOT_AUTHORITY: PolicyPin = PolicyPin {
    id: "c5-evidence-not-authority",
    version: 1,
    content_hash: "sha256:2350a0809ba49c4fa64fa4f403924e35aab861e7986d03c1ed7f1319f93e8229",
    concern: "C5",
    home: "concerns.yaml",
};

/// C6a v1 — **superseded**. The live pin is [`PIN_C6A_BOUNDED_WORK_V2`].
///
/// This one also changed *home*: `@1` was a Precedent row in `concerns.yaml` (no predicate);
/// registering `epr:validator-bounded-work` moved the class to `policies.yaml` as `@2`. The
/// version continues across the move because the artifact is the same artifact, and because two
/// rows keyed `c6a-bounded-work@1` in two files would make a pin ambiguous — the one thing a
/// declared dependency may never be.
#[deprecated(
    since = "0.1.0",
    note = "concern-canon policy `c6a-bounded-work` moved @1 -> @2 (2026-08-02) AND changed home \
            (concerns.yaml -> policies.yaml): the class acquired an evaluable predicate, \
            `epr:validator-bounded-work`. Re-pin to PIN_C6A_BOUNDED_WORK_V2; migration note in \
            elohim/sdk/schemas/v1/registries/concern-canon-changelog.json"
)]
pub const PIN_C6A_BOUNDED_WORK_V1: PolicyPin = PolicyPin {
    id: "c6a-bounded-work",
    version: 1,
    content_hash: "sha256:9f3134f0d23dfa0f1697aef66bd39b3432293d1a0084023f705520a0389f6a4a",
    concern: "C6a",
    home: "concerns.yaml",
};

/// C6a — bounded work. Every loop, sweep and retry ladder respects a declared budget.
pub const PIN_C6A_BOUNDED_WORK_V2: PolicyPin = PolicyPin {
    id: "c6a-bounded-work",
    version: 2,
    content_hash: "sha256:88b41ea4940d3e461c4727128c2907e0f1f1e47a14f52c132d56e19fda7b6617",
    concern: "C6a",
    home: "policies.yaml",
};

/// C6b — idempotent effect. Replay against settled state mints nothing and double-applies nothing.
pub const PIN_C6B_IDEMPOTENT_EFFECT: PolicyPin = PolicyPin {
    id: "c6b-idempotent-effect",
    version: 1,
    content_hash: "sha256:055ecc7d50692aef933b336bf60f539f16c472a4b0886374fde01cb993d2cb56",
    concern: "C6b",
    home: "concerns.yaml",
};

/// C7 — advertise/serve symmetry. What a surface advertises equals what it serves.
pub const PIN_C7_ADVERTISE_SERVE_SYMMETRY: PolicyPin = PolicyPin {
    id: "c7-advertise-serve-symmetry",
    version: 1,
    content_hash: "sha256:ba502823e801e8a0e91e50cf7545b97595865fce068af3c3290f58f562267576",
    concern: "C7",
    home: "concerns.yaml",
};

/// C8 — observability-per-decision. Every outcome increments a labeled counter through a typed reason.
pub const PIN_C8_OBSERVABILITY_PER_DECISION: PolicyPin = PolicyPin {
    id: "c8-observability-per-decision",
    version: 1,
    content_hash: "sha256:598145dfcae8f2e72236ac8199565d0824c9bff037f2b48553c31c9a387c88d9",
    concern: "C8",
    home: "concerns.yaml",
};

/// C9 — identity-lineage continuity. A re-key never silently orphans state keyed on the old identity.
pub const PIN_C9_IDENTITY_LINEAGE_CONTINUITY: PolicyPin = PolicyPin {
    id: "c9-identity-lineage-continuity",
    version: 1,
    content_hash: "sha256:c2114b626c5a968b74323074fc061b41133aae5fd4a97ae9d01806e5f9cfdef2",
    concern: "C9",
    home: "concerns.yaml",
};

/// C10 — contract-evolution honesty. A wire/config/schema change is rejected or observed, never defaulted.
pub const PIN_C10_CONTRACT_EVOLUTION_HONESTY: PolicyPin = PolicyPin {
    id: "c10-contract-evolution-honesty",
    version: 1,
    content_hash: "sha256:7eb05838e13be3eda8a20f64fbd948fd2e2a4400bbb108b331724098bd0ef97c",
    concern: "C10",
    home: "concerns.yaml",
};

/// C11 — externally-imposed backpressure. Degrade by a declared, counted policy; never by OOM.
pub const PIN_C11_EXTERNALLY_IMPOSED_BACKPRESSURE: PolicyPin = PolicyPin {
    id: "c11-externally-imposed-backpressure",
    version: 1,
    content_hash: "sha256:3c5f870f3785f2c4375de13febffeca621808b221d3fded7aa528fbb798b9fce",
    concern: "C11",
    home: "concerns.yaml",
};

/// C12 — consent/authorization (may-act). Authority is verified structurally at the acting node.
pub const PIN_C12_CONSENT_AUTHORIZATION: PolicyPin = PolicyPin {
    id: "c12-consent-authorization",
    version: 1,
    content_hash: "sha256:7479976f0ff983157ec79cb739427ee73e2d66e87ab2e5e51154498cd1951667",
    concern: "C12",
    home: "concerns.yaml",
};

/// C13 — graduated authority. Every scaffold names its successor criterion and graduation trigger.
pub const PIN_C13_GRADUATED_AUTHORITY: PolicyPin = PolicyPin {
    id: "c13-graduated-authority",
    version: 1,
    content_hash: "sha256:5ca1d2733c81ff5ceb27d5afd73ed5fa1b12b487b6e900f7be9d511c26c0b7df",
    concern: "C13",
    home: "concerns.yaml",
};

/// C14 — witnessed residual. The outcome set closes with an arm that is witnessed, never dropped.
///
/// The capsule this class contracts for is [`crate::residual::ResidualWitness`].
pub const PIN_C14_WITNESSED_RESIDUAL: PolicyPin = PolicyPin {
    id: "c14-witnessed-residual",
    version: 1,
    content_hash: "sha256:2ec2de528770ad1c492ed58557fcce8fb1cc88c602d79600c4fe149a5b6ae810",
    concern: "C14",
    home: "concerns.yaml",
};

/// Every **live** pin, one per concern class, in canon order (`C0`..`C14`, `C6` split).
///
/// **Concerns:** C7 (advertise/serve symmetry — this slice is what the crate advertises as "the
/// canon"; [`canon_conformance`] proves the registries serve exactly it).
///
/// Superseded pins are deliberately absent: this is the *tip* set, the same rule
/// `concern-classes.json` follows. A consumer walking `ALL_PINS` and finding a pin it has not
/// declared conformance to has found its own gap.
pub const ALL_PINS: &[PolicyPin] = &[
    PIN_C0_PLANE_LOCATION,
    PIN_C1_ANTI_SELF_ELECTION,
    PIN_C2_MONOTONIC_AUTHORITY_V2,
    PIN_C3_LIVENESS,
    PIN_C4_HONEST_ABSENCE,
    PIN_C5_EVIDENCE_NOT_AUTHORITY,
    PIN_C6A_BOUNDED_WORK_V2,
    PIN_C6B_IDEMPOTENT_EFFECT,
    PIN_C7_ADVERTISE_SERVE_SYMMETRY,
    PIN_C8_OBSERVABILITY_PER_DECISION,
    PIN_C9_IDENTITY_LINEAGE_CONTINUITY,
    PIN_C10_CONTRACT_EVOLUTION_HONESTY,
    PIN_C11_EXTERNALLY_IMPOSED_BACKPRESSURE,
    PIN_C12_CONSENT_AUTHORIZATION,
    PIN_C13_GRADUATED_AUTHORITY,
    PIN_C14_WITNESSED_RESIDUAL,
];

/// The live pin for a concern class id (`"C0"`..`"C14"`), or `None` if the id is not a canon class.
///
/// **Concerns:** C4 (honest absence — an unknown class id yields `None`, which here means
/// *observed absence from a closed set*, not "lookup failed"; the set is [`ALL_PINS`] and it is
/// entirely in memory, so there is no second provenance to keep apart).
///
/// **Contract test:** `pin_for_concern_covers_every_class`.
pub fn pin_for_concern(concern: &str) -> Option<PolicyPin> {
    ALL_PINS.iter().copied().find(|p| p.concern == concern)
}

#[cfg(test)]
mod canon_conformance {
    use super::*;
    use std::path::{Path, PathBuf};

    /// One row as the registries declare it — parsed from YAML *independently* of any Python
    /// helper, any generated JSON, and any const in this module. Deliberately a tiny hand-rolled
    /// scanner rather than a YAML dependency: pulling `serde_yaml` in would put a parser in the
    /// leaf crate's lockfile and trip `no_heavy_deps_in_dep_tree` — and the registry rows are a
    /// fixed two-space-list / four-space-field shape whose drift would itself be a finding.
    #[derive(Debug, Default, Clone)]
    struct Row {
        id: String,
        version: u32,
        content_hash: String,
        concern: String,
        status: String,
        home: &'static str,
    }

    fn parse_rows(text: &str, home: &'static str) -> Vec<Row> {
        let mut rows: Vec<Row> = Vec::new();
        let mut cur: Option<Row> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("  - id: ") {
                if let Some(r) = cur.take() {
                    rows.push(r);
                }
                cur = Some(Row {
                    id: rest.trim().to_string(),
                    home,
                    ..Row::default()
                });
                continue;
            }
            if line.starts_with("  - ") {
                // a list item that is not a row start closes the current row
                if let Some(r) = cur.take() {
                    rows.push(r);
                }
                continue;
            }
            let Some(row) = cur.as_mut() else { continue };
            // Row fields sit at EXACTLY four spaces; nested blocks (scope/recurrence/statement
            // bodies) sit deeper and must not be read as row fields.
            if !line.starts_with("    ") || line.starts_with("     ") {
                continue;
            }
            let Some((key, value)) = line[4..].split_once(": ") else {
                continue;
            };
            let value = value.trim();
            match key {
                "version" => row.version = value.parse().unwrap_or(0),
                "contentHash" => row.content_hash = value.to_string(),
                "concern" => row.concern = value.to_string(),
                "status" => row.status = value.to_string(),
                _ => {}
            }
        }
        if let Some(r) = cur {
            rows.push(r);
        }
        rows
    }

    fn repo_root() -> Option<PathBuf> {
        let mut dir: &Path = Path::new(env!("CARGO_MANIFEST_DIR"));
        for _ in 0..8 {
            if dir.join(".claude/epr-meta/policies.yaml").is_file() {
                return Some(dir.to_path_buf());
            }
            dir = dir.parent()?;
        }
        None
    }

    /// Load both homes, or report the registries UNEVALUATED (C5).
    fn live_rows() -> Option<Vec<Row>> {
        let require = std::env::var("SEAM_CONTRACTS_REQUIRE_REGISTRIES").is_ok();
        let Some(root) = repo_root() else {
            assert!(
                !require,
                "SEAM_CONTRACTS_REQUIRE_REGISTRIES is set but .claude/epr-meta was not found \
                 above {}",
                env!("CARGO_MANIFEST_DIR")
            );
            eprintln!(
                "canon_conformance: UNEVALUATED — .claude/epr-meta/{{policies,concerns}}.yaml \
                 not reachable from {}. This is the published-crate case: the pins cannot be \
                 checked against their source here, so this check confers NOTHING in either \
                 direction (C5). Set SEAM_CONTRACTS_REQUIRE_REGISTRIES=1 to make it fatal.",
                env!("CARGO_MANIFEST_DIR")
            );
            return None;
        };
        let mut rows = Vec::new();
        for (rel, home) in [
            (".claude/epr-meta/policies.yaml", "policies.yaml"),
            (".claude/epr-meta/concerns.yaml", "concerns.yaml"),
        ] {
            let text = std::fs::read_to_string(root.join(rel))
                .unwrap_or_else(|e| panic!("{rel} unreadable: {e}"));
            rows.extend(parse_rows(&text, home));
        }
        Some(rows)
    }

    /// Canon rows only (a `concern:` field), keyed by (id, version).
    fn canon_rows(rows: &[Row]) -> Vec<&Row> {
        rows.iter().filter(|r| !r.concern.is_empty()).collect()
    }

    /// The parser is the one thing this test file cannot check against something else, so it is
    /// checked against a fixture whose expected values are written out by hand.
    #[test]
    fn parser_reads_a_row_shape_it_did_not_write() {
        const FIXTURE: &str = concat!(
            "policies:\n",
            "  - id: some-other-policy\n",
            "    version: 3\n",
            "    class: measure\n",
            "  # a comment between rows\n",
            "  - id: c2-monotonic-authority\n",
            "    version: 1\n",
            "    contentHash: sha256:deadbeef\n",
            "    concern: C2\n",
            "    scope:\n",
            "      write: \"*.rs\"\n",
            "      contains-any: [\"StampMode\"]\n",
            "    recurrence:\n",
            "      unit: distinct-shifts\n",
            "      instances:\n",
            "        - \"version: 99 inside a quoted instance string\"\n",
            "    status: superseded\n",
            "    superseded_by: c2-monotonic-authority@2\n",
        );
        let rows = parse_rows(FIXTURE, "policies.yaml");
        assert_eq!(rows.len(), 2, "expected two rows, got {rows:?}");
        assert_eq!(rows[0].id, "some-other-policy");
        assert!(
            rows[0].concern.is_empty(),
            "a non-canon policy row must not acquire a concern id"
        );
        let c2 = &rows[1];
        assert_eq!(c2.id, "c2-monotonic-authority");
        assert_eq!(
            c2.version, 1,
            "nested `version: 99` inside a quoted instance must NOT overwrite the row version"
        );
        assert_eq!(c2.content_hash, "sha256:deadbeef");
        assert_eq!(c2.concern, "C2");
        assert_eq!(c2.status, "superseded");
    }

    /// Every live pin matches an ACTIVE registry row, byte for byte.
    #[test]
    fn every_live_pin_matches_an_active_registry_row() {
        let Some(rows) = live_rows() else { return };
        for pin in ALL_PINS {
            let row = canon_rows(&rows)
                .into_iter()
                .find(|r| r.id == pin.id && r.version == pin.version)
                .unwrap_or_else(|| {
                    panic!(
                        "pin {}@{} ({}) names no row in either registry home. Either the row was \
                         deleted (a MAJOR bump — remove the const) or the pin was hand-edited to \
                         a version that never landed.",
                        pin.id, pin.version, pin.concern
                    )
                });
            assert_eq!(
                row.status, "active",
                "pin {}@{} is in ALL_PINS but its registry row is `{}` — ALL_PINS is the TIP set; \
                 a superseded row belongs behind a #[deprecated] const, never here.",
                pin.id, pin.version, row.status
            );
            assert_eq!(
                row.content_hash, pin.content_hash,
                "pin {}@{} content_hash DRIFTED (const {}, registry {}). A landed version is \
                 immutable: if the row's bytes changed, that is a NEW version with supersession \
                 lineage — never a re-pointed const, which would make every downstream \
                 conformance claim retroactively false while the build stayed green.",
                pin.id, pin.version, pin.content_hash, row.content_hash
            );
            assert_eq!(
                row.concern, pin.concern,
                "pin {} concern id drifted",
                pin.id
            );
            assert_eq!(
                row.home, pin.home,
                "pin {}@{} declares home {} but the row lives in {} — a home change is a \
                 graduation, i.e. a version bump with a changelog row.",
                pin.id, pin.version, pin.home, row.home
            );
        }
    }

    /// And the other direction: every active canon row has a pin (C7 — advertise == serve).
    #[test]
    fn every_active_registry_row_has_a_pin() {
        let Some(rows) = live_rows() else { return };
        for row in canon_rows(&rows) {
            if row.status != "active" {
                continue;
            }
            assert!(
                ALL_PINS
                    .iter()
                    .any(|p| p.id == row.id && p.version == row.version),
                "registry row {}@{} ({}) is ACTIVE canon but has NO PolicyPin const. An external \
                 consumer would inherit {} classes while the canon declares {} — the crate would \
                 advertise a canon it does not serve (C7). Add the const AND the ALL_PINS entry.",
                row.id,
                row.version,
                row.concern,
                ALL_PINS.len(),
                canon_rows(&rows)
                    .iter()
                    .filter(|r| r.status == "active")
                    .count(),
            );
        }
    }

    /// Superseded rows are exactly the deprecated consts — the cascade's compile-time surface.
    #[test]
    fn superseded_rows_are_carried_as_deprecated_pins() {
        let Some(rows) = live_rows() else { return };
        #[allow(deprecated)]
        let deprecated_pins = [PIN_C2_MONOTONIC_AUTHORITY_V1, PIN_C6A_BOUNDED_WORK_V1];
        for pin in deprecated_pins {
            let row = canon_rows(&rows)
                .into_iter()
                .find(|r| r.id == pin.id && r.version == pin.version)
                .unwrap_or_else(|| {
                    panic!("deprecated pin {}@{} names no row", pin.id, pin.version)
                });
            assert_eq!(
                row.status, "superseded",
                "{}@{} is marked #[deprecated] here but is still ACTIVE in {} — the crate would \
                 warn consumers off a policy that still binds.",
                pin.id, pin.version, row.home
            );
            assert_eq!(row.content_hash, pin.content_hash);
        }
        let superseded: Vec<_> = canon_rows(&rows)
            .into_iter()
            .filter(|r| r.status == "superseded")
            .collect();
        for row in &superseded {
            assert!(
                deprecated_pins
                    .iter()
                    .any(|p| p.id == row.id && p.version == row.version),
                "registry row {}@{} is superseded but carries NO #[deprecated] const. An external \
                 consumer pinned to it would keep building silently — which is exactly the \
                 fingerprinted-finding cascade failing to cross the repo boundary.",
                row.id,
                row.version,
            );
        }
    }

    /// `CONCERN_CANON_VERSION` equals `concerns.yaml`'s own declaration.
    #[test]
    fn canon_version_matches_the_registry_declaration() {
        let Some(root) = repo_root() else {
            eprintln!("canon_version: UNEVALUATED — registries not reachable (C5)");
            return;
        };
        let text = std::fs::read_to_string(root.join(".claude/epr-meta/concerns.yaml")).unwrap();
        let declared: u32 = text
            .lines()
            .find_map(|l| l.strip_prefix("concern-canon-version:"))
            .and_then(|v| v.trim().parse().ok())
            .expect("concerns.yaml declares no `concern-canon-version`");
        assert_eq!(
            declared, CONCERN_CANON_VERSION,
            "CONCERN_CANON_VERSION ({CONCERN_CANON_VERSION}) disagrees with concerns.yaml \
             ({declared}). A consumer gating on the crate constant would gate on the wrong canon."
        );
    }

    #[test]
    fn pin_for_concern_covers_every_class() {
        for pin in ALL_PINS {
            assert_eq!(pin_for_concern(pin.concern), Some(*pin));
        }
        assert_eq!(pin_for_concern("C99"), None);
        assert_eq!(
            ALL_PINS.len(),
            16,
            "the canon is 16 classes (C6 splits a/b)"
        );
        let mut ids: Vec<_> = ALL_PINS.iter().map(|p| p.concern).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            ALL_PINS.len(),
            "two pins claim the same concern class"
        );
    }

    /// **The cross-repo cascade, demonstrated end to end.**
    ///
    /// Materializes a throwaway *external consumer* crate that references the superseded pin,
    /// builds it, and asserts the compiler warning names the exact policy that moved and where to
    /// re-pin. This is the compile-time analog of a fingerprinted finding: inside the repo a stale
    /// pin becomes a census finding + a dispatch, and outside it becomes this.
    ///
    /// `trybuild` was the obvious tool and is deliberately not used — it would put a test
    /// framework, `glob`, `toml` and `termcolor` into this leaf crate's own lockfile and trip
    /// `no_heavy_deps_in_dep_tree`. The check that matters here is "does rustc emit the note we
    /// wrote", which `cargo build` + captured stderr answers directly.
    ///
    /// The output is **captured, never printed on success**: the repo's deprecation sentinel
    /// scans build output and would otherwise file this deliberate fixture as real deprecation
    /// debt every time the suite runs.
    ///
    /// Degrades to UNEVALUATED (C5) when cargo cannot run or cannot resolve offline — an
    /// unevaluatable check confers nothing in either direction, and a nested toolchain
    /// invocation is exactly the kind of thing that is legitimately unavailable in a sandbox.
    #[test]
    fn a_superseded_pin_warns_the_external_consumer_by_name() {
        use std::process::Command;

        let Some(root) = repo_root() else {
            eprintln!("deprecated-pin demo: UNEVALUATED — crate not in-tree (C5)");
            return;
        };
        let crate_dir = root.join("crates/seam-contracts");
        let tmp = std::env::temp_dir().join(format!(
            "seam-contracts-pin-consumer-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).expect("temp fixture dir");

        // Its own `[workspace]` table so it never joins any surrounding workspace.
        std::fs::write(
            tmp.join("Cargo.toml"),
            format!(
                "[workspace]\n\
                 [package]\nname = \"external-peer-runtime\"\nversion = \"0.0.0\"\n\
                 edition = \"2021\"\n\n\
                 [dependencies]\n\
                 elohim-seam-contracts = {{ path = \"{}\", default-features = false }}\n",
                crate_dir.display()
            ),
        )
        .unwrap();
        std::fs::write(
            tmp.join("src/main.rs"),
            "use seam_contracts::canon::PIN_C2_MONOTONIC_AUTHORITY_V1;\n\
             fn main() { println!(\"{}\", PIN_C2_MONOTONIC_AUTHORITY_V1.id); }\n",
        )
        .unwrap();

        let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["build", "--offline"])
            .current_dir(&tmp)
            .env("CARGO_TARGET_DIR", tmp.join("target"))
            .env("RUSTFLAGS", "")
            .output();
        let Ok(out) = out else {
            eprintln!("deprecated-pin demo: UNEVALUATED — cargo not runnable here (C5)");
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        };
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let _ = std::fs::remove_dir_all(&tmp);

        if !out.status.success() {
            // A resolution/offline failure is UNREACHABLE, not a failed contract (C4): the
            // consumer never got to compile, so nothing was learned in either direction.
            let compile_error = stderr.contains("error[E") || stderr.contains("cannot find");
            assert!(
                !compile_error,
                "the external-consumer fixture failed to COMPILE, which is a real regression \
                 (the pin const or its path changed):\n{stderr}"
            );
            eprintln!("deprecated-pin demo: UNEVALUATED — cargo build could not run (C5)");
            return;
        }

        for needle in [
            "use of deprecated constant",
            "PIN_C2_MONOTONIC_AUTHORITY_V1",
            "c2-monotonic-authority",
            "@1 -> @2",
            "PIN_C2_MONOTONIC_AUTHORITY_V2",
            "concern-canon-changelog.json",
        ] {
            assert!(
                stderr.contains(needle),
                "an external consumer referencing the superseded pin must be told, by name, \
                 WHICH policy moved and where to re-pin — the build output is missing {needle:?}. \
                 Without it the cross-repo cascade is a silent version bump, which is the whole \
                 failure this surface exists to prevent.\n--- captured stderr ---\n{stderr}"
            );
        }
    }

    /// Every pin's `content_hash` is a well-formed `sha256:<64 hex>` seal.
    #[test]
    fn pin_hashes_are_well_formed() {
        #[allow(deprecated)]
        let all = ALL_PINS
            .iter()
            .copied()
            .chain([PIN_C2_MONOTONIC_AUTHORITY_V1, PIN_C6A_BOUNDED_WORK_V1]);
        for pin in all {
            let hex = pin
                .content_hash
                .strip_prefix("sha256:")
                .unwrap_or_else(|| panic!("{} content_hash lacks the sha256: prefix", pin.id));
            assert_eq!(hex.len(), 64, "{} content_hash is not 32 bytes", pin.id);
            assert!(
                hex.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{} content_hash is not lowercase hex",
                pin.id
            );
        }
    }
}
