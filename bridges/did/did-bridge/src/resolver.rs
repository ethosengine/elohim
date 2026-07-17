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

/// Metadata about the DID *document* (DID 1.1 §7.1.2). Left minimal in phase 1
/// (no on-chain identity head yet — see the spec's phase-2 follow-on).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidDocumentMetadata {
    /// Document creation timestamp, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Document last-update timestamp, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
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
}

impl DidResolutionError {
    /// The standard DID 1.1 metadata error code string for this error.
    pub fn error_code(&self) -> &'static str {
        match self {
            DidResolutionError::NotFound(_) => ERR_NOT_FOUND,
            DidResolutionError::InvalidDid(_) => ERR_INVALID_DID,
            DidResolutionError::MethodNotSupported(_) => ERR_METHOD_NOT_SUPPORTED,
            DidResolutionError::Internal(_) => ERR_INTERNAL,
        }
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
