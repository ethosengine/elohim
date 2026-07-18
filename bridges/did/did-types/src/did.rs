//! The `Did` identifier and `DidUrl`, validated against the DID 1.1 ABNF.
//!
//! DID 1.1 syntax (RFC-style ABNF, §3.1):
//! ```text
//! did                = "did:" method-name ":" method-specific-id
//! method-name        = 1*method-char
//! method-char        = %x61-7A / DIGIT            ; lowercase a-z / 0-9
//! method-specific-id = *( *idchar ":" ) 1*idchar
//! idchar             = ALPHA / DIGIT / "." / "-" / "_" / pct-encoded
//! pct-encoded        = "%" HEXDIG HEXDIG
//! ```

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::DidParseError;

/// A parsed, syntactically valid DID identifier.
///
/// Serializes/deserializes as the canonical `did:<method>:<method-specific-id>`
/// string (never as a struct), so it is wire-faithful inside DID documents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Did {
    method: String,
    method_specific_id: String,
}

impl Did {
    /// Parse and validate a DID string per the DID 1.1 ABNF.
    pub fn parse(input: &str) -> Result<Self, DidParseError> {
        let rest = input
            .strip_prefix("did:")
            .ok_or(DidParseError::MissingScheme)?;

        // method-name is up to the next ':'.
        let (method, msi) = rest
            .split_once(':')
            .ok_or_else(|| DidParseError::InvalidMethod(rest.to_string()))?;

        validate_method(method)?;
        validate_method_specific_id(msi)?;

        Ok(Did {
            method: method.to_string(),
            method_specific_id: msi.to_string(),
        })
    }

    /// The DID method name (e.g. `key`, `elohim`, `web`).
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The method-specific identifier (everything after `did:<method>:`).
    pub fn method_specific_id(&self) -> &str {
        &self.method_specific_id
    }

    /// The canonical DID string.
    pub fn as_string(&self) -> String {
        format!("did:{}:{}", self.method, self.method_specific_id)
    }

    /// Build a [`DidUrl`] for this DID with the given fragment (without a leading `#`).
    pub fn with_fragment(&self, fragment: &str) -> DidUrl {
        DidUrl(format!("{}#{}", self.as_string(), fragment))
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "did:{}:{}", self.method, self.method_specific_id)
    }
}

impl FromStr for Did {
    type Err = DidParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Did::parse(s)
    }
}

impl TryFrom<String> for Did {
    type Error = DidParseError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Did::parse(&s)
    }
}

impl From<Did> for String {
    fn from(d: Did) -> String {
        d.as_string()
    }
}

/// A DID URL: a DID plus optional path, query, and/or fragment (DID 1.1 §3.2).
///
/// Verification-relationship references (`did:key:z6Mk…#z6Mk…`) are DID URLs.
/// Stored as the validated string and serialized transparently.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DidUrl(String);

impl DidUrl {
    /// Parse a DID URL: the leading DID portion must be syntactically valid.
    pub fn parse(input: &str) -> Result<Self, DidParseError> {
        // Split off fragment / query / path; validate the DID head only —
        // the tail characters follow generic-URI rules we do not re-police here.
        let head_end = input.find(['#', '?', '/']).unwrap_or(input.len());
        let did_head = &input[..head_end];
        Did::parse(did_head).map_err(|_| DidParseError::InvalidDidUrl(input.to_string()))?;
        Ok(DidUrl(input.to_string()))
    }

    /// The full DID URL string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DidUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DidUrl {
    type Err = DidParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DidUrl::parse(s)
    }
}

impl TryFrom<String> for DidUrl {
    type Error = DidParseError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        DidUrl::parse(&s)
    }
}

impl From<DidUrl> for String {
    fn from(u: DidUrl) -> String {
        u.0
    }
}

impl From<Did> for DidUrl {
    fn from(d: Did) -> DidUrl {
        DidUrl(d.as_string())
    }
}

fn validate_method(method: &str) -> Result<(), DidParseError> {
    if method.is_empty()
        || !method
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return Err(DidParseError::InvalidMethod(method.to_string()));
    }
    Ok(())
}

fn validate_method_specific_id(msi: &str) -> Result<(), DidParseError> {
    if msi.is_empty() {
        return Err(DidParseError::InvalidMethodSpecificId(
            "empty method-specific-id".to_string(),
        ));
    }
    // Per ABNF the final segment is `1*idchar`, so the id may not end with ':'.
    if msi.ends_with(':') {
        return Err(DidParseError::InvalidMethodSpecificId(format!(
            "must not end with ':' (empty final segment): {msi:?}"
        )));
    }

    let bytes = msi.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'%' => {
                // pct-encoded = "%" HEXDIG HEXDIG
                if i + 2 >= bytes.len()
                    || !bytes[i + 1].is_ascii_hexdigit()
                    || !bytes[i + 2].is_ascii_hexdigit()
                {
                    return Err(DidParseError::InvalidMethodSpecificId(format!(
                        "malformed percent-encoding at byte {i} in {msi:?}"
                    )));
                }
                i += 3;
            }
            b':' => i += 1, // segment separator
            _ if b.is_ascii_alphanumeric() => i += 1,
            b'.' | b'-' | b'_' => i += 1,
            _ => {
                return Err(DidParseError::InvalidMethodSpecificId(format!(
                    "illegal character {:?} at byte {i} in {msi:?}",
                    b as char
                )));
            }
        }
    }
    Ok(())
}
