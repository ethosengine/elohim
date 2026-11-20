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
use elohim_device_runtime::{
    bin_store::{file_cid, Store},
    manifest_store,
};
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
/// The slot under the cargo pool that `hc-mesh.sh` prefers for `ark`.
const ARK_POOL_REL: &str = "family/dev/elohim/dev/debug/ark";

/// `ark`'s pool slot on this device, when the device layout names a cargo pool.
fn ark_pool_slot() -> Option<PathBuf> {
    elohim_device_runtime::layout::resolve()
        .ok()?
        .cargo_pool()
        .map(|pool| pool.join(ARK_POOL_REL))
}
/// Margin past the longest declared shutdown grace before a stop is called failed: ark
/// signals each child, waits its grace, then reaps and writes its passport.
const STOP_MARGIN: Duration = Duration::from_secs(10);
/// How long a fresh ark is watched for an immediate refusal (bad berth, bad manifest).
const START_WATCH: Duration = Duration::from_millis(1500);
/// Two clocks (the kernel's boot-relative start tick and ark's epoch stamp) disagree by
/// rounding; a pid whose start is further than this from the passport's is someone else.
const START_TOLERANCE_MS: u64 = 2_000;
/// How long `--handover` waits for an interrupted `epr peer serve` to release its serve lock.
const HANDOVER_WAIT: Duration = Duration::from_secs(20);

type Outcome<T> = Result<T, String>;

mod receipt;

/// Whether supervise speaks on stderr: a `bin` verb's stdout is its JSON, which a restart's lines
/// must not precede.
static SAY_ON_STDERR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Route this process's supervise lines to stderr (the `bin` verbs do).
pub(crate) fn say_on_stderr() {
    SAY_ON_STDERR.store(true, std::sync::atomic::Ordering::Relaxed);
}

macro_rules! say {
    ($($arg:tt)*) => {
        if SAY_ON_STDERR.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*)
        } else {
            println!($($arg)*)
        }
    };
}

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
    handover: bool,
}

pub(super) fn run(args: &[String]) -> Outcome<ExitCode> {
    let mut opts = Options {
        status: false,
        stop: false,
        json: false,
        root: None,
        data_root: None,
        handover: false,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--status" => opts.status = true,
            "--stop" => opts.stop = true,
            "--json" => opts.json = true,
            "--handover" => opts.handover = true,
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
    if opts.handover && (opts.status || opts.stop) {
        return Err(
            "--handover belongs to a supervise that starts ark, not --status or --stop".into(),
        );
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
                say!(
                    "device ark: stopped pid {} (manifest {})",
                    started.pid,
                    started.manifest_cid
                );
            }
            None => say!("device ark: not running"),
        }
        release_lease(berth.as_deref());
        return Ok(ExitCode::SUCCESS);
    }
    supervise(&data_root, &head, berth.as_deref(), opts.handover)
}

pub(super) fn usage() -> &'static str {
    "usage: epr device supervise [--handover] [--status [--json] | --stop] \
     [--root <device root>] [--data-root <path>]\n  \
     (ark runs this device's declared manifest from <main checkout>/.eprfs/status/ark/device; \
     a moved head or a child running other bytes than its channel's current reloads it by \
     restart; --handover interrupts an `epr peer serve` this ark did not start so the \
     declared one can take its serve lock)"
}

