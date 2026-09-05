//! Pure outward resource projection consumed by the orchestrator through this crate's CLI.
//! Manifest identity always comes from ark-core; this bridge neither anchors nor enforces it.

use ark_core::manifest::RuntimeManifest;
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const BUDGETS: &str = include_str!("../../../genesis/data/devices/archetype-resource-budgets.json");
const MIB: u64 = 1024 * 1024;

/// Kubernetes quantities; an absent envelope is serialized as an empty object.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedEnvelope {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub edgenode_memory_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub edgenode_memory_limit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub edgenode_cpu_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub edgenode_cpu_limit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conductor_memory_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_memory_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conductor_memory_limit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_memory_limit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conductor_cpu_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_cpu_request: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conductor_cpu_limit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_cpu_limit: String,
}

/// Evidence only: no verdict rewrites a deployment or changes its declared head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DriftVerdict {
    Fresh,
    PinMismatch {
        human: String,
        expected: String,
        actual: String,
    },
    RenderDrift {
        human: String,
        field: String,
        pinned: String,
        rendered: String,
    },
    Unpinned {
        human: String,
    },
}

/// Parse the supported fleet quantity vocabulary exactly, without float rounding.
/// Memory returns bytes, CPU returns millicores. Fractional base units are refused.
pub fn quantity(value: &str, memory: bool) -> Result<u64> {
    let (number, multiplier) = if memory {
        [
            ("Ti", 1024_u64.pow(4)),
            ("Gi", 1024_u64.pow(3)),
            ("Mi", 1024_u64.pow(2)),
            ("Ki", 1024_u64),
        ]
        .into_iter()
        .find_map(|(unit, scale)| value.strip_suffix(unit).map(|n| (n, scale)))
        .ok_or_else(|| format!("unsupported memory quantity: {value}"))?
    } else {
        value.strip_suffix('m').map_or((value, 1000), |n| (n, 1))
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(format!("invalid quantity: {value}").into());
    }
    let denominator = 10_u128
        .checked_pow(fraction.len().try_into()?)
        .ok_or("quantity too precise")?;
    let digits: u128 = format!("{whole}{fraction}").parse()?;
    let scaled = digits
        .checked_mul(multiplier.into())
        .ok_or("quantity overflow")?;
    if scaled % denominator != 0 {
        return Err(format!("fractional base unit: {value}").into());
    }
    Ok((scaled / denominator).try_into()?)
}

fn share(value: Option<u64>, numerator: u64, denominator: u64, unit: &str) -> (String, String) {
    match value {
        None => (String::new(), String::new()),
        Some(value) => {
            let conductor =
                (u128::from(value) * u128::from(numerator) / u128::from(denominator)) as u64;
            (
                format!("{conductor}{unit}"),
                format!("{}{unit}", value - conductor),
            )
        }
    }
}

/// Render limits from the root bound, requests from the archetype floor (or a pinned
/// override), and the exact 5/8 memory, 1/2 CPU split with the remainder to storage.
pub fn render_envelope(manifest: &RuntimeManifest) -> Result<RenderedEnvelope> {
    RuntimeManifest::from_json(&serde_json::to_string(manifest)?)?;
    let Some(envelope) = &manifest.envelope else {
        return Ok(RenderedEnvelope::default());
    };
    let archetype = manifest
        .archetype
        .as_deref()
        .ok_or("envelope requires archetype")?;
    let budgets: Value = serde_json::from_str(BUDGETS)?;
    let budget = budgets["budgets"]
        .get(archetype)
        .ok_or_else(|| format!("unknown archetype: {archetype}"))?;
    let floor_memory = quantity(
        budget["memoryRequest"]
            .as_str()
            .ok_or("memory request floor missing")?,
        true,
    )?;
    let floor_cpu = quantity(
        budget["cpuRequest"]
            .as_str()
            .ok_or("cpu request floor missing")?,
        false,
    )?;
    let requested = envelope.requests.as_ref();
    let memory = requested
        .and_then(|q| q.memory_bytes)
        .unwrap_or(floor_memory);
    let cpu = requested
        .and_then(|q| q.cpu_millis)
        .map(u64::from)
        .unwrap_or(floor_cpu);
    if memory < floor_memory || cpu < floor_cpu {
        return Err("request override is below archetype floor".into());
    }
    if envelope
        .bound
        .memory_bytes
        .is_some_and(|limit| memory > limit)
        || envelope
            .bound
            .cpu_millis
            .is_some_and(|limit| cpu > u64::from(limit))
    {
        return Err("request exceeds declared limit".into());
    }
    let memory_limit = envelope.bound.memory_bytes.map(|v| v / MIB);
    let cpu_limit = envelope.bound.cpu_millis.map(u64::from);
    let (cmr, smr) = share(Some(memory / MIB), 5, 8, "Mi");
    let (cml, sml) = share(memory_limit, 5, 8, "Mi");
    let (ccr, scr) = share(Some(cpu), 1, 2, "m");
    let (ccl, scl) = share(cpu_limit, 1, 2, "m");
    Ok(RenderedEnvelope {
        edgenode_memory_request: format!("{}Mi", memory / MIB),
        edgenode_memory_limit: memory_limit.map_or(String::new(), |v| format!("{v}Mi")),
        edgenode_cpu_request: format!("{cpu}m"),
        edgenode_cpu_limit: cpu_limit.map_or(String::new(), |v| format!("{v}m")),
        conductor_memory_request: cmr,
        storage_memory_request: smr,
        conductor_memory_limit: cml,
        storage_memory_limit: sml,
        conductor_cpu_request: ccr,
        storage_cpu_request: scr,
        conductor_cpu_limit: ccl,
        storage_cpu_limit: scl,
    })
}

