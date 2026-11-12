//! Application of existing policy, Bound and algedonic primitives to private feedback.
//! This is a resolved local contract, not a second measure or authority ontology.
use crate::{atom_cid, feedback::invalid, Bound, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameters {
    pub total_bytes: u64,
    pub active_bytes: u64,
    pub replacement_bytes: u64,
    pub control_bytes: u64,
    pub record_bytes: u64,
    pub trigger_percent: u32,
    pub reset_percent: u32,
    pub routine_days: u32,
    pub resolved_days: u32,
    pub recent_receipts: usize,
    pub dwell_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capacity {
    pub policy: String,
    pub policy_cid: String,
    /// Exact resolved precedent; retained records never consult a newer registry row.
    pub precedent: Value,
    pub parameters: Parameters,
}

impl Capacity {
    pub fn resolve(policy: String, precedent: Value, overrides: Value) -> Result<Self> {
        let mut parameters = precedent["measure"].clone();
        if !parameters.is_object() {
            return invalid("capacity policy parameters must be an object");
        }
        let Some(overrides) = overrides.as_object() else {
            return invalid("capacity parameters must be an object");
        };
        for (key, value) in overrides {
            if !precedent["permitted-local-parameters"]
                .as_array()
                .is_some_and(|allowed| allowed.iter().any(|v| v.as_str() == Some(key)))
            {
                return invalid("capacity policy does not authorize this local parameter");
            }
            parameters[key] = value.clone();
        }
        let parameters: Parameters = serde_json::from_value(parameters)
            .map_err(|e| crate::FabricError::InvalidFeedback(e.to_string()))?;
        let result = Self {
            policy,
            policy_cid: atom_cid(&(&precedent, &parameters))?.to_string(),
            parameters,
            precedent,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<()> {
        let p = &self.parameters;
        let row = &self.precedent;
        if self.policy != format!("{}@{}", row["id"].as_str().unwrap_or(""), row["version"])
            || self.policy_cid != atom_cid(&(row, p))?.to_string()
            || row["established_by"].as_str().is_none_or(|s| !s.starts_with("operator-"))
            || row["why"].as_str().is_none_or(|s| s.trim().is_empty())
            || !row["permits"].as_array().is_some_and(|v|v.iter().any(|p| p == "reclaim-eligible-feedback"))
            || row["binding"] != "binding-local"
            || row["scope"]["path"] != ".eprfs/status/feedback"
            || p.active_bytes == 0 || p.record_bytes == 0 || p.record_bytes > p.active_bytes
            || p.replacement_bytes < p.active_bytes
            || p.active_bytes.checked_add(p.replacement_bytes)
                .and_then(|n| n.checked_add(p.control_bytes)).is_none_or(|n| n > p.total_bytes)
            // Integers must remain exact in the existing f64 Bound evaluator.
            || p.total_bytes > (1_u64 << 53)
            || p.trigger_percent == 0 || p.trigger_percent > 100
            || p.reset_percent >= p.trigger_percent
            || p.recent_receipts == 0
        {
            return invalid("invalid or unauthorized feedback capacity contract");
        }
        let effective =
            serde_json::to_value(p).map_err(|e| crate::FabricError::Encode(e.to_string()))?;
        let bounds = row["parameter-bounds"].as_object().ok_or_else(|| {
            crate::FabricError::InvalidFeedback("declared capacity parameter bounds missing".into())
        })?;
        for (key, rule) in bounds {
            let value = effective[key].as_u64().ok_or_else(|| {
                crate::FabricError::InvalidFeedback("unknown bounded capacity parameter".into())
            })?;
            let object = rule.as_object().ok_or_else(|| {
                crate::FabricError::InvalidFeedback("invalid parameter bound".into())
            })?;
            if object.keys().any(|k| k != "min" && k != "max")
                || object.values().any(|v| v.as_u64().is_none())
                || rule["min"].as_u64().is_some_and(|min| value < min)
                || rule["max"].as_u64().is_some_and(|max| value > max)
            {
                return invalid("capacity variance violates declared parameter bounds");
            }
        }
        // A snapshot is independently verifiable: only declared variance may differ.
        for (key, value) in effective.as_object().into_iter().flatten() {
            if row["measure"][key] != *value
                && !row["permitted-local-parameters"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(key)))
            {
                return invalid("snapshot contains unauthorized capacity variance");
            }
        }
        Ok(())
    }

    pub fn bound(&self) -> Result<Bound> {
        Bound::new(
            self.parameters.active_bytes as f64,
            "byte".into(),
            f64::from(self.parameters.trigger_percent),
        )
    }

    pub fn target(&self) -> u64 {
        (u128::from(self.parameters.active_bytes) * u128::from(self.parameters.reset_percent) / 100)
            as u64
    }
}
