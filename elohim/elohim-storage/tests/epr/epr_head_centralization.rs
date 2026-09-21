//! Integration tests: `derive_epr_head` centralisation — B.7.
//!
//! Verifies that the single canonical helper produces the same EPR Head shape
//! that the two legacy inline construction sites produced, and that the
//! wire-round-trip is stable (encode → decode → structural equality).
//!
//! ## What is tested here
//!
//! 1. `derive_epr_head` returns `None` for unknown content IDs.
//! 2. With `gate_provenance=true`, unpublished rows (no `dht_anchor_hash`,
//!    no `p2p_published_at`) are invisible — matching the HTTP handler's
//!    original behaviour.
//! 3. With `gate_provenance=false`, those same rows are visible — matching
//!    the P2P handler's original behaviour.
//! 4. With `enrich_pillars=false`, shefa/qahal are empty — matching the HTTP
//!    handler's original behaviour.
//! 5. With `enrich_pillars=true`, shefa.stewards and qahal.attestation_requirements
//!    are populated from the DB — matching the P2P handler's original behaviour.
//! 6. The resulting `EprHead` survives a DAG-CBOR encode → decode round-trip
//!    without structural change, confirming wire stability.

use diesel::{prelude::*, RunQueryDsl};
use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::context::AppContext;
use elohim_storage::epr_codec::{decode_epr_head, encode_epr_head};
use elohim_storage::epr_head::{compose_head_view, derive_epr_head};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Seed a content row and return the connection.  By default the row has no
/// provenance markers (`dht_anchor_hash = NULL`, `p2p_published_at = NULL`).
fn seed_content(conn: &mut SqliteConnection, id: &str) {
    let ctx = AppContext::default_lamad();
    create_content(
        conn,
        &ctx,
        CreateContentInput {
            id: id.to_string(),
            title: "Test Content".to_string(),
            description: Some("A test concept for derive_epr_head".to_string()),
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: Some("bafkreitest000000000000000000000000000000000000000000001".to_string()),
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: Some("did:key:z6Mk".to_string()),
            content_body: None,
            tags: vec!["rust".to_string(), "protocol".to_string()],
            dht_anchor_hash: None,
        },
    )
    .expect("create_content failed");
}

/// Mark a content row as DHT-notarised so the provenance gate passes.
fn mark_dht_notarised(conn: &mut SqliteConnection, id: &str) {
    diesel::sql_query(format!(
        "UPDATE content SET dht_anchor_hash = 'uhCkkTestAnchor001' WHERE id = '{id}'"
    ))
    .execute(conn)
    .expect("mark_dht_notarised failed");
}

/// Seed a content row exactly like [`seed_content`], but with a `blob_hash`
/// set — needed to key `peer_blob_inventory` rows (distribution facts) against
/// the content for the epr-head-envelope tests.
fn seed_content_with_blob_hash(conn: &mut SqliteConnection, id: &str, blob_hash: &str) {
    let ctx = AppContext::default_lamad();
    create_content(
        conn,
        &ctx,
        CreateContentInput {
            id: id.to_string(),
            title: "Test Content".to_string(),
            description: Some("A test concept for derive_epr_head".to_string()),
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: Some(blob_hash.to_string()),
            blob_cid: Some("bafkreitest000000000000000000000000000000000000000000001".to_string()),
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: Some("did:key:z6Mk".to_string()),
            content_body: None,
            tags: vec![],
            dht_anchor_hash: None,
        },
    )
    .expect("create_content failed");
}

/// Stamp `declared_head_at` (microseconds since the Unix epoch — the zome
/// `Timestamp` of the DHT declaration act) directly. Bypasses the real
/// notarized-commit / reconcile-stamp write paths, which is fine for a
/// pure-derivation test that only cares about the column's value.
fn set_declared_head_at(conn: &mut SqliteConnection, id: &str, micros: i64) {
    diesel::sql_query(format!(
        "UPDATE content SET declared_head_at = {micros} WHERE id = '{id}'"
    ))
    .execute(conn)
    .expect("set_declared_head_at failed");
}

