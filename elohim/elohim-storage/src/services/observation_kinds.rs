//! Observation-kind registry — the manifest-declared vocabulary of observations
//! this node will accept, loaded from the pillar manifests on disk.
//!
//! Each pillar declares its observation kinds in `manifest.json` under
//! `observation_kinds`, either inline (an array) or by `{"$ref": "./manifest/observation-kinds.json"}`
//! (lamad). Both shapes resolve here, relative to the pillar directory; a pillar
//! whose `manifest.json` names no kinds but ships `manifest/observation-kinds.json`
//! is read from that file. The declaration shape is
//! `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`.
//!
//! The directory follows the same resolution as the write-through layer-1 loader
//! (`manifest_registry::load_pillar_manifest_layer1`): `ELOHIM_PILLAR_MANIFEST_DIR`,
//! falling back to `elohim/sdk/domains` relative to the working directory.
//!
//! `validate_payload` checks an observation's payload against the kind's declared
//! field map. Type map: `Cid` | `String` → JSON string; `u8`/`u16`/`u32`/`u64` →
//! non-negative integer no larger than the type's max; `f32` → finite number;
//! `bool` → boolean. A trailing `?` on the declared type makes the field optional.
//! Fields the map does not declare are refused. Validation lives in storage, never
//! in the browser (cross-lane ruling X5).
//!
//! See genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md §7.

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Env var naming the pillar manifest root (shared with the write-through loader).
pub const PILLAR_MANIFEST_DIR_ENV: &str = "ELOHIM_PILLAR_MANIFEST_DIR";

/// Default pillar manifest root, relative to the working directory.
pub const DEFAULT_PILLAR_MANIFEST_DIR: &str = "elohim/sdk/domains";

/// One manifest-declared observation kind (mirror of `observation-kind.schema.json`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ObservationKindDeclaration {
    pub kind: String,
    pub namespace: String,
    /// Field name → declared type (`Cid`, `u64`, `Cid?`, …).
    pub schema: BTreeMap<String, String>,
    pub retention_class: String,
    pub reach: String,
    #[serde(default)]
    pub diversity_threshold: Option<serde_json::Value>,
    #[serde(default)]
    pub graduates_to: Option<String>,
    #[serde(default)]
    pub graduation_window_seconds: Option<i64>,
    #[serde(default)]
    pub graduation_policy: Option<String>,
}

impl ObservationKindDeclaration {
    /// True when the kind's reach keeps it on the observer's own node.
    pub fn is_agent_private(&self) -> bool {
        self.reach == "agent-private"
    }
}

/// Registry of every observation kind declared across the pillar manifests.
#[derive(Debug, Clone, Default)]
pub struct ObservationKindRegistry {
    kinds: HashMap<String, ObservationKindDeclaration>,
}

impl ObservationKindRegistry {
    /// An empty registry: every kind is unknown.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The pillar manifest root: `ELOHIM_PILLAR_MANIFEST_DIR`, else `elohim/sdk/domains`.
    pub fn default_dir() -> PathBuf {
        std::env::var(PILLAR_MANIFEST_DIR_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_PILLAR_MANIFEST_DIR))
    }

    /// Load from [`Self::default_dir`]. A missing directory yields an empty
    /// registry (warned), the same degradation the write-through loader uses.
    pub fn load_default() -> Self {
        Self::load(&Self::default_dir())
    }

    /// Load every pillar's declared observation kinds under `dir`.
    ///
    /// Never fails: an unreadable or malformed pillar is warned and skipped so
    /// one bad manifest cannot empty the whole vocabulary. A kind declared by
    /// two pillars keeps the first one read (pillars are read in name order).
    pub fn load(dir: &Path) -> Self {
        let mut registry = Self::empty();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!(
                    target = "observation::kinds",
                    dir = ?dir,
                    error = %e,
                    "pillar manifest directory unreadable; observation-kind registry stays empty"
                );
                return registry;
            }
        };
        let mut pillars: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        pillars.sort();
        for pillar in pillars {
            for decl in read_pillar_kinds(&pillar) {
                registry.kinds.entry(decl.kind.clone()).or_insert(decl);
            }
        }
        registry
    }

    /// The declaration for `kind`, if any pillar declares it.
    pub fn get(&self, kind: &str) -> Option<&ObservationKindDeclaration> {
        self.kinds.get(kind)
    }

    /// Number of declared kinds.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    /// True when no kind is declared.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Validate `payload` against the declared field map of `kind`.
    ///
    /// `Err` carries a one-line reason naming the kind and the field.
    pub fn validate_payload(&self, kind: &str, payload: &serde_json::Value) -> Result<(), String> {
        let decl = self.get(kind).ok_or_else(|| {
            format!("observation kind '{kind}' is not declared by any pillar manifest")
        })?;
        let obj = payload
            .as_object()
            .ok_or_else(|| format!("{kind}: payload must be a JSON object"))?;

        for field in obj.keys() {
            if !decl.schema.contains_key(field) {
                return Err(format!(
                    "{kind}: field '{field}' is not declared by the kind"
                ));
            }
        }

        for (field, declared) in &decl.schema {
            let (base, optional) = match declared.strip_suffix('?') {
                Some(base) => (base, true),
                None => (declared.as_str(), false),
            };
            match obj.get(field) {
                None if optional => {}
                None => return Err(format!("{kind}: required field '{field}' is missing")),
                Some(value) => check_type(kind, field, base, value)?,
            }
        }
        Ok(())
    }
}

