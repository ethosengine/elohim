//! # did-bridge
//!
//! W3C DID resolution for the Elohim Protocol. The [`DidResolver`] trait is the
//! IoC seam; DID methods plug in as implementations and route through a
//! [`MethodRegistry`]:
//!
//! - [`DidKeyResolver`] — `did:key`, fully offline. Every `AgentPubKey` gains a
//!   standards-legible DID for free via the [`codec`].
//! - [`ElohimResolver`] — `did:elohim:<agent_cid>`, projection-assembled from an
//!   [`ElohimIdentityStore`] (the contract `elohim-storage` implements).
//! - [`DidWebResolver`] — `did:web`, feature-gated (`web-resolver`) over an
//!   injected [`DidWebFetch`].
//!
//! Documents are a **projection of substrate truth, never truth itself** (P1).
//! See `bridges/did/CLAUDE.md` and the design spec.
//!
//! ## Usage
//!
//! ```
//! use did_bridge::{DidKeyResolver, MethodRegistry};
//! use did_types::Did;
//!
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = MethodRegistry::new().with(Box::new(DidKeyResolver::new()));
//! let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr")?;
//! let result = registry.resolve(&did).await?;
//! assert!(result.did_document.is_some());
//! # Ok(())
//! # }
//! ```

pub mod codec;
pub mod did_elohim;
pub mod did_key;
pub mod registry;
pub mod resolver;

#[cfg(feature = "web-resolver")]
pub mod did_web;

pub use codec::{
    agent_cid_to_core32, agent_cid_to_did_key, core32_to_agent_cid, core32_to_did_key,
    core32_to_multikey, dht_location_bytes, did_key_to_agent_cid, did_key_to_core32, CodecError,
};
pub use did_elohim::{ElohimIdentityStore, ElohimResolver, ElohimStoreError, ServiceRef};
pub use did_key::{assemble_did_key_document, DidKeyResolver};
pub use registry::MethodRegistry;
pub use resolver::{
    DidDocumentMetadata, DidResolutionError, DidResolutionMetadata, DidResolutionResult,
    DidResolver, DID_DOCUMENT_CONTENT_TYPE, ERR_INTERNAL, ERR_INVALID_DID,
    ERR_METHOD_NOT_SUPPORTED, ERR_NOT_FOUND,
};

#[cfg(feature = "web-resolver")]
pub use did_web::{derive_did_web_url, DidWebFetch, DidWebResolver};
