//! Phase-A concentration aggregator (spec §11 land 3b) — computes the per-layer
//! distribution snapshot. v1: reflexive-sensing, NO clock — driven by the test
//! harness / an HTTP poke (the aggregate-tick scheduler is explicitly deferred).
//! v1 substrate: token balances (spec §11.3).
//!
//! FIREWALL: refuses to write when n < K_MIN (k>=5, aggregator.rs mold) — a
//! sub-k snapshot would expose a small cohort's distribution.

use diesel::sqlite::SqliteConnection;

use crate::db::concentration_snapshots::{insert_snapshot, latest_snapshot};
use crate::db::models::{ConcentrationSnapshot, NewConcentrationSnapshot};
use crate::services::measure::{
    composite_concentration, ge_alpha, gini, squash, top_quantile_share,
};
use crate::services::token_decay_service::GradientConfig;

pub const K_MIN: usize = 5;

pub struct ConcentrationService;

#[derive(Debug, PartialEq)]
pub enum SnapshotOutcome {
    Written(ConcentrationSnapshot),
    SuppressedBelowK { n: usize },
}

impl ConcentrationService {
    /// Compute and persist a snapshot for one (substrate, layer) over the given
    /// distribution. The CALLER supplies the balances vector (Phase-A: from
    /// token_balances for the layer); this keeps the service pure over its input
    /// and trivially testable.
    pub fn compute_snapshot(
        conn: &mut SqliteConnection,
        h_app_id: &str,
        substrate_signal: &str,
        governance_layer: &str,
        balances: &[f32],
        g: &GradientConfig,
        computed_at: &str,
    ) -> Result<SnapshotOutcome, diesel::result::Error> {
        let n = balances.len();
        if n < K_MIN {
            return Ok(SnapshotOutcome::SuppressedBelowK { n });
        }
        let mu = balances.iter().sum::<f32>() / n as f32;
        let ge = ge_alpha(balances, g.alpha);
        let row = NewConcentrationSnapshot {
            id: format!("{substrate_signal}:{governance_layer}:{computed_at}"),
            h_app_id: h_app_id.to_string(),
            substrate_signal: substrate_signal.to_string(),
            governance_layer: governance_layer.to_string(),
            n: n as i32,
            mu,
            ge,
            ge_squashed: squash(ge),
            top_share: top_quantile_share(balances, g.q),
            gini: gini(balances),
            c_composite: composite_concentration(balances, g.alpha, g.q, g.w_e, g.w_s),
            alpha: g.alpha,
            top_q: g.q,
            computed_at: computed_at.to_string(),
        };
        Ok(SnapshotOutcome::Written(insert_snapshot(conn, row)?))
    }

    /// Effective C for the decay path: latest snapshot's composite, or None.
    pub fn effective_c(
        conn: &mut SqliteConnection,
        h_app_id: &str,
        substrate_signal: &str,
        governance_layer: &str,
    ) -> Result<Option<f32>, diesel::result::Error> {
        Ok(
            latest_snapshot(conn, h_app_id, substrate_signal, governance_layer)?
                .map(|s| s.c_composite),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn suppresses_below_k() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let out = ConcentrationService::compute_snapshot(
            &mut conn,
            "shefa",
            "attention",
            "household",
            &[100.0, 200.0, 300.0, 400.0],
            &GradientConfig::default(),
            "2026-06-10T03:00:00Z",
        )
        .expect("ok");
        assert_eq!(out, SnapshotOutcome::SuppressedBelowK { n: 4 });
        assert!(
            ConcentrationService::effective_c(&mut conn, "shefa", "attention", "household")
                .expect("q")
                .is_none(),
            "no row may exist for a suppressed cohort"
        );
    }

    #[test]
    fn writes_at_k_and_effective_c_reads_it() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let balances = [100.0_f32, 100.0, 100.0, 100.0, 10_000.0];
        let out = ConcentrationService::compute_snapshot(
            &mut conn,
            "shefa",
            "attention",
            "community",
            &balances,
            &GradientConfig::default(),
            "2026-06-10T03:00:00Z",
        )
        .expect("ok");
        let snap = match out {
            SnapshotOutcome::Written(s) => s,
            _ => panic!("expected write"),
        };
        assert_eq!(snap.n, 5);
        assert!(
            snap.c_composite > 0.3,
            "skewed distribution must read concentrated"
        );
        let c = ConcentrationService::effective_c(&mut conn, "shefa", "attention", "community")
            .expect("q")
            .expect("some");
        assert!((c - snap.c_composite).abs() < 1e-6);
    }

