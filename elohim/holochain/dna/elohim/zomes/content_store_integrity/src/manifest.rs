//! Manifest integrity entry type — Phase 3 P3.2 / Phase 3.5 P3.5.7.
//!
//! Manifests are constitutional EPRs declaring pillar projections, app
//! vocabularies, standing-policy rules. Validation is structural and
//! deterministic (no get_links per project_hdi_no_get_links_in_validators);
//! authority gating happens at the coordinator level (mishpat-mediated for
//! constitutional manifests in Phase 3.5).
//!
//! ## Phase 3 floors (create-time)
//!
//! 1. `manifest_kind` whitelist (5 bootstrap kinds)
//! 2. `payload_json` is syntactically valid JSON
//! 3. `revision` is at least 1
//! 4. `pillar-projection` requires `pillar`
//!
//! ## Phase 3.5 floors (create-time, conditional)
//!
//! 5. `standing-policy` and `tending-policy` manifests must include a `floor`
//!    sub-object in `payload_json` whose `classes` array exactly matches the
//!    constitutional floor table (brainstorm §2.8). Structural Rust validation
//!    mirrors `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json`
//!    and `tending-policy-floor.schema.json` (T3). No `jsonschema` crate dep —
//!    avoids adding heavyweight WASM dependency; T3 schema tests verify the JSON
//!    Schema independently; this mirror enforces at zome-create time.
//!
//! ## Deferred to future phases (richer dispatch with original-entry fetch)
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

/// Standing-immune floor class identifiers.
///
/// Mirrors `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json`
/// (T3 source of truth). Any drift between T3's enum values and this array is
/// a CRITICAL bug — the schema and this Rust mirror must stay identical.
/// T3's schema tests verify the JSON Schema; T7's tests verify this mirror
/// against a valid standing-policy payload.
///
/// Vocabulary parity invariant: update T3 first, then update this array.
const STANDING_IMMUNE_CLASSES: &[&str] = &[
    "local-relationship-reach",
    "cid-targeted-lookup",
    "constitutional-floor-signatures",
    "new-voice-baseline",
    "vulnerable-class-elevation",
];

/// Tending-immune floor class identifiers.
///
/// Mirrors `elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json`
/// (T3 source of truth). Same parity invariant as `STANDING_IMMUNE_CLASSES`.
///
/// Vocabulary parity invariant: update T3 first, then update this array.
const TENDING_IMMUNE_CLASSES: &[&str] = &[
    "accountability-information",
    "community-facts",
    "custodial-communications",
    "constitutional-updates",
    "elohim-as-counsel-notifications",
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
    /// Phase 3 / 3.5 create-time floor checks. Deterministic, no DHT lookups.
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
        let payload: serde_json::Value =
            serde_json::from_str(&self.payload_json).map_err(|_| "payload_json is not valid JSON".to_string())?;

        // Floor 3: revision must be >= 1.
        if self.revision == 0 {
            return Err("revision must be >= 1".to_string());
        }

        // Floor 4: pillar-projection requires the pillar field.
        if self.manifest_kind == "pillar-projection" && self.pillar.is_none() {
            return Err("pillar-projection manifest requires pillar field".to_string());
        }

        // Floor 5: standing-policy and tending-policy manifests require a
        // `floor` sub-object in payload_json that exactly matches the
        // constitutional class table (brainstorm §2.8 / T3 schemas).
        //
        // The discriminator is conditional — `app`, `pillar-projection`, and
        // `onboarding` manifests are NOT required to carry a floor object.
        // Cross-entity authority (mishpat-DNA-notarization gate) remains at
        // the coordinator level per project_hdi_no_get_links_in_validators.
        if self.manifest_kind == "standing-policy" || self.manifest_kind == "tending-policy" {
            let floor = payload
                .get("floor")
                .ok_or_else(|| format!(
                    "{} manifest requires payload.floor sub-object",
                    self.manifest_kind
                ))?;
            let allowed_classes = match self.manifest_kind.as_str() {
                "standing-policy" => STANDING_IMMUNE_CLASSES,
                "tending-policy" => TENDING_IMMUNE_CLASSES,
                _ => unreachable!(),
            };
            validate_floor_sub_object(floor, allowed_classes)?;
        }

        Ok(())
    }
}

