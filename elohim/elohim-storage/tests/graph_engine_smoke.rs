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
