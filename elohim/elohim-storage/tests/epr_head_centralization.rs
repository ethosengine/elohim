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
use elohim_storage::epr_head::derive_epr_head;
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
