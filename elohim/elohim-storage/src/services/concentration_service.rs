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
use crate::services::measure::{composite_concentration, ge_alpha, gini, squash, top_quantile_share};
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
        Ok(latest_snapshot(conn, h_app_id, substrate_signal, governance_layer)?
            .map(|s| s.c_composite))
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
            &mut conn, "shefa", "attention", "household",
            &[100.0, 200.0, 300.0, 400.0], &GradientConfig::default(),
            "2026-06-10T03:00:00Z",
        ).expect("ok");
        assert_eq!(out, SnapshotOutcome::SuppressedBelowK { n: 4 });
        assert!(ConcentrationService::effective_c(&mut conn, "shefa", "attention", "household")
            .expect("q").is_none(), "no row may exist for a suppressed cohort");
    }

    #[test]
    fn writes_at_k_and_effective_c_reads_it() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let balances = [100.0_f32, 100.0, 100.0, 100.0, 10_000.0];
        let out = ConcentrationService::compute_snapshot(
            &mut conn, "shefa", "attention", "community", &balances,
            &GradientConfig::default(), "2026-06-10T03:00:00Z",
        ).expect("ok");
        let snap = match out { SnapshotOutcome::Written(s) => s, _ => panic!("expected write") };
        assert_eq!(snap.n, 5);
        assert!(snap.c_composite > 0.3, "skewed distribution must read concentrated");
        let c = ConcentrationService::effective_c(&mut conn, "shefa", "attention", "community")
            .expect("q").expect("some");
        assert!((c - snap.c_composite).abs() < 1e-6);
    }

    /// Firewall (spec §8.1, aggregator.rs:491 mold): exhaustive construction +
    /// serialize-absent — the snapshot carries NO per-agent identity and cannot
    /// grow one without breaking this test's compile or asserts.
    #[test]
    fn snapshot_struct_has_no_peer_identity() {
        let row = NewConcentrationSnapshot {
            id: "attention:community:t".into(), h_app_id: "shefa".into(),
            substrate_signal: "attention".into(), governance_layer: "community".into(),
            n: 5, mu: 1.0, ge: 0.0, ge_squashed: 0.0, top_share: 0.2,
            gini: 0.0, c_composite: 0.08, alpha: 1.0, top_q: 0.01,
            computed_at: "t".into(),
        };
        let json = serde_json::to_string(&serde_json::json!({
            "id": row.id, "hAppId": row.h_app_id, "substrateSignal": row.substrate_signal,
            "governanceLayer": row.governance_layer, "n": row.n, "mu": row.mu,
            "ge": row.ge, "geSquashed": row.ge_squashed, "topShare": row.top_share,
            "gini": row.gini, "cComposite": row.c_composite, "alpha": row.alpha, "topQ": row.top_q,
        })).expect("serialize");
        for leak in ["agent_id", "agentId", "pubkey", "agentKey", "signer", "human_id", "humanId", "balance"] {
            assert!(!json.contains(leak), "snapshot must not carry per-agent field '{leak}': {json}");
        }
    }
}
