#![cfg(feature = "graph-native")]

use elohim_storage::graph::{
    engine::GraphEngine, primitives::register_core_primitives, schema::apply_core_schema,
};

fn fixture() -> (GraphEngine, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    register_core_primitives(&engine).unwrap();
    // A → B → C edge chain via TEACHES
    // Validity column asserted_at requires [int, bool] tuple format
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
              ['A','B','TEACHES',[1700000000, true]],
              ['B','C','TEACHES',[1700000000, true]]
           ]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();
    (engine, tmp)
}

#[test]
fn neighborhood_walks_at_max_depth_2() {
    let (engine, _tmp) = fixture();
    let script = format!(
        "{}\n?[to, hops] := neighborhood[to, hops], hops <= 2",
        elohim_storage::graph::primitives::scripts::NEIGHBORHOOD
    );
    let out = engine
        .run_script(
            &script,
            &[
                ("start", cozo::DataValue::from("A")),
                ("max_hops", cozo::DataValue::from(2_i64)),
            ],
        )
        .unwrap();
    // Expect: B at hops=1, C at hops=2 → 2 rows
    assert_eq!(
        out.rows.len(),
        2,
        "expected 2 neighbours, got: {:?}",
        out.rows
    );
}

#[test]
fn version_chain_walks_supersedes_edges() {
    let (engine, _tmp) = fixture();
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [['v1','v2','SUPERSEDES',[1700000000, true]]]
           :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();
    let script = format!(
        "{}\n?[node] := version_chain[node]",
        elohim_storage::graph::primitives::scripts::VERSION_CHAIN
    );
    let out = engine
        .run_script(&script, &[("start", cozo::DataValue::from("v1"))])
        .unwrap();
    assert_eq!(
        out.rows.len(),
        1,
        "expected v2 as successor, got: {:?}",
        out.rows
    );
}
