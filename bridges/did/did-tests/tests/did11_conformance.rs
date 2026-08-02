//! DID 1.1 conformance gate.
//!
//! Every DID document this crate can *produce* — plus the wire-fidelity
//! fixtures it must *accept* — is validated against the hand-derived W3C DID 1.1
//! JSON Schema (`bridges/did/schemas/did-document-1.1.schema.json`). This turns
//! the one-off review into a standing gate: the invariants that drifted once at
//! birth (v1.1 context pin, extension-property fidelity) can never drift silent.
//!
//! Run: `cargo test -p did-tests --test did11_conformance`.

use did_bridge::{DidKeyResolver, DidResolver, ElohimResolver};
use did_tests::MockElohimStore;
use did_types::Did;
use serde_json::Value;

/// The hand-derived DID 1.1 schema, embedded at compile time.
const SCHEMA: &str = include_str!("../../schemas/did-document-1.1.schema.json");

const AGENT_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";

fn validator() -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(SCHEMA).expect("schema is valid JSON");
    jsonschema::validator_for(&schema).expect("schema compiles as a draft 2020-12 validator")
}

/// Collect all schema violations for `instance` as human-readable strings.
fn violations(v: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    v.iter_errors(instance)
        .map(|e| format!("{e} (at instance path {})", e.instance_path))
        .collect()
}

/// Assert `instance` conforms; on failure, name what was being validated.
fn assert_conforms(what: &str, instance: &Value) {
    let v = validator();
    let errs = violations(&v, instance);
    assert!(
        errs.is_empty(),
        "DID 1.1 conformance violated for {what}:\n  - {}",
        errs.join("\n  - ")
    );
}

#[tokio::test]
async fn did_key_resolution_output_conforms_to_did11_schema() {
    let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    let doc = DidKeyResolver::new()
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();
    assert_conforms(
        "did:key resolution output",
        &serde_json::to_value(&doc).unwrap(),
    );
}

#[tokio::test]
async fn did_elohim_assembly_output_conforms_to_did11_schema() {
    // Fully populated: verification method, alsoKnownAs transport ids, services.
    let resolver = ElohimResolver::new(MockElohimStore::populated(AGENT_KEY));
    let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();
    let doc = resolver.resolve(&did).await.unwrap().did_document.unwrap();

    // Sanity: the populated fields we most care about are actually present.
    assert!(doc.also_known_as.is_some(), "expected transport ids");
    assert!(doc.service.is_some(), "expected services");

    assert_conforms(
        "did:elohim assembly output (populated store)",
        &serde_json::to_value(&doc).unwrap(),
    );
}

#[tokio::test]
async fn did_elohim_with_controllers_conforms_to_did11_schema() {
    // Wave C1: a did:elohim document assembled WITH an identity head — populated
    // `controller` (Group Control / recovery quorum) + a chain-root lineage alias
    // — must still validate against the DID 1.1 schema.
    const CONTROLLER_KEY: &str = "uhCAkcHVja32JeYONbe1Dag4rWFB8Jj4-Q5Nv6iyClY-J6-teAgTr";
    const CHAIN_ROOT: &str = "bafyreichainrootgenesisnodecid00000000000000000000000000";

    let store = MockElohimStore::populated(AGENT_KEY).with_head(
        AGENT_KEY,
        CHAIN_ROOT,
        &[AGENT_KEY, CONTROLLER_KEY],
    );
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();
    let doc = resolver.resolve(&did).await.unwrap().did_document.unwrap();

    // Sanity: the head-derived fields we care about are present.
    assert!(doc.controller.is_some(), "expected populated controller");
    let json = serde_json::to_value(&doc).unwrap();
    assert!(json.get("controller").is_some(), "controller serialized");

    assert_conforms(
        "did:elohim assembly output (with controllers + lineage)",
        &json,
    );
}

