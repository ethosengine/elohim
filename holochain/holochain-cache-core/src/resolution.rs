//! Content Resolution - Unified tiered content source resolution
//!
//! Provides O(1) content resolution with learning from history.
//! Compiled to WASM for browser use, native for edgenode/doorway.
//!
//! # Tiers (in resolution priority order)
//!
//! 1. **Local** - IndexedDB, in-memory (fastest, offline-capable)
//! 2. **Projection** - Doorway's MongoDB cache (fast, eventually consistent)
//! 3. **Authoritative** - Conductor → Edgenode → DHT (slow, source of truth)
//! 4. **External** - Fallback URLs (last resort)
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! const resolver = new ContentResolver();
//!
//! // Register sources
//! resolver.register_source('indexeddb', SourceTier.Local, 100, '["path","content"]', null);
//! resolver.register_source('projection', SourceTier.Projection, 80, '["path","content"]', 'https://doorway.example.com');
//! resolver.register_source('conductor', SourceTier.Authoritative, 50, '["path","content","blob"]', null);
//!
//! // Resolve content
//! const result = JSON.parse(resolver.resolve('content', 'my-content-id'));
//! // { source_id: 'indexeddb', tier: 0, url: null, cached: false }
//!
//! // After successful fetch, record location for future resolutions
//! resolver.record_content_location('my-content-id', 'indexeddb');
//!
//! // Next resolution will prefer known location
//! const result2 = JSON.parse(resolver.resolve('content', 'my-content-id'));
//! // { source_id: 'indexeddb', tier: 0, url: null, cached: true }
//! ```

use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

use crate::current_time_ms;

// =============================================================================
// Source Tier - Priority ordering for content sources
// =============================================================================

/// Content source tiers, in resolution priority order.
///
/// Lower numeric value = higher priority (tried first).
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum SourceTier {
    /// Local storage (IndexedDB, in-memory) - fastest, offline-capable
    Local = 0,
    /// Projection cache (Doorway's MongoDB) - fast, eventually consistent
    Projection = 1,
    /// Authoritative source (Conductor → Edgenode → DHT) - slow, source of truth
    Authoritative = 2,
    /// External fallback (URLs outside the network) - last resort
    External = 3,
}

impl Default for SourceTier {
    fn default() -> Self {
        SourceTier::Authoritative
    }
}

// =============================================================================
// Content Source - A registered source that can provide content
// =============================================================================

/// A registered content source.
///
/// Sources are tried in order of (tier, priority desc).
/// Higher priority within a tier means tried first.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentSource {
    /// Unique identifier (e.g., "indexeddb", "projection", "conductor")
    id: String,
    /// Which tier this source belongs to
    tier: SourceTier,
    /// Priority within tier (0-100, higher = preferred)
    priority: u8,
    /// What content types this source provides (e.g., ["path", "content", "blob"])
    content_types: Vec<String>,
    /// Whether this source is currently available
    available: bool,
    /// Base URL for URL-based sources (e.g., doorway URL)
    base_url: Option<String>,
}

// =============================================================================
// App Registration - Metadata for HTML5 apps
// =============================================================================

/// Registration info for an HTML5 app.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppRegistration {
    /// Content hash of the zip blob
    blob_hash: String,
    /// Entry point file within the zip (e.g., "index.html")
    entry_point: String,
    /// Fallback URL if resolution fails
    fallback_url: Option<String>,
    /// When this app was registered
    registered_at: u64,
}

// =============================================================================
// Resolution Result - Returned to caller
// =============================================================================

/// Result of content resolution.
///
/// Tells the caller which source to try and whether it was found in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Which source to try
    pub source_id: String,
    /// Tier of the source
    pub tier: u8,
    /// URL if this is a URL-based source
    pub url: Option<String>,
    /// Whether this came from content index (previously found here)
    pub cached: bool,
}

/// Error result when resolution fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionError {
    pub error: String,
    pub content_type: String,
    pub content_id: String,
}

// =============================================================================
// Content Resolver - Main resolution engine
// =============================================================================

