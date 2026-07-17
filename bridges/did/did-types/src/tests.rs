//! Unit tests for the DID 1.1 data model: identifier parsing and wire-faithful
//! serde round-trips against realistic DID document JSON.

use crate::did::{Did, DidUrl};
use crate::document::*;

#[test]
fn parses_valid_dids() {
    let d = Did::parse("did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH").unwrap();
    assert_eq!(d.method(), "elohim");
    assert_eq!(
        d.method_specific_id(),
        "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH"
    );

    let k = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    assert_eq!(k.method(), "key");

    // Multi-segment method-specific-id (did:web with port).
    let w = Did::parse("did:web:example.com:3000:user:alice").unwrap();
    assert_eq!(w.method(), "web");
    assert_eq!(w.method_specific_id(), "example.com:3000:user:alice");

    // Percent-encoded idchar.
    assert!(Did::parse("did:web:example.com%3A3000").is_ok());
}

#[test]
fn rejects_invalid_dids() {
    assert!(Did::parse("urn:uuid:1234").is_err()); // wrong scheme
    assert!(Did::parse("did:").is_err()); // no method
    assert!(Did::parse("did:key").is_err()); // no msi separator
    assert!(Did::parse("did:key:").is_err()); // empty msi
    assert!(Did::parse("did:KEY:abc").is_err()); // uppercase method
    assert!(Did::parse("did:key:abc:").is_err()); // trailing colon (empty final segment)
    assert!(Did::parse("did:key:ab cd").is_err()); // illegal space
    assert!(Did::parse("did:key:ab%zz").is_err()); // malformed pct-encoding
}

#[test]
fn did_display_round_trips() {
    let s = "did:elohim:uhCAkwfTgZ5eDJwI6ZV5vGt-kg8cVgXvcf35XKj6HnMv4PBH8noYB";
    assert_eq!(Did::parse(s).unwrap().to_string(), s);
}

#[test]
fn did_url_parses_fragment_reference() {
    let u = DidUrl::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    assert!(u.as_str().contains('#'));
    // A DID-less URL fragment must fail.
    assert!(DidUrl::parse("#justafragment").is_err());
}

#[test]
fn did_serializes_as_bare_string() {
    let d = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    let j = serde_json::to_string(&d).unwrap();
    assert_eq!(
        j,
        "\"did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr\""
    );
    let back: Did = serde_json::from_str(&j).unwrap();
    assert_eq!(back, d);
}

/// A realistic `did:key` DID document — verification method plus all five
/// relationships referencing it.
const DID_KEY_DOC: &str = r#"{
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
  ],
  "assertionMethod": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ],
  "keyAgreement": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ],
  "capabilityInvocation": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ],
  "capabilityDelegation": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ]
}"#;

/// A realistic `did:elohim` DID document with alsoKnownAs transport ids and
/// services (profile + doorway).
const DID_ELOHIM_DOC: &str = r#"{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://w3id.org/security/multikey/v1"
  ],
  "id": "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
  "alsoKnownAs": [
    "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
    "12D3KooWABCDEexamplePeerId",
    "iroh:nodeidexample0000"
  ],
  "verificationMethod": [
    {
      "id": "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
      "type": "Multikey",
      "controller": "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
      "publicKeyMultibase": "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
    }
  ],
  "authentication": [
    "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ],
  "assertionMethod": [
    "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH#z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
  ],
  "service": [
    {
      "id": "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH#profile",
      "type": "ProfileService",
      "serviceEndpoint": "https://doorway.elohim.host/profile/uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH"
    },
    {
      "id": "did:elohim:uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH#doorway",
      "type": "DoorwayService",
      "serviceEndpoint": "https://doorway.elohim.host"
    }
  ]
}"#;

fn assert_json_round_trip(src: &str) {
    let doc: DidDocument = serde_json::from_str(src).expect("parse doc");
    let reserialized = serde_json::to_value(&doc).expect("serialize doc");
    let original: serde_json::Value = serde_json::from_str(src).expect("parse src value");
    assert_eq!(
        reserialized, original,
        "DID document did not round-trip byte-faithfully"
    );
}

#[test]
fn did_key_document_round_trips() {
    assert_json_round_trip(DID_KEY_DOC);
}

#[test]
fn did_elohim_document_round_trips() {
    assert_json_round_trip(DID_ELOHIM_DOC);
}

#[test]
fn verification_relationship_discriminates_reference_vs_embedded() {
    let doc: DidDocument = serde_json::from_str(DID_KEY_DOC).unwrap();
    let auth = doc.authentication.unwrap();
    assert!(matches!(auth[0], VerificationRelationship::Reference(_)));

    let embedded = r#"[
      {
        "id": "did:key:z6Mkabc#z6Mkabc",
        "type": "Multikey",
        "controller": "did:key:z6Mkabc",
        "publicKeyMultibase": "z6Mkabc"
      }
    ]"#;
    let rels: Vec<VerificationRelationship> = serde_json::from_str(embedded).unwrap();
    assert!(matches!(rels[0], VerificationRelationship::Embedded(_)));
}

#[test]
fn default_context_is_did_v1_1() {
    // The default this crate emits must be the DID 1.1 base context, not 1.0.
    let ctx = Context::did_v1_1_multikey();
    match &ctx {
        Context::Many(entries) => match &entries[0] {
            ContextEntry::Uri(u) => assert_eq!(u, DID_CONTEXT_V1_1),
            _ => panic!("first context entry must be a URI"),
        },
        _ => panic!("default context should be a set"),
    }
    assert!(ctx.has_did_base());
    // Serialized form carries the 1.1 URI verbatim.
    let j = serde_json::to_string(&ctx).unwrap();
    assert!(j.contains("https://www.w3.org/ns/did/v1.1"));
    assert!(!j.contains("did/v1\"")); // not the bare 1.0 URI
}

#[test]
fn legacy_v1_context_round_trips_without_rewrite() {
    // The fixtures above carry the 1.0 context; deserializing then reserializing
    // must preserve it exactly (wire fidelity — never silently upgrade to 1.1).
    let doc: DidDocument = serde_json::from_str(DID_KEY_DOC).unwrap();
    let out = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        out["@context"][0], "https://www.w3.org/ns/did/v1",
        "1.0 context must be preserved, not rewritten to 1.1"
    );
}

#[test]
fn controller_accepts_one_or_many() {
    let one: Controller = serde_json::from_str("\"did:key:z6Mkabc\"").unwrap();
    assert!(matches!(one, Controller::One(_)));
    let many: Controller =
        serde_json::from_str("[\"did:key:z6Mkabc\", \"did:elohim:uhCAkabc\"]").unwrap();
    assert!(matches!(many, Controller::Many(_)));
}