/// Force `content.updated_at` (the local row mtime) to an explicit value —
/// simulates two peers' independently-drifted local clocks (the finding's
/// `02:00:30` vs `02:00:31`) without depending on real wall-clock skew
/// between two `create_content` calls.
fn set_updated_at(conn: &mut SqliteConnection, id: &str, value: &str) {
    diesel::sql_query(format!(
        "UPDATE content SET updated_at = '{value}' WHERE id = '{id}'"
    ))
    .execute(conn)
    .expect("set_updated_at failed");
}

/// Seed a `peer_blob_inventory` row so a distribution-summary composition
/// over `blob_hash` sees `peer_id` as a replica holder.
fn seed_peer_blob_inventory(conn: &mut SqliteConnection, blob_hash: &str, peer_id: &str) {
    diesel::sql_query(format!(
        "INSERT INTO peer_blob_inventory (peer_id, blob_hash, last_seen_at, source, sequence) \
         VALUES ('{peer_id}', '{blob_hash}', '2026-09-21T00:00:00Z', 'gossip-snapshot', 1)"
    ))
    .execute(conn)
    .expect("seed_peer_blob_inventory failed");
}

/// Seed an active stewardship allocation for the given content.
fn seed_stewardship_allocation(conn: &mut SqliteConnection, content_id: &str) {
    // Insert the minimal contributor_presences row required by the FK.
    // `establishing_content_ids_json` is NOT NULL so we supply an empty array.
    diesel::sql_query(
        "INSERT OR IGNORE INTO contributor_presences \
         (id, h_app_id, display_name, establishing_content_ids_json) \
         VALUES ('presence-steward-01', 'lamad', 'Steward One', '[]')",
    )
    .execute(conn)
    .expect("seed contributor_presences failed");

    diesel::sql_query(format!(
        "INSERT INTO stewardship_allocations \
         (id, h_app_id, content_id, steward_presence_id, allocation_ratio, \
          allocation_method, contribution_type, governance_state) \
         VALUES ('alloc-01', 'lamad', '{content_id}', 'presence-steward-01', \
                 0.75, 'manual', 'authorship', 'active')"
    ))
    .execute(conn)
    .expect("seed stewardship_allocations failed");
}

/// Seed an active (not revoked) attestation for the given content.
fn seed_attestation(conn: &mut SqliteConnection, content_id: &str) {
    diesel::sql_query(format!(
        "INSERT INTO content_attestations \
         (id, content_id, attestor_presence_id, scope, attestation_type, evidence, is_revoked) \
         VALUES ('att-01', '{content_id}', 'presence-steward-01', 'lamad', \
                 'prerequisite-mastery', 'calculus-101', 0)"
    ))
    .execute(conn)
    .expect("seed content_attestations failed");
}

// ---------------------------------------------------------------------------
// Test 1: missing content returns None
// ---------------------------------------------------------------------------

#[test]
fn derive_epr_head_returns_none_for_missing_content() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    let result = derive_epr_head(&mut conn, &ctx, "does-not-exist", false, false)
        .expect("derive_epr_head must not error on missing content");
    assert!(result.is_none(), "expected None for unknown content ID");
}

// ---------------------------------------------------------------------------
// Test 2: provenance gate filters unpublished rows (HTTP path behaviour)
// ---------------------------------------------------------------------------

