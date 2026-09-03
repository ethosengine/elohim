//! Station 3b (M9) Task 2 phase B — receiver-side pre-authorization.
//!
//! Integration coverage for the exact production decision predicate
//! `elohim_storage::p2p::preauthorize_private_record`, which
//! `p2p::store_acquired_record` calls before ever building a
//! `CreateContentInput` for an ACQUIRED record — the receive-side twin of
//! the serve-side custody gate (`private_reach::private_serve_verdict`).
//!
//! `store_acquired_record` itself stays `pub(crate)` (it needs a running
//! libp2p swarm / iroh endpoint to reach from the outside — see the phase A
//! report), so this test drives the PRODUCTION resolver
//! (`ProjectionCustodyStanding`, real `peer_identity_bindings` +
//! `rea_commitments` rows, a real SQLite pool) directly against the public
//! decision seam `store_acquired_record` calls, exactly as phase A's report
//! specified the seam needed to be exposed.
//!
//! `preauthorize_private_record` resolves "who am I" itself through
//! `CustodyStanding::resolve_agent(&Requester::local())` (see
//! `p2p::private_receive`'s module docs) rather than taking it as an
//! argument, so this peer's own agent is made resolvable the SAME way
//! `custody_standing.rs`'s own tests do it: a `local_sessions` active-session
//! row naming `agent_pub_key = THIS_PEER` (no conductor bridge wired in this
//! test, so `self_agent()` falls back to that projection).

use std::sync::Arc;

use diesel::prelude::*;
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::{run_migrations, DbPool};
use elohim_storage::p2p::binding_proof_wire::BindingProofStatus;
use elohim_storage::p2p::identity_map::HolochainBackedPeerIdentityMap;
use elohim_storage::p2p::shard_protocol::ContentRecord;
use elohim_storage::p2p::{preauthorize_private_record, PrivateReceiveVerdict};
use elohim_storage::private_reach::WithholdReason;
use elohim_storage::services::custody_standing::{ProjectionCustodyStanding, Requester};
use elohim_storage::services::rea_commitment_service::{
    deterministic_spool_custody_id, spool_classification,
};
use elohim_storage::services::spool_custody_author::SPOOL_CUSTODY_ACTION;

/// This receiving peer's own agent — the custodian side of every fixture.
/// Made resolvable via `Requester::local()` through a seeded active
/// `local_sessions` row (see module docs).
const THIS_PEER: &str = "uhCAkThisPeerAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
/// The sender/ward agent — the record travelled here FROM this agent.
const SENDER_AGENT: &str = "uhCAkSenderWardAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// A REAL, parseable libp2p `PeerId` — `ProjectionCustodyStanding::requester_agent`
/// calls `peer_b58.parse::<libp2p::PeerId>()` before it ever consults
/// `peer_identity_bindings`, so a hand-typed placeholder string that isn't
/// valid base58-multihash would resolve to `None` regardless of what's
/// seeded, silently turning every "resolved sender" fixture below into an
/// "unresolved sender" one.
fn sender_peer_id() -> String {
    libp2p::PeerId::random().to_base58()
}

fn test_pool() -> DbPool {
    use diesel::r2d2::{ConnectionManager, Pool};
    let url = format!(
        "file:private_preauth_receive_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    run_migrations(&pool).expect("migrations");
    pool
}

/// Seed a real `peer_identity_bindings` row — the production join
/// `HolochainBackedPeerIdentityMap::lookup` reads on the hot path, exactly
/// as a real handshake-observed binding would.
fn seed_peer_binding(pool: &DbPool, peer_id: &str, agent_cid: &str) {
    let mut conn = pool.get().expect("conn");
    let row = NewPeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: format!("anchor-{peer_id}"),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: "2026-09-03T00:00:00Z".to_string(),
        source: "handshake".to_string(),
        device_archetype: "node".to_string(),
        superseded_by: None,
        signature: String::new(),
        proof_status: BindingProofStatus::unverified(),
    };
    elohim_storage::db::peer_identity_bindings::upsert(&mut conn, &row).expect("seed binding");
}

