//! Responsibility demand curve — evaluates an agent's obligation level based on
//! their token position within a governance layer.
//!
//! The curve is initialized from the 92% consensus (Ariely/Norton, 2011):
//! wealthiest 10-20x the poorest, healthy middle, nobody destitute.
//!
//! **Friction is obligation, not taxation.** The network does not take tokens.
//! It demands that agents DO something with their position.
//!
//! Sprint 2: all levels allow transfers (Extreme logs a warning). Full
//! enforcement with elohim discernment infrastructure arrives in Sprint 3.

use serde::Serialize;

use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::models::ResponsibilityDemandConfig;
use crate::db::{responsibility_demand_configs, token_balances};
use crate::error::StorageError;

// ============================================================================
// Obligation Level
// ============================================================================

/// The level of responsibility obligation for an agent at their current
/// token position in a governance layer.
///
/// Levels map to the demand curve thresholds configured per governance layer
/// by the community's ratified `ResponsibilityDemandConfig`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ObligationLevel {
    /// Below the dignity floor — agent is in supported range.
    /// No obligations; the community is supporting this agent.
    Supported,

    /// Between dignity floor and median — ordinary participant.
    /// Standard participation obligations apply.
    Normal,

    /// Between median and soft ceiling (median × soft_ceiling_multiplier).
    /// Elevated participation expected; stewardship visibility increases.
    Elevated {
        visibility_required: bool,
    },

    /// Between soft ceiling and hard ceiling (median × hard_ceiling_multiplier).
    /// Active stewardship required; justification for concentration expected.
    High {
        stewardship_required: bool,
        justification_required: bool,
    },

    /// At or above hard ceiling (median × hard_ceiling_multiplier).
    /// Full constitutional review of position; elohim discernment triggered in Sprint 3.
    Extreme {
        elohim_review_required: bool,
        constitutional_justification: bool,
    },
}

// ============================================================================
// Service
// ============================================================================

pub struct ResponsibilityDemandService;

