//! `epr device supervise` — ark runs this device's declared manifest.
//!
//! The device-head → ark node: the private manifest chain (B) says what runs; this verb
//! writes the berth ark needs and starts one `ark run` per data root, detached. When the
//! chain's head moves, a later `supervise` stops ark gracefully and starts it again under
//! the new CID, carrying the incarnation. That is reload by restart; there is no admin
//! socket.
//!
//! Every record here is device-local. The rendered manifest, the berth, the start record
//! and ark's own passport and witnesses under `<data_root>/ark/` are Ephemeral (C)
//! projections of the private chain: no DHT head, no route, no peer fact. Deleting the
//! data root loses only the death history; the next `supervise` rebuilds the rest.
//!
//! Declaration is not enforcement: ark reports `EffectiveTier::None`. Supervision here
//! means restart and witnessed deaths, never a bound. The workspace RAM guard and the
//! berth lease remain the only enforcers.
use ark_core::{declaration, ArtifactRef, Berth, Passport, RuntimeManifest};
use elohim_device_runtime::{bin_store::Store, manifest_store};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The berth resource and its daemon class (`genesis/agentic/pool-policy.json`).
const LEASE: &str = "device-ark";
/// Where the device ark keeps its data, under the main checkout.
const DATA_REL: &str = ".eprfs/status/ark/device";
/// The pool slot `hc-mesh.sh` prefers for `ark`.
const ARK_POOL_SLOT: &str = "/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark";
/// Margin past the longest declared shutdown grace before a stop is called failed: ark
/// signals each child, waits its grace, then reaps and writes its passport.
const STOP_MARGIN: Duration = Duration::from_secs(10);
/// How long a fresh ark is watched for an immediate refusal (bad berth, bad manifest).
const START_WATCH: Duration = Duration::from_millis(1500);
/// Two clocks (the kernel's boot-relative start tick and ark's epoch stamp) disagree by
/// rounding; a pid whose start is further than this from the passport's is someone else.
const START_TOLERANCE_MS: u64 = 2_000;

type Outcome<T> = Result<T, String>;

/// What `supervise` recorded about the ark it started.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Started {
    pid: u32,
    /// `/proc/<pid>/stat` field 22, so a reused pid is never mistaken for this ark.
    start_ticks: u64,
    manifest_cid: String,
    ark: PathBuf,
    started_at_epoch_ms: u64,
}

/// One supervised child as the passport reports it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChildStatus {
    name: String,
    pid: Option<u32>,
    ready: bool,
    deaths_in_window: u32,
}

/// The device ark as read from its data root; constructing it starts or stops nothing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    data_root: PathBuf,
    running: bool,
    pid: Option<u32>,
    /// The manifest the running ark was started with.
    manifest_cid: Option<String>,
    /// The chain's head now.
    head_cid: String,
    /// Whether the head declares any process at all.
    declares_processes: bool,
    current: bool,
    incarnation: Option<u64>,
    witnesses: usize,
    processes: Vec<ChildStatus>,
}

struct Options {
    status: bool,
    stop: bool,
    json: bool,
    root: Option<PathBuf>,
    data_root: Option<PathBuf>,
}

pub(super) fn run(args: &[String]) -> Outcome<ExitCode> {
    let mut opts = Options {
        status: false,
        stop: false,
        json: false,
        root: None,
        data_root: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--status" => opts.status = true,
            "--stop" => opts.stop = true,
            "--json" => opts.json = true,
            "--root" | "--data-root" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("{arg} needs a path"))?
                    .into();
                if arg == "--root" {
                    opts.root = Some(value);
                } else {
                    opts.data_root = Some(value);
                }
            }
            "--help" | "-h" => {
                println!("{}", usage());
                return Ok(ExitCode::SUCCESS);
            }
            other => return Err(format!("unexpected argument {other}\n{}", usage())),
        }
    }
    if opts.status && opts.stop {
        return Err("--status reads and --stop acts; ask for one".into());
    }
    let checkout = here_checkout();
    let data_root = opts
        .data_root
        .clone()
        .unwrap_or_else(|| default_data_root(&checkout));
    let head = super::declaration::store(opts.root.clone(), None, None)?
        .read()
        .map_err(|e| e.to_string())?;
    if opts.status {
        let status = read_status(&data_root, &head)?;
        if opts.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&status).map_err(|e| e.to_string())?
            );
        } else {
            print_status(&status);
        }
        return Ok(ExitCode::SUCCESS);
    }
    let berth = lease_tool(&checkout);
    if opts.stop {
        match running(&data_root)? {
            Some(started) => {
                stop_ark(&data_root, &started)?;
                println!(
                    "device ark: stopped pid {} (manifest {})",
                    started.pid, started.manifest_cid
                );
            }
            None => println!("device ark: not running"),
        }
        release_lease(berth.as_deref());
        return Ok(ExitCode::SUCCESS);
    }
    supervise(&data_root, &head, berth.as_deref())
}

