//! Task 31: Apollo Federation v2 SDL compliance + async-graphql parser validation.

#![cfg(feature = "graph-native")]

use elohim_storage::graphql::codegen::generate_sdl_from_manifests;

#[test]
fn generated_sdl_includes_federation_v2_directives() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    assert!(
        sdl.contains("@link(url:"),
        "SDL must contain @link(url: ...) directive"
    );
    assert!(
        sdl.contains("federation/v2"),
        "SDL must reference federation/v2 spec URL"
    );
    assert!(
        sdl.contains("@key(fields:"),
        "SDL must declare @key(fields: ...) on federated entities"
    );
}

/// Use async-graphql's own parser to validate the SDL is well-formed.
///
/// NOTE: `async_graphql::parser::parse_schema` validates GraphQL SDL syntax.
/// The Federation v2 `@link` and `extend schema` directives are valid GraphQL
/// extension syntax per the June 2018 spec and are accepted by the parser.
#[test]
fn generated_sdl_parses_as_valid_graphql() {
    let sdl = generate_sdl_from_manifests(&["lamad", "shefa"]);
    let parsed = async_graphql::parser::parse_schema(&sdl);
    assert!(
        parsed.is_ok(),
        "SDL parse error: {:?}\n\nSDL was:\n{}",
        parsed.err(),
        sdl
    );
}

/// Validate that core-only SDL (no manifests) is also valid.
#[test]
fn core_only_sdl_parses_as_valid_graphql() {
    let sdl = generate_sdl_from_manifests(&[]);
    let parsed = async_graphql::parser::parse_schema(&sdl);
    assert!(parsed.is_ok(), "Core SDL parse error: {:?}", parsed.err());
}
