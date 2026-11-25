//! Bounded local capacity and build-role reporting for `epr device status`.
//!
//! The build role is device-local operational state. It is not part of a runtime manifest, does
//! not cross to peers, and confers no placement or carry authority.

use super::arrival::ToolchainFact;
use elohim_device_runtime::layout::DeviceLayout;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Output, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

const BUILD_ROLE_FILE: &str = "build-role.json";
const STATUS_BUDGET: Duration = Duration::from_secs(2);
const WOULD_BUDGET: Duration = Duration::from_secs(10);

#[cfg(test)]
const BUILD_ROLE_SCHEMA: &str = include_str!("build-role.schema.json");

/// This device's local participation declaration. Carry activation remains a separate expiring
/// switch; these values never elect the device to run work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildRole {
    Builder,
    Carrier,
    Neither,
}

/// The independently versioned record beside the device key and layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildRoleRecord {
    pub version: u32,
    pub role: BuildRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_error: Option<String>,
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
            (Some(path), _) if self.timed_out.is_some() => {
                format!("pool {} presence and free bytes unknown", path.display())
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
            || "container runtime absent or unknown".into(),
            |name| format!("container runtime {name}"),
        );
        let role = if let Some(error) = &self.role_error {
            format!("role unreadable: {error}")
        } else {
            match (self.build_role, self.role_until.as_deref()) {
                (None, _) => "role undeclared".into(),
                (Some(BuildRole::Builder), Some(until)) => {
                    let expired = chrono::DateTime::parse_from_rfc3339(until)
                        .is_ok_and(|date| date <= chrono::Utc::now());
                    if expired {
                        format!("role builder, expired {until}: declare again")
                    } else {
                        format!("role builder until {until}")
                    }
                }
                (Some(BuildRole::Builder), None) => {
                    "role builder (invalid: no until): declare again".into()
                }
                (Some(BuildRole::Carrier), _) => {
                    "role carrier (eligibility only; expiring carry switch decides opt-in)".into()
                }
                (Some(BuildRole::Neither), _) => "role neither; does not build".into(),
            }
        };
        let mut fields = vec![pool, rust, cores, memory, runtime, role];
        if let Some(stage) = &self.timed_out {
            fields.push(format!("timed out after 2s while reading {stage}"));
        }
        fields.join(" · ")
    }

    fn timed_out(layout: Option<&DeviceLayout>, toolchain: &ToolchainFact, stage: String) -> Self {
        Self {
            cargo_pool: layout
                .and_then(DeviceLayout::cargo_pool)
                .map(Path::to_owned),
            cargo_pool_present: false,
            cargo_pool_free_bytes: None,
            toolchain_pin: toolchain.pin.clone(),
            toolchain_installed: toolchain.installed,
            cores: None,
            memory_bytes: None,
            container_runtime: None,
            build_role: None,
            role_until: None,
            does_not_build: false,
            timed_out: Some(stage),
            role_error: None,
        }
    }
}

fn status_budget() -> Duration {
    test_budget("EPR_STATUS_TIMEOUT_MS", STATUS_BUDGET)
}

fn would_budget() -> Duration {
    test_budget("EPR_WOULD_TIMEOUT_MS", WOULD_BUDGET)
}

