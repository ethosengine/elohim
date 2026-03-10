//! Route Registration Protocol
//!
//! DNAs declare routes they want doorway to expose via `__doorway_routes`.
//! This replaces hard-coded routes in doorway with dynamic, DNA-driven configuration.
//!
//! ## Architecture
//!
//! ```text
//! DNA declares routes via __doorway_routes
//!                │
//!                ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  DOORWAY (auto-discovers and registers routes)                      │
//! │                                                                     │
//! │  For each DNA:                                                      │
//! │  1. Call __doorway_routes to get DoorwayRoutes                      │
//! │  2. Register HTTP routes based on config                            │
//! │  3. Proxy requests to agent's conductor/elohim-storage              │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Zome Contract
//!
//! ```rust,ignore
//! #[hdk_extern]
//! pub fn __doorway_routes(_: ()) -> ExternResult<DoorwayRoutes> {
//!     Ok(DoorwayRoutes {
//!         version: 1,
//!         routes: vec![
//!             Route::get("/api/content/{id}")
//!                 .handler("get_content")
//!                 .cache_ttl(3600)
//!                 .public_if_reach("commons"),
//!             Route::post("/api/content")
//!                 .handler("create_content")
//!                 .auth_required(),
//!         ],
//!         blob_proxy: Some(BlobProxyConfig {
//!             enabled: true,
//!             base_path: "/store",
//!             // Agent's elohim-storage is authoritative, doorway caches
//!         }),
//!         agent_endpoint: None, // Auto-detect from conductor connection
//!     })
//! }
//! ```
//!
//! ## External Agent Registration
//!
//! For agents on devices that can't run doorway (IoT, mobile, etc.):
//!
//! ```rust,ignore
//! // Agent registers via API call to doorway
//! POST /doorway/register
//! {
//!     "agent_pubkey": "uhCAk...",
//!     "endpoint": "https://my-device.local:8080",
//!     "capabilities": ["content", "blobs"],
//!     "signature": "..." // Prove ownership of agent key
//! }
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Constants
// =============================================================================

/// The standard function name for route introspection
pub const ROUTES_FN: &str = "__doorway_routes";

/// Current version of the routes protocol
pub const ROUTES_PROTOCOL_VERSION: u32 = 1;

// =============================================================================
// DoorwayRoutes - Top-level route configuration from DNA
// =============================================================================

/// Route configuration declared by a DNA.
///
/// Doorway calls `__doorway_routes()` on startup to discover what routes
/// this DNA wants exposed via the web2 gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoorwayRoutes {
    /// Protocol version (for future compatibility)
    #[serde(default = "default_version")]
    pub version: u32,

    /// HTTP routes to register
    #[serde(default)]
    pub routes: Vec<Route>,

    /// Blob proxy configuration (for /store/{hash} style endpoints)
    #[serde(default)]
    pub blob_proxy: Option<BlobProxyConfig>,

    /// Stream proxy configuration (for /stream/{id} style endpoints)
    #[serde(default)]
    pub stream_proxy: Option<StreamProxyConfig>,

    /// Agent endpoint override (default: auto-detect from conductor)
    /// Use this if agent's elohim-storage is on a different endpoint
    #[serde(default)]
    pub agent_endpoint: Option<String>,

    /// Whether this DNA requires doorway services
    /// If false, doorway won't error if it can't reach the agent
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_version() -> u32 {
    ROUTES_PROTOCOL_VERSION
}

fn default_true() -> bool {
    true
}

impl Default for DoorwayRoutes {
    fn default() -> Self {
        Self {
            version: ROUTES_PROTOCOL_VERSION,
            routes: Vec::new(),
            blob_proxy: None,
            stream_proxy: None,
            agent_endpoint: None,
            required: true,
        }
    }
}

impl DoorwayRoutes {
    /// Create empty routes (no doorway exposure)
    pub fn none() -> Self {
        Self {
            required: false,
            ..Default::default()
        }
    }

