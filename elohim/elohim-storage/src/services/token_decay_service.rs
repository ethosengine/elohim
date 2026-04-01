//! Token decay service — obligation-proportional balance decay with dignity floor.
//!
//! Tokens behave like memories: stories that aren't retold fade. The decay rate
//! scales with an agent's obligation level — the more concentrated their position,
//! the faster unsteward-ed tokens erode. The dignity floor is always protected.
//!
//! ## Decay Rates by Obligation Level
//!
//! | Level     | Rate per period | Notes |
//! |-----------|----------------|-------|
//! | Supported | 0.0%           | Protected — community is supporting this agent |
//! | Normal    | 0.1%           | Ordinary participant; gentle pressure to remain active |
//! | Elevated  | 0.5%           | Increased visibility obligations; faster fade if dormant |
//! | High      | 2.0%           | Active stewardship expected; significant decay if idle |
//! | Extreme   | 5.0%           | Constitutional review; fastest decay to prevent lock-in |

use diesel::SqliteConnection;
use serde::Serialize;
use uuid::Uuid;

use crate::db::context::AppContext;
use crate::db::models::NewTokenDecayEvent;
use crate::db::{responsibility_demand_configs, token_balances, token_decay_events};
use crate::error::StorageError;
use crate::services::responsibility_demand_service::{evaluate_position, ObligationLevel};

// ============================================================================
// Result Type
// ============================================================================

/// Outcome of a single decay application for an agent + governance layer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayResult {
    /// Whether any decay was actually applied this period.
    pub decay_applied: bool,
    /// Amount subtracted from the balance (0.0 if not applied).
    pub amount: f32,
    /// Balance before decay was applied.
    pub balance_before: f32,
    /// Balance after decay was applied.
    pub balance_after: f32,
    /// String label for the obligation level that determined the decay rate.
    pub obligation_level: String,
    /// True if the dignity floor clamped the decay (actual decay < calculated decay).
    pub dignity_floor_protected: bool,
}

// ============================================================================
// Pure Function
// ============================================================================

/// Pure decay rate lookup — no I/O, fully unit-testable.
///
/// Returns the fraction of balance to remove per decay period for the given
/// obligation level. `Supported` always returns 0.0 (no decay while protected).
pub fn calculate_decay_rate(level: &ObligationLevel) -> f32 {
    match level {
        ObligationLevel::Supported => 0.0,
        ObligationLevel::Normal => 0.001,
        ObligationLevel::Elevated { .. } => 0.005,
        ObligationLevel::High { .. } => 0.02,
        ObligationLevel::Extreme { .. } => 0.05,
    }
}

/// Human-readable label for an obligation level (used in audit records).
fn obligation_level_label(level: &ObligationLevel) -> &'static str {
    match level {
        ObligationLevel::Supported => "supported",
        ObligationLevel::Normal => "normal",
        ObligationLevel::Elevated { .. } => "elevated",
        ObligationLevel::High { .. } => "high",
        ObligationLevel::Extreme { .. } => "extreme",
    }
}

// ============================================================================
// Service
// ============================================================================

pub struct TokenDecayService;

