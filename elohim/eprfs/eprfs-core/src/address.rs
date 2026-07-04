use serde::{Deserialize, Serialize};

/// Protocol-level reference to an EPR record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EprRef(String);

impl EprRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for EprRef {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for EprRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Content-addressed blob identifier. The canonical form should be CID-first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlobCid(String);

impl BlobCid {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for BlobCid {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for BlobCid {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable identifier for a local projection of an EPR-backed tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectionId(String);

impl ProjectionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