/// Unified content resolver with tiered fallback.
///
/// Maintains:
/// - Registered sources ordered by (tier, priority)
/// - Content index mapping content IDs to known locations
/// - App registry for HTML5 app resolution
///
/// # Performance
///
/// - Source lookup: O(n) where n = number of sources (typically < 10)
/// - Content index lookup: O(1) hash map
/// - Memory: ~100 bytes per indexed content item
#[wasm_bindgen]
pub struct ContentResolver {
    /// Registered sources, sorted by (tier, priority desc)
    sources: Vec<ContentSource>,
    /// Known content locations: content_id -> Vec<(source_id, last_seen_ms)>
    content_index: HashMap<String, Vec<(String, u64)>>,
    /// HTML5 app registry: app_id -> AppRegistration
    app_registry: HashMap<String, AppRegistration>,
    /// Statistics
    resolution_count: u64,
    cache_hit_count: u64,
}

#[wasm_bindgen]
impl ContentResolver {
    /// Create a new content resolver.
    #[wasm_bindgen(constructor)]
    pub fn new() -> ContentResolver {
        ContentResolver {
            sources: Vec::new(),
            content_index: HashMap::new(),
            app_registry: HashMap::new(),
            resolution_count: 0,
            cache_hit_count: 0,
        }
    }

    /// Register a content source.
    ///
    /// # Arguments
    /// * `id` - Unique source identifier (e.g., "indexeddb", "projection")
    /// * `tier` - Source tier (Local, Projection, Authoritative, External)
    /// * `priority` - Priority within tier (0-100, higher = preferred)
    /// * `content_types_json` - JSON array of content types this source provides
    /// * `base_url` - Optional base URL for URL-based sources
    ///
    /// # Example
    /// ```javascript
    /// resolver.register_source(
    ///   'projection',
    ///   SourceTier.Projection,
    ///   80,
    ///   '["path", "content", "human"]',
    ///   'https://doorway.example.com'
    /// );
    /// ```
    #[wasm_bindgen]
    pub fn register_source(
        &mut self,
        id: String,
        tier: SourceTier,
        priority: u8,
        content_types_json: &str,
        base_url: Option<String>,
    ) {
        // Remove existing source with same ID
        self.sources.retain(|s| s.id != id);

        let content_types: Vec<String> = serde_json::from_str(content_types_json)
            .unwrap_or_default();

        self.sources.push(ContentSource {
            id,
            tier,
            priority: priority.min(100),
            content_types,
            available: true,
            base_url,
        });

        // Keep sorted by (tier asc, priority desc)
        self.sources.sort_by(|a, b| {
            match a.tier.cmp(&b.tier) {
                std::cmp::Ordering::Equal => b.priority.cmp(&a.priority),
                other => other,
            }
        });
    }

    /// Update a source's base URL.
    #[wasm_bindgen]
    pub fn set_source_url(&mut self, source_id: &str, base_url: Option<String>) {
        if let Some(source) = self.sources.iter_mut().find(|s| s.id == source_id) {
            source.base_url = base_url;
        }
    }

    /// Mark a source as available or unavailable.
    ///
    /// Unavailable sources are skipped during resolution.
    #[wasm_bindgen]
    pub fn set_source_available(&mut self, source_id: &str, available: bool) {
        if let Some(source) = self.sources.iter_mut().find(|s| s.id == source_id) {
            source.available = available;
        }
    }

    /// Check if a source is available.
    #[wasm_bindgen]
    pub fn is_source_available(&self, source_id: &str) -> bool {
        self.sources
            .iter()
            .find(|s| s.id == source_id)
            .map(|s| s.available)
            .unwrap_or(false)
    }

    /// Record that content was successfully found at a source.
    ///
    /// This updates the content index so future resolutions can prefer
    /// sources where content was previously found (cache hits).
    #[wasm_bindgen]
    pub fn record_content_location(&mut self, content_id: String, source_id: String) {
        let now = current_time_ms();
        let locations = self.content_index.entry(content_id).or_default();

        // Update existing or add new
        if let Some(loc) = locations.iter_mut().find(|(s, _)| s == &source_id) {
            loc.1 = now;
        } else {
            locations.push((source_id, now));
        }
    }

    /// Remove a content location (e.g., after cache eviction).
    #[wasm_bindgen]
    pub fn remove_content_location(&mut self, content_id: &str, source_id: &str) {
        if let Some(locations) = self.content_index.get_mut(content_id) {
            locations.retain(|(s, _)| s != source_id);
            if locations.is_empty() {
                self.content_index.remove(content_id);
            }
        }
    }

    /// Clear all content locations for a source (e.g., when cache is cleared).
    #[wasm_bindgen]
    pub fn clear_source_locations(&mut self, source_id: &str) {
        for locations in self.content_index.values_mut() {
            locations.retain(|(s, _)| s != source_id);
        }
        // Remove empty entries
        self.content_index.retain(|_, v| !v.is_empty());
    }

