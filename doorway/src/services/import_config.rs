//! Import Configuration Discovery
//!
//! Discovers import capabilities from DNA zomes that implement
//! the `__doorway_import_config` function.
//!
//! ## Zome Contract
//!
//! Any DNA can export import configuration:
//!
//! ```rust,ignore
//! #[hdk_extern]
//! pub fn __doorway_import_config(_: ()) -> ExternResult<ImportConfig> {
//!     Ok(ImportConfig {
//!         enabled: true,
//!         batch_types: vec![
//!             ImportBatchType {
//!                 batch_type: "content".to_string(),
//!                 queue_fn: "queue_import".to_string(),
//!                 process_fn: "process_import_chunk".to_string(),
//!                 max_items: 5000,
//!                 chunk_size: 50,
//!                 chunk_interval_ms: 100,
//!                 schema_version: 1,
//!             },
//!         ],
//!         require_auth: true,
//!         allowed_agents: None,
//!     })
//! }
//! ```
//!
//! ## Discovery Flow
//!
//! 1. Doorway connects to conductor
//! 2. Calls `__doorway_import_config` on each DNA
//! 3. Stores discovered configs in ImportConfigStore
//! 4. ImportOrchestrator uses configs to route batch imports

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, info};

// Re-export from doorway-client
pub use doorway_client::{ImportConfig, ImportBatchType, IMPORT_CONFIG_FN};

/// Import configuration for a specific DNA
#[derive(Debug, Clone)]
pub struct DnaImportConfig {
    /// DNA hash
    pub dna_hash: String,

    /// Import configuration from zome
    pub config: ImportConfig,

    /// Whether discovery has been attempted
    pub discovered: bool,
}

impl DnaImportConfig {
    /// Create empty config (discovery not yet attempted)
    pub fn empty(dna_hash: &str) -> Self {
        Self {
            dna_hash: dna_hash.to_string(),
            config: ImportConfig::default(),
            discovered: false,
        }
    }

    /// Create from discovered config
    pub fn from_config(dna_hash: &str, config: ImportConfig) -> Self {
        Self {
            dna_hash: dna_hash.to_string(),
            config,
            discovered: true,
        }
    }

    /// Get batch type config by name
    pub fn get_batch_type(&self, batch_type: &str) -> Option<&ImportBatchType> {
        self.config.batch_types.iter()
            .find(|bt| bt.batch_type == batch_type)
    }

    /// Check if a batch type is supported
    pub fn supports_batch_type(&self, batch_type: &str) -> bool {
        self.config.enabled && self.get_batch_type(batch_type).is_some()
    }
}

/// Store for discovered import configurations
///
/// Thread-safe, multi-DNA configuration store
#[derive(Debug, Default)]
pub struct ImportConfigStore {
    /// DNA hash -> Import config
    configs: DashMap<String, DnaImportConfig>,
}

impl ImportConfigStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
        }
    }

    /// Store discovered config for a DNA
    pub fn set_config(&self, dna_hash: &str, config: ImportConfig) {
        let dna_config = DnaImportConfig::from_config(dna_hash, config);
        info!(
            dna = dna_hash,
            enabled = dna_config.config.enabled,
            batch_types = dna_config.config.batch_types.len(),
            "Import config discovered"
        );
        self.configs.insert(dna_hash.to_string(), dna_config);
    }

    /// Mark a DNA as discovered (even if no config found)
    pub fn mark_discovered(&self, dna_hash: &str) {
        self.configs
            .entry(dna_hash.to_string())
            .or_insert_with(|| DnaImportConfig::empty(dna_hash))
            .discovered = true;

        debug!(dna = dna_hash, "Import config discovery attempted (no config)");
    }

    /// Check if discovery has been attempted for a DNA
    pub fn is_discovered(&self, dna_hash: &str) -> bool {
        self.configs
            .get(dna_hash)
            .map(|c| c.discovered)
            .unwrap_or(false)
    }

    /// Get import config for a DNA
    pub fn get_config(&self, dna_hash: &str) -> Option<DnaImportConfig> {
        self.configs.get(dna_hash).map(|c| c.clone())
    }

    /// Check if a DNA supports a specific batch type
    pub fn supports_batch_type(&self, dna_hash: &str, batch_type: &str) -> bool {
        self.configs
            .get(dna_hash)
            .map(|c| c.supports_batch_type(batch_type))
            .unwrap_or(false)
    }

    /// Get batch type config for a DNA
    pub fn get_batch_type(&self, dna_hash: &str, batch_type: &str) -> Option<ImportBatchType> {
        self.configs
            .get(dna_hash)
            .and_then(|c| c.get_batch_type(batch_type).cloned())
    }

    /// Get all DNAs with import enabled
    pub fn get_import_enabled_dnas(&self) -> Vec<String> {
        self.configs
            .iter()
            .filter(|c| c.config.enabled)
            .map(|c| c.dna_hash.clone())
            .collect()
    }

    /// Get all supported batch types across all DNAs
    pub fn get_all_batch_types(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        for entry in self.configs.iter() {
            if entry.config.enabled {
                for bt in &entry.config.batch_types {
                    result
                        .entry(bt.batch_type.clone())
                        .or_default()
                        .push(entry.dna_hash.clone());
                }
            }
        }

        result
    }
}

