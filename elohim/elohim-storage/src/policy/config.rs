//! Peer policy configuration loaded from `peer-policy.toml`.
//!
//! Defines operator-declared preferences for pool participation, stewardship
//! intake, and conductor network exposure. Evaluated into runtime
//! `PeerCapabilityFlags` by `policy::evaluator` (Task 11).

use serde::{Deserialize, Serialize};

/// Either `"auto"` (derive from live state) or an explicit boolean override.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AutoOrBool {
    Bool(bool),
    #[serde(with = "auto_literal")]
    Auto,
}

mod auto_literal {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("auto")
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<(), D::Error> {
        let s = String::deserialize(d)?;
        if s == "auto" {
            Ok(())
        } else {
            Err(serde::de::Error::custom("expected \"auto\""))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub accept_general_traffic: AutoOrBool,
    pub min_free_storage_pct: u8,
    pub require_conductor_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardshipConfig {
    pub accept_new_reserves: AutoOrBool,
    pub max_storage_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Master switch. When true, elohim-storage spawns TCP forwarders that
    /// bridge pod-network ports to the embedded conductor's localhost ports.
    /// The conductor itself stays on 127.0.0.1 (Holochain's safe default);
    /// elohim-storage owns the pod-network exposure point and is the auth
    /// boundary for external traffic.
    pub expose_conductor_externally: bool,

    /// Bind address for the app-WS forwarder (zome calls).
    pub conductor_external_bind: String,
    /// Upstream app-WS port on 127.0.0.1 that Holochain is listening on.
    pub conductor_internal_port: u16,

    /// Bind address for the admin-WS forwarder (register, install hApp, etc).
    /// On headless k8s services this port is what doorway pods actually reach.
    pub conductor_admin_external_bind: String,
    /// Upstream admin-WS port on 127.0.0.1 that Holochain is listening on.
    pub conductor_admin_internal_port: u16,
}

/// Per-pillar write-through override entry as it appears in policy.toml.
///
/// Mirrors the manifest `WriteThrough` shape (EPR Phase 2B Task C.4) but
/// keyed under `[write_through.<pillar>]`. The `kinds` list is optional —
/// when absent the override applies to every kind the pillar emits.
///
/// Example:
/// ```toml
/// [write_through.shefa]
/// enabled = true
/// kinds = ["EconomicEvent"]
///
/// [write_through.lamad]
/// enabled = false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WriteThroughEntry {
    pub enabled: bool,
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub pool: PoolConfig,
    pub stewardship: StewardshipConfig,
    pub network: NetworkConfig,
    /// Layer-2 (policy.toml) write-through overrides keyed by pillar name.
    /// Optional — operators that don't care about ramping leave the section
    /// out entirely; layers 1/3/4 still apply. EPR Phase 2B Task C.6.
    #[serde(default)]
    pub write_through: std::collections::HashMap<String, WriteThroughEntry>,
}

impl PolicyConfig {
    /// Load a `PolicyConfig` from a TOML file on disk.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// The policy a node runs under when it has no usable policy file: the
    /// shipped example, which exposes nothing externally.
    pub fn builtin() -> Self {
        toml::from_str(include_str!("../../config/peer-policy.example.toml"))
            .expect("the shipped example policy parses (pinned by parses_example_config)")
    }

    /// Load the policy at `path`, or run under [`Self::builtin`] and say so.
    /// A missing file is a WARN (a hand-launched node); a file that is present
    /// but does not parse is an ERROR, because the operator's stated policy is
    /// NOT the one in force.
    pub fn load_or_builtin(path: &std::path::Path) -> Self {
        match Self::load(path) {
            Ok(cfg) => cfg,
            Err(e) => {
                let missing = e
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound);
                if missing {
                    tracing::warn!(
                        policy_path = %path.display(),
                        "no peer policy file — running under the built-in policy (nothing exposed externally)"
                    );
                } else {
                    tracing::error!(
                        policy_path = %path.display(),
                        error = %e,
                        "peer policy file did not load — the operator's policy is NOT in force; running under the built-in policy"
                    );
                }
                Self::builtin()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_malformed_policy_file_runs_under_the_builtin_policy() {
        let dir = tempfile::tempdir().unwrap();
        let missing = PolicyConfig::load_or_builtin(&dir.path().join("absent.toml"));
        assert!(!missing.network.expose_conductor_externally);
        let bad = dir.path().join("bad.toml");
        std::fs::write(&bad, "[pool]\nmin_free_storage_pct = \"not a number\"").unwrap();
        let malformed = PolicyConfig::load_or_builtin(&bad);
        assert_eq!(malformed.pool.min_free_storage_pct, 20);
    }

    #[test]
    fn parses_example_config() {
        let toml_str = include_str!("../../config/peer-policy.example.toml");
        let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(matches!(cfg.pool.accept_general_traffic, AutoOrBool::Auto));
        assert_eq!(cfg.pool.min_free_storage_pct, 20);
        assert!(cfg.pool.require_conductor_healthy);
        assert_eq!(cfg.stewardship.max_storage_pct, 80);
        assert!(!cfg.network.expose_conductor_externally);
        assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:8445");
        assert_eq!(cfg.network.conductor_internal_port, 4445);
        assert_eq!(cfg.network.conductor_admin_external_bind, "0.0.0.0:8444");
        assert_eq!(cfg.network.conductor_admin_internal_port, 4444);
    }

    #[test]
    fn network_config_parses_admin_fields() {
        let toml_str = r#"
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = true
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444
"#;
        let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.network.expose_conductor_externally);
        assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:8445");
        assert_eq!(cfg.network.conductor_internal_port, 4445);
        assert_eq!(cfg.network.conductor_admin_external_bind, "0.0.0.0:8444");
        assert_eq!(cfg.network.conductor_admin_internal_port, 4444);
    }

    #[test]
    fn write_through_section_parses_pillar_overrides() {
        // EPR Phase 2B Task C.6 — policy.toml layer-2 wiring.
        let toml_str = r#"
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444

[write_through.shefa]
enabled = true
kinds = ["EconomicEvent"]

[write_through.lamad]
enabled = false
"#;
        let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.write_through.contains_key("shefa"));
        assert!(cfg.write_through.contains_key("lamad"));
        assert!(cfg.write_through["shefa"].enabled);
        assert_eq!(
            cfg.write_through["shefa"].kinds.as_deref(),
            Some(&["EconomicEvent".to_string()][..])
        );
        assert!(!cfg.write_through["lamad"].enabled);
        assert!(cfg.write_through["lamad"].kinds.is_none());
    }

    #[test]
    fn write_through_section_is_optional() {
        // Operators that don't care about layer 2 leave the section out.
        let toml_str = r#"
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444
"#;
        let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.write_through.is_empty());
    }

    #[test]
    fn auto_or_bool_accepts_literal_true() {
        let cfg: PolicyConfig = toml::from_str(
            r#"
[pool]
accept_general_traffic = true
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = false
max_storage_pct = 80

[network]
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444
"#,
        )
        .unwrap();
        assert!(matches!(
            cfg.pool.accept_general_traffic,
            AutoOrBool::Bool(true)
        ));
        assert!(matches!(
            cfg.stewardship.accept_new_reserves,
            AutoOrBool::Bool(false)
        ));
    }
}
