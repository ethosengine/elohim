//! Round-trip integration test for the EPR projector (Task B.4).
//!
//! Proves that an `EconomicEvent` EPR ingested via `EprService::ingest` is
//! correctly projected into the `economic_events` read-model table after a
//! single `Projector::run_one_pass` call.
//!
//! The test constructs a signed EPR whose payload is a JSON object carrying
//! the fields declared in the shefa manifest's `columnMapping`, ingests it,
//! runs the projector, and asserts the projected row in `economic_events`
//! reflects the EPR payload exactly.

use diesel::prelude::*;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::db::diesel_schema::economic_events;
use elohim_storage::db::models::EconomicEvent;
use elohim_storage::projector::{mapping::ManifestRegistry, Projector};
use elohim_storage::services::epr_service;
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Helper — build a signed EconomicEvent EPR with a structured JSON payload
// ---------------------------------------------------------------------------

/// Build a signed `EconomicEvent` EPR.
///
/// The payload is a JSON object shaped to match the shefa manifest's
/// `columnMapping` source paths:
///
/// ```json
/// {
///   "provider":       <provider_id>,
///   "receiver":       <receiver_id>,
///   "action":         <action>,
///   "hasPointInTime": <issued_at_iso>,
///   "resourceConformsTo": <resource_conforms_to>,
///   "resourceQuantity": { "numericValue": <qty_value>, "hasUnit": <qty_unit> }
/// }
/// ```
///
/// The EPR is signed with a deterministic test keypair so the CID is stable
/// across repetitions with the same inputs (deterministic payload → deterministic CID).
fn build_economic_event_epr(
    provider_id: &str,
    receiver_id: &str,
    qty_value: f64,
    qty_unit: &str,
    action: &str,
    resource_conforms_to: &str,
) -> Epr {
    let issued_at = chrono::DateTime::parse_from_rfc3339("2026-04-25T10:00:00Z")
        .expect("fixed test timestamp must parse")
        .with_timezone(&chrono::Utc);

    let payload_json = serde_json::json!({
        "provider": provider_id,
        "receiver": receiver_id,
        "action": action,
        "hasPointInTime": "2026-04-25T10:00:00Z",
        "resourceConformsTo": resource_conforms_to,
        "resourceQuantity": {
            "numericValue": qty_value,
            "hasUnit": qty_unit
        }
    });
    let payload_bytes =
        serde_json::to_vec(&payload_json).expect("serialize economic event payload");

    // Deterministic keypair for this test.
    let kp = AgentKeypair::from_secret(&[0xEC; 32]).expect("test keypair");
    let signer_cid = compute_cid(b"economic-event-test-signer-b4");

    // EconomicEvent kind requires only the Value coupling leg.
    let value_target = compute_cid(b"shefa-value-coupling-target");
    let coupling = Coupling {
        knowledge: None,
        value: Some(value_target),
        governance: None,
    };

    // schema_ref must be a valid CID; schema_key must match the projector query.
    let schema_ref = compute_cid(b"economic-event-schema-v1");

    Epr::builder()
        .kind(EprKind::EconomicEvent)
        .schema_ref(schema_ref)
        .schema_key("economic-event")
        .reach(Reach::Commons)
        .coupling(coupling)
        .issued_at(issued_at)
        .payload(payload_bytes)
        .sign(&kp, signer_cid)
        .expect("sign economic event epr")
}

// ---------------------------------------------------------------------------
// B.4 round-trip test
// ---------------------------------------------------------------------------

