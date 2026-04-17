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
    pub expose_conductor_externally: bool,
    pub conductor_external_bind: String,
    pub conductor_internal_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub pool: PoolConfig,
    pub stewardship: StewardshipConfig,
    pub network: NetworkConfig,
}

impl PolicyConfig {
    /// Load a `PolicyConfig` from a TOML file on disk.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_config() {
        let toml_str = include_str!("../../config/peer-policy.example.toml");
        let cfg: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(matches!(cfg.pool.accept_general_traffic, AutoOrBool::Auto));
        assert_eq!(cfg.pool.min_free_storage_pct, 20);
        assert!(cfg.pool.require_conductor_healthy);
        assert_eq!(cfg.stewardship.max_storage_pct, 80);
        assert!(!cfg.network.expose_conductor_externally);
        assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:4445");
        assert_eq!(cfg.network.conductor_internal_port, 4445);
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
conductor_external_bind = "0.0.0.0:4445"
conductor_internal_port = 4445
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
