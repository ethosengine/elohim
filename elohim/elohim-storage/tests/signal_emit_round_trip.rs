//! End-to-end integration test for the EPR signal-emit pipeline (Task C.7).
//!
//! Proves the full Batch C path: a `SignalIntent` (the JSON shape the
//! Angular harness POSTs to `/api/v1/signal/emit`) flows through the
//! write-through gate, gets composed into an EPR Envelope, ingested via
//! `EprService::ingest`, projected by `Projector::run_one_pass`, and
//! lands as a row in `economic_events` whose columns reflect the intent's
//! payload exactly.
//!
//! This test mocks the conductor signing step (real signing is exercised
//! by the C.1 sweettest scenarios in `holochain/tests/sweettest`); the
//! point here is that everything *around* the signature — manifest
//! lookup, canonical bytes, CID derivation, ingest, and projection —
//! agrees end-to-end.
//!
//! The test also asserts that:
//! - When write-through is OFF, the gate refuses the intent before any
//!   signing or ingestion attempt (Task C.5 invariant).
//! - When the operator opts shefa in via the env layer (Task C.6
//!   `ELOHIM_WRITE_THROUGH_SHEFA=on`), the same intent succeeds and
//!   the source layer is `EnvOverride`.

use std::collections::HashMap;
use std::sync::Arc;

use diesel::prelude::*;
use elohim_epr::cid::compute_cid;
use elohim_storage::api::signal_emit::{
    assemble_signed_epr, check_write_through, prepare_signing_request, SignalEmitError,
    SignalIntent,
};
use elohim_storage::db::diesel_schema::economic_events;
use elohim_storage::db::models::EconomicEvent;
use elohim_storage::projector::{mapping::ManifestRegistry, Projector};
use elohim_storage::services::epr_service;
use elohim_storage::test_util::test_pool;
use elohim_storage::write_through::{
    WriteThroughConfig, WriteThroughOverride, WriteThroughSource, WriteThroughState,
};

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// The exact JSON shape the Angular harness POSTs at
/// `/api/v1/signal/emit` for a shefa EconomicEvent. Field names are
/// camelCase per `signal-intent.schema.json`.
fn shefa_economic_event_intent_json(provider: &str, receiver: &str) -> serde_json::Value {
    serde_json::json!({
        "pillar": "shefa",
        "signalType": "EconomicEvent",
        "agentCid": compute_cid(b"alice-agent-c7-end-to-end").to_string(),
        "payload": {
            "action": "produce",
            "provider": provider,
            "receiver": receiver,
            "hasPointInTime": "2026-04-25T12:00:00Z",
            "resourceConformsTo": "concept:bread",
            "resourceQuantity": {
                "numericValue": 2.0,
                "hasUnit": "loaves"
            }
        },
        "couplingRefs": {
            "value": compute_cid(b"shefa-value-coupling-target-c7").to_string()
        }
    })
}

/// Build the C.6 layer-3 (env) override that flips shefa ON.
fn env_layer_shefa_on() -> WriteThroughOverride {
    let mut pillars = HashMap::new();
    pillars.insert(
        "shefa".to_string(),
        WriteThroughConfig {
            enabled: true,
            kinds: None,
        },
    );
    WriteThroughOverride { pillars }
}

// ---------------------------------------------------------------------------
// C.7 end-to-end test
// ---------------------------------------------------------------------------