/// Validate a `floor` sub-object from a policy manifest payload.
///
/// Mirrors the structural constraints from T3's JSON Schema:
/// - `floor.classes` must be an array of exactly 5 entries (`minItems`/`maxItems`)
/// - each entry must have `class` (string in allowed set) and `protection` (non-empty string)
/// - no duplicate `class` values (`uniqueItems`)
///
/// This is a free function (not `pub`) — it's an implementation detail of
/// `Manifest::validate`. The allowed-class slice is caller-supplied so the
/// same function serves both standing-policy and tending-policy kinds.
fn validate_floor_sub_object(
    floor: &serde_json::Value,
    allowed_classes: &[&str],
) -> Result<(), String> {
    let classes = floor
        .get("classes")
        .and_then(|v| v.as_array())
        .ok_or("floor.classes must be an array")?;

    if classes.len() != 5 {
        return Err(format!(
            "floor.classes must have exactly 5 entries, got {}",
            classes.len()
        ));
    }

    let mut seen = std::collections::HashSet::new();
    for entry in classes {
        let class = entry
            .get("class")
            .and_then(|v| v.as_str())
            .ok_or("floor.classes[*].class must be a string")?;
        let protection = entry
            .get("protection")
            .and_then(|v| v.as_str())
            .ok_or("floor.classes[*].protection must be a string")?;

        if !allowed_classes.contains(&class) {
            return Err(format!(
                "unknown floor class: {} (allowed: {:?})",
                class, allowed_classes
            ));
        }
        if protection.is_empty() {
            return Err(format!("floor class {} has empty protection", class));
        }
        if !seen.insert(class.to_string()) {
            return Err(format!("duplicate floor class: {}", class));
        }
    }

    Ok(())
}

