//! The `DidResolver` trait — the IoC seam — plus DID 1.1 resolution result and
//! error types with standard metadata error codes.

use async_trait::async_trait;
use did_types::{Did, DidDocument};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standard DID resolution metadata error code: the DID does not exist.
pub const ERR_NOT_FOUND: &str = "notFound";
/// Standard DID resolution metadata error code: the DID is syntactically invalid.
pub const ERR_INVALID_DID: &str = "invalidDid";
/// Standard DID resolution metadata error code: no resolver for the method.
pub const ERR_METHOD_NOT_SUPPORTED: &str = "methodNotSupported";
/// Standard DID resolution metadata error code: an internal error occurred.
pub const ERR_INTERNAL: &str = "internalError";
/// DID resolution metadata error code: the *fetched document* is not a valid
/// answer for the DID that was asked about.
///
/// From the DID Resolution error vocabulary (the companion registry to DID 1.1,
/// which itself registers only the four codes above). Named here rather than
/// reusing `invalidDid` on purpose: `invalidDid` blames the *requester's* DID,
/// and blaming the requester for a responder's bad answer is exactly the
/// misattribution that makes a transfer-boundary defect hard to see.
pub const ERR_INVALID_DID_DOCUMENT: &str = "invalidDidDocument";

/// The JSON media type for a resolved DID document.
pub const DID_DOCUMENT_CONTENT_TYPE: &str = "application/did+ld+json";

/// Metadata about the resolution *process* (DID 1.1 §7.1.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidResolutionMetadata {
    /// The media type of the returned document (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// A standard error code (present on failure): `notFound`, `invalidDid`, …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Metadata about the DID *document* (DID 1.1 §7.1.2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidDocumentMetadata {
    /// Document creation timestamp, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Document last-update timestamp, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// `true` when the DID has been **deactivated** (DID 1.1 §7.1.2): the
    /// subject's identity head is revoked, so the document confers no authority.
    ///
    /// `None` (omitted) means "not deactivated, and no claim recorded" — the
    /// ordinary case. This is deliberately NOT defaulted to `Some(false)`: an
    /// omitted property and an asserted `false` are different statements, and
    /// the spec makes the property OPTIONAL exactly when it is false.
    ///
    /// A revoked identity is the one case where the *absence* of this flag was
    /// silently load-bearing — see `did_elohim::IdentityHeadAnswer::Revoked`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<bool>,
}

/// The result of a DID resolution (DID 1.1 §7.1): the document plus the two
/// metadata blocks. On error, `did_document` is `None` and
/// `did_resolution_metadata.error` carries the standard code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidResolutionResult {
    /// The resolved DID document, or `None` on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_document: Option<DidDocument>,
    /// Process metadata (content type or error code).
    pub did_resolution_metadata: DidResolutionMetadata,
    /// Document metadata.
    pub did_document_metadata: DidDocumentMetadata,
}

impl DidResolutionResult {
    /// A successful result carrying `document` with the standard content type.
    pub fn success(document: DidDocument) -> Self {
        DidResolutionResult {
            did_document: Some(document),
            did_resolution_metadata: DidResolutionMetadata {
                content_type: Some(DID_DOCUMENT_CONTENT_TYPE.to_string()),
                error: None,
            },
            did_document_metadata: DidDocumentMetadata::default(),
        }
    }

    /// An error result whose resolution metadata carries the standard error code.
    pub fn from_error(err: &DidResolutionError) -> Self {
        DidResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(err.error_code().to_string()),
            },
            did_document_metadata: DidDocumentMetadata::default(),
        }
    }
}