    /// Create routes with just blob proxy
    pub fn blobs_only(base_path: &str) -> Self {
        Self {
            blob_proxy: Some(BlobProxyConfig::new(base_path)),
            ..Default::default()
        }
    }
}

// =============================================================================
// Route - Individual HTTP route
// =============================================================================

/// A single HTTP route to register with doorway
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Route {
    /// HTTP method (GET, POST, PUT, DELETE)
    pub method: HttpMethod,

    /// Path pattern (e.g., "/api/content/{id}")
    /// Supports {param} placeholders
    pub path: String,

    /// Zome function to call
    pub handler: String,

    /// Whether authentication is required
    #[serde(default)]
    pub auth_required: bool,

    /// Cache TTL in seconds (0 = no caching)
    #[serde(default)]
    pub cache_ttl_secs: u64,

    /// Public if response.{field} == {value}
    #[serde(default)]
    pub public_if_reach: Option<ReachCondition>,

    /// Rate limit (requests per minute, 0 = no limit)
    #[serde(default)]
    pub rate_limit_rpm: u32,

    /// Description for documentation
    #[serde(default)]
    pub description: Option<String>,
}

impl Route {
    /// Create a GET route
    pub fn get(path: &str) -> RouteBuilder {
        RouteBuilder::new(HttpMethod::Get, path)
    }

    /// Create a POST route
    pub fn post(path: &str) -> RouteBuilder {
        RouteBuilder::new(HttpMethod::Post, path)
    }

    /// Create a PUT route
    pub fn put(path: &str) -> RouteBuilder {
        RouteBuilder::new(HttpMethod::Put, path)
    }

    /// Create a DELETE route
    pub fn delete(path: &str) -> RouteBuilder {
        RouteBuilder::new(HttpMethod::Delete, path)
    }
}

/// HTTP methods supported by route registration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// Condition for reach-based public access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReachCondition {
    /// Field path in response to check (e.g., "reach")
    pub field: String,
    /// Value that makes it public (e.g., "commons")
    pub value: String,
}

// =============================================================================
// RouteBuilder - Fluent API for building routes
// =============================================================================

/// Builder for constructing routes with a fluent API
#[derive(Debug, Clone)]
pub struct RouteBuilder {
    route: Route,
}

impl RouteBuilder {
    pub fn new(method: HttpMethod, path: &str) -> Self {
        Self {
            route: Route {
                method,
                path: path.to_string(),
                handler: String::new(),
                auth_required: false,
                cache_ttl_secs: 0,
                public_if_reach: None,
                rate_limit_rpm: 0,
                description: None,
            },
        }
    }

    /// Set the zome function handler
    pub fn handler(mut self, fn_name: &str) -> Self {
        self.route.handler = fn_name.to_string();
        self
    }

    /// Require authentication
    pub fn auth_required(mut self) -> Self {
        self.route.auth_required = true;
        self
    }

    /// Set cache TTL in seconds
    pub fn cache_ttl(mut self, seconds: u64) -> Self {
        self.route.cache_ttl_secs = seconds;
        self
    }