/// Ingest an EconomicEvent EPR, run one projector pass, and assert the
/// `economic_events` row reflects the EPR payload.
///
/// ## Invariants checked
///
/// 1. `id = epr.envelope.cid.to_string()` — CID used as PK (idempotent UPSERT).
/// 2. `provider`, `receiver`, `action` — from payload paths.
/// 3. `resource_quantity_value` — from nested `resourceQuantity.numericValue`.
/// 4. `state = "recorded"` — projector-derived constant.
/// 5. `dht_anchor_hash = epr.envelope.cid.to_string()` — provenance back-link.
/// 6. `created_at = epr.envelope.issued_at (ISO-8601)` — from `$issuedAt`.
/// 7. `h_app_id = epr.envelope.proof.signer.to_string()` — from `$signer`.
#[test]
fn economic_event_epr_projects_to_economic_events_table() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    // Step 1: Build and ingest the EPR.
    let epr = build_economic_event_epr(
        "provider-A",
        "receiver-B",
        5.0,
        "hour",
        "use",
        "concept:foo",
    );
    let epr_cid = epr.envelope.cid.to_string();
    let signer_cid = epr.envelope.proof.signer.to_string();

    epr_service::ingest(&mut conn, epr).expect("ingest must succeed");

    // Step 2: Build the projector with the mock registry
    // (same mapping as the shefa manifest — see ManifestRegistry::with_shefa_economic_event).
    let registry = ManifestRegistry::with_shefa_economic_event();
    let projector = Projector::new(registry);

    // Step 3: Run one projection pass.
    let report = projector.run_one_pass(&mut conn).expect("run_one_pass");

    assert_eq!(report.atoms_processed, 1, "one atom must be processed");
    assert_eq!(
        report.projections_advanced, 1,
        "one projection must be advanced"
    );

    // Step 4: Query the economic_events table.
    let row: EconomicEvent = economic_events::table
        .filter(economic_events::id.eq(&epr_cid))
        .first(&mut conn)
        .expect("projected row must exist in economic_events");

    // PK sourced from $cid.
    assert_eq!(row.id, epr_cid, "id must equal the EPR CID");

    // Payload-sourced columns.
    assert_eq!(row.provider, "provider-A", "provider from payload");
    assert_eq!(row.receiver, "receiver-B", "receiver from payload");
    assert_eq!(row.action, "use", "action from payload");
    assert_eq!(
        row.resource_conforms_to.as_deref(),
        Some("concept:foo"),
        "resource_conforms_to from payload"
    );
    assert!(
        row.resource_quantity_value.is_some(),
        "resource_quantity_value must be projected from resourceQuantity.numericValue"
    );
    let qty: f32 = row.resource_quantity_value.unwrap();
    assert!(
        (qty - 5.0_f32).abs() < 0.001,
        "resource_quantity_value must be 5.0, got {qty}"
    );

    // Projector-derived columns.
    assert_eq!(
        row.state, "recorded",
        "state must be the $state:recorded default"
    );
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some(epr_cid.as_str()),
        "dht_anchor_hash must equal the EPR CID"
    );
    assert_eq!(
        row.h_app_id, signer_cid,
        "h_app_id must equal the signer CID ($signer)"
    );
    assert_eq!(
        row.created_at, "2026-04-25T10:00:00Z",
        "created_at must equal the EPR issued_at ($issuedAt)"
    );
}

/// A second pass with the same ingested EPR is a no-op (cursor advanced past it).
/// The `economic_events` row must remain unchanged.
#[test]
fn projector_second_pass_with_same_epr_is_noop() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    let epr = build_economic_event_epr(
        "provider-A",
        "receiver-B",
        5.0,
        "hour",
        "use",
        "concept:foo",
    );
    let epr_cid = epr.envelope.cid.to_string();
    epr_service::ingest(&mut conn, epr).expect("ingest");

    let projector = Projector::new(ManifestRegistry::with_shefa_economic_event());

    // First pass — projects.
    let r1 = projector.run_one_pass(&mut conn).unwrap();
    assert_eq!(r1.atoms_processed, 1);

    // Second pass — no new atoms since cursor was advanced.
    let r2 = projector.run_one_pass(&mut conn).unwrap();
    assert_eq!(r2.atoms_processed, 0, "second pass must process zero atoms");

    // Row still present (INSERT OR REPLACE is idempotent; second pass just skips).
    let count: i64 = economic_events::table
        .filter(economic_events::id.eq(&epr_cid))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(count, 1, "exactly one row must exist after two passes");
}