/// Import config discovery service
pub struct ImportConfigDiscovery {
    store: Arc<ImportConfigStore>,
}

impl ImportConfigDiscovery {
    /// Create a new discovery service with the given store
    pub fn new(store: Arc<ImportConfigStore>) -> Self {
        Self { store }
    }

    /// Record a discovered import config
    pub fn record_config(&self, dna_hash: &str, config: ImportConfig) {
        self.store.set_config(dna_hash, config);
    }

    /// Mark discovery as attempted (no config found)
    pub fn record_no_config(&self, dna_hash: &str) {
        self.store.mark_discovered(dna_hash);
    }

    /// Check if discovery has been attempted
    pub fn is_discovered(&self, dna_hash: &str) -> bool {
        self.store.is_discovered(dna_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doorway_client::ImportBatchTypeBuilder;

    #[test]
    fn test_empty_store() {
        let store = ImportConfigStore::new();

        assert!(!store.is_discovered("some_dna"));
        assert!(!store.supports_batch_type("some_dna", "content"));
        assert!(store.get_import_enabled_dnas().is_empty());
    }

    #[test]
    fn test_store_config() {
        let store = ImportConfigStore::new();

        let config = ImportConfig {
            enabled: true,
            batch_types: vec![
                ImportBatchTypeBuilder::new("content")
                    .max_items(1000)
                    .build(),
            ],
            require_auth: true,
            allowed_agents: None,
        };

        store.set_config("dna1", config);

        assert!(store.is_discovered("dna1"));
        assert!(store.supports_batch_type("dna1", "content"));
        assert!(!store.supports_batch_type("dna1", "paths"));

        let dnas = store.get_import_enabled_dnas();
        assert_eq!(dnas.len(), 1);
        assert_eq!(dnas[0], "dna1");
    }

    #[test]
    fn test_disabled_config() {
        let store = ImportConfigStore::new();

        let config = ImportConfig {
            enabled: false,
            batch_types: vec![
                ImportBatchTypeBuilder::new("content").build(),
            ],
            require_auth: true,
            allowed_agents: None,
        };

        store.set_config("dna1", config);

        // Discovery was attempted, but import is disabled
        assert!(store.is_discovered("dna1"));
        assert!(!store.supports_batch_type("dna1", "content"));
        assert!(store.get_import_enabled_dnas().is_empty());
    }

    #[test]
    fn test_get_all_batch_types() {
        let store = ImportConfigStore::new();

        store.set_config("dna1", ImportConfig {
            enabled: true,
            batch_types: vec![
                ImportBatchTypeBuilder::new("content").build(),
                ImportBatchTypeBuilder::new("paths").build(),
            ],
            require_auth: true,
            allowed_agents: None,
        });

        store.set_config("dna2", ImportConfig {
            enabled: true,
            batch_types: vec![
                ImportBatchTypeBuilder::new("content").build(),
            ],
            require_auth: true,
            allowed_agents: None,
        });

        let batch_types = store.get_all_batch_types();

        assert_eq!(batch_types.get("content").map(|v| v.len()), Some(2));
        assert_eq!(batch_types.get("paths").map(|v| v.len()), Some(1));
        assert!(batch_types.get("steps").is_none());
    }
}
