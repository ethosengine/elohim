#![cfg(feature = "graph-native")]

use elohim_storage::graph::engine::GraphEngine;

#[test]
fn engine_initializes_with_sled_backend() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("graph.db");
    let engine = GraphEngine::open(&path).expect("engine opens");
    let result = engine
        .run_script("?[a] := a = 1", &[])
        .expect("trivial query runs");
    assert_eq!(result.rows.len(), 1);
}

#[test]
fn epr_node_relation_created_and_upserts_by_cid() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // Validity columns require [int, bool] tuple format — plain integers do not coerce
    engine
        .run_script(
            r#"?[cid, slug, content_cid, version, author_did, updated_at] <- [['bafyreitest1', 'test-1', 'bafycontent', 1, 'did:test', [1700000000, true]]]
           :put epr_node { cid => slug, content_cid, version, author_did, updated_at }"#,
            &[],
        )
        .unwrap();

    let out = engine
        .run_script(
            r#"?[slug] := *epr_node{cid: 'bafyreitest1', slug}"#,
            &[],
        )
        .unwrap();
    assert_eq!(out.rows.len(), 1);
}

#[test]
fn epr_edge_upserts_and_tolerates_missing_target() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // Write edge whose target doesn't exist in epr_node — must succeed (no FK in CozoDB)
    // Validity column asserted_at requires [int, bool] tuple
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [['bafyA', 'bafyB', 'PREREQUISITE', [1700000000, true]]]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let out = engine
        .run_script(
            r#"?[to_cid] := *epr_edge{from_cid: 'bafyA', to_cid, rel_type: 'PREREQUISITE'}"#,
            &[],
        )
        .unwrap();
    assert_eq!(out.rows.len(), 1);
}

#[test]
fn three_pillar_relations_created_and_independently_upsertable() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    engine
        .run_script(
            r#"?[cid, title, content_type, description, content_format, tags] <-
            [['bafyL', 'Sample Concept', 'concept', null, null, []]]
           :put epr_lamad { cid => title, content_type, description, content_format, tags }"#,
            &[],
        )
        .unwrap();

    engine
        .run_script(
            r#"?[cid, stewards, allocations] <- [['bafyL', [], []]]
           :put epr_shefa { cid => stewards, allocations }"#,
            &[],
        )
        .unwrap();

    engine
        .run_script(
            r#"?[cid, reach, layer, attestation_requirements] <- [['bafyL', 'commons', null, []]]
           :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
            &[],
        )
        .unwrap();

    let out = engine
        .run_script(
            r#"?[title, reach] := *epr_lamad{cid: 'bafyL', title}, *epr_qahal{cid: 'bafyL', reach}"#,
            &[],
        )
        .unwrap();
    assert_eq!(out.rows.len(), 1);
}

#[test]
fn epr_node_has_embedding_slot_for_future_hnsw() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // embedding slot accepts null; Validity column requires [int, bool] tuple
    engine
        .run_script(
            r#"?[cid, slug, content_cid, version, author_did, updated_at, embedding] <-
            [['bafyE', 'embed-test', 'bafycontent', 1, null, [1700000000, true], null]]
           :put epr_node { cid => slug, content_cid, version, author_did, updated_at, embedding }"#,
            &[],
        )
        .unwrap();

    let out = engine
        .run_script(
            r#"?[slug] := *epr_node{cid: 'bafyE', slug}"#,
            &[],
        )
        .unwrap();
    assert_eq!(out.rows.len(), 1);
}
