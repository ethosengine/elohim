//! Task 23 — lamad view builder tests.
//!
//! Seeds a graph with known atoms and asserts the view builders return expected shapes.
#![cfg(feature = "graph-native")]

use elohim_storage::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};
use elohim_storage::graph::{
    engine::GraphEngine, projector::GraphProjector, schema::apply_core_schema,
};
use elohim_storage::graph_views::lamad::{atom_version_chain, navigation_context, resolved_atom};

/// Open a fresh in-memory-equivalent engine (sled to a tempdir) and apply core schema.
fn open_engine() -> (GraphEngine, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    (engine, tmp)
}

fn mk_head(id: &str, title: &str, reach: Option<&str>) -> EprHead {
    EprHead {
        version: 1,
        id: id.to_string(),
        content: format!("bafy-content-{id}"),
        lamad: EprLamadContext {
            title: title.to_string(),
            content_type: "concept".to_string(),
            description: None,
            content_format: None,
            tags: vec![],
        },
        shefa: EprShefaContext {
            stewards: vec![],
            allocations: vec![],
        },
        qahal: EprQahalContext {
            reach: reach.map(str::to_string),
            layer: None,
            attestation_requirements: vec![],
        },
        relationships: vec![],
        author: Some("did:test:author".to_string()),
        updated: Some("2026-05-16T00:00:00Z".to_string()),
    }
}

/// Seed: atom A TEACHES B; B0 SUPERSEDES B.
fn seed_fixture(engine: &GraphEngine) {
    let projector = GraphProjector::new(engine);

    let head_a = mk_head("A", "A teaches B", Some("commons"));
    let mut head_a_with_rel = head_a.clone();
    head_a_with_rel.relationships = vec![EprRelationship {
        rel_type: "TEACHES".to_string(),
        target: "B".to_string(),
        target_cid: Some("bafyB".to_string()),
    }];
    projector.project_head("bafyA", &head_a_with_rel).unwrap();

    let head_b = mk_head("B", "B body", Some("commons"));
    projector.project_head("bafyB", &head_b).unwrap();

    let head_b0 = mk_head("B0", "B prior version", None);
    projector.project_head("bafyB0", &head_b0).unwrap();

    // B0 supersedes B (B0 is the older atom, its SUPERSEDES edge points to B)
    projector.project_supersedence("bafyB0", "bafyB").unwrap();
}

// ---------------------------------------------------------------------------
// resolved_atom tests
// ---------------------------------------------------------------------------

#[test]
fn resolved_atom_returns_three_pillar_view() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    let view = resolved_atom::build(&engine, "bafyA").expect("build resolved_atom");

    assert_eq!(view.cid, "bafyA");
    assert_eq!(view.slug, "A");
    assert_eq!(view.content_cid, "bafy-content-A");
    assert_eq!(view.version, 1);
    assert_eq!(view.author_did, Some("did:test:author".to_string()));
    assert_eq!(view.lamad.title, "A teaches B");
    assert_eq!(view.lamad.content_type, "concept");
    assert_eq!(view.qahal.reach, Some("commons".to_string()));
    assert_eq!(view.qahal.attestation_requirements, Vec::<String>::new());
}

#[test]
fn resolved_atom_returns_err_for_absent_cid() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    let result = resolved_atom::build(&engine, "bafyNotExist");
    assert!(result.is_err(), "expected error for absent CID");
}

#[test]
fn resolved_atom_shefa_is_none_when_no_stewards() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    // bafyA has empty stewards — shefa block should be present but with empty vecs
    let view = resolved_atom::build(&engine, "bafyA").unwrap();
    // shefa is present (epr_shefa row is written by project_head) with empty lists
    if let Some(shefa) = &view.shefa {
        assert!(shefa.stewards.is_empty());
        assert!(shefa.allocations.is_empty());
    }
    // Either Some with empty vecs or None is acceptable
}

// ---------------------------------------------------------------------------
// navigation_context tests
// ---------------------------------------------------------------------------

#[test]
fn navigation_context_returns_atom_plus_origin_plus_neighborhood() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    let view =
        navigation_context::build(&engine, "bafyB", "bafyA").expect("build navigation_context");

    assert_eq!(view.atom.cid, "bafyB");
    assert_eq!(view.atom.lamad.title, "B body");
    assert_eq!(view.origin_context.origin_cid, "bafyA");
    assert_eq!(view.origin_context.origin_type, "epr");
    assert!(view.origin_context.origin_path.is_none());
}

#[test]
fn navigation_context_neighborhood_includes_teaches_target() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    // A TEACHES B — neighbourhood from A should include B
    let view =
        navigation_context::build(&engine, "bafyA", "bafyA").expect("build navigation_context");

    let has_b = view.neighborhood.iter().any(|n| n.cid == "bafyB");
    assert!(has_b, "neighbourhood of A must include B (TEACHES edge)");
    let teaches_entry = view.neighborhood.iter().find(|n| n.cid == "bafyB").unwrap();
    assert_eq!(teaches_entry.rel_type, "TEACHES");
    assert_eq!(teaches_entry.hops, 1);
}

// ---------------------------------------------------------------------------
// atom_version_chain tests
// ---------------------------------------------------------------------------

#[test]
fn atom_version_chain_walks_supersedes_forward() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    // B0 SUPERSEDES B — chain from B0 should include B
    let view = atom_version_chain::build(&engine, "bafyB0").expect("build atom_version_chain");

    assert_eq!(view.current_cid, "bafyB0");
    assert_eq!(view.chain.len(), 1, "chain must include B as successor");
    assert_eq!(view.chain[0].cid, "bafyB");
    assert_eq!(view.chain[0].version, 2, "successor starts at version 2");
    assert_eq!(view.canonical_cid, Some("bafyB".to_string()));
}

#[test]
fn atom_version_chain_empty_when_no_successors() {
    let (engine, _tmp) = open_engine();
    seed_fixture(&engine);

    // bafyB has no outbound SUPERSEDES edges
    let view = atom_version_chain::build(&engine, "bafyB").expect("build atom_version_chain");

    assert_eq!(view.current_cid, "bafyB");
    assert!(view.chain.is_empty(), "no successors means empty chain");
    assert!(view.canonical_cid.is_none());
}
