//! Integration tests for the manifest graph extension validator and applicator
//! (Tasks 16-17, 2026-05-16 graph-native-projection-substrate spec).
//!
//! All tests require the `graph-native` feature.
#![cfg(feature = "graph-native")]

use elohim_storage::graph::registry::{
    apply_graph_extension, validate_graph_extension, EdgeDecl, GraphExtension, IndexDecl, NodeDecl,
    RuleDecl,
};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn prereq_edge() -> EdgeDecl {
    EdgeDecl {
        rel_type: "PREREQUISITE".into(),
        from: "EprHead".into(),
        to: "EprHead".into(),
        indexed: true,
        directional: true,
        weighted: false,
        temporal: false,
        description: None,
    }
}

fn prereq_rule() -> RuleDecl {
    RuleDecl {
        name: "prerequisite_chain".into(),
        datalog: "prerequisite_chain[to, hops] := *epr_edge{from_cid: $start, to_cid: to, rel_type: 'PREREQUISITE'}, hops = 1".into(),
        description: None,
    }
}

fn valid_extension() -> GraphExtension {
    GraphExtension {
        edges: vec![prereq_edge()],
        nodes: vec![],
        indexes: vec![],
        rules: vec![prereq_rule()],
    }
}

// ---------------------------------------------------------------------------
// Task 16: validate_graph_extension unit tests
// ---------------------------------------------------------------------------

