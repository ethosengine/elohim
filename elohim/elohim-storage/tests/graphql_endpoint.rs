//! Task 29: GraphQL endpoint reachability — introspection via schema execute.
//!
//! We test the schema directly (without spinning up a hyper server) because
//! the integration test harness does not bind a port. The `handle` function
//! in `graphql::server` is tested end-to-end in `graphql_demonstration_queries`.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema};
use elohim_storage::graphql::server::build_schema;

fn open_seeded_engine() -> Arc<GraphEngine> {
    let tmp = tempfile::tempdir().unwrap();
    // Leak the TempDir so the sled db file isn't deleted while the engine is open.
    let path = tmp.path().join("graph.db");
    std::mem::forget(tmp);
    let engine = Arc::new(GraphEngine::open(&path).unwrap());
    apply_core_schema(&engine).unwrap();
    engine
}

/// POST /api/v1/graphql with `{ "query": "{ __schema { types { name } } }" }`
/// must return a response with __schema.types non-empty.
#[tokio::test]
async fn graphql_endpoint_serves_introspection() {
    let engine = open_seeded_engine();
    let schema = build_schema(engine);

    let result = schema.execute("{ __schema { types { name } } }").await;

    assert!(
        result.errors.is_empty(),
        "introspection returned errors: {:?}",
        result.errors
    );

    let json = serde_json::to_value(&result.data).unwrap();
    let types = &json["__schema"]["types"];
    assert!(
        types.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "__schema.types must be non-empty; got: {}",
        types
    );
}

/// Verify that `eprHead` and `contributor` appear in the introspected type list.
#[tokio::test]
async fn graphql_schema_exposes_epr_head_and_contributor() {
    let engine = open_seeded_engine();
    let schema = build_schema(engine);

    let result = schema.execute("{ __schema { types { name } } }").await;
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let json = serde_json::to_value(&result.data).unwrap();
    let type_names: Vec<String> = json["__schema"]["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str().map(String::from))
        .collect();

    assert!(
        type_names.contains(&"EprHead".to_string()),
        "EprHead not found in schema types; got: {:?}",
        type_names
    );
    assert!(
        type_names.contains(&"Contributor".to_string()),
        "Contributor not found in schema types"
    );
}
