use crate::bin_store::Refusal;
pub use seam_contracts::freshness::NetworkStage;
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

/// Environment declaration for the stage of the invoking process.
pub const DEVICE_STAGE_ENV: &str = "ELOHIM_DEVICE_STAGE";

const DECLARATION_FILE: &str = "stage.json";
const ISOLATED_HOME_CURE: &str =
    "run fixtures under an isolated XDG_CONFIG_HOME declared simulacra";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Declaration {
    stage: String,
    declared_by: String,
    reason: String,
}

/// Resolve and verify the process stage against the declaration beside a device key.
pub fn resolve(device_key: &Path) -> Result<NetworkStage, Refusal> {
    resolve_with(device_key, &|key| std::env::var_os(key))
}

/// Resolve with an injected environment reader so stage admission needs no process-global tests.
pub fn resolve_with<F>(device_key: &Path, env: &F) -> Result<NetworkStage, Refusal>
where
    F: Fn(&str) -> Option<OsString>,
{
    let process = match env(DEVICE_STAGE_ENV) {
        Some(raw) => parse_os_stage(&raw, DEVICE_STAGE_ENV)?,
        None => enforced(),
    };
    let home = home_stage(device_key)?;
    if process != home {
        return Err(Refusal::new(
            "stage-mismatch",
            format!(
                "{} session in a {} device home; {ISOLATED_HOME_CURE}",
                process.label(),
                home.label()
            ),
        ));
    }
    Ok(process)
}

fn device_home(device_key: &Path) -> Result<&Path, Refusal> {
    device_key
        .parent()
        .ok_or_else(|| Refusal::new("device-home", "key has no parent"))
}

fn home_stage(device_key: &Path) -> Result<NetworkStage, Refusal> {
    let path = device_home(device_key)?.join(DECLARATION_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(enforced()),
        Err(error) => {
            return Err(Refusal::new(
                "stage-invalid",
                format!("{}: {error}", path.display()),
            ))
        }
    };
    let declaration: Declaration = serde_json::from_slice(&bytes)
        .map_err(|error| Refusal::new("stage-invalid", format!("{}: {error}", path.display())))?;
    if declaration.declared_by.trim().is_empty() || declaration.reason.trim().is_empty() {
        return Err(Refusal::new(
            "stage-invalid",
            format!(
                "{}: declaredBy and reason must be non-empty",
                path.display()
            ),
        ));
    }
    parse_stage(&declaration.stage, &path.display().to_string())
}

fn parse_os_stage(raw: &OsString, source: &str) -> Result<NetworkStage, Refusal> {
    let raw = raw
        .to_str()
        .ok_or_else(|| Refusal::new("stage-invalid", format!("{source} is not UTF-8")))?;
    parse_stage(raw, source)
}

fn parse_stage(raw: &str, source: &str) -> Result<NetworkStage, Refusal> {
    let stage = raw.parse::<NetworkStage>().map_err(|()| {
        Refusal::new(
            "stage-invalid",
            format!("{source} declares unrecognized device stage {raw:?}"),
        )
    })?;
    match stage {
        NetworkStage::Simulacra | NetworkStage::Enforced => Ok(stage),
        NetworkStage::Bootstrap | NetworkStage::Coordinated => Err(Refusal::new(
            "stage-invalid",
            format!(
                "{source} declares {}; device homes support only simulacra or enforced",
                stage.label()
            ),
        )),
    }
}

/// Return the stage-partitioned session directory without reading or creating device state.
pub fn sessions_directory(device_key: &Path, stage: NetworkStage) -> Result<PathBuf, Refusal> {
    let home = device_home(device_key)?;
    match stage {
        NetworkStage::Enforced => Ok(home.join("sessions")),
        NetworkStage::Simulacra => Ok(home.join("stages/simulacra/sessions")),
        NetworkStage::Bootstrap | NetworkStage::Coordinated => Err(Refusal::new(
            "stage-invalid",
            format!(
                "{} is not a device-home stage; expected simulacra or enforced",
                stage.label()
            ),
        )),
    }
}

pub(crate) fn enforced() -> NetworkStage {
    NetworkStage::Enforced
}

pub(crate) fn is_enforced(stage: &NetworkStage) -> bool {
    *stage == NetworkStage::Enforced
}

pub(crate) fn serialize<S>(stage: &NetworkStage, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(stage.label())
}

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<NetworkStage, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_stage(&raw, "envelope stage").map_err(serde::de::Error::custom)
}
