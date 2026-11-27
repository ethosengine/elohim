//! Rust wire vocabularies pinned to their SDK schema sources of truth.

use eprfs_core::ReachReason;

const REACH_REASON_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../sdk/schemas/v1/enums/reach-reason.schema.json"
));

fn schema_values(text: &str) -> Vec<String> {
    let schema: serde_json::Value = serde_json::from_str(text).expect("valid reach-reason schema");
    schema["enum"]
        .as_array()
        .expect("reach-reason schema has an enum array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("reach-reason enum values are strings")
                .to_owned()
        })
        .collect()
}

#[test]
fn reach_reason_schema_equals_every_serialized_rust_variant() {
    let rust_values: Vec<String> = ReachReason::ALL
        .iter()
        .map(|reason| {
            serde_json::to_value(reason)
                .expect("ReachReason serializes")
                .as_str()
                .expect("ReachReason serializes as a string")
                .to_owned()
        })
        .collect();

    assert_eq!(
        schema_values(REACH_REASON_SCHEMA),
        rust_values,
        "reach-reason.schema.json disagrees with eprfs_core::ReachReason"
    );
}