/// A DID resolution error mapped to a standard DID 1.1 metadata error code.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DidResolutionError {
    /// The DID does not exist in the resolver's namespace.
    #[error("DID not found: {0}")]
    NotFound(String),
    /// The DID (or method-specific-id) is syntactically invalid.
    #[error("invalid DID: {0}")]
    InvalidDid(String),
    /// No resolver is registered for the DID's method.
    #[error("method not supported: {0}")]
    MethodNotSupported(String),
    /// An internal error occurred during resolution (I/O, parse, assembly).
    #[error("internal resolution error: {0}")]
    Internal(String),
    /// A *fetched* document does not describe the DID that was resolved — its
    /// `id` names a different subject. The response is refused, never passed
    /// through. See [`verify_resolved_subject`].
    #[error(
        "resolved document subject mismatch: asked for {requested}, document claims {returned}"
    )]
    SubjectMismatch {
        /// The DID the caller asked to resolve.
        requested: String,
        /// The subject the fetched document actually claims to describe.
        returned: String,
    },
    /// The identity head for the subject could not be determined — the store
    /// reached for it and got no answer (an undelivered DHT record, a timed-out
    /// read). **Not** "no head is declared": that is
    /// `IdentityHeadAnswer::NeverDeclared`, a different answer with a different
    /// document. Resolution fails closed rather than assuming self-control.
    #[error("identity head unresolvable: {0}")]
    IdentityHeadUnresolvable(String),
    /// An identity head WAS resolved but is unusable as declared — e.g. it
    /// declares an empty controller set, which would serialize to a document
    /// with no `controller` and therefore read as implicit self-control, which
    /// is precisely what the declaration did not say.
    #[error("identity head malformed: {0}")]
    IdentityHeadMalformed(String),
}

impl DidResolutionError {
    /// The standard DID metadata error code string for this error.
    ///
    /// This is a deliberately **lossy** projection: the DID error-code
    /// vocabulary has no finer bucket than `internalError` for "the head is
    /// unresolvable" versus "the head is malformed," so both project there while
    /// staying distinct Rust variants. The distinction survives where it is
    /// actionable (in-process, at the call site that must decide whether to
    /// retry or to refuse) and the wire stays conformant — rather than inventing
    /// unregistered codes that no universal resolver would understand.
    pub fn error_code(&self) -> &'static str {
        match self {
            DidResolutionError::NotFound(_) => ERR_NOT_FOUND,
            DidResolutionError::InvalidDid(_) => ERR_INVALID_DID,
            DidResolutionError::MethodNotSupported(_) => ERR_METHOD_NOT_SUPPORTED,
            DidResolutionError::Internal(_)
            | DidResolutionError::IdentityHeadUnresolvable(_)
            | DidResolutionError::IdentityHeadMalformed(_) => ERR_INTERNAL,
            DidResolutionError::SubjectMismatch { .. } => ERR_INVALID_DID_DOCUMENT,
        }
    }
}

/// Verify that a **fetched** DID document actually describes the DID that was
/// resolved — C5 (evidence-not-authority) at the resolution transfer boundary.
///
/// A transferred claim confers only what the receiver re-derives. `did:key` and
/// `did:elohim` are self-certifying (the document is *assembled* locally from
/// the key, so its subject is derived, not asserted); `did:web` is not — the
/// document arrives from a host over the network, and nothing about a successful
/// HTTPS fetch establishes that the bytes describe the DID we asked about. Left
/// unchecked, a host that can answer for `example.com` can hand back a document
/// whose `id` is `did:web:bank.example`, and the resolver would report it as a
/// successful resolution of *that* DID.
///
/// ## The comparison is EXACT, because this crate normalizes nothing
///
/// [`Did::parse`](did_types::Did::parse) stores the method and
/// method-specific-id verbatim — no case folding, no percent-decoding, no
/// canonicalization. So the honest check is byte equality of the two canonical
/// DID strings, and DID Core agrees: DIDs are case-sensitive identifiers.
/// `did:web:Example.com` and `did:web:example.com` are different DIDs here, and
/// a document answering with the other spelling is refused rather than accepted
/// under a normalization rule this crate does not implement.
///
/// **If normalization is ever added, it must be applied to BOTH sides through
/// the same function.** Deriving the expected value with the code under test is
/// how a green test measures its own mirror instead of the behaviour (the
/// standing C5 verification discipline).
///
/// # Errors
///
/// [`DidResolutionError::SubjectMismatch`] (metadata code
/// [`ERR_INVALID_DID_DOCUMENT`]) when the subjects differ. A mismatch is a
/// **typed refusal**: never a pass-through, never downgraded to a warning.
pub fn verify_resolved_subject(
    requested: &Did,
    document: &DidDocument,
) -> Result<(), DidResolutionError> {
    let requested_s = requested.as_string();
    let returned_s = document.id.as_string();
    if requested_s == returned_s {
        Ok(())
    } else {
        Err(DidResolutionError::SubjectMismatch {
            requested: requested_s,
            returned: returned_s,
        })
    }
}