#[test]
fn derive_epr_head_gate_provenance_hides_unpublished_rows() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-unpublished");

    // gate_provenance=true → unpublished row is invisible (HTTP handler behaviour)
    let gated =
        derive_epr_head(&mut conn, &ctx, "content-unpublished", true, false).expect("no DB error");
    assert!(
        gated.is_none(),
        "gate_provenance=true must hide rows without dht_anchor_hash or p2p_published_at"
    );

    // gate_provenance=false → same row is visible (P2P handler behaviour)
    let ungated =
        derive_epr_head(&mut conn, &ctx, "content-unpublished", false, false).expect("no DB error");
    assert!(
        ungated.is_some(),
        "gate_provenance=false must expose pre-publish rows"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Lamad fields match what the legacy inline code set
// ---------------------------------------------------------------------------

#[test]
fn derive_epr_head_lamad_fields_match_legacy_http_shape() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-lamad-check");
    mark_dht_notarised(&mut conn, "content-lamad-check");
    // epr-head-envelope design (Option A): `updated` now sources from
    // `declared_head_at`, not the row mtime — a row with no declared head
    // legitimately has no `updated` (test 3 pins that). Stamp one here so
    // this test's "author / updated populated" assertion stays meaningful.
    set_declared_head_at(&mut conn, "content-lamad-check", 1_700_000_000_000_000);

    let head = derive_epr_head(&mut conn, &ctx, "content-lamad-check", true, false)
        .expect("no DB error")
        .expect("head must be present after notarising");

    // Lamad context — mirrors what both legacy sites set
    assert_eq!(head.id, "content-lamad-check");
    assert_eq!(head.version, 1);
    assert_eq!(head.lamad.title, "Test Content");
    assert_eq!(head.lamad.content_type, "concept");
    assert_eq!(
        head.lamad.description.as_deref(),
        Some("A test concept for derive_epr_head")
    );
    assert_eq!(head.lamad.content_format.as_deref(), Some("markdown"));
    // Tags are stored in the content_tags index; order is alphabetical from the DB.
    let mut tags = head.lamad.tags.clone();
    tags.sort();
    assert_eq!(tags, vec!["protocol".to_string(), "rust".to_string()]);

    // author / updated populated
    assert_eq!(head.author.as_deref(), Some("did:key:z6Mk"));
    assert!(head.updated.is_some(), "updated must be set");

    // No enrichment — shefa/qahal empty (HTTP handler behaviour)
    assert!(head.shefa.stewards.is_empty());
    assert!(head.shefa.allocations.is_empty());
    assert!(head.qahal.attestation_requirements.is_empty());
    assert_eq!(head.qahal.reach.as_deref(), Some("commons"));
}

// ---------------------------------------------------------------------------
// Test 4: enrich_pillars=true populates shefa and qahal (P2P path behaviour)
// ---------------------------------------------------------------------------

// TODO(attestation-consolidation): The `content_attestations` table was dropped
// by migration 2026-05-12-100300_drop_legacy_attestation_tables (Stage A of the
// attestation consolidation sprint). The unified `attestations` table has a
// different schema (id is CID, no `attestor_presence_id`/`scope`/etc.). The
// seed_attestation helper below and the qahal enrichment path in
// `derive_epr_head` need to be migrated to the new table before re-enabling.
// Tracked: spec 2026-05-11-attestation-consolidation-design.md §7.4.
#[ignore = "blocked on attestation-consolidation table migration"]
#[test]
fn derive_epr_head_enrich_pillars_populates_shefa_and_qahal() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-enriched");
    seed_stewardship_allocation(&mut conn, "content-enriched");
    seed_attestation(&mut conn, "content-enriched");

    // gate_provenance=false, enrich_pillars=true — P2P handler behaviour
    let head = derive_epr_head(&mut conn, &ctx, "content-enriched", false, true)
        .expect("no DB error")
        .expect("head present");

    // Shefa enriched
    assert_eq!(head.shefa.stewards, vec!["presence-steward-01".to_string()]);
    assert!(
        (head.shefa.allocations[0] - 0.75_f64).abs() < 1e-6,
        "allocation_ratio must round-trip as f64 from f32 source"
    );

    // Qahal enriched — active attestation appears
    assert_eq!(
        head.qahal.attestation_requirements,
        vec!["prerequisite-mastery:calculus-101".to_string()]
    );
    assert_eq!(head.qahal.reach.as_deref(), Some("commons"));
}