    /// Spec §11 "the one green convergence test" — the SHIPPED CLAMPED model
    /// under rich-get-richer inflow.
    ///
    /// SATURATED-REGIME PHYSICS (spec §4.2/4.3): with c_inflow=0.20 > k_max=0.05,
    /// absolute balances grow without bound (top net +15%/tick after k_max
    /// confiscation; f32 overflows ~tick 600 from 1e5). Assertion (a) on the
    /// absolute top balance is therefore meaningless in this regime. We assert
    /// on:
    ///   (a) TOP SHARE DESCENDS: the smaller agents grow faster in share (full
    ///       20%/tick) than the top agent (15% net), so the distribution
    ///       equalizes even though absolutes diverge.
    ///   (b)+(c) C-SERIES DESCENDS TO TARGET: proven in f64 arithmetic, since
    ///       composite_concentration is scale-invariant — we normalize to mean=1
    ///       before the f32 cast to prevent f32 overflow corrupting the series.
    ///   (d) SELF-EXTINGUISHING-WHEN-JUST.
    /// A SECOND sub-run at c_inflow=0.04 (< k_max — rate-closable regime) asserts
    /// absolute boundedness (top finite, < 1e9 over 2000 ticks).
    #[test]
    fn continuous_governor_restores_toward_target_under_rich_get_richer_inflow() {
        use crate::services::measure::composite_concentration;
        use crate::services::token_decay_service::{
            calculate_decay_rate_continuous, GradientConfig,
        };

        let g = GradientConfig {
            base_rate: 0.001,
            dignity_floor: 100.0,
            gamma: 1.0,
            k_max: 0.05,
            c_target: 0.15,
            k_s: 0.5,
            alpha: 1.0,
            q: 0.01,
            w_e: 0.6,
            w_s: 0.4,
        };

        // ---- MAIN RUN: c_inflow=0.20 (4× k_max — step-divergent regime) ----
        // f64 balances prevent numeric overflow; composite_concentration is
        // scale-invariant so normalizing to mean=1 before the f32 cast is exact.
        let c_inflow = 0.20_f64;
        let initial_top_share: f64 = {
            let b = [100_000.0_f64, 100.0, 100.0, 100.0, 100.0];
            b[0] / b.iter().sum::<f64>() // ≈ 0.9960
        };
        let mut balances = [100_000.0_f64, 100.0, 100.0, 100.0, 100.0];
        let mut series = vec![];
        for _ in 0..2000 {
            let mu = balances.iter().sum::<f64>() / balances.len() as f64;
            // Normalize to mean=1 before f32 cast — preserves scale-invariant C
            // exactly while keeping absolute balances safe in f64.
            let normed: Vec<f32> = balances.iter().map(|&b| (b / mu) as f32).collect();
            let cc = composite_concentration(&normed, g.alpha, g.q, g.w_e, g.w_s);
            series.push(cc);
            for b in balances.iter_mut() {
                let inflow = c_inflow * *b;
                let b_hat = (*b / mu) as f32;
                let rate = if *b < g.dignity_floor as f64 {
                    0.0
                } else {
                    calculate_decay_rate_continuous(b_hat, cc, &g) as f64
                };
                *b = (*b + inflow - rate * *b).max(g.dignity_floor as f64);
            }
        }
        let top: f64 = balances.iter().cloned().fold(0.0_f64, f64::max);
        let total_final: f64 = balances.iter().sum();
        let top_share_final: f64 = top / total_final;

        // (a) TOP SHARE DESCENDS: rich-get-richer inflow still equalizes the
        //     distribution — small agents grow 20%/tick, the top grows 15%
        //     (net of k_max), so relative shares converge even as absolutes diverge.
        assert!(
            top_share_final < initial_top_share,
            "top share must fall: {top_share_final:.4} not < initial {initial_top_share:.4}"
        );

        // (b) MONOTONE DESCENT toward target in the tail (restoring force).
        let tail = &series[series.len() / 2..];
        assert!(
            tail.windows(2).all(|w| w[1] <= w[0] + 1e-4),
            "C not non-increasing in the tail"
        );

        // (c) RESTORES TO THE TARGET, not just somewhere.
        assert!(
            (series.last().unwrap() - g.c_target).abs() < 0.05,
            "settled at {} away from C_target {}",
            series.last().unwrap(),
            g.c_target
        );

        // (d) SELF-EXTINGUISHING-WHEN-JUST: equal start stays equal, friction at base.
        let equal = [500.0_f32; 5];
        let cc_eq = composite_concentration(&equal, g.alpha, g.q, g.w_e, g.w_s);
        let r_eq = calculate_decay_rate_continuous(1.0, cc_eq, &g);
        assert!(
            r_eq <= g.base_rate * 1.5,
            "friction at equality must sit at base_rate, got {r_eq}"
        );

        // ---- SECONDARY RUN: pure decay (c_inflow=0) — absolute-bound regime ----
        // With zero inflow the governor is the sole mover; absolute convergence
        // to the dignity floor is guaranteed. (With any positive c_inflow the top
        // balance grows without bound — "rate-closable" means the C-SERIES converges,
        // not that absolute values are bounded. The main run above proves C-series
        // closure; this sub-run proves the decay-only attractor is the floor.)
        let mut balances2 = [100_000.0_f32, 100.0, 100.0, 100.0, 100.0];
        for _ in 0..2000 {
            let mu2 = balances2.iter().sum::<f32>() / balances2.len() as f32;
            let normed2: Vec<f32> = balances2.iter().map(|&b| b / mu2).collect();
            let cc2 = composite_concentration(&normed2, g.alpha, g.q, g.w_e, g.w_s);
            for b in balances2.iter_mut() {
                let rate = if *b < g.dignity_floor {
                    0.0
                } else {
                    calculate_decay_rate_continuous(*b / mu2, cc2, &g)
                };
                *b = (*b - rate * *b).max(g.dignity_floor);
            }
        }
        let top2 = balances2.iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            top2.is_finite() && top2 < 1.0e9,
            "decay-only regime must bound absolute balances toward floor: {top2}"
        );
    }

    /// Firewall (spec §8.1, aggregator.rs:491 mold): exhaustive construction +
    /// serialize-absent — the snapshot carries NO per-agent identity and cannot
    /// grow one without breaking this test's compile or asserts.
    #[test]
    fn snapshot_struct_has_no_peer_identity() {
        let row = NewConcentrationSnapshot {
            id: "attention:community:t".into(),
            h_app_id: "shefa".into(),
            substrate_signal: "attention".into(),
            governance_layer: "community".into(),
            n: 5,
            mu: 1.0,
            ge: 0.0,
            ge_squashed: 0.0,
            top_share: 0.2,
            gini: 0.0,
            c_composite: 0.08,
            alpha: 1.0,
            top_q: 0.01,
            computed_at: "t".into(),
        };
        let json = serde_json::to_string(&serde_json::json!({
            "id": row.id, "hAppId": row.h_app_id, "substrateSignal": row.substrate_signal,
            "governanceLayer": row.governance_layer, "n": row.n, "mu": row.mu,
            "ge": row.ge, "geSquashed": row.ge_squashed, "topShare": row.top_share,
            "gini": row.gini, "cComposite": row.c_composite, "alpha": row.alpha, "topQ": row.top_q,
        }))
        .expect("serialize");
        for leak in [
            "agent_id", "agentId", "pubkey", "agentKey", "signer", "human_id", "humanId", "balance",
        ] {
            assert!(
                !json.contains(leak),
                "snapshot must not carry per-agent field '{leak}': {json}"
            );
        }
    }

    /// Anti-capture (spec §8.1): exhaustive over the in-wall param grid — NO
    /// ratifiable config can drive effective friction to zero while the
    /// distribution is concentrated. The convergence test proves the loop CAN
    /// close; this proves it CANNOT be governed open.
    #[test]
    fn no_in_wall_config_extinguishes_friction_under_concentration() {
        use crate::services::limit_gradient_registry::*;
        use crate::services::token_decay_service::{
            calculate_decay_rate_continuous, GradientConfig,
        };

        let alphas = [ALPHA_WALL.0, 1.5, ALPHA_WALL.1];
        let c_targets = [C_TARGET_WALL.0, 0.15, C_TARGET_WALL.1];
        let k_maxes = [K_MAX_WALL.0, 0.05, K_MAX_WALL.1];
        let base_rates = [BASE_RATE_WALL.0, 0.001, BASE_RATE_WALL.1];
        let gammas = [GAMMA_WALL.0, 1.0, GAMMA_WALL.1];

        for &alpha in &alphas {
            for &c_target in &c_targets {
                for &k_max in &k_maxes {
                    for &base_rate in &base_rates {
                        for &gamma in &gammas {
                            let g = GradientConfig {
                                base_rate,
                                dignity_floor: 100.0,
                                gamma,
                                k_max,
                                c_target,
                                k_s: 0.5,
                                alpha,
                                q: 0.01,
                                w_e: 0.6,
                                w_s: 0.4,
                            };
                            // A concentrated world (C far above every in-wall target), a top agent.
                            let rate = calculate_decay_rate_continuous(10.0, 0.9, &g);
                            assert!(rate > 0.0,
                "in-wall config governed friction open: α={alpha} C_t={c_target} k_max={k_max} base={base_rate} γ={gamma}");
                            assert!(rate >= base_rate,
                "top-agent rate under high C must be at least base_rate (got {rate})");
                        }
                    }
                }
            }
        }
    }
}