impl ResponsibilityDemandService {
    /// Evaluate the obligation level for an agent in a governance layer.
    ///
    /// 1. Fetches the demand curve config for the layer.
    /// 2. If no config or enforcement is inactive, returns `Normal`.
    /// 3. Fetches the agent's current balance (defaults to 0.0 if absent).
    /// 4. Delegates to the pure `evaluate_position` function.
    pub fn evaluate(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<ObligationLevel, StorageError> {
        let config =
            responsibility_demand_configs::get_config_for_layer(conn, ctx, governance_layer)?;

        let config = match config {
            None => return Ok(ObligationLevel::Normal),
            Some(c) if c.enforcement_active == 0 => return Ok(ObligationLevel::Normal),
            Some(c) => c,
        };

        let balance = token_balances::get_balance(conn, ctx, agent_id, governance_layer)?
            .map(|b| b.balance)
            .unwrap_or(0.0_f32);

        Ok(evaluate_position(balance, &config))
    }

    /// Gate check before a token transfer.
    ///
    /// Sprint 2: all obligation levels allow the transfer to proceed.
    /// Extreme level emits a warning to stderr so operators can observe the
    /// pattern. Full enforcement (Extreme → block, High → require justification)
    /// ships in Sprint 3 with the elohim discernment infrastructure.
    pub fn check_transfer_allowed(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        from_agent: &str,
        amount: f32,
        governance_layer: &str,
    ) -> Result<(), StorageError> {
        let level = Self::evaluate(conn, ctx, from_agent, governance_layer)?;

        if let ObligationLevel::Extreme { .. } = &level {
            eprintln!(
                "[responsibility-demand] WARN: agent {} is at Extreme obligation level \
                 in layer {} (transferring {:.6}). \
                 Sprint 3 enforcement will block or require elohim review.",
                from_agent, governance_layer, amount
            );
        }

        Ok(())
    }
}

// ============================================================================
// Pure Curve Function
// ============================================================================

/// Pure function — the responsibility demand curve itself.
///
/// Maps a token balance to an obligation level using the thresholds in
/// `config`. No I/O; fully testable without a database.
///
/// Boundary semantics (inclusive lower, exclusive upper):
/// - `[0, dignity_floor)` → Supported
/// - `[dignity_floor, median)` → Normal
/// - `[median, median × soft_ceiling_multiplier)` → Elevated
/// - `[median × soft_ceiling_multiplier, median × hard_ceiling_multiplier)` → High
/// - `[median × hard_ceiling_multiplier, ∞)` → Extreme
pub fn evaluate_position(balance: f32, config: &ResponsibilityDemandConfig) -> ObligationLevel {
    let median = config.median_estimate;
    let soft_ceiling = median * config.soft_ceiling_multiplier;
    let hard_ceiling = median * config.hard_ceiling_multiplier;

    if balance < config.dignity_floor {
        ObligationLevel::Supported
    } else if balance < median {
        ObligationLevel::Normal
    } else if balance < soft_ceiling {
        ObligationLevel::Elevated {
            visibility_required: true,
        }
    } else if balance < hard_ceiling {
        ObligationLevel::High {
            stewardship_required: true,
            justification_required: true,
        }
    } else {
        ObligationLevel::Extreme {
            elohim_review_required: true,
            constitutional_justification: true,
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `ResponsibilityDemandConfig` with known values for curve tests.
    ///
    /// dignity_floor = 100, median = 1000, soft_ceiling_mult = 10 (→ 10 000),
    /// hard_ceiling_mult = 20 (→ 20 000).
    fn test_config() -> ResponsibilityDemandConfig {
        ResponsibilityDemandConfig {
            id: "test-config".to_string(),
            h_app_id: "test-app".to_string(),
            governance_layer: "global".to_string(),
            dignity_floor: 100.0,
            median_estimate: 1000.0,
            soft_ceiling_multiplier: 10.0,
            hard_ceiling_multiplier: 20.0,
            social_contract_health: 1.0,
            enforcement_active: 1,
            ratified_by: None,
            ratified_at: None,
            dht_anchor_hash: None,
            created_at: "2026-04-01T00:00:00Z".to_string(),
            updated_at: "2026-04-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_below_dignity_floor() {
        let config = test_config();
        let level = evaluate_position(50.0, &config);
        assert_eq!(level, ObligationLevel::Supported);
    }

    #[test]
    fn test_normal_range() {
        let config = test_config();
        let level = evaluate_position(500.0, &config);
        assert_eq!(level, ObligationLevel::Normal);
    }

    #[test]
    fn test_elevated_range() {
        let config = test_config();
        // 5000 is between median (1000) and soft ceiling (10 000)
        let level = evaluate_position(5000.0, &config);
        assert_eq!(
            level,
            ObligationLevel::Elevated {
                visibility_required: true
            }
        );
    }

    #[test]
    fn test_high_range() {
        let config = test_config();
        // 15 000 is between soft ceiling (10 000) and hard ceiling (20 000)
        let level = evaluate_position(15000.0, &config);
        assert_eq!(
            level,
            ObligationLevel::High {
                stewardship_required: true,
                justification_required: true,
            }
        );
    }

    #[test]
    fn test_extreme_range() {
        let config = test_config();
        // 25 000 is at or above hard ceiling (20 000)
        let level = evaluate_position(25000.0, &config);
        assert_eq!(
            level,
            ObligationLevel::Extreme {
                elohim_review_required: true,
                constitutional_justification: true,
            }
        );
    }

    #[test]
    fn test_at_boundaries() {
        let config = test_config();

        // Exactly at dignity_floor (100) → Normal (inclusive lower bound)
        let at_floor = evaluate_position(100.0, &config);
        assert_eq!(at_floor, ObligationLevel::Normal);

        // Exactly at median (1000) → Elevated (inclusive lower bound of elevated range)
        let at_median = evaluate_position(1000.0, &config);
        assert_eq!(
            at_median,
            ObligationLevel::Elevated {
                visibility_required: true
            }
        );
    }
}