// ---------------------------------------------------------------------------
// Test 5: DAG-CBOR wire round-trip (encode → decode → structural equality)
// ---------------------------------------------------------------------------

#[test]
fn derive_epr_head_wire_round_trip_is_stable() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-wire-rt");
    mark_dht_notarised(&mut conn, "content-wire-rt");

    let head = derive_epr_head(&mut conn, &ctx, "content-wire-rt", true, false)
        .expect("no DB error")
        .expect("head present");

    let (bytes, cid) = encode_epr_head(&head).expect("encode must succeed");
    assert!(!bytes.is_empty(), "encoded bytes must be non-empty");
    assert!(
        cid.to_string().starts_with("bafyr"),
        "CID must be dag-cbor CIDv1"
    );

    let decoded = decode_epr_head(&bytes).expect("decode must succeed");
    assert_eq!(
        decoded, head,
        "decoded head must be structurally identical to original"
    );
}

// ---------------------------------------------------------------------------
// Test 6: revoked attestations are excluded from qahal requirements
// ---------------------------------------------------------------------------

// TODO(attestation-consolidation): see note above on
// derive_epr_head_enrich_pillars_populates_shefa_and_qahal.
#[ignore = "blocked on attestation-consolidation table migration"]
#[test]
fn derive_epr_head_revoked_attestations_excluded() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-revoked-att");

    // Seed contributor presence first (needed by the attestation row).
    // `establishing_content_ids_json` is NOT NULL so we supply an empty array.
    diesel::sql_query(
        "INSERT OR IGNORE INTO contributor_presences \
         (id, h_app_id, display_name, establishing_content_ids_json) \
         VALUES ('presence-revoker-01', 'lamad', 'Revoker', '[]')",
    )
    .execute(&mut conn)
    .expect("seed contributor_presences");

    // Insert a revoked attestation (is_revoked = 1)
    diesel::sql_query(
        "INSERT INTO content_attestations \
         (id, content_id, attestor_presence_id, scope, attestation_type, is_revoked) \
         VALUES ('att-revoked-01', 'content-revoked-att', 'presence-revoker-01', \
                 'lamad', 'prerequisite-mastery', 1)",
    )
    .execute(&mut conn)
    .expect("seed revoked attestation");

    let head = derive_epr_head(&mut conn, &ctx, "content-revoked-att", false, true)
        .expect("no DB error")
        .expect("head present");

    assert!(
        head.qahal.attestation_requirements.is_empty(),
        "revoked attestations must not appear in qahal.attestation_requirements"
    );
}

// ---------------------------------------------------------------------------
// epr-head-envelope design (Option A, 2026-09-21) — peer-invariance tests.
//
// The finding: two peers that agree on a content's declared head served
// `GET /epr-head/{id}` bodies with different `cid` and different bytes.
// Cause 1: `updated` was `content.updated_at` (a local row mtime bumped by
// every stamp, including no-ops — the live incident recorded at
// `content_diesel.rs`'s `StampOutcome::Refreshed` doc comment: `updated_at`
// advanced 05:33:54 → 05:49:00 on an unchanged head). Cause 2: `distribution`
// (per-peer topology) rode in the JSON body, differing per peer.
//
// Cure: `updated` sources from `content.declared_head_at` (the DHT
// declaration act's own Timestamp — identical on every peer that holds that
// action); `distribution` moves to a sibling route.
// ---------------------------------------------------------------------------

