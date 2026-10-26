//! `ark task spec` and `ark task run`: a device's own task, placed in its own envelope.
//!
//! The first instance is a local gate (`brit-gate:<project>`), which `gate-runner.mjs` hands over
//! as a closure material file and a request. `spec` builds the [`TaskSpec`], so the method's exec
//! CID and the input root are computed here and nowhere in JavaScript. `run` follows the
//! interfaces spec §F flow on one device, offline and with no grant:
//!
//! 1. decode the spec (checked) and refuse anything but an own-envelope task;
//! 2. ask the device membrane (`decide_private`) whether it carries tasks, then admit the need
//!    through placement's floor against the device's own [`BerthOffer`];
//! 3. look the derivation up in brit ([`BritIndex`]); a hit runs no child;
//! 4. otherwise run the child under ark's native driver, its stdout and stderr passed through
//!    byte for byte so a wrapping `gate-tap.mjs` still reads them.
//!
//! The answer travels as `PendingStatus<TaskOutcome, DerivationUnanswered>` JSON lines appended to
//! `--status` (never stdout, which is the child's, and never fd 3, which is the tap's): `queued`,
//! then `answered`, or `unanswered` with ruling 7's three parts when the run is interrupted or the
//! floor does not carry it. An interrupted run is never answered.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI32, Ordering},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ark_core::{
    manifest::{ResourceQuota, RuntimeEnvelope},
    Berth, BerthOffer, ExitClass, Passport, RuntimeManifest, TaskChild, TaskCompletion, TaskKind,
    TaskOutcome, TaskSpec, PASSPORT_KIND, TASK_CHILD_NAME, UNIDENTIFIED_AGENT,
};
use ark_supervisor::{
    become_subreaper, reap_with_rusage, wait_nowait, Driver, DriverError, NativeDriver, WaitEvent,
};
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use eprfs_core::{
    BlobCid, DerivationIndex, DerivationLookup, DerivationUnanswered, KeyCustody, MethodDescriptor,
    Requester,
};
use nix::{
    libc,
    sys::{
        signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
        wait::{waitid, waitpid, Id, WaitPidFlag},
    },
};
use placement_core::{
    admit, decide_private, AgentRefKey, Candidate, Capacity, Consent, DeviceFacts, FailureDomain,
    GrantScope, KindId, NeedClass, PersonChoices, PrivateSnapshot,
};
use seam_contracts::{Answer, ComeBack, Meanwhile, PendingStatus, Unanswered};
use serde::Deserialize;

use crate::{
    brit_index::{resolve_build_ref, BritIndex, ReuseContext},
    material::Material,
    Failure, EXIT_ARTIFACT,
};

/// `EX_TEMPFAIL`: the floor did not carry the task here; nothing ran.
const EXIT_NOT_CARRIED: u8 = 75;

/// How long a forwarded interrupt waits for the child before the tree is killed.
const INTERRUPT_GRACE: Duration = Duration::from_secs(10);

/// How long an interrupted run waits for its output pipes after the child is gone.
const DRAIN_AFTER_INTERRUPT: Duration = Duration::from_secs(2);

/// The interrupt this process received, 0 for none; set only by [`record_interrupt`].
static INTERRUPT: AtomicI32 = AtomicI32::new(0);

/// What `gate-runner.mjs` asks for: the method's identity without its exec CID or view, the child
/// and its outputs. `spec` fills the rest from the child and the material.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskRequest {
    method: MethodRequest,
    child: TaskChild,
    #[serde(default)]
    outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MethodRequest {
    name: String,
    #[serde(default)]
    tools: BTreeMap<String, String>,
    #[serde(default)]
    flags: Vec<String>,
    step: Option<String>,
    #[serde(default)]
    non_path_inputs: BTreeMap<String, String>,
    deterministic: bool,
}

