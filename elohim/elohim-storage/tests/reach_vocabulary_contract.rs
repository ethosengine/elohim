//! Drift-prevention contract: the canonical Rust Reach enum must serialize to
//! EXACTLY the schema enum values, in order. Source of truth:
//! elohim/sdk/schemas/v1/enums/reach.schema.json (spec: reach-ontology-vocabulary-split-spec §1).

use elohim_epr::Reach;

const ALL: [Reach; 8] = [
    Reach::Private,
    Reach::SelfScope,
    Reach::Intimate,
    Reach::Trusted,
    Reach::Familiar,
    Reach::Community,
    Reach::Public,
    Reach::Commons,
];

#[test]
fn rust_reach_matches_schema_enum_exactly() {
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sdk/schemas/v1/enums/reach.schema.json"
    );
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();
    let schema_values: Vec<String> = schema["enum"]
        .as_array()
        .expect("reach.schema.json must carry an enum array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let rust_values: Vec<String> = ALL
        .iter()
        .map(|r| serde_json::to_value(r).unwrap().as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        rust_values, schema_values,
        "elohim_epr::Reach diverged from reach.schema.json — the schema is the source of record; fix the Rust side (or run the schema-change process, never hand-drift)"
    );
}
