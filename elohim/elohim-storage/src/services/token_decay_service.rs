//! Token decay service — continuous limitarian balance decay with dignity floor.
//!
//! Tokens behave like memories: stories that aren't retold fade. The decay rate
//! follows a continuous limitarian curve (spec §3): shape rises with wealth
//! concentration C above the governed extinction setpoint C_target; rank rises
//! with the agent's relational position b̂ = b_i/μ. The dignity floor always
//! silences decay — no agent decays below the sufficientarian gate.
//!
//! ## Rate Function (continuous — replaces the 5-rung step)
//!
//! ```text
//!   shape(C)  = 1 + k_s · relu(C − C_target)
//!   rank(b̂)   = b̂^γ
//!   rate      = clamp(base_rate · shape · rank, 0, k_max)
//! ```
//!
//! apply_decay Steps 4-6 (decay_amount, dignity-floor clamp, audit row) are
//! reused verbatim (spec §3 honest costing). The ObligationLevel label string
//! is kept for audit continuity.

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
// Pure Functions
// ============================================================================

/// The ratifiable gradient parameters (spec §3/§5.1 payload shape; every
/// numeric default is in-wall and marked TBD-operator at the wall layer).
#[derive(Debug, Clone, Copy)]
pub struct GradientConfig {
    pub base_rate: f32,     // default 0.001 — the old Normal rung becomes the curve floor
    pub dignity_floor: f32, // sufficientarian gate; decay OFF below
    pub gamma: f32,         // rank exponent; friction exponent = 1+gamma
    pub k_max: f32,         // per-tick confiscation ceiling (top-side dignity)
    pub c_target: f32,      // governed extinction setpoint (NOT zero — spec §4.4)
    pub k_s: f32,           // shape gain
    pub alpha: f32,         // GE order, wall [1,2]
    pub q: f32,             // top-share quantile
    pub w_e: f32,           // composite weight: shape term
    pub w_s: f32,           // composite weight: tail term
}

impl Default for GradientConfig {
    fn default() -> Self {
        // In-wall, value-laden core defaults (spec §5.1 payload defaults).
        Self { base_rate: 0.001, dignity_floor: 100.0, gamma: 1.0, k_max: 0.05,
               c_target: 0.15, k_s: 0.5, alpha: 1.0, q: 0.01, w_e: 0.6, w_s: 0.4 }
    }
}

/// Continuous limitarian rate (spec §3, signature per the §11 done-when test —
/// arc decision #3: the composite C drives the shape term; S_q is folded into
/// C via w_s, not a separate factor).
///
///   shape(C)   = 1 + k_s · relu(C − C_target)
///   rank(b̂)    = b̂^γ          (b̂ = b_i/μ — the agent's RELATIONAL position)
///   rate       = clamp(base_rate · shape · rank, 0, k_max)
///
/// The sufficientarian floor_factor is applied by the CALLER (decay is skipped
/// entirely below dignity_floor); downstream of the rate, apply_decay Steps 4-6
/// (decay_amount, the .max(dignity_floor) clamp, the audit row) are REUSED
/// VERBATIM (spec §3 honest costing).
pub fn calculate_decay_rate_continuous(b_hat: f32, c: f32, g: &GradientConfig) -> f32 {
    let shape = 1.0 + g.k_s * (c - g.c_target).max(0.0);
    let rank = b_hat.max(0.0).powf(g.gamma);
    (g.base_rate * shape * rank).clamp(0.0, g.k_max)
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
        let balance_row = token_balances::get_balance(conn, ctx, agent_id, governance_layer)?;

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

        // Step 3 — obligation level (AUDIT LABEL ONLY, spec §3) + continuous rate
        // v1: single-substrate governor — "attention" is the only governed
        // substrate (spec §11); multi-substrate routing arrives with
        // substrate_signal-keyed distributions.
        let g = crate::services::limit_gradient_registry::LimitGradientRegistry::core_default(
            "attention", governance_layer);
        // ONE snapshot read serves both C and μ (T5/T6-review: no double-fetch).
        let snap = crate::db::concentration_snapshots::latest_snapshot(
            conn, &ctx.h_app_id, "attention", governance_layer)?;
        let c = snap.as_ref().map(|s| s.c_composite)
            .unwrap_or(g.c_target); // no snapshot yet → C at target → base-rate-only (fail-coherent)
        let snapshot_mu = snap.map(|s| s.mu)
            .unwrap_or(config.median_estimate); // pre-snapshot fallback: the legacy estimate
        let b_hat = if snapshot_mu > 0.0 { balance / snapshot_mu } else { 1.0 };
        // T2-REVIEW FIX (dual-source divergence): the sufficientarian gate reads the
        // SAME source as the Step-5 clamp — config.dignity_floor (the governed row),
        // never the GradientConfig default.
        let rate = if balance < config.dignity_floor {
            0.0 // sufficientarian floor_factor: decay OFF below the floor (spec §3)
        } else {
            calculate_decay_rate_continuous(b_hat, c, &g)
        };
        let level = evaluate_position(balance, b_hat, c, config.dignity_floor, g.c_target);
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
    fn continuous_rate_at_target_equals_base_rate_for_mean_agent() {
        let g = GradientConfig::default();
        let r = calculate_decay_rate_continuous(1.0, g.c_target, &g);
        assert!((r - g.base_rate).abs() < 1e-6, "rate at (b̂=1, C=C_target) must be base_rate, got {r}");
    }

    #[test]
    fn continuous_rate_monotone_in_concentration() {
        let g = GradientConfig::default();
        let r_low = calculate_decay_rate_continuous(2.0, g.c_target, &g);
        let r_mid = calculate_decay_rate_continuous(2.0, g.c_target + 0.2, &g);
        let r_high = calculate_decay_rate_continuous(2.0, g.c_target + 0.5, &g);
        assert!(r_low < r_mid && r_mid < r_high, "rate must rise with C: {r_low} {r_mid} {r_high}");
    }

    #[test]
    fn continuous_rate_monotone_in_relational_position() {
        let g = GradientConfig::default();
        let c = g.c_target + 0.3;
        let r1 = calculate_decay_rate_continuous(1.0, c, &g);
        let r4 = calculate_decay_rate_continuous(4.0, c, &g);
        assert!(r4 > r1, "rate must rise with b̂ (super-linear friction): {r1} vs {r4}");
    }

    #[test]
    fn continuous_rate_clamped_at_k_max() {
        let g = GradientConfig::default();
        let r = calculate_decay_rate_continuous(1_000.0, 0.99, &g);
        assert!((r - g.k_max).abs() < 1e-6, "rate must clamp at k_max, got {r}");
    }

    #[test]
    fn continuous_rate_never_negative_below_target() {
        let g = GradientConfig::default();
        let r = calculate_decay_rate_continuous(0.5, 0.0, &g);
        assert!(r >= 0.0 && r <= g.base_rate, "below-target rate bounded by base_rate·b̂^γ, got {r}");
    }
}