/// `ark task spec`: prints `{spec, derivation: {method, input}}`, the checked [`TaskSpec`] for a
/// request over a closure material and its derivation key's two CIDs. The method's view is every
/// material field and its exec is the child's CID; the input root is the material's tree.
pub fn spec(request: &Path, material: &Path) -> Result<u8, Failure> {
    let request: TaskRequest = serde_json::from_str(&read(request, "task request")?)
        .map_err(|error| Failure::data(format!("task request: {error}")))?;
    let material = Material::read(material).map_err(Failure::data)?;
    let view = material.view().map_err(Failure::data)?;
    let exec = request
        .child
        .exec_cid(None, &request.outputs)
        .map_err(|error| Failure::data(error.to_string()))?;
    let method = MethodDescriptor {
        name: request.method.name,
        tools: request.method.tools,
        flags: request.method.flags,
        params: None,
        exec: Some(exec),
        step: request.method.step,
        embedder: None,
        view: view.paths().cloned().collect(),
        non_path_inputs: request.method.non_path_inputs,
        deterministic: request.method.deterministic,
    };
    let spec = TaskSpec::new(
        method,
        material.root().clone(),
        None,
        request.child,
        None,
        request.outputs,
    )
    .map_err(|error| Failure::data(error.to_string()))?;
    let projected = material.projected(spec.input()).map_err(Failure::data)?;
    let key = spec
        .derivation_key(&projected)
        .map_err(|error| Failure::data(error.to_string()))?;
    let json = serde_json::to_string(&serde_json::json!({
        "spec": spec,
        "derivation": { "method": key.method, "input": key.input.cid() },
    }))
    .map_err(|error| Failure::data(error.to_string()))?;
    println!("{json}");
    Ok(0)
}

/// Where `ark task run` reads and writes.
pub struct RunArgs {
    pub spec: PathBuf,
    pub material: PathBuf,
    pub artifact: PathBuf,
    pub status: PathBuf,
    pub reuse: Option<PathBuf>,
    pub repo: PathBuf,
    pub data_root: PathBuf,
    pub device: Option<String>,
}

type Status = PendingStatus<TaskOutcome, DerivationUnanswered>;

/// Appends status lines to `--status`, one JSON object each.
struct StatusFile(PathBuf);

impl StatusFile {
    fn append(&self, status: &Status) -> Result<(), Failure> {
        let mut line = serde_json::to_string(status)
            .map_err(|error| Failure::data(format!("status: {error}")))?;
        line.push('\n');
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .map_err(|error| Failure::data(format!("writing {}: {error}", self.0.display())))
    }
}