/// A DID method resolver — one implementation per method. The IoC seam: methods
/// plug in here (did:key, did:elohim, did:web now; did:plc with the atproto
/// bridge later) rather than inventing bespoke resolution paths.
#[async_trait]
pub trait DidResolver: Send + Sync {
    /// The DID method this resolver handles (`"key"`, `"elohim"`, `"web"`, …).
    fn method(&self) -> &'static str;

    /// Resolve a DID to its document and resolution metadata.
    async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use did_types::Context;

    /// Build a document claiming `subject` — deliberately hand-built from a DID
    /// string rather than from the value under comparison, so the two sides of
    /// every assertion below have independent provenance (anti-mirror).
    fn doc_claiming(subject: &str) -> DidDocument {
        DidDocument::new(
            Context::did_v1_1_multikey(),
            Did::parse(subject).expect("fixture subject parses"),
        )
    }

    #[test]
    fn matching_subject_is_accepted() {
        let requested = Did::parse("did:web:example.com").unwrap();
        let doc = doc_claiming("did:web:example.com");
        assert!(verify_resolved_subject(&requested, &doc).is_ok());
    }

    #[test]
    fn foreign_subject_is_refused_as_invalid_did_document() {
        // The forecast-row-7 attack: a host that can answer for example.com
        // hands back a document describing someone else's DID.
        let requested = Did::parse("did:web:example.com").unwrap();
        let doc = doc_claiming("did:web:bank.example");
        let err = verify_resolved_subject(&requested, &doc).unwrap_err();
        assert!(matches!(err, DidResolutionError::SubjectMismatch { .. }));
        assert_eq!(err.error_code(), ERR_INVALID_DID_DOCUMENT);
        // Blame lands on the response, not on the requester's DID and not on us.
        assert_ne!(err.error_code(), ERR_INVALID_DID);
        assert_ne!(err.error_code(), ERR_INTERNAL);
        assert_ne!(err.error_code(), ERR_NOT_FOUND);
    }

    #[test]
    fn subject_comparison_is_exact_not_case_folded() {
        // This crate normalizes nothing, so equality is byte-exact and a
        // differently-cased authority is a refusal, not a silent acceptance.
        let requested = Did::parse("did:web:example.com").unwrap();
        let doc = doc_claiming("did:web:Example.com");
        assert!(matches!(
            verify_resolved_subject(&requested, &doc),
            Err(DidResolutionError::SubjectMismatch { .. })
        ));
    }

    #[test]
    fn cross_method_subject_is_refused() {
        // A did:web host may not answer with a did:key document either.
        let requested = Did::parse("did:web:example.com").unwrap();
        let doc = doc_claiming("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr");
        assert!(matches!(
            verify_resolved_subject(&requested, &doc),
            Err(DidResolutionError::SubjectMismatch { .. })
        ));
    }

    #[test]
    fn head_error_codes_are_distinct_variants_on_one_wire_code() {
        // The lossy-projection contract, asserted: two distinct in-process
        // provenances, one conformant wire code.
        let unresolvable =
            DidResolutionError::IdentityHeadUnresolvable("dht read timed out".into());
        let malformed = DidResolutionError::IdentityHeadMalformed("empty controller set".into());
        assert_ne!(unresolvable, malformed);
        assert_eq!(unresolvable.error_code(), ERR_INTERNAL);
        assert_eq!(malformed.error_code(), ERR_INTERNAL);
    }

    #[test]
    fn deactivated_metadata_is_omitted_unless_asserted() {
        let default = DidDocumentMetadata::default();
        assert_eq!(default.deactivated, None);
        let json = serde_json::to_value(&default).unwrap();
        assert!(
            json.get("deactivated").is_none(),
            "an unset deactivated flag must not serialize as false: {json}"
        );
    }
}