#[test]
fn rejects_shadowing_of_core_primitives() {
    let ext = GraphExtension {
        edges: vec![],
        nodes: vec![],
        indexes: vec![],
        rules: vec![RuleDecl {
            name: "neighborhood".into(),
            datalog: "...".into(),
            description: None,
        }],
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
    let msg = format!("{:?}", result);
    assert!(
        msg.contains("shadow") || msg.contains("Shadow"),
        "error should mention shadow: {msg}"
    );
}

#[test]
fn rejects_shadowing_reach_filtered() {
    let ext = GraphExtension {
        rules: vec![RuleDecl {
            name: "reach_filtered".into(),
            datalog: "reach_filtered[x] := *epr_node{cid: x}".into(),
            description: None,
        }],
        ..Default::default()
    };
    assert!(validate_graph_extension(&ext).is_err());
}

#[test]
fn rejects_shadowing_version_chain() {
    let ext = GraphExtension {
        rules: vec![RuleDecl {
            name: "version_chain".into(),
            datalog: "version_chain[x] := *epr_node{cid: x}".into(),
            description: None,
        }],
        ..Default::default()
    };
    assert!(validate_graph_extension(&ext).is_err());
}

#[test]
fn rejects_shadowing_path() {
    let ext = GraphExtension {
        rules: vec![RuleDecl {
            name: "path".into(),
            datalog: "path[x] := *epr_node{cid: x}".into(),
            description: None,
        }],
        ..Default::default()
    };
    assert!(validate_graph_extension(&ext).is_err());
}

#[test]
fn rejects_duplicate_edge_types_within_manifest() {
    let ext = GraphExtension {
        edges: vec![prereq_edge(), prereq_edge()],
        nodes: vec![],
        indexes: vec![],
        rules: vec![],
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
}

#[test]
fn rejects_duplicate_rule_names_within_manifest() {
    let ext = GraphExtension {
        edges: vec![],
        nodes: vec![],
        indexes: vec![],
        rules: vec![prereq_rule(), prereq_rule()],
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
}

#[test]
fn rejects_edge_with_undeclared_node_type() {
    let ext = GraphExtension {
        edges: vec![EdgeDecl {
            rel_type: "FOO".into(),
            from: "UnknownType".into(),
            to: "EprHead".into(),
            indexed: false,
            directional: true,
            weighted: false,
            temporal: false,
            description: None,
        }],
        ..Default::default()
    };
    let result = validate_graph_extension(&ext);
    assert!(result.is_err());
}

#[test]
fn accepts_valid_extension() {
    let ext = valid_extension();
    assert!(validate_graph_extension(&ext).is_ok());
}

#[test]
fn accepts_empty_extension() {
    let ext = GraphExtension::default();
    assert!(validate_graph_extension(&ext).is_ok());
}

#[test]
fn accepts_edge_with_domain_declared_node_type() {
    let ext = GraphExtension {
        edges: vec![EdgeDecl {
            rel_type: "SEQUENCE".into(),
            from: "LearningPath".into(),
            to: "EprHead".into(),
            indexed: false,
            directional: true,
            weighted: false,
            temporal: false,
            description: None,
        }],
        nodes: vec![NodeDecl {
            node_type: "LearningPath".into(),
            properties: [("step_count".to_string(), "Int".to_string())]
                .into_iter()
                .collect(),
        }],
        indexes: vec![],
        rules: vec![],
    };
    assert!(validate_graph_extension(&ext).is_ok());
}

// ---------------------------------------------------------------------------
// Task 17: apply_graph_extension integration tests
// ---------------------------------------------------------------------------

#[test]
fn apply_graph_extension_rejects_invalid_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let engine =
        elohim_storage::graph::engine::GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    // Shadow a core primitive — should fail at validate stage before touching engine
    let ext = GraphExtension {
        rules: vec![RuleDecl {
            name: "neighborhood".into(),
            datalog: "neighborhood[x] := *epr_node{cid: x}".into(),
            description: None,
        }],
        ..Default::default()
    };
    let result = apply_graph_extension(&engine, "test-domain", &ext);
    assert!(result.is_err());
}

#[test]
fn apply_graph_extension_creates_declared_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    let engine =
        elohim_storage::graph::engine::GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    let ext = GraphExtension {
        edges: vec![EdgeDecl {
            rel_type: "PREREQUISITE".into(),
            from: "EprHead".into(),
            to: "EprHead".into(),
            indexed: true,
            directional: true,
            weighted: false,
            temporal: false,
            description: None,
        }],
        nodes: vec![],
        indexes: vec![IndexDecl {
            name: "prereq_forward".into(),
            on: "epr_edge".into(),
            where_clause: Some("rel_type = 'PREREQUISITE'".into()),
            order_by: None,
        }],
        rules: vec![],
    };
    apply_graph_extension(&engine, "lamad", &ext).expect("apply succeeds");

    let indexes = engine.run_script("::indices epr_edge", &[]).unwrap();
    let names: Vec<String> = indexes
        .rows
        .iter()
        .filter_map(|r| {
            r.first().and_then(|v| match v {
                cozo::DataValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
        })
        .collect();
    assert!(
        names.iter().any(|n| n.contains("prereq_forward")),
        "expected prereq_forward index, got: {names:?}"
    );
}

#[test]
fn apply_graph_extension_creates_domain_node_relation() {
    let tmp = tempfile::tempdir().unwrap();
    let engine =
        elohim_storage::graph::engine::GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    let ext = GraphExtension {
        edges: vec![],
        nodes: vec![NodeDecl {
            node_type: "LearningPath".into(),
            properties: [("step_count".to_string(), "Int".to_string())]
                .into_iter()
                .collect(),
        }],
        indexes: vec![],
        rules: vec![],
    };
    apply_graph_extension(&engine, "lamad", &ext).expect("apply succeeds");

    // Can now insert into the created relation
    let insert_result = engine.run_script(
        "?[cid, step_count] <- [['bafytest', 3]] :put lamad_learningpath { cid => step_count }",
        &[],
    );
    assert!(
        insert_result.is_ok(),
        "insert into domain relation should succeed"
    );
}

#[test]
fn apply_graph_extension_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let engine =
        elohim_storage::graph::engine::GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    elohim_storage::graph::schema::apply_core_schema(&engine).unwrap();

    let ext = GraphExtension {
        edges: vec![prereq_edge()],
        nodes: vec![],
        indexes: vec![IndexDecl {
            name: "prereq_forward".into(),
            on: "epr_edge".into(),
            where_clause: None,
            order_by: None,
        }],
        rules: vec![prereq_rule()],
    };

    // Apply twice — must not error
    apply_graph_extension(&engine, "lamad", &ext).expect("first apply");
    apply_graph_extension(&engine, "lamad", &ext).expect("second apply is idempotent");
}
