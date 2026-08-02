//! `did:elohim` projection-assembly against the mock identity store — proves the
//! assembly contract per spec §3.4.

use did_bridge::{DidResolutionError, DidResolver, ElohimResolver};
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
async fn single_controller_head_uses_controller_one_and_skips_equal_root_alias() {
    // Exercises TWO branches with one head whose chain_root is deliberately set
    // EQUAL to the subject key: (1) a single controller emits `Controller::One`;
    // (2) the lineage-alias skip fires when `chain_root == subject`.
    //
    // NOTE (Wave A1 dependency): whether a real UN-rotated identity actually has
    // `chain_root == head_key` depends on A1's `identity_root_cid` derivation. If
    // A1 derives a DISTINCT root cid for the degenerate single-node chain, an
    // un-rotated identity would EMIT the alias (the skip only fires on literal
    // equality). This test asserts the skip *branch*, not a claim about A1's
    // output — the root is pinned == subject here to reach that branch.
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
async fn no_head_document_is_self_consistent_and_controller_free() {
    // A SELF-CONSISTENCY + shape check (not a historical-bytes regression guard):
    // two calls to the SAME current resolver over a no-head store must agree, and
    // the no-head document must carry no `controller` and no lineage alias — only
    // transport ids in alsoKnownAs. `MockElohimStore` implements identity_head but
    // answers `NeverDeclared` for this agent, so this exercises the
    // never-declared branch specifically (revoked and unresolvable have their own
    // tests, and the three-way distinction is asserted in
    // `never_declared_revoked_and_unresolvable_are_three_distinct_outcomes`).
    // (The load-bearing phase-1 shape guard is `assembles_full_document_from_store`.)
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let a = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();
    let b = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();

    assert!(b.controller.is_none(), "no head ⇒ no controller");
    let aka = b.also_known_as.as_ref().unwrap();
    assert!(
        aka.iter().all(|a| !a.starts_with("did:elohim:")),
        "no head ⇒ no lineage alias in alsoKnownAs: {aka:?}"
    );
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap(),
        "the no-head assembly is deterministic (self-consistent) across calls"
    );
}

#[tokio::test]
async fn revoked_head_deactivates_the_document_and_confers_no_authority() {
    // Forecast row 8: a revoked identity used to resolve to an ordinary,
    // fully-armed, implicitly self-controlled document. It now resolves to a
    // DEACTIVATED document (DID 1.1 §7.1.2) carrying no key, no relationship,
    // and no endpoint — even though the store still has a profile, a doorway and
    // transport ids for this agent.
    let store = MockElohimStore::populated(FLEET_KEY).with_revoked_head(
        FLEET_KEY,
        CHAIN_ROOT,
        Some(CONTROLLER_KEY),
    );
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let result = resolver.resolve(&did).await.unwrap();
    assert_eq!(
        result.did_document_metadata.deactivated,
        Some(true),
        "revocation must be stated, not inferred"
    );
    let doc = result
        .did_document
        .expect("deactivation resolves, not 404s");

    assert!(doc.verification_method.is_none(), "no key to act with");
    assert!(doc.authentication.is_none());
    assert!(doc.assertion_method.is_none());
    assert!(
        doc.service.is_none(),
        "a revoked identity's endpoints must not be projected as live"
    );
    let aka = doc.also_known_as.as_ref().expect("lineage preserved");
    assert!(
        !aka.contains(&"12D3KooWABCDEexamplePeerId".to_string()),
        "transport ids must not be served for a revoked identity: {aka:?}"
    );
    assert!(aka.contains(&format!("did:elohim:{CHAIN_ROOT}")), "{aka:?}");
    assert!(
        aka.contains(&format!("did:elohim:{CONTROLLER_KEY}")),
        "successor named by the revocation is followable: {aka:?}"
    );

    // The whole point, stated once more: the deactivated document is NOT the
    // never-declared document with a flag on it.
    let never = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .unwrap();
    assert_eq!(never.did_document_metadata.deactivated, None);
    assert!(never.did_document.unwrap().verification_method.is_some());
}

#[tokio::test]
async fn unresolvable_head_fails_closed_instead_of_defaulting_to_self_control() {
    // The third provenance the old `Option` had nowhere to put: the store looked
    // and could not find out. Serving the implicit-self document here would
    // assert self-control nobody verified.
    let store = MockElohimStore::populated(FLEET_KEY)
        .with_unresolvable_head(FLEET_KEY, "binds-identity record not yet gossiped");
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let err = resolver.resolve(&did).await.unwrap_err();
    assert!(
        matches!(err, DidResolutionError::IdentityHeadUnresolvable(_)),
        "expected IdentityHeadUnresolvable, got {err:?}"
    );
    // Distinct from "this agent has no head" AND from "this agent does not
    // exist" — three answers, three outcomes.
    assert_ne!(err.error_code(), "notFound");
    assert!(did_bridge::DidResolutionResult::from_error(&err)
        .did_document
        .is_none());
}

#[tokio::test]
async fn never_declared_revoked_and_unresolvable_are_three_distinct_outcomes() {
    // One assertion for the whole C4 split: the same agent, three store answers,
    // three materially different resolutions. Under the previous
    // `Option<IdentityHead>` shape the first two were byte-identical and the
    // third was indistinguishable from the first.
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let never = ElohimResolver::new(MockElohimStore::populated(FLEET_KEY))
        .resolve(&did)
        .await
        .expect("never-declared resolves");
    let revoked = ElohimResolver::new(
        MockElohimStore::populated(FLEET_KEY).with_revoked_head(FLEET_KEY, CHAIN_ROOT, None),
    )
    .resolve(&did)
    .await
    .expect("revoked resolves (deactivated)");
    let unresolvable = ElohimResolver::new(
        MockElohimStore::populated(FLEET_KEY).with_unresolvable_head(FLEET_KEY, "read timed out"),
    )
    .resolve(&did)
    .await;

    assert!(
        unresolvable.is_err(),
        "unresolvable must not serve a document"
    );
    assert_ne!(never, revoked, "never-declared and revoked must not agree");
    assert!(never.did_document_metadata.deactivated.is_none());
    assert_eq!(revoked.did_document_metadata.deactivated, Some(true));
}

#[tokio::test]
async fn declared_head_with_empty_controller_set_is_refused() {
    // A declared-but-empty controller set would serialize to a document with no
    // `controller`, i.e. implicit self-control — a stronger claim than the
    // declaration made. Refused, not silently promoted.
    let store = MockElohimStore::populated(FLEET_KEY).with_head(FLEET_KEY, CHAIN_ROOT, &[]);
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{FLEET_KEY}")).unwrap();

    let err = resolver.resolve(&did).await.unwrap_err();
    assert!(
        matches!(err, DidResolutionError::IdentityHeadMalformed(_)),
        "expected IdentityHeadMalformed, got {err:?}"
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