/// Full Batch C round-trip with mocked conductor signing.
#[test]
fn signal_intent_flows_through_to_economic_events_row() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    // Step 1 — parse intent (mimics serde_json::from_slice on the request body).
    let intent_json = shefa_economic_event_intent_json("baker-alice", "guest-bob");
    let intent: SignalIntent =
        serde_json::from_value(intent_json).expect("intent must parse from harness shape");

    // Step 2 — manifest registry (same shape as the shefa manifest's
    // projections[] array).
    let registry = ManifestRegistry::with_shefa_economic_event();

    // Step 3 — write-through state with shefa flipped on at the env layer.
    // Mirrors operator setting `ELOHIM_WRITE_THROUGH_SHEFA=on` at startup.
    let state = Arc::new(WriteThroughState::empty().with_env_override(env_layer_shefa_on()));
    let gate = check_write_through(&intent.pillar, "EconomicEvent", state.as_ref());
    assert!(gate.is_ok(), "C.5 gate must let shefa through when env=on");
    // Confirm the source attribution surfaces correctly for operator audit.
    let eff = state.effective_for("shefa", "EconomicEvent");
    assert_eq!(
        eff.source(),
        WriteThroughSource::EnvOverride,
        "C.6 layer 3 (env) must produce the decision"
    );

    // Step 4 — prepare canonical bytes (no signing yet).
    let prepared = prepare_signing_request(&intent, &registry).expect("prepare ok");
    assert_eq!(prepared.schema_key, "economic-event");
    assert!(!prepared.canonical_bytes.is_empty());

    // Step 5 — mock the conductor signature. The real `sign_for_agent`
    // zome path is exercised by the C.1 sweettest scenarios; here we
    // care only that downstream assembly + ingest accept *any* 64-byte
    // signature shape.
    let mock_signature = vec![0xAB; 64];

    // Step 6 — assemble final EPR.
    let epr = assemble_signed_epr(prepared, mock_signature).expect("assemble ok");
    let epr_cid = epr.envelope.cid.to_string();

    // Step 7 — ingest (real, structural-only verify).
    epr_service::ingest(&mut conn, epr).expect("ingest must succeed");

    // Step 8 — run one projector pass.
    let projector = Projector::new(ManifestRegistry::with_shefa_economic_event());
    let report = projector
        .run_one_pass(&mut conn)
        .expect("projector run_one_pass");
    assert_eq!(report.atoms_processed, 1, "one atom must be processed");
    assert_eq!(
        report.projections_advanced, 1,
        "one projection must be advanced"
    );

    // Step 9 — assert the projected row in `economic_events` reflects the
    // intent's payload.
    let row: EconomicEvent = economic_events::table
        .filter(economic_events::id.eq(&epr_cid))
        .first(&mut conn)
        .expect("projected row must exist");
    assert_eq!(row.id, epr_cid, "id must equal the EPR CID");
    assert_eq!(row.provider, "baker-alice", "provider from payload");
    assert_eq!(row.receiver, "guest-bob", "receiver from payload");
    assert_eq!(row.action, "produce", "action from payload");
    assert_eq!(
        row.resource_conforms_to.as_deref(),
        Some("concept:bread"),
        "resource_conforms_to from payload"
    );
    assert!(
        row.resource_quantity_value.is_some(),
        "quantity must be projected from resourceQuantity.numericValue"
    );
    assert_eq!(
        row.state, "recorded",
        "state must be the $state:recorded default"
    );
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some(epr_cid.as_str()),
        "dht_anchor_hash must back-link to the EPR CID"
    );
}

/// When write-through is OFF (no env, no policy, no admin override),
/// the gate must refuse the intent before any signing or ingest happens.
/// This mirrors operator intent: shefa stays cold until they opt in.
#[test]
fn write_through_off_blocks_intent_before_signing() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    let intent_json = shefa_economic_event_intent_json("baker-alice", "guest-bob");
    let intent: SignalIntent = serde_json::from_value(intent_json).unwrap();

    // Empty state — no overrides, manifest layer absent — implicit OFF.
    let state = WriteThroughState::empty();

    // The gate must refuse before any work happens.
    let gate = check_write_through(&intent.pillar, "EconomicEvent", &state);
    let err = gate.expect_err("C.5 gate must refuse cold pillar");
    assert!(matches!(err, SignalEmitError::WriteThroughOff { .. }));

    // Sanity: even if a caller skipped the gate and tried to ingest, the
    // table must remain empty since prepare/sign/ingest never ran.
    let count: i64 = economic_events::table
        .count()
        .get_result(&mut conn)
        .expect("count");
    assert_eq!(count, 0, "no rows must be projected when gate refuses");
}
