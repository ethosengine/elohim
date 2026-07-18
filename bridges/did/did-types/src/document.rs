//! The W3C DID 1.1 document data model.
//!
//! Every wire field uses the DID-core JSON name. Structs carry
//! `#[serde(rename_all = "camelCase")]`; `@context` is renamed explicitly.
//! `Option` fields are omitted from output when empty so assembled documents
//! stay minimal and round-trip byte-stably.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::did::{Did, DidUrl};

/// The W3C DID **1.1** base context — the default this crate emits when
/// assembling or resolving documents (target spec: <https://www.w3.org/TR/did-1.1/>).
pub const DID_CONTEXT_V1_1: &str = "https://www.w3.org/ns/did/v1.1";

/// The legacy DID Core **1.0** base context. Accepted on deserialization and
/// preserved on round-trip for interop, but never emitted by default.
pub const DID_CONTEXT_V1: &str = "https://www.w3.org/ns/did/v1";

/// The Multikey security context (verification-method key material).
pub const MULTIKEY_CONTEXT: &str = "https://w3id.org/security/multikey/v1";

/// The `@context` of a DID document — either a single URI or an ordered set of
/// URIs and inline context objects (DID 1.1 §4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Context {
    /// A single context URI.
    Single(String),
    /// An ordered set of context entries.
    Many(Vec<ContextEntry>),
}

impl Context {
    /// The default context this crate emits: DID **1.1** base context plus the
    /// Multikey context. Used by all assembled/resolved documents.
    pub fn did_v1_1_multikey() -> Self {
        Context::Many(vec![
            ContextEntry::Uri(DID_CONTEXT_V1_1.to_string()),
            ContextEntry::Uri(MULTIKEY_CONTEXT.to_string()),
        ])
    }

    /// Whether this context declares a recognized DID base context (1.0 or 1.1)
    /// as its first entry. Interop check — does not mutate the value.
    pub fn has_did_base(&self) -> bool {
        let first = match self {
            Context::Single(s) => Some(s.as_str()),
            Context::Many(entries) => entries.first().and_then(|e| match e {
                ContextEntry::Uri(u) => Some(u.as_str()),
                ContextEntry::Map(_) => None,
            }),
        };
        matches!(first, Some(DID_CONTEXT_V1_1) | Some(DID_CONTEXT_V1))
    }
}

/// A single `@context` entry: a URI string or an inline object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextEntry {
    /// A context URI.
    Uri(String),
    /// An inline context definition object.
    Map(BTreeMap<String, Value>),
}

/// One or many controllers (DID 1.1 §5.1.2). A DID document's controller MAY be
/// the subject itself or one/many other DIDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Controller {
    /// A single controller DID.
    One(Did),
    /// Multiple controller DIDs (e.g. Group Control / community-recovery quorum).
    Many(Vec<Did>),
}

/// A verification method (DID 1.1 §5.2). We emit the `Multikey` type with a
/// `publicKeyMultibase` value (multibase-encoded multicodec key material).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMethod {
    /// The verification method identifier (a DID URL, usually `<did>#<key>`).
    pub id: DidUrl,
    /// The method type — `Multikey` for the keys this crate emits.
    #[serde(rename = "type")]
    pub type_: String,
    /// The controller DID of this key.
    pub controller: Did,
    /// The multibase-encoded public key (present for `Multikey`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_multibase: Option<String>,
    /// The JWK-encoded public key (present for JWK-typed methods). **Preserved,
    /// not interpreted** in phase 1 — carried opaquely so JWK-based documents
    /// (common in `did:web`) survive round-trip and are not silently keyless.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_jwk: Option<Value>,
    /// Any other verification-method properties, preserved verbatim so legal DID
    /// extension properties round-trip. Empty ⇒ nothing emitted.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl VerificationMethod {
    /// Construct a `Multikey` verification method.
    pub fn multikey(id: DidUrl, controller: Did, public_key_multibase: String) -> Self {
        VerificationMethod {
            id,
            type_: "Multikey".to_string(),
            controller,
            public_key_multibase: Some(public_key_multibase),
            public_key_jwk: None,
            extra: BTreeMap::new(),
        }
    }
}

/// An entry in a verification relationship: either a reference (DID URL) to a
/// verification method defined elsewhere, or an embedded method (DID 1.1 §5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationRelationship {
    /// A reference to a verification method by DID URL.
    Reference(DidUrl),
    /// An embedded verification method.
    Embedded(Box<VerificationMethod>),
}

/// A service endpoint value (DID 1.1 §5.4): a URI string, a map, or a set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServiceEndpoint {
    /// A single endpoint URI.
    Uri(String),
    /// A structured endpoint object.
    Map(BTreeMap<String, Value>),
    /// A set of endpoints.
    Set(Vec<ServiceEndpoint>),
}

/// A service entry (DID 1.1 §5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// The service identifier (a DID URL).
    pub id: DidUrl,
    /// The service type.
    #[serde(rename = "type")]
    pub type_: String,
    /// The service endpoint(s).
    pub service_endpoint: ServiceEndpoint,
    /// Any other service properties, preserved verbatim. Empty ⇒ nothing emitted.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A W3C DID 1.1 document — the resolvable identity artifact.
///
/// This type is the assembly contract: `did:elohim` resolution in
/// `elohim-storage` populates exactly these fields, and `did:key`/`did:web`
/// resolution parse into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidDocument {
    /// The JSON-LD context.
    #[serde(rename = "@context")]
    pub context: Context,

    /// The DID subject — the identifier this document describes.
    pub id: Did,

    /// The controller(s) of this document. Absent means the subject controls it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<Controller>,

    /// Other identifiers for the same subject (transport ids, alias DIDs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_known_as: Option<Vec<String>>,

    /// The verification methods (keys) defined by this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<Vec<VerificationMethod>>,

    /// Authentication relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Vec<VerificationRelationship>>,

    /// Assertion-method relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertion_method: Option<Vec<VerificationRelationship>>,

    /// Key-agreement relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_agreement: Option<Vec<VerificationRelationship>>,

    /// Capability-invocation relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_invocation: Option<Vec<VerificationRelationship>>,

    /// Capability-delegation relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_delegation: Option<Vec<VerificationRelationship>>,

    /// Service endpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<Service>>,

    /// Any other top-level document properties, preserved verbatim so legal DID
    /// extension properties round-trip. Empty ⇒ nothing emitted.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl DidDocument {
    /// A minimal document with only `@context` and `id` populated.
    pub fn new(context: Context, id: Did) -> Self {
        DidDocument {
            context,
            id,
            controller: None,
            also_known_as: None,
            verification_method: None,
            authentication: None,
            assertion_method: None,
            key_agreement: None,
            capability_invocation: None,
            capability_delegation: None,
            service: None,
            extra: BTreeMap::new(),
        }
    }
}
