//! Observation-kind registry — the manifest-declared vocabulary of observations
//! this node will accept.
//!
//! Each pillar declares its observation kinds in `manifest.json` under
//! `observation_kinds`, either inline (an array) or by `{"$ref": "./manifest/observation-kinds.json"}`
//! (lamad). Both shapes resolve here, relative to the pillar directory; a pillar
//! whose `manifest.json` names no kinds but ships `manifest/observation-kinds.json`
//! is read from that file. The declaration shape is
//! `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`.
//!
//! **The vocabulary is compiled into the binary** ([`ObservationKindRegistry::embedded`]).
//! A deployed node sets no manifest directory and runs from a working directory
//! that holds no `elohim/sdk`, so a registry read from disk there is empty and
//! every observation would be an unknown kind. The embedded table lists the
//! declaring pillars explicitly (no build script); a test pins it to the
//! manifests on disk, so a pillar that starts declaring kinds fails the tests
//! until it is added here. `ELOHIM_PILLAR_MANIFEST_DIR` stays as a development
//! override: when it names a readable directory, the kinds read there REPLACE
//! the embedded set (ruling R-A7, 2026-09-24).
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

/// Env var naming a pillar manifest root whose kinds replace the embedded set
/// (development override; shared with the write-through loader).
pub const PILLAR_MANIFEST_DIR_ENV: &str = "ELOHIM_PILLAR_MANIFEST_DIR";

/// One pillar's manifest bytes, compiled in: its `manifest.json` and any file
/// that manifest's `observation_kinds` `$ref` names (path as written there).
struct EmbeddedPillar {
    name: &'static str,
    manifest: &'static str,
    referenced: &'static [(&'static str, &'static str)],
}

macro_rules! pillar_file {
    ($rel:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sdk/domains/",
            $rel
        ))
    };
}

