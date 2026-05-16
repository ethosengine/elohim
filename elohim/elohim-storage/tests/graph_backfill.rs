//! Integration test: backfill_graph projects relational epr_atoms into the CozoDB graph.
//!
//! Inserts N EprHead atoms directly into epr_atoms (bypassing HTTP) to simulate
//! pre-graph data, then runs backfill_graph and asserts every row has a corresponding
//! epr_node in CozoDB.
//!
//! Feature gate: `graph-native` (default-on).

#![cfg(feature = "graph-native")]

use chrono::{TimeZone, Utc};
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::epr_codec::{EprHead, EprLamadContext, EprQahalContext, EprShefaContext};
use elohim_storage::graph::{
    backfill::{backfill_graph, BackfillOpts},
    engine::GraphEngine,
    schema::apply_core_schema,
};
use elohim_storage::services::epr_service;
use elohim_storage::test_util::test_pool;

/// Build and ingest an EPR whose canonical_bytes encode the given EprHead.
///
/// We build a real `Epr` with the head JSON-encoded as payload so that
/// `epr_service::ingest` writes `canonical_bytes` that `decode_epr_head` can recover.
fn ingest_head_as_epr(conn: &mut diesel::SqliteConnection, seed: u8, id: &str) -> String {
    let kp = AgentKeypair::from_secret(&[seed; 32]).expect("test keypair");
    let agent_cid = compute_cid(&[seed]);
    let schema_ref = compute_cid(b"backfill-schema");

    let head = EprHead {
        version: 1,
        id: id.to_string(),
        content: format!("bafycontent-{id}"),
        lamad: EprLamadContext {
            title: format!("Backfill {id}"),
            content_type: "concept".into(),
            description: None,
            content_format: None,
            tags: vec![],
        },
        shefa: EprShefaContext {
            stewards: vec![],
            allocations: vec![],
        },
        qahal: EprQahalContext {
            reach: Some("commons".into()),
            layer: None,
            attestation_requirements: vec![],
        },
        relationships: vec![],
        author: None,
        updated: None,
    };
    let head_bytes = serde_json::to_vec(&head).expect("serialize head");

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(schema_ref)
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(compute_cid(&[seed, 1])),
            value: Some(compute_cid(&[seed, 2])),
            governance: Some(compute_cid(&[seed, 3])),
        })
        .issued_at(
            Utc.with_ymd_and_hms(2026, 5, 16, 0, 0, (seed % 60) as u32)
                .unwrap(),
        )
        .payload(head_bytes)
        .sign(&kp, agent_cid)
        .expect("sign epr");

    let cid = epr.envelope.cid.to_string();
    epr_service::ingest(conn, epr).expect("ingest");
    cid
}

#[test]
fn backfill_projects_all_existing_relational_atoms_to_graph() {
    // Step 1: Set up diesel pool with migrations applied.
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");

    // Step 2: Insert 3 EPR atoms directly into epr_atoms (pre-graph data simulation).
    let cids: Vec<String> = (0u8..3)
        .map(|i| ingest_head_as_epr(&mut conn, i + 10, &format!("backfill-concept-{i}")))
        .collect();
    assert_eq!(cids.len(), 3, "three atoms ingested");

    // Step 3: Set up an empty CozoDB graph (no prior projections).
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    // Confirm graph starts empty for these CIDs.
    for cid in &cids {
        let pre = engine
            .run_script(
                r#"?[slug] := *epr_node{cid: $cid, slug}"#,
                &[("cid", cozo::DataValue::from(cid.as_str()))],
            )
            .unwrap();
        assert_eq!(pre.rows.len(), 0, "graph must start empty for {cid}");
    }

    // Step 4: Run backfill_graph.
    let report =
        backfill_graph(&mut conn, &engine, BackfillOpts::default()).expect("backfill must succeed");

    // Step 5: Assert all atoms were projected (no failures).
    assert_eq!(report.projected, 3, "all 3 atoms must be projected");
    assert_eq!(report.failed, 0, "no failures expected");

    // Step 6: Assert every diesel row has a corresponding epr_node in CozoDB.
    for cid in &cids {
        let post = engine
            .run_script(
                r#"?[slug] := *epr_node{cid: $cid, slug}"#,
                &[("cid", cozo::DataValue::from(cid.as_str()))],
            )
            .unwrap();
        assert_eq!(
            post.rows.len(),
            1,
            "epr_node must exist for {cid} after backfill"
        );
    }
}

#[test]
fn backfill_is_idempotent() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test db connection");
    let cid = ingest_head_as_epr(&mut conn, 99, "idempotent-concept");

    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    // Run twice — second run should also succeed with the same projected count.
    let r1 = backfill_graph(&mut conn, &engine, BackfillOpts::default()).unwrap();
    let r2 = backfill_graph(&mut conn, &engine, BackfillOpts::default()).unwrap();

    assert_eq!(r1.projected, 1);
    assert_eq!(r2.projected, 1, "second run must re-project idempotently");
    assert_eq!(r1.failed + r2.failed, 0);

    let post = engine
        .run_script(
            r#"?[slug] := *epr_node{cid: $cid, slug}"#,
            &[("cid", cozo::DataValue::from(cid.as_str()))],
        )
        .unwrap();
    assert_eq!(
        post.rows.len(),
        1,
        "exactly one node after idempotent backfill"
    );
}