// =============================================================================
// Inline tests (host target; no HDK context required)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_manifest(
        manifest_kind: &str,
        pillar: Option<&str>,
        payload_json: &str,
        revision: u32,
    ) -> Manifest {
        Manifest {
            manifest_kind: manifest_kind.to_string(),
            pillar: pillar.map(|s| s.to_string()),
            payload_json: payload_json.to_string(),
            schema_ref: None,
            revision,
        }
    }

    /// A minimal valid `app` manifest — no floor required.
    fn valid_app() -> Manifest {
        make_manifest("app", None, r#"{"name":"elohim-app"}"#, 1)
    }

    /// Build a standing-policy payload with the full floor object.
    fn standing_floor_payload() -> String {
        serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",         "protection": "unconditional family/household propagation" },
                    { "class": "cid-targeted-lookup",              "protection": "anyone with a CID can fetch" },
                    { "class": "constitutional-floor-signatures",  "protection": "mishpat decisions are always verified" },
                    { "class": "new-voice-baseline",               "protection": "first-time author has nonzero floor" },
                    { "class": "vulnerable-class-elevation",       "protection": "children and displaced persons get elevated floor" }
                ]
            }
        })
        .to_string()
    }

    /// Build a tending-policy payload with the full floor object.
    fn tending_floor_payload() -> String {
        serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "accountability-information",            "protection": "corrections bypass all filters" },
                    { "class": "community-facts",                       "protection": "civic emergencies never filterable" },
                    { "class": "custodial-communications",              "protection": "ward-targeting messages bypass steward filter" },
                    { "class": "constitutional-updates",                "protection": "mishpat rule changes always reach recipients" },
                    { "class": "elohim-as-counsel-notifications",       "protection": "elohim may override filters for counsel" }
                ]
            }
        })
        .to_string()
    }

    // -----------------------------------------------------------------------
    // Existing Phase 3 tests (regression)
    // -----------------------------------------------------------------------

    #[test]
    fn baseline_valid_app_passes() {
        assert!(valid_app().validate().is_ok());
    }

    #[test]
    fn rejects_unknown_manifest_kind() {
        let m = make_manifest("unknown-kind", None, r#"{}"#, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("unknown manifest_kind"), "got: {msg}");
        assert!(msg.contains("unknown-kind"), "got: {msg}");
    }

    #[test]
    fn rejects_invalid_payload_json() {
        let m = make_manifest("app", None, "not-json", 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("payload_json is not valid JSON"), "got: {msg}");
    }

    #[test]
    fn rejects_revision_zero() {
        let m = make_manifest("app", None, r#"{}"#, 0);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("revision must be >= 1"), "got: {msg}");
    }

    #[test]
    fn rejects_pillar_projection_without_pillar() {
        let m = make_manifest("pillar-projection", None, r#"{}"#, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("pillar-projection manifest requires pillar field"), "got: {msg}");
    }

    #[test]
    fn accepts_pillar_projection_with_pillar() {
        let m = make_manifest("pillar-projection", Some("lamad"), r#"{}"#, 1);
        assert!(m.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Regression: non-policy kinds accepted WITHOUT floor (Floor 5 is conditional)
    // -----------------------------------------------------------------------

    #[test]
    fn app_manifest_without_floor_accepted() {
        let m = make_manifest("app", None, r#"{"name":"elohim-app"}"#, 1);
        assert!(m.validate().is_ok(), "app manifests must not require a floor");
    }

    #[test]
    fn pillar_projection_without_floor_accepted() {
        let m = make_manifest("pillar-projection", Some("lamad"), r#"{"version":"1.0"}"#, 1);
        assert!(m.validate().is_ok(), "pillar-projection manifests must not require a floor");
    }

    #[test]
    fn onboarding_without_floor_accepted() {
        let m = make_manifest("onboarding", None, r#"{"steps":[]}"#, 1);
        assert!(m.validate().is_ok(), "onboarding manifests must not require a floor");
    }

    // -----------------------------------------------------------------------
    // Floor 5: valid standing-policy with full floor passes
    // -----------------------------------------------------------------------

    #[test]
    fn valid_standing_policy_passes() {
        let m = make_manifest("standing-policy", None, &standing_floor_payload(), 1);
        assert!(m.validate().is_ok());
    }

    #[test]
    fn valid_tending_policy_passes() {
        let m = make_manifest("tending-policy", None, &tending_floor_payload(), 1);
        assert!(m.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Floor 5: missing floor sub-object rejected
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_without_floor_rejected() {
        let m = make_manifest("standing-policy", None, r#"{"someOtherField": true}"#, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("standing-policy manifest requires payload.floor sub-object"),
            "expected missing-floor error, got: {msg}"
        );
    }

    #[test]
    fn tending_policy_without_floor_rejected() {
        let m = make_manifest("tending-policy", None, r#"{"someOtherField": true}"#, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("tending-policy manifest requires payload.floor sub-object"),
            "expected missing-floor error, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Floor 5: wrong class count (4 classes instead of 5)
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_four_classes_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",        "protection": "p1" },
                    { "class": "cid-targeted-lookup",             "protection": "p2" },
                    { "class": "constitutional-floor-signatures", "protection": "p3" },
                    { "class": "new-voice-baseline",              "protection": "p4" }
                    // missing vulnerable-class-elevation
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("floor.classes must have exactly 5 entries"),
            "expected count error, got: {msg}"
        );
        assert!(msg.contains("4"), "expected '4' in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 5: 6 classes (over the limit) rejected
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_six_classes_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",        "protection": "p1" },
                    { "class": "cid-targeted-lookup",             "protection": "p2" },
                    { "class": "constitutional-floor-signatures", "protection": "p3" },
                    { "class": "new-voice-baseline",              "protection": "p4" },
                    { "class": "vulnerable-class-elevation",      "protection": "p5" },
                    // extra entry — uniqueItems means this would need a new class value,
                    // but the count check fires first before the unknown-class check
                    // because we check len() != 5 before iterating.
                    // We use a fabricated class to hit the count error cleanly.
                    { "class": "local-relationship-reach",        "protection": "p6" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("floor.classes must have exactly 5 entries"),
            "expected count error, got: {msg}"
        );
        assert!(msg.contains("6"), "expected '6' in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 5: unknown class value rejected
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_unknown_class_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",        "protection": "p1" },
                    { "class": "cid-targeted-lookup",             "protection": "p2" },
                    { "class": "constitutional-floor-signatures", "protection": "p3" },
                    { "class": "new-voice-baseline",              "protection": "p4" },
                    { "class": "made-up-class",                   "protection": "this is not a real class" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown floor class"),
            "expected unknown-class error, got: {msg}"
        );
        assert!(msg.contains("made-up-class"), "expected offending class in error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 5: empty protection string rejected
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_empty_protection_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",        "protection": "p1" },
                    { "class": "cid-targeted-lookup",             "protection": "p2" },
                    { "class": "constitutional-floor-signatures", "protection": "p3" },
                    { "class": "new-voice-baseline",              "protection": "" },
                    { "class": "vulnerable-class-elevation",      "protection": "p5" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("empty protection"),
            "expected empty-protection error, got: {msg}"
        );
        assert!(
            msg.contains("new-voice-baseline"),
            "expected offending class in error, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Floor 5: duplicate class entries rejected (uniqueItems mirror)
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_duplicate_classes_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",        "protection": "p1" },
                    { "class": "cid-targeted-lookup",             "protection": "p2" },
                    { "class": "constitutional-floor-signatures", "protection": "p3" },
                    { "class": "new-voice-baseline",              "protection": "p4" },
                    { "class": "local-relationship-reach",        "protection": "duplicate!" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("duplicate floor class"),
            "expected duplicate-class error, got: {msg}"
        );
        assert!(
            msg.contains("local-relationship-reach"),
            "expected offending class in error, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Floor 5: cross-table class (standing-immune class used in tending-policy) rejected
    // -----------------------------------------------------------------------

    #[test]
    fn tending_policy_with_standing_class_rejected() {
        // `local-relationship-reach` is a STANDING-immune class; using it in a
        // tending-policy manifest should be rejected because the allowed set is
        // the tending-immune table, not the standing-immune table.
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "local-relationship-reach",            "protection": "wrong table" },
                    { "class": "community-facts",                     "protection": "p2" },
                    { "class": "custodial-communications",            "protection": "p3" },
                    { "class": "constitutional-updates",              "protection": "p4" },
                    { "class": "elohim-as-counsel-notifications",     "protection": "p5" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("tending-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown floor class"),
            "expected unknown-class error (cross-table), got: {msg}"
        );
        assert!(
            msg.contains("local-relationship-reach"),
            "expected cross-table class name in error, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Floor 5: reverse cross-table (tending-immune class in standing-policy) rejected
    // -----------------------------------------------------------------------

    #[test]
    fn standing_policy_with_tending_class_rejected() {
        let payload = serde_json::json!({
            "floor": {
                "classes": [
                    { "class": "accountability-information",          "protection": "wrong table" },
                    { "class": "cid-targeted-lookup",                 "protection": "p2" },
                    { "class": "constitutional-floor-signatures",     "protection": "p3" },
                    { "class": "new-voice-baseline",                  "protection": "p4" },
                    { "class": "vulnerable-class-elevation",          "protection": "p5" }
                ]
            }
        })
        .to_string();
        let m = make_manifest("standing-policy", None, &payload, 1);
        let result = m.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown floor class"),
            "expected unknown-class error (cross-table), got: {msg}"
        );
        assert!(
            msg.contains("accountability-information"),
            "expected cross-table class name in error, got: {msg}"
        );
    }
}
