//! Slice-2b — per-instance validator for `replicates-commons` commitments.
//!
//! Three-stage validation mirroring `replicates_dwelling_validator.rs`:
//!   1. Schema/structural check on the typed [`ReplicatesContentPayload`].
//!   2. Donut check — **only for the `capacity` variant** (a byte-budget pledge
//!      to the commons tier carries `ratio_attestation`). The `content` variant
//!      is a provide of a specific EPR with no counterparty and NO donut.
//!   3. Substrate bounds check (event-time only; at author time the commitment
//!      is not yet notarized, so `validate_typed_for_creation_commons` stops
//!      after the donut — same split as dwelling's `validate_typed_for_creation`).
//!
//! Pattern: per `project_bounds_validator_pattern` memory; commons is the
//! commons-tier instance after the dwelling-tier instance.
//!
//! ## Typed-view shape this consumes
//!
//! The landed `ReplicatesContentPayload` (T1-T4, `elohim-views`) is
//! `#[serde(tag = "variant")]` — the variant IS the discriminator, so there is
//! NO `action` field on either variant (the DNA action discriminator lives one
//! level up, on the `Commitment` entry). The `Capacity` variant carries no
//! `reach` field either; `Content` carries `reach`. The attestation struct is
//! `CommonsRatioAttestation`. The validator destructures exactly these names —
//! the validator follows the view.

use crate::services::constitutional_ratio_registry;
use elohim_views::replicates_commons::ReplicatesContentPayload;

#[derive(Debug, thiserror::Error)]
pub enum CommonsValidationError {
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("constitutional ratio breach: {0}")]
    ConstitutionalRatio(String),
}

