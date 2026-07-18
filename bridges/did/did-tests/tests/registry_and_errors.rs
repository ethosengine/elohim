//! Registry routing and standard error-metadata semantics.

use did_bridge::{
    DidKeyResolver, DidResolutionError, MethodRegistry, ERR_INVALID_DID, ERR_METHOD_NOT_SUPPORTED,
    ERR_NOT_FOUND,
};
use did_tests::MockElohimStore;
use did_types::Did;

const FLEET_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";

#[tokio::test]
async fn registry_routes_to_registered_method() {
    let registry = MethodRegistry::new().with(Box::new(DidKeyResolver::new()));
    let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    let result = registry.resolve(&did).await.unwrap();
    assert!(result.did_document.is_some());
    assert_eq!(
        result.did_resolution_metadata.content_type.as_deref(),
        Some("application/did+ld+json")
    );
    assert!(result.did_resolution_metadata.error.is_none());
}

#[tokio::test]
async fn unregistered_method_yields_method_not_supported() {
    let registry = MethodRegistry::new().with(Box::new(DidKeyResolver::new()));
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();
    let err = registry.resolve(&did).await.unwrap_err();
    assert!(matches!(err, DidResolutionError::MethodNotSupported(_)));
    assert_eq!(err.error_code(), ERR_METHOD_NOT_SUPPORTED);

    // The error maps into resolution metadata with the standard code.
    let result = did_bridge::DidResolutionResult::from_error(&err);
    assert_eq!(
        result.did_resolution_metadata.error.as_deref(),
        Some(ERR_METHOD_NOT_SUPPORTED)
    );
    assert!(result.did_document.is_none());
}

#[tokio::test]
async fn invalid_did_string_yields_invalid_did() {
    let registry = MethodRegistry::new().with(Box::new(DidKeyResolver::new()));
    let err = registry.resolve_str("not-a-did").await.unwrap_err();
    assert_eq!(err.error_code(), ERR_INVALID_DID);
}

#[tokio::test]
async fn nonexistent_elohim_agent_yields_not_found() {
    // Store where the agent does NOT exist.
    let registry = MethodRegistry::new().with(Box::new(did_bridge::ElohimResolver::new(
        MockElohimStore::default(),
    )));
    let err = registry
        .resolve_str(&format!("did:elohim:{FLEET_KEY}"))
        .await
        .unwrap_err();
    assert!(matches!(err, DidResolutionError::NotFound(_)));
    assert_eq!(err.error_code(), ERR_NOT_FOUND);
}

#[tokio::test]
async fn registry_reports_registered_methods() {
    let registry = MethodRegistry::new()
        .with(Box::new(DidKeyResolver::new()))
        .with(Box::new(did_bridge::ElohimResolver::new(
            MockElohimStore::default(),
        )));
    let mut methods = registry.methods();
    methods.sort_unstable();
    assert_eq!(methods, vec!["elohim", "key"]);
}