/// Seed a LIVE `custody-spool` commitment: `provider` = the custodian
/// (`THIS_PEER`), `receiver` = the ward (`SENDER_AGENT`) — exactly the shape
/// phase A's report specified, using the SAME id/classification helpers
/// Task 1's own resolver and tests use.
fn seed_live_spool_custody(pool: &DbPool, provider: &str, ward: &str) {
    use elohim_storage::db::diesel_schema::rea_commitments::dsl as rc;
    let mut conn = pool.get().expect("conn");
    let id = deterministic_spool_custody_id(provider, ward, ward);
    diesel::insert_into(rc::rea_commitments)
        .values((
            rc::id.eq(&id),
            rc::h_app_id.eq("lamad"),
            rc::action.eq(SPOOL_CUSTODY_ACTION),
            rc::provider.eq(provider),
            rc::receiver.eq(ward),
            rc::resource_classified_as
                .eq(serde_json::json!([spool_classification(ward)]).to_string()),
            rc::state.eq("active"),
            rc::finished.eq(0),
            rc::created_at.eq("2026-09-03T00:00:00Z"),
        ))
        .execute(&mut conn)
        .expect("seed live custody-spool commitment");
}

/// Seed an active `local_sessions` row naming `THIS_PEER` — the fallback
/// `ProjectionCustodyStanding::self_agent` reads when no conductor bridge is
/// wired (no `.with_conductor()` in this test), so `Requester::local()`
/// resolves to `THIS_PEER` exactly as production resolves its own agent when
/// a conductor bridge is available.
fn seed_self_session(pool: &DbPool, agent_cid: &str) {
    use elohim_storage::db::diesel_schema::local_sessions::dsl as ls;
    let mut conn = pool.get().expect("conn");
    diesel::insert_into(ls::local_sessions)
        .values((
            ls::id.eq("session-this-peer"),
            ls::human_id.eq("human-this-peer"),
            ls::agent_pub_key.eq(agent_cid),
            ls::doorway_url.eq("http://localhost"),
            ls::identifier.eq("this-peer"),
            ls::is_active.eq(1),
            ls::created_at.eq("2026-09-03T00:00:00Z"),
            ls::updated_at.eq("2026-09-03T00:00:00Z"),
        ))
        .execute(&mut conn)
        .expect("seed self session");
}

fn resolver(pool: &DbPool) -> ProjectionCustodyStanding {
    let identity_map: Arc<dyn elohim_storage::p2p::identity_map::PeerIdentityMap> =
        Arc::new(HolochainBackedPeerIdentityMap::new(pool.clone()));
    ProjectionCustodyStanding::new(pool.clone()).with_libp2p_identity_map(identity_map)
}

fn record(reach: &str) -> ContentRecord {
    ContentRecord {
        id: "bafkreiacquiredwitness".to_string(),
        title: "acquired".to_string(),
        description: None,
        content_type: "issue-report".to_string(),
        content_format: "json".to_string(),
        blob_hash: None,
        blob_cid: None,
        content_size_bytes: None,
        metadata_json: None,
        reach: reach.to_string(),
        // The wire's own claim — deliberately NOT the real sender/ward, so a
        // fixture that passed by trusting this field would fail the assertion.
        created_by: Some("spoofed-created-by-claim".to_string()),
        tags: vec![],
        content_body: None,
    }
}

/// A private record whose SENDER never resolved to any agent (a real,
/// parseable libp2p `PeerId` with no `peer_identity_bindings` row) is
/// refused — this peer must never keep a private copy from a stranger it
/// cannot even name.
#[tokio::test]
async fn private_record_from_unresolved_sender_is_not_persisted() {
    let pool = test_pool();
    seed_self_session(&pool, THIS_PEER);
    // The SENDER never registered a binding at all.
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer_id());
    let verdict = preauthorize_private_record(Some(&standing), &sender, &record("private")).await;

    assert_eq!(
        verdict,
        PrivateReceiveVerdict::Skip(WithholdReason::UnresolvedRequester)
    );
}

