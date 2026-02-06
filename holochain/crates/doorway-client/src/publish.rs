//! Content Publishing Extensions for Doorway
//!
//! Extends doorway-client with content publishing capabilities.
//! Agents can register as content publishers, and doorways discover
//! publishers via DHT to serve content without hardcoded routes.
//!
//! ## Core Concepts
//!
//! 1. **Publishable trait** - Content types that can be served as raw bytes
//! 2. **ContentServer** - DHT entry registering an agent as content publisher
//! 3. **PublishSignal** - Signal to announce publishing availability
//!
//! ## Usage in DNAs
//!
//! ```ignore
//! use doorway_client::{Publishable, ContentServer, emit_publish_signal};
//!
//! // 1. Implement Publishable for blob-like content
//! impl Publishable for Html5AppBlob {
//!     fn content_hash(&self) -> String { self.hash.clone() }
//!     fn content_type(&self) -> &'static str { "html5-app" }
//!     fn mime_type(&self) -> &'static str { "application/zip" }
//! }
//!
//! // 2. Register as content server when storing content
//! #[hdk_extern]
//! fn publish_html5_app(input: PublishInput) -> ExternResult<ActionHash> {
//!     // Store the blob
//!     let hash = create_entry(&input.blob)?;
//!
//!     // Register as publisher
//!     let server = ContentServer::new(
//!         input.blob.content_hash(),
//!         ContentServerCapability::Html5App,
//!     );
//!     create_entry(&server)?;
//!
//!     // Signal to doorway
//!     emit_publish_signal(PublishSignal::online(&server))?;
//!
//!     Ok(hash)
//! }
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Publishable Trait - Content that can be served as raw bytes
// =============================================================================

/// Trait for content types that can be published and served by doorway.
///
/// Unlike `Cacheable` (which caches JSON API responses), `Publishable`
/// content is served as raw bytes - files, media, zip archives, etc.
pub trait Publishable {
    /// The content-addressed hash of this content (e.g., "sha256-abc123")
    fn content_hash(&self) -> String;

    /// The content type category (e.g., "html5-app", "media", "document")
    fn content_type(&self) -> &'static str;

    /// MIME type for HTTP Content-Type header
    fn mime_type(&self) -> &'static str;

    /// Size in bytes (for Content-Length header)
    fn size_bytes(&self) -> u64;

    /// Whether this content requires authentication to access
    fn requires_auth(&self) -> bool {
        false
    }

    /// Reach level for access control (e.g., "commons", "local", "private")
    fn reach(&self) -> &str {
        "commons"
    }

    /// Optional: entry point for compound content (e.g., "index.html" for html5-app)
    fn entry_point(&self) -> Option<&str> {
        None
    }
}

// =============================================================================
// ContentServer - DHT Entry for Publisher Registration
// =============================================================================

/// A DHT entry that registers an agent as a content publisher.
///
/// When an agent stores content and wants to serve it, they create
/// a ContentServer entry. Doorways query the DHT for these entries
/// to discover who can serve specific content hashes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentServer {
    /// Content hash this server can provide (e.g., "sha256-abc123")
    pub content_hash: String,

    /// What type of content serving this server supports
    pub capability: ContentServerCapability,

    /// URL where this server accepts content requests
    /// For Holochain agents, this is their doorway URL
    /// For external servers, this is their direct URL
    pub serve_url: Option<String>,

    /// Whether this server is currently online and serving
    pub online: bool,

    /// Server priority (higher = preferred)
    /// Used for load balancing and failover
    pub priority: u8,

    /// Geographic region for latency-based routing
    pub region: Option<String>,

    /// Bandwidth capacity in Mbps (self-reported)
    pub bandwidth_mbps: Option<u32>,

    /// Unix timestamp when this registration was created
    pub registered_at: u64,

    /// Unix timestamp of last heartbeat (updated periodically)
    pub last_heartbeat: u64,
}