/// Test 1 — the two-peer test, address half. Two independent peers agree on
/// the declared head but have different local row mtimes (mirrors the
/// finding's own evidence, `02:00:30` vs `02:00:31`). The cid must be
/// identical: it names the head, not either peer's moment.
///
/// This is the finding, reproduced in a harness — it fails on the code this
/// slice replaces (`updated: Some(content.updated_at.clone())`, sourced from
/// the row mtime instead of `declared_head_at`).
#[test]
fn epr_head_cid_is_invariant_to_local_row_mtime() {
    let pool_a = test_pool();
    let mut conn_a = pool_a.get().expect("pool connection A");
    let pool_b = test_pool();
    let mut conn_b = pool_b.get().expect("pool connection B");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn_a, "elohim-host-landing");
    mark_dht_notarised(&mut conn_a, "elohim-host-landing");
    set_declared_head_at(&mut conn_a, "elohim-host-landing", 1_700_000_000_000_000);
    set_updated_at(&mut conn_a, "elohim-host-landing", "2026-09-20T02:00:30Z");

    seed_content(&mut conn_b, "elohim-host-landing");
    mark_dht_notarised(&mut conn_b, "elohim-host-landing");
    set_declared_head_at(&mut conn_b, "elohim-host-landing", 1_700_000_000_000_000);
    set_updated_at(&mut conn_b, "elohim-host-landing", "2026-09-20T02:00:31Z");

    let head_a = derive_epr_head(&mut conn_a, &ctx, "elohim-host-landing", true, false)
        .expect("no DB error on peer A")
        .expect("head present on peer A");
    let head_b = derive_epr_head(&mut conn_b, &ctx, "elohim-host-landing", true, false)
        .expect("no DB error on peer B")
        .expect("head present on peer B");

    let (_bytes_a, cid_a) = encode_epr_head(&head_a).expect("encode A");
    let (_bytes_b, cid_b) = encode_epr_head(&head_b).expect("encode B");

    assert_eq!(
        cid_a, cid_b,
        "two peers agreeing on the declared head must mint one cid, \
         regardless of differing local row mtimes (02:00:30Z vs 02:00:31Z — \
         the finding's own evidence)"
    );
}

/// Test 2 (negative control / converse of test 1) — a REAL divergence in the
/// declared head must move the cid. Without this, test 1 would be
/// satisfiable by simply deleting `updated`, which would bless a field that
/// means nothing. `updated_at` is deliberately pinned IDENTICAL across both
/// peers here, isolating `declared_head_at` as the only variable — a
/// stronger, deterministic negative control than relying on incidental
/// wall-clock skew between two `create_content` calls.
#[test]
fn epr_head_cid_moves_when_the_declared_head_moves() {
    let pool_a = test_pool();
    let mut conn_a = pool_a.get().expect("pool connection A");
    let pool_b = test_pool();
    let mut conn_b = pool_b.get().expect("pool connection B");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn_a, "elohim-host-landing");
    mark_dht_notarised(&mut conn_a, "elohim-host-landing");
    set_declared_head_at(&mut conn_a, "elohim-host-landing", 1_700_000_000_000_000);
    set_updated_at(&mut conn_a, "elohim-host-landing", "2026-09-20T02:00:30Z");

    seed_content(&mut conn_b, "elohim-host-landing");
    mark_dht_notarised(&mut conn_b, "elohim-host-landing");
    // The only variable that changed: a real divergence in the declared head.
    set_declared_head_at(&mut conn_b, "elohim-host-landing", 1_700_000_001_000_000);
    set_updated_at(&mut conn_b, "elohim-host-landing", "2026-09-20T02:00:30Z");

    let head_a = derive_epr_head(&mut conn_a, &ctx, "elohim-host-landing", true, false)
        .expect("no DB error on peer A")
        .expect("head present on peer A");
    let head_b = derive_epr_head(&mut conn_b, &ctx, "elohim-host-landing", true, false)
        .expect("no DB error on peer B")
        .expect("head present on peer B");

    let (_bytes_a, cid_a) = encode_epr_head(&head_a).expect("encode A");
    let (_bytes_b, cid_b) = encode_epr_head(&head_b).expect("encode B");

    assert_ne!(
        cid_a, cid_b,
        "a real divergence in the declared head must move the cid — \
         invariance that survives a real divergence is the property; \
         invariance alone is just a deleted field"
    );
}

