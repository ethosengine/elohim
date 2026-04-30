//! Manifest integrity entry type — Phase 3 P3.2.
//!
//! Manifests are constitutional EPRs declaring pillar projections, app
//! vocabularies, standing-policy rules. Validation is structural and
//! deterministic (no get_links per project_hdi_no_get_links_in_validators);
//! authority gating happens at the coordinator level (mishpat-mediated for
//! constitutional manifests in Phase 3.5).
//!
//! ## Phase 3 floors (create-time)
//!
//! - `manifest_kind` whitelist (5 bootstrap kinds)
//! - `payload_json` is syntactically valid JSON
//! - `revision` is at least 1
//! - `pillar-projection` requires `pillar`
//!
//! ## Deferred to Phase 3.5 (richer dispatch with original-entry fetch)
//!
//! - kind immutability across revisions
//! - monotonic revision check
//! - mishpat-DNA-notarized authority gating

use hdi::prelude::*;

/// Bootstrap manifest_kind taxonomy. Phase 3.5 adds mishpat-DNA-notarized
/// kinds (constitutional-floor, tending-policy variants, etc.) by extending
/// this list via a separate integrity-zome change.
const MANIFEST_KINDS: &[&str] = &[
    "app",
    "pillar-projection",
    "standing-policy",
    "tending-policy",
    "onboarding",
];

#[hdk_entry_helper]
#[derive(Clone)]
pub struct Manifest {
    /// Manifest classification — drives consumer dispatch.
    pub manifest_kind: String,
    /// Optional pillar association. Required when `manifest_kind == "pillar-projection"`.
    pub pillar: Option<String>,
    /// JSON-encoded payload conforming to the manifest_kind's JSON schema.
    pub payload_json: String,
    /// Optional schemaRef pointing to a more specific schema EPR.
    pub schema_ref: Option<String>,
    /// Revision counter for upserts; coordinator increments on update.
    pub revision: u32,
}

impl Manifest {
    /// Phase 3 create-time floor checks. Deterministic, no DHT lookups.
    /// Returns `Ok(())` when valid; `Err(reason)` when a floor is violated.
    pub fn validate(&self) -> Result<(), String> {
        // Floor 1: manifest_kind must be from the bootstrap taxonomy.
        if !MANIFEST_KINDS.contains(&self.manifest_kind.as_str()) {
            return Err(format!(
                "unknown manifest_kind: {} (allowed: {:?})",
                self.manifest_kind, MANIFEST_KINDS
            ));
        }

        // Floor 2: payload_json must parse as JSON.
        if serde_json::from_str::<serde_json::Value>(&self.payload_json).is_err() {
            return Err("payload_json is not valid JSON".to_string());
        }

        // Floor 3: revision must be >= 1.
        if self.revision == 0 {
            return Err("revision must be >= 1".to_string());
        }

        // Floor 4: pillar-projection requires the pillar field.
        if self.manifest_kind == "pillar-projection" && self.pillar.is_none() {
            return Err("pillar-projection manifest requires pillar field".to_string());
        }

        Ok(())
    }
}
