//! Ephemeral session identity, reconstructed from environment and the local process table.
//! Runtime names belong only to the registry; this module has no transport or persisted state.

use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// A session and its process owner, reconstructed on this device.
pub struct SessionIdentity {
    /// Registry runtime id, or declared, tty, or none.
    pub runtime: String,
    /// Stable session key when an owner or declaration exists.
    pub session: Option<String>,
    /// Owner pid, paired with its start time to reject pid reuse.
    pub owner_pid: Option<u32>,
    #[serde(
        serialize_with = "serialize_ticks",
        deserialize_with = "deserialize_ticks"
    )]
    /// Linux process start time in clock ticks.
    pub owner_start_time: Option<u64>,
    /// How the identity was resolved.
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Origin of an ephemeral session identity.
pub enum Source {
    /// Explicit harness declaration.
    Declared,
    /// Nearest registered runtime process.
    RuntimeAncestor,
    /// Controlling terminal session leader.
    TtyLeader,
    /// No session can be inferred.
    None,
}

#[derive(Debug, Default, Deserialize)]
/// Agent runtime names and their session environment variables.
pub struct Registry {
    /// Registered runtime descriptions.
    pub runtimes: Vec<Runtime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// One agent runtime description.
pub struct Runtime {
    /// Runtime identifier.
    pub id: String,
    /// Process comm names identifying this runtime.
    pub comm: Vec<String>,
    /// Environment variables containing native session ids.
    pub session_env: Vec<String>,
}

struct Process {
    comm: String,
    parent: u32,
    leader: u32,
    tty: i64,
    start: u64,
}

fn process(root: &Path, pid: u32) -> Option<Process> {
    if pid == 0 {
        return None;
    }
    let stat = fs::read_to_string(root.join(pid.to_string()).join("stat")).ok()?;
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let comm = stat.get(open + 1..close)?.to_owned();
    let fields: Vec<_> = stat.get(close + 1..)?.split_whitespace().collect();
    Some(Process {
        comm,
        parent: fields.get(1)?.parse().ok()?,
        leader: fields.get(3)?.parse().ok()?,
        tty: fields.get(4)?.parse().ok()?,
        start: fields.get(19)?.parse().ok()?,
    })
}

fn nonblank(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn identity(
    runtime: &str,
    session: Option<String>,
    owner: Option<(u32, u64)>,
    source: Source,
) -> SessionIdentity {
    SessionIdentity {
        runtime: runtime.to_owned(),
        session,
        owner_pid: owner.map(|o| o.0),
        owner_start_time: owner.map(|o| o.1),
        source,
    }
}

/// Resolve declared identity, nearest registered ancestor, controlling TTY leader, then none.
/// A missing process or registry is an honest lack of a detectable session, never a CLI error.
pub fn resolve_with(
    env: &dyn Fn(&str) -> Option<String>,
    proc_root: &Path,
    start_pid: u32,
    registry: &Registry,
) -> SessionIdentity {
    if let Some(session) = nonblank(env("ELOHIM_SESSION_ID")) {
        let owner = nonblank(env("ELOHIM_SESSION_OWNER_PID"))
            .and_then(|pid| pid.parse::<u32>().ok())
            .and_then(|pid| process(proc_root, pid).map(|p| (pid, p.start)));
        return identity("declared", Some(session), owner, Source::Declared);
    }
    let current = process(proc_root, start_pid);
    let mut pid = current.as_ref().map_or(0, |p| p.parent);
    let mut seen = HashSet::from([start_pid]);
    for _ in 0..256 {
        if pid == 0 || !seen.insert(pid) {
            break;
        }
        let Some(ancestor) = process(proc_root, pid) else {
            break;
        };
        if let Some(runtime) = registry
            .runtimes
            .iter()
            .find(|r| r.comm.contains(&ancestor.comm))
        {
            // /proc exposes the environment P inherited at exec. A child value
            // already there belongs to an outer session, not to P.
            let inherited = fs::read(proc_root.join(pid.to_string()).join("environ")).ok();
            let session = inherited
                .as_ref()
                .and_then(|bytes| {
                    runtime.session_env.iter().find_map(|key| {
                        let raw = env(key)?;
                        let value = nonblank(Some(raw.clone()))?;
                        let entry = format!("{key}={raw}");
                        (!bytes.split(|b| *b == 0).any(|v| v == entry.as_bytes())).then_some(value)
                    })
                })
                .unwrap_or_else(|| format!("{}-{pid}-{}", runtime.id, ancestor.start));
            return identity(
                &runtime.id,
                Some(session),
                Some((pid, ancestor.start)),
                Source::RuntimeAncestor,
            );
        }
        pid = ancestor.parent;
    }
    if let Some(current) = current.filter(|p| p.tty != 0) {
        if let Some(leader) = process(proc_root, current.leader) {
            return identity(
                "tty",
                Some(format!("tty-{}-{}", current.leader, leader.start)),
                Some((current.leader, leader.start)),
                Source::TtyLeader,
            );
        }
    }
    identity("none", None, None, Source::None)
}

/// Resolve the current process using the environment and runtime registry.
pub fn resolve() -> SessionIdentity {
    let env = |key: &str| std::env::var(key).ok();
    let proc_root = nonblank(env("EPR_PROC_ROOT")).unwrap_or_else(|| "/proc".into());
    let registry_path = nonblank(env("EPR_AGENT_RUNTIMES"))
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let path = std::env::current_exe()
                .ok()?
                .parent()?
                .join("agent-runtimes.json");
            path.is_file().then_some(path)
        })
        .or_else(|| {
            std::env::current_dir()
                .ok()?
                .ancestors()
                .find(|p| p.join("genesis/agentic").is_dir())
                .map(|p| p.join("genesis/agentic/agent-runtimes.json"))
        });
    let registry = registry_path
        .and_then(|p| fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    resolve_with(&env, Path::new(&proc_root), std::process::id(), &registry)
}

// Proc ticks use the view convention for u64: decimal text keeps JavaScript lossless.
fn serialize_ticks<S: serde::Serializer>(
    ticks: &Option<u64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    ticks.map(|value| value.to_string()).serialize(serializer)
}

fn deserialize_ticks<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|value| value.parse().map_err(serde::de::Error::custom))
        .transpose()
}

/// Read a process start time without walking its ancestry; used for lease liveness.
pub fn process_start_time(root: &Path, pid: u32) -> Option<u64> {
    process(root, pid).map(|p| p.start)
}
