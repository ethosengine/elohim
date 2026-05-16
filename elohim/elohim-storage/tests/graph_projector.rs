#![cfg(feature = "graph-native")]

use elohim_storage::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};
use elohim_storage::graph::{engine::GraphEngine, projector::GraphProjector, schema::apply_core_schema};

fn sample_head() -> EprHead {
    EprHead {
        version: 1,
        id: "test-slug".into(),
        content: "bafycontent".into(),
        lamad: EprLamadContext {
            title: "Sample".into(),
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
        author: Some("did:test:abc".into()),
        updated: Some("2026-05-16T00:00:00Z".into()),
    }
}

// ---------------------------------------------------------------------------
// Task 9: project_head writes node + three pillars
// ---------------------------------------------------------------------------

#[test]
fn project_head_writes_node_and_three_pillars() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    let cid = "bafyN1";
    let head = sample_head();
    projector.project_head(cid, &head).expect("project");

    let node = engine
        .run_script(
            r#"?[slug] := *epr_node{cid: $cid, slug}"#,
            &[("cid", cozo::DataValue::from(cid))],
        )
        .unwrap();
    assert_eq!(node.rows.len(), 1, "epr_node row must be present");

    let lamad = engine
        .run_script(
            r#"?[title] := *epr_lamad{cid: $cid, title}"#,
            &[("cid", cozo::DataValue::from(cid))],
        )
        .unwrap();
    assert_eq!(lamad.rows.len(), 1, "epr_lamad row must be present");

    let shefa = engine
        .run_script(
            r#"?[stewards] := *epr_shefa{cid: $cid, stewards}"#,
            &[("cid", cozo::DataValue::from(cid))],
        )
        .unwrap();
    assert_eq!(shefa.rows.len(), 1, "epr_shefa row must be present");

    let qahal = engine
        .run_script(
            r#"?[reach] := *epr_qahal{cid: $cid, reach}"#,
            &[("cid", cozo::DataValue::from(cid))],
        )
        .unwrap();
    assert_eq!(qahal.rows.len(), 1, "epr_qahal row must be present");
}

// ---------------------------------------------------------------------------
// Task 10: edge projection tolerates missing target
// ---------------------------------------------------------------------------

#[test]
fn project_head_writes_edges_even_when_target_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    let mut head = sample_head();
    // target CID "bafyFuture" is not in epr_node — must still succeed
    head.relationships = vec![EprRelationship {
        rel_type: "PREREQUISITE".into(),
        target: "future-concept".into(),
        target_cid: Some("bafyFuture".into()),
    }];
    projector.project_head("bafyN2", &head).unwrap();

    let edges = engine
        .run_script(
            r#"?[to] := *epr_edge{from_cid: 'bafyN2', to_cid: to, rel_type: 'PREREQUISITE'}"#,
            &[],
        )
        .unwrap();
    assert_eq!(edges.rows.len(), 1, "edge present even when target absent in epr_node");
}

// ---------------------------------------------------------------------------
// Task 11: project_supersedence writes SUPERSEDES edge
// ---------------------------------------------------------------------------

#[test]
fn project_supersedence_writes_supersedes_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    let projector = GraphProjector::new(&engine);
    projector.project_supersedence("bafyV1", "bafyV2").unwrap();

    let edges = engine
        .run_script(
            r#"?[to] := *epr_edge{from_cid: 'bafyV1', to_cid: to, rel_type: 'SUPERSEDES'}"#,
            &[],
        )
        .unwrap();
    assert_eq!(edges.rows.len(), 1, "SUPERSEDES edge must be written");
}