    /// Public if response.reach == value
    pub fn public_if_reach(mut self, value: &str) -> Self {
        self.route.public_if_reach = Some(ReachCondition {
            field: "reach".to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Public if response.{field} == value
    pub fn public_if(mut self, field: &str, value: &str) -> Self {
        self.route.public_if_reach = Some(ReachCondition {
            field: field.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Set rate limit (requests per minute)
    pub fn rate_limit(mut self, rpm: u32) -> Self {
        self.route.rate_limit_rpm = rpm;
        self
    }

    /// Add description
    pub fn description(mut self, desc: &str) -> Self {
        self.route.description = Some(desc.to_string());
        self
    }

    /// Build the route
    pub fn build(self) -> Route {
        self.route
    }
}

// =============================================================================
// BlobProxyConfig - Blob serving configuration
// =============================================================================

/// Configuration for blob proxy (serving content from elohim-storage)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlobProxyConfig {
    /// Whether blob proxy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Base path for blob routes (e.g., "/store")
    /// Doorway will serve: GET {base_path}/{hash}
    #[serde(default = "default_blob_path")]
    pub base_path: String,

    /// Whether doorway should cache blobs (CDN mode)
    /// Agent's elohim-storage remains authoritative
    #[serde(default = "default_true")]
    pub cache_enabled: bool,

    /// Cache TTL in seconds (blobs are immutable, so can be long)
    #[serde(default = "default_blob_cache_ttl")]
    pub cache_ttl_secs: u64,

    /// Maximum blob size to cache (bytes)
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size_bytes: u64,

    /// Whether to support range requests (for streaming)
    #[serde(default = "default_true")]
    pub range_requests: bool,
}

fn default_blob_path() -> String {
    "/store".to_string()
}

fn default_blob_cache_ttl() -> u64 {
    86400 // 24 hours (blobs are immutable)
}

fn default_max_cache_size() -> u64 {
    100 * 1024 * 1024 // 100 MB
}

impl Default for BlobProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_path: default_blob_path(),
            cache_enabled: true,
            cache_ttl_secs: default_blob_cache_ttl(),
            max_cache_size_bytes: default_max_cache_size(),
            range_requests: true,
        }
    }
}

impl BlobProxyConfig {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            ..Default::default()
        }
    }

    /// Disable caching (doorway just proxies)
    pub fn no_cache(mut self) -> Self {
        self.cache_enabled = false;
        self
    }
}

// =============================================================================
// StreamProxyConfig - Media streaming configuration
// =============================================================================

/// Configuration for stream proxy (HLS/DASH media streaming)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamProxyConfig {
    /// Whether stream proxy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Base path for stream routes (e.g., "/stream")
    #[serde(default = "default_stream_path")]
    pub base_path: String,

    /// Supported formats
    #[serde(default = "default_stream_formats")]
    pub formats: Vec<StreamFormat>,
}

fn default_stream_path() -> String {
    "/stream".to_string()
}

fn default_stream_formats() -> Vec<StreamFormat> {
    vec![StreamFormat::Hls, StreamFormat::Dash]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StreamFormat {
    Hls,
    Dash,
}

impl Default for StreamProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_path: default_stream_path(),
            formats: default_stream_formats(),
        }
    }
}

// =============================================================================
// AgentRegistration - External agent registration
// =============================================================================

/// Registration request from an external agent.
///
/// For agents on devices that can't run doorway (IoT, mobile, constrained),
/// they can register their endpoint with a doorway instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRegistration {
    /// Agent's public key (base64)
    pub agent_pubkey: String,

    /// Agent's HTTP endpoint (where doorway should proxy requests)
    pub endpoint: String,

    /// Capabilities this agent supports
    #[serde(default)]
    pub capabilities: Vec<AgentCapability>,

    /// Signature proving ownership of agent key
    /// Signs: "{agent_pubkey}:{endpoint}:{timestamp}"
    pub signature: String,

    /// Timestamp of signature (for replay protection)
    pub timestamp: u64,

    /// Optional: Routes this agent wants exposed (overrides DNA discovery)
    #[serde(default)]
    pub routes: Option<DoorwayRoutes>,

    /// TTL for this registration (seconds, 0 = permanent until revoked)
    #[serde(default)]
    pub ttl_secs: u64,
}

/// Capabilities an agent can declare for registration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    /// Can serve content via zome calls
    Content,
    /// Can serve blobs from elohim-storage
    Blobs,
    /// Can serve media streams
    Streaming,
    /// Can accept bulk imports
    Import,
    /// Custom capability
    Custom(String),
}