fn test_budget(name: &str, default: Duration) -> Duration {
    if std::env::var_os("EPR_UNDER_TEST").is_none() {
        return default;
    }
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

/// Status owns one two-second budget for statvfs, proc reads, PATH checks and the role record.
/// Synchronous filesystem calls cannot be cancelled, so an expired worker is detached and its
/// late answer is discarded; the status reader itself returns at the deadline.
pub(super) fn observe_bounded(
    layout: Option<&DeviceLayout>,
    toolchain: &ToolchainFact,
) -> Capacity {
    observe_bounded_for(layout, toolchain, status_budget(), None)
}

fn observe_bounded_for(
    layout: Option<&DeviceLayout>,
    toolchain: &ToolchainFact,
    budget: Duration,
    stall_at: Option<&'static str>,
) -> Capacity {
    let layout = layout.cloned();
    let toolchain = toolchain.clone();
    let fallback_layout = layout.clone();
    let fallback_toolchain = toolchain.clone();
    let proc_root = std::env::var("EPR_PROC_ROOT").ok();
    let path = std::env::var("PATH").ok();
    let stage = Arc::new(Mutex::new("capacity inputs"));
    let worker_stage = Arc::clone(&stage);
    let (send, receive) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("device-capacity".into())
        .spawn(move || {
            let set_stage = |name| {
                if let Ok(mut current) = worker_stage.lock() {
                    *current = name;
                }
                if stall_at == Some(name) {
                    std::thread::sleep(Duration::from_millis(100));
                }
            };
            let capacity = observe_values(
                layout.as_ref(),
                &toolchain,
                proc_root.as_deref(),
                path.as_deref(),
                &set_stage,
            );
            let _ = send.send(capacity);
        });
    if let Err(error) = spawned {
        return Capacity::timed_out(
            fallback_layout.as_ref(),
            &fallback_toolchain,
            format!("capacity worker: {error}"),
        );
    }
    match receive.recv_timeout(budget) {
        Ok(capacity) => capacity,
        Err(_) => {
            let current = stage
                .lock()
                .map(|value| (*value).to_owned())
                .unwrap_or_else(|_| "capacity inputs".into());
            Capacity::timed_out(fallback_layout.as_ref(), &fallback_toolchain, current)
        }
    }
}

#[cfg(test)]
fn observe(
    layout: Option<&DeviceLayout>,
    toolchain: &ToolchainFact,
    env: &dyn Fn(&str) -> Option<String>,
) -> Capacity {
    observe_values(
        layout,
        toolchain,
        env("EPR_PROC_ROOT").as_deref(),
        env("PATH").as_deref(),
        &|_| {},
    )
}

fn observe_values(
    layout: Option<&DeviceLayout>,
    toolchain: &ToolchainFact,
    proc_root: Option<&str>,
    path: Option<&str>,
    stage: &dyn Fn(&'static str),
) -> Capacity {
    let cargo_pool = layout
        .and_then(DeviceLayout::cargo_pool)
        .map(Path::to_owned);
    stage("statvfs");
    let cargo_pool_present = cargo_pool.as_deref().is_some_and(Path::is_dir);
    let cargo_pool_free_bytes = cargo_pool
        .as_deref()
        .filter(|_| cargo_pool_present)
        .and_then(|path| rustix::fs::statvfs(path).ok())
        .map(|stat| stat.f_bavail.saturating_mul(stat.f_frsize));

    stage("processor and memory files");
    let proc_root = proc_root.map_or_else(|| PathBuf::from("/proc"), PathBuf::from);
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

    stage("PATH checks");
    let container_runtime = path.and_then(runtime_on_path);

    stage("build-role.json");
    let role = layout
        .and_then(|layout| layout.config_dir().ok())
        .map(read_build_role)
        .transpose()
        .map(Option::flatten);
    let (record, role_error) = match role {
        Ok(record) => (record, None),
        Err(error) => (None, Some(error)),
    };
    let build_role = record.as_ref().map(|record| record.role);
    let role_until = record.and_then(|record| record.until);
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
        timed_out: None,
        role_error,
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

fn validate_build_role(record: &BuildRoleRecord) -> Result<(), String> {
    if record.version != 1 {
        return Err(format!(
            "build role version {} is unsupported; expected 1",
            record.version
        ));
    }
    match (record.role, record.until.as_deref()) {
        (BuildRole::Builder, Some(until)) => {
            chrono::DateTime::parse_from_rfc3339(until)
                .map_err(|error| format!("build role until must be RFC3339: {error}"))?;
        }
        (BuildRole::Builder, None) => {
            return Err("builder is a standing commitment and needs an until time".into());
        }
        (BuildRole::Carrier, Some(_)) => {
            return Err(
                "carrier expiry belongs to the held-tool-watch carry switch, not build-role.json"
                    .into(),
            );
        }
        (BuildRole::Neither, Some(_)) => {
            return Err("neither carries no standing commitment".into());
        }
        (BuildRole::Carrier | BuildRole::Neither, None) => {}
    }
    Ok(())
}

fn read_build_role(config_dir: &Path) -> Result<Option<BuildRoleRecord>, String> {
    let path = config_dir.join(BUILD_ROLE_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let record: BuildRoleRecord =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    validate_build_role(&record)?;
    Ok(Some(record))
}

fn write_build_role(config_dir: &Path, record: &BuildRoleRecord) -> Result<PathBuf, String> {
    validate_build_role(record)?;
    fs::create_dir_all(config_dir)
        .map_err(|error| format!("create {}: {error}", config_dir.display()))?;
    let path = config_dir.join(BUILD_ROLE_FILE);
    let mut bytes = serde_json::to_vec_pretty(record).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let mut temporary = tempfile::NamedTempFile::new_in(config_dir)
        .map_err(|error| format!("create temporary build role: {error}"))?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| format!("write temporary build role: {error}"))?;
    temporary
        .persist(&path)
        .map_err(|error| format!("replace {}: {}", path.display(), error.error))?;
    fs::File::open(config_dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("fsync {}: {error}", config_dir.display()))?;
    Ok(path)
}

/// Declare this device's role. There is deliberately no root argument: the key and current
/// checkout's roster standing identify the only device this command may change.
pub(super) fn run_build_role(args: &[String]) -> Result<ExitCode, String> {
    let mut role = None;
    let mut until = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "builder" | "carrier" | "neither" if role.is_none() => role = Some(arg.as_str()),
            "--until" => {
                until = Some(
                    iter.next()
                        .ok_or("--until needs an RFC3339 time")?
                        .to_owned(),
                )
            }
            other => {
                return Err(format!(
                    "unexpected build-role argument {other}; no --root is accepted"
                ))
            }
        }
    }
    let role = match role {
        Some("builder") => BuildRole::Builder,
        Some("carrier") => BuildRole::Carrier,
        Some("neither") => BuildRole::Neither,
        _ => return Err("build-role needs builder, carrier or neither".into()),
    };
    let record = BuildRoleRecord {
        version: 1,
        role,
        until,
    };
    validate_build_role(&record)?;

    let here = std::env::current_dir().map_err(|error| error.to_string())?;
    let root = crate::git::checkout(&here).unwrap_or(here);
    let key = crate::device_key::resolve_path().map_err(|error| error.to_string())?;
    match crate::actor::device_state_read_only(&root, &key) {
        crate::actor::DeviceState::Standing(_) => {}
        crate::actor::DeviceState::RootContested(_) => {
            return Err("this device's roster standing is contested; build role not written".into())
        }
        crate::actor::DeviceState::Unwitnessed => {
            return Err(
                "this device has no roster standing in this checkout; bind it before declaring a build role"
                    .into(),
            )
        }
    }
    let config_dir = key
        .parent()
        .ok_or("device key has no parent config directory")?;
    let path = write_build_role(config_dir, &record)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?
    );
    eprintln!("build role written atomically to {}", path.display());
    Ok(ExitCode::SUCCESS)
}