    /// Resolve which source to try for content.
    ///
    /// Returns JSON with resolution result or error:
    /// ```json
    /// { "source_id": "indexeddb", "tier": 0, "url": null, "cached": true }
    /// ```
    /// or
    /// ```json
    /// { "error": "no_source_available", "content_type": "path", "content_id": "..." }
    /// ```
    #[wasm_bindgen]
    pub fn resolve(&mut self, content_type: &str, content_id: &str) -> String {
        self.resolution_count += 1;

        // 1. Check content index for known locations
        if let Some(known_locations) = self.content_index.get(content_id) {
            // Sort by recency (most recent first)
            let mut sorted_locs = known_locations.clone();
            sorted_locs.sort_by(|a, b| b.1.cmp(&a.1));

            for (source_id, _last_seen) in sorted_locs {
                if let Some(source) = self.sources.iter().find(|s| s.id == source_id && s.available) {
                    self.cache_hit_count += 1;
                    return self.build_result(source, content_type, content_id, true);
                }
            }
        }

        // 2. Find first available source that supports this content type
        for source in &self.sources {
            if source.available && source.content_types.iter().any(|t| t == content_type) {
                return self.build_result(source, content_type, content_id, false);
            }
        }

        // 3. No source found
        serde_json::to_string(&ResolutionError {
            error: "no_source_available".to_string(),
            content_type: content_type.to_string(),
            content_id: content_id.to_string(),
        }).unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string())
    }

    /// Get ordered list of sources to try for a content type.
    ///
    /// Returns JSON array of source objects:
    /// ```json
    /// [
    ///   { "id": "indexeddb", "tier": 0, "priority": 100, "url": null },
    ///   { "id": "projection", "tier": 1, "priority": 80, "url": "https://..." }
    /// ]
    /// ```
    #[wasm_bindgen]
    pub fn get_resolution_chain(&self, content_type: &str) -> String {
        let chain: Vec<serde_json::Value> = self.sources
            .iter()
            .filter(|s| s.available && s.content_types.iter().any(|t| t == content_type))
            .map(|s| serde_json::json!({
                "id": s.id,
                "tier": s.tier as u8,
                "priority": s.priority,
                "url": s.base_url,
            }))
            .collect();

        serde_json::to_string(&chain).unwrap_or_else(|_| "[]".to_string())
    }

    // =========================================================================
    // HTML5 App Resolution
    // =========================================================================

    /// Register an HTML5 app for resolution.
    ///
    /// # Arguments
    /// * `app_id` - Unique app identifier (used in URL: /apps/{app_id}/...)
    /// * `blob_hash` - Content hash of the zip blob
    /// * `entry_point` - Entry point file (e.g., "index.html")
    /// * `fallback_url` - Optional external fallback URL
    #[wasm_bindgen]
    pub fn register_app(
        &mut self,
        app_id: String,
        blob_hash: String,
        entry_point: String,
        fallback_url: Option<String>,
    ) {
        self.app_registry.insert(app_id, AppRegistration {
            blob_hash,
            entry_point,
            fallback_url,
            registered_at: current_time_ms(),
        });
    }

    /// Unregister an HTML5 app.
    #[wasm_bindgen]
    pub fn unregister_app(&mut self, app_id: &str) {
        self.app_registry.remove(app_id);
    }

    /// Check if an app is registered.
    #[wasm_bindgen]
    pub fn has_app(&self, app_id: &str) -> bool {
        self.app_registry.contains_key(app_id)
    }

    /// Get app blob hash.
    #[wasm_bindgen]
    pub fn get_app_blob_hash(&self, app_id: &str) -> Option<String> {
        self.app_registry.get(app_id).map(|r| r.blob_hash.clone())
    }

    /// Resolve URL for an HTML5 app.
    ///
    /// Tries sources in order:
    /// 1. Sources with "app" content type and base_url
    /// 2. Registered fallback URL
    /// 3. Empty string (resolution failed)
    ///
    /// Returns the URL to load in an iframe, or empty string if not resolvable.
    #[wasm_bindgen]
    pub fn resolve_app_url(&self, app_id: &str, path: Option<String>) -> String {
        let entry_point = self.app_registry
            .get(app_id)
            .map(|r| r.entry_point.clone())
            .unwrap_or_else(|| "index.html".to_string());

        let file_path = path.unwrap_or(entry_point);

        // 1. Find source that can serve apps
        for source in &self.sources {
            if source.available && source.content_types.iter().any(|t| t == "app") {
                if let Some(base) = &source.base_url {
                    return format!("{}/apps/{}/{}", base, app_id, file_path);
                }
            }
        }

        // 2. Try fallback URL
        if let Some(reg) = self.app_registry.get(app_id) {
            if let Some(fallback) = &reg.fallback_url {
                return fallback.clone();
            }
        }

        // 3. Not resolvable
        String::new()
    }

    /// Resolve app URL with full metadata.
    ///
    /// Returns JSON:
    /// ```json
    /// {
    ///   "url": "https://doorway.example.com/apps/my-app/index.html",
    ///   "source_id": "projection",
    ///   "blob_hash": "sha256-abc123",
    ///   "fallback_url": "https://external.com/my-app/"
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn resolve_app_url_full(&self, app_id: &str, path: Option<String>) -> String {
        let url = self.resolve_app_url(app_id, path);
        let reg = self.app_registry.get(app_id);

        let source_id = self.sources
            .iter()
            .find(|s| s.available && s.content_types.iter().any(|t| t == "app") && s.base_url.is_some())
            .map(|s| s.id.clone());

        serde_json::to_string(&serde_json::json!({
            "url": if url.is_empty() { None } else { Some(&url) },
            "source_id": source_id,
            "blob_hash": reg.map(|r| &r.blob_hash),
            "fallback_url": reg.and_then(|r| r.fallback_url.as_ref()),
        })).unwrap_or_else(|_| "{}".to_string())
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get resolution statistics as JSON.
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        let cache_hit_rate = if self.resolution_count > 0 {
            (self.cache_hit_count as f64 / self.resolution_count as f64) * 100.0
        } else {
            0.0
        };

        serde_json::to_string(&serde_json::json!({
            "resolution_count": self.resolution_count,
            "cache_hit_count": self.cache_hit_count,
            "cache_hit_rate": cache_hit_rate,
            "source_count": self.sources.len(),
            "indexed_content_count": self.content_index.len(),
            "registered_app_count": self.app_registry.len(),
        })).unwrap_or_else(|_| "{}".to_string())
    }

    /// Reset statistics.
    #[wasm_bindgen]
    pub fn reset_stats(&mut self) {
        self.resolution_count = 0;
        self.cache_hit_count = 0;
    }

    /// Get number of registered sources.
    #[wasm_bindgen]
    pub fn source_count(&self) -> u32 {
        self.sources.len() as u32
    }

    /// Get number of indexed content items.
    #[wasm_bindgen]
    pub fn indexed_content_count(&self) -> u32 {
        self.content_index.len() as u32
    }

    /// Get number of registered apps.
    #[wasm_bindgen]
    pub fn registered_app_count(&self) -> u32 {
        self.app_registry.len() as u32
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn build_result(
        &self,
        source: &ContentSource,
        content_type: &str,
        content_id: &str,
        cached: bool,
    ) -> String {
        let url = source.base_url.as_ref().map(|base| {
            // Build appropriate URL based on content type
            match content_type {
                "app" => format!("{}/apps/{}", base, content_id),
                "blob" => format!("{}/store/{}", base, content_id),
                "stream" => format!("{}/stream/{}", base, content_id),
                _ => format!("{}/api/v1/{}/{}", base, content_type, content_id),
            }
        });

        serde_json::to_string(&ResolutionResult {
            source_id: source.id.clone(),
            tier: source.tier as u8,
            url,
            cached,
        }).unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string())
    }
}

