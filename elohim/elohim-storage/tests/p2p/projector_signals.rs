//! B.8 acceptance test — projector emits a [`ProjectorSignal`] on every
//! successful UPSERT into a pillar read-model table.
//!
//! The test wires a `tokio::sync::mpsc` channel to the projector, runs one
//! pass, and asserts that the expected signal is received.

use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::projector::{mapping::ManifestRegistry, Projector, ProjectorSignal};
use elohim_storage::services::epr_service;
use elohim_storage::test_util::test_pool;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Helper — build a signed EconomicEvent EPR (mirrors projector_round_trip.rs)
// ---------------------------------------------------------------------------

fn build_economic_event_epr() -> Epr {
    let issued_at = chrono::DateTime::parse_from_rfc3339("2026-04-25T10:00:00Z")
        .expect("fixed timestamp")
        .with_timezone(&chrono::Utc);

    let payload_json = serde_json::json!({
        "provider": "provider-signal-test",
        "receiver": "receiver-signal-test",
        "action": "use",
        "hasPointInTime": "2026-04-25T10:00:00Z",
        "resourceConformsTo": "concept:signal",
        "resourceQuantity": { "numericValue": 1.0, "hasUnit": "hour" }
    });
    let payload_bytes = serde_json::to_vec(&payload_json).unwrap();

    let kp = AgentKeypair::from_secret(&[0xB8; 32]).expect("test keypair");
    let signer_cid = compute_cid(b"signal-test-signer");
    let value_target = compute_cid(b"signal-value-coupling");
    let coupling = Coupling {
        knowledge: None,
        value: Some(value_target),
        governance: None,
    };
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
        .expect("sign epr")
}

// ---------------------------------------------------------------------------
// B.8 signal emission test
// ---------------------------------------------------------------------------

/// Projector emits a [`ProjectorSignal`] on successful UPSERT (B.8 acceptance).
///
/// Setup: ingest one `EconomicEvent` EPR, attach an `mpsc` channel to the
/// projector, run one pass.
/// Assert: the signal has the correct `pillar`, `kind`, `target_table`, and
/// non-empty `epr_cid`.
#[tokio::test]
async fn projector_emits_signal_on_write() {
    let pool = test_pool();
    let mut conn = pool.get().expect("connection");

    let epr = build_economic_event_epr();
    let epr_cid = epr.envelope.cid.to_string();

    epr_service::ingest(&mut conn, epr).expect("ingest must succeed");

    let (tx, mut rx) = mpsc::channel::<ProjectorSignal>(4);

    let registry = ManifestRegistry::with_shefa_economic_event();
    let projector = Projector::new(registry).with_signal_sink(tx);

    projector.run_one_pass(&mut conn).expect("run_one_pass");

    // The channel must have exactly one signal (one atom projected).
    let signal = rx.recv().await.expect("signal must be sent");

    assert_eq!(signal.pillar, "shefa");
    assert_eq!(signal.kind, "EconomicEvent");
    assert_eq!(signal.target_table, "economic_events");
    assert!(!signal.epr_cid.is_empty(), "epr_cid must not be empty");
    assert_eq!(signal.epr_cid, epr_cid, "epr_cid must match ingested EPR");
    assert_eq!(
        signal.row_key, epr_cid,
        "row_key is CID for CID-keyed tables"
    );
    assert!(!signal.timestamp.is_empty(), "timestamp must be set");

    // No more signals in the channel (only one atom was ingested).
    assert!(
        rx.try_recv().is_err(),
        "no further signals expected after one atom"
    );
}

/// When no sink is attached, the projector runs normally without panicking.
#[tokio::test]
async fn projector_without_sink_runs_normally() {
    let pool = test_pool();
    let mut conn = pool.get().expect("connection");

    let epr = build_economic_event_epr();
    epr_service::ingest(&mut conn, epr).expect("ingest");

    // No sink attached — default constructor.
    let registry = ManifestRegistry::with_shefa_economic_event();
    let projector = Projector::new(registry);

    let report = projector.run_one_pass(&mut conn).expect("run_one_pass");
    assert_eq!(report.atoms_processed, 1);
}
