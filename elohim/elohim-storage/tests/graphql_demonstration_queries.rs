//! Task 32: Demonstration queries — end-to-end integration tests.
//!
//! Both `LearningNeighborhood` (lamad) and `HouseholdTopology` (shefa) queries
//! execute against in-memory CozoDB instances with seeded fixtures.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use elohim_storage::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};
use elohim_storage::graph::{
    engine::GraphEngine, projector::GraphProjector, schema::apply_core_schema,
};
use elohim_storage::graphql::server::{build_schema, AppSchema};
use elohim_storage::services::federator::Federator;
use elohim_storage::test_util::test_pool;
use elohim_storage::P2PHandle;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn open_engine() -> Arc<GraphEngine> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("graph.db");
    std::mem::forget(tmp);
    let engine = Arc::new(GraphEngine::open(&path).unwrap());
    apply_core_schema(&engine).unwrap();
    engine
}

fn make_head(slug: &str, title: &str, rels: Vec<EprRelationship>) -> EprHead {
    EprHead {
        version: 1,
        id: slug.into(),
        content: format!("bafy-content-{slug}"),
        lamad: EprLamadContext {
            title: title.into(),
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
        relationships: rels,
        author: Some("did:test:author".into()),
        updated: Some("2026-05-16T00:00:00Z".into()),
    }
}

/// Build a lamad-seeded schema: concept-a is a prerequisite of concept-b.
fn build_lamad_schema() -> AppSchema {
    let engine = open_engine();
    let projector = GraphProjector::new(&engine);

    // Seed concept-a (no prerequisites)
    projector
        .project_head("bafyA", &make_head("concept-a", "Concept A", vec![]))
        .unwrap();

    // Seed concept-b with a PREREQUISITE edge to concept-a
    projector
        .project_head(
            "bafyB",
            &make_head(
                "concept-b",
                "Concept B",
                vec![EprRelationship {
                    rel_type: "PREREQUISITE".into(),
                    target: "concept-a".into(),
                    target_cid: Some("bafyA".into()),
                }],
            ),
        )
        .unwrap();

    build_schema(
        engine,
        test_pool(),
        Arc::new(Federator::new(P2PHandle::for_testing())),
    )
}

/// Build a shefa-seeded schema: matthew and jessica belong to household-h1.
///
/// MEMBER_OF edges are projected from contributor DIDs → household CID.
/// RECIPROCATES_WITH edge: jessica → matthew.
fn build_shefa_schema() -> AppSchema {
    let engine = open_engine();

    // Project MEMBER_OF edges directly via run_script (these are edge-only nodes;
    // contributor DID nodes don't require epr_node entries for edge traversal).
    // GraphProjector::upsert_edge is private; we use project_head with MEMBER_OF rels.
    let projector = GraphProjector::new(&engine);

    // Use project_head with MEMBER_OF relationship to seed membership edges.
    // The "target_cid" carries the household CID.
    projector
        .project_head(
            "did:test:matthew",
            &make_head(
                "contributor-matthew",
                "Matthew",
                vec![EprRelationship {
                    rel_type: "MEMBER_OF".into(),
                    target: "household-h1".into(),
                    target_cid: Some("household-h1".into()),
                }],
            ),
        )
        .unwrap();

    projector
        .project_head(
            "did:test:jessica",
            &make_head(
                "contributor-jessica",
                "Jessica",
                vec![
                    EprRelationship {
                        rel_type: "MEMBER_OF".into(),
                        target: "household-h1".into(),
                        target_cid: Some("household-h1".into()),
                    },
                    EprRelationship {
                        rel_type: "RECIPROCATES_WITH".into(),
                        target: "contributor-matthew".into(),
                        target_cid: Some("did:test:matthew".into()),
                    },
                ],
            ),
        )
        .unwrap();

    build_schema(
        engine,
        test_pool(),
        Arc::new(Federator::new(P2PHandle::for_testing())),
    )
}

// ---------------------------------------------------------------------------
// Task 32a: LearningNeighborhood (lamad)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lamad_learning_neighborhood_query_works() {
    let schema = build_lamad_schema();

    let query = r#"
        query {
            eprHead(cid: "bafyB") {
                cid
                slug
                prerequisites(maxDepth: 3) {
                    cid
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(
        result.errors.is_empty(),
        "LearningNeighborhood query returned errors: {:?}",
        result.errors
    );

    let json = serde_json::to_value(&result.data).unwrap();
    assert_eq!(
        json["eprHead"]["cid"], "bafyB",
        "expected eprHead.cid = bafyB"
    );
    assert_eq!(
        json["eprHead"]["slug"], "concept-b",
        "expected slug resolved from epr_node"
    );

    let prereqs = &json["eprHead"]["prerequisites"];
    let prereq_arr = prereqs.as_array().expect("prerequisites must be an array");
    assert!(
        !prereq_arr.is_empty(),
        "expected at least one prerequisite for bafyB; got empty array"
    );

    // concept-a should appear in the neighbourhood
    let cids: Vec<&str> = prereq_arr
        .iter()
        .filter_map(|p| p["cid"].as_str())
        .collect();
    assert!(
        cids.contains(&"bafyA"),
        "expected bafyA in prerequisites; got: {:?}",
        cids
    );
}

// ---------------------------------------------------------------------------
// Task 32b: HouseholdTopology (shefa)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shefa_household_topology_query_works() {
    let schema = build_shefa_schema();

    // Query: contributor by DID → household → members
    let query = r#"
        query {
            contributor(did: "did:test:matthew") {
                did
                household {
                    cid
                    members {
                        did
                    }
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(
        result.errors.is_empty(),
        "HouseholdTopology query returned errors: {:?}",
        result.errors
    );

    let json = serde_json::to_value(&result.data).unwrap();
    assert_eq!(
        json["contributor"]["did"], "did:test:matthew",
        "contributor DID round-trips"
    );

    // Household should be resolved via MEMBER_OF edge
    let household = &json["contributor"]["household"];
    assert!(
        !household.is_null(),
        "expected household to be non-null for did:test:matthew"
    );
    assert_eq!(
        household["cid"], "household-h1",
        "household CID should be household-h1"
    );

    // Both members should appear (matthew and jessica both have MEMBER_OF → h1)
    let members = household["members"]
        .as_array()
        .expect("members must be array");
    let member_dids: Vec<&str> = members.iter().filter_map(|m| m["did"].as_str()).collect();
    assert!(
        member_dids.contains(&"did:test:matthew"),
        "matthew must appear as member; got: {:?}",
        member_dids
    );
    assert!(
        member_dids.contains(&"did:test:jessica"),
        "jessica must appear as member; got: {:?}",
        member_dids
    );
}

/// Reciprocity inbound — jessica reciprocates with matthew; query matthew's inbound.
#[tokio::test]
async fn shefa_reciprocity_inbound_query_works() {
    let schema = build_shefa_schema();

    let query = r#"
        query {
            contributor(did: "did:test:matthew") {
                did
                reciprocityInbound {
                    from
                    amount
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(
        result.errors.is_empty(),
        "reciprocityInbound query returned errors: {:?}",
        result.errors
    );

    let json = serde_json::to_value(&result.data).unwrap();
    let flows = json["contributor"]["reciprocityInbound"]
        .as_array()
        .expect("reciprocityInbound must be array");

    // jessica seeded a RECIPROCATES_WITH edge → matthew
    let from_dids: Vec<&str> = flows.iter().filter_map(|f| f["from"].as_str()).collect();
    assert!(
        from_dids.contains(&"did:test:jessica"),
        "jessica must appear as inbound reciprocity source; got: {:?}",
        from_dids
    );
}