/// Every pillar that declares observation kinds, in name order — the order
/// [`ObservationKindRegistry::load`] reads a directory in, so "first declared
/// wins" resolves identically from the binary and from disk.
const EMBEDDED_PILLARS: &[EmbeddedPillar] = &[
    EmbeddedPillar {
        name: "elohim",
        manifest: pillar_file!("elohim/manifest.json"),
        referenced: &[],
    },
    EmbeddedPillar {
        name: "imagodei",
        manifest: pillar_file!("imagodei/manifest.json"),
        referenced: &[],
    },
    EmbeddedPillar {
        name: "infrastructure",
        manifest: pillar_file!("infrastructure/manifest.json"),
        referenced: &[],
    },
    EmbeddedPillar {
        name: "lamad",
        manifest: pillar_file!("lamad/manifest.json"),
        referenced: &[(
            "./manifest/observation-kinds.json",
            pillar_file!("lamad/manifest/observation-kinds.json"),
        )],
    },
    EmbeddedPillar {
        name: "mishpat",
        manifest: pillar_file!("mishpat/manifest.json"),
        referenced: &[],
    },
    EmbeddedPillar {
        name: "qahal",
        manifest: pillar_file!("qahal/manifest.json"),
        referenced: &[],
    },
    EmbeddedPillar {
        name: "shefa",
        manifest: pillar_file!("shefa/manifest.json"),
        referenced: &[],
    },
];

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

    /// The development override directory, when `ELOHIM_PILLAR_MANIFEST_DIR` is set.
    pub fn override_dir() -> Option<PathBuf> {
        std::env::var_os(PILLAR_MANIFEST_DIR_ENV).map(PathBuf::from)
    }

    /// The node's registry: the kinds read under `ELOHIM_PILLAR_MANIFEST_DIR`
    /// when it names a readable directory (development override, replaces the
    /// embedded set), else the compiled-in kinds ([`Self::embedded`]).
    pub fn load_default() -> Self {
        if let Some(dir) = Self::override_dir() {
            if dir.is_dir() {
                return Self::load(&dir);
            }
            tracing::warn!(
                target = "observation::kinds",
                dir = ?dir,
                "ELOHIM_PILLAR_MANIFEST_DIR is not a readable directory; using the compiled-in observation kinds"
            );
        }
        Self::embedded()
    }

    /// The observation kinds compiled into this binary from the pillar manifests.
    pub fn embedded() -> Self {
        let mut registry = Self::empty();
        for pillar in EMBEDDED_PILLARS {
            let Some(manifest) = parse_json(pillar.name, pillar.manifest) else {
                continue;
            };
            let fetch = |rel: &str| {
                pillar
                    .referenced
                    .iter()
                    .find(|(path, _)| normalize_rel(path) == normalize_rel(rel))
                    .and_then(|(path, body)| parse_json(path, body))
            };
            for decl in resolve_pillar_kinds(pillar.name, Some(manifest), &fetch) {
                registry.kinds.entry(decl.kind.clone()).or_insert(decl);
            }
        }
        registry
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

    /// Every declared kind, in no particular order.
    pub fn kinds(&self) -> impl Iterator<Item = &ObservationKindDeclaration> {
        self.kinds.values()
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

/// Read one pillar directory's kinds from disk.
fn read_pillar_kinds(pillar: &Path) -> Vec<ObservationKindDeclaration> {
    let manifest_path = pillar.join("manifest.json");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(body) => match parse_json(&manifest_path.display().to_string(), &body) {
            Some(json) => Some(json),
            // Malformed manifest: warned in `parse_json`; the pillar is skipped.
            None => return Vec::new(),
        },
        Err(_) => None,
    };
    let fetch = |rel: &str| {
        let path = pillar.join(rel);
        let body = std::fs::read_to_string(&path).ok()?;
        parse_json(&path.display().to_string(), &body)
    };
    resolve_pillar_kinds(&pillar.display().to_string(), manifest, &fetch)
}

/// Resolve one pillar's kinds: `manifest.json`'s `observation_kinds` (inline
/// array or `$ref`), else `manifest/observation-kinds.json`. `fetch` reads a
/// path relative to the pillar — from disk or from the compiled-in table.
fn resolve_pillar_kinds(
    pillar: &str,
    manifest: Option<serde_json::Value>,
    fetch: &dyn Fn(&str) -> Option<serde_json::Value>,
) -> Vec<ObservationKindDeclaration> {
    let declared = manifest.and_then(|json| json.get("observation_kinds").cloned());
    let kinds_value = match declared {
        Some(serde_json::Value::Object(obj)) => match obj.get("$ref").and_then(|r| r.as_str()) {
            Some(rel) => fetch(rel),
            None => None,
        },
        Some(array @ serde_json::Value::Array(_)) => Some(array),
        _ => fetch("manifest/observation-kinds.json"),
    };

    let Some(value) = kinds_value else {
        return Vec::new();
    };
    match serde_json::from_value::<Vec<ObservationKindDeclaration>>(value) {
        Ok(kinds) => kinds,
        Err(e) => {
            tracing::warn!(
                target = "observation::kinds",
                pillar = pillar,
                error = %e,
                "observation_kinds does not match the declaration shape; skipping pillar"
            );
            Vec::new()
        }
    }
}

/// `./manifest/x.json` and `manifest/x.json` name the same file.
fn normalize_rel(rel: &str) -> &str {
    rel.trim_start_matches("./")
}

fn parse_json(label: &str, body: &str) -> Option<serde_json::Value> {
    match serde_json::from_str(body) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(
                target = "observation::kinds",
                path = label,
                error = %e,
                "pillar manifest file is not valid JSON; skipping"
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

    /// Env var that turns [`embedded_registry_child_probe`] from a no-op into
    /// the real probe; only the parent test below sets it.
    const CHILD_PROBE_ENV: &str = "OBSERVATION_KINDS_EMBEDDED_CHILD_PROBE";

    /// R-A7: a deployed node sets no `ELOHIM_PILLAR_MANIFEST_DIR` and runs from
    /// a working directory that holds no `elohim/sdk/domains`. The declared
    /// kinds must still be there, because they are compiled in.
    ///
    /// The working directory is process-global and the test harness is
    /// multi-threaded, so the probe runs in a CHILD process (this same test
    /// binary, filtered to [`embedded_registry_child_probe`]) started in an
    /// empty temp directory with the env var removed — never by mutating this
    /// process's cwd under other tests.
    #[test]
    fn embedded_kinds_include_lamad_content_viewed_without_env_or_cwd() {
        let cwd = tempfile::tempdir().expect("temp cwd");
        let out = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "services::observation_kinds::tests::embedded_registry_child_probe",
                "--nocapture",
                "--test-threads=1",
            ])
            .current_dir(cwd.path())
            .env_remove(PILLAR_MANIFEST_DIR_ENV)
            .env(CHILD_PROBE_ENV, "1")
            .output()
            .expect("spawn the child probe");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "child probe failed:\n{stdout}\n{stderr}"
        );
        assert!(
            stdout.contains("EMBEDDED-PROBE lamad:content-viewed agent-private"),
            "the child probe did not run or did not find the kind:\n{stdout}\n{stderr}"
        );
    }

    /// The child half of the test above; a no-op unless the parent started it.
    #[test]
    fn embedded_registry_child_probe() {
        if std::env::var(CHILD_PROBE_ENV).is_err() {
            return;
        }
        assert!(std::env::var(PILLAR_MANIFEST_DIR_ENV).is_err());
        assert!(
            !Path::new("elohim/sdk/domains").exists(),
            "the child must run where no repo-relative manifest dir resolves"
        );
        for registry in [
            ObservationKindRegistry::embedded(),
            ObservationKindRegistry::load_default(),
        ] {
            let decl = registry
                .get("lamad:content-viewed")
                .expect("lamad:content-viewed is compiled into the binary");
            println!("EMBEDDED-PROBE {} {}", decl.kind, decl.reach);
        }
    }

    /// The embedded table lists the pillars explicitly (no build script), so a
    /// pillar that starts declaring kinds must be added to it. This pins the
    /// compiled-in vocabulary to the manifests on disk.
    #[test]
    fn embedded_set_matches_the_pillar_manifests_on_disk() {
        let disk = repo_registry();
        let embedded = ObservationKindRegistry::embedded();
        let mut disk_kinds: Vec<_> = disk.kinds().map(|d| d.kind.clone()).collect();
        let mut embedded_kinds: Vec<_> = embedded.kinds().map(|d| d.kind.clone()).collect();
        disk_kinds.sort();
        embedded_kinds.sort();
        assert_eq!(embedded_kinds, disk_kinds);
        for decl in disk.kinds() {
            assert_eq!(
                embedded.get(&decl.kind),
                Some(decl),
                "{} drifted",
                decl.kind
            );
        }
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