fn check_type(
    kind: &str,
    field: &str,
    base: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let unsigned_max = match base {
        "u8" => Some(u8::MAX as u64),
        "u16" => Some(u16::MAX as u64),
        "u32" => Some(u32::MAX as u64),
        "u64" => Some(u64::MAX),
        _ => None,
    };
    let ok = match (base, unsigned_max) {
        (_, Some(max)) => value.as_u64().is_some_and(|n| n <= max),
        ("Cid" | "String", None) => value.is_string(),
        ("f32", None) => value
            .as_f64()
            .is_some_and(|n| n.is_finite() && n.abs() <= f32::MAX as f64),
        ("bool", None) => value.is_boolean(),
        _ => {
            return Err(format!(
                "{kind}: field '{field}' declares unsupported type '{base}'"
            ))
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{kind}: field '{field}' must be {base}, got {value}"
        ))
    }
}

/// Read one pillar's kinds: `manifest.json`'s `observation_kinds` (inline array
/// or `$ref`), else `manifest/observation-kinds.json`.
fn read_pillar_kinds(pillar: &Path) -> Vec<ObservationKindDeclaration> {
    let manifest_path = pillar.join("manifest.json");
    let declared = match std::fs::read_to_string(&manifest_path) {
        Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => json.get("observation_kinds").cloned(),
            Err(e) => {
                tracing::warn!(
                    target = "observation::kinds",
                    path = ?manifest_path,
                    error = %e,
                    "pillar manifest is not valid JSON; skipping its observation kinds"
                );
                return Vec::new();
            }
        },
        Err(_) => None,
    };

    let kinds_value = match declared {
        Some(serde_json::Value::Object(obj)) => match obj.get("$ref").and_then(|r| r.as_str()) {
            Some(rel) => read_json(&pillar.join(rel)),
            None => None,
        },
        Some(array @ serde_json::Value::Array(_)) => Some(array),
        _ => read_json(&pillar.join("manifest").join("observation-kinds.json")),
    };

    let Some(value) = kinds_value else {
        return Vec::new();
    };
    match serde_json::from_value::<Vec<ObservationKindDeclaration>>(value) {
        Ok(kinds) => kinds,
        Err(e) => {
            tracing::warn!(
                target = "observation::kinds",
                pillar = ?pillar,
                error = %e,
                "observation_kinds does not match the declaration shape; skipping pillar"
            );
            Vec::new()
        }
    }
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    let body = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&body) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(
                target = "observation::kinds",
                path = ?path,
                error = %e,
                "observation-kinds file is not valid JSON; skipping"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn repo_registry() -> ObservationKindRegistry {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/domains");
        ObservationKindRegistry::load(&dir)
    }

    #[test]
    fn loads_lamad_kinds_from_manifest_dir() {
        let registry = repo_registry();
        let decl = registry
            .get("lamad:content-viewed")
            .expect("lamad:content-viewed is declared in the lamad manifest");
        assert_eq!(decl.reach, "agent-private");
        assert!(decl.is_agent_private());
        assert_eq!(
            decl.schema.get("session_id").map(String::as_str),
            Some("Cid?")
        );
        // Inline-declared kinds from other pillars resolve too.
        assert!(registry.get("infrastructure:doorway-heartbeat").is_some());
    }

    #[test]
    fn validate_payload_accepts_declared_shape() {
        let registry = repo_registry();
        let payload = json!({ "ref_cid": "bafy-node", "dwell_ms": 3200, "scroll_depth_pct": 64 });
        assert_eq!(
            registry.validate_payload("lamad:content-viewed", &payload),
            Ok(())
        );
    }

    #[test]
    fn validate_payload_rejects_unknown_field() {
        let registry = repo_registry();
        let payload = json!({
            "ref_cid": "bafy-node", "dwell_ms": 3200, "scroll_depth_pct": 64, "mood": "curious"
        });
        let err = registry
            .validate_payload("lamad:content-viewed", &payload)
            .unwrap_err();
        assert!(err.contains("mood"), "{err}");
    }

    #[test]
    fn validate_payload_rejects_u8_overflow() {
        let registry = repo_registry();
        let payload = json!({ "ref_cid": "bafy-node", "dwell_ms": 3200, "scroll_depth_pct": 256 });
        let err = registry
            .validate_payload("lamad:content-viewed", &payload)
            .unwrap_err();
        assert!(err.contains("scroll_depth_pct"), "{err}");
    }

    #[test]
    fn validate_payload_rejects_negative_u64() {
        let registry = repo_registry();
        let payload = json!({ "ref_cid": "bafy-node", "dwell_ms": -1, "scroll_depth_pct": 10 });
        let err = registry
            .validate_payload("lamad:content-viewed", &payload)
            .unwrap_err();
        assert!(err.contains("dwell_ms"), "{err}");
    }

    #[test]
    fn validate_payload_rejects_missing_required() {
        let registry = repo_registry();
        let payload = json!({ "ref_cid": "bafy-node", "scroll_depth_pct": 10 });
        let err = registry
            .validate_payload("lamad:content-viewed", &payload)
            .unwrap_err();
        assert!(err.contains("dwell_ms"), "{err}");
    }

    #[test]
    fn unknown_kind_is_none() {
        let registry = repo_registry();
        assert!(registry.get("lamad:never-declared").is_none());
        assert!(registry
            .validate_payload("lamad:never-declared", &json!({}))
            .is_err());
    }
}
