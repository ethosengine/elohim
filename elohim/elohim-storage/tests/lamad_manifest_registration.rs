//! Phase 4 T19 — lamad manifest graph extension registration tests.
//!
//! Validates that the lamad manifest's "graph" section parses correctly into
//! a `GraphExtension`, declares the required edges and rules, and can be
//! applied to a live `GraphEngine` without error.
//!
//! Also runs minimal CozoDB sanity queries against each rule to catch
//! Datalog rule-body syntax errors before Phase 5's view builders depend
//! on them.
#![cfg(feature = "graph-native")]

use elohim_storage::graph::{
    engine::GraphEngine,
    registry::{apply_graph_extension, GraphExtension},
    schema::apply_core_schema,
};

fn open_engine() -> (GraphEngine, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    (engine, tmp)
}

fn load_lamad_graph() -> GraphExtension {
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/domains/lamad/manifest.json");
    let manifest_json = std::fs::read_to_string(&manifest_path).expect("read lamad manifest.json");
    let parsed: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    serde_json::from_value(parsed["graph"].clone()).expect("lamad graph section must parse")
}

#[test]
fn lamad_manifest_registers_with_prerequisite_chain_rule() {
    let graph = load_lamad_graph();

    // Required edges
    assert!(
        graph.edges.iter().any(|e| e.rel_type == "PREREQUISITE"),
        "lamad graph must declare PREREQUISITE edge"
    );
    assert!(
        graph.edges.iter().any(|e| e.rel_type == "TEACHES"),
        "lamad graph must declare TEACHES edge"
    );
    assert!(
        graph.edges.iter().any(|e| e.rel_type == "MASTERY_OF"),
        "lamad graph must declare MASTERY_OF edge"
    );

    // Required rules
    assert!(
        graph.rules.iter().any(|r| r.name == "prerequisite_chain"),
        "lamad graph must declare prerequisite_chain rule"
    );
    assert!(
        graph.rules.iter().any(|r| r.name == "mastery_frontier"),
        "lamad graph must declare mastery_frontier rule"
    );

    // Apply to a live engine
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "lamad", &graph).expect("apply lamad graph extension");
}

#[test]
fn prerequisite_chain_rule_body_syntax_valid() {
    let graph = load_lamad_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "lamad", &graph).unwrap();

    // Seed a PREREQUISITE edge: A -> B
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [['prereq-A','prereq-B','PREREQUISITE',[1700000000, true]]]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let prereq_rule = graph
        .rules
        .iter()
        .find(|r| r.name == "prerequisite_chain")
        .unwrap();

    // Run the rule body — any valid response (including 0 rows) means syntax is OK
    let script = format!(
        "{}\n?[ancestor, node, depth] := prerequisite_chain[ancestor, node, depth], depth <= 2",
        prereq_rule.datalog
    );
    let res = engine.run_script(&script, &[("max_depth", cozo::DataValue::from(2_i64))]);
    assert!(
        res.is_ok(),
        "prerequisite_chain rule body has Datalog syntax error: {:?}",
        res.err()
    );
    // Should find A → B with depth=1
    let rows = res.unwrap().rows;
    assert_eq!(rows.len(), 1, "expected 1 prerequisite row, got: {rows:?}");
}

#[test]
fn mastery_frontier_rule_body_syntax_valid() {
    let graph = load_lamad_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "lamad", &graph).unwrap();

    // Seed: contributor mastered concept-A; concept-A is prereq for concept-B
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
                ['contrib-1','concept-A','MASTERY_OF',[1700000000, true]],
                ['concept-A','concept-B','PREREQUISITE',[1700000000, true]]
               ]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let frontier_rule = graph
        .rules
        .iter()
        .find(|r| r.name == "mastery_frontier")
        .unwrap();

    let script = format!(
        "{}\n?[concept] := mastery_frontier[concept]",
        frontier_rule.datalog
    );
    let res = engine.run_script(
        &script,
        &[("contributor", cozo::DataValue::from("contrib-1"))],
    );
    assert!(
        res.is_ok(),
        "mastery_frontier rule body has Datalog syntax error: {:?}",
        res.err()
    );
    // concept-B should be in the frontier (prereq mastered, target not yet mastered)
    let rows = res.unwrap().rows;
    assert_eq!(rows.len(), 1, "expected 1 frontier concept, got: {rows:?}");
}

#[test]
fn lamad_graph_apply_is_idempotent() {
    let graph = load_lamad_graph();
    let (engine, _tmp) = open_engine();

    apply_graph_extension(&engine, "lamad", &graph).expect("first apply");
    apply_graph_extension(&engine, "lamad", &graph).expect("second apply is idempotent");
}
