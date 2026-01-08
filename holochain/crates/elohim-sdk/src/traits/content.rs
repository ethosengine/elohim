//! Content traits for read/write operations

use crate::error::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for content types that can be read from the content store.
///
/// Implement this trait for your content types to enable reading
/// through the ContentClient.
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::{ContentReadable, ContentClient, Result};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct Content {
///     id: String,
///     title: String,
///     description: String,
/// }
///
/// impl ContentReadable for Content {
///     fn content_type() -> &'static str { "content" }
///     fn content_id(&self) -> &str { &self.id }
/// }
///
/// // Now you can use it with ContentClient
/// let content = client.get::<Content>("manifesto").await?;
/// ```
#[async_trait]
pub trait ContentReadable: Sized + Send + Sync + Serialize + DeserializeOwned {
    /// The content type identifier (e.g., "content", "path", "blob")
    /// This maps to the /db/{type}/* endpoints in elohim-storage
    fn content_type() -> &'static str;

    /// The unique ID of this content instance
    fn content_id(&self) -> &str;

    /// Optional: reach level for access control
    /// Defaults to "commons" (publicly accessible)
    fn reach(&self) -> &str {
        "commons"
    }

    /// Optional: whether this content is cacheable
    /// Defaults to true
    fn is_cacheable() -> bool {
        true
    }

    /// Optional: cache TTL in seconds
    /// Defaults to 3600 (1 hour)
    fn cache_ttl() -> u64 {
        3600
    }
}

/// Trait for content types that can be written to the content store.
///
/// Extends ContentReadable with write capabilities.
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::{ContentWriteable, ContentClient, Result};
///
/// impl ContentWriteable for Content {
///     fn validate(&self) -> Result<()> {
///         if self.title.is_empty() {
///             return Err(SdkError::Validation("title required".into()));
///         }
///         Ok(())
///     }
/// }
///
/// // Now you can save content
/// client.save(&content).await?;
/// ```
#[async_trait]
pub trait ContentWriteable: ContentReadable {
    /// Validate the content before writing
    /// Override to add custom validation
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Convert to JSON for storage
    fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

/// Query options for batch content retrieval
#[derive(Debug, Clone, Default)]
pub struct ContentQuery {
    /// Filter by content type
    pub content_type: Option<String>,
    /// Filter by tags (AND logic)
    pub tags: Vec<String>,
    /// Search in title/description
    pub search: Option<String>,
    /// Maximum results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

impl ContentQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }
}