/// Compare the declared CID first, then resource values. Equivalent Ki/Mi/Gi/Ti
/// spellings compare by quantity; no fleet strings are normalized or rewritten.
pub fn drift_verdict(human: &Value, manifest: &RuntimeManifest) -> Result<DriftVerdict> {
    let name = human["name"]
        .as_str()
        .ok_or("human name missing")?
        .to_owned();
    let Some(pin) = human.get("runtimeManifest") else {
        return Ok(DriftVerdict::Unpinned { human: name });
    };
    let expected = pin["cid"].as_str().ok_or("runtimeManifest.cid missing")?;
    let actual = manifest.cid()?;
    if expected != actual {
        return Ok(DriftVerdict::PinMismatch {
            human: name,
            expected: expected.into(),
            actual,
        });
    }
    let rendered = serde_json::to_value(render_envelope(manifest)?)?;
    for field in [
        "edgenodeMemoryRequest",
        "edgenodeMemoryLimit",
        "edgenodeCpuRequest",
        "edgenodeCpuLimit",
    ] {
        let pinned = human[field].as_str().unwrap_or("");
        let value = rendered[field].as_str().unwrap_or("");
        if pinned == value {
            continue;
        }
        let memory = field.contains("Memory");
        let equivalent = !pinned.is_empty()
            && !value.is_empty()
            && quantity(pinned, memory)? == quantity(value, memory)?;
        if !equivalent {
            return Ok(DriftVerdict::RenderDrift {
                human: name,
                field: field.into(),
                pinned: pinned.into(),
                rendered: value.into(),
            });
        }
    }
    // A pin must also belong to this human's declared archetype.
    if let Some(archetype) = manifest.archetype.as_deref() {
        if human["deviceArchetype"].as_str() != Some(archetype) {
            return Ok(DriftVerdict::RenderDrift {
                human: name,
                field: "deviceArchetype".into(),
                pinned: human["deviceArchetype"].as_str().unwrap_or("").into(),
                rendered: archetype.into(),
            });
        }
    }
    Ok(DriftVerdict::Fresh)
}

/// Verify active deployment records using files confined to the supplied manifest directory.
/// Repository-relative pin paths resolve by their runtime-directory filename, independent of cwd.
pub fn verify(deployments: &Path, manifests_dir: &Path) -> Result<Vec<DriftVerdict>> {
    let data: Value = serde_json::from_str(&fs::read_to_string(deployments)?)?;
    let humans = data["humans"]
        .as_array()
        .ok_or("deployments.humans must be an array")?;
    let root = manifests_dir.canonicalize()?;
    let mut verdicts = Vec::new();
    for human in humans {
        if human["suspended"].as_bool() == Some(true) {
            continue;
        }
        let name = human["name"].as_str().ok_or("human name missing")?;
        let Some(pin) = human.get("runtimeManifest") else {
            verdicts.push(DriftVerdict::Unpinned { human: name.into() });
            continue;
        };
        let path = pin["path"].as_str().ok_or("runtimeManifest.path missing")?;
        let relative = path
            .strip_prefix("genesis/orchestrator/manifests/runtime/")
            .unwrap_or(path);
        if Path::new(relative).components().count() != 1 || !relative.ends_with(".manifest.json") {
            return Err(
                format!("{name}: manifest path must name a file inside runtime directory").into(),
            );
        }
        let file = root.join(relative).canonicalize()?;
        if !file.starts_with(&root) {
            return Err(format!("{name}: manifest escapes runtime directory").into());
        }
        let manifest = RuntimeManifest::from_json(&fs::read_to_string(file)?)?;
        verdicts.push(drift_verdict(human, &manifest)?);
    }
    Ok(verdicts)
}

pub mod observe;
pub use observe::{fold_observation, FieldChange, Observation, ObserveRefusal};
