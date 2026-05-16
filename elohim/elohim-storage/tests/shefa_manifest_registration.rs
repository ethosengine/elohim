//! Phase 4 T20 — shefa manifest graph extension registration tests.
//!
//! Validates that the shefa manifest's "graph" section parses correctly into
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

fn load_shefa_graph() -> GraphExtension {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sdk/domains/shefa/manifest.json");
    let manifest_json =
        std::fs::read_to_string(&manifest_path).expect("read shefa manifest.json");
    let parsed: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    serde_json::from_value(parsed["graph"].clone()).expect("shefa graph section must parse")
}

#[test]
fn shefa_manifest_registers_topology_rules() {
    let graph = load_shefa_graph();

    // Required edges
    let edge_types: Vec<&str> = graph.edges.iter().map(|e| e.rel_type.as_str()).collect();
    assert!(
        edge_types.contains(&"STEWARDS"),
        "shefa graph must declare STEWARDS edge"
    );
    assert!(
        edge_types.contains(&"VALUE_FLOW"),
        "shefa graph must declare VALUE_FLOW edge"
    );
    assert!(
        edge_types.contains(&"MEMBER_OF"),
        "shefa graph must declare MEMBER_OF edge"
    );
    assert!(
        edge_types.contains(&"RECIPROCATES_WITH"),
        "shefa graph must declare RECIPROCATES_WITH edge"
    );
    assert!(
        edge_types.contains(&"OPERATES_DEVICE"),
        "shefa graph must declare OPERATES_DEVICE edge"
    );

    // Required rules
    let rule_names: Vec<&str> = graph.rules.iter().map(|r| r.name.as_str()).collect();
    assert!(
        rule_names.contains(&"household_topology"),
        "shefa graph must declare household_topology rule"
    );
    assert!(
        rule_names.contains(&"collective_topology"),
        "shefa graph must declare collective_topology rule"
    );
    assert!(
        rule_names.contains(&"reciprocity_flow_to"),
        "shefa graph must declare reciprocity_flow_to rule"
    );
    assert!(
        rule_names.contains(&"value_flow_chain"),
        "shefa graph must declare value_flow_chain rule"
    );

    // Apply to a live engine
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "shefa", &graph).expect("apply shefa graph extension");
}

#[test]
fn household_topology_rule_body_syntax_valid() {
    let graph = load_shefa_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "shefa", &graph).unwrap();

    // Seed: member-1 is in household-1, member-1 operates device-1
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
                ['member-1','household-1','MEMBER_OF',[1700000000, true]],
                ['member-1','device-1','OPERATES_DEVICE',[1700000000, true]]
               ]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let rule = graph
        .rules
        .iter()
        .find(|r| r.name == "household_topology")
        .unwrap();

    let script = format!(
        "{}\n?[member, device] := household_topology[member, device]",
        rule.datalog
    );
    let res = engine.run_script(
        &script,
        &[("household", cozo::DataValue::from("household-1"))],
    );
    assert!(
        res.is_ok(),
        "household_topology rule body has Datalog syntax error: {:?}",
        res.err()
    );
    let rows = res.unwrap().rows;
    assert_eq!(
        rows.len(),
        1,
        "expected 1 household topology row, got: {rows:?}"
    );
}

#[test]
fn collective_topology_rule_body_syntax_valid() {
    let graph = load_shefa_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "shefa", &graph).unwrap();

    // Seed: two members in collective-1
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
                ['member-A','collective-1','MEMBER_OF',[1700000000, true]],
                ['member-B','collective-1','MEMBER_OF',[1700000000, true]]
               ]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let rule = graph
        .rules
        .iter()
        .find(|r| r.name == "collective_topology")
        .unwrap();

    let script = format!(
        "{}\n?[member] := collective_topology[member]",
        rule.datalog
    );
    let res = engine.run_script(
        &script,
        &[("collective", cozo::DataValue::from("collective-1"))],
    );
    assert!(
        res.is_ok(),
        "collective_topology rule body has Datalog syntax error: {:?}",
        res.err()
    );
    let rows = res.unwrap().rows;
    assert_eq!(rows.len(), 2, "expected 2 collective members, got: {rows:?}");
}

#[test]
fn reciprocity_flow_to_rule_body_syntax_valid() {
    let graph = load_shefa_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "shefa", &graph).unwrap();

    // Seed: two contributors reciprocate toward contrib-target
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
                ['contrib-X','contrib-target','RECIPROCATES_WITH',[1700000000, true]],
                ['contrib-Y','contrib-target','RECIPROCATES_WITH',[1700000000, true]]
               ]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let rule = graph
        .rules
        .iter()
        .find(|r| r.name == "reciprocity_flow_to")
        .unwrap();

    let script = format!(
        "{}\n?[from] := reciprocity_flow_to[from]",
        rule.datalog
    );
    let res = engine.run_script(
        &script,
        &[("contributor", cozo::DataValue::from("contrib-target"))],
    );
    assert!(
        res.is_ok(),
        "reciprocity_flow_to rule body has Datalog syntax error: {:?}",
        res.err()
    );
    let rows = res.unwrap().rows;
    assert_eq!(
        rows.len(),
        2,
        "expected 2 reciprocity flows, got: {rows:?}"
    );
}

#[test]
fn value_flow_chain_rule_body_syntax_valid() {
    let graph = load_shefa_graph();
    let (engine, _tmp) = open_engine();
    apply_graph_extension(&engine, "shefa", &graph).unwrap();

    // Seed: start -> mid -> end via VALUE_FLOW
    engine
        .run_script(
            r#"?[from_cid, to_cid, rel_type, asserted_at] <- [
                ['flow-start','flow-mid','VALUE_FLOW',[1700000000, true]],
                ['flow-mid','flow-end','VALUE_FLOW',[1700000000, true]]
               ]
               :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
            &[],
        )
        .unwrap();

    let rule = graph
        .rules
        .iter()
        .find(|r| r.name == "value_flow_chain")
        .unwrap();

    let script = format!(
        "{}\n?[to, depth] := value_flow_chain[to, depth], depth <= 3",
        rule.datalog
    );
    let res = engine.run_script(
        &script,
        &[
            ("start", cozo::DataValue::from("flow-start")),
            ("max_depth", cozo::DataValue::from(3_i64)),
        ],
    );
    assert!(
        res.is_ok(),
        "value_flow_chain rule body has Datalog syntax error: {:?}",
        res.err()
    );
    let rows = res.unwrap().rows;
    // flow-start -> flow-mid (depth=1), flow-start -> flow-end (depth=2)
    assert_eq!(
        rows.len(),
        2,
        "expected 2 value flow chain nodes, got: {rows:?}"
    );
}

#[test]
fn shefa_graph_apply_is_idempotent() {
    let graph = load_shefa_graph();
    let (engine, _tmp) = open_engine();

    apply_graph_extension(&engine, "shefa", &graph).expect("first apply");
    apply_graph_extension(&engine, "shefa", &graph).expect("second apply is idempotent");
}
