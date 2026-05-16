//! Task 30: manifest-driven SDL codegen — content and structure assertions.

#![cfg(feature = "graph-native")]

use elohim_storage::graphql::codegen::generate_sdl_from_manifests;

#[test]
fn codegen_emits_core_types_and_lamad_subgraph() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    assert!(sdl.contains("type EprHead"), "missing EprHead type");
    assert!(
        sdl.contains("@key(fields: \"cid\")"),
        "missing EprHead @key directive"
    );
    assert!(sdl.contains("type LamadContext"), "missing LamadContext type");
    assert!(
        sdl.contains("prerequisites"),
        "lamad rule not exposed as field"
    );
    assert!(sdl.contains("household"), "shefa household field missing");
}

#[test]
fn codegen_with_only_lamad_omits_shefa_types() {
    let sdl = generate_sdl_from_manifests(&["lamad"]);
    assert!(sdl.contains("prerequisites"), "lamad field must be present");
    assert!(
        !sdl.contains("type Household"),
        "Household must not appear without shefa manifest"
    );
}

#[test]
fn codegen_with_only_shefa_omits_lamad_fields() {
    let sdl = generate_sdl_from_manifests(&["shefa"]);
    assert!(sdl.contains("type Household"), "Household type must be present");
    assert!(
        !sdl.contains("prerequisites"),
        "prerequisites must not appear without lamad manifest"
    );
}

#[test]
fn codegen_unknown_manifest_is_ignored() {
    // Must not panic; unknown manifest names produce no output.
    let sdl = generate_sdl_from_manifests(&["lamad", "unknown-domain"]);
    assert!(
        sdl.contains("type EprHead"),
        "core types must still be present"
    );
}