/// Content serving capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentServerCapability {
    /// Serve raw blob content (GET /store/{hash})
    Blob,

    /// Serve HTML5 app from zip (GET /apps/{id}/{path})
    /// Doorway extracts files from cached zip on demand
    Html5App,

    /// Serve media with streaming/range requests (GET /media/{hash})
    MediaStream,

    /// Serve SCORM/xAPI learning packages
    LearningPackage,

    /// Custom capability with type name
    Custom(String),
}

impl ContentServer {
    /// Create a new content server registration
    pub fn new(content_hash: impl Into<String>, capability: ContentServerCapability) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            content_hash: content_hash.into(),
            capability,
            serve_url: None,
            online: true,
            priority: 50, // Default middle priority
            region: None,
            bandwidth_mbps: None,
            registered_at: now,
            last_heartbeat: now,
        }
    }

    /// Set the serve URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.serve_url = Some(url.into());
        self
    }

    /// Set the region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set bandwidth capacity
    pub fn with_bandwidth(mut self, mbps: u32) -> Self {
        self.bandwidth_mbps = Some(mbps);
        self
    }

    /// Set priority (0-100, higher = preferred)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(100);
        self
    }

    /// Mark as offline
    pub fn offline(mut self) -> Self {
        self.online = false;
        self
    }

    /// Update heartbeat timestamp
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if this server is stale (no heartbeat for given seconds)
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.last_heartbeat > max_age_secs
    }
}

// =============================================================================
// PublishSignal - Announce Publishing Availability
// =============================================================================

/// Signal type for publish events sent via post_commit.
///
/// Doorway subscribes to these signals to update its publisher registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublishSignal {
    /// Signal type
    pub signal_type: PublishSignalType,

    /// Content hash being published/unpublished
    pub content_hash: String,

    /// Content server details (for online signals)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ContentServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PublishSignalType {
    /// Agent is online and serving this content
    Online,

    /// Agent is going offline, stop routing to them
    Offline,

    /// Heartbeat - still alive and serving
    Heartbeat,

    /// Content removed - no longer available from this agent
    Removed,
}

impl PublishSignal {
    /// Create an online signal (agent is serving content)
    pub fn online(server: &ContentServer) -> Self {
        Self {
            signal_type: PublishSignalType::Online,
            content_hash: server.content_hash.clone(),
            server: Some(server.clone()),
        }
    }

    /// Create an offline signal (agent stopping service)
    pub fn offline(content_hash: impl Into<String>) -> Self {
        Self {
            signal_type: PublishSignalType::Offline,
            content_hash: content_hash.into(),
            server: None,
        }
    }

    /// Create a heartbeat signal
    pub fn heartbeat(content_hash: impl Into<String>) -> Self {
        Self {
            signal_type: PublishSignalType::Heartbeat,
            content_hash: content_hash.into(),
            server: None,
        }
    }

    /// Create a removed signal (content deleted)
    pub fn removed(content_hash: impl Into<String>) -> Self {
        Self {
            signal_type: PublishSignalType::Removed,
            content_hash: content_hash.into(),
            server: None,
        }
    }
}

/// Wrapper for emitting publish signals in a consistent format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayPublishSignal {
    /// Signal namespace - "doorway.publish"
    pub namespace: String,
    /// The publish signal payload
    pub payload: PublishSignal,
}

impl DoorwayPublishSignal {
    pub fn new(signal: PublishSignal) -> Self {
        Self {
            namespace: "doorway.publish".to_string(),
            payload: signal,
        }
    }

    /// Convert to bytes for emit_signal
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

// =============================================================================
// Html5AppBundle - Specific type for HTML5 app publishing
// =============================================================================

/// Metadata for an HTML5 app bundle (zip file).
///
/// This is stored alongside the blob to provide app-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Html5AppBundle {
    /// Content hash of the zip file
    pub hash: String,

    /// App identifier (used in URL: /apps/{app_id}/...)
    pub app_id: String,