#[tokio::test]
async fn deactivated_did_elohim_document_conforms_to_did11_schema() {
    // A revoked identity's document is stripped to `@context` + `id` +
    // `alsoKnownAs`. That is a legal DID 1.1 document (only @context and id are
    // required) — assert it, so "confers no authority" can never be achieved by
    // emitting something the schema would reject.
    const CHAIN_ROOT: &str = "bafyreichainrootgenesisnodecid00000000000000000000000000";

    let store =
        MockElohimStore::populated(AGENT_KEY).with_revoked_head(AGENT_KEY, CHAIN_ROOT, None);
    let resolver = ElohimResolver::new(store);
    let did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();
    let result = resolver.resolve(&did).await.unwrap();

    assert_eq!(result.did_document_metadata.deactivated, Some(true));
    let doc = result.did_document.unwrap();
    assert!(doc.verification_method.is_none(), "deactivated ⇒ no keys");

    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        json["@context"][0], "https://www.w3.org/ns/did/v1.1",
        "deactivated output must still pin the DID 1.1 context first"
    );
    assert_conforms("did:elohim deactivated (revoked head) output", &json);
}

/// A wire-fidelity fixture carrying the legacy DID **1.0** context. It must be
/// accepted (the schema recognizes v1 as a legal base context).
const V1_CONTEXT_FIXTURE: &str = r#"{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://w3id.org/security/multikey/v1"
  ],
  "id": "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
  "verificationMethod": [
    {
      "id": "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
      "type": "Multikey",
      "controller": "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
      "publicKeyMultibase": "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
    }
  ],
  "authentication": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ]
}"#;

/// A wire-fidelity fixture with a JWK verification method AND extension
/// properties at document, verification-method, and service levels. It must be
/// accepted (additionalProperties stays true; JWK is a legal key form).
const EXTENSION_BEARING_FIXTURE: &str = r#"{
  "@context": [
    "https://www.w3.org/ns/did/v1.1",
    "https://w3id.org/security/jwk/v1"
  ],
  "id": "did:web:example.com",
  "unknownDocumentProperty": {"nested": ["a", "b"], "n": 42},
  "verificationMethod": [
    {
      "id": "did:web:example.com#key-1",
      "type": "JsonWebKey2020",
      "controller": "did:web:example.com",
      "publicKeyJwk": {"kty": "OKP", "crv": "Ed25519", "x": "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"},
      "unknownVmProperty": "preserve-me"
    }
  ],
  "service": [
    {
      "id": "did:web:example.com#svc",
      "type": "SomeService",
      "serviceEndpoint": "https://example.com/svc",
      "unknownServiceProperty": true
    }
  ]
}"#;

#[test]
fn wire_fidelity_v1_context_fixture_conforms() {
    let doc: Value = serde_json::from_str(V1_CONTEXT_FIXTURE).unwrap();
    assert_conforms("wire-fidelity 1.0-context fixture", &doc);
}

#[test]
fn wire_fidelity_extension_bearing_fixture_conforms() {
    let doc: Value = serde_json::from_str(EXTENSION_BEARING_FIXTURE).unwrap();
    assert_conforms("wire-fidelity extension-bearing fixture", &doc);
}

#[tokio::test]
async fn emitted_documents_pin_v1_1_context_first() {
    // did:key output.
    let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    let key_doc = DidKeyResolver::new()
        .resolve(&did)
        .await
        .unwrap()
        .did_document
        .unwrap();
    let key_json = serde_json::to_value(&key_doc).unwrap();
    assert_eq!(
        key_json["@context"][0], "https://www.w3.org/ns/did/v1.1",
        "did:key output must pin the DID 1.1 context first"
    );

    // did:elohim output.
    let resolver = ElohimResolver::new(MockElohimStore::populated(AGENT_KEY));
    let el_did = Did::parse(&format!("did:elohim:{AGENT_KEY}")).unwrap();
    let el_doc = resolver
        .resolve(&el_did)
        .await
        .unwrap()
        .did_document
        .unwrap();
    let el_json = serde_json::to_value(&el_doc).unwrap();
    assert_eq!(
        el_json["@context"][0], "https://www.w3.org/ns/did/v1.1",
        "did:elohim output must pin the DID 1.1 context first"
    );
}

#[test]
fn document_missing_id_fails_schema_gate() {
    // Proves the gate has teeth: a document lacking the required `id` is rejected.
    let bad = serde_json::json!({
        "@context": "https://www.w3.org/ns/did/v1.1"
    });
    let v = validator();
    let errs = violations(&v, &bad);
    assert!(
        !errs.is_empty(),
        "schema gate must REJECT a document missing the required `id` — it did not"
    );
}
