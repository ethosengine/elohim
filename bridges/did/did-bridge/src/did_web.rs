//! The `did:web` resolver — feature-gated (`web-resolver`), off by default.
//!
//! No HTTP client is pulled in: the caller injects a [`DidWebFetch`]. This crate
//! owns the standards part — deriving the HTTPS URL from the DID and parsing the
//! fetched document — while the runtime owns transport.
//!
//! URL derivation (did:web method spec):
//! - `did:web:example.com` → `https://example.com/.well-known/did.json`
//! - `did:web:example.com:user:alice` → `https://example.com/user/alice/did.json`
//! - `did:web:example.com%3A3000` → `https://example.com:3000/.well-known/did.json`
//!   (the first `:`-segment is percent-decoded, so `%3A` becomes the port `:`)
//!
//! ## Security — the response is evidence, not authority (C5)
//!
//! `did:web` is the one method here that is **not** self-certifying: `did:key`
//! and `did:elohim` derive their document from the key itself, while `did:web`
//! is handed a document by a host. A successful HTTPS fetch establishes that the
//! host answered — nothing more. So after parsing, the document's own `id` is
//! re-checked against the DID that was asked about
//! ([`verify_resolved_subject`](crate::verify_resolved_subject)); a mismatch is a
//! typed refusal, never a pass-through. Without that check, any host that can
//! answer for one domain can return a *different* domain's DID document and have
//! it reported as a successful resolution of that other DID.
//!
//! ## Security — authority confusion (SSRF)
//!
//! `Did::parse` forbids a literal `@` in the method-specific-id, but `%40` is a
//! legal percent-encoded idchar that only becomes `@` *after* decoding. Left
//! unchecked, `did:web:trusted.example.com%40169.254.169.254` would derive
//! `https://trusted.example.com@169.254.169.254/…` — userinfo/host confusion
//! pointing at a link-local metadata address. So **after** percent-decoding,
//! every segment is rejected (not sanitized) if it contains `@`, `/`, `\`, `?`,
//! `#`, or a control character. This closes the confusion class; it does **not**
//! make the resolved host safe — see [`DidWebFetch`].

use async_trait::async_trait;
use did_types::{Did, DidDocument};
use percent_encoding::percent_decode_str;

use crate::resolver::{
    verify_resolved_subject, DidResolutionError, DidResolutionResult, DidResolver,
};

/// The transport seam for `did:web`: fetch the bytes at a derived URL. The
/// caller (doorway) provides an HTTP client; tests provide a fixture map.
///
/// ## Security obligation (defense in depth)
///
/// [`derive_did_web_url`] rejects authority/host *confusion* (see the module
/// docs), but it cannot know your network topology. A `did:web` DID can name
/// **any** host — including `localhost`, RFC-1918 ranges, or a cloud metadata
/// endpoint (`169.254.169.254`). Implementations **MUST** validate the resolved
/// URL's actual host against their egress policy (block loopback, link-local,
/// private, and metadata ranges; resolve DNS and re-check; consider an allowlist)
/// before making the request. The crate owns confusion-rejection; the fetcher
/// owns network policy.
#[async_trait]
pub trait DidWebFetch: Send + Sync {
    /// Fetch the raw bytes served at `url` (an `https://…/did.json` URL).
    ///
    /// Implementations MUST enforce their egress policy on the URL's host before
    /// connecting — the derivation guarantees no userinfo/path confusion, not a
    /// safe destination.
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, DidResolutionError>;
}

/// Derive the HTTPS document URL for a `did:web` DID per the method spec.
///
/// Rejects (does not sanitize) any segment that, after percent-decoding,
/// contains an authority/path-confusion character — see the module docs.
pub fn derive_did_web_url(did: &Did) -> Result<String, DidResolutionError> {
    if did.method() != "web" {
        return Err(DidResolutionError::MethodNotSupported(
            did.method().to_string(),
        ));
    }
    let msi = did.method_specific_id();

    // Split on ':' into segments, percent-decode each, then validate.
    let mut segments = msi.split(':');
    let authority_raw = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DidResolutionError::InvalidDid(format!("empty authority in {msi:?}")))?;
    let authority = decode_and_validate(authority_raw, "authority")?;

    let path_segments: Vec<String> = segments
        .map(|s| decode_and_validate(s, "path segment"))
        .collect::<Result<Vec<_>, _>>()?;

    let path = if path_segments.is_empty() {
        "/.well-known/did.json".to_string()
    } else {
        format!("/{}/did.json", path_segments.join("/"))
    };

    Ok(format!("https://{authority}{path}"))
}

/// Percent-decode one segment and reject authority/path-confusion characters.
///
/// `@` (userinfo separator), `/` `\` (path/authority injection), `?` `#`
/// (query/fragment injection), and control characters are all illegal in a
/// decoded segment — their presence means the encoded form was smuggling
/// structure past `Did::parse`.
fn decode_and_validate(s: &str, role: &str) -> Result<String, DidResolutionError> {
    let decoded = percent_decode_str(s)
        .decode_utf8()
        .map(|c| c.into_owned())
        .map_err(|e| DidResolutionError::InvalidDid(format!("bad percent-encoding: {e}")))?;

    if let Some(bad) = decoded
        .chars()
        .find(|&c| matches!(c, '@' | '/' | '\\' | '?' | '#') || c.is_control())
    {
        return Err(DidResolutionError::InvalidDid(format!(
            "illegal character {bad:?} in did:web {role} after percent-decoding ({decoded:?}) \
             — authority/path confusion rejected"
        )));
    }
    Ok(decoded)
}

/// Resolver for the `did:web` method over an injected fetch.
pub struct DidWebResolver<F: DidWebFetch> {
    fetch: F,
}

