//! Local capacity declaration for `epr device status`.
//!
//! Harms held at this first station:
//! - This is a declaration about one device, never a rank or refusal. It changes no arrival
//!   state except that an explicit `neither` ("does not build") makes a missing gate toolchain
//!   okay.
//! - Observation is cheap local I/O only: layout resolution, `statvfs`, `/proc`, `PATH`, and the
//!   existing device-private manifest reader. It spawns no probe and takes no flow-store lock.
//! - Nothing here crosses to siblings. A later lane may publish a bounded capacity offer and its
//!   reach; this lane has no signal, gossip record, peer projection, or doorway route.
//! - The manifest field is additive. Serde's previous reader shape ignores it, so an older `epr`
//!   neither aborts nor assigns meaning to it.
//! - The human line and `--json` are rendered from the same [`Capacity`] value.

use super::arrival::ToolchainFact;
use ark_core::BuildRole;
use elohim_device_runtime::layout::DeviceLayout;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// The one local capacity reading attached to the capacity status line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    pub cargo_pool: Option<PathBuf>,
    pub cargo_pool_present: bool,
    pub cargo_pool_free_bytes: Option<u64>,
    pub toolchain_pin: Option<String>,
    pub toolchain_installed: Option<bool>,
    pub cores: Option<usize>,
    pub memory_bytes: Option<u64>,
    pub container_runtime: Option<String>,
    pub build_role: Option<BuildRole>,
    pub role_until: Option<String>,
    pub does_not_build: bool,
}

impl Capacity {
    pub(super) fn line(&self) -> String {
        let pool = match (&self.cargo_pool, self.cargo_pool_free_bytes) {
            (Some(path), Some(free)) if self.cargo_pool_present => {
                format!("pool present {} free {free} bytes", path.display())
            }
            (Some(path), _) if self.cargo_pool_present => {
                format!("pool present {} free bytes unknown", path.display())
            }
            (Some(path), _) => format!("pool absent {} free 0 bytes", path.display()),
            (None, _) => "pool absent (nothing names it) free 0 bytes".into(),
        };
        let rust = match (&self.toolchain_pin, self.toolchain_installed) {
            (None, _) => "rust pin absent".into(),
            (Some(pin), Some(true)) => format!("rust {pin} installed"),
            (Some(pin), Some(false)) => format!("rust {pin} not installed"),
            (Some(pin), None) => format!("rust {pin} installation unknown"),
        };
        let cores = self
            .cores
            .map_or_else(|| "cores unknown".into(), |n| format!("{n} cores"));
        let memory = self.memory_bytes.map_or_else(
            || "memory unknown".into(),
            |bytes| format!("memory {bytes} bytes"),
        );
        let runtime = self.container_runtime.as_ref().map_or_else(
            || "container runtime absent".into(),
            |name| format!("container runtime {name}"),
        );
        let role = match (self.build_role, self.role_until.as_deref()) {
            (None, _) => "role undeclared".into(),
            (Some(BuildRole::Builder), Some(until)) => format!("role builder until {until}"),
            (Some(BuildRole::Builder), None) => "role builder (invalid: no until)".into(),
            (Some(BuildRole::Carrier), _) => {
                "role carrier (eligibility only; expiring carry switch decides opt-in)".into()
            }
            (Some(BuildRole::Neither), _) => "role neither; does not build".into(),
        };
        [pool, rust, cores, memory, runtime, role].join(" · ")
    }

    fn builder_reason(&self, now: chrono::DateTime<chrono::Utc>) -> Option<String> {
        match self.build_role {
            None => return Some("build role undeclared".into()),
            Some(BuildRole::Neither) => return Some("device declares does not build".into()),
            Some(BuildRole::Carrier) => {
                return Some("carrier eligibility is not a local builder declaration".into())
            }
            Some(BuildRole::Builder) => {}
        }
        let current = self
            .role_until
            .as_deref()
            .and_then(|until| chrono::DateTime::parse_from_rfc3339(until).ok())
            .is_some_and(|until| until > now);
        if !current {
            return Some(format!(
                "builder commitment expired or has no valid until ({})",
                self.role_until.as_deref().unwrap_or("absent")
            ));
        }
        if !self.cargo_pool_present {
            return Some("cargo pool absent".into());
        }
        if self.toolchain_pin.is_some() && self.toolchain_installed != Some(true) {
            return Some("pinned Rust toolchain not installed".into());
        }
        None
    }
}