/// Response to agent registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    /// Whether registration succeeded
    pub success: bool,

    /// Registration ID (for updating/revoking)
    #[serde(default)]
    pub registration_id: Option<String>,

    /// Base URL for this agent's routes
    #[serde(default)]
    pub base_url: Option<String>,

    /// Error message if failed
    #[serde(default)]
    pub error: Option<String>,

    /// When registration expires (unix timestamp)
    #[serde(default)]
    pub expires_at: Option<u64>,
}

// =============================================================================
// DoorwayRoutesBuilder - Fluent API for building routes config
// =============================================================================

/// Builder for DoorwayRoutes
#[derive(Debug, Clone, Default)]
pub struct DoorwayRoutesBuilder {
    config: DoorwayRoutes,
}

impl DoorwayRoutesBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a route
    pub fn route(mut self, route: Route) -> Self {
        self.config.routes.push(route);
        self
    }

    /// Enable blob proxy with default config
    pub fn with_blobs(mut self) -> Self {
        self.config.blob_proxy = Some(BlobProxyConfig::default());
        self
    }

    /// Enable blob proxy with custom base path
    pub fn with_blobs_at(mut self, base_path: &str) -> Self {
        self.config.blob_proxy = Some(BlobProxyConfig::new(base_path));
        self
    }

    /// Enable stream proxy
    pub fn with_streaming(mut self) -> Self {
        self.config.stream_proxy = Some(StreamProxyConfig::default());
        self
    }

    /// Set agent endpoint override
    pub fn agent_endpoint(mut self, endpoint: &str) -> Self {
        self.config.agent_endpoint = Some(endpoint.to_string());
        self
    }

    /// Mark as optional (doorway won't error if unreachable)
    pub fn optional(mut self) -> Self {
        self.config.required = false;
        self
    }

    /// Build the config
    pub fn build(self) -> DoorwayRoutes {
        self.config
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_builder() {
        let route = Route::get("/api/content/{id}")
            .handler("get_content")
            .cache_ttl(3600)
            .public_if_reach("commons")
            .build();

        assert_eq!(route.method, HttpMethod::Get);
        assert_eq!(route.path, "/api/content/{id}");
        assert_eq!(route.handler, "get_content");
        assert_eq!(route.cache_ttl_secs, 3600);
        assert!(route.public_if_reach.is_some());
    }

    #[test]
    fn test_doorway_routes_builder() {
        let routes = DoorwayRoutesBuilder::new()
            .route(Route::get("/api/content/{id}")
                .handler("get_content")
                .cache_ttl(3600)
                .build())
            .route(Route::post("/api/content")
                .handler("create_content")
                .auth_required()
                .build())
            .with_blobs()
            .build();

        assert_eq!(routes.routes.len(), 2);
        assert!(routes.blob_proxy.is_some());
    }

    #[test]
    fn test_serialization() {
        let routes = DoorwayRoutesBuilder::new()
            .route(Route::get("/api/test").handler("test").build())
            .with_blobs()
            .build();

        let json = serde_json::to_string(&routes).unwrap();
        let deserialized: DoorwayRoutes = serde_json::from_str(&json).unwrap();

        assert_eq!(routes, deserialized);
    }

    #[test]
    fn test_blob_proxy_defaults() {
        let config = BlobProxyConfig::default();

        assert!(config.enabled);
        assert_eq!(config.base_path, "/store");
        assert!(config.cache_enabled);
        assert!(config.range_requests);
    }

    #[test]
    fn test_agent_registration() {
        let reg = AgentRegistration {
            agent_pubkey: "uhCAk...".to_string(),
            endpoint: "https://my-device.local:8080".to_string(),
            capabilities: vec![AgentCapability::Content, AgentCapability::Blobs],
            signature: "sig...".to_string(),
            timestamp: 1234567890,
            routes: None,
            ttl_secs: 3600,
        };

        let json = serde_json::to_string(&reg).unwrap();
        assert!(json.contains("agent_pubkey"));
        assert!(json.contains("capabilities"));
    }
}