/// `ark task run`: places the spec's task in the device's own envelope and answers it.
pub fn run(args: RunArgs) -> Result<u8, Failure> {
    let spec: TaskSpec = serde_json::from_str(&read(&args.spec, "task spec")?)
        .map_err(|error| Failure::data(format!("task spec: {error}")))?;
    if spec.scope() != Some(GrantScope::OwnEnvelope) {
        return Err(Failure::data(format!(
            "ark task run carries only the device's own task (a `{}` method); `{}` goes through a \
             provider",
            placement_core::GATE_METHOD_PREFIX,
            spec.method().name
        )));
    }
    let material = Material::read(&args.material).map_err(Failure::data)?;
    if material.root() != spec.input().root() {
        return Err(Failure::data(
            "the closure material is not the spec's input root",
        ));
    }
    let step = spec
        .method()
        .step
        .clone()
        .ok_or_else(|| Failure::data("a gate method names its brit step"))?;
    let projected = material.projected(spec.input()).map_err(Failure::data)?;
    let task = spec
        .cid()
        .map_err(|error| Failure::data(error.to_string()))?;
    let key = spec
        .derivation_key(&projected)
        .map_err(|error| Failure::data(error.to_string()))?;
    let me = AgentRef(
        args.device
            .clone()
            .unwrap_or_else(|| UNIDENTIFIED_AGENT.to_string()),
    );
    let status = StatusFile(args.status.clone());
    install_interrupt_handlers()?;
    status.append(&PendingStatus::Queued)?;

    // 2. The device membrane, then the floor, against the device's own offer.
    let berth = berth(&args)?;
    if let Err(refused) = carried_here(&spec, &material, &me, &berth, &args.data_root) {
        eprintln!("ark task: not carried here: {:?}", refused.why);
        status.append(&PendingStatus::Unanswered(
            (*refused).map_why(|_| DerivationUnanswered::NotCarried),
        ))?;
        return Ok(EXIT_NOT_CARRIED);
    }

    // 3. The derivation, from brit.
    let index = BritIndex {
        build_ref: resolve_build_ref(),
        repo: args.repo.clone(),
        step: step.clone(),
        inputs_hash: material.inputs_hash(),
        input: spec.input().clone(),
        projected,
        method: spec.method().clone(),
        ctx: reuse_context(args.reuse.as_deref())?,
        last: Mutex::new(None),
    };
    let me_requester = Requester::authenticated(me.0.clone(), KeyCustody::DeviceHeld);
    let answer =
        futures::executor::block_on(index.lookup(&me_requester, spec.input(), spec.method()));
    let note = index
        .last
        .lock()
        .ok()
        .and_then(|mut last| last.take())
        .unwrap_or_default();
    let held = note.held;
    match answer {
        Ok(DerivationLookup::Found(records)) => {
            if let (Some(record), Some(row)) = (records.into_iter().next(), note.row) {
                let commit: String = row
                    .commit
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(9)
                    .collect();
                eprintln!(
                    "ark task: derivation hit, method {} input {}: green attestation at {commit} \
                     (signer this workspace); no child runs",
                    key.method,
                    key.input.cid(),
                );
                let completion = TaskCompletion::Local {
                    device: record.computed_by.clone(),
                    at: row.attestation.built_at_secs(),
                };
                status.append(&PendingStatus::Answered(TaskOutcome::reused(
                    task, record, completion,
                )))?;
                return Ok(0);
            }
        }
        Ok(DerivationLookup::NotRecorded) => {
            eprintln!(
                "ark task: not reused: {}",
                note.why.as_deref().unwrap_or("not recorded")
            );
        }
        Err(_) => eprintln!(
            "ark task: derivation index unreachable ({}); running here",
            note.why.as_deref().unwrap_or("unknown")
        ),
    }

    // 4. The child, under the native driver.
    let child = spec.child_spec();
    let started = NativeDriver
        .start(&child, &berth)
        .map_err(|error| match error {
            DriverError::ArtifactHashMismatch { .. } => Failure {
                code: EXIT_ARTIFACT,
                message: error.to_string(),
            },
            other => Failure::data(other.to_string()),
        })?;
    let pid = started.pid;
    let pumps = [
        pump(started.stdout, || -> Box<dyn Write> {
            Box::new(io::stdout())
        }),
        pump(started.stderr, || -> Box<dyn Write> {
            Box::new(io::stderr())
        }),
    ];
    let (exit, interrupted) = supervise(pid)?;
    let deadline = interrupted.map(|_| Instant::now() + DRAIN_AFTER_INTERRUPT);
    for pump in pumps {
        while deadline.is_some_and(|at| Instant::now() < at) && !pump.is_finished() {
            thread::sleep(Duration::from_millis(20));
        }
        if deadline.is_none() || pump.is_finished() {
            let _ = pump.join();
        }
    }

    if let Some(signal) = interrupted {
        let meanwhile = if held {
            Meanwhile::UseHeld {
                what: format!(
                    "the attestation already recorded for this closure \
                     (brit-build-ref build find --step {step} --inputs-hash {})",
                    material.inputs_hash()
                ),
            }
        } else {
            Meanwhile::Nothing {
                because: "no verdict exists for this closure yet, and an interrupted run is \
                          never one"
                    .into(),
            }
        };
        status.append(&PendingStatus::Unanswered(Unanswered::new(
            DerivationUnanswered::NotCarried,
            ComeBack::OnEvent {
                event: "rerun-requested".into(),
            },
            meanwhile,
        )))?;
        eprintln!("ark task: interrupted by signal {signal}; no verdict");
        return Ok(128u8.saturating_add(signal as u8));
    }

    let completion = TaskCompletion::Local {
        device: me.0.clone(),
        at: epoch_secs(),
    };
    let outcome = TaskOutcome::new(task, key, exit, None, None, completion)
        .map_err(|error| Failure::data(error.to_string()))?;
    status.append(&PendingStatus::Answered(outcome))?;
    Ok(exit_code(&exit))
}