fn supervise(
    data_root: &Path,
    head: &RuntimeManifest,
    berth: Option<&Path>,
    handover: bool,
) -> Outcome<ExitCode> {
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
    refuse_declared_launcher_env(head)?;
    // Before anything is judged, claimed, stopped or signalled: a serve lock held outside this
    // ark refuses here unless the person asked for the handover. A current ark is no answer to
    // it, since its declared peer child only waits in backoff behind the foreign holder.
    let holder = peer_serve_preflight(head, data_root, started.as_ref(), handover)?;
    // At the head, the manifest alone cannot say a child is current: ark resolves `bin:` at
    // each spawn, so a child started before a landing still runs the bytes it was started on.
    let mut stale_binary = false;
    if let Some(started) = &started {
        if started.manifest_cid == head_cid && holder.is_some() {
            say!(
                "device ark: running pid {} at current manifest {head_cid}, but an epr peer \
                 serve it did not start holds the serve lock; handing over, then restarting",
                started.pid
            );
        } else if started.manifest_cid == head_cid {
            match child_identity(data_root, head, started)? {
                Identity::Current => {
                    say!(
                        "device ark: running pid {} at current manifest {head_cid}",
                        started.pid
                    );
                    return Ok(ExitCode::SUCCESS);
                }
                Identity::NoLiveChild => {
                    say!(
                        "device ark: running pid {} at current manifest {head_cid}; no child is \
                         live yet (starting or in restart backoff), so none is judged stale and \
                         nothing restarts",
                        started.pid
                    );
                    return Ok(ExitCode::SUCCESS);
                }
                Identity::Stale {
                    child,
                    running,
                    current,
                } => {
                    say!("device ark: {child} runs {running}, current is {current}; restarting");
                    stale_binary = true;
                }
            }
        }
    }
    if head.processes.is_empty() {
        if let Some(started) = &started {
            stop_ark(data_root, started)?;
            release_lease(berth);
            say!(
                "device ark: the head {head_cid} declares no process; stopped pid {}",
                started.pid
            );
        } else {
            say!(
                "device ark: the head {head_cid} declares no process; nothing to supervise \
                 (declare one with `epr device run <tool>`)"
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    preflight(head)?;
    let (ark, ark_source) = ark_bin(data_root)?;
    let leased = claim_lease(berth, &head_cid)?;
    let result = (|| {
        // The holder goes first: a handover that times out leaves this device's own ark as
        // it was, never stopped for a daemon that would not stop.
        if let Some(holder) = &holder {
            hand_over(holder)?;
        }
        let interrupted = |error: String| match &holder {
            Some(holder) => format!(
                "{error} — the unowned epr peer serve (pid {}) was already interrupted, so \
                 nothing serves this device now; rerun `epr device supervise`",
                holder.pid
            ),
            None => error,
        };
        if let Some(started) = &started {
            if stale_binary {
                say!(
                    "device ark: stopping pid {} (manifest {head_cid}) to start current bytes",
                    started.pid
                );
            } else {
                say!(
                    "device ark: reloading — stopping pid {} (manifest {}) for head {head_cid}",
                    started.pid,
                    started.manifest_cid
                );
            }
            stop_ark(data_root, started).map_err(interrupted)?;
        }
        start_ark(data_root, head, &head_cid, &ark).map_err(interrupted)
    })();
    match result {
        Ok(started) => {
            say!(
                "device ark: started pid {} at manifest {head_cid} (log {}; ark {}, {ark_source})",
                started.pid,
                data_root.join("ark.log").display(),
                ark.display()
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

/// A manifest that already carries a launcher candidate key on a `bin:` process is refused
/// before anything is judged, stopped or started — even when an ark already runs it.
fn refuse_declared_launcher_env(head: &RuntimeManifest) -> Outcome<()> {
    let mut registry = None;
    for process in &head.processes {
        let Some(tool) = channel_tool(&process.artifact) else {
            continue;
        };
        if process.env.is_empty() {
            continue;
        }
        if registry.is_none() {
            registry = Some(manifest_store::registry(head).map_err(|e| e.to_string())?);
        }
        if let Some(registry) = &registry {
            super::declaration::refuse_launcher_env(
                &process.name,
                tool,
                process.env.keys(),
                registry,
                true,
            )?;
        }
    }
    Ok(())
}

/// Refuse before anything is stopped or started: every child must be a `bin:` channel the
/// device bin store can resolve to a verified current, because that is all this berth places.
fn preflight(head: &RuntimeManifest) -> Outcome<()> {
    let registry = manifest_store::registry(head).map_err(|e| e.to_string())?;
    for process in &head.processes {
        let tool = channel_tool(&process.artifact).ok_or_else(|| {
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

/// Whether the children a running ark holds run their channels' current bytes.
enum Identity {
    Current,
    /// The ark is up but no declared child has a live process (starting or backing off).
    NoLiveChild,
    Stale {
        child: String,
        running: String,
        current: String,
    },
}

/// Each live child of `started` (by the passport's pid, confirmed as this ark's child in
/// `/proc`) is judged by the bytes the kernel executes, `/proc/<pid>/exe`, against the
/// verified current of its declared `bin:` channel: the same byte CID the store files by.
fn child_identity(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
) -> Outcome<Identity> {
    child_identity_on(data_root, head, started, None)
}

/// [`child_identity`] for the children on `tool`'s channel alone, when `tool` is given.
fn child_identity_on(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
    only: Option<&str>,
) -> Outcome<Identity> {
    let Some(passport) = load_passport(data_root) else {
        return Ok(Identity::NoLiveChild);
    };
    let registry = manifest_store::registry(head).map_err(|e| e.to_string())?;
    let mut live = false;
    for child in &passport.processes {
        let Some(pid) = child.pid else { continue };
        if proc_ppid(pid) != Some(started.pid) {
            continue;
        }
        let Some(tool) = head
            .processes
            .iter()
            .find(|spec| spec.name == child.name)
            .and_then(|spec| channel_tool(&spec.artifact))
            .filter(|tool| only.is_none_or(|only| only == *tool))
        else {
            continue;
        };
        let exe = PathBuf::from(format!("/proc/{pid}/exe"));
        // A child that died between the passport and this read is not live, not stale.
        let Ok(running_bytes) = file_cid(&exe) else {
            continue;
        };
        live = true;
        let store = Store::from_environment(tool, &registry).map_err(|e| e.to_string())?;
        let (current, path) = store.current_artifact().map_err(|e| e.to_string())?;
        if running_bytes != file_cid(&path).map_err(|e| e.to_string())? {
            let running = store.executable_cid(&exe).unwrap_or(running_bytes);
            return Ok(Identity::Stale {
                child: child.name.clone(),
                running,
                current,
            });
        }
    }
    Ok(if live {
        Identity::Current
    } else {
        Identity::NoLiveChild
    })
}

fn channel_tool(artifact: &ArtifactRef) -> Option<&str> {
    match artifact {
        ArtifactRef::Channel { channel_id } => channel_id.strip_prefix("bin:"),
        ArtifactRef::Pinned { .. } => None,
    }
}

/// The declared process whose argv after `{artifact}` begins `peer serve`, if any.
pub(crate) fn declared_peer_serve(head: &RuntimeManifest) -> Option<&ark_core::ChildSpec> {
    head.processes
        .iter()
        .find(|p| p.argv.get(1..3).is_some_and(|a| a == ["peer", "serve"]))
}

/// The `--root` an `epr peer serve` argv tail names, as `--root <path>` or `--root=<path>`.
/// `None` when it names none: serve's own default (its working directory) is never assumed,
/// so a daemon matched by working directory alone is never taken for the declared one.
fn serve_root(tail: &[String]) -> Option<&str> {
    tail.iter().enumerate().find_map(|(i, arg)| {
        if arg == "--root" {
            tail.get(i + 1).map(String::as_str)
        } else {
            arg.strip_prefix("--root=")
        }
    })
}

/// An `epr peer serve` holding the serve lock that this ark did not start, confirmed to be
/// the one process `--handover` may interrupt.
struct Holder {
    pid: u32,
    start_ticks: u64,
    lock: PathBuf,
    /// How long the interrupted daemon may take to release the lock: the declared peer
    /// process's shutdown grace plus a margin, as `stop_ark` allows its own children.
    wait: Duration,
    /// The running ark, whose own peer child may take the lock the moment it is released.
    ark: Option<u32>,
}

/// The serve-lock handover preflight. Nothing here signals: it returns the holder only when
/// `--handover` was asked and every check passed.
fn peer_serve_preflight(
    head: &RuntimeManifest,
    data_root: &Path,
    started: Option<&Started>,
    handover: bool,
) -> Outcome<Option<Holder>> {
    let Some(declared) = declared_peer_serve(head) else {
        return Ok(None);
    };
    let (lock, held) = super::arrival::serve_lock_probe()
        .map_err(|e| format!("REFUSED peer-serve-unprobed: {e}"))?;
    if !held {
        return Ok(None);
    }
    let holders = lock_holders(&lock);
    if let Some(started) = started.filter(|s| is_same_process(s.pid, s.start_ticks)) {
        if holders.iter().any(|pid| descends_from(*pid, started.pid)) {
            // This ark's own child: stopping the ark releases it.
            return Ok(None);
        }
    }
    let holder = holders
        .iter()
        .copied()
        .find(|pid| peer_serve_tail(*pid).is_some())
        .or_else(|| holders.first().copied());
    let start_ticks = holder.and_then(|pid| proc_stat(pid).map(|(_, ticks)| ticks));
    let Some(pid) = holder else {
        let detail = format!(
            "a process this user cannot see holds {}; ark would start a second daemon beside \
             it, so nothing starts and nothing is signalled — stop the daemon holding it with \
             SIGINT, then rerun `epr device supervise`",
            lock.display()
        );
        return Err(if handover {
            format!("REFUSED peer-serve-handover-refused: holds serve.lock: {detail}")
        } else {
            format!("REFUSED peer-serve-unowned: {detail}")
        });
    };
    let leased = daemon_lease_names(pid);
    let exe = holder_exe_cid(pid);
    if !handover {
        return Err(format!(
            "REFUSED peer-serve-unowned: pid {pid} (exe {exe}; daemon lease {}) holds {} and \
             this device ark did not start it; ark would start {} beside it, so nothing \
             starts and nothing is signalled — stop it with SIGINT (kill -INT {pid}), then \
             rerun `epr device supervise`, or rerun with --handover",
            if leased {
                "names it"
            } else {
                "names no such pid"
            },
            lock.display(),
            declared.name
        ));
    }
    let refused = |check: &str| {
        format!(
            "REFUSED peer-serve-handover-refused: pid {pid} (exe {exe}) holds {} but fails the \
             {check} check; it is left running — stop it yourself with SIGINT, then rerun \
             `epr device supervise`",
            lock.display()
        )
    };
    if proc_uid(pid).is_none() || proc_uid(pid) != proc_uid(std::process::id()) {
        return Err(refused("same-user"));
    }
    let start_ticks = start_ticks.ok_or_else(|| refused("still-running"))?;
    let Some((cwd, tail)) = peer_serve_tail(pid) else {
        return Err(refused("`epr peer serve` command line"));
    };
    let Some(declared_root) = serve_root(&declared.argv[3..]) else {
        return Err(refused(&format!(
            "same --root (the declared {} names no --root to compare)",
            declared.name
        )));
    };
    let Some(holder_root) = serve_root(&tail) else {
        return Err(refused("same --root (it names no --root)"));
    };
    let template = Berth {
        data_root: data_root.to_owned(),
        ..Berth::default()
    };
    let declared_root = template
        .resolve_template(&declared.name, Path::new(""), declared_root)
        .map_err(|e| e.to_string())?;
    let canonical = |base: &Path, root: &str| {
        let path = base.join(root);
        fs::canonicalize(&path).unwrap_or(path)
    };
    if canonical(&cwd, holder_root) != canonical(data_root, &declared_root) {
        return Err(refused("same --root"));
    }
    if !lock_holders(&lock).contains(&pid) {
        return Err(refused("holds-serve.lock"));
    }
    if !leased {
        return Err(refused("daemon-lease"));
    }
    // The checks above read /proc over time; the pid they judged must still be that process.
    if !is_same_process(pid, start_ticks) {
        return Err(refused("still-running"));
    }
    let grace = declared.policy.shutdown.grace_ms;
    Ok(Some(Holder {
        pid,
        start_ticks,
        lock,
        ark: started
            .filter(|s| is_same_process(s.pid, s.start_ticks))
            .map(|s| s.pid),
        wait: if grace == 0 {
            HANDOVER_WAIT
        } else {
            Duration::from_millis(grace) + STOP_MARGIN
        },
    }))
}

/// SIGINT to the one confirmed holder, then wait for the kernel to release its lock. Never
/// escalates: a daemon that keeps the lock past its wait is reported, still draining.
fn hand_over(holder: &Holder) -> Outcome<()> {
    if is_same_process(holder.pid, holder.start_ticks) {
        let pid = libc::pid_t::try_from(holder.pid).map_err(|e| e.to_string())?;
        say!(
            "device ark: handing over — SIGINT to epr peer serve pid {}",
            holder.pid
        );
        // SAFETY: `kill` takes plain integers; the pid was confirmed against its start tick
        // just above, so the signal cannot reach a recycled pid.
        if unsafe { libc::kill(pid, libc::SIGINT) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(format!("signalling pid {}: {error}", holder.pid));
            }
        }
    }
    let deadline = Instant::now() + holder.wait;
    loop {
        match super::arrival::serve_lock_probe() {
            Ok((_, false)) => return Ok(()),
            // Released, and already taken by the declared peer this ark was holding back.
            Ok((_, true))
                if !is_same_process(holder.pid, holder.start_ticks)
                    && holder.ark.is_some_and(|ark| {
                        let holders = lock_holders(&holder.lock);
                        !holders.is_empty() && holders.iter().all(|pid| descends_from(*pid, ark))
                    }) =>
            {
                return Ok(())
            }
            Ok((_, true)) => {}
            Err(error) => return Err(format!("REFUSED peer-serve-unprobed: {error}")),
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "REFUSED peer-serve-handover-timeout: {} is still held {}s after SIGINT to pid \
                 {}; it was interrupted and is probably still draining, and ark is not started \
                 beside it — wait for pid {} to exit (never kill it), then rerun \
                 `epr device supervise`",
                holder.lock.display(),
                holder.wait.as_secs(),
                holder.pid,
                holder.pid
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Every process of this user whose open file description holds a flock on `lock`, read from
/// `/proc/<pid>/fdinfo` (the kernel lists a lock there only for the description that holds it).
fn lock_holders(lock: &Path) -> Vec<u32> {
    let lock = fs::canonicalize(lock).unwrap_or_else(|_| lock.to_owned());
    let me = std::process::id();
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid != me)
        .filter(|pid| {
            let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
                return false;
            };
            fds.filter_map(Result::ok).any(|fd| {
                fs::read_link(fd.path()).is_ok_and(|target| target == lock)
                    && fs::read_to_string(format!(
                        "/proc/{pid}/fdinfo/{}",
                        fd.file_name().to_string_lossy()
                    ))
                    .is_ok_and(|info| {
                        info.lines()
                            .any(|l| l.starts_with("lock:") && l.contains("FLOCK"))
                    })
            })
        })
        .collect()
}

/// `(cwd, argv after "serve")` when `/proc/<pid>/cmdline` is `epr peer serve …`.
fn peer_serve_tail(pid: u32) -> Option<(PathBuf, Vec<String>)> {
    let cmdline = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let args: Vec<String> = cmdline
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let named = Path::new(args.first()?)
        .file_name()
        .is_some_and(|name| name == "epr" || name == "bin-launch");
    let stored = || {
        let exe = fs::canonicalize(format!("/proc/{pid}/exe")).ok()?;
        let store = crate::bin::daemon_store().ok()?;
        let root = fs::canonicalize(store.root()).ok()?;
        exe.starts_with(root).then_some(())
    };
    (named || stored().is_some()).then_some(())?;
    (args.get(1..3)? == ["peer", "serve"]).then_some(())?;
    let cwd = fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
    Some((cwd, args[3..].to_vec()))
}

/// Whether a live daemon lease in the device `epr` store names `pid` as its owner.
fn daemon_lease_names(pid: u32) -> bool {
    crate::bin::daemon_store()
        .and_then(|store| store.live_daemons(&crate::bin::daemon_proc_root()))
        .is_ok_and(|leases| leases.iter().any(|lease| lease.owner_pid == Some(pid)))
}

/// The release (or raw byte) CID of the bytes `pid` executes, when they can be read.
fn holder_exe_cid(pid: u32) -> String {
    let exe = PathBuf::from(format!("/proc/{pid}/exe"));
    crate::bin::daemon_store()
        .ok()
        .and_then(|store| store.executable_cid(&exe).ok())
        .or_else(|| file_cid(&exe).ok())
        .unwrap_or_else(|| "unreadable".into())
}

/// The real uid `/proc/<pid>/status` reports.
fn proc_uid(pid: u32) -> Option<u32> {
    fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("Uid:")?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}

/// After arrival moved a tool's current, or found a supervised child behind it: when this
/// device's ark runs, the head declares a process and `device_restart.on_landing` is not `off`,
/// supervise once (serialised with every other restart of this ark by `restart.lock`) so a
/// child still on the old bytes restarts onto the new ones, and write the row. An ark nobody
/// started stays unstarted; arrival never begins supervision on its own. Its authority is the
/// hand command's (row 35, F6): an unresolvable session still supervises unleased.
pub(crate) fn after_landing(checkout: &Path) -> Outcome<()> {
    let data_root = default_data_root(checkout);
    let head = super::declaration::store(None, None, None)?
        .read()
        .map_err(|e| e.to_string())?;
    if head.processes.is_empty() {
        return Ok(());
    }
    let Some(before) = running(&data_root)? else {
        say!("device ark: not running; the landed binary waits for `epr device supervise`");
        return Ok(());
    };
    let stale = match child_identity(&data_root, &head, &before) {
        Ok(Identity::Stale {
            child,
            running,
            current,
        }) => Some(StaleChild {
            child,
            running,
            current,
        }),
        _ => None,
    };
    let row = || {
        receipt::Row::new(
            "arrival",
            stale.as_ref().map(|s| s.running.clone()),
            stale.as_ref().map(|s| s.current.clone()),
        )
    };
    if !receipt::restart_on_landing(&main_checkout(checkout)) {
        let reason = format!(
            "device_restart.on_landing is off in {} at HEAD; the landed binary waits for `epr \
             device supervise`",
            main_checkout(checkout).join(receipt::POLICY).display()
        );
        say!("device ark: not restarting: {reason}");
        if stale.is_some() {
            row().record(&data_root, "skipped", Some(reason), 0);
        }
        return Ok(());
    }
    let turn = RestartTurn::take(&data_root)?;
    let supervised = supervise(&data_root, &head, lease_tool(checkout).as_deref(), false);
    drop(turn);
    let after = running(&data_root).ok().flatten();
    match supervised {
        Ok(_) => {
            let (Some(stale), Some(after)) = (&stale, after) else {
                return Ok(());
            };
            if after.pid == before.pid {
                return Ok(());
            }
            let tool = head
                .processes
                .iter()
                .find(|p| p.name == stale.child)
                .and_then(|p| channel_tool(&p.artifact))
                .unwrap_or_default()
                .to_owned();
            match wait_landed(&data_root, &head, &after, &tool) {
                Ok(took) => {
                    say!(
                        "device ark: {} ready on {} in {} s",
                        stale.child,
                        child_release(&data_root, &head, &after, &tool)
                            .unwrap_or_else(|| stale.current.clone()),
                        whole_seconds(took)
                    );
                    row().record(&data_root, "ready", None, whole_seconds(took));
                }
                Err((took, why)) => {
                    say!(
                        "device ark: {} not ready after {} s ({why}); the device is degraded; \
                         cure: epr device supervise --status",
                        stale.current,
                        whole_seconds(took)
                    );
                    row().record(&data_root, "not-ready", Some(why), whole_seconds(took));
                }
            }
            Ok(())
        }
        Err(error) => {
            // The ark that ran is gone: the failure came after the stop, and the device is down.
            if after.is_none() {
                say!("device ark: {error}; the device is degraded; cure: epr device supervise");
                row().record(&data_root, "failed", Some(error.clone()), 0);
            }
            Err(error)
        }
    }
}

/// How long a landing waits its turn behind another restart of the same ark.
const RESTART_TURN: Duration = Duration::from_secs(120);
/// No child's readiness budget, counted from its own spawn, is shorter than this.
const READY_FLOOR: Duration = Duration::from_secs(10);
/// While the replacement ark lives and has not yet spawned a child of the landed tool, the wait
/// is for that first spawn, this long from the ark's start. A wrong "not ready" costs more than
/// a slow one: it tells the person the device is degraded when it is only busy (CI #197 read
/// four healthy restarts as not ready on a 12 s floor; the device's real restarts on
/// 2026-11-20 served 23-43 s after the command).
const FIRST_SPAWN_WAIT: Duration = Duration::from_secs(60);
/// Ark publishes a passed rung a little after its own deadline clock reads it.
const READY_SLACK: Duration = Duration::from_secs(2);

/// What a restart after a landing did, as the landing's own record carries it (row 35, F1).
pub(crate) struct Report {
    /// `ready`, `not-ready`, `failed` or `skipped`.
    pub(crate) outcome: &'static str,
    /// What the child ran before.
    pub(crate) from: Option<String>,
    /// What the child runs now, read from the child itself; else what it was restarted onto.
    pub(crate) to: Option<String>,
    pub(crate) seconds: u64,
    pub(crate) reason: Option<String>,
    /// The command that restores the device, when it is degraded.
    pub(crate) cure: Option<String>,
}

impl Report {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "outcome": self.outcome,
            "from": self.from,
            "to": self.to,
            "seconds": self.seconds,
            "reason": self.reason,
            "cure": self.cure,
        })
    }
}

/// How the restart after a landing ended.
pub(crate) enum AfterLanding {
    /// Nothing was owed: no ark, no child on the landed channel behind current, no main checkout.
    Untouched,
    /// A restart was owed; this is what happened (written to the receipt too).
    Reported(Report),
}

/// After `epr bin follow` or a forward `epr bin land` moved `tool`'s current: the hand command
/// `epr device supervise`, run for the person, with the failure behaviour it has. The landing's
/// own exit status is never this function's: its outcome goes into the record, the receipt and
/// status, and a degraded device says so in one line with the cure.
///
/// The device root comes from the main checkout of `checkout` (git's common directory), as the
/// hand command finds it. It acts only when a child on `tool`'s channel runs other bytes than
/// its current, and then restarts the WHOLE ark, as the hand command does: ark cannot restart
/// one child (a per-child restart goes with ark's admin socket). The verdict is judged on the
/// landed tool's children alone, on ark's clock; the other children's state is reported.
/// Nothing is reverted or held.
///
/// The restart is by itself only when this act holds the device-ark lease (it claims it; berth
/// renews a lease the session already holds) or no live holder exists. With no berth, no
/// session berth can resolve, or another live holder, nothing restarts, the reason is said, and
/// a known holder is sent a berth note naming the one command to run. Under `restart.lock` the
/// lease, the declaration and the switch are judged again, and the decision is made from the
/// child: a child another act already restarted is waited for, never assumed.
pub(crate) fn after_landed(
    checkout: &Path,
    trigger: &str,
    tool: &str,
    landed: Option<&str>,
) -> AfterLanding {
    let Some(device) = device_checkout(checkout) else {
        say!(
            "device ark: {} has no main checkout git can name; no automatic restart — run `epr \
             device supervise`",
            checkout.display()
        );
        return AfterLanding::Untouched;
    };
    let data_root = device.join(DATA_REL);
    let read_head = || {
        super::declaration::store(None, None, None)
            .and_then(|store| store.read().map_err(|e| e.to_string()))
    };
    let declares = |head: &RuntimeManifest| {
        head.processes
            .iter()
            .any(|p| channel_tool(&p.artifact) == Some(tool))
    };
    let head = match read_head() {
        Ok(head) => head,
        Err(error) => {
            say!("device ark: device manifest unreadable ({error}); no automatic restart");
            return AfterLanding::Untouched;
        }
    };
    if !declares(&head) {
        return AfterLanding::Untouched;
    }
    let started = match running(&data_root) {
        Ok(Some(started)) => started,
        Ok(None) => {
            match custom_root_ark(&data_root) {
                Some(other) => say!(
                    "device ark: none runs {}; an ark runs a custom data root ({}), which a \
                     landing never restarts — run `epr device supervise --data-root {}`",
                    data_root.display(),
                    other.display(),
                    other.display()
                ),
                None => say!(
                    "device ark: not running; the landed binary waits for `epr device supervise`"
                ),
            }
            return AfterLanding::Untouched;
        }
        Err(error) => {
            say!("device ark: {error}; no automatic restart");
            return AfterLanding::Untouched;
        }
    };
    let report = |outcome: &'static str,
                  stale: Option<&StaleChild>,
                  to: Option<String>,
                  seconds: u64,
                  reason: Option<String>,
                  cure: Option<&str>| {
        let report = Report {
            outcome,
            from: stale.map(|s| s.running.clone()),
            to: to.or_else(|| stale.map(|s| s.current.clone())),
            seconds,
            reason,
            cure: cure.map(str::to_owned),
        };
        receipt::Row::new(trigger, report.from.clone(), report.to.clone()).record(
            &data_root,
            outcome,
            report.reason.clone(),
            seconds,
        );
        AfterLanding::Reported(report)
    };
    let skip = |stale: Option<&StaleChild>, reason: String| {
        say!("device ark: not restarting: {reason}");
        report("skipped", stale, None, 0, Some(reason), None)
    };
    let head_cid = match head.cid() {
        Ok(cid) => cid,
        Err(error) => return skip(None, error.to_string()),
    };
    if started.manifest_cid != head_cid {
        return skip(
            None,
            format!(
                "ark runs manifest {} but the head is {head_cid}; a reload is the hand command's \
                 — run `epr device supervise`",
                started.manifest_cid
            ),
        );
    }
    let stale = match child_identity_on(&data_root, &head, &started, Some(tool)) {
        Ok(Identity::Stale {
            child,
            running,
            current,
        }) => StaleChild {
            child,
            running,
            current,
        },
        // Nothing on the landed channel runs older bytes: nothing to restart, nothing owed.
        Ok(Identity::Current | Identity::NoLiveChild) => return AfterLanding::Untouched,
        Err(error) => return skip(None, error),
    };
    let stale = Some(&stale);
    let off = |device: &Path| {
        (!receipt::restart_on_landing(device)).then(|| {
            format!(
                "device_restart.on_landing is off in {} at HEAD; the landed binary waits for \
                 `epr device supervise`",
                device.join(receipt::POLICY).display()
            )
        })
    };
    if let Some(reason) = off(&device) {
        return skip(stale, reason);
    }
    // The hand command's own refusals, before anything is claimed or stopped.
    let refused = refuse_declared_launcher_env(&head)
        .and_then(|()| peer_serve_preflight(&head, &data_root, Some(&started), false).map(|_| ()))
        .and_then(|()| {
            let berth = data_root.join("berth.json");
            match arks_for(&berth).into_iter().find(|pid| *pid != started.pid) {
                Some(pid) => Err(format!(
                    "REFUSED: ark pid {pid} already runs {} and supervise did not start it",
                    berth.display()
                )),
                None => Ok(()),
            }
        })
        .and_then(|()| preflight(&head))
        .and_then(|()| ark_bin(&data_root).map(|(ark, _)| ark));
    let ark = match refused {
        Ok(ark) => ark,
        Err(error) => return skip(stale, error),
    };
    let berth = lease_tool(&device);
    let authorised =
        |stale: Option<&StaleChild>| match claim_for_landing(berth.as_deref(), &head_cid) {
            Lease::Held => None,
            Lease::Refused { why, holder } => {
                if let (Some(berth), Some(holder)) = (berth.as_deref(), holder.as_deref()) {
                    tell_holder(berth, holder, trigger, stale);
                }
                Some(format!(
                    "{why}; nothing restarted — run `epr device supervise` from the session that \
                 holds device-ark"
                ))
            }
        };
    if let Some(reason) = authorised(stale) {
        return skip(stale, reason);
    }
    // One restart of this ark at a time.
    let turn = match RestartTurn::take(&data_root) {
        Ok(turn) => turn,
        Err(error) => return skip(stale, error),
    };
    // F4: what authorised the stop is judged again at the stop boundary, under the lock: the
    // lease (claimed or renewed again), the declaration and the switch.
    match read_head() {
        Ok(now) if now.cid().ok().as_deref() == Some(head_cid.as_str()) && declares(&now) => {}
        Ok(_) => {
            return skip(
                stale,
                "the device manifest changed while this landing waited its turn; run `epr \
                 device supervise`"
                    .into(),
            )
        }
        Err(error) => return skip(stale, error),
    }
    if let Some(reason) = off(&device) {
        return skip(stale, reason);
    }
    if let Some(reason) = authorised(stale) {
        return skip(stale, reason);
    }
    let started = match running(&data_root) {
        Ok(Some(started)) => started,
        _ => {
            return skip(
                stale,
                "the ark stopped while this landing waited its turn".into(),
            )
        }
    };
    // F2: decide from the child, never from the ark's pid.
    let stale = match child_identity_on(&data_root, &head, &started, Some(tool)) {
        Ok(Identity::Stale {
            child,
            running,
            current,
        }) => StaleChild {
            child,
            running,
            current,
        },
        Ok(Identity::Current | Identity::NoLiveChild) => {
            drop(turn);
            let child = stale.map_or("the child", |s| s.child.as_str()).to_owned();
            say!(
                "device ark: {child} was already restarted (ark pid {}); waiting for it to be \
                 ready rather than restarting it again",
                started.pid
            );
            return settle(
                &data_root, &head, &started, tool, landed, &child, stale, &report,
            );
        }
        Err(error) => return skip(stale, error),
    };
    say!(
        "device ark: {} runs {}, current is {}; restarting the ark (after {trigger}, session \
         {}): every child restarts with it, as with `epr device supervise`",
        stale.child,
        stale.running,
        stale.current,
        session_id().as_deref().unwrap_or("none")
    );
    // From the stop on, every failure leaves the device down or degraded; it is said once, with
    // the cure, and reported. The landing itself stands either way.
    let failed = |error: String| {
        say!("device ark: {error}; the device is degraded; cure: epr device supervise");
        release_lease(berth.as_deref());
        report(
            "failed",
            Some(&stale),
            None,
            0,
            Some(error),
            Some("epr device supervise"),
        )
    };
    if let Err(error) = stop_ark(&data_root, &started) {
        return failed(error);
    }
    let restarted = match start_ark(&data_root, &head, &head_cid, &ark) {
        Ok(restarted) => restarted,
        Err(error) => {
            return failed(format!(
                "the old ark stopped and its replacement did not start cleanly: {error}"
            ))
        }
    };
    say!(
        "device ark: started pid {} at manifest {head_cid} (log {})",
        restarted.pid,
        data_root.join("ark.log").display()
    );
    drop(turn);
    let child = stale.child.clone();
    settle(
        &data_root,
        &head,
        &restarted,
        tool,
        landed,
        &child,
        Some(&stale),
        &report,
    )
}

/// Record one outcome: `(outcome, stale, observed release, seconds, reason, cure)`.
type ReportFn<'a> = dyn Fn(
        &'static str,
        Option<&StaleChild>,
        Option<String>,
        u64,
        Option<String>,
        Option<&str>,
    ) -> AfterLanding
    + 'a;

/// Wait for `tool`'s children under `started`, report what the child actually runs (F3) and the
/// other children's state (F5), and write the row through `report`.
#[allow(
    clippy::too_many_arguments,
    reason = "The settlement takes the restart's context as the call site already holds it"
)]
fn settle(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
    tool: &str,
    landed: Option<&str>,
    child: &str,
    stale: Option<&StaleChild>,
    report: &ReportFn<'_>,
) -> AfterLanding {
    let waited = wait_landed(data_root, head, started, tool);
    let observed = child_release(data_root, head, started, tool);
    let aimed = observed
        .clone()
        .or_else(|| stale.map(|s| s.current.clone()))
        .unwrap_or_else(|| "current".into());
    let also = match landed {
        Some(landed) if landed != aimed => format!(" (this act landed {landed})"),
        _ => String::new(),
    };
    for other in head
        .processes
        .iter()
        .filter(|p| channel_tool(&p.artifact) != Some(tool))
    {
        let row = load_passport(data_root)
            .and_then(|p| p.processes.into_iter().find(|row| row.name == other.name));
        let state = match row {
            Some(row)
                if row
                    .pid
                    .is_some_and(|pid| proc_ppid(pid) == Some(started.pid)) =>
            {
                if other.readiness.is_empty() || row.ready {
                    "ready"
                } else {
                    "not ready"
                }
            }
            _ => "not running",
        };
        say!(
            "device ark: {} ({}) restarted with the ark: {state}; not judged for this landing",
            other.name,
            match &other.artifact {
                ark_core::ArtifactRef::Channel { channel_id } => channel_id.as_str(),
                ark_core::ArtifactRef::Pinned { .. } => "pinned",
            }
        );
    }
    match waited {
        Ok(took) => {
            say!(
                "device ark: {child} ready on {aimed} in {} s{also}",
                whole_seconds(took)
            );
            report("ready", stale, observed, whole_seconds(took), None, None)
        }
        Err((took, why)) => {
            say!(
                "device ark: {aimed} not ready after {} s ({why}){also}; the device is degraded; \
                 cure: epr device supervise --status",
                whole_seconds(took)
            );
            report(
                "not-ready",
                stale,
                observed,
                whole_seconds(took),
                Some(why),
                Some("epr device supervise --status"),
            )
        }
    }
}

/// The release the first live child of `started` on `tool`'s channel executes, read from the
/// child itself (`/proc/<pid>/exe`).
fn child_release(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
    tool: &str,
) -> Option<String> {
    let registry = manifest_store::registry(head).ok()?;
    let store = Store::from_environment(tool, &registry).ok()?;
    load_passport(data_root)?
        .processes
        .iter()
        .filter(|row| {
            head.processes
                .iter()
                .any(|p| p.name == row.name && channel_tool(&p.artifact) == Some(tool))
        })
        .filter_map(|row| row.pid)
        .filter(|pid| proc_ppid(*pid) == Some(started.pid))
        .find_map(|pid| {
            let exe = PathBuf::from(format!("/proc/{pid}/exe"));
            store
                .executable_cid(&exe)
                .ok()
                .or_else(|| file_cid(&exe).ok())
        })
}

/// A child found on other bytes than its channel's current.
struct StaleChild {
    child: String,
    running: String,
    current: String,
}

/// The session an act runs in, written down as its cause.
fn session_id() -> Option<String> {
    elohim_device_runtime::session::resolve().session
}

/// Whole seconds, rounded up so a quick start never reads as zero.
fn whole_seconds(took: Duration) -> u64 {
    took.as_millis()
        .div_ceil(1000)
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The main checkout of `checkout`, through git's common directory (with the declared trust).
fn device_checkout(checkout: &Path) -> Option<PathBuf> {
    let output = crate::git::command(checkout)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let common = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    (common.file_name()? == ".git")
        .then(|| common.parent().map(Path::to_owned))
        .flatten()
}

/// An ark of this user running a berth other than `data_root`'s: a custom data root.
fn custom_root_ark(data_root: &Path) -> Option<PathBuf> {
    let mine = data_root.join("berth.json");
    let me = proc_uid(std::process::id());
    fs::read_dir("/proc")
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| proc_uid(*pid) == me)
        .find_map(|pid| {
            let cmdline = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
            let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
            if !args.contains(&b"run".as_slice()) {
                return None;
            }
            let berth = args
                .windows(2)
                .find(|w| w[0] == b"--berth")
                .map(|w| PathBuf::from(String::from_utf8_lossy(w[1]).into_owned()))?;
            (berth != mine && berth.file_name()? == "berth.json")
                .then(|| berth.parent().map(Path::to_owned))
                .flatten()
        })
}

/// Whether this act may restart the device's ark.
enum Lease {
    Held,
    Refused { why: String, holder: Option<String> },
}

/// Claim (or renew) device-ark for a landing. Unlike the hand command, no berth and no
/// resolvable session are refusals here: an unattended act never stops an ark unleased.
fn claim_for_landing(berth: Option<&Path>, head_cid: &str) -> Lease {
    let Some(berth) = berth else {
        return Lease::Refused {
            why: "no berth on this device to hold the device-ark lease".into(),
            holder: None,
        };
    };
    let output = match Command::new(berth)
        .args(["claim", LEASE, "--class", LEASE, "--note"])
        .arg(format!("epr bin landing restarts {head_cid}"))
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return Lease::Refused {
                why: format!("{}: {error}", berth.display()),
                holder: None,
            }
        }
    };
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    match output.status.code() {
        Some(0) => Lease::Held,
        Some(4) => Lease::Refused {
            why: "no berth session resolvable for this act, so it cannot hold device-ark".into(),
            holder: None,
        },
        code => Lease::Refused {
            holder: stderr
                .split("held by ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
                .map(|holder| holder.trim_end_matches(['.', ',', ';']).to_owned()),
            why: format!(
                "{stderr}{}",
                code.map_or_else(String::new, |c| format!(" (berth exit {c})"))
            ),
        },
    }
}

/// A berth note to the session holding device-ark, naming the one command to run.
fn tell_holder(berth: &Path, holder: &str, trigger: &str, stale: Option<&StaleChild>) {
    let me = session_id();
    let text = format!(
        "device ark: an `epr bin {trigger}` (session {}) landed {} on this device, and you hold \
         device-ark, so nothing restarted; run `epr device supervise` to restart {} onto it",
        me.as_deref().unwrap_or("unresolved"),
        stale.map_or("a new release", |s| s.current.as_str()),
        stale.map_or("the child", |s| s.child.as_str())
    );
    let mut command = Command::new(berth);
    command.arg("say");
    if let Some(me) = &me {
        command.args(["--session", me]);
    }
    match command.args(["--to", holder]).arg(text).output() {
        Ok(output) if output.status.success() => {
            say!("device ark: told {holder}, who holds device-ark (berth say)");
        }
        Ok(output) => say!(
            "device ark: the note to {holder} was not sent: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => say!("device ark: the note to {holder} was not sent: {error}"),
    }
}

/// This act's turn to restart the device's ark: an exclusive flock on `<data root>/restart.lock`,
/// held only around the stop and start, never during the readiness wait.
struct RestartTurn {
    /// Closing it releases the turn.
    _held: fs::File,
}

impl RestartTurn {
    fn take(data_root: &Path) -> Outcome<RestartTurn> {
        use std::os::fd::AsRawFd;
        let path = data_root.join("restart.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let deadline = Instant::now() + RESTART_TURN;
        loop {
            // SAFETY: the fd is open and owned by `file` for this call; flock keeps no pointer.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(RestartTurn { _held: file });
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::WouldBlock {
                return Err(format!("{}: {error}", path.display()));
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "another restart of this ark held {} for {}s",
                    path.display(),
                    RESTART_TURN.as_secs()
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

/// Wait for the children on `tool`'s channel of `started`, on ark's clock (row 35, F7): each
/// child's budget is its own declared readiness patience (never less than [`READY_FLOOR`]),
/// counted from its own first spawn as the passport records it. Until ark has spawned a child,
/// the wait is for that first spawn, up to [`FIRST_SPAWN_WAIT`] from the ark's start, while
/// the ark lives. A child that declares no readiness is ready once
/// ark runs it. Any child past its budget fails the wait, whatever time a sibling has left.
/// Other channels' children are never judged. Reads the passport only; holds nothing.
fn wait_landed(
    data_root: &Path,
    head: &RuntimeManifest,
    started: &Started,
    tool: &str,
) -> Result<Duration, (Duration, String)> {
    let children: Vec<&ark_core::ChildSpec> = head
        .processes
        .iter()
        .filter(|p| channel_tool(&p.artifact) == Some(tool))
        .collect();
    let millis = |duration: Duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
    let budget = |child: &ark_core::ChildSpec| {
        let declared: u64 = child
            .readiness
            .iter()
            .map(|probe| match probe {
                ark_core::Probe::ExecutableIdentity { patience_ms }
                | ark_core::Probe::StdoutLine { patience_ms, .. }
                | ark_core::Probe::TcpListen { patience_ms, .. } => *patience_ms,
            })
            .fold(0, u64::saturating_add);
        millis(Duration::from_millis(declared).max(READY_FLOOR) + READY_SLACK)
    };
    let unseen = millis(FIRST_SPAWN_WAIT);
    let begun = Instant::now();
    let mut spawned: std::collections::BTreeMap<String, u64> = Default::default();
    loop {
        if !is_same_process(started.pid, started.start_ticks) {
            return Err((begun.elapsed(), "ark exited".into()));
        }
        let passport = load_passport(data_root);
        let now = epoch_ms();
        let mut waiting = Vec::new();
        let mut overdue = Vec::new();
        for child in &children {
            let row = passport
                .as_ref()
                .and_then(|p| p.processes.iter().find(|row| row.name == child.name));
            let ours = row
                .and_then(|row| row.pid)
                .is_some_and(|pid| proc_ppid(pid) == Some(started.pid));
            if ours {
                let at = row.and_then(|row| row.started_at_epoch_ms).unwrap_or(now);
                spawned.entry(child.name.clone()).or_insert(at);
            }
            if ours && (child.readiness.is_empty() || row.is_some_and(|row| row.ready)) {
                continue;
            }
            waiting.push(child.name.as_str());
            let deadline = match spawned.get(&child.name) {
                Some(first) => first.saturating_add(budget(child)),
                None => started.started_at_epoch_ms.saturating_add(unseen),
            };
            if now >= deadline {
                overdue.push(child.name.as_str());
            }
        }
        if waiting.is_empty() {
            return Ok(begun.elapsed());
        }
        if !overdue.is_empty() {
            return Err((
                begun.elapsed(),
                format!("not ready: {}", overdue.join(", ")),
            ));
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// What the newest restart row says on the peer daemon line.
pub(crate) enum RestartNote {
    /// A ready restart: the suffix for a line serving at current.
    Ready(String),
    /// A restart that did not come up or failed after the stop: the line is degraded while the
    /// supervisor reads anything short of every child ready.
    Degraded { note: String, cure: String },
    /// A landing that skipped its restart, and why.
    Note(String),
}

/// The newest restart row this device's ark data root holds, as the peer daemon line says it.
pub(crate) fn restart_note(checkout: &Path) -> Option<RestartNote> {
    let data_root = device_checkout(checkout)
        .map(|device| device.join(DATA_REL))
        .unwrap_or_else(|| default_data_root(checkout));
    let row = receipt::newest(&data_root)?;
    let to = row.to.as_deref().unwrap_or("-");
    let reason = row.reason.clone().unwrap_or_default();
    Some(match row.outcome.as_str() {
        "ready" => RestartNote::Ready(format!(
            " (restarted by {} at {} in {} s)",
            row.trigger, row.at, row.seconds
        )),
        "not-ready" => RestartNote::Degraded {
            note: format!(
                " (last restart by {} at {}: {to} not ready after {} s: {reason})",
                row.trigger, row.at, row.seconds
            ),
            cure: "epr device supervise --status".into(),
        },
        "failed" => RestartNote::Degraded {
            note: format!(
                " (last restart by {} at {} failed after the old ark stopped: {reason})",
                row.trigger, row.at
            ),
            cure: "epr device supervise".into(),
        },
        _ => RestartNote::Note(format!(
            " (the {} at {} did not restart: {reason})",
            row.trigger, row.at
        )),
    })
}

/// Whether this device's running ark, at the head manifest, holds a child on other bytes than
/// its channel's current. Arrival asks it so a current that some other verb moved (`epr bin
/// follow`, a landing's own ingest) still reaches the running child: found on shem at dev
/// 34b2fdabe, where a followed epr left the supervised peer on the old bytes and the next
/// arrival, having moved nothing itself, asked nothing. Anything unreadable answers `false`;
/// arrival never disturbs supervision on a guess.
pub(crate) fn child_behind(checkout: &Path) -> bool {
    let data_root = default_data_root(checkout);
    let Ok(head) = super::declaration::store(None, None, None)
        .and_then(|store| store.read().map_err(|e| e.to_string()))
    else {
        return false;
    };
    let (Ok(head_cid), Ok(Some(started))) = (head.cid(), running(&data_root)) else {
        return false;
    };
    started.manifest_cid == head_cid
        && matches!(
            child_identity(&data_root, &head, &started),
            Ok(Identity::Stale { .. })
        )
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
        // Ark (and the bin: children it resolves) choose epr from the verified current, never
        // from a candidate whoever ran supervise happened to carry.
        .env_remove("EPR_BIN")
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
    // The record stays: `running` reads it as stopped (the pid no longer matches its start
    // tick), and `recorded_ark` still finds the ark this device chose after a clean stop.
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

/// The parent pid from `/proc/<pid>/stat`, for a process that is not a zombie.
fn proc_ppid(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.get(stat.rfind(')')? + 2..)?;
    let mut fields = rest.split_whitespace();
    let state = fields.next()?;
    (state != "Z").then_some(())?;
    fields.next()?.parse().ok()
}

/// Whether `pid` is `ancestor`'s child or a descendant of one (a child may hand its lock to a
/// worker it forked), walking at most a few parents.
fn descends_from(pid: u32, ancestor: u32) -> bool {
    let mut current = pid;
    for _ in 0..8 {
        match proc_ppid(current) {
            Some(parent) if parent == ancestor => return true,
            Some(parent) if parent > 1 => current = parent,
            _ => return false,
        }
    }
    false
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

fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// The ark this data root last started, while that file is still executable. It is read
/// whether or not that ark still runs: a follow driven by a hook carries no `ARK_BIN`, and the
/// ark a steward chose once must not silently fall back to an older pool build.
fn recorded_ark(data_root: &Path) -> Option<PathBuf> {
    let bytes = fs::read(data_root.join("supervise.json")).ok()?;
    let started: Started = serde_json::from_slice(&bytes).ok()?;
    executable(&started.ark).then_some(started.ark)
}

/// `ARK_BIN`, else the ark this data root last started (see [`recorded_ark`]), else the pool
/// slot `hc-mesh.sh` prefers, else `ark` on PATH — with the name of the rule that chose it.
fn ark_bin(data_root: &Path) -> Outcome<(PathBuf, &'static str)> {
    if let Some(explicit) = std::env::var_os("ARK_BIN").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        return if executable(&path) {
            Ok((path, "ARK_BIN"))
        } else {
            Err(format!(
                "REFUSED: ARK_BIN={} is not an executable file",
                path.display()
            ))
        };
    }
    if let Some(recorded) = recorded_ark(data_root) {
        return Ok((recorded, "recorded in supervise.json"));
    }
    let pool = ark_pool_slot();
    if let Some(pool) = pool.as_ref().filter(|pool| executable(pool)) {
        return Ok((pool.clone(), "cargo pool slot"));
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join("ark"))
        .find(|p| executable(p))
        .map(|path| (path, "PATH"))
        .ok_or_else(|| {
            let target = pool.as_ref().map_or_else(
                || "<target dir>".to_owned(),
                |slot| slot.display().to_string().trim_end_matches("/debug/ark").to_owned(),
            );
            format!(
                "REFUSED: no ark binary — use the toolchain pinned in genesis/agentic/pool-policy.json \
                 to run `(cd elohim && CARGO_TARGET_DIR={target} RUSTFLAGS= cargo build -p elohim-ark)`, \
                 or set ARK_BIN"
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
        say!("device ark: no berth on this device; supervising unleased");
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
            say!("device ark: no berth session resolvable; supervising unleased");
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
    fn the_recorded_ark_is_reused_only_while_it_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        assert!(recorded_ark(dir.path()).is_none(), "no record");
        let ark = dir.path().join("ark-bin");
        fs::write(&ark, "#!/bin/sh\n").unwrap();
        let record = Started {
            pid: 1,
            start_ticks: 0,
            manifest_cid: "bafy".into(),
            ark: ark.clone(),
            started_at_epoch_ms: 0,
        };
        write_atomic(
            &dir.path().join("supervise.json"),
            &serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        fs::set_permissions(&ark, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(recorded_ark(dir.path()).is_none(), "not executable");
        fs::set_permissions(&ark, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            recorded_ark(dir.path()),
            Some(ark.clone()),
            "read although that ark no longer runs"
        );
        fs::remove_file(&ark).unwrap();
        assert!(recorded_ark(dir.path()).is_none(), "gone");
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
