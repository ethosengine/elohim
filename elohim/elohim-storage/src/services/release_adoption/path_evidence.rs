//! **Rung 6.** The FETCH behind `VerifyInput.path` — Task 4 declared the
//! evidence and left every construction site passing `Answer::Absent`; this is
//! the site that reads it.
//!
//! # Why the fetch lives here and not in `verify`
//!
//! [`super::verify`] is pure by contract (its module docs: "this module does
//! no I/O"), which is what lets the whole floor be unit-tested against
//! fixtures with no conductor in the room. The evidence a floor check reads is
//! therefore always assembled by the caller. Every other floor input already
//! works this way — installed reality, the L2 lineage chain, the attestation
//! count — and the path is the fourth of the same shape, not a new one.
//!
//! # I1 / C5 — through THIS peer's own conductor, never a peer's word
//!
//! The commitment is read with `mishpat::get_commitment` over the peer's own
//! `HcClient` (the same bridge every other adoption read uses). A migration
//! path is authority, and authority is never taken from the party asking for
//! it: a manifest's `adoptionDiscipline.path` is a *claim* — a pointer at a
//! commitment CID — and this module turns that claim into evidence by looking
//! the commitment up locally. A release cannot ship its own permission.
//!
//! # C4 — Unreachable is never Absent
//!
//! Three outcomes, and the difference between the last two is the whole point:
//!
//! | outcome | meaning | what `verify_path` does |
//! |---|---|---|
//! | [`Answer::Present`] | the commitment is on this conductor's DHT view | checks it |
//! | [`Answer::Absent`] | the conductor answered, and answered "no such entry" | `path_not_notarized` |
//! | [`Answer::Unreachable`] | we could not ask, or could not read the answer | `conductor_unavailable` |
//!
//! A conductor bridge that is down must never read as "the elohim did not
//! notarize this path" — that would turn our own outage into a statement
//! about someone else's governance. So every failure to *ask* maps to
//! `Unreachable`, and only the conductor's own "not found" maps to `Absent`.
//!
//! # Where each field comes from
//!
//! Deliberately two sources, because they are two different facts:
//!
//! - **The commitment body** (`from_dna_hash`, `to_dna_hash`,
//!   `constitution_root`, `signatures`, `required_signatures`) is read out of
//!   the DHT entry's `payload_json` — the notarized bytes themselves.
//! - **The lifecycle** (`state`, `revoked_at`) is read off the
//!   `mishpat_commitments` projection row, which is where the
//!   `CommitmentByState` link and the revocation land
//!   (`db::mishpat_commitments::get_by_cid`). A commitment whose row has not
//!   projected yet is read as `proposed` — fail-closed, because
//!   [`super::verify::verify_path`] establishes a path only on `"active"`.

use std::sync::Arc;

use seam_contracts::Answer;

use super::{ArtifactClass, PathEvidence, ReleaseManifest};
use crate::db::DbPool;
use crate::hc_client::HcClient;

/// The lifecycle state a commitment is read as when its projection row has not
/// landed (or could not be read). Fail-closed: `verify_path` establishes a path
/// only on `"active"`, so an unknown lifecycle refuses rather than adopts.
const UNPROJECTED_STATE: &str = "proposed";

/// The signature count a path's discipline requires when the commitment body
/// does not declare one. One is the floor, never zero — a `required_signatures`
/// of zero would make the quorum check in `verify_path` vacuous, which is the
/// one value a defaulting rule must not be able to produce.
const DEFAULT_REQUIRED_SIGNATURES: usize = 1;

