//! Integration tests: EPR reach value persistence and enforcement.
//!
//! ## What is tested here
//!
//! `EprStore::put` must persist the `reach` field verbatim so that downstream
//! consumers (route handlers, Angular clients) can enforce access-control
//! decisions based on the stored value.  This test layer asserts service-level
//! behaviour — that reach comes out of storage exactly as it went in.
//!
//! ## What is NOT tested here (deferred)
//!
//! Route-level auth gating (HTTP 404 for restricted reach without authentication)
//! is intentionally deferred.  The full hyper server must be bootable in the
//! test harness for that, which requires a live port, middleware stack, and JWT
//! fixtures.  The right home for that test is an HTTP integration test that
//! spins up the axum/hyper app via `axum::Server` + `reqwest` — expected in
//! Phase 2b when the auth middleware lands.
//!
//! The split is deliberate:
//!   - Service layer  → verified here (no network, fast, in-memory DB)
//!   - Route/auth layer → deferred to HTTP integration tests in Phase 2b
//!
//! All tests are marked `#[ignore = "requires test database"]`.
//! Jenkins provisions a test database; local environments may not have one.
//! To run locally: `cargo test --test epr_reach_enforcement -- --ignored`

use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::services::epr_store::{default_epr_store, EprStore};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn cid_from_byte(b: u8) -> Cid {
    compute_cid(&[b])
}

/// Build a signed EPR with the given reach value and distinguishing payload.
fn build_epr_with_reach(reach: Reach, payload: &[u8]) -> Epr {
    let kp = AgentKeypair::from_secret(&[55u8; 32]).unwrap();
    let agent_cid = cid_from_byte(200);

    Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid_from_byte(10))
        .schema_key("concept")
        .reach(reach)
        .coupling(Coupling {
            knowledge: Some(cid_from_byte(20)),
            value: Some(cid_from_byte(21)),
            governance: Some(cid_from_byte(22)),
        })
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 0, 0, 0).unwrap())
        .payload(payload.to_vec())
        .sign(&kp, agent_cid)
        .expect("sign EPR")
}

// ---------------------------------------------------------------------------
// Tests — one per supported Reach variant
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires test database"]
fn reach_commons_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Commons, b"commons reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "commons");
}

#[test]
#[ignore = "requires test database"]
fn reach_public_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Public, b"public reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "public");
}

#[test]
#[ignore = "requires test database"]
fn reach_community_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Community, b"community reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "community");
}

#[test]
#[ignore = "requires test database"]
fn reach_private_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Private, b"private reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "private");
}

#[test]
#[ignore = "requires test database"]
fn reach_trusted_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Trusted, b"trusted reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "trusted");
}

#[test]
#[ignore = "requires test database"]
fn reach_familiar_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Familiar, b"familiar reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "familiar");
}

#[test]
#[ignore = "requires test database"]
fn reach_intimate_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::Intimate, b"intimate reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "intimate");
}

#[test]
#[ignore = "requires test database"]
fn reach_self_scope_is_persisted_verbatim() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let store = default_epr_store(None, None, None);

    let epr = build_epr_with_reach(Reach::SelfScope, b"self-scope reach payload");
    let cid = epr.envelope.cid.to_string();
    store.put(&mut conn, epr).expect("put");

    let fetched = store
        .fetch(&mut conn, &cid)
        .expect("fetch")
        .expect("atom present");
    assert_eq!(fetched.fetched.atom.reach, "self");
}