/// Test 3 — `declared_head_at` NULL must OMIT `updated` entirely: never the
/// row mtime, never an epoch-0 string. Honest absence (C4).
#[test]
fn epr_head_updated_is_absent_when_no_declared_head() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-no-declared-head");
    mark_dht_notarised(&mut conn, "content-no-declared-head");
    // declared_head_at is left NULL — the default on a freshly-created row.

    let head = derive_epr_head(&mut conn, &ctx, "content-no-declared-head", true, false)
        .expect("no DB error")
        .expect("head present");

    assert_eq!(
        head.updated, None,
        "declared_head_at NULL must OMIT `updated` — never the row mtime, \
         never an epoch-0 string"
    );

    let json = serde_json::to_value(&head).expect("serialize");
    assert!(
        json.get("updated").is_none(),
        "the JSON wire form must have no `updated` key at all, not a `null` \
         value: {json}"
    );
}

/// Test 4 — a fixed microsecond `declared_head_at` renders to one exact
/// RFC3339-UTC-seconds string. Any format drift here re-addresses every head
/// in the corpus, so this test is what makes that drift loud.
#[test]
fn epr_head_updated_renders_declared_head_at_in_rfc3339_seconds() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-declared-render");
    mark_dht_notarised(&mut conn, "content-declared-render");
    // 2026-09-20T02:00:30Z, in whole microseconds since the Unix epoch.
    set_declared_head_at(&mut conn, "content-declared-render", 1_789_869_630_000_000);

    let head = derive_epr_head(&mut conn, &ctx, "content-declared-render", true, false)
        .expect("no DB error")
        .expect("head present");

    assert_eq!(
        head.updated.as_deref(),
        Some("2026-09-20T02:00:30Z"),
        "declared_head_at must render as RFC3339 UTC, seconds precision"
    );
}

/// Test 5 — the two-peer test, validator half. Same declared head as test 1,
/// PLUS different `peer_blob_inventory` rows for the same `blob_hash` (3
/// peers vs 2 — reproducing the finding's `replicaCount` 3-vs-2 divergence).
/// The full JSON response body, composed through `compose_head_view`, must
/// be byte-identical: `mint_served_head` is a pure function of these exact
/// bytes, so byte-equality here IS validator-equality (the doorway ETag
/// claim, story 6.1) — asserted without any cross-crate doorway import.
#[test]
fn epr_head_body_is_byte_identical_across_peers_differing_only_in_local_state() {
    let pool_a = test_pool();
    let mut conn_a = pool_a.get().expect("pool connection A");
    let pool_b = test_pool();
    let mut conn_b = pool_b.get().expect("pool connection B");
    let ctx = AppContext::default_lamad();
    let blob_hash = "sha256-shared-blob-test5";

    seed_content_with_blob_hash(&mut conn_a, "elohim-host-landing", blob_hash);
    mark_dht_notarised(&mut conn_a, "elohim-host-landing");
    set_declared_head_at(&mut conn_a, "elohim-host-landing", 1_700_000_000_000_000);
    set_updated_at(&mut conn_a, "elohim-host-landing", "2026-09-20T02:00:30Z");
    // Peer A observes 3 replicas for the shared blob.
    seed_peer_blob_inventory(&mut conn_a, blob_hash, "peer-a1");
    seed_peer_blob_inventory(&mut conn_a, blob_hash, "peer-a2");
    seed_peer_blob_inventory(&mut conn_a, blob_hash, "peer-a3");

    seed_content_with_blob_hash(&mut conn_b, "elohim-host-landing", blob_hash);
    mark_dht_notarised(&mut conn_b, "elohim-host-landing");
    set_declared_head_at(&mut conn_b, "elohim-host-landing", 1_700_000_000_000_000);
    set_updated_at(&mut conn_b, "elohim-host-landing", "2026-09-20T02:00:31Z");
    // Peer B observes only 2 replicas — the finding's 3-vs-2 divergence.
    seed_peer_blob_inventory(&mut conn_b, blob_hash, "peer-b1");
    seed_peer_blob_inventory(&mut conn_b, blob_hash, "peer-b2");

    let view_a = compose_head_view(&mut conn_a, &ctx, "elohim-host-landing", true)
        .expect("no DB error on peer A")
        .expect("head present on peer A");
    let view_b = compose_head_view(&mut conn_b, &ctx, "elohim-host-landing", true)
        .expect("no DB error on peer B")
        .expect("head present on peer B");

    let bytes_a = serde_json::to_vec(&view_a).expect("serialize A");
    let bytes_b = serde_json::to_vec(&view_b).expect("serialize B");

    assert_eq!(
        bytes_a, bytes_b,
        "the full JSON response body must be byte-identical across peers \
         that agree on the declared head, even with different \
         peer_blob_inventory replica counts (3 vs 2) and different local row \
         mtimes"
    );
}

