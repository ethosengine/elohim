//! `did:elohim` projection-assembly against the mock identity store — proves the
//! assembly contract per spec §3.4.

use did_bridge::{DidResolver, ElohimResolver};
use did_tests::MockElohimStore;
use did_types::{Did, ServiceEndpoint, VerificationRelationship};

const FLEET_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";
const EXPECTED_MULTIKEY: &str = "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr";

#[tokio::test]
async fn assembles_full_document_from_store() {
    let store = MockElohimStore::populated(FLEET_KEY);
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let result = resolver.resolve(&did).await.unwrap();
    let doc = result.did_document.unwrap();

    // Subject.
    assert_eq!(doc.id, did);

    // Verification method derived from the agent key (Multikey).
    let vm = &doc.verification_method.as_ref().unwrap()[0];
    assert_eq!(vm.type_, "Multikey");
    assert_eq!(vm.public_key_multibase.as_deref(), Some(EXPECTED_MULTIKEY));
    assert_eq!(vm.controller, did);
    assert!(vm.id.as_str().ends_with(&format!("#{EXPECTED_MULTIKEY}")));

    // §3.4: authentication + assertionMethod reference the key; no keyAgreement.
    assert!(matches!(
        doc.authentication.as_ref().unwrap()[0],
        VerificationRelationship::Reference(_)
    ));
    assert!(matches!(
        doc.assertion_method.as_ref().unwrap()[0],
        VerificationRelationship::Reference(_)
    ));
    assert!(doc.key_agreement.is_none());

    // Transport ids as alsoKnownAs.
    let aka = doc.also_known_as.as_ref().unwrap();
    assert!(aka.contains(&"12D3KooWABCDEexamplePeerId".to_string()));
    assert!(aka.contains(&"iroh:nodeidexample0000".to_string()));

    // Services: profile + doorway, framed with the subject DID.
    let services = doc.service.as_ref().unwrap();
    assert_eq!(services.len(), 2);
    let profile = services
        .iter()
        .find(|s| s.type_ == "ProfileService")
        .unwrap();
    assert!(profile.id.as_str().ends_with("#profile"));
    assert!(matches!(profile.service_endpoint, ServiceEndpoint::Uri(_)));
    assert!(services.iter().any(|s| s.type_ == "DoorwayService"));

    // Controller left implicit in phase 1 (subject controls).
    assert!(doc.controller.is_none());

    // Assembled document carries the DID 1.1 context.
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["@context"][0], "https://www.w3.org/ns/did/v1.1");
}

#[tokio::test]
async fn minimal_agent_assembles_without_optional_sections() {
    // Agent exists but has no profile/doorway/transport projections.
    let mut store = MockElohimStore::default();
    store.agents.insert(FLEET_KEY.to_string());
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let doc = resolver.resolve(&did).await.unwrap().did_document.unwrap();
    assert!(doc.verification_method.is_some());
    assert!(doc.also_known_as.is_none());
    assert!(doc.service.is_none());
}

#[tokio::test]
async fn malformed_agent_cid_yields_invalid_did() {
    let store = MockElohimStore::default();
    let resolver = ElohimResolver::new(store);
    // Valid DID syntax, but the method-specific-id is not a valid agent_cid.
    let did = Did::parse("did:elohim:notanagentcid").unwrap();
    let err = resolver.resolve(&did).await.unwrap_err();
    assert_eq!(err.error_code(), "invalidDid");
}