impl<F: DidWebFetch> DidWebResolver<F> {
    /// Construct the resolver over a fetch implementation.
    pub fn new(fetch: F) -> Self {
        DidWebResolver { fetch }
    }
}

#[async_trait]
impl<F: DidWebFetch> DidResolver for DidWebResolver<F> {
    fn method(&self) -> &'static str {
        "web"
    }

    async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError> {
        let url = derive_did_web_url(did)?;
        let bytes = self.fetch.fetch(&url).await?;
        let doc: DidDocument = serde_json::from_slice(&bytes)
            .map_err(|e| DidResolutionError::Internal(format!("did.json parse failed: {e}")))?;
        // C5: the host answered — that is all the fetch established. Re-derive
        // the one thing that makes this document an answer to THIS question.
        verify_resolved_subject(did, &doc)?;
        Ok(DidResolutionResult::success(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_well_known_for_bare_domain() {
        let did = Did::parse("did:web:example.com").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap(),
            "https://example.com/.well-known/did.json"
        );
    }

    #[test]
    fn derives_path_form() {
        let did = Did::parse("did:web:example.com:user:alice").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap(),
            "https://example.com/user/alice/did.json"
        );
    }

    #[test]
    fn derives_port_via_percent_decode() {
        let did = Did::parse("did:web:example.com%3A3000").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap(),
            "https://example.com:3000/.well-known/did.json"
        );
    }

    #[test]
    fn derives_port_and_path() {
        let did = Did::parse("did:web:example.com%3A3000:user:alice").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap(),
            "https://example.com:3000/user/alice/did.json"
        );
    }

    #[test]
    fn rejects_at_sign_userinfo_confusion() {
        // %40 → '@'. Would derive https://trusted@169.254.169.254/… (SSRF).
        let did = Did::parse("did:web:trusted.example.com%40169.254.169.254").unwrap();
        let err = derive_did_web_url(&did).unwrap_err();
        assert_eq!(err.error_code(), "invalidDid");
    }

    #[test]
    fn rejects_slash_path_injection() {
        // %2F → '/'. Would inject an extra path/authority boundary.
        let did = Did::parse("did:web:example.com%2Fevil.com").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap_err().error_code(),
            "invalidDid"
        );
    }

    #[test]
    fn rejects_backslash_confusion() {
        // %5C → '\'. Some URL parsers treat '\' as '/'. (A literal '@' can't
        // appear pre-decode — Did::parse forbids it — so we test '\' alone.)
        let did = Did::parse("did:web:example.com%5Cevil.com").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap_err().error_code(),
            "invalidDid"
        );
    }

    #[test]
    fn rejects_query_and_fragment_injection() {
        // '=' is not a legal idchar, so keep the payload idchar-only pre-decode.
        let q = Did::parse("did:web:example.com%3Ffoo").unwrap(); // %3F → '?'
        assert_eq!(
            derive_did_web_url(&q).unwrap_err().error_code(),
            "invalidDid"
        );
        let h = Did::parse("did:web:example.com%23frag").unwrap(); // %23 → '#'
        assert_eq!(
            derive_did_web_url(&h).unwrap_err().error_code(),
            "invalidDid"
        );
    }

    #[test]
    fn rejects_control_character() {
        // %00 → NUL. A control character smuggled into the authority.
        let did = Did::parse("did:web:example.com%00.evil.com").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap_err().error_code(),
            "invalidDid"
        );
    }

    #[test]
    fn rejects_confusion_in_path_segment_too() {
        // The guard applies to path segments, not just the authority.
        let did = Did::parse("did:web:example.com:user%40evil.com").unwrap();
        assert_eq!(
            derive_did_web_url(&did).unwrap_err().error_code(),
            "invalidDid"
        );
    }

    /// A fetch that always serves the same bytes, whatever URL is asked for —
    /// i.e. a host answering for itself. The document it serves is the variable
    /// under test.
    struct FixedBytes(&'static str);

    #[async_trait]
    impl DidWebFetch for FixedBytes {
        async fn fetch(&self, _url: &str) -> Result<Vec<u8>, DidResolutionError> {
            Ok(self.0.as_bytes().to_vec())
        }
    }

    const SELF_DOC: &str = r#"{
      "@context": "https://www.w3.org/ns/did/v1.1",
      "id": "did:web:example.com"
    }"#;

    /// The document `example.com` serves claims to BE `bank.example` — the
    /// forecast-row-7 defect, at the resolver rather than the predicate.
    const FOREIGN_DOC: &str = r#"{
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

    #[tokio::test]
    async fn resolve_accepts_a_document_that_names_the_requested_subject() {
        let resolver = DidWebResolver::new(FixedBytes(SELF_DOC));
        let did = Did::parse("did:web:example.com").unwrap();
        let result = resolver.resolve(&did).await.unwrap();
        assert_eq!(result.did_document.unwrap().id, did);
    }

    #[tokio::test]
    async fn resolve_refuses_a_document_naming_another_subject() {
        let resolver = DidWebResolver::new(FixedBytes(FOREIGN_DOC));
        let did = Did::parse("did:web:example.com").unwrap();
        let err = resolver.resolve(&did).await.unwrap_err();
        match &err {
            DidResolutionError::SubjectMismatch {
                requested,
                returned,
            } => {
                assert_eq!(requested, "did:web:example.com");
                assert_eq!(returned, "did:web:bank.example");
            }
            other => panic!("expected SubjectMismatch, got {other:?}"),
        }
        assert_eq!(err.error_code(), "invalidDidDocument");
        // And nothing leaks through: the error result carries no document.
        assert!(DidResolutionResult::from_error(&err).did_document.is_none());
    }
}
