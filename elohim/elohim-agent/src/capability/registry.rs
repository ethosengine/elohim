//! Capability registry for tracking available capabilities.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::ElohimCapability;

/// Registry of available capabilities and their handlers.
pub struct CapabilityRegistry {
    /// Set of registered capabilities
    capabilities: Arc<RwLock<HashSet<ElohimCapability>>>,
    /// Capability metadata
    metadata: Arc<RwLock<HashMap<ElohimCapability, CapabilityMetadata>>>,
}

/// Metadata about a registered capability.
#[derive(Debug, Clone)]
pub struct CapabilityMetadata {
    /// Whether this capability is currently enabled
    pub enabled: bool,
    /// Custom timeout override (ms)
    pub timeout_ms: Option<u64>,
    /// Required constitutional layer
    pub required_layer: Option<constitution::ConstitutionalLayer>,
}

impl Default for CapabilityMetadata {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_ms: None,
            required_layer: None,
        }
    }
}

impl CapabilityRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashSet::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a capability.
    pub async fn register(&self, capability: ElohimCapability) {
        let mut caps = self.capabilities.write().await;
        caps.insert(capability);

        let mut meta = self.metadata.write().await;
        meta.entry(capability).or_insert_with(|| CapabilityMetadata {
            required_layer: capability.required_layer(),
            ..Default::default()
        });
    }

    /// Register multiple capabilities.
    pub async fn register_all(&self, capabilities: impl IntoIterator<Item = ElohimCapability>) {
        for cap in capabilities {
            self.register(cap).await;
        }
    }

    /// Check if a capability is registered.
    pub async fn has(&self, capability: ElohimCapability) -> bool {
        let caps = self.capabilities.read().await;
        caps.contains(&capability)
    }

    /// Check if a capability is registered and enabled.
    pub async fn is_available(&self, capability: ElohimCapability) -> bool {
        let caps = self.capabilities.read().await;
        if !caps.contains(&capability) {
            return false;
        }

        let meta = self.metadata.read().await;
        meta.get(&capability).map(|m| m.enabled).unwrap_or(false)
    }

    /// Get all registered capabilities.
    pub async fn all(&self) -> Vec<ElohimCapability> {
        let caps = self.capabilities.read().await;
        caps.iter().cloned().collect()
    }

    /// Get all available (registered and enabled) capabilities.
    pub async fn available(&self) -> Vec<ElohimCapability> {
        let caps = self.capabilities.read().await;
        let meta = self.metadata.read().await;

        caps.iter()
            .filter(|c| meta.get(c).map(|m| m.enabled).unwrap_or(true))
            .cloned()
            .collect()
    }

    /// Enable or disable a capability.
    pub async fn set_enabled(&self, capability: ElohimCapability, enabled: bool) {
        let mut meta = self.metadata.write().await;
        if let Some(m) = meta.get_mut(&capability) {
            m.enabled = enabled;
        }
    }

    /// Set timeout for a capability.
    pub async fn set_timeout(&self, capability: ElohimCapability, timeout_ms: u64) {
        let mut meta = self.metadata.write().await;
        if let Some(m) = meta.get_mut(&capability) {
            m.timeout_ms = Some(timeout_ms);
        }
    }

    /// Get timeout for a capability.
    pub async fn get_timeout(&self, capability: ElohimCapability) -> u64 {
        let meta = self.metadata.read().await;
        meta.get(&capability)
            .and_then(|m| m.timeout_ms)
            .unwrap_or_else(|| capability.estimated_time_ms() * 2)
    }

    /// Get metadata for a capability.
    pub async fn get_metadata(&self, capability: ElohimCapability) -> Option<CapabilityMetadata> {
        let meta = self.metadata.read().await;
        meta.get(&capability).cloned()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry() {
        let registry = CapabilityRegistry::new();

        registry.register(ElohimCapability::SpiralDetection).await;
        registry.register(ElohimCapability::ContentSafetyReview).await;

        assert!(registry.has(ElohimCapability::SpiralDetection).await);
        assert!(!registry.has(ElohimCapability::PathAnalysis).await);

        assert!(registry.is_available(ElohimCapability::SpiralDetection).await);

        registry
            .set_enabled(ElohimCapability::SpiralDetection, false)
            .await;

        assert!(registry.has(ElohimCapability::SpiralDetection).await);
        assert!(!registry.is_available(ElohimCapability::SpiralDetection).await);
    }
}
