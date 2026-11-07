//! Test utilities shared across integration tests.
//!
//! Exposed as a public module so `tests/*.rs` integration test binaries can
//! reference `elohim_storage::test_util::test_pool`.  Not intended for
//! production use.

use crate::db::{init_pool, run_migrations, DbPool};

/// Create an in-memory SQLite pool with all migrations applied.
///
/// Each call returns a fresh, isolated database — safe to use concurrently
/// across test threads because SQLite in-memory databases are not shared.
pub fn test_pool() -> DbPool {
    // Use a unique URI so concurrent tests don't share the same in-memory DB.
    // The `?mode=memory&cache=private` prevents connection sharing across pool
    // connections (each checkout is a fresh empty DB), so we use a named DB
    // instead and allow the pool to keep one connection alive.
    let url = format!(
        "file:testdb_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = init_pool(&url).expect("test pool init failed");
    run_migrations(&pool).expect("test migrations failed");
    pool
}

// ---------------------------------------------------------------------------
// P2P test harness — only compiled when the `p2p` feature is enabled
// ---------------------------------------------------------------------------

/// Harness returned by `spawn_p2p_with_peers`.
///
/// Holds a stub [`crate::p2p::P2PHandle`] (no real libp2p swarm — all push/fetch
/// calls return stub errors) and the pool that was seeded with the given peers.
///
/// **Limitation:** because the stub handle does not run a real libp2p swarm,
/// `push_shard` always returns `Err("stub: no P2P swarm in test")`.  This
/// means `distribute_shards` will always report `distributed == 0` and
/// `shard_locations` will never be populated.  Tests that need to verify
/// household diversity via `shard_locations` must use a live harness — see
/// Task 17 for live integration coverage.
///
/// Tests that only need to verify `placement_gaps` recording (gap written on
/// short placement; cleared on full placement) work correctly with this stub
/// because the gap logic runs before any push attempt.
#[cfg(feature = "p2p")]
pub struct P2PTestHarness {
    pub p2p: crate::p2p::P2PHandle,
    pub pool: DbPool,
}

/// Seed the DB with the given peers and return a stub P2P harness.
///
/// Each entry in `peers` is `(agent_key, household_id, lifecycle)` where
/// `lifecycle` is `"accepting"` or `"leaving"` (matches peer_statuses semantics
/// used in peer_selection tests).
///
/// For each peer an active REA commitment is inserted for `content:commons` so
/// peer-selection sees a provider, plus a notarized `replicates-commons`
/// commitment in `mishpat_commitments` (recipient == head_ref) so the commons
/// scorer arm has a row to load.
///
/// Returns a `P2PTestHarness` with:
/// - `pool`: the seeded database
/// - `p2p`:  a stub handle (see `P2PHandle::for_testing()`)
#[cfg(feature = "p2p")]
pub async fn spawn_p2p_with_peers(pool: DbPool, peers: &[(&str, &str, &str)]) -> P2PTestHarness {
    use crate::db::diesel_schema::{peer_statuses, rea_commitments};
    use crate::db::models::NewHuman;
    use diesel::prelude::*;
    use diesel::RunQueryDsl;

    let mut conn = pool.get().expect("test pool connection");

    // Live peers carry a FRESH heartbeat timestamp. The production selection path
    // (`distribute_shards`) applies a liveness staleness window
    // (`PEER_STATUS_STALENESS_SECS`, default 900s), so an "accepting" fixture must
    // look recently-heartbeated or it would be (correctly) treated as dark. `now`
    // is stamped once so every seeded row shares one instant.
    let now_micros = chrono::Utc::now().timestamp_micros();

    for (agent_key, household_id, lifecycle) in peers {
        // Insert human row
        let human_id = format!("human-{agent_key}");
        diesel::insert_or_ignore_into(crate::db::diesel_schema::humans::table)
            .values(&NewHuman {
                id: human_id.clone(),
                agent_pub_key: Some((*agent_key).to_string()),
                display_name: human_id.clone(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: crate::db::HUMANS_HAPP_ID.to_string(),
                household_id: Some((*household_id).to_string()),
            })
            .execute(&mut conn)
            .expect("insert human");

        // Insert peer status
        let (status, pool_member, reserves) = match *lifecycle {
            "accepting" => ("online", 1i32, 1i32),
            "leaving" => ("leaving", 0i32, 0i32),
            other => panic!("unexpected lifecycle in spawn_p2p_with_peers: {other}"),
        };
        diesel::insert_or_ignore_into(peer_statuses::table)
            .values((
                peer_statuses::peer_id.eq(*agent_key),
                peer_statuses::status.eq(status),
                peer_statuses::general_pool_member.eq(pool_member),
                peer_statuses::accepting_stewardship_reserves.eq(reserves),
                peer_statuses::timestamp.eq(now_micros),
                peer_statuses::dht_anchor_hash.eq("anchor-placeholder"),
                peer_statuses::updated_at.eq(now_micros),
            ))
            .execute(&mut conn)
            .expect("insert peer_status");

        // Insert REA commitment for content:commons
        let cmt_id = format!("cmt-{agent_key}-commons");
        diesel::insert_or_ignore_into(rea_commitments::table)
            .values((
                rea_commitments::id.eq(&cmt_id),
                rea_commitments::h_app_id.eq("lamad"),
                rea_commitments::action.eq("provide"),
                rea_commitments::provider.eq(*agent_key),
                rea_commitments::receiver.eq(""),
                rea_commitments::resource_classified_as.eq(Some("content:commons")),
                rea_commitments::state.eq("active"),
                rea_commitments::finished.eq(0),
                rea_commitments::created_at.eq("2026-04-19T00:00:00Z"),
            ))
            .execute(&mut conn)
            .expect("insert rea_commitment");

        // Insert a notarized replicates-commons commitment so the commons
        // scorer arm (active_commitments_for_provider → mishpat_commitments)
        // has a real row to load. recipient == head_ref (spec contract). The
        // legacy rea_commitments "provide" row above is left intact because
        // peer_selection/household_resilience still filter action == "provide".
        let commons_cid = format!("commons-{agent_key}-head");
        diesel::insert_or_ignore_into(crate::db::diesel_schema::mishpat_commitments::table)
            .values((
                crate::db::diesel_schema::mishpat_commitments::cid.eq(&commons_cid),
                crate::db::diesel_schema::mishpat_commitments::action.eq("replicates-commons"),
                crate::db::diesel_schema::mishpat_commitments::scope.eq("replicates-commons"),
                crate::db::diesel_schema::mishpat_commitments::provider.eq(*agent_key),
                crate::db::diesel_schema::mishpat_commitments::recipient.eq("head:commons-fixture"),
                crate::db::diesel_schema::mishpat_commitments::bounds_json
                    .eq(r#"{"rate_per_minute":60,"reach_ceiling":"commons"}"#),
                crate::db::diesel_schema::mishpat_commitments::valid_from
                    .eq("2026-01-01T00:00:00Z"),
                crate::db::diesel_schema::mishpat_commitments::valid_until
                    .eq("2027-01-01T00:00:00Z"),
                crate::db::diesel_schema::mishpat_commitments::revoked_at.eq(None::<String>),
                crate::db::diesel_schema::mishpat_commitments::state.eq("active"),
                crate::db::diesel_schema::mishpat_commitments::dht_anchor_hash
                    .eq(Some(format!("{commons_cid}-anchor"))),
                crate::db::diesel_schema::mishpat_commitments::created_at
                    .eq("2026-01-01T00:00:00Z"),
                crate::db::diesel_schema::mishpat_commitments::updated_at
                    .eq("2026-01-01T00:00:00Z"),
            ))
            .execute(&mut conn)
            .expect("insert replicates-commons commitment");
    }

    let p2p = crate::p2p::P2PHandle::for_testing();
    P2PTestHarness { p2p, pool }
}

// ---------------------------------------------------------------------------
// Cross-signed identity fixtures — the ONLY authority admission reads
// (`p2p::transport_authority`). A test that binds an author to an agent goes
// through these, never through `peer_transport_manifest`.
// ---------------------------------------------------------------------------

/// The agent signing key for a readable test name (seeded from the name's BLAKE3).
#[cfg(feature = "p2p")]
pub fn test_agent_key(name: &str) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(blake3::hash(name.as_bytes()).as_bytes())
}

/// A real HoloHash `agent_cid` (`uhCAk…`) for a readable test name: the agent key a
/// cross-signed binding can actually name.
#[cfg(feature = "p2p")]
pub fn test_agent(name: &str) -> String {
    crate::p2p::binding_proof_wire::agent_cid_from_agent_pubkey(
        &test_agent_key(name).verifying_key().to_bytes(),
    )
}

/// Insert an active, cross-signed `AgentPeerBinding` row binding [`test_agent`]`(name)`
/// to the libp2p PeerId of `transport_secret` — both halves really signed and
/// classified by the production chokepoint. Returns the PeerId.
#[cfg(feature = "p2p")]
pub fn bind_cross_signed(
    conn: &mut diesel::SqliteConnection,
    name: &str,
    transport_secret: [u8; 32],
) -> String {
    use crate::p2p::binding_cross_signature::{
        canonical_bytes, BindingCore, CrossSignatureProof, AGENT_DOMAIN, SCHEME_VERSION,
        TRANSPORT_DOMAIN, TRANSPORT_KIND_LIBP2P,
    };
    use crate::p2p::binding_proof_wire::{
        classify_binding_signature, encode_proof, libp2p_peer_id_from_ed25519_pubkey,
    };
    use ed25519_dalek::Signer;

    let agent_sk = test_agent_key(name);
    let transport_sk = ed25519_dalek::SigningKey::from_bytes(&transport_secret);
    let agent = test_agent(name);
    let peer_id = libp2p_peer_id_from_ed25519_pubkey(&transport_sk.verifying_key().to_bytes())
        .expect("ed25519 PeerId");
    let now = chrono::Utc::now();
    let fmt = |t: chrono::DateTime<chrono::Utc>| t.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let (from, until) = (
        fmt(now - chrono::Duration::days(1)),
        fmt(now + chrono::Duration::days(29)),
    );
    let core = BindingCore {
        agent_cid: agent.clone(),
        transport_id: peer_id.clone(),
        transport_kind: TRANSPORT_KIND_LIBP2P,
        valid_from: from.clone(),
        valid_until: Some(until.clone()),
        nonce: "dGVzdC1iaW5kaW5n".into(),
        issued_at: from.clone(),
    };
    let signature = encode_proof(&CrossSignatureProof {
        scheme_version: SCHEME_VERSION,
        transport_kind: TRANSPORT_KIND_LIBP2P,
        transport_pubkey: transport_sk.verifying_key().to_bytes(),
        transport_signature: transport_sk
            .sign(&canonical_bytes(TRANSPORT_DOMAIN, &core))
            .to_bytes(),
        agent_pubkey: agent_sk.verifying_key().to_bytes(),
        agent_signature: agent_sk
            .sign(&canonical_bytes(AGENT_DOMAIN, &core))
            .to_bytes(),
        nonce: core.nonce.clone(),
        issued_at: from.clone(),
    });
    let proof_status = classify_binding_signature(
        &agent,
        &peer_id,
        TRANSPORT_KIND_LIBP2P,
        &from,
        Some(&until),
        &signature,
    );
    assert!(
        proof_status.is_cross_signed(),
        "fixture must classify cross_signed"
    );
    crate::db::peer_identity_bindings::upsert(
        conn,
        &crate::db::models::NewPeerIdentityBindingRow {
            peer_id: peer_id.clone(),
            agent_cid: agent,
            dht_anchor_hash: format!("anchor-{peer_id}"),
            valid_from: from.clone(),
            valid_until: Some(until),
            observed_at: from,
            source: "dht".into(),
            device_archetype: "node".into(),
            superseded_by: None,
            signature,
            proof_status,
        },
    )
    .expect("insert cross-signed binding");
    peer_id
}

/// Record a verified two-key transport link between the libp2p key `libp2p_secret`
/// and the iroh key `iroh_secret`. Returns the iroh NodeId.
#[cfg(feature = "p2p")]
pub fn link_transports(
    conn: &mut diesel::SqliteConnection,
    libp2p_secret: [u8; 32],
    iroh_secret: [u8; 32],
) -> String {
    let link = crate::p2p::transport_link::TransportLink::sign(
        &elohim_epr::proof::AgentKeypair::from_secret(&libp2p_secret).expect("libp2p key"),
        &elohim_epr::proof::AgentKeypair::from_secret(&iroh_secret).expect("iroh key"),
        chrono::Utc::now().timestamp_millis(),
    )
    .expect("link");
    link.verify(chrono::Utc::now().timestamp_millis())
        .expect("fixture link verifies");
    crate::db::transport_links::record_verified(conn, &link).expect("record link");
    link.iroh_node_id
}