impl TokenDecayService {
    /// Apply one decay period for an agent in a governance layer.
    ///
    /// Steps:
    /// 1. Fetch demand curve config — if absent or enforcement_active == 0, skip.
    /// 2. Fetch current balance — if zero or absent, skip.
    /// 3. Evaluate obligation level via the demand curve.
    /// 4. Calculate `decay_amount = balance * rate`.
    /// 5. Clamp to dignity floor: `new_balance = (balance - decay_amount).max(dignity_floor)`.
    /// 6. If actual decay > 0: debit balance, record `token_decay_events` row.
    pub fn apply_decay(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<DecayResult, StorageError> {
        // Step 1 — config check
        let config =
            responsibility_demand_configs::get_config_for_layer(conn, ctx, governance_layer)?;

        let config = match config {
            None => {
                return Ok(DecayResult {
                    decay_applied: false,
                    amount: 0.0,
                    balance_before: 0.0,
                    balance_after: 0.0,
                    obligation_level: "none".into(),
                    dignity_floor_protected: false,
                })
            }
            Some(c) if c.enforcement_active == 0 => {
                return Ok(DecayResult {
                    decay_applied: false,
                    amount: 0.0,
                    balance_before: 0.0,
                    balance_after: 0.0,
                    obligation_level: "unenforced".into(),
                    dignity_floor_protected: false,
                })
            }
            Some(c) => c,
        };

        // Step 2 — balance check
        let balance_row =
            token_balances::get_balance(conn, ctx, agent_id, governance_layer)?;

        let balance = match balance_row {
            None => {
                return Ok(DecayResult {
                    decay_applied: false,
                    amount: 0.0,
                    balance_before: 0.0,
                    balance_after: 0.0,
                    obligation_level: "no-balance".into(),
                    dignity_floor_protected: false,
                })
            }
            Some(ref row) if row.balance <= 0.0 => {
                return Ok(DecayResult {
                    decay_applied: false,
                    amount: 0.0,
                    balance_before: row.balance,
                    balance_after: row.balance,
                    obligation_level: "zero-balance".into(),
                    dignity_floor_protected: false,
                })
            }
            Some(ref row) => row.balance,
        };

        // Step 3 — obligation level
        let level = evaluate_position(balance, &config);
        let rate = calculate_decay_rate(&level);
        let label = obligation_level_label(&level).to_string();

        // Step 4 — raw decay
        let decay_amount = balance * rate;

        // Step 5 — dignity floor clamp
        let new_balance = (balance - decay_amount).max(config.dignity_floor);
        let actual_decay = balance - new_balance;
        let dignity_floor_protected = actual_decay < decay_amount - f32::EPSILON;

        // Step 6 — skip if no real decay (already at or below floor, or rate == 0.0)
        if actual_decay <= 0.0 {
            return Ok(DecayResult {
                decay_applied: false,
                amount: 0.0,
                balance_before: balance,
                balance_after: balance,
                obligation_level: label,
                dignity_floor_protected,
            });
        }

        // Debit balance
        token_balances::debit_balance(conn, ctx, agent_id, governance_layer, actual_decay)?;

        // Record audit event
        let decay_id = Uuid::new_v4().to_string();
        let event = NewTokenDecayEvent {
            id: &decay_id,
            h_app_id: &ctx.h_app_id,
            agent_id,
            governance_layer,
            balance_before: balance,
            balance_after: new_balance,
            decay_amount: actual_decay,
            obligation_level: &label,
            dignity_floor: config.dignity_floor,
        };
        token_decay_events::create_decay_event(conn, ctx, event)?;

        Ok(DecayResult {
            decay_applied: true,
            amount: actual_decay,
            balance_before: balance,
            balance_after: new_balance,
            obligation_level: label,
            dignity_floor_protected,
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_rate_supported_zero() {
        let rate = calculate_decay_rate(&ObligationLevel::Supported);
        assert_eq!(rate, 0.0, "Supported must have zero decay");
    }

    #[test]
    fn test_decay_rate_normal() {
        let rate = calculate_decay_rate(&ObligationLevel::Normal);
        assert!(
            (rate - 0.001).abs() < f32::EPSILON,
            "Normal decay rate must be 0.001, got {rate}"
        );
    }

    #[test]
    fn test_decay_rate_elevated() {
        let rate = calculate_decay_rate(&ObligationLevel::Elevated {
            visibility_required: true,
        });
        assert!(
            (rate - 0.005).abs() < f32::EPSILON,
            "Elevated decay rate must be 0.005, got {rate}"
        );
    }

    #[test]
    fn test_decay_rate_high() {
        let rate = calculate_decay_rate(&ObligationLevel::High {
            stewardship_required: true,
            justification_required: true,
        });
        assert!(
            (rate - 0.02).abs() < f32::EPSILON,
            "High decay rate must be 0.02, got {rate}"
        );
    }

    #[test]
    fn test_decay_rate_extreme() {
        let rate = calculate_decay_rate(&ObligationLevel::Extreme {
            elohim_review_required: true,
            constitutional_justification: true,
        });
        assert!(
            (rate - 0.05).abs() < f32::EPSILON,
            "Extreme decay rate must be 0.05, got {rate}"
        );
    }
}
