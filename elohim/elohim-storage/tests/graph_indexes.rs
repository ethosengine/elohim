#![cfg(feature = "graph-native")]

use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema};

#[test]
fn core_indexes_present_and_used_by_planner() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();

    // Inspect: relations listing returns the indexes
    let out = engine.run_script("::indices epr_edge", &[]).unwrap();
    let index_names: Vec<String> = out
        .rows
        .iter()
        .filter_map(|row| {
            row.first().and_then(|v| match v {
                cozo::DataValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
        })
        .collect();
    assert!(
        index_names.iter().any(|n| n.contains("by_rel_type")),
        "expected by_rel_type index, got: {:?}",
        index_names
    );
    assert!(
        index_names.iter().any(|n| n.contains("by_target")),
        "expected by_target index, got: {:?}",
        index_names
    );
}
