//! Integration test: EPR projection fan-out — relational + graph projections hold the same CID.
//!
//! Validates that after an EPR is ingested into epr_atoms (diesel) and projected
//! via GraphProjector (CozoDB), both projections agree on the CID (spec §6.2).
//!
//! This test uses the service layer directly (not HTTP) because the HTTP-level
//! harness requires a running server. The fan-out logic in `put_epr` does the
//! same decode + project_head sequence tested here.
//!
//! Feature gate: `graph-native` (default-on; cozo dep).

#![cfg(feature = "graph-native")]

use chrono::{TimeZone, Utc};
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::db::epr_atoms::fetch_atom_by_cid;
use elohim_storage::epr_codec::decode_epr_head;
use elohim_storage::graph::{
    engine::GraphEngine, projector::GraphProjector, schema::apply_core_schema,
};
use elohim_storage::services::epr_service;
use elohim_storage::test_util::test_pool;

/// Build a minimal Content EPR with an EprHead-compatible canonical_bytes payload.
fn build_content_epr_with_head() -> Epr {
    let kp = AgentKeypair::from_secret(&[0xFA; 32]).expect("test keypair");
    let agent_cid = compute_cid(b"fanout-test-signer");
    let schema_ref = compute_cid(b"fanout-schema-ref");

    // Payload: a valid EprHead serialized as DAG-CBOR so decode_epr_head succeeds.
    // canonical_bytes in epr_atoms are written by epr_service::ingest via Epr::canonical_bytes().
    // For this test we use a simple JSON payload (EprHead supports JSON or DAG-CBOR detection).
    let head = elohim_storage::epr_codec::EprHead {
        version: 1,
        id: "fanout-test".into(),
        content: "bafycontent-fanout".into(),
        lamad: elohim_storage::epr_codec::EprLamadContext {
            title: "Fan-out Test".into(),
            content_type: "concept".into(),
            description: None,
            content_format: None,
            tags: vec![],
        },
        shefa: elohim_storage::epr_codec::EprShefaContext {
            stewards: vec![],
            allocations: vec![],
        },
        qahal: elohim_storage::epr_codec::EprQahalContext {
            reach: Some("commons".into()),
            layer: None,
            attestation_requirements: vec![],
        },
        relationships: vec![],
        author: Some("did:test:fanout".into()),
        updated: Some("2026-05-16T00:00:00Z".into()),
    };
    // Encode as JSON (0x7B prefix) — decode_epr_head auto-detects.
    let head_bytes = serde_json::to_vec(&head).expect("serialize head");

    Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(schema_ref)
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(compute_cid(b"knowledge")),
            value: Some(compute_cid(b"value")),
            governance: Some(compute_cid(b"governance")),
        })
        .issued_at(Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, 0).unwrap())
        .payload(head_bytes)
        .sign(&kp, agent_cid)
        .expect("sign epr")
}

#[test]
fn put_epr_fans_out_to_both_relational_and_graph_projections() {
    // Step 1: Set up diesel pool (in-memory SQLite with migrations applied).
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    // Step 2: Set up graph engine (sled, temp dir).
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    // Step 3: Ingest EPR into diesel (simulates what put_epr's store.put does).
    let epr = build_content_epr_with_head();
    let epr_cid = epr.envelope.cid.to_string();
    epr_service::ingest(&mut conn, epr).expect("ingest must succeed");

    // Step 4: Assert relational projection: epr_atoms row with matching CID.
    let atom = fetch_atom_by_cid(&mut conn, &epr_cid)
        .expect("diesel query")
        .expect("atom row must exist after ingest");
    assert_eq!(atom.cid, epr_cid, "diesel CID must match");

    // Step 5: Simulate the graph fan-out (same logic as put_epr's graph-native block).
    // Content-kind EPRs carry the EprHead JSON as payload_bytes (not canonical_bytes,
    // which is the IPLD envelope encoding). Non-Content kinds are skipped in production.
    let head = decode_epr_head(&atom.payload_bytes).expect("decode EprHead from payload_bytes");
    let projector = GraphProjector::new(&engine);
    projector
        .project_head(&epr_cid, &head)
        .expect("graph projection must succeed");

    // Step 6: Assert graph projection: epr_node row with matching CID.
    let graph_result = engine
        .run_script(
            r#"?[slug] := *epr_node{cid: $cid, slug}"#,
            &[("cid", cozo::DataValue::from(epr_cid.as_str()))],
        )
        .expect("graph query");
    assert_eq!(
        graph_result.rows.len(),
        1,
        "epr_node row must be present in graph after fan-out"
    );

    // Step 7: Assert both projections agree on the CID (the EPR CID is the key in both).
    // Diesel: atom.cid; Graph: epr_node primary key is $cid — the query above uses it as a param.
    // A single row returned confirms the same CID is the primary key in both stores.
}
