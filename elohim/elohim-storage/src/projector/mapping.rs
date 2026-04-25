//! Manifest registry — loads pillar manifests and registers projection mappings.
//!
//! For B.3 the registry is an in-memory struct. Loading from on-disk manifest
//! JSON is supported via [`ManifestRegistry::from_pillar_manifests`]; the test
//! helper [`mock_manifest_registry`] hand-builds a registry without I/O.
//!
//! The `projections[]` array in each pillar's `manifest.json` is the source of
//! truth for which (pillar, kind, schema_key) tuples map to which target table
//! and column path expressions.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from manifest loading.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("failed to read manifest at {path}: {cause}")]
    Io { path: String, cause: std::io::Error },

    #[error("failed to parse manifest at {path}: {cause}")]
    Parse { path: String, cause: String },
}

// ---------------------------------------------------------------------------
// Registered projection
// ---------------------------------------------------------------------------

/// A single projection registered from a pillar manifest's `projections[]` entry.
#[derive(Debug, Clone)]
pub struct RegisteredProjection {
    /// Pillar name, e.g. `"shefa"`, `"lamad"`.
    pub pillar: String,
    /// EPR `kind` value to match, e.g. `"EconomicEvent"`.
    pub kind: String,
    /// `schemaKey` value to match, e.g. `"economic-event"`.
    pub schema_key: String,
    /// Target SQLite table, e.g. `"economic_events"`.
    pub target_table: String,
    /// Column-to-JSONPath mapping declared in the manifest.
    /// Key = DB column name, Value = JSONPath expression into `atom.payload`.
    pub column_mapping: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Manifest registry
// ---------------------------------------------------------------------------

/// In-memory registry of all projections declared across pillar manifests.
pub struct ManifestRegistry {
    projections: Vec<RegisteredProjection>,
}

impl ManifestRegistry {
    /// Construct a registry from a set of (pillar_name, manifest_path) pairs.
    ///
    /// Reads each manifest JSON file, extracts its `projections[]` array, and
    /// registers each entry. Manifests without a `projections` key are silently
    /// skipped (not all pillars declare projections).
    pub fn from_pillar_manifests(paths: &[(String, &Path)]) -> Result<Self, RegistryError> {
        let mut projections = Vec::new();

        for (pillar, path) in paths {
            let bytes = std::fs::read(path).map_err(|e| RegistryError::Io {
                path: path.display().to_string(),
                cause: e,
            })?;

            // Parse only the `projections` key; ignore the rest of the manifest.
            let manifest: PillarManifest =
                serde_json::from_slice(&bytes).map_err(|e| RegistryError::Parse {
                    path: path.display().to_string(),
                    cause: e.to_string(),
                })?;

            for entry in manifest.projections.unwrap_or_default() {
                projections.push(RegisteredProjection {
                    pillar: pillar.clone(),
                    kind: entry.kind,
                    schema_key: entry.schema_key,
                    target_table: entry.target_table,
                    column_mapping: entry.column_mapping,
                });
            }
        }

        Ok(Self { projections })
    }

    /// Construct an empty registry (no projections). Useful as a null object.
    pub fn empty() -> Self {
        Self {
            projections: Vec::new(),
        }
    }

    /// Return all registered projections.
    pub fn all_projections(&self) -> &[RegisteredProjection] {
        &self.projections
    }
}

// ---------------------------------------------------------------------------
// Serde shapes for manifest JSON parsing
// ---------------------------------------------------------------------------

/// Minimal deserialization of a pillar manifest — only the `projections` key.
#[derive(Debug, Deserialize)]
struct PillarManifest {
    #[serde(default)]
    projections: Option<Vec<ManifestProjectionEntry>>,
}

/// One entry in the `projections[]` array of a pillar manifest.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestProjectionEntry {
    kind: String,
    schema_key: String,
    target_table: String,
    #[serde(default)]
    column_mapping: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Test helper
// ---------------------------------------------------------------------------

/// Build a [`ManifestRegistry`] for tests without any file I/O.
///
/// Registers the `shefa / EconomicEvent / economic-event → economic_events`
/// projection, which matches the shefa manifest's `projections[]` declaration.
#[cfg(test)]
pub fn mock_manifest_registry() -> ManifestRegistry {
    let mut column_mapping = BTreeMap::new();
    column_mapping.insert("provider".to_string(), "payload.provider".to_string());
    column_mapping.insert("receiver".to_string(), "payload.receiver".to_string());
    column_mapping.insert("action".to_string(), "payload.action".to_string());

    ManifestRegistry {
        projections: vec![RegisteredProjection {
            pillar: "shefa".to_string(),
            kind: "EconomicEvent".to_string(),
            schema_key: "economic-event".to_string(),
            target_table: "economic_events".to_string(),
            column_mapping,
        }],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_registry_has_shefa_economic_event_projection() {
        let registry = mock_manifest_registry();
        let projections = registry.all_projections();
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].pillar, "shefa");
        assert_eq!(projections[0].kind, "EconomicEvent");
        assert_eq!(projections[0].target_table, "economic_events");
    }

    #[test]
    fn empty_registry_has_no_projections() {
        let registry = ManifestRegistry::empty();
        assert!(registry.all_projections().is_empty());
    }
}