/// Validate a typed [`ReplicatesContentPayload`] at **commitment-author time**.
///
/// Runs structural checks on the variant fields, then the donut check **only**
/// for the `capacity` variant. The `content` variant skips the donut entirely
/// (no counterparty, no ratio attestation). Does NOT run the substrate
/// `bounds_validator` (event-time gate — the commitment does not yet exist in
/// the conductor at author time).
pub fn validate_typed_for_creation_commons(
    payload: &ReplicatesContentPayload,
) -> Result<(), CommonsValidationError> {
    match payload {
        ReplicatesContentPayload::Content {
            head_ref,
            reach,
            bounds,
            ..
        } => {
            // ── Stage 1: structural (content) — NO donut ─────────────────────
            if head_ref.is_empty() {
                return Err(CommonsValidationError::Schema(
                    "content variant head_ref must not be empty".into(),
                ));
            }
            if reach != "commons" {
                return Err(CommonsValidationError::Schema(format!(
                    "reach must be 'commons', got '{reach}'"
                )));
            }
            if bounds.reach_ceiling != "commons" {
                return Err(CommonsValidationError::Schema(format!(
                    "bounds.reach_ceiling must be 'commons', got '{}'",
                    bounds.reach_ceiling
                )));
            }
            if bounds.rate_per_minute == 0 {
                return Err(CommonsValidationError::Schema(
                    "bounds.rate_per_minute must be > 0".into(),
                ));
            }
            Ok(())
        }
        ReplicatesContentPayload::Capacity {
            commons_bytes,
            bounds,
            ratio_attestation: att,
        } => {
            // ── Stage 1: structural (capacity) ───────────────────────────────
            if *commons_bytes == 0 {
                return Err(CommonsValidationError::Schema(
                    "commons_bytes must be > 0".into(),
                ));
            }
            if bounds.reach_ceiling != "commons" {
                return Err(CommonsValidationError::Schema(format!(
                    "bounds.reach_ceiling must be 'commons', got '{}'",
                    bounds.reach_ceiling
                )));
            }

            // ── Stage 2: donut (capacity ONLY) ───────────────────────────────
            let provenance = constitutional_ratio_registry::effective_ratios();
            let effective = provenance.ratios;
            let manifest_cid = provenance.manifest_cid;

            // (a) Sum-to-100
            let sum = att.commons_pct as u16
                + att.dwelling_pct as u16
                + att.collective_pct as u16
                + att.free_pct as u16;
            if sum != 100 {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "ratio_attestation pct sum {sum} != 100"
                )));
            }

            // (b) Attested values must match effective ratios.
            if att.commons_pct != effective.commons_pct {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "attested commons_pct {} != effective {} (manifest {})",
                    att.commons_pct, effective.commons_pct, manifest_cid
                )));
            }
            if att.dwelling_pct != effective.dwelling_pct {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "attested dwelling_pct {} != effective {}",
                    att.dwelling_pct, effective.dwelling_pct
                )));
            }
            if att.collective_pct != effective.collective_pct {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "attested collective_pct {} != effective {}",
                    att.collective_pct, effective.collective_pct
                )));
            }
            if att.free_pct != effective.free_pct {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "attested free_pct {} != effective {}",
                    att.free_pct, effective.free_pct
                )));
            }

            // (c) Floor check via declaration.
            if att.commons_pct < constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "attested commons_pct {} below DNA floor {}",
                    att.commons_pct,
                    constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT
                )));
            }

            // (d) Provenance match.
            if att.effective_ratio_cid != manifest_cid {
                return Err(CommonsValidationError::ConstitutionalRatio(format!(
                    "ratio_attestation effective_ratio_cid {} != current manifest {}",
                    att.effective_ratio_cid, manifest_cid
                )));
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_views::replicates_commons::{
        CommonsBounds, CommonsRatioAttestation, ReplicatesContentPayload,
    };

    fn content_payload(head_ref: &str) -> ReplicatesContentPayload {
        ReplicatesContentPayload::Content {
            head_ref: head_ref.to_string(),
            closure_rule: Some("direct".to_string()),
            reach: "commons".to_string(),
            bounds: CommonsBounds {
                rate_per_minute: 6,
                reach_ceiling: "commons".to_string(),
            },
        }
    }

    fn capacity_payload(commons_bytes: u64) -> ReplicatesContentPayload {
        let provenance = constitutional_ratio_registry::effective_ratios();
        let r = provenance.ratios;
        ReplicatesContentPayload::Capacity {
            commons_bytes,
            bounds: CommonsBounds {
                rate_per_minute: 6,
                reach_ceiling: "commons".to_string(),
            },
            ratio_attestation: CommonsRatioAttestation {
                commons_pct: r.commons_pct,
                dwelling_pct: r.dwelling_pct,
                collective_pct: r.collective_pct,
                free_pct: r.free_pct,
                effective_ratio_cid: provenance.manifest_cid,
            },
        }
    }

    #[test]
    fn content_variant_passes_without_donut() {
        // A content provide has no counterparty / no ratio_attestation: it must
        // pass the validator (schema only — donut is skipped).
        let payload = content_payload("bafy-epr-head");
        assert!(
            validate_typed_for_creation_commons(&payload).is_ok(),
            "well-formed content variant must pass (no donut)"
        );
    }

    #[test]
    fn content_variant_wrong_reach_rejected() {
        let mut payload = content_payload("bafy-epr-head");
        if let ReplicatesContentPayload::Content { reach, .. } = &mut payload {
            *reach = "household".to_string();
        }
        assert!(matches!(
            validate_typed_for_creation_commons(&payload),
            Err(CommonsValidationError::Schema(_))
        ));
    }

    #[test]
    fn content_variant_empty_head_ref_rejected() {
        let payload = content_payload("");
        assert!(matches!(
            validate_typed_for_creation_commons(&payload),
            Err(CommonsValidationError::Schema(_))
        ));
    }

    #[test]
    fn capacity_variant_well_formed_passes_donut() {
        let payload = capacity_payload(25_000_000_000);
        assert!(
            validate_typed_for_creation_commons(&payload).is_ok(),
            "well-formed capacity variant must pass the donut"
        );
    }

    #[test]
    fn capacity_variant_zero_bytes_rejected() {
        let payload = capacity_payload(0);
        assert!(matches!(
            validate_typed_for_creation_commons(&payload),
            Err(CommonsValidationError::Schema(_))
        ));
    }

    #[test]
    fn capacity_variant_ratio_sum_not_100_rejected() {
        let mut payload = capacity_payload(25_000_000_000);
        if let ReplicatesContentPayload::Capacity {
            ratio_attestation, ..
        } = &mut payload
        {
            // Break sum-to-100 without touching effective_ratio_cid.
            ratio_attestation.free_pct = ratio_attestation.free_pct.saturating_add(5);
        }
        assert!(matches!(
            validate_typed_for_creation_commons(&payload),
            Err(CommonsValidationError::ConstitutionalRatio(_))
        ));
    }

    #[test]
    fn capacity_variant_wrong_effective_ratio_cid_rejected() {
        let mut payload = capacity_payload(25_000_000_000);
        if let ReplicatesContentPayload::Capacity {
            ratio_attestation, ..
        } = &mut payload
        {
            ratio_attestation.effective_ratio_cid = "bafkrei-stale-manifest".to_string();
        }
        assert!(matches!(
            validate_typed_for_creation_commons(&payload),
            Err(CommonsValidationError::ConstitutionalRatio(_))
        ));
    }
}