enum CommandAnswer {
    Done(Output),
    TimedOut,
}

fn command_until(command: &mut Command, deadline: Instant) -> Result<CommandAnswer, String> {
    match crate::host_link::bounded_output(command, deadline).map_err(|error| error.to_string())? {
        crate::host_link::Bounded::Done(output) => Ok(CommandAnswer::Done(output)),
        crate::host_link::Bounded::Expired => Ok(CommandAnswer::TimedOut),
        crate::host_link::Bounded::TooLarge => {
            Err("selection command wrote more than 4 MiB".into())
        }
    }
}

fn push_changed_files(
    root: &Path,
    target: &str,
    tip: &str,
    deadline: Instant,
) -> Result<CommandAnswer, String> {
    let script = root.join("genesis/orchestrator/push-changed-files.sh");
    let script = script
        .to_str()
        .ok_or_else(|| format!("{} is not UTF-8", script.display()))?;
    let mut command = crate::process::build_command("bash", &[script, target, tip], root, &[]);
    command.envs(crate::git::trust_env(root));
    command_until(&mut command, deadline)
}

fn gate_selection(
    root: &Path,
    changed: Vec<u8>,
    deadline: Instant,
) -> Result<CommandAnswer, String> {
    let root = root.to_owned();
    let stall_input = std::env::var_os("EPR_UNDER_TEST").is_some()
        && std::env::var_os("EPR_WOULD_STALL_INPUT").is_some();
    let (send, receive) = mpsc::channel();
    std::thread::Builder::new()
        .name("device-would-selection".into())
        .spawn(move || {
            let answer = (|| {
                if stall_input {
                    std::thread::sleep(WOULD_BUDGET);
                }
                let runner = root.join("genesis/orchestrator/gate-runner.mjs");
                let mut input = tempfile::NamedTempFile::new().map_err(|error| {
                    format!("forecast error creating changed-file input: {error}")
                })?;
                input
                    .write_all(&changed)
                    .and_then(|()| input.flush())
                    .map_err(|error| {
                        format!("forecast error writing changed-file input: {error}")
                    })?;
                let stdin = input.reopen().map_err(|error| {
                    format!("forecast error reopening changed-file input: {error}")
                })?;
                let runner_text = runner
                    .to_str()
                    .ok_or_else(|| format!("forecast error: {} is not UTF-8", runner.display()))?;
                let mut command = crate::process::build_command(
                    "node",
                    &[
                        runner_text,
                        "--changed-file-list",
                        "--names",
                        "--order",
                        "cost",
                    ],
                    &root,
                    &[],
                );
                command
                    .env("GATE_ROOT", &root)
                    .stdin(Stdio::from(stdin))
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                command_until(&mut command, deadline)
            })();
            let _ = send.send(answer);
        })
        .map_err(|error| format!("forecast error starting gate selection worker: {error}"))?;

    match receive.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(answer) => answer,
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(CommandAnswer::TimedOut),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("forecast error: gate selection worker stopped without an answer".into())
        }
    }
}

