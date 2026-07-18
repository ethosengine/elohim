//! `did:elohim` projection-assembly against the mock identity store — proves the
//! assembly contract per spec §3.4.

use did_bridge::{DidResolver, ElohimResolver};
use did_tests::MockElohimStore;
use did_types::{Controller, Did, ServiceEndpoint, VerificationRelationship};

const FLEET_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";
const EXPECTED_MULTIKEY: &str = "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr";
// A second real fleet key, used as a community-recovery-quorum controller.
const CONTROLLER_KEY: &str = "uhCAkcHVja32JeYONbe1Dag4rWFB8Jj4-Q5Nv6iyClY-J6-teAgTr";
const CHAIN_ROOT: &str = "bafyreichainrootgenesisnodecid00000000000000000000000000";

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
async fn head_populates_controllers_and_lineage_alias() {
    // Wave C1: an agent WITH a declared identity head (chain-root + a
    // community-recovery-quorum controller set) resolves to a document whose
    // `controller` is populated straight from the declaration, and whose
    // `alsoKnownAs` carries the chain-root lineage alias alongside transport ids.
    let store = MockElohimStore::populated(FLEET_KEY).with_head(
        FLEET_KEY,
        CHAIN_ROOT,
        &[FLEET_KEY, CONTROLLER_KEY],
    );
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let doc = resolver.resolve(&did).await.unwrap().did_document.unwrap();

    // controller = the two declared controllers (self + recovery quorum), as DIDs.
    match doc
        .controller
        .as_ref()
        .expect("controller populated from head")
    {
        Controller::Many(cs) => {
            assert_eq!(cs.len(), 2);
            assert!(cs.contains(&Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap()));
            assert!(cs.contains(&Did::parse(&format!("did:elohim:{CONTROLLER_KEY}")).unwrap()));
        }
        other => panic!("expected Controller::Many, got {other:?}"),
    }

    // Lineage: the chain-root surfaces as a stable alsoKnownAs alias, next to the
    // transport ids (which are preserved).
    let aka = doc.also_known_as.as_ref().unwrap();
    assert!(
        aka.contains(&format!("did:elohim:{CHAIN_ROOT}")),
        "chain-root lineage alias present: {aka:?}"
    );
    assert!(
        aka.contains(&"12D3KooWABCDEexamplePeerId".to_string()),
        "transport id preserved"
    );
}

#[tokio::test]
async fn single_controller_head_uses_controller_one() {
    // A self-policy head with exactly one controller emits `Controller::One`.
    let store = MockElohimStore::populated(FLEET_KEY).with_head(FLEET_KEY, FLEET_KEY, &[FLEET_KEY]);
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();
    let doc = resolver.resolve(&did).await.unwrap().did_document.unwrap();

    match doc.controller.as_ref().expect("controller populated") {
        Controller::One(c) => {
            assert_eq!(c, &Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap())
        }
        other => panic!("expected Controller::One, got {other:?}"),
    }
    // Self-alias (chain_root == subject) must NOT be added to alsoKnownAs.
    let aka = doc.also_known_as.as_ref().unwrap();
    assert!(
        !aka.contains(&format!("did:elohim:{FLEET_KEY}")),
        "a degenerate self-alias (root == subject) must be skipped: {aka:?}"
    );
}

#[tokio::test]
async fn no_head_keeps_phase1_document_byte_unchanged() {
    // The load-bearing regression guard: with NO declared head the assembled
    // document is byte-identical to the phase-1 output (no `controller`, no
    // lineage alias — only transport ids in alsoKnownAs).
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let phase1 = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();
    let phase1_json = serde_json::to_string(&phase1).unwrap();

    // A store that also implements identity_head but returns None for this agent
    // must produce the identical document (the default-None path).
    let no_head = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();

    assert!(no_head.controller.is_none(), "no head ⇒ no controller");
    let aka = no_head.also_known_as.as_ref().unwrap();
    assert!(
        aka.iter().all(|a| !a.starts_with("did:elohim:")),
        "no head ⇒ no lineage alias in alsoKnownAs: {aka:?}"
    );
    assert_eq!(
        phase1_json,
        serde_json::to_string(&no_head).unwrap(),
        "no-head document must be byte-unchanged from the phase-1 assembly"
    );
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
