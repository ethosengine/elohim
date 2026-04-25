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

    /// Return true if there is a registered projection for the given (kind, schema_key) pair.
    ///
    /// Used to document Invariant I2 (manifest-authority): the projector only fetches
    /// atoms whose (kind, schema_key) match a declared projection. Undeclared atoms are
    /// never fetched, so this guard is structural — the projector loop iterates
    /// `all_projections()` and queries `epr_atoms` filtered by `(kind, schema_key)`,
    /// which means atoms whose kind/schema_key have no declared projection are simply
    /// never returned by `fetch_epr_atoms_since`. This method exists for clarity and
    /// test-assertion purposes only.
    pub fn has_projection(&self, kind: &str, schema_key: &str) -> bool {
        self.projections
            .iter()
            .any(|p| p.kind == kind && p.schema_key == schema_key)
    }

    /// Look up a projection by `(pillar, kind)` for signal-emit composition.
    ///
    /// Returns the first projection matching both fields. Used by
    /// `/api/v1/signal/emit` (EPR Phase 2B Task C.2) to resolve an inbound
    /// `SignalIntent { pillar, signalType }` into the `(schema_key, target_table)`
    /// pair the composed `Envelope` needs. Today shefa is the only pillar
    /// declaring a projection; future pillars register more.
    pub fn find_projection(&self, pillar: &str, kind: &str) -> Option<&RegisteredProjection> {
        self.projections
            .iter()
            .find(|p| p.pillar == pillar && p.kind == kind)
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

/// Build the canonical shefa `EconomicEvent → economic_events` column mapping.
///
/// This is the authoritative in-process representation of the shefa manifest's
/// `projections[0].columnMapping`. Both the unit-test mock helper and the
/// integration test use this to construct a [`ManifestRegistry`] without file I/O.
///
/// Column mapping covers all NOT NULL columns of `economic_events`:
/// - `id`, `dht_anchor_hash` ← `$cid` (content address = idempotent PK + provenance)
/// - `h_app_id` ← `$signer`
/// - `created_at` ← `$issuedAt`
/// - `state` ← `$state:recorded` (constant default)
/// - `action`, `provider`, `receiver`, `has_point_in_time` ← dotted payload paths
/// - optional quantity/resource columns ← dotted payload paths (nullable; ok if absent)
fn shefa_economic_event_column_mapping() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    // Projector-derived NOT NULL columns.
    m.insert("id".to_string(), "$cid".to_string());
    m.insert("h_app_id".to_string(), "$signer".to_string());
    m.insert("created_at".to_string(), "$issuedAt".to_string());
    m.insert("state".to_string(), "$state:recorded".to_string());
    m.insert("dht_anchor_hash".to_string(), "$cid".to_string());
    // Payload-sourced columns.
    m.insert("action".to_string(), "payload.action".to_string());
    m.insert("provider".to_string(), "payload.provider".to_string());
    m.insert("receiver".to_string(), "payload.receiver".to_string());
    m.insert(
        "has_point_in_time".to_string(),
        "payload.hasPointInTime".to_string(),
    );
    m.insert(
        "resource_conforms_to".to_string(),
        "payload.resourceConformsTo".to_string(),
    );
    m.insert(
        "resource_quantity_value".to_string(),
        "payload.resourceQuantity.numericValue".to_string(),
    );
    m.insert(
        "resource_quantity_unit".to_string(),
        "payload.resourceQuantity.hasUnit".to_string(),
    );
    m.insert(
        "effort_quantity_value".to_string(),
        "payload.effortQuantity.numericValue".to_string(),
    );
    m.insert(
        "effort_quantity_unit".to_string(),
        "payload.effortQuantity.hasUnit".to_string(),
    );
    // Mirror the source EPR atom's verified_at into the projection row (I4/I5).
    // When the source atom is unverified, this resolves to None and the column
    // is omitted from the INSERT, defaulting to NULL. On revocation sweep,
    // the projection row's verified_at is cleared atomically.
    m.insert("verified_at".to_string(), "$verifiedAt".to_string());
    m
}

impl ManifestRegistry {
    /// Construct a registry pre-loaded with the shefa `EconomicEvent → economic_events`
    /// projection. Available in all build profiles so integration tests can use it
    /// without a feature flag.
    pub fn with_shefa_economic_event() -> Self {
        Self {
            projections: vec![RegisteredProjection {
                pillar: "shefa".to_string(),
                kind: "EconomicEvent".to_string(),
                schema_key: "economic-event".to_string(),
                target_table: "economic_events".to_string(),
                column_mapping: shefa_economic_event_column_mapping(),
            }],
        }
    }
}

/// Build a [`ManifestRegistry`] for unit tests without any file I/O.
///
/// Alias for [`ManifestRegistry::with_shefa_economic_event`], kept as a
/// `#[cfg(test)]` free function for backwards compat with unit tests that
/// import `mock_manifest_registry` directly.
#[cfg(test)]
pub fn mock_manifest_registry() -> ManifestRegistry {
    ManifestRegistry::with_shefa_economic_event()
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