/// The berth the child runs in: the device's own, holding the pinned artifact.
fn berth(args: &RunArgs) -> Result<Berth, Failure> {
    let manifest = device_manifest(&args.data_root)?;
    Ok(Berth {
        manifest: manifest
            .cid()
            .map_err(|error| Failure::data(error.to_string()))?,
        data_root: args.data_root.clone(),
        artifacts: BTreeMap::from([(TASK_CHILD_NAME.to_string(), args.artifact.clone())]),
        ..Berth::default()
    })
}

/// The manifest the device's own berth declares for a task: no children of its own, and an
/// envelope whose disk bound is what the data root's filesystem has free, read now. A
/// declaration, not enforcement (`EffectiveTier` stays `None`).
fn device_manifest(data_root: &Path) -> Result<RuntimeManifest, Failure> {
    let stat = nix::sys::statvfs::statvfs(data_root)
        .map_err(|error| Failure::data(format!("statvfs {}: {error}", data_root.display())))?;
    let free = stat.blocks_available().saturating_mul(stat.fragment_size());
    Ok(RuntimeManifest {
        envelope: Some(RuntimeEnvelope {
            bound: ResourceQuota {
                disk_bytes: Some(free),
                ..ResourceQuota::default()
            },
            ..RuntimeEnvelope::default()
        }),
        ..RuntimeManifest::default()
    })
}

