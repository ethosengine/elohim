//! Integration tests: EPR ingest → fetch → verify lifecycle.
//!
//! These tests exercise the full EprStore trait via `default_epr_store()`,
//! covering the LocalEprStore → FederatedEprStore (Phase 2a) code path.
//!
//! All tests are marked `#[ignore = "requires test database"]`.
//! Jenkins provisions a test database; local environments may not have one.
//! To run locally: `cargo test --test epr_ingest_integration -- --ignored`

use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::services::epr_store::{default_epr_store, EprSource, EprStore};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Minimal CID derived from a single-byte seed — deterministic across tests.
fn cid_from_byte(b: u8) -> Cid {
    compute_cid(&[b])
}

/// Build a fully-signed `EprKind::Content` EPR for test use.
///
/// Uses a fixed keypair seed so the CID is stable across repeated calls with
/// the same `payload`.  Pass different payloads to produce distinct CIDs.
fn build_content_epr(payload: &[u8]) -> (Epr, AgentKeypair) {
    let kp = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let agent_cid = cid_from_byte(100);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid_from_byte(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(cid_from_byte(2)),
            value: Some(cid_from_byte(3)),
            governance: Some(cid_from_byte(4)),
        })
        .claim(cid_from_byte(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(payload.to_vec())
        .sign(&kp, agent_cid)
        .expect("sign EPR");

    (epr, kp)
}

// ---------------------------------------------------------------------------
// Test 1: ingest → fetch → verify roundtrip
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires test database"]
fn ingest_fetch_verify_roundtrip() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store();

    // --- Ingest ---
    let (epr, kp) = build_content_epr(b"roundtrip payload");
    let declared_cid = epr.envelope.cid.to_string();

    let ingest_result = store.put(&mut conn, epr).expect("put should succeed");
    assert_eq!(
        ingest_result.cid, declared_cid,
        "ingest result CID must match declared envelope CID"
    );

    // --- Fetch ---
    let fetch_outcome = store
        .fetch(&mut conn, &declared_cid)
        .expect("fetch should succeed")
        .expect("atom should be present after ingest");

    assert_eq!(
        fetch_outcome.source,
        EprSource::Local,
        "Phase 2a: source must be Local"
    );
    assert_eq!(fetch_outcome.fetched.atom.cid, declared_cid);

    // --- Verify ---
    let pub_key = kp.public_key_bytes();
    let report = store
        .verify(&mut conn, &declared_cid, &pub_key)
        .expect("verify should succeed");

    assert!(
        report.verified,
        "signature must verify under the signing key"
    );
    assert!(
        report.stages_run.contains(&"canonicalization".to_string()),
        "stages_run must include canonicalization"
    );
    assert!(
        report.stages_run.contains(&"signature".to_string()),
        "stages_run must include signature"
    );
    assert!(
        report.stages_run.contains(&"coupling".to_string()),
        "stages_run must include coupling"
    );
    assert!(
        report.stages_skipped.contains(&"payloadSchema".to_string()),
        "stages_skipped must include payloadSchema (deferred to Phase 3)"
    );
    assert!(report.error.is_none(), "no error expected on valid EPR");
}

// ---------------------------------------------------------------------------
// Test 2: verify with wrong key → rejected
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires test database"]
fn verify_with_wrong_key_is_rejected() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store();

    let (epr, _correct_kp) = build_content_epr(b"wrong key payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put should succeed");

    // Generate a different keypair — this one did NOT sign the EPR
    let wrong_kp = AgentKeypair::from_secret(&[99u8; 32]).unwrap();
    let report = store
        .verify(&mut conn, &cid, &wrong_kp.public_key_bytes())
        .expect("verify call should succeed (returns report, not error)");

    assert!(
        !report.verified,
        "verification must fail under the wrong key"
    );
    let error = report
        .error
        .expect("error detail must be present on failure");
    assert_eq!(
        error.stage, "signature",
        "failure must be attributed to the signature stage"
    );
}

// ---------------------------------------------------------------------------
// Test 3: idempotent put — same canonical bytes → same CID, no error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires test database"]
fn put_same_canonical_bytes_is_idempotent() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store();

    let (epr_a, _kp) = build_content_epr(b"idempotent payload");
    // Build a second EPR from the same deterministic seed — must produce identical bytes
    let (epr_b, _kp2) = build_content_epr(b"idempotent payload");

    assert_eq!(
        epr_a.envelope.cid, epr_b.envelope.cid,
        "deterministic builder must produce the same CID for the same inputs"
    );

    let first = store
        .put(&mut conn, epr_a)
        .expect("first put should succeed");
    let second = store
        .put(&mut conn, epr_b)
        .expect("second put (same canonical bytes) should be idempotent — not an error");

    assert_eq!(first.cid, second.cid, "both puts must return the same CID");
}

// ---------------------------------------------------------------------------
// Test 4: collision detection — same CID, different canonical bytes → error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires test database"]
fn put_different_bytes_same_cid_is_rejected() {
    // This test documents the collision-detection path in LocalEprStore::put.
    // It is difficult to produce a genuine CID collision (that would require
    // breaking SHA-256), so we simulate the scenario by ingesting a valid EPR
    // and then constructing a tampered Epr with the CID swapped to the first
    // EPR's CID but different payload/canonical bytes.
    //
    // The LocalEprStore::put implementation computes canonical_bytes for the
    // inbound EPR and compares them to the stored canonical_bytes when the CID
    // already exists.  If they differ, it returns InvalidInput — preventing a
    // bad actor from overwriting the atom.

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store();

    // Ingest the legitimate EPR
    let (epr_a, _kp) = build_content_epr(b"original payload collision test");
    let stored_cid = epr_a.envelope.cid.to_string();
    store
        .put(&mut conn, epr_a)
        .expect("original put should succeed");

    // Build a *different* EPR with different payload — it will have a different CID
    let (mut epr_b, _kp2) = build_content_epr(b"different payload collision test");

    // Swap the CID on epr_b to match epr_a's — simulating a collision attempt.
    // The canonical_bytes will NOT match the swapped CID, triggering the guard.
    epr_b.envelope.cid = stored_cid.parse().expect("valid CID string");

    let result = store.put(&mut conn, epr_b);
    match result {
        Err(elohim_storage::error::StorageError::InvalidInput(msg)) => {
            assert!(
                msg.contains("different canonical bytes"),
                "error message should mention canonical bytes mismatch, got: {msg}"
            );
        }
        other => panic!("expected InvalidInput for collision attempt, got: {other:?}"),
    }
}
