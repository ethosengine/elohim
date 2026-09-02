//! Live runtime-passport records for a berth and its processes.

use serde::{Deserialize, Serialize};

use crate::RestartVerdict;

/// Record kind carried by every S0 runtime passport.
pub const PASSPORT_KIND: &str = "runtime-passport";

/// Effective enforcement tier observed for a process.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveTier {
    /// Limits enforced directly by the host.
    Enforced,
    /// Limits bounded by an explicit grant.
    Bounded,
    /// Enforcement delegated to another runtime.
    Delegated,
    /// Limits intrinsic to the execution environment.
    Intrinsic,
    /// No effective limit tier is available in S0.
    #[default]
    None,
}

/// Current runtime facts about one declared process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ProcessPassport {
    /// Process name from the runtime manifest.
    pub name: String,
    /// Hash of the artifact actually started.
    pub artifact_sha256: String,
    /// Resolved local artifact path.
    pub artifact_path: String,
    /// Current operating-system process identifier.
    pub pid: Option<u32>,
    /// Wall-clock start time for the current child.
    pub started_at_epoch_ms: Option<u64>,
    /// Whether the current child completed its readiness ladder.
    pub ready: bool,
    /// Effective resource-enforcement tier.
    pub effective_tier: EffectiveTier,
    /// Deaths currently retained in the policy window.
    pub deaths_in_window: u32,
}

/// Current live projection of one berth and all of its processes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Passport {
    /// Runtime-passport schema version.
    pub schema: u32,
    /// Record kind; S0 passports carry `runtime-passport`.
    pub kind: String,
    /// CID string of the runtime manifest occupying the berth.
    pub manifest: String,
    /// Agent CID string, once identity is available.
    pub node: Option<String>,
    /// Monotonic berth incarnation.
    pub incarnation: u64,
    /// Version of the ark producing this passport.
    pub ark_version: String,
    /// Live process projections.
    pub processes: Vec<ProcessPassport>,
    /// Most recent restart-policy verdict.
    pub last_verdict: Option<RestartVerdict>,
    /// Wall-clock time of the latest update.
    pub updated_at_epoch_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passport_json_kind_is_runtime_passport() {
        let passport = Passport {
            schema: 1,
            kind: PASSPORT_KIND.to_string(),
            manifest: "bafy-manifest".to_string(),
            node: None,
            incarnation: 1,
            ark_version: "0.1.0".to_string(),
            processes: Vec::new(),
            last_verdict: None,
            updated_at_epoch_ms: 1_000,
        };

        let json = serde_json::to_value(passport).unwrap();
        assert_eq!(json["kind"], PASSPORT_KIND);
    }
}
