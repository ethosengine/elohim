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

use async_trait::async_trait;
use did_types::{Did, DidDocument};
use percent_encoding::percent_decode_str;

use crate::resolver::{DidResolutionError, DidResolutionResult, DidResolver};

/// The transport seam for `did:web`: fetch the bytes at a derived URL. The
/// caller (doorway) provides an HTTP client; tests provide a fixture map.
#[async_trait]
pub trait DidWebFetch: Send + Sync {
    /// Fetch the raw bytes served at `url` (an `https://…/did.json` URL).
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, DidResolutionError>;
}

/// Derive the HTTPS document URL for a `did:web` DID per the method spec.
pub fn derive_did_web_url(did: &Did) -> Result<String, DidResolutionError> {
    if did.method() != "web" {
        return Err(DidResolutionError::MethodNotSupported(
            did.method().to_string(),
        ));
    }
    let msi = did.method_specific_id();

    // Split on ':' into segments, percent-decode each.
    let mut segments = msi.split(':');
    let authority_raw = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DidResolutionError::InvalidDid(format!("empty authority in {msi:?}")))?;
    let authority = percent_decode(authority_raw)?;

    let path_segments: Vec<String> = segments
        .map(percent_decode)
        .collect::<Result<Vec<_>, _>>()?;

    let path = if path_segments.is_empty() {
        "/.well-known/did.json".to_string()
    } else {
        format!("/{}/did.json", path_segments.join("/"))
    };

    Ok(format!("https://{authority}{path}"))
}

fn percent_decode(s: &str) -> Result<String, DidResolutionError> {
    percent_decode_str(s)
        .decode_utf8()
        .map(|c| c.into_owned())
        .map_err(|e| DidResolutionError::InvalidDid(format!("bad percent-encoding: {e}")))
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
}