/// A private record from a sender this peer holds a LIVE `custody-spool` for
/// (provider = this peer's agent, receiver = the sender's resolved agent) is
/// kept — real `peer_identity_bindings` + `rea_commitments` rows, the exact
/// production predicate `store_acquired_record` calls.
#[tokio::test]
async fn private_record_from_sender_with_live_spool_custody_is_persisted() {
    let pool = test_pool();
    seed_self_session(&pool, THIS_PEER);
    let sender_peer = sender_peer_id();
    seed_peer_binding(&pool, &sender_peer, SENDER_AGENT);
    seed_live_spool_custody(&pool, THIS_PEER, SENDER_AGENT);
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer);
    let rec = record("private");
    // The wire's `created_by` names neither this peer nor the ward — proves
    // the grant came from the sender's resolved identity, not the payload.
    assert_ne!(rec.created_by.as_deref(), Some(SENDER_AGENT));
    assert_ne!(rec.created_by.as_deref(), Some(THIS_PEER));

    let verdict = preauthorize_private_record(Some(&standing), &sender, &rec).await;

    assert_eq!(verdict, PrivateReceiveVerdict::Keep);
}

/// A `public` record is kept regardless of sender binding — this station
/// changes behaviour for `private` alone.
#[tokio::test]
async fn public_record_is_persisted_without_sender_binding() {
    let pool = test_pool();
    // No bindings and no self session at all — proves the short circuit
    // never touches resolution for a non-`private` reach.
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer_id());
    let verdict = preauthorize_private_record(Some(&standing), &sender, &record("public")).await;

    assert_eq!(verdict, PrivateReceiveVerdict::Keep);
}

/// A private record from a sender who DOES resolve, but for whom this peer
/// holds no custody standing at all, is refused (`NoStanding`) — the
/// resolved-stranger case.
#[tokio::test]
async fn private_record_from_a_resolved_stranger_is_not_persisted() {
    let pool = test_pool();
    seed_self_session(&pool, THIS_PEER);
    let sender_peer = sender_peer_id();
    seed_peer_binding(&pool, &sender_peer, SENDER_AGENT);
    // Deliberately NO custody-spool / custody-blob commitment seeded.
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer);
    let verdict = preauthorize_private_record(Some(&standing), &sender, &record("private")).await;

    assert_eq!(
        verdict,
        PrivateReceiveVerdict::Skip(WithholdReason::NoStanding)
    );
}

/// With no custody-standing resolver wired at all (`None`), a `private`
/// record fails CLOSED — the same posture `ShardService` takes on the serve
/// side when its own resolver is missing.
#[tokio::test]
async fn no_resolver_wired_fails_closed_for_a_private_record() {
    let sender = Requester::libp2p(sender_peer_id());
    let verdict = preauthorize_private_record(None, &sender, &record("private")).await;

    assert_eq!(
        verdict,
        PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable)
    );
}

/// The sender IS this peer's own agent (e.g. a caching peer handed the row
/// back) — always kept, no custody commitment needed.
#[tokio::test]
async fn a_record_from_this_peers_own_agent_is_always_kept() {
    let pool = test_pool();
    seed_self_session(&pool, THIS_PEER);
    let sender_peer = sender_peer_id();
    seed_peer_binding(&pool, &sender_peer, THIS_PEER);
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer);
    let verdict = preauthorize_private_record(Some(&standing), &sender, &record("private")).await;

    assert_eq!(verdict, PrivateReceiveVerdict::Keep);
}

/// With no self-session seeded at all, this peer cannot name itself — the
/// decision must fail closed rather than silently treat an empty/absent
/// agent as a match or a non-match.
#[tokio::test]
async fn this_peers_own_agent_unresolved_fails_closed() {
    let pool = test_pool();
    // No local_sessions row at all — self_agent() resolves to None.
    let sender_peer = sender_peer_id();
    seed_peer_binding(&pool, &sender_peer, SENDER_AGENT);
    seed_live_spool_custody(&pool, THIS_PEER, SENDER_AGENT);
    let standing = resolver(&pool);

    let sender = Requester::libp2p(sender_peer);
    let verdict = preauthorize_private_record(Some(&standing), &sender, &record("private")).await;

    assert_eq!(
        verdict,
        PrivateReceiveVerdict::Skip(WithholdReason::AuthorityUnavailable)
    );
}