/// Fetch the evidence for a manifest's `adoptionDiscipline.path`.
///
/// Returns [`Answer::Absent`] without touching the conductor for any artifact
/// class but [`ArtifactClass::HappLineage`] — `verify_path` is a no-op for
/// those and never consults the value, so paying for a zome call would be
/// work with no reader.
pub async fn fetch_path_evidence(
    hc: Option<&Arc<HcClient>>,
    db: Option<&DbPool>,
    manifest: &ReleaseManifest,
) -> Answer<PathEvidence> {
    if manifest.artifact_class != ArtifactClass::HappLineage {
        return Answer::Absent;
    }
    // A `happ-lineage` manifest with no path is a schema violation, and
    // `verify_path` says so (`manifest_schema_invalid`) without needing
    // evidence. Absent is the honest input: there is no commitment named to
    // go and read.
    let Some(path) = manifest.adoption_discipline.path.as_ref() else {
        return Answer::Absent;
    };
    let cid = path.commitment_cid.as_str();

    // No bridge at all — we could not ask. NEVER Absent (C4).
    let Some(hc) = hc else {
        tracing::debug!(
            commitment_cid = %cid,
            "release-adoption: no conductor bridge to read path evidence through — unreachable, \
             which establishes nothing about the commitment"
        );
        return Answer::Unreachable;
    };

    let out = match crate::services::conductor_writes::get_commitment(hc, cid).await {
        Ok(Some(out)) => out,
        // The conductor ANSWERED, and answered "not on my DHT view". That is
        // an observed absence, and `verify_path` reports it as
        // `path_not_notarized` — a refusal that self-heals the moment the
        // commitment gossips to this peer.
        Ok(None) => {
            tracing::debug!(
                commitment_cid = %cid,
                "release-adoption: path commitment is not on this conductor's DHT view yet"
            );
            return Answer::Absent;
        }
        Err(e) => {
            tracing::debug!(
                commitment_cid = %cid,
                error = %e,
                "release-adoption: path commitment unreadable — unreachable, never absence"
            );
            return Answer::Unreachable;
        }
    };

    let payload: serde_json::Value = match serde_json::from_str(&out.payload_json) {
        Ok(v) => v,
        // We reached the commitment but cannot read what it says. That is a
        // failure to READ, not an absence and not a mismatch — treating it as
        // either would put a fabricated fact in front of the floor.
        Err(e) => {
            tracing::warn!(
                commitment_cid = %cid,
                error = %e,
                "release-adoption: path commitment payload_json does not parse — unreadable"
            );
            return Answer::Unreachable;
        }
    };

    // The projection carries the lifecycle; the DHT entry carries the body.
    let (state, revoked_at) = lifecycle(db, cid).await;

    Answer::Present(evidence_from(
        // The commitment's CID is its ENTRY hash, never its action hash —
        // returning the wrong one here would fail `verify_path`'s
        // commitment-identity check on every legitimate path.
        &format!("{}", out.entry_hash),
        &payload,
        state,
        revoked_at,
    ))
}

/// Build the evidence from a commitment body plus its projected lifecycle.
///
/// Pure, so the parsing rule is unit-testable against a payload fixture with
/// no conductor and no pool. A field the body omits becomes an empty string
/// rather than an error: `verify_path` then reports the precise crossing
/// mismatch (`path X names →, release is A→B`), which tells an operator far
/// more than "the payload was malformed" would.
pub fn evidence_from(
    commitment_cid: &str,
    payload: &serde_json::Value,
    state: String,
    revoked_at: Option<String>,
) -> PathEvidence {
    PathEvidence {
        commitment_cid: commitment_cid.to_string(),
        state,
        revoked_at,
        from_dna_hash: string_field(payload, "from_dna_hash"),
        to_dna_hash: string_field(payload, "to_dna_hash"),
        constitution_root: string_field(payload, "constitution_root"),
        // The COUNT of signatures the commitment carries — read as the length
        // of the array, so a body that lists three signers cannot claim four.
        signatures: payload
            .get("signatures")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0),
        required_signatures: payload
            .get("required_signatures")
            .or_else(|| payload.get("requiredSignatures"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_REQUIRED_SIGNATURES),
    }
}

