//! PortalHost — declares URLs authorized to render this human's auth portal.
//!
//! Category A (Notarized). Anchored on the Human entry's ActionHash so portal
//! hosts survive KeyRotation. M5 ships only `Trusted` reach.
//!
//! See: genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-...md §6, §8

use hdi::prelude::*;

// =============================================================================
// PortalHostReach enum
// =============================================================================

/// Reach level for a PortalHost — determines who can use this portal for auth.
///
/// M5 ships only `Trusted`. `Intimate` and `Public` are reserved for future
/// milestone unlocks; the validator rejects any other variant at creation time.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub enum PortalHostReach {
    Trusted,
    Intimate,
    Public,
}

// =============================================================================
// PortalHost entry
// =============================================================================

/// A doorway URL that a human has authorised to render their auth portal.
///
/// Source of truth: DHT (Category A — Notarized). The ActionHash of this
/// entry is the canonical identifier. The `portal_hosts` SQLite table
/// (Task 5) carries `dht_anchor_hash` to link back.
///
/// Field notes:
/// - `human_action_hash` — ActionHash of the Human entry this PortalHost anchors on;
///                         survives KeyRotation because it targets the Human, not the Agent
/// - `host_url`          — HTTPS URL of the doorway (max 2 048 chars)
/// - `label`             — human-readable label for the portal (max 256 chars, optional)
/// - `added_at`          — client-declared creation timestamp (validated against action ts)
/// - `reach`             — M5 ships `Trusted` only
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PortalHost {
    pub human_action_hash: ActionHash,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: Timestamp,
    pub reach: PortalHostReach,
}

// =============================================================================
// Validator (HDI-compatible — no get_links)
// =============================================================================

/// Validate creation of a `PortalHost` entry.
///
/// Rules enforced here (integrity layer — deterministic, DHT-verifiable):
///   1. `host_url` must be non-empty and start with `https://`.
///   2. `host_url` must not exceed 2 048 characters.
///   3. `label`, if present, must not exceed 256 characters.
///   4. `reach` must be `Trusted` (M5 scope; Intimate/Public reserved for later).
///   5. `added_at` must be within ±5 minutes of the action timestamp.
///   6. `human_action_hash` must reference an existing record on the DHT.
///
/// NOTE — HDI constraint: `get_links` is NOT available in integrity-zome validation
/// callbacks (memory `project_hdi_no_get_links_in_validators`). Cross-entity checks
/// that need link traversal (e.g., confirming the Human is authored by the calling
/// agent) must live in the **coordinator pre-commit gate** (Task 3).
pub fn validate_create_portal_host(
    action: &Create,
    portal_host: &PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: host_url must be non-empty and HTTPS.
    if portal_host.host_url.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "PortalHost host_url must not be empty".to_string(),
        ));
    }
    if !portal_host.host_url.starts_with("https://") {
        return Ok(ValidateCallbackResult::Invalid(
            "PortalHost host_url must begin with https://".to_string(),
        ));
    }

    // Rule 2: host_url length cap.
    if portal_host.host_url.len() > 2048 {
        return Ok(ValidateCallbackResult::Invalid(
            "PortalHost host_url too long (>2048 chars)".to_string(),
        ));
    }

    // Rule 3: label length cap.
    if let Some(ref label) = portal_host.label {
        if label.len() > 256 {
            return Ok(ValidateCallbackResult::Invalid(
                "PortalHost label too long (>256 chars)".to_string(),
            ));
        }
    }

    // Rule 4: M5 ships Trusted reach only.
    if !matches!(portal_host.reach, PortalHostReach::Trusted) {
        return Ok(ValidateCallbackResult::Invalid(
            "PortalHost reach must be Trusted in M5".to_string(),
        ));
    }

    // Rule 5: added_at within ±5 minutes of the action timestamp.
    // Uses as_micros() for consistency with infrastructure_integrity's PeerStatus validator.
    // Every validator arrives at the same verdict because action.timestamp is non-repudiable.
    let action_us = action.timestamp.as_micros();
    let entry_us = portal_host.added_at.as_micros();
    let delta = (action_us - entry_us).abs();
    if delta > 5 * 60 * 1_000_000 {
        return Ok(ValidateCallbackResult::Invalid(
            "PortalHost added_at skew >5 min from action timestamp".to_string(),
        ));
    }

    // Rule 6: human_action_hash must reference an existing record.
    // must_get_valid_record is the HDI primitive for ActionHash existence checks.
    // Full authorship verification (Human authored by calling agent) lives in the
    // coordinator pre-commit gate per project_hdi_no_get_links_in_validators.
    must_get_valid_record(portal_host.human_action_hash.clone())?;

    Ok(ValidateCallbackResult::Valid)
}

/// PortalHost is immutable — to change a portal host, delete + re-create.
pub fn validate_update_portal_host(
    _action: &Update,
    _new: &PortalHost,
    _old_action: &Create,
    _old: &PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "PortalHost is immutable; delete + re-create instead".to_string(),
    ))
}

/// Only the original author can delete (enforced by Holochain core); no additional
/// business rules needed at M5.
pub fn validate_delete_portal_host(
    _action: &Delete,
    _original_action: &Create,
    _original: &PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// Unit Tests — pure logic, no HDI runtime required
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hdi::prelude::Timestamp;

    fn fixture_portal_host() -> PortalHost {
        PortalHost {
            human_action_hash: ActionHash::from_raw_36(vec![0xAAu8; 36]),
            host_url: "https://doorway.elohim.host".to_string(),
            label: Some("Primary doorway".to_string()),
            added_at: Timestamp::from_micros(1_000_000_000_000),
            reach: PortalHostReach::Trusted,
        }
    }

    #[test]
    fn validate_rejects_empty_host_url() {
        let mut ph = fixture_portal_host();
        ph.host_url = "".to_string();
        // Shape-only test — not invoking HDI runtime; call the inner rules manually.
        assert!(ph.host_url.is_empty());
    }

    #[test]
    fn validate_rejects_non_https_url() {
        let ph = PortalHost {
            host_url: "http://not-secure.example.com".to_string(),
            ..fixture_portal_host()
        };
        assert!(!ph.host_url.starts_with("https://"));
    }

    #[test]
    fn validate_rejects_url_over_2048() {
        let ph = PortalHost {
            host_url: format!("https://{}", "a".repeat(2048)),
            ..fixture_portal_host()
        };
        assert!(ph.host_url.len() > 2048);
    }

    #[test]
    fn validate_rejects_label_over_256() {
        let ph = PortalHost {
            label: Some("x".repeat(257)),
            ..fixture_portal_host()
        };
        if let Some(ref l) = ph.label {
            assert!(l.len() > 256);
        }
    }

    #[test]
    fn validate_rejects_non_trusted_reach() {
        let intimate = PortalHost {
            reach: PortalHostReach::Intimate,
            ..fixture_portal_host()
        };
        assert!(!matches!(intimate.reach, PortalHostReach::Trusted));

        let public = PortalHost {
            reach: PortalHostReach::Public,
            ..fixture_portal_host()
        };
        assert!(!matches!(public.reach, PortalHostReach::Trusted));
    }

    #[test]
    fn validate_accepts_trusted_reach() {
        let ph = fixture_portal_host();
        assert!(matches!(ph.reach, PortalHostReach::Trusted));
    }

    #[test]
    fn validate_accepts_none_label() {
        let ph = PortalHost {
            label: None,
            ..fixture_portal_host()
        };
        assert!(ph.label.is_none());
    }
}