pub(super) fn usage() -> &'static str {
    "usage: epr device supervise [--status [--json] | --stop] [--root <device root>] \
     [--data-root <path>]\n  \
     (ark runs this device's declared manifest from <main checkout>/.eprfs/status/ark/device; \
     a moved head reloads it by restart)"
}

fn supervise(data_root: &Path, head: &RuntimeManifest, berth: Option<&Path>) -> Outcome<ExitCode> {
    let head_cid = head.cid().map_err(|e| e.to_string())?;
    let started = running(data_root)?;
    let berth_path = data_root.join("berth.json");
    let foreign: Vec<u32> = arks_for(&berth_path)
        .into_iter()
        .filter(|pid| started.as_ref().is_none_or(|s| s.pid != *pid))
        .collect();
    if let Some(pid) = foreign.first() {
        return Err(format!(
            "REFUSED: ark pid {pid} already runs {} and supervise did not start it — \
             stop it first; one data root, one ark",
            berth_path.display()
        ));
    }
    if let Some(started) = &started {
        if started.manifest_cid == head_cid {
            println!(
                "device ark: running pid {} at current manifest {head_cid}",
                started.pid
            );
            return Ok(ExitCode::SUCCESS);
        }
    }
    if head.processes.is_empty() {
        if let Some(started) = &started {
            stop_ark(data_root, started)?;
            release_lease(berth);
            println!(
                "device ark: the head {head_cid} declares no process; stopped pid {}",
                started.pid
            );
        } else {
            println!(
                "device ark: the head {head_cid} declares no process; nothing to supervise \
                 (declare one with `epr device run <tool>`)"
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    preflight(head)?;
    let ark = ark_bin()?;
    let leased = claim_lease(berth, &head_cid)?;
    let result = (|| {
        if let Some(started) = &started {
            println!(
                "device ark: reloading — stopping pid {} (manifest {}) for head {head_cid}",
                started.pid, started.manifest_cid
            );
            stop_ark(data_root, started)?;
        }
        start_ark(data_root, head, &head_cid, &ark)
    })();
    match result {
        Ok(started) => {
            println!(
                "device ark: started pid {} at manifest {head_cid} (log {})",
                started.pid,
                data_root.join("ark.log").display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            if leased {
                release_lease(berth);
            }
            Err(error)
        }
    }
}

/// Refuse before anything is stopped or started: every child must be a `bin:` channel the
/// device bin store can resolve to a verified current, because that is all this berth places.
fn preflight(head: &RuntimeManifest) -> Outcome<()> {
    let registry = manifest_store::registry(head).map_err(|e| e.to_string())?;
    for process in &head.processes {
        let tool = match &process.artifact {
            ArtifactRef::Channel { channel_id } => channel_id.strip_prefix("bin:"),
            ArtifactRef::Pinned { .. } => None,
        }
        .ok_or_else(|| {
            format!(
                "REFUSED: process {} is not a bin: channel; the device berth places only \
                 tools from the device bin store",
                process.name
            )
        })?;
        Store::from_environment(tool, &registry)
            .and_then(|store| store.current_artifact())
            .map_err(|e| format!("{e} — epr bin bootstrap --tool {tool}"))?;
    }
    Ok(())
}

fn start_ark(
    data_root: &Path,
    head: &RuntimeManifest,
    head_cid: &str,
    ark: &Path,
) -> Outcome<Started> {
    if let Some((name, pid)) = live_orphan(data_root) {
        return Err(format!(
            "REFUSED: {name} (pid {pid}) from an earlier ark still runs without it; a second \
             ark would start a second {name} — stop pid {pid}, then supervise again"
        ));
    }
    create_private_dir(data_root)?;
    let incarnation = carried_incarnation(data_root)?;
    let manifest_path = data_root.join("manifest.json");
    let rendered = declaration::render_manifest(head).map_err(|e| e.to_string())?;
    // The bytes ark reads must address to the head; never start under a CID they do not carry.
    let reread = RuntimeManifest::from_json(&rendered)
        .and_then(|m| m.cid())
        .map_err(|e| e.to_string())?;
    if reread != head_cid {
        return Err(format!(
            "REFUSED: the rendered manifest addresses {reread}, not the head {head_cid}"
        ));
    }
    write_atomic(&manifest_path, rendered.as_bytes())?;
    let berth = Berth {
        manifest: head_cid.to_owned(),
        data_root: data_root.to_owned(),
        incarnation,
        ..Berth::default()
    };
    let berth_path = data_root.join("berth.json");
    write_atomic(
        &berth_path,
        &serde_json::to_vec_pretty(&berth).map_err(|e| e.to_string())?,
    )?;
    let log_path = data_root.join("ark.log");
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("{}: {e}", log_path.display()))?;
    let mut command = Command::new(ark);
    command
        .arg("run")
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--berth")
        .arg(&berth_path)
        .current_dir(data_root)
        // The registry ark resolves `bin:` channels against is the declaration it runs, never
        // a committed base found by walking up from its working directory.
        .env("EPR_BIN_TOOLS", &manifest_path)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log);
    // SAFETY: `setsid` is async-signal-safe and touches no memory of the parent; it only
    // detaches the child from this terminal's session so the ark outlives the shell.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", ark.display()))?;
    let pid = child.id();
    let deadline = Instant::now() + START_WATCH;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!(
                "REFUSED: ark exited at once ({status}); its last words:\n{}",
                tail(&log_path, 8)
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
    let start_ticks = proc_stat(pid)
        .map(|(_, ticks)| ticks)
        .ok_or_else(|| format!("ark pid {pid} vanished before it could be recorded"))?;
    let started = Started {
        pid,
        start_ticks,
        manifest_cid: head_cid.to_owned(),
        ark: ark.to_owned(),
        started_at_epoch_ms: epoch_ms(),
    };
    write_atomic(
        &data_root.join("supervise.json"),
        &serde_json::to_vec_pretty(&started).map_err(|e| e.to_string())?,
    )?;
    Ok(started)
}

/// SIGTERM, then wait out the longest declared grace. Never escalates: an ark that will not
/// stop is reported, because killing it would orphan the children it is still stopping.
fn stop_ark(data_root: &Path, started: &Started) -> Outcome<()> {
    let grace = fs::read_to_string(data_root.join("manifest.json"))
        .ok()
        .and_then(|text| RuntimeManifest::from_json(&text).ok())
        .map(|m| {
            m.processes
                .iter()
                .map(|p| p.policy.shutdown.grace_ms)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(20_000);
    let pid = libc::pid_t::try_from(started.pid).map_err(|e| e.to_string())?;
    // SAFETY: `kill` takes plain integers; the pid was just confirmed to be this ark by its
    // recorded start tick, so the signal cannot reach a recycled pid.
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("signalling ark pid {}: {error}", started.pid));
        }
    }
    let deadline = Instant::now() + Duration::from_millis(grace) + STOP_MARGIN;
    while is_same_process(started.pid, started.start_ticks) {
        if Instant::now() >= deadline {
            return Err(format!(
                "REFUSED: ark pid {} did not stop within {}s; not starting a second ark \
                 beside it",
                started.pid,
                (Duration::from_millis(grace) + STOP_MARGIN).as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = fs::remove_file(data_root.join("supervise.json"));
    Ok(())
}

/// The ark `supervise` started for this data root, when it is still that process.
fn running(data_root: &Path) -> Outcome<Option<Started>> {
    let path = data_root.join("supervise.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let started: Started =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(is_same_process(started.pid, started.start_ticks).then_some(started))
}

pub(crate) fn read_status(data_root: &Path, head: &RuntimeManifest) -> Outcome<Status> {
    let head_cid = head.cid().map_err(|e| e.to_string())?;
    let started = running(data_root)?;
    let passport = load_passport(data_root);
    let processes = match (&started, &passport) {
        (Some(_), Some(passport)) => passport
            .processes
            .iter()
            .map(|p| ChildStatus {
                name: p.name.clone(),
                pid: p.pid,
                ready: p.ready,
                deaths_in_window: p.deaths_in_window,
            })
            .collect(),
        _ => Vec::new(),
    };
    let witnesses = fs::read_dir(data_root.join("ark/witnesses"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .count()
        })
        .unwrap_or(0);
    Ok(Status {
        data_root: data_root.to_owned(),
        running: started.is_some(),
        pid: started.as_ref().map(|s| s.pid),
        current: started.as_ref().is_some_and(|s| s.manifest_cid == head_cid),
        manifest_cid: started.map(|s| s.manifest_cid),
        declares_processes: !head.processes.is_empty(),
        head_cid,
        incarnation: passport.map(|p| p.incarnation),
        witnesses,
        processes,
    })
}

fn print_status(status: &Status) {
    if !status.running {
        println!(
            "device ark: stopped (head {}{})",
            status.head_cid,
            if status.declares_processes {
                ""
            } else {
                ", which declares no process"
            }
        );
        return;
    }
    println!(
        "device ark: running pid {} incarnation {} — manifest {} {} (head {})",
        status.pid.unwrap_or_default(),
        status
            .incarnation
            .map_or_else(|| "-".into(), |i| i.to_string()),
        status.manifest_cid.as_deref().unwrap_or("-"),
        if status.current { "current" } else { "STALE" },
        status.head_cid
    );
    for child in &status.processes {
        println!(
            "  {}: {} pid {} deaths-in-window {}",
            child.name,
            if child.ready { "ready" } else { "not ready" },
            child.pid.map_or_else(|| "-".into(), |p| p.to_string()),
            child.deaths_in_window
        );
    }
    println!(
        "  witnesses: {} (ark witness ls --berth {})",
        status.witnesses,
        status.data_root.join("berth.json").display()
    );
}

/// The arrival line: running at the head with every child ready, or a cure. Never refused —
/// the device works without a supervisor; it only loses restart and witnessed deaths.
pub(crate) fn check(checkout: &Path) -> (bool, String, Option<String>) {
    let cure = Some("epr device supervise".to_owned());
    let head = match super::declaration::store(None, None, None)
        .and_then(|s| s.read().map_err(|e| e.to_string()))
    {
        Ok(head) => head,
        Err(error) => return (false, format!("device manifest unreadable: {error}"), cure),
    };
    let status = match read_status(&default_data_root(checkout), &head) {
        Ok(status) => status,
        Err(error) => return (false, error, cure),
    };
    if !status.declares_processes && !status.running {
        return (
            true,
            "the head declares no process to supervise".into(),
            None,
        );
    }
    if !status.running {
        return (
            false,
            format!("no device ark runs head {}", status.head_cid),
            cure,
        );
    }
    if !status.current {
        return (
            false,
            format!(
                "ark runs {}; head is {}",
                status.manifest_cid.as_deref().unwrap_or("-"),
                status.head_cid
            ),
            cure,
        );
    }
    // A child declared without a ready line makes no readiness claim, so ark never marks it
    // ready; it counts as waiting only while it has no live process.
    let claims_readiness = |name: &str| {
        head.processes
            .iter()
            .any(|child| child.name == name && !child.readiness.is_empty())
    };
    let waiting: Vec<_> = status
        .processes
        .iter()
        .filter(|p| {
            if claims_readiness(&p.name) {
                !p.ready
            } else {
                p.pid.is_none()
            }
        })
        .map(|p| p.name.as_str())
        .collect();
    if !waiting.is_empty() {
        return (
            false,
            format!("not ready: {}", waiting.join(", ")),
            Some("epr device supervise --status".into()),
        );
    }
    (
        true,
        format!(
            "ark pid {} at head {}, {} ready",
            status.pid.unwrap_or_default(),
            status.head_cid,
            status.processes.len()
        ),
        None,
    )
}

fn load_passport(data_root: &Path) -> Option<Passport> {
    serde_json::from_slice(&fs::read(data_root.join("ark/passport.json")).ok()?).ok()
}

/// Incarnation is monotone across restarts. A passport that exists but cannot say its count
/// fails closed: substituting 0 would rewind what the spool remembers.
fn carried_incarnation(data_root: &Path) -> Outcome<u64> {
    let path = data_root.join("ark/passport.json");
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<Passport>(&bytes)
            .map(|p| p.incarnation)
            .map_err(|e| {
                format!(
                    "REFUSED: {} carries no readable incarnation ({e}); remove it only if this \
                     data root is deliberately re-seeded",
                    path.display()
                )
            }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// A child named in the passport that is still the process the passport started.
fn live_orphan(data_root: &Path) -> Option<(String, u32)> {
    let passport = load_passport(data_root)?;
    passport.processes.into_iter().find_map(|p| {
        let pid = p.pid?;
        let started = p.started_at_epoch_ms?;
        let actual = proc_started_epoch_ms(pid)?;
        (actual.abs_diff(started) <= START_TOLERANCE_MS).then_some((p.name, pid))
    })
}

/// Every live `ark run … --berth <berth>` on this host, whoever started it.
fn arks_for(berth: &Path) -> Vec<u32> {
    let want = berth.as_os_str().as_encoded_bytes();
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            let Ok(cmdline) = fs::read(format!("/proc/{pid}/cmdline")) else {
                return false;
            };
            let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
            args.iter().any(|a| *a == b"run")
                && args.windows(2).any(|w| w[0] == b"--berth" && w[1] == want)
                && proc_stat(*pid).is_some_and(|(state, _)| state != 'Z')
        })
        .collect()
}

/// `(state, start ticks)` from `/proc/<pid>/stat`, past the parenthesised command name.
fn proc_stat(pid: u32) -> Option<(char, u64)> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.get(stat.rfind(')')? + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    Some((
        fields.first()?.chars().next()?,
        fields.get(19)?.parse().ok()?,
    ))
}

fn is_same_process(pid: u32, start_ticks: u64) -> bool {
    proc_stat(pid).is_some_and(|(state, ticks)| state != 'Z' && ticks == start_ticks)
}

fn proc_started_epoch_ms(pid: u32) -> Option<u64> {
    let (state, ticks) = proc_stat(pid)?;
    if state == 'Z' {
        return None;
    }
    let boot_s: u64 = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime ")?.trim().parse().ok())?;
    // SAFETY: `sysconf` reads a constant of the running kernel and has no preconditions.
    let hz = u64::try_from(unsafe { libc::sysconf(libc::_SC_CLK_TCK) }).ok()?;
    (hz > 0).then(|| boot_s * 1000 + ticks * 1000 / hz)
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// `ARK_BIN`, else the pool slot `hc-mesh.sh` prefers, else `ark` on PATH.
fn ark_bin() -> Outcome<PathBuf> {
    let executable = |p: &Path| {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if let Some(explicit) = std::env::var_os("ARK_BIN").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        return if executable(&path) {
            Ok(path)
        } else {
            Err(format!(
                "REFUSED: ARK_BIN={} is not an executable file",
                path.display()
            ))
        };
    }
    let pool = PathBuf::from(ARK_POOL_SLOT);
    if executable(&pool) {
        return Ok(pool);
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join("ark"))
        .find(|p| executable(p))
        .ok_or_else(|| {
            format!(
                "REFUSED: no ark binary — build it with `(cd elohim && RUSTUP_TOOLCHAIN=1.96.1 \
                 CARGO_TARGET_DIR={} RUSTFLAGS= cargo build -p elohim-ark)` or set ARK_BIN",
                ARK_POOL_SLOT.trim_end_matches("/debug/ark")
            )
        })
}

/// The berth script that holds the daemon lease: `EPR_BERTH_BIN` (empty = none), else the
/// checkout's own. A device without one supervises unleased (local-first).
fn lease_tool(checkout: &Path) -> Option<PathBuf> {
    match std::env::var_os("EPR_BERTH_BIN") {
        Some(value) if value.is_empty() => None,
        Some(value) => Some(PathBuf::from(value)),
        None => Some(checkout.join("genesis/agentic/bin/berth")).filter(|p| p.is_file()),
    }
}

/// Exit 0 holds or renews the lease; exit 4 (no session) proceeds unleased as `just mesh
/// start` does; any other code is a refusal. Returns whether a lease is now held.
fn claim_lease(berth: Option<&Path>, head_cid: &str) -> Outcome<bool> {
    let Some(berth) = berth else {
        println!("device ark: no berth on this device; supervising unleased");
        return Ok(false);
    };
    let output = Command::new(berth)
        .args(["claim", LEASE, "--class", LEASE, "--note"])
        .arg(format!("epr device supervise {head_cid}"))
        .output()
        .map_err(|e| format!("{}: {e}", berth.display()))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(4) => {
            println!("device ark: no berth session resolvable; supervising unleased");
            Ok(false)
        }
        code => Err(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr).trim(),
            code.map_or_else(String::new, |c| format!(" (berth exit {c})"))
        )),
    }
}

fn release_lease(berth: Option<&Path>) {
    if let Some(berth) = berth {
        let _ = Command::new(berth)
            .args(["release", LEASE])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn here_checkout() -> PathBuf {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let checkout = crate::git::checkout(&here).unwrap_or(here);
    main_checkout(&checkout)
}

pub(crate) fn default_data_root(checkout: &Path) -> PathBuf {
    main_checkout(checkout).join(DATA_REL)
}

/// A linked worktree's main checkout (the same rule as `berth`'s `main_checkout`): one device,
/// one ark, whichever worktree asks. A submodule or separate git dir keeps its own root.
fn main_checkout(root: &Path) -> PathBuf {
    let resolve = || -> Option<PathBuf> {
        let marker = fs::read_to_string(root.join(".git")).ok()?;
        let git_dir = root.join(marker.trim().strip_prefix("gitdir: ")?);
        let common = fs::read_to_string(git_dir.join("commondir")).ok()?;
        let common = fs::canonicalize(git_dir.join(common.trim())).ok()?;
        (common.file_name()? == ".git" && common.is_dir())
            .then(|| common.parent().map(Path::to_owned))
            .flatten()
    };
    resolve().unwrap_or_else(|| root.to_owned())
}

fn create_private_dir(path: &Path) -> Outcome<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Outcome<()> {
    let dir = path.parent().ok_or("a record path has no directory")?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.persist(path)
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_reads_this_process_and_survives_a_spaced_name() {
        let (state, ticks) = proc_stat(std::process::id()).unwrap();
        assert_ne!(state, 'Z');
        assert!(is_same_process(std::process::id(), ticks));
        assert!(!is_same_process(std::process::id(), ticks + 1));
        assert!(proc_started_epoch_ms(std::process::id()).unwrap() <= epoch_ms());
    }

    #[test]
    fn a_worktree_resolves_to_its_main_checkout_and_a_plain_root_to_itself() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main");
        let wt_git = main.join(".git/worktrees/wt");
        fs::create_dir_all(&wt_git).unwrap();
        fs::write(wt_git.join("commondir"), "../..\n").unwrap();
        let wt = dir.path().join("wt");
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join(".git"), format!("gitdir: {}\n", wt_git.display())).unwrap();
        let main = fs::canonicalize(&main).unwrap();
        assert_eq!(main_checkout(&wt), main);
        assert_eq!(default_data_root(&wt), main.join(DATA_REL));
        assert_eq!(main_checkout(&main), main);
    }

    #[test]
    fn an_unreadable_passport_refuses_to_reset_the_incarnation() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(carried_incarnation(dir.path()).unwrap(), 0);
        fs::create_dir_all(dir.path().join("ark")).unwrap();
        fs::write(dir.path().join("ark/passport.json"), "{").unwrap();
        assert!(carried_incarnation(dir.path())
            .unwrap_err()
            .contains("incarnation"));
    }

    #[test]
    fn a_stale_record_is_not_a_running_ark() {
        let dir = tempfile::tempdir().unwrap();
        assert!(running(dir.path()).unwrap().is_none());
        let (_, ticks) = proc_stat(std::process::id()).unwrap();
        let record = |ticks| Started {
            pid: std::process::id(),
            start_ticks: ticks,
            manifest_cid: "bafy".into(),
            ark: "/bin/true".into(),
            started_at_epoch_ms: 0,
        };
        write_atomic(
            &dir.path().join("supervise.json"),
            &serde_json::to_vec(&record(ticks + 7)).unwrap(),
        )
        .unwrap();
        assert!(running(dir.path()).unwrap().is_none(), "a recycled pid");
        write_atomic(
            &dir.path().join("supervise.json"),
            &serde_json::to_vec(&record(ticks)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            running(dir.path()).unwrap().unwrap().pid,
            std::process::id()
        );
    }
}