/// Steps 2 of the flow: the device membrane offers tasks, and the floor admits this need on the
/// device's own offer (consent `NotNeeded`, the own-envelope scope, the material's size within
/// the declared disk bound). A refusal is the floor's own, with its three parts.
fn carried_here(
    spec: &TaskSpec,
    material: &Material,
    me: &AgentRef,
    berth: &Berth,
    data_root: &Path,
) -> Result<(), Box<Unanswered<placement_core::IneligibleReason>>> {
    let now = epoch_secs();
    let not_known =
        |input: &'static str| placement_core::IneligibleReason::Unknown { input }.unanswered(now);
    // The person running their own gate has chosen to run it now, so the battery default (which
    // withholds carrying for others) is lifted for this device's own task.
    let plan = decide_private(&PrivateSnapshot {
        facts: DeviceFacts {
            on_battery: on_battery(),
            metered: false,
            always_on: false,
            can_steward: false,
            can_infer: false,
            budget_bytes: 0,
        },
        choices: PersonChoices {
            will_carry: BTreeSet::from([KindId::Task]),
            lift_battery_default: true,
            budget_bytes: None,
        },
        needs: Vec::new(),
        me: me.clone(),
        reach: Reach::Private,
        now,
        params: None,
    });
    if let Some((_, refused)) = plan
        .refusals
        .into_iter()
        .find(|(kind, _)| *kind == KindId::Task)
    {
        return Err(Box::new(refused));
    }
    let manifest = device_manifest(data_root).map_err(|_| not_known("capacity"))?;
    let (berth_cid, _) = BlobCid::encode_manifest(berth).map_err(|_| not_known("berth"))?;
    let passport = Passport {
        schema: 1,
        kind: PASSPORT_KIND.to_string(),
        manifest: berth.manifest.clone(),
        node: None,
        incarnation: berth.incarnation,
        ark_version: env!("CARGO_PKG_VERSION").to_string(),
        processes: Vec::new(),
        follows: Vec::new(),
        tools: Vec::new(),
        last_verdict: None,
        updated_at_epoch_ms: now.saturating_mul(1000),
    };
    let offer = BerthOffer::from_berth(
        berth,
        *berth_cid.as_cid(),
        me.clone(),
        &manifest,
        &passport,
        Vec::new(),
        BTreeSet::from([GrantScope::OwnEnvelope]),
    );
    let receiver = AgentRefKey::of(me);
    let candidate = Candidate {
        posture: offer.posture(Reach::Private, now),
        domain: FailureDomain::of(None, me),
        live: Answer::Present(true),
        admitted: BTreeMap::from([(receiver.clone(), Answer::Present(Reach::Private))]),
        consent: BTreeMap::from([(receiver, Answer::Present(Consent::NotNeeded))]),
    };
    let space = elohim_epr::cid::compute_cid(format!("device:{}", me.0).as_bytes());
    let need = spec
        .as_need(
            me.clone(),
            space,
            Reach::Private,
            NeedClass::Member,
            None,
            material.byte_len(),
        )
        .map_err(|_| not_known("task-spec"))?;
    let kind = TaskKind::new()
        .with_spec(spec, Some(material.byte_len()))
        .map_err(|_| not_known("task-spec"))?;
    let remaining = Capacity::of_offer(&candidate.posture.offers[&KindId::Task]);
    admit(
        &kind,
        &need,
        &candidate,
        Some(&candidate.domain),
        &remaining,
        now,
    )
    .map(|_| ())
    .map_err(|ineligible| Box::new(ineligible.why))
}

/// Any power supply reporting `Discharging`; a host without one is not on battery.
fn on_battery() -> bool {
    let Ok(supplies) = fs::read_dir("/sys/class/power_supply") else {
        return false;
    };
    supplies.flatten().any(|supply| {
        fs::read_to_string(supply.path().join("status"))
            .is_ok_and(|status| status.trim() == "Discharging")
    })
}

fn reuse_context(path: Option<&Path>) -> Result<ReuseContext, Failure> {
    match path {
        None => Ok(ReuseContext {
            mode: None,
            enabled: false,
            self_id: None,
            dirty: None,
            commit: "HEAD".into(),
        }),
        Some(path) => serde_json::from_str(&read(path, "reuse context")?)
            .map_err(|error| Failure::data(format!("reuse context: {error}"))),
    }
}

/// Copies a child pipe to this process's stream byte for byte, flushing each read, until EOF.
/// A closed destination stops the copy but not the read, so the child never blocks on a full
/// pipe.
fn pump(mut from: impl Read + Send + 'static, to: fn() -> Box<dyn Write>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut out = to();
        let mut open = true;
        let mut buffer = [0u8; 8192];
        loop {
            match from.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if open {
                        open = out
                            .write_all(&buffer[..read])
                            .and_then(|()| out.flush())
                            .is_ok();
                    }
                }
            }
        }
    })
}

