//! Parse errors for the DID data model.

use thiserror::Error;

/// Error raised when a string fails DID 1.1 syntax validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DidParseError {
    /// The string did not begin with the required `did:` scheme.
    #[error("missing `did:` scheme prefix")]
    MissingScheme,

    /// The method name was empty or contained characters outside `[a-z0-9]`.
    #[error("invalid method name: {0:?}")]
    InvalidMethod(String),

    /// The method-specific-id was empty or contained an illegal character.
    #[error("invalid method-specific-id: {0}")]
    InvalidMethodSpecificId(String),

    /// A DID URL was expected but the value was not even a syntactically valid DID.
    #[error("invalid DID URL: {0}")]
    InvalidDidUrl(String),
}