pub(super) fn observe(
    layout: Option<&DeviceLayout>,
    toolchain: &ToolchainFact,
    env: &dyn Fn(&str) -> Option<String>,
) -> Capacity {
    let cargo_pool = layout
        .and_then(DeviceLayout::cargo_pool)
        .map(Path::to_owned);
    let cargo_pool_present = cargo_pool.as_deref().is_some_and(Path::is_dir);
    let cargo_pool_free_bytes = cargo_pool
        .as_deref()
        .filter(|_| cargo_pool_present)
        .and_then(|path| rustix::fs::statvfs(path).ok())
        .map(|stat| stat.f_bavail.saturating_mul(stat.f_frsize));
    let proc_root = env("EPR_PROC_ROOT")
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from("/proc"), PathBuf::from);
    let cores = fs::read_to_string(proc_root.join("cpuinfo"))
        .ok()
        .map(|text| {
            text.lines()
                .filter(|line| {
                    line.split_once(':')
                        .is_some_and(|(key, _)| key.trim() == "processor")
                })
                .count()
        })
        .filter(|count| *count > 0)
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from));
    let memory_bytes = fs::read_to_string(proc_root.join("meminfo"))
        .ok()
        .and_then(|text| memory_from_meminfo(&text));
    let container_runtime = env("PATH").and_then(|path| runtime_on_path(&path));
    let build = crate::device::declaration::local_root()
        .and_then(|root| {
            elohim_device_runtime::manifest_store::ManifestStore::read_build_role(&root)
                .map_err(|error| error.to_string())
        })
        .ok()
        .flatten();
    let build_role = build.as_ref().map(|build| build.role);
    let role_until = build.as_ref().and_then(|build| build.until.clone());
    Capacity {
        cargo_pool,
        cargo_pool_present,
        cargo_pool_free_bytes,
        toolchain_pin: toolchain.pin.clone(),
        toolchain_installed: toolchain.installed,
        cores,
        memory_bytes,
        container_runtime,
        build_role,
        role_until,
        does_not_build: build_role == Some(BuildRole::Neither),
    }
}

fn memory_from_meminfo(text: &str) -> Option<u64> {
    let value = text.lines().find_map(|line| {
        let rest = line.strip_prefix("MemTotal:")?;
        rest.split_whitespace().next()?.parse::<u64>().ok()
    })?;
    value.checked_mul(1024)
}

fn runtime_on_path(path: &str) -> Option<String> {
    std::env::split_paths(path).find_map(|dir| {
        ["podman", "docker"].into_iter().find_map(|name| {
            let candidate = dir.join(name);
            is_executable(&candidate).then(|| name.to_owned())
        })
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Read one commit's changed files, then hand selection to the repository's gate runner.
pub(super) fn would_run(
    root: &Path,
    commit: &str,
    capacity: &Capacity,
) -> Result<Vec<String>, String> {
    let changed = crate::process::require(
        "git",
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            commit,
        ],
        root,
    )
    .map_err(|error| error.to_string())?
    .stdout;
    let runner = root.join("genesis/orchestrator/gate-runner.mjs");
    let mut child = Command::new("node")
        .arg(&runner)
        .args(["--changed-file-list", "--names", "--order", "cost"])
        .env("GATE_ROOT", root)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("start {}: {error}", runner.display()))?;
    child
        .stdin
        .take()
        .ok_or("gate runner stdin unavailable")?
        .write_all(changed.as_bytes())
        .map_err(|error| format!("write changed paths to gate runner: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("wait for gate runner: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "gate selection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let reason = capacity.builder_reason(chrono::Utc::now());
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|gate| match &reason {
            None => format!("{gate}: run here"),
            Some(reason) => format!("{gate}: leave as miss ({reason})"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn local_capacity_reads_proc_path_and_statvfs_without_a_probe() {
        let dir = tempfile::tempdir().unwrap();
        let proc = dir.path().join("proc");
        let pool = dir.path().join("pool");
        let bin = dir.path().join("bin");
        fs::create_dir_all(&proc).unwrap();
        fs::create_dir(&pool).unwrap();
        fs::create_dir(&bin).unwrap();
        fs::write(proc.join("cpuinfo"), "processor: 0\nprocessor: 1\n").unwrap();
        fs::write(proc.join("meminfo"), "MemTotal: 2048 kB\n").unwrap();
        let docker = bin.join("docker");
        fs::write(&docker, "").unwrap();
        fs::set_permissions(&docker, fs::Permissions::from_mode(0o755)).unwrap();
        let mut layout =
            elohim_device_runtime::layout::resolve_with(&elohim_device_runtime::layout::Host {
                env: &|name| {
                    (name == "CARGO_TARGET_POOL_ROOT").then(|| pool.as_os_str().to_owned())
                },
                exists: &|path| path == Path::new("/projects"),
                cwd: None,
            })
            .unwrap();
        layout.cargo_pool.path = Some(pool.clone());
        let env = |name: &str| match name {
            "EPR_PROC_ROOT" => Some(proc.display().to_string()),
            "PATH" => Some(bin.display().to_string()),
            _ => None,
        };
        let started = std::time::Instant::now();
        let capacity = observe(
            Some(&layout),
            &ToolchainFact {
                pin: Some("1.100.0".into()),
                installed: Some(false),
                location: None,
                why_unknown: None,
            },
            &env,
        );
        assert!(started.elapsed() < std::time::Duration::from_millis(50));
        assert!(capacity.cargo_pool_present);
        assert!(capacity.cargo_pool_free_bytes.is_some());
        assert_eq!(capacity.cores, Some(2));
        assert_eq!(capacity.memory_bytes, Some(2 * 1024 * 1024));
        assert_eq!(capacity.container_runtime.as_deref(), Some("docker"));
    }

    #[test]
    fn capacity_never_turns_observed_absence_into_a_refusal() {
        let capacity = Capacity {
            cargo_pool: None,
            cargo_pool_present: false,
            cargo_pool_free_bytes: None,
            toolchain_pin: Some("1.100.0".into()),
            toolchain_installed: Some(false),
            cores: None,
            memory_bytes: None,
            container_runtime: None,
            build_role: Some(BuildRole::Neither),
            role_until: None,
            does_not_build: true,
        };
        assert!(capacity.line().contains("does not build"));
        assert_eq!(
            capacity.builder_reason(chrono::Utc::now()).as_deref(),
            Some("device declares does not build")
        );
    }
}