/// Waits for the child. On an interrupt, the signal goes to every descendant (this process is
/// their subreaper, so orphans are found too), and the tree is killed after
/// [`INTERRUPT_GRACE`]. Returns the exit and the interrupt, if one arrived.
fn supervise(pid: u32) -> Result<(ExitClass, Option<i32>), Failure> {
    let mut forwarded: Option<Instant> = None;
    let mut killed = false;
    loop {
        match wait_nowait(pid) {
            Ok(WaitEvent::Exited { .. }) => {
                let (exit, _) = reap_with_rusage(pid)
                    .map_err(|error| Failure::data(format!("reaping the task: {error}")))?;
                let signal = INTERRUPT.load(Ordering::SeqCst);
                if signal != 0 {
                    signal_descendants(Signal::SIGTERM);
                }
                return Ok((exit, (signal != 0).then_some(signal)));
            }
            Ok(WaitEvent::StillRunning) => reap_orphans(pid),
            Err(error) => return Err(Failure::data(format!("waiting for the task: {error}"))),
        }
        let signal = INTERRUPT.load(Ordering::SeqCst);
        if signal != 0 {
            match forwarded {
                None => {
                    let signal = Signal::try_from(signal).unwrap_or(Signal::SIGTERM);
                    signal_descendants(signal);
                    forwarded = Some(Instant::now());
                }
                Some(since) if !killed && since.elapsed() >= INTERRUPT_GRACE => {
                    signal_descendants(Signal::SIGKILL);
                    killed = true;
                }
                Some(_) => {}
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Reaps the orphaned descendants this subreaper has inherited and that have exited, leaving the
/// task's own child for [`supervise`]: a long gate spawns thousands of short processes (every
/// `git` its tests run), and unreaped they would hold a pid each until ark exits.
fn reap_orphans(task: u32) {
    let peek = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
    while let Ok(status) = waitid(Id::All, peek) {
        match status.pid() {
            Some(pid) if pid.as_raw() as u32 != task => {
                let _ = waitpid(pid, None);
            }
            _ => break,
        }
    }
}

/// Signals every live descendant of this process, read from `/proc`.
fn signal_descendants(signal: Signal) {
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            // `pid (comm) state ppid …`: the command may hold spaces and parentheses.
            let ppid = stat
                .rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().nth(1))
                .and_then(|ppid| ppid.parse::<u32>().ok());
            if let Some(ppid) = ppid {
                children.entry(ppid).or_default().push(pid);
            }
        }
    }
    let mut stack = children
        .get(&std::process::id())
        .cloned()
        .unwrap_or_default();
    while let Some(pid) = stack.pop() {
        let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), signal);
        stack.extend(children.get(&pid).into_iter().flatten().copied());
    }
}

extern "C" fn record_interrupt(signal: libc::c_int) {
    INTERRUPT.store(signal, Ordering::SeqCst);
}

/// SIGINT and SIGTERM are recorded, never acted on in the handler; this process also becomes the
/// subreaper of its descendants, so an interrupt reaches the whole tree.
fn install_interrupt_handlers() -> Result<(), Failure> {
    let action = SigAction::new(
        SigHandler::Handler(record_interrupt),
        SaFlags::empty(),
        SigSet::empty(),
    );
    // SAFETY: the installed handler has C linkage and performs only an atomic store.
    let installed = unsafe {
        sigaction(Signal::SIGTERM, &action).and_then(|_| sigaction(Signal::SIGINT, &action))
    };
    installed.map_err(|error| Failure::data(format!("installing signal handlers: {error}")))?;
    become_subreaper().map_err(|error| Failure::data(error.to_string()))
}

/// The child's exit as this process's: its code, or 128 plus the signal that ended it.
fn exit_code(exit: &ExitClass) -> u8 {
    match exit {
        ExitClass::Exited { code } => (*code & 0xff) as u8,
        ExitClass::Signaled { signal, .. } => 128u8.saturating_add(*signal as u8),
        ExitClass::OomKilled => 128 + 9,
        ExitClass::Unknown => 1,
    }
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn read(path: &Path, kind: &str) -> Result<String, Failure> {
    fs::read_to_string(path)
        .map_err(|error| Failure::data(format!("reading {kind} {}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exit_is_the_childs() {
        assert_eq!(exit_code(&ExitClass::Exited { code: 0 }), 0);
        assert_eq!(exit_code(&ExitClass::Exited { code: 101 }), 101);
        assert_eq!(
            exit_code(&ExitClass::Signaled {
                signal: 15,
                core_dumped: false
            }),
            143
        );
    }
}