impl Default for ContentResolver {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_registration() {
        let mut resolver = ContentResolver::new();

        resolver.register_source(
            "local".into(),
            SourceTier::Local,
            100,
            r#"["path", "content"]"#,
            None,
        );

        resolver.register_source(
            "projection".into(),
            SourceTier::Projection,
            80,
            r#"["path", "content"]"#,
            Some("https://doorway.example.com".into()),
        );

        resolver.register_source(
            "conductor".into(),
            SourceTier::Authoritative,
            50,
            r#"["path", "content", "blob"]"#,
            None,
        );

        assert_eq!(resolver.source_count(), 3);

        // Sources should be sorted by tier, then priority desc
        let chain = resolver.get_resolution_chain("content");
        assert!(chain.contains("local"));
        assert!(chain.contains("projection"));
        assert!(chain.contains("conductor"));
    }

    #[test]
    fn test_resolve_order() {
        let mut resolver = ContentResolver::new();

        resolver.register_source("local".into(), SourceTier::Local, 100, r#"["content"]"#, None);
        resolver.register_source("projection".into(), SourceTier::Projection, 80, r#"["content"]"#, None);

        let result = resolver.resolve("content", "test-id");
        let parsed: ResolutionResult = serde_json::from_str(&result).unwrap();

        // Should resolve to local first (highest priority)
        assert_eq!(parsed.source_id, "local");
        assert_eq!(parsed.tier, 0);
        assert!(!parsed.cached);
    }

    #[test]
    fn test_content_location_learning() {
        let mut resolver = ContentResolver::new();

        resolver.register_source("local".into(), SourceTier::Local, 100, r#"["content"]"#, None);
        resolver.register_source("projection".into(), SourceTier::Projection, 80, r#"["content"]"#, None);

        // First resolution - no cached location
        let result1 = resolver.resolve("content", "test-id");
        let parsed1: ResolutionResult = serde_json::from_str(&result1).unwrap();
        assert!(!parsed1.cached);

        // Record that content was found at projection
        resolver.record_content_location("test-id".into(), "projection".into());

        // Second resolution - should prefer projection (cached location)
        let result2 = resolver.resolve("content", "test-id");
        let parsed2: ResolutionResult = serde_json::from_str(&result2).unwrap();
        assert_eq!(parsed2.source_id, "projection");
        assert!(parsed2.cached);
    }

    #[test]
    fn test_source_availability() {
        let mut resolver = ContentResolver::new();

        resolver.register_source("local".into(), SourceTier::Local, 100, r#"["content"]"#, None);
        resolver.register_source("projection".into(), SourceTier::Projection, 80, r#"["content"]"#, None);

        // Mark local as unavailable
        resolver.set_source_available("local", false);

        let result = resolver.resolve("content", "test-id");
        let parsed: ResolutionResult = serde_json::from_str(&result).unwrap();

        // Should skip local, resolve to projection
        assert_eq!(parsed.source_id, "projection");
    }

    #[test]
    fn test_app_resolution() {
        let mut resolver = ContentResolver::new();

        resolver.register_source(
            "doorway".into(),
            SourceTier::Projection,
            80,
            r#"["app"]"#,
            Some("https://doorway.example.com".into()),
        );

        resolver.register_app(
            "evolution-of-trust".into(),
            "sha256-abc123".into(),
            "index.html".into(),
            Some("https://ncase.me/trust/".into()),
        );

        let url = resolver.resolve_app_url("evolution-of-trust", None);
        assert_eq!(url, "https://doorway.example.com/apps/evolution-of-trust/index.html");

        // Test with custom path
        let url2 = resolver.resolve_app_url("evolution-of-trust", Some("js/main.js".into()));
        assert_eq!(url2, "https://doorway.example.com/apps/evolution-of-trust/js/main.js");
    }

    #[test]
    fn test_app_fallback() {
        let mut resolver = ContentResolver::new();

        // No sources registered - should fall back to fallback_url
        resolver.register_app(
            "evolution-of-trust".into(),
            "sha256-abc123".into(),
            "index.html".into(),
            Some("https://ncase.me/trust/".into()),
        );

        let url = resolver.resolve_app_url("evolution-of-trust", None);
        assert_eq!(url, "https://ncase.me/trust/");
    }

    #[test]
    fn test_no_source_error() {
        let mut resolver = ContentResolver::new();

        // No sources registered
        let result = resolver.resolve("content", "test-id");
        let parsed: ResolutionError = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.error, "no_source_available");
        assert_eq!(parsed.content_type, "content");
    }

    #[test]
    fn test_stats() {
        let mut resolver = ContentResolver::new();

        resolver.register_source("local".into(), SourceTier::Local, 100, r#"["content"]"#, None);

        // Make some resolutions
        resolver.resolve("content", "id1");
        resolver.resolve("content", "id2");
        resolver.record_content_location("id1".into(), "local".into());
        resolver.resolve("content", "id1"); // This should be a cache hit

        let stats = resolver.get_stats();
        assert!(stats.contains("\"resolution_count\":3"));
        assert!(stats.contains("\"cache_hit_count\":1"));
    }
}