    /// Entry point file within the zip (default: "index.html")
    pub entry_point: String,

    /// Size of the zip in bytes
    pub size_bytes: u64,

    /// List of files in the zip (for validation/discovery)
    pub files: Vec<String>,

    /// Optional manifest from within the zip
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Html5AppManifest>,
}

/// Manifest file (elohim-app.json) that can be included in the zip
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Html5AppManifest {
    /// App name
    pub name: String,

    /// App version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Entry point override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<String>,

    /// Author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<AuthorInfo>,

    /// License
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Content Security Policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csp: Option<String>,

    /// Iframe sandbox flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthorInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Publishable for Html5AppBundle {
    fn content_hash(&self) -> String {
        self.hash.clone()
    }

    fn content_type(&self) -> &'static str {
        "html5-app"
    }

    fn mime_type(&self) -> &'static str {
        "application/zip"
    }

    fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    fn entry_point(&self) -> Option<&str> {
        Some(&self.entry_point)
    }
}

// =============================================================================
// Publisher Registry Query Types
// =============================================================================

/// Query input for finding content publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersInput {
    /// Content hash to find publishers for
    pub content_hash: String,

    /// Optional: filter by capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<ContentServerCapability>,

    /// Optional: prefer publishers in this region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_region: Option<String>,

    /// Maximum number of publishers to return
    #[serde(default = "default_max_publishers")]
    pub limit: usize,

    /// Only return online publishers
    #[serde(default = "default_true")]
    pub online_only: bool,
}

fn default_max_publishers() -> usize {
    10
}

fn default_true() -> bool {
    true
}

/// Result from finding publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersOutput {
    /// Content hash queried
    pub content_hash: String,

    /// Found publishers, sorted by priority/latency
    pub publishers: Vec<ContentServer>,

    /// Whether more publishers exist (pagination)
    pub has_more: bool,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_server_creation() {
        let server = ContentServer::new("sha256-abc123", ContentServerCapability::Html5App)
            .with_url("https://doorway.example.com")
            .with_region("us-west")
            .with_bandwidth(100)
            .with_priority(75);

        assert_eq!(server.content_hash, "sha256-abc123");
        assert_eq!(server.capability, ContentServerCapability::Html5App);
        assert_eq!(server.serve_url, Some("https://doorway.example.com".to_string()));
        assert_eq!(server.region, Some("us-west".to_string()));
        assert_eq!(server.bandwidth_mbps, Some(100));
        assert_eq!(server.priority, 75);
        assert!(server.online);
    }

    #[test]
    fn test_publish_signal_serialization() {
        let server = ContentServer::new("sha256-abc123", ContentServerCapability::Html5App);
        let signal = PublishSignal::online(&server);

        let json = serde_json::to_string(&signal).unwrap();
        let deserialized: PublishSignal = serde_json::from_str(&json).unwrap();

        assert_eq!(signal, deserialized);
    }

    #[test]
    fn test_html5_app_bundle_publishable() {
        let bundle = Html5AppBundle {
            hash: "sha256-abc123".to_string(),
            app_id: "evolution-of-trust".to_string(),
            entry_point: "index.html".to_string(),
            size_bytes: 6_800_000,
            files: vec!["index.html".to_string(), "js/main.js".to_string()],
            manifest: None,
        };

        assert_eq!(bundle.content_hash(), "sha256-abc123");
        assert_eq!(bundle.content_type(), "html5-app");
        assert_eq!(bundle.mime_type(), "application/zip");
        assert_eq!(bundle.entry_point(), Some("index.html"));
    }

    #[test]
    fn test_stale_detection() {
        let mut server = ContentServer::new("sha256-abc123", ContentServerCapability::Blob);

        // Fresh server should not be stale
        assert!(!server.is_stale(60));

        // Manually set old timestamp
        server.last_heartbeat = 0;
        assert!(server.is_stale(60));
    }
}
