//! `did:web` URL derivation and resolution over an injected fixture fetch (no
//! network).

use did_bridge::{
    derive_did_web_url, DidResolutionError, DidResolutionResult, DidResolver, DidWebResolver,
    ERR_INVALID_DID_DOCUMENT,
};
use did_tests::FixtureFetch;
use did_types::Did;

#[test]
fn url_derivation_covers_the_method_spec_forms() {
    let cases = [
        (
            "did:web:example.com",
            "https://example.com/.well-known/did.json",
        ),
        (
            "did:web:example.com:user:alice",
            "https://example.com/user/alice/did.json",
        ),
        (
            "did:web:example.com%3A3000",
            "https://example.com:3000/.well-known/did.json",
        ),
        (
            "did:web:example.com%3A3000:user:alice",
            "https://example.com:3000/user/alice/did.json",
        ),
    ];
    for (did, expected) in cases {
        let parsed = Did::parse(did).unwrap();
        assert_eq!(derive_did_web_url(&parsed).unwrap(), expected, "for {did}");
    }
}

#[tokio::test]
async fn resolves_did_web_via_injected_fetch() {
    let doc_json = r#"{
      "@context": "https://www.w3.org/ns/did/v1.1",
      "id": "did:web:example.com",
      "verificationMethod": [
        {
          "id": "did:web:example.com#key-1",
          "type": "Multikey",
          "controller": "did:web:example.com",
          "publicKeyMultibase": "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
        }
      ]
    }"#;
    let fetch =
        FixtureFetch::default().with_doc("https://example.com/.well-known/did.json", doc_json);
    let resolver = DidWebResolver::new(fetch);

    let did = Did::parse("did:web:example.com").unwrap();
    let result = resolver.resolve(&did).await.unwrap();
    let doc = result.did_document.unwrap();
    assert_eq!(doc.id.to_string(), "did:web:example.com");
    assert_eq!(doc.verification_method.unwrap().len(), 1);
}

#[tokio::test]
async fn a_host_may_not_answer_with_another_domains_did() {
    // Forecast row 8 (C5, confirmed live): `example.com` serves a document whose
    // `id` is someone else's DID. The fetch succeeded, the JSON parsed, the SSRF
    // guard was never in play — and before the subject re-derivation this
    // returned as a SUCCESSFUL resolution of `did:web:bank.example`.
    let impostor = r#"{
      "@context": "https://www.w3.org/ns/did/v1.1",
      "id": "did:web:bank.example",
      "verificationMethod": [
        {
          "id": "did:web:bank.example#key-1",
          "type": "Multikey",
          "controller": "did:web:bank.example",
          "publicKeyMultibase": "z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr"
        }
      ]
    }"#;
    let fetch =
        FixtureFetch::default().with_doc("https://example.com/.well-known/did.json", impostor);
    let resolver = DidWebResolver::new(fetch);

    let did = Did::parse("did:web:example.com").unwrap();
    let err = resolver.resolve(&did).await.unwrap_err();
    assert!(
        matches!(err, DidResolutionError::SubjectMismatch { .. }),
        "expected SubjectMismatch, got {err:?}"
    );
    assert_eq!(err.error_code(), ERR_INVALID_DID_DOCUMENT);
    // Nothing is served, and the failure is not blamed on the requested DID.
    let result = DidResolutionResult::from_error(&err);
    assert!(result.did_document.is_none());
    assert_eq!(
        result.did_resolution_metadata.error.as_deref(),
        Some(ERR_INVALID_DID_DOCUMENT)
    );
}

#[tokio::test]
async fn missing_did_web_document_is_not_found() {
    let resolver = DidWebResolver::new(FixtureFetch::default());
    let did = Did::parse("did:web:absent.example").unwrap();
    let err = resolver.resolve(&did).await.unwrap_err();
    assert_eq!(err.error_code(), "notFound");
}