/// A string field from the commitment body, accepting either the snake_case
/// spelling the mishpat payloads use or the camelCase one the manifest schema
/// uses. Absent → empty string (see [`evidence_from`]).
fn string_field(payload: &serde_json::Value, snake: &str) -> String {
    let camel = to_lower_camel(snake);
    payload
        .get(snake)
        .or_else(|| payload.get(camel.as_str()))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// `from_dna_hash` → `fromDnaHash`. Small enough to keep local; the alternative
/// is a dependency on a case crate for four field names.
fn to_lower_camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper_next = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// The commitment's projected lifecycle: `(state, revoked_at)`.
///
/// A missing pool, a pool checkout failure, a query failure, or a row that has
/// not projected yet all resolve to [`UNPROJECTED_STATE`] — fail-closed in
/// every direction, because the only state that establishes a path is
/// `"active"` and none of those four situations is evidence of one.
async fn lifecycle(db: Option<&DbPool>, cid: &str) -> (String, Option<String>) {
    let unprojected = || (UNPROJECTED_STATE.to_string(), None);
    let Some(pool) = db.cloned() else {
        return unprojected();
    };
    let cid = cid.to_string();
    // The pool checkout + diesel query are blocking; offload exactly as
    // `ProjectionCommitmentFetcher::fetch` does rather than stalling a runtime
    // worker on a sqlite read.
    let joined = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        crate::db::mishpat_commitments::get_by_cid(&mut conn, &cid).ok()?
    })
    .await;
    match joined {
        Ok(Some(row)) => (row.state, row.revoked_at),
        Ok(None) => unprojected(),
        Err(e) => {
            tracing::warn!(error = %e, "release-adoption: path lifecycle read panicked/aborted");
            unprojected()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> serde_json::Value {
        serde_json::json!({
            "from_dna_hash": "uhC0kINSTALLED",
            "to_dna_hash": "uhC0kV2NODEREG",
            "constitution_root": "bafyLineageConstitutionRoot",
            "signatures": ["uhCAkSignerOne", "uhCAkSignerTwo", "uhCAkSignerThree"],
            "required_signatures": 3,
        })
    }

    /// The body's fields land where `verify_path` reads them, and `signatures`
    /// is the ARRAY LENGTH — a body listing three signers must never be able
    /// to report a different number.
    #[test]
    fn a_payload_fixture_parses_into_the_evidence_verify_path_reads() {
        let ev = evidence_from("uhCEkPathCommitment", &body(), "active".to_string(), None);
        assert_eq!(ev.commitment_cid, "uhCEkPathCommitment");
        assert_eq!(ev.from_dna_hash, "uhC0kINSTALLED");
        assert_eq!(ev.to_dna_hash, "uhC0kV2NODEREG");
        assert_eq!(ev.constitution_root, "bafyLineageConstitutionRoot");
        assert_eq!(ev.signatures, 3);
        assert_eq!(ev.required_signatures, 3);
        assert_eq!(ev.state, "active");
        assert!(ev.revoked_at.is_none());
    }

    /// The projection row's lifecycle — not the body's — decides `state` and
    /// `revoked_at`. A revoked commitment whose BODY still reads active must
    /// come through as revoked, because revocation is a lifecycle fact that
    /// lands after the entry was written and can never be inside it.
    #[test]
    fn the_projection_row_supplies_the_lifecycle_not_the_body() {
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            "active".to_string(),
            Some("2026-09-04T10:00:00Z".to_string()),
        );
        assert_eq!(ev.revoked_at.as_deref(), Some("2026-09-04T10:00:00Z"));
        // And the fail-closed default a missing row produces.
        let unprojected = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            UNPROJECTED_STATE.to_string(),
            None,
        );
        assert_eq!(unprojected.state, "proposed");
        assert_ne!(
            unprojected.state, "active",
            "an unprojected row must never read as the one state that establishes a path"
        );
    }

    /// A body missing every field yields empty crossings and the quorum FLOOR
    /// — never a vacuous `0 of 0` that would pass the quorum check.
    #[test]
    fn an_empty_body_defaults_to_the_quorum_floor_never_a_vacuous_pass() {
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &serde_json::json!({}),
            "active".to_string(),
            None,
        );
        assert_eq!(ev.signatures, 0);
        assert_eq!(ev.required_signatures, DEFAULT_REQUIRED_SIGNATURES);
        assert!(ev.required_signatures > 0);
        assert!(ev.signatures < ev.required_signatures, "must fail quorum");
        assert!(ev.from_dna_hash.is_empty());
    }

    /// camelCase bodies read identically — the manifest schema spells these
    /// fields one way and the mishpat payloads the other, and a path must not
    /// depend on which side authored the commitment.
    #[test]
    fn camel_case_bodies_read_the_same_as_snake_case_ones() {
        let camel = serde_json::json!({
            "fromDnaHash": "uhC0kINSTALLED",
            "toDnaHash": "uhC0kV2NODEREG",
            "constitutionRoot": "bafyRoot",
            "signatures": ["a"],
            "requiredSignatures": 1,
        });
        let ev = evidence_from("uhCEkX", &camel, "active".to_string(), None);
        assert_eq!(ev.from_dna_hash, "uhC0kINSTALLED");
        assert_eq!(ev.to_dna_hash, "uhC0kV2NODEREG");
        assert_eq!(ev.constitution_root, "bafyRoot");
        assert_eq!(ev.signatures, 1);
        assert_eq!(ev.required_signatures, 1);
        assert_eq!(to_lower_camel("from_dna_hash"), "fromDnaHash");
    }

    /// Every class but `happ-lineage` is Absent WITHOUT a conductor round-trip
    /// — asserted by passing no bridge at all: a class that consulted the
    /// conductor would answer `Unreachable` here instead.
    #[tokio::test]
    async fn a_non_lineage_class_never_pays_for_a_conductor_call() {
        for class in [
            ArtifactClass::CoordinatorBundle,
            ArtifactClass::ConfigEpr,
            ArtifactClass::StorageBinary,
            ArtifactClass::HappBundle,
        ] {
            let mut m = super::super::test_support::lineage_manifest();
            m.artifact_class = class;
            let answer = fetch_path_evidence(None, None, &m).await;
            assert!(
                matches!(answer, Answer::Absent),
                "{} must be Absent without asking the conductor",
                class.label()
            );
        }
    }

    /// A `happ-lineage` release with no bridge to ask through is UNREACHABLE,
    /// never Absent — our outage is never a statement about the elohim's
    /// governance (C4).
    #[tokio::test]
    async fn no_bridge_is_unreachable_never_absent() {
        let m = super::super::test_support::lineage_manifest();
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Unreachable
        ));
    }

    /// …and a `happ-lineage` release that names NO path needs no evidence at
    /// all: `verify_path` refuses it on the schema, so there is nothing to go
    /// and read.
    #[tokio::test]
    async fn a_lineage_release_naming_no_path_is_absent_not_unreachable() {
        let mut m = super::super::test_support::lineage_manifest();
        m.adoption_discipline.path = None;
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Absent
        ));
    }
}
