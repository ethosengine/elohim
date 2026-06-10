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
use crate::db::{concentration_snapshots, responsibility_demand_configs, token_balances};
use crate::error::StorageError;
use crate::services::concentration_service::ConcentrationService;
use crate::services::limit_gradient_registry::LimitGradientRegistry;

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
    Elevated { visibility_required: bool },

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
    /// Evaluate the obligation level (audit label) for an agent in a governance layer.
    ///
    /// 1. Fetches the demand curve config for the layer.
    /// 2. If no config or enforcement is inactive, returns `Normal`.
    /// 3. Fetches the agent's current balance (defaults to 0.0 if absent).
    /// 4. Reads the snapshot C and relational μ; computes b̂.
    /// 5. Delegates to the pure `evaluate_position` function.
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

        let g = LimitGradientRegistry::core_default("attention", governance_layer);
        let c = ConcentrationService::effective_c(
                conn, &ctx.h_app_id, "attention", governance_layer)?
            .unwrap_or(g.c_target);
        let snapshot_mu = concentration_snapshots::latest_snapshot(
                conn, &ctx.h_app_id, "attention", governance_layer)?
            .map(|s| s.mu)
            .unwrap_or(config.median_estimate);
        let b_hat = if snapshot_mu > 0.0 { balance / snapshot_mu } else { 1.0 };

        Ok(evaluate_position(balance, b_hat, c, config.dignity_floor, g.c_target))
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

/// AUDIT LABEL ONLY (spec §3) — derived from the snapshot C and the agent's
/// relational position; no longer a rate driver. Bands: Supported below the
/// dignity floor; then C-relative bands at the gradient's target.
///
/// Boundary semantics:
/// - `balance < dignity_floor` → Supported
/// - `c <= c_target` → Normal (concentration at or below target, regardless of b̂)
/// - `c > c_target && b̂ < 2.0` → Elevated
/// - `c > c_target && b̂ < 5.0` → High
/// - `c > c_target && b̂ >= 5.0` → Extreme
///
/// `dignity_floor` must come from the governed config row (never a GradientConfig
/// default); `c_target` from the gradient registry.
pub fn evaluate_position(
    balance: f32,
    b_hat: f32,
    c: f32,
    dignity_floor: f32,
    c_target: f32,
) -> ObligationLevel {
    if balance < dignity_floor {
        ObligationLevel::Supported
    } else if c <= c_target {
        ObligationLevel::Normal
    } else if b_hat < 2.0 {
        ObligationLevel::Elevated { visibility_required: true }
    } else if b_hat < 5.0 {
        ObligationLevel::High { stewardship_required: true, justification_required: true }
    } else {
        ObligationLevel::Extreme { elohim_review_required: true, constitutional_justification: true }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Shared test constants: dignity_floor=100, c_target=0.15 (gradient default).
    const DIGNITY_FLOOR: f32 = 100.0;
    const C_TARGET: f32 = 0.15;

    #[test]
    fn evaluate_supported_below_floor() {
        // Any balance strictly below the dignity floor → Supported,
        // regardless of concentration or relational position.
        let level = evaluate_position(50.0, 0.05, 0.5, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(level, ObligationLevel::Supported);
    }

    #[test]
    fn evaluate_normal_at_or_below_target() {
        // When C <= C_target the system is not over-concentrated — Normal regardless
        // of b̂ (high-balance agent in a well-distributed layer).
        let level_at_target = evaluate_position(5000.0, 5.0, C_TARGET, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(level_at_target, ObligationLevel::Normal);

        let level_below_target = evaluate_position(500.0, 0.5, 0.05, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(level_below_target, ObligationLevel::Normal);
    }

    #[test]
    fn evaluate_bands_by_relational_position() {
        // With C above target (concentrated system), band is driven by b̂.
        let c_high = C_TARGET + 0.2; // C > c_target: shape engaged

        // b̂ < 2.0 → Elevated
        let elevated = evaluate_position(200.0, 1.5, c_high, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(elevated, ObligationLevel::Elevated { visibility_required: true });

        // b̂ in [2.0, 5.0) → High
        let high = evaluate_position(200.0, 3.0, c_high, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(high, ObligationLevel::High {
            stewardship_required: true,
            justification_required: true,
        });

        // b̂ >= 5.0 → Extreme
        let extreme = evaluate_position(200.0, 6.0, c_high, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(extreme, ObligationLevel::Extreme {
            elohim_review_required: true,
            constitutional_justification: true,
        });
    }

    #[test]
    fn evaluate_floor_boundary_inclusive() {
        // Exactly at dignity_floor → NOT Supported (balance is not strictly below floor)
        // With c <= c_target this lands Normal.
        let at_floor = evaluate_position(DIGNITY_FLOOR, 1.0, 0.05, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(at_floor, ObligationLevel::Normal);
    }

    #[test]
    fn evaluate_floor_trumps_concentration() {
        // Even with extreme concentration and high b̂, below-floor → Supported.
        let level = evaluate_position(50.0, 10.0, 0.99, DIGNITY_FLOOR, C_TARGET);
        assert_eq!(level, ObligationLevel::Supported);
    }
}