/// Test 6 — the head envelope must never carry `distribution`, even when
/// `peer_blob_inventory` rows exist for the content's blob. Pins the removal
/// against re-introduction: distribution is served separately at
/// `GET /api/v1/blob/{hash}/distribution/summary`.
#[test]
fn epr_head_json_body_carries_no_distribution() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();
    let blob_hash = "sha256-test6-blob";

    seed_content_with_blob_hash(&mut conn, "content-no-dist", blob_hash);
    mark_dht_notarised(&mut conn, "content-no-dist");
    seed_peer_blob_inventory(&mut conn, blob_hash, "peer-x1");

    let view = compose_head_view(&mut conn, &ctx, "content-no-dist", true)
        .expect("no DB error")
        .expect("head present");

    let json = serde_json::to_value(&view).expect("serialize");
    assert!(
        json.get("distribution").is_none(),
        "the head envelope must never carry `distribution`, even when \
         inventory rows exist for the content's blob: {json}"
    );
}

/// Test 7 — the dag-cbor arm (what the HTTP handler's `wants_cbor` branch
/// serves) and the JSON arm (`compose_head_view`, what the default branch
/// serves) must agree on every field the cid covers, for one content id.
/// Pins that the two representations cannot drift apart again — the drift
/// that produced this finding was exactly two representations of "the same"
/// head disagreeing (the enrich-split, F1, is a DIFFERENT and out-of-scope
/// divergence between the HTTP and P2P call sites; this test is scoped to
/// the two representations of one call site's derivation).
#[test]
fn epr_head_json_and_cbor_agree_on_every_addressed_field() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let ctx = AppContext::default_lamad();

    seed_content(&mut conn, "content-json-cbor-agree");
    mark_dht_notarised(&mut conn, "content-json-cbor-agree");
    set_declared_head_at(&mut conn, "content-json-cbor-agree", 1_700_000_000_000_000);

    // The dag-cbor arm: derive + encode directly.
    let head = derive_epr_head(&mut conn, &ctx, "content-json-cbor-agree", true, false)
        .expect("no DB error")
        .expect("head present");
    let (cbor_bytes, _cid) = encode_epr_head(&head).expect("cbor encode");
    let decoded_from_cbor = decode_epr_head(&cbor_bytes).expect("cbor decode");

    // The JSON arm: compose_head_view. Reconstruct the addressed struct from
    // the view's own fields (excluding the response-only `cid`, and
    // `distribution`, which is structurally absent) for a direct comparison.
    let view = compose_head_view(&mut conn, &ctx, "content-json-cbor-agree", true)
        .expect("no DB error")
        .expect("head present");
    let decoded_from_json = elohim_storage::epr_codec::EprHead {
        version: view.version,
        id: view.id.clone(),
        content: view.content.clone(),
        lamad: view.lamad.clone(),
        shefa: view.shefa.clone(),
        qahal: view.qahal.clone(),
        relationships: view.relationships.clone(),
        author: view.author.clone(),
        updated: view.updated.clone(),
    };

    assert_eq!(
        decoded_from_cbor, decoded_from_json,
        "the dag-cbor arm and the JSON arm must agree on every addressed \
         field for one content id"
    );
}
