//! # did-types
//!
//! The W3C **DID 1.1** data model for the Elohim Protocol's identity bridge —
//! pure types, no I/O. Wire-faithful serde (camelCase, `@context`), so a
//! `DidDocument` produced here is a standards-legible identity artifact any
//! DID-aware consumer can resolve.
//!
//! This crate is the *assembly contract*: `did:elohim` resolution (in
//! `elohim-storage`) conforms to these types rather than inventing its own
//! shape. See `bridges/did/CLAUDE.md`.

pub mod did;
pub mod document;
pub mod error;

#[cfg(test)]
mod tests;

pub use did::{Did, DidUrl};
pub use document::{
    Context, ContextEntry, Controller, DidDocument, Service, ServiceEndpoint, VerificationMethod,
    VerificationRelationship, DID_CONTEXT_V1, DID_CONTEXT_V1_1, MULTIKEY_CONTEXT,
};
pub use error::DidParseError;
