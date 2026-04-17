//! Policy evaluator — derives runtime capability flags from config + live state.
//!
//! Task 12 will call [`evaluate`] inside the heartbeat task to project flags
//! into Mishpat.

use crate::policy::config::{AutoOrBool, PolicyConfig};

#[derive(Debug, Clone, Copy)]
pub struct LiveState {
    pub free_storage_pct: u8,
    pub conductor_healthy: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedFlags {
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
}

pub fn evaluate(cfg: &PolicyConfig, state: &LiveState) -> EvaluatedFlags {
    EvaluatedFlags {
        general_pool_member: eval_pool(cfg, state),
        accepting_stewardship_reserves: eval_stewardship(cfg, state),
    }
}

fn eval_pool(cfg: &PolicyConfig, state: &LiveState) -> bool {
    match cfg.pool.accept_general_traffic {
        AutoOrBool::Bool(b) => b,
        AutoOrBool::Auto => {
            state.free_storage_pct >= cfg.pool.min_free_storage_pct
                && (!cfg.pool.require_conductor_healthy || state.conductor_healthy)
        }
    }
}

fn eval_stewardship(cfg: &PolicyConfig, state: &LiveState) -> bool {
    match cfg.stewardship.accept_new_reserves {
        AutoOrBool::Bool(b) => b,
        AutoOrBool::Auto => {
            // used_pct = 100 - free_pct; accept if used_pct <= max_storage_pct
            (100u8.saturating_sub(state.free_storage_pct)) <= cfg.stewardship.max_storage_pct
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::config::{NetworkConfig, PoolConfig, StewardshipConfig};

    fn base_cfg() -> PolicyConfig {
        PolicyConfig {
            pool: PoolConfig {
                accept_general_traffic: AutoOrBool::Auto,
                min_free_storage_pct: 20,
                require_conductor_healthy: true,
            },
            stewardship: StewardshipConfig {
                accept_new_reserves: AutoOrBool::Auto,
                max_storage_pct: 80,
            },
            network: NetworkConfig {
                expose_conductor_externally: false,
                conductor_external_bind: "0.0.0.0:4445".into(),
                conductor_internal_port: 4445,
            },
        }
    }

    #[test]
    fn auto_pool_member_respects_conductor_and_storage() {
        let cfg = base_cfg();
        assert!(evaluate(&cfg, &LiveState { free_storage_pct: 50, conductor_healthy: true }).general_pool_member);
        assert!(!evaluate(&cfg, &LiveState { free_storage_pct: 50, conductor_healthy: false }).general_pool_member);
        assert!(!evaluate(&cfg, &LiveState { free_storage_pct: 10, conductor_healthy: true }).general_pool_member);
    }

    #[test]
    fn explicit_bool_overrides_auto() {
        let mut cfg = base_cfg();
        cfg.pool.accept_general_traffic = AutoOrBool::Bool(false);
        assert!(!evaluate(&cfg, &LiveState { free_storage_pct: 50, conductor_healthy: true }).general_pool_member);

        cfg.pool.accept_general_traffic = AutoOrBool::Bool(true);
        // even when auto would say false (conductor unhealthy), explicit true wins
        assert!(evaluate(&cfg, &LiveState { free_storage_pct: 50, conductor_healthy: false }).general_pool_member);
    }

    #[test]
    fn stewardship_respects_max_storage() {
        let cfg = base_cfg();
        // used=15, max=80 → accept
        assert!(evaluate(&cfg, &LiveState { free_storage_pct: 85, conductor_healthy: true }).accepting_stewardship_reserves);
        // used=85, max=80 → refuse
        assert!(!evaluate(&cfg, &LiveState { free_storage_pct: 15, conductor_healthy: true }).accepting_stewardship_reserves);
        // used=80 exactly → accept (inclusive <=)
        assert!(evaluate(&cfg, &LiveState { free_storage_pct: 20, conductor_healthy: true }).accepting_stewardship_reserves);
    }

    #[test]
    fn conductor_healthy_ignored_when_not_required() {
        let mut cfg = base_cfg();
        cfg.pool.require_conductor_healthy = false;
        // conductor unhealthy but storage fine → still pool member (Auto, no conductor gate)
        assert!(evaluate(&cfg, &LiveState { free_storage_pct: 50, conductor_healthy: false }).general_pool_member);
    }
}