/// Forecast the gate names selected by the same changed-file range shape the pre-push hook uses:
/// destination-to-tip, with the merge-base form against origin/dev only when that cannot be read.
pub(super) fn would_run(
    root: &Path,
    tip: &str,
    target: &str,
    capacity: Option<&Capacity>,
) -> Vec<String> {
    let deadline = Instant::now() + would_budget();
    let mut lines = vec![
        "FORECAST".into(),
        "selection only; reuse, attestation and hand-over can change what runs".into(),
        capacity.map_or_else(
            || "capacity: unavailable".into(),
            |capacity| format!("capacity: {}", capacity.line()),
        ),
    ];
    if capacity.is_none() {
        lines.push("forecast error: device capacity could not be read".into());
        return lines;
    }
    let changed = match push_changed_files(root, target, tip, deadline) {
        Err(error) => {
            lines.push(format!(
                "forecast error while reading changed files: {error}"
            ));
            return lines;
        }
        Ok(CommandAnswer::TimedOut) => {
            lines.push("timed out after 10s while reading the push changed-file range".into());
            return lines;
        }
        Ok(CommandAnswer::Done(output))
            if output.status.success() || output.status.code() == Some(3) =>
        {
            output.stdout
        }
        Ok(CommandAnswer::Done(output)) => {
            let detail = String::from_utf8_lossy(&output.stderr);
            let detail = detail.trim();
            lines.push(if detail.is_empty() {
                format!(
                    "changed-file range unreadable (exit {})",
                    output
                        .status
                        .code()
                        .map_or_else(|| "signal".into(), |code| code.to_string())
                )
            } else {
                format!("changed-file range unreadable: {detail}")
            });
            return lines;
        }
    };

    match gate_selection(root, changed, deadline) {
        Err(error) => {
            lines.push(format!(
                "forecast error while running gate selection: {error}"
            ));
        }
        Ok(CommandAnswer::TimedOut) => {
            lines.push("timed out after 10s while running gate selection".into());
        }
        Ok(CommandAnswer::Done(output)) if output.status.success() => {
            lines.push("selected gates:".into());
            let gates: Vec<_> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|gate| format!("  {gate}"))
                .collect();
            if gates.is_empty() {
                lines.push("  (none)".into());
            } else {
                lines.extend(gates);
            }
        }
        Ok(CommandAnswer::Done(output)) => {
            let detail = String::from_utf8_lossy(&output.stderr);
            let detail = detail.trim();
            lines.push(if detail.is_empty() {
                format!(
                    "gate selection failed (exit {})",
                    output
                        .status
                        .code()
                        .map_or_else(|| "signal".into(), |code| code.to_string())
                )
            } else {
                format!("gate selection failed: {detail}")
            });
        }
    }
    lines
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
        assert!(capacity.cargo_pool_present);
        assert!(capacity.cargo_pool_free_bytes.is_some());
        assert_eq!(capacity.cores, Some(2));
        assert_eq!(capacity.memory_bytes, Some(2 * 1024 * 1024));
        assert_eq!(capacity.container_runtime.as_deref(), Some("docker"));
    }

    #[test]
    fn build_role_record_has_its_own_schema_and_atomic_file() {
        let dir = tempfile::tempdir().unwrap();
        let record = BuildRoleRecord {
            version: 1,
            role: BuildRole::Builder,
            until: Some("2099-01-01T00:00:00Z".into()),
        };
        let path = write_build_role(dir.path(), &record).unwrap();
        assert_eq!(path, dir.path().join(BUILD_ROLE_FILE));
        assert_eq!(read_build_role(dir.path()).unwrap(), Some(record.clone()));
        let schema: serde_json::Value = serde_json::from_str(BUILD_ROLE_SCHEMA).unwrap();
        let value = serde_json::to_value(record).unwrap();
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "the rename leaves no temporary sibling"
        );
    }

    #[test]
    fn status_capacity_deadline_names_the_operation_that_timed_out() {
        let capacity = observe_bounded_for(
            None,
            &ToolchainFact {
                pin: Some("1.100.0".into()),
                installed: Some(true),
                location: None,
                why_unknown: None,
            },
            Duration::from_millis(5),
            Some("PATH checks"),
        );
        assert_eq!(capacity.timed_out.as_deref(), Some("PATH checks"));
        assert!(capacity
            .line()
            .contains("timed out after 2s while reading PATH checks"));
    }

    #[test]
    fn expired_builder_is_evaluated_when_status_reads_it() {
        let capacity = Capacity {
            cargo_pool: None,
            cargo_pool_present: false,
            cargo_pool_free_bytes: None,
            toolchain_pin: None,
            toolchain_installed: None,
            cores: None,
            memory_bytes: None,
            container_runtime: None,
            build_role: Some(BuildRole::Builder),
            role_until: Some("2000-01-01T00:00:00Z".into()),
            does_not_build: false,
            timed_out: None,
            role_error: None,
        };
        assert!(capacity
            .line()
            .contains("role builder, expired 2000-01-01T00:00:00Z: declare again"));
    }

    #[test]
    fn neither_is_the_only_toolchain_exception() {
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
            timed_out: None,
            role_error: None,
        };
        assert!(capacity.line().contains("does not build"));
    }
}
