//! The supervision loop: one thread per declared child, every transition through `step`.
//!
//! The loop owns no policy. [`ark_core::lifecycle::step`] decides what a state and an event
//! become, and this module does exactly two things the pure machine cannot: it supplies the
//! events (a clock, a `/proc` read, a pipe, a `waitid`) and it performs the [`Action`]s the
//! machine asks for. Where `step` is deliberately clockless or policy-blind, the stamping
//! happens here and nowhere else — a `Dying` state gets its real `since_epoch_ms`, and a
//! `Stop` intent gets the grace period the child's own [`ChildPolicy`] declares.
//!
//! **Order is the contract.** A witness reaches disk BEFORE the governor is asked anything,
//! so a crash between the death and the decision still leaves the death witnessed. The tally
//! records the death BEFORE the governor reads it, because the governor's context is defined
//! to already contain the death being judged. An intent reaches disk BEFORE the action it
//! names. None of that is an optimisation to be reordered later; it is what makes the spool's
//! contents true after a crash rather than merely usual.
//!
//! **Sharing.** The sink lives behind an `Arc<Mutex<…>>` rather than a writer thread fed by
//! channels. [`WitnessSink::witness`] returns the CID its caller must have before it can
//! advance — the incident's witness list and the restart bookkeeping are both built from it —
//! so a channel would need a reply channel per write, which is a mutex with extra parts and
//! an extra failure mode. The lock is held for one small file write and never across a sleep,
//! a poll, or a spawn.

use std::{
    collections::VecDeque,
    fs::{DirBuilder, File, OpenOptions},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ark_core::{
    berth::Berth,
    lifecycle::{step, Action, ChildState, Event, IncidentCloseKind},
    manifest::{ChildSpec, ManifestError, Probe, RuntimeManifest},
    rea::ReaProjectionError,
    BoundedBy, Clock, DeathRecord, DeathTally, DeathWitness, ExitClass, GiveUpReason, Incident,
    IncidentClose, Intent, IntentAction, Passport, ProcessPassport, ProcessSample, RestartContext,
    RestartGovernor, RestartGrant, RestartRequest, RestartVerdict, RuntimeScope, SinkError,
    WitnessSink, PASSPORT_KIND, WITNESS_KIND,
};
use elohim_epr_rea::model::atom_cid;

use crate::{
    driver::{Driver, DriverError},
    pipes::{spawn_line_reader, StreamTap},
    reaper::{
        become_subreaper, proc_status_sample, reap_with_rusage, wait_nowait, ReapError, WaitEvent,
    },
};

/// Schema version carried by the records this loop writes.
const RECORD_SCHEMA: u32 = 1;
/// Interval between readiness polls while a child climbs its boot ladder.
const BOOT_POLL: Duration = Duration::from_millis(100);
/// Interval between liveness polls once a child is ready.
const LIVE_POLL: Duration = Duration::from_millis(250);
/// Interval between polls while a child is expected to terminate.
const DYING_POLL: Duration = Duration::from_millis(100);
/// Granularity at which a backoff sleep re-checks the shutdown flag.
const SLEEP_TICK: Duration = Duration::from_millis(50);
/// Budget for one TCP readiness attempt against a declared local port.
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
/// Grace given to the pipe readers to drain after a child dies, before its tail is read.
///
/// The readers are NOT joined: a child that forked a grandchild holding the write end would
/// keep the pipe open, and an unbounded join would hang the supervision of a dead process.
const PIPE_DRAIN_GRACE: Duration = Duration::from_millis(50);
/// Deadline used where a declared duration is too large for the monotonic clock to hold.
///
/// An hour rather than "forever": a manifest that declares an unrepresentable patience or
/// backoff has declared an error, and the loop's job is to keep supervising rather than to
/// honour the number or to die on it.
const OVERFLOW_DEADLINE: Duration = Duration::from_secs(3_600);
/// Band edge, as a percentage of the intensity ceiling, carried on the declared bound.
const INTENSITY_THRESHOLD_PCT: f64 = 80.0;
/// Directory and file modes for everything this loop creates.
const DIRECTORY_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

type SharedSink = Arc<Mutex<Box<dyn WitnessSink + Send>>>;
type SharedClock = Arc<dyn Clock + Send + Sync>;

/// Receives the process state associated with every durable passport rewrite.
pub trait StateObserver: Send + Sync {
    /// Observes one process state after its passport has reached the sink.
    fn on_state(&self, process: &str, state: &str, pid: Option<u32>, incarnation: u64);
}

/// The host clock, which is the only clock outside tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or_default()
    }
}

/// How a supervised run ended.
#[derive(Clone, Debug, PartialEq)]
pub struct RunOutcome {
    /// `0` when the run stopped cleanly; `3` when a declared process was permanently
    /// abandoned.
    pub exit_code: i32,
    /// The berth passport as it stood when the last thread finished.
    pub passport: Passport,
}

/// A refusal or failure of supervision itself, as distinct from a child's death.
#[derive(thiserror::Error, Debug)]
pub enum SupervisorError {
    /// The berth names no `data_root`, so the spool has nowhere to live.
    #[error("berth declares no data_root: the spool has nowhere to live")]
    BerthWithoutDataRoot,
    /// The berth's manifest field is not a content address.
    #[error("berth manifest {manifest} is not a CID")]
    BerthManifestNotACid {
        /// The uninterpretable string the berth carried.
        manifest: String,
    },
    /// The berth is placed for a different manifest than the one supplied.
    #[error(
        "berth is placed for manifest {berth}, but the supplied manifest addresses to {manifest}"
    )]
    BerthManifestMismatch {
        /// The CID the berth declares.
        berth: String,
        /// The CID the supplied manifest actually has.
        manifest: String,
    },
    /// A declared process has no local artifact in the berth.
    #[error(
        "process {process} is declared by the manifest, but the berth places no artifact for it"
    )]
    ArtifactNotPlaced {
        /// The declared process with no placement.
        process: String,
    },
    /// The manifest could not be addressed.
    #[error("manifest identity: {0}")]
    Manifest(#[from] ManifestError),
    /// A record could not be projected into the substrate vocabulary.
    #[error("projection: {0}")]
    Projection(#[from] ReaProjectionError),
    /// The spool refused a write or a read.
    #[error("spool: {0}")]
    Sink(#[from] SinkError),
    /// A child could not be started.
    #[error("process {process}: {source}")]
    Driver {
        /// The process being started.
        process: String,
        /// The driver's refusal.
        source: DriverError,
    },
    /// A child's death could not be learned of or consumed.
    #[error("process {process}: {source}")]
    Reap {
        /// The process being waited on.
        process: String,
        /// The reaper's failure.
        source: ReapError,
    },
    /// This process could not claim its orphaned descendants.
    #[error("becoming a subreaper: {0}")]
    Subreaper(ReapError),
    /// A supervision file could not be created or opened.
    #[error("{path}: {message}")]
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The underlying error text.
        message: String,
    },
    /// A supervision thread panicked, which leaves its child's state unknown.
    #[error("supervision of {process} panicked")]
    Panicked {
        /// The process whose thread panicked.
        process: String,
    },
}

/// Runs one [`RuntimeManifest`] in one [`Berth`] until its processes stop.
pub struct Supervisor {
    manifest: RuntimeManifest,
    berth: Berth,
    scope: RuntimeScope,
    driver: Arc<dyn Driver>,
    sink: SharedSink,
    clock: SharedClock,
    shutdown: Arc<AtomicBool>,
    observer: Option<Arc<dyn StateObserver>>,
}

impl Supervisor {
    /// Validates a manifest/berth pairing and builds the one scope every record is
    /// accountable to.
    ///
    /// Called by [`Supervisor::new`], and callable BEFORE it by whoever opens the spool —
    /// the spool needs the same scope, and the refusals belong in one place rather than
    /// duplicated at every construction site.
    pub fn scope_for(
        manifest: &RuntimeManifest,
        berth: &Berth,
    ) -> Result<RuntimeScope, SupervisorError> {
        if berth.data_root.as_os_str().is_empty() {
            return Err(SupervisorError::BerthWithoutDataRoot);
        }

        let scope =
            RuntimeScope::from_berth(berth).map_err(|_| SupervisorError::BerthManifestNotACid {
                manifest: berth.manifest.clone(),
            })?;

        let declared = manifest.cid()?;
        if berth.manifest != declared {
            return Err(SupervisorError::BerthManifestMismatch {
                berth: berth.manifest.clone(),
                manifest: declared,
            });
        }

        for process in &manifest.processes {
            if !berth.artifacts.contains_key(&process.name) {
                return Err(SupervisorError::ArtifactNotPlaced {
                    process: process.name.clone(),
                });
            }
        }

        Ok(scope)
    }

    /// Builds a supervisor, refusing any manifest/berth pairing that could not be run.
    pub fn new(
        manifest: RuntimeManifest,
        berth: Berth,
        driver: Box<dyn Driver>,
        sink: Box<dyn WitnessSink + Send>,
        clock: Box<dyn Clock + Send + Sync>,
    ) -> Result<Self, SupervisorError> {
        let scope = Self::scope_for(&manifest, &berth)?;
        Ok(Self {
            manifest,
            berth,
            scope,
            driver: Arc::from(driver),
            sink: Arc::new(Mutex::new(sink)),
            clock: Arc::from(clock),
            shutdown: Arc::new(AtomicBool::new(false)),
            observer: None,
        })
    }

    /// Adds an observer for state-bearing passport rewrites.
    pub fn with_observer(mut self, observer: Arc<dyn StateObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    /// The flag a signal handler sets to ask every child to stop.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Supervises every declared process until each one stops, gives up, or is stopped.
    pub fn run(mut self) -> Result<RunOutcome, SupervisorError> {
        // Claims this process's orphaned descendants, so a conductor's grandchildren are
        // re-parented HERE rather than to pid 1 and lost. It does not reap them: an adopted
        // orphan stays a zombie for the life of the run, because the only thing that would
        // harvest it is a blind `waitpid(-1)` — and a blind wait would consume a SUPERVISED
        // child's status ahead of `reap_with_rusage`, destroying the rusage its witness is
        // made of. The orphan reaper this gap needs filters by supervised pid, so that it can
        // harvest what nobody is waiting on and touch nothing that is being witnessed; that
        // is S1 work.
        become_subreaper().map_err(SupervisorError::Subreaper)?;

        let incarnation = self.bump_incarnation()?;
        self.berth.incarnation = incarnation;
        let passport = Arc::new(Mutex::new(self.initial_passport(incarnation)?));
        write_passport(&self.sink, &passport)?;
        if let Some(observer) = &self.observer {
            for process in &self.manifest.processes {
                observer.on_state(&process.name, "idle", None, incarnation);
            }
        }

        let logs = self.berth.data_root.join("ark").join("logs");
        DirBuilder::new()
            .recursive(true)
            .mode(DIRECTORY_MODE)
            .create(&logs)
            .map_err(|error| SupervisorError::Io {
                path: logs.clone(),
                message: error.to_string(),
            })?;

        let mut threads = Vec::with_capacity(self.manifest.processes.len());
        for spec in &self.manifest.processes {
            let mut worker = Worker {
                spec: spec.clone(),
                berth: self.berth.clone(),
                scope: self.scope.clone(),
                incarnation,
                driver: Arc::clone(&self.driver),
                sink: Arc::clone(&self.sink),
                clock: Arc::clone(&self.clock),
                shutdown: Arc::clone(&self.shutdown),
                observer: self.observer.clone(),
                passport: Arc::clone(&passport),
                logs: logs.clone(),
                state: ChildState::Idle,
                running: None,
                incident: None,
                tally: DeathTally::default(),
                last_intent: None,
                pending_death: None,
                pending_witness: None,
                last_give_up_reason: None,
                grace_expired: false,
                gave_up: false,
            };
            let name = spec.name.clone();
            let shutdown = Arc::clone(&self.shutdown);
            threads.push((
                name.clone(),
                thread::Builder::new()
                    .name(format!("ark-{name}"))
                    .spawn(move || {
                        // Armed for the whole of `supervise`, including the unwind: a thread
                        // that panics leaves its child's state unknown, and `run` would
                        // otherwise block forever joining siblings nobody has asked to stop.
                        let mut guard = PanicGuard::new(shutdown);
                        let outcome = worker.supervise();
                        // One child that cannot be supervised ends the run: the manifest is a
                        // whole, and half a runtime is not a smaller success. A clean return
                        // disarms the guard — `GaveUp` is a decision, not a failure, and it
                        // leaves the siblings running.
                        if outcome.is_ok() {
                            guard.disarm();
                        }
                        outcome.map(|()| worker.gave_up)
                    })
                    .map_err(|error| SupervisorError::Io {
                        path: PathBuf::from(format!("thread ark-{name}")),
                        message: error.to_string(),
                    })?,
            ));
        }

        let mut abandoned = false;
        let mut failure = None;
        for (name, handle) in threads {
            match handle.join() {
                Ok(Ok(gave_up)) => abandoned |= gave_up,
                Ok(Err(error)) => failure = failure.or(Some(error)),
                Err(_) => {
                    failure = failure.or(Some(SupervisorError::Panicked {
                        process: name.clone(),
                    }))
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }

        let passport = passport.lock().expect("passport lock").clone();
        Ok(RunOutcome {
            // 3 says the ark did not deliver the manifest: a declared process was permanently
            // abandoned, and reporting 0 would tell the mesh a dead conductor is fine.
            exit_code: if abandoned { 3 } else { 0 },
            passport,
        })
    }

    /// Reads the previous passport and returns the incarnation this run occupies.
    fn bump_incarnation(&self) -> Result<u64, SupervisorError> {
        let previous = self
            .sink
            .lock()
            .expect("sink lock")
            .load_passport()?
            .map(|passport| passport.incarnation)
            .unwrap_or_default();
        Ok(previous.max(self.berth.incarnation).saturating_add(1))
    }

    fn initial_passport(&self, incarnation: u64) -> Result<Passport, SupervisorError> {
        let tier = self.driver.fingerprint().effective_tier;
        Ok(Passport {
            schema: RECORD_SCHEMA,
            kind: PASSPORT_KIND.to_string(),
            manifest: self.berth.manifest.clone(),
            node: self.berth.node.clone(),
            incarnation,
            ark_version: env!("CARGO_PKG_VERSION").to_string(),
            processes: self
                .manifest
                .processes
                .iter()
                .map(|spec| ProcessPassport {
                    name: spec.name.clone(),
                    artifact_sha256: String::new(),
                    artifact_path: String::new(),
                    pid: None,
                    started_at_epoch_ms: None,
                    ready: false,
                    effective_tier: tier.clone(),
                    deaths_in_window: 0,
                })
                .collect(),
            last_verdict: None,
            updated_at_epoch_ms: self.clock.now_epoch_ms(),
        })
    }
}

/// What one child's supervision loop should do after applying an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Continue,
    Exit,
}

/// Raises the shutdown flag unless it is disarmed, including when its thread unwinds.
///
/// A supervision thread that panics leaves its child's state unknown, and the manifest is a
/// whole: siblings still running under an unknown half are worse than a runtime that stopped.
/// Without this, `run` would block forever joining siblings nobody had asked to stop.
struct PanicGuard {
    shutdown: Arc<AtomicBool>,
    armed: bool,
}

impl PanicGuard {
    fn new(shutdown: Arc<AtomicBool>) -> Self {
        Self {
            shutdown,
            armed: true,
        }
    }

    /// The clean-exit path: a thread that returned — `GaveUp` included — leaves siblings alone.
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PanicGuard {
    fn drop(&mut self) {
        if self.armed {
            self.shutdown.store(true, Ordering::SeqCst);
        }
    }
}

/// A deadline `after` from now, clamped rather than panicking.
///
/// `Instant + Duration` panics on overflow, and every duration reaching this function is
/// manifest-declared (`patience_ms`, backoff seconds). A number somebody wrote in a JSON file
/// must never be able to kill the loop whose whole job is to witness deaths.
fn deadline_after(after: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(after)
        .or_else(|| now.checked_add(OVERFLOW_DEADLINE))
        .unwrap_or(now)
}

/// A child that is currently running, with the taps reading its output.
struct Running {
    pid: u32,
    stdout: StreamTap,
    stderr: StreamTap,
    artifact_sha256: String,
    artifact_path: PathBuf,
    started_at_epoch_ms: u64,
}

/// A death that has been observed and consumed, awaiting its witness.
struct PendingDeath {
    pid: u32,
    class: ExitClass,
    sample: ProcessSample,
    artifact_sha256: String,
    artifact_path: PathBuf,
    started_at_epoch_ms: u64,
    died_at_epoch_ms: u64,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

/// The supervision of one declared child.
struct Worker {
    spec: ChildSpec,
    berth: Berth,
    scope: RuntimeScope,
    incarnation: u64,
    driver: Arc<dyn Driver>,
    sink: SharedSink,
    clock: SharedClock,
    shutdown: Arc<AtomicBool>,
    observer: Option<Arc<dyn StateObserver>>,
    passport: Arc<Mutex<Passport>>,
    logs: PathBuf,

    state: ChildState,
    running: Option<Running>,
    incident: Option<Incident>,
    tally: DeathTally,
    last_intent: Option<Intent>,
    pending_death: Option<PendingDeath>,
    pending_witness: Option<DeathWitness>,
    last_give_up_reason: Option<GiveUpReason>,
    grace_expired: bool,
    gave_up: bool,
}

impl Worker {
    /// Drives `step` until this child reaches a state with nothing left to observe.
    fn supervise(&mut self) -> Result<(), SupervisorError> {
        let mut events: VecDeque<Event> = VecDeque::from([Event::SpawnRequested]);
        loop {
            let event = match events.pop_front() {
                Some(event) => event,
                None => match self.poll()? {
                    Some(event) => event,
                    None => return Ok(()),
                },
            };

            let (next, actions) = step(self.state.clone(), event);
            self.state = self.stamp(next);

            // One batch, one kill intent: `step` records the intent itself where it knows a
            // kill is coming (a rung that ran out of patience), and stays silent where the
            // kill is the grace period expiring. The write-ahead rule holds either way.
            let mut kill_recorded = actions
                .iter()
                .any(|action| matches!(action, Action::RecordIntent(IntentAction::Kill)));
            for action in actions {
                if self.apply(action, &mut events, &mut kill_recorded)? == Flow::Exit {
                    return Ok(());
                }
            }
        }
    }

    /// Stamps the wall-clock instant `step` deliberately does not know.
    fn stamp(&mut self, state: ChildState) -> ChildState {
        match state {
            ChildState::Dying {
                pid,
                since_epoch_ms: 0,
            } => {
                self.grace_expired = false;
                ChildState::Dying {
                    pid,
                    since_epoch_ms: self.now_ms(),
                }
            }
            other => other,
        }
    }

    /// Observes the world until it yields the next event, or `None` when the child's
    /// supervision is over.
    fn poll(&mut self) -> Result<Option<Event>, SupervisorError> {
        match self.state.clone() {
            // Terminal: `Idle` is only re-entered after a stop, and `GaveUp` is permanent.
            ChildState::Idle | ChildState::GaveUp => Ok(None),
            // Transient: the follow-up event is always enqueued by the action that made them.
            ChildState::Spawning { .. } | ChildState::Dead => Ok(None),
            ChildState::Booting { pid, rung } => self.poll_booting(pid, rung),
            ChildState::Live { pid } => self.poll_live(pid),
            ChildState::Dying {
                pid,
                since_epoch_ms,
            } => self.poll_dying(pid, since_epoch_ms),
        }
    }

    /// Climbs one rung, preferring an observed death over the rung's patience budget.
    fn poll_booting(&mut self, pid: u32, rung: usize) -> Result<Option<Event>, SupervisorError> {
        let of = self.spec.readiness.len();
        let Some(probe) = self.spec.readiness.get(rung).cloned() else {
            // A child that declares no readiness probe makes no readiness claim, so the ark
            // never asserts one on its behalf — and never resets its death window either.
            // It is supervised for death and for shutdown, and nothing else.
            return self.poll_live(pid);
        };

        let patience = Duration::from_millis(match &probe {
            Probe::StdoutLine { patience_ms, .. } | Probe::TcpListen { patience_ms, .. } => {
                *patience_ms
            }
        });
        let deadline = deadline_after(patience);

        loop {
            // Death BEFORE shutdown, always. A child that died while this thread slept is
            // dead however the flag now stands, and reading the flag first would launder that
            // death into a clean stop: no witness on disk, no verdict, and an incident closed
            // as if the ark had meant it. A stop we asked for can wait one more poll; a death
            // nobody witnessed is gone.
            if let Some(event) = self.check_death(pid)? {
                return Ok(Some(event));
            }
            if let Some(event) = self.shutdown_event() {
                return Ok(Some(event));
            }
            if self.probe_satisfied(&probe) {
                return Ok(Some(Event::RungPassed { rung, of }));
            }
            if Instant::now() >= deadline {
                return Ok(Some(Event::RungTimedOut { rung }));
            }
            thread::sleep(BOOT_POLL);
        }
    }

    /// Watches a running child for its death or for a shutdown request.
    fn poll_live(&mut self, pid: u32) -> Result<Option<Event>, SupervisorError> {
        loop {
            // Death before shutdown, for the reason `poll_booting` states: the flag is a
            // request, the death already happened, and only one of the two can be lost.
            if let Some(event) = self.check_death(pid)? {
                return Ok(Some(event));
            }
            if let Some(event) = self.shutdown_event() {
                return Ok(Some(event));
            }
            thread::sleep(LIVE_POLL);
        }
    }

    /// Waits out the grace period, then reports it expired exactly once.
    fn poll_dying(
        &mut self,
        pid: u32,
        since_epoch_ms: u64,
    ) -> Result<Option<Event>, SupervisorError> {
        let grace = self.spec.policy.shutdown.grace_ms;
        loop {
            if let Some(event) = self.check_death(pid)? {
                return Ok(Some(event));
            }
            if !self.grace_expired && self.now_ms().saturating_sub(since_epoch_ms) >= grace {
                self.grace_expired = true;
                return Ok(Some(Event::GraceExpired));
            }
            thread::sleep(DYING_POLL);
        }
    }

    /// Learns of a death without consuming it, then consumes it with its accounting.
    fn check_death(&mut self, pid: u32) -> Result<Option<Event>, SupervisorError> {
        let observed = wait_nowait(pid).map_err(|source| SupervisorError::Reap {
            process: self.spec.name.clone(),
            source,
        })?;
        let WaitEvent::Exited { sample, .. } = observed else {
            return Ok(None);
        };

        // The zombie's `/proc` first, because `wait4` frees the pid for reuse the instant it
        // returns; `rusage` second, because a zombie has neither peak RSS nor CPU time.
        let live = proc_status_sample(pid).unwrap_or(sample);
        let (class, accounted) = reap_with_rusage(pid).map_err(|source| SupervisorError::Reap {
            process: self.spec.name.clone(),
            source,
        })?;

        thread::sleep(PIPE_DRAIN_GRACE);
        let tail = self.spec.listen.tail_lines;
        let (stdout_tail, stderr_tail) = match &self.running {
            Some(running) => (
                running
                    .stdout
                    .ring
                    .lock()
                    .expect("stdout ring")
                    .last_n(tail),
                running
                    .stderr
                    .ring
                    .lock()
                    .expect("stderr ring")
                    .last_n(tail),
            ),
            None => (Vec::new(), Vec::new()),
        };
        let (artifact_sha256, artifact_path, started_at_epoch_ms) = match &self.running {
            Some(running) => (
                running.artifact_sha256.clone(),
                running.artifact_path.clone(),
                running.started_at_epoch_ms,
            ),
            None => (String::new(), PathBuf::new(), 0),
        };

        self.running = None;
        // The passport says what is true NOW, and what is true now is that nothing is
        // running under this name — on every death path, including a deliberate stop, which
        // writes no witness at all.
        self.update_passport(|entry| {
            entry.pid = None;
            entry.ready = false;
        })?;
        self.pending_death = Some(PendingDeath {
            pid,
            class,
            sample: merge_samples(live, accounted),
            artifact_sha256,
            artifact_path,
            started_at_epoch_ms,
            died_at_epoch_ms: self.now_ms(),
            stdout_tail,
            stderr_tail,
        });
        Ok(Some(Event::Died { class }))
    }

    /// The stop request a raised shutdown flag becomes, carrying the child's own signal.
    fn shutdown_event(&self) -> Option<Event> {
        self.shutdown
            .load(Ordering::SeqCst)
            .then_some(Event::StopRequested {
                signal: self.spec.policy.shutdown.signal,
            })
    }

    fn probe_satisfied(&self, probe: &Probe) -> bool {
        match probe {
            Probe::StdoutLine { contains, .. } => self.running.as_ref().is_some_and(|running| {
                running
                    .stdout
                    .matched
                    .lock()
                    .expect("stdout matcher")
                    .iter()
                    .any(|needle| needle == contains)
            }),
            Probe::TcpListen { port_key, .. } => {
                self.berth.ports.get(port_key).is_some_and(|port| {
                    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, *port));
                    TcpStream::connect_timeout(&address, TCP_CONNECT_TIMEOUT).is_ok()
                })
            }
        }
    }

    /// Performs one [`Action`], enqueuing whatever event it produces.
    fn apply(
        &mut self,
        action: Action,
        events: &mut VecDeque<Event>,
        kill_recorded: &mut bool,
    ) -> Result<Flow, SupervisorError> {
        match action {
            Action::RecordIntent(action) => self.record_intent(action)?,
            Action::Spawn => {
                // A refusal propagates: a child that never started is a supervision failure,
                // not a death (see `spawn`).
                let pid = self.spawn()?;
                events.push_back(Event::Spawned { pid });
            }
            Action::OpenIncident => self.open_incident(),
            Action::WriteWitness => self.write_witness()?,
            Action::Decide => {
                let verdict = self.decide()?;
                events.push_back(Event::VerdictReached { verdict });
            }
            Action::SleepThen(seconds) => self.sleep_then(seconds),
            Action::SendSignal(signal) => self.signal(signal),
            Action::Kill => {
                if !*kill_recorded {
                    self.record_intent(IntentAction::Kill)?;
                    *kill_recorded = true;
                }
                self.signal(libc::SIGKILL);
            }
            Action::CloseIncident(kind) => self.close_incident(kind)?,
            Action::MarkReady => self.mark_ready()?,
            Action::Exit => return Ok(Flow::Exit),
        }
        Ok(Flow::Continue)
    }

    /// Writes the decision before the action it names, and remembers it for the next witness.
    fn record_intent(&mut self, action: IntentAction) -> Result<(), SupervisorError> {
        // `step` carries only the policy signal on a stop; the grace period is the child's
        // own policy and is stamped here, so the log says what will actually be waited out.
        let action = match action {
            IntentAction::Stop { signal, .. } => IntentAction::Stop {
                signal,
                grace_ms: self.spec.policy.shutdown.grace_ms,
            },
            other => other,
        };
        let intent = Intent {
            at_epoch_ms: self.now_ms(),
            incarnation: self.incarnation,
            process: self.spec.name.clone(),
            action: action.clone(),
            reason: self.reason_for(&action),
        };
        self.sink.lock().expect("sink lock").intent(&intent)?;

        // A restart is an output of the incident it recovers from: the atom appended to the
        // flow sidecar is the same one named here, so the projection and the log agree.
        if matches!(action, IntentAction::Restart { .. }) {
            if let Some(incident) = self.incident.as_mut() {
                let atom = atom_cid(&intent.as_rea_intent(&self.scope))
                    .map_err(|error| ReaProjectionError::Encode(error.to_string()))?;
                incident.restarts.push(atom.to_string());
            }
        }

        self.last_intent = Some(intent);
        Ok(())
    }

    fn reason_for(&self, action: &IntentAction) -> String {
        match action {
            IntentAction::Spawn => format!("first start of {} in this incarnation", self.spec.name),
            IntentAction::Restart { attempt, after_s } => {
                format!("restart attempt {attempt} after {after_s}s of backoff")
            }
            IntentAction::Stop { signal, grace_ms } => {
                format!("shutdown requested: signal {signal}, then {grace_ms}ms of grace")
            }
            IntentAction::Kill => {
                if self.grace_expired {
                    "grace period expired".to_string()
                } else {
                    "readiness patience exhausted".to_string()
                }
            }
            IntentAction::GiveUp => match &self.last_give_up_reason {
                Some(reason) => format!("restart policy gave up: {reason:?}"),
                None => "restart policy gave up".to_string(),
            },
        }
    }

    /// Starts the child, taps its output, and records what was actually executed.
    ///
    /// A driver refusal — at the first spawn or at a restart — is a supervision FAILURE and
    /// leaves as an `Err`, never as a death. Nothing ran, so there is no exit status to
    /// classify, no rusage to account, and no witness to write; calling it a death would put a
    /// fabricated exit in the spool. Judging a refusal the way a crash is judged (a restart
    /// policy that can give up on a child whose artifact has gone missing) needs `ExitClass` to
    /// grow a never-started `SpawnFailed { errno }` variant, which is S1 work.
    fn spawn(&mut self) -> Result<u32, SupervisorError> {
        let started = self
            .driver
            .start(&self.spec, &self.berth)
            .map_err(|source| SupervisorError::Driver {
                process: self.spec.name.clone(),
                source,
            })?;

        let needles: Vec<String> = self
            .spec
            .readiness
            .iter()
            .filter_map(|probe| match probe {
                Probe::StdoutLine { contains, .. } => Some(contains.clone()),
                Probe::TcpListen { .. } => None,
            })
            .collect();
        let ring_lines = self.spec.listen.ring_lines;
        let stdout_log = self.log_file("stdout")?;
        let stderr_log = self.log_file("stderr")?;

        let running = Running {
            pid: started.pid,
            stdout: spawn_line_reader(
                "stdout",
                started.stdout,
                ring_lines,
                Some(stdout_log),
                needles,
            ),
            stderr: spawn_line_reader(
                "stderr",
                started.stderr,
                ring_lines,
                Some(stderr_log),
                Vec::new(),
            ),
            artifact_sha256: started.artifact_sha256.clone(),
            artifact_path: started.artifact_path.clone(),
            started_at_epoch_ms: started.started_at_epoch_ms,
        };
        let pid = running.pid;
        self.running = Some(running);

        self.update_passport(|entry| {
            entry.pid = Some(started.pid);
            entry.started_at_epoch_ms = Some(started.started_at_epoch_ms);
            entry.ready = false;
            entry.artifact_sha256 = started.artifact_sha256.clone();
            entry.artifact_path = started.artifact_path.display().to_string();
        })?;
        Ok(pid)
    }

    fn log_file(&self, stream: &str) -> Result<File, SupervisorError> {
        let path = self.logs.join(format!("{}.{stream}.log", self.spec.name));
        OpenOptions::new()
            .create(true)
            .append(true)
            .mode(FILE_MODE)
            .open(&path)
            .map_err(|error| SupervisorError::Io {
                path,
                message: error.to_string(),
            })
    }

    /// Opens an incident only when this process has none open — repeated deaths inside one
    /// outage are one incident, which is what makes "gave up on the same cause" legible.
    fn open_incident(&mut self) {
        if self
            .incident
            .as_ref()
            .is_some_and(ark_core::Incident::is_open)
        {
            return;
        }
        self.incident = Some(Incident::open(
            &self.spec.name,
            self.now_ms(),
            self.incarnation,
        ));
    }

    /// The write-ahead witness: durable BEFORE anything is decided.
    fn write_witness(&mut self) -> Result<(), SupervisorError> {
        let Some(death) = self.pending_death.take() else {
            return Ok(());
        };
        let witness = self.build_witness(&death);
        let cid = self.sink.lock().expect("sink lock").witness(&witness)?;
        self.append_witness(cid)?;
        self.pending_witness = Some(witness);
        Ok(())
    }

    fn build_witness(&self, death: &PendingDeath) -> DeathWitness {
        let passport = self.passport.lock().expect("passport lock").clone();
        DeathWitness {
            schema: RECORD_SCHEMA,
            kind: WITNESS_KIND.to_string(),
            incident: self
                .incident
                .as_ref()
                .map(|incident| incident.id.clone())
                .unwrap_or_default(),
            process: self.spec.name.clone(),
            incarnation: self.incarnation,
            pid: death.pid,
            artifact_sha256: death.artifact_sha256.clone(),
            artifact_path: death.artifact_path.display().to_string(),
            started_at_epoch_ms: death.started_at_epoch_ms,
            died_at_epoch_ms: death.died_at_epoch_ms,
            uptime_ms: death
                .died_at_epoch_ms
                .saturating_sub(death.started_at_epoch_ms),
            exit: death.class,
            last_stderr: death.stderr_tail.clone(),
            last_stdout: death.stdout_tail.clone(),
            sample: Some(death.sample.clone()),
            last_intent: self.last_intent.clone(),
            passport,
            verdict: None,
            refusal: None,
            // S0 runs on manifest policy: no commitment bounds this runtime yet, and an
            // absent promise is recorded as absent rather than as a zero.
            bounded_by: None,
            pain: None,
        }
    }

    fn append_witness(&mut self, cid: String) -> Result<(), SupervisorError> {
        let Some(incident) = self.incident.as_mut() else {
            return Ok(());
        };
        incident.witnesses.push(cid);
        let incident = incident.clone();
        self.sink.lock().expect("sink lock").incident(&incident)?;
        Ok(())
    }

    /// Records the death, asks the governor, then re-writes the witness with its verdict.
    fn decide(&mut self) -> Result<RestartVerdict, SupervisorError> {
        let mut witness = match self.pending_witness.take() {
            Some(witness) => witness,
            // No witness means no death to judge; the pure machine only reaches `Decide`
            // after `WriteWitness`, so this is a stop rather than a silent restart loop.
            None => return Ok(RestartVerdict::Stop),
        };

        let now_epoch_s = witness.died_at_epoch_ms / 1_000;
        let death = DeathRecord {
            at_epoch_s: now_epoch_s,
            class: witness.exit,
            uptime_ms: witness.uptime_ms,
            // The oldest RETAINED stderr line: for a child that dies fast the ring holds
            // everything it ever said, which is exactly the case same-cause coalescing is for.
            first_stderr_line: witness.last_stderr.first().cloned(),
        };

        // The governor's context is defined to already contain the death being judged.
        self.tally.record(death.clone());
        let tally = self.tally.clone();
        self.sink
            .lock()
            .expect("sink lock")
            .tally(&self.spec.name, &tally)?;

        let grant = RestartGrant {
            bounded_by: BoundedBy::ManifestPolicy,
            policy: self.spec.policy.clone(),
            bound: Some(self.spec.policy.intensity_bound(INTENSITY_THRESHOLD_PCT)?),
        };
        let request = RestartRequest {
            process: self.spec.name.clone(),
            death,
        };
        let (verdict, refusal) = RestartGovernor.verdict(
            &request,
            &grant,
            &RestartContext {
                now_epoch_s,
                tally: tally.clone(),
            },
        );

        if let RestartVerdict::GiveUp { reason } = &verdict {
            witness.pain = grant.pain(reason);
            self.last_give_up_reason = Some(reason.clone());
            self.gave_up = true;
        }
        witness.verdict = Some(verdict.clone());
        witness.refusal = refusal;

        let cid = self.sink.lock().expect("sink lock").witness(&witness)?;
        self.append_witness(cid)?;

        let deaths_in_window =
            tally.deaths_within(now_epoch_s, self.spec.policy.intensity.window_s);
        self.update_passport(|entry| entry.deaths_in_window = deaths_in_window)?;
        self.set_last_verdict(verdict.clone())?;
        Ok(verdict)
    }

    /// Waits out a backoff, waking early when a shutdown is requested.
    ///
    /// Waking early does NOT cancel the spawn that follows. `step` emits `SleepThen` and
    /// `Spawn` as one batch, so a shutdown raised during a backoff spawns the child once and
    /// then stops it on the next poll — one short-lived child, correctly witnessed, rather than
    /// a loop that silently drops an action the pure machine already decided. Cancelling a
    /// batch mid-flight requires the machine to learn a cancellation event, which is S1 work.
    fn sleep_then(&self, seconds: u64) {
        let deadline = deadline_after(Duration::from_secs(seconds));
        while Instant::now() < deadline {
            if self.shutdown.load(Ordering::SeqCst) {
                return;
            }
            thread::sleep(SLEEP_TICK);
        }
    }

    /// Delivers a signal, tolerating a child that died between the decision and the delivery.
    ///
    /// A signal that cannot be delivered because the process is already gone is not a
    /// supervision failure: the death it was meant to cause has happened, and the next poll
    /// will observe it.
    fn signal(&self, signal: i32) {
        let Some(running) = self.running.as_ref() else {
            return;
        };
        let _ = self.driver.signal(running.pid, signal);
    }

    fn close_incident(&mut self, kind: IncidentCloseKind) -> Result<(), SupervisorError> {
        let at_epoch_ms = self.now_ms();
        let reason = self.last_give_up_reason.clone();
        let Some(incident) = self.incident.as_mut() else {
            return Ok(());
        };
        if !incident.is_open() {
            return Ok(());
        }
        incident.closed = Some(match kind {
            IncidentCloseKind::ReadyAgain => IncidentClose::ReadyAgain { at_epoch_ms },
            IncidentCloseKind::Stopped => IncidentClose::Stopped { at_epoch_ms },
            IncidentCloseKind::GaveUp => IncidentClose::GaveUp {
                at_epoch_ms,
                reason: reason.unwrap_or(GiveUpReason::PolicyTemporary),
            },
        });
        let incident = incident.clone();
        self.sink.lock().expect("sink lock").incident(&incident)?;
        Ok(())
    }

    /// Readiness is the claim that resets the death window: the child got all the way up.
    fn mark_ready(&mut self) -> Result<(), SupervisorError> {
        self.tally.reset_on_ready();
        let tally = self.tally.clone();
        self.sink
            .lock()
            .expect("sink lock")
            .tally(&self.spec.name, &tally)?;
        self.update_passport(|entry| {
            entry.ready = true;
            entry.deaths_in_window = 0;
        })
    }

    fn update_passport(
        &self,
        mutate: impl FnOnce(&mut ProcessPassport),
    ) -> Result<(), SupervisorError> {
        {
            let mut passport = self.passport.lock().expect("passport lock");
            if let Some(entry) = passport
                .processes
                .iter_mut()
                .find(|entry| entry.name == self.spec.name)
            {
                mutate(entry);
            }
            passport.updated_at_epoch_ms = self.clock.now_epoch_ms();
        }
        write_passport(&self.sink, &self.passport)?;
        self.notify_state();
        Ok(())
    }

    fn set_last_verdict(&self, verdict: RestartVerdict) -> Result<(), SupervisorError> {
        {
            let mut passport = self.passport.lock().expect("passport lock");
            passport.last_verdict = Some(verdict);
            passport.updated_at_epoch_ms = self.clock.now_epoch_ms();
        }
        write_passport(&self.sink, &self.passport)?;
        self.notify_state();
        Ok(())
    }

    fn notify_state(&self) {
        let Some(observer) = &self.observer else {
            return;
        };
        let pid = match self.state {
            ChildState::Booting { pid, .. }
            | ChildState::Live { pid }
            | ChildState::Dying { pid, .. } => Some(pid),
            ChildState::Idle
            | ChildState::Spawning { .. }
            | ChildState::Dead
            | ChildState::GaveUp => None,
        };
        observer.on_state(
            &self.spec.name,
            child_state_name(&self.state),
            pid,
            self.incarnation,
        );
    }

    fn now_ms(&self) -> u64 {
        self.clock.now_epoch_ms()
    }
}

fn child_state_name(state: &ChildState) -> &'static str {
    match state {
        ChildState::Idle => "idle",
        ChildState::Spawning { .. } => "spawning",
        ChildState::Booting { .. } => "booting",
        ChildState::Live { .. } => "live",
        ChildState::Dying { .. } => "dying",
        ChildState::Dead => "dead",
        ChildState::GaveUp => "give_up",
    }
}

/// Writes the passport without holding both locks at once.
fn write_passport(
    sink: &SharedSink,
    passport: &Arc<Mutex<Passport>>,
) -> Result<(), SupervisorError> {
    let snapshot = passport.lock().expect("passport lock").clone();
    sink.lock().expect("sink lock").passport(&snapshot)?;
    Ok(())
}

/// Prefers the kernel's accounting where it has an answer, and `/proc` where it does not.
///
/// `rusage` is authoritative for peak RSS and CPU time and knows nothing about descriptors,
/// threads, or I/O counters; a zombie's `/proc` is the reverse. Neither is completed with a
/// zero: an unread measurement stays absent.
fn merge_samples(live: ProcessSample, accounted: ProcessSample) -> ProcessSample {
    ProcessSample {
        max_rss_bytes: accounted.max_rss_bytes.or(live.max_rss_bytes),
        rss_bytes: accounted.rss_bytes.or(live.rss_bytes),
        user_us: accounted.user_us.or(live.user_us),
        system_us: accounted.system_us.or(live.system_us),
        fds: accounted.fds.or(live.fds),
        threads: accounted.threads.or(live.threads),
        io_read_bytes: accounted.io_read_bytes.or(live.io_read_bytes),
        io_write_bytes: accounted.io_write_bytes.or(live.io_write_bytes),
        oom_score_adj: accounted.oom_score_adj.or(live.oom_score_adj),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_samples_prefer_the_kernels_accounting_and_never_invent_a_zero() {
        let live = ProcessSample {
            rss_bytes: Some(512),
            threads: Some(4),
            fds: Some(3),
            ..ProcessSample::default()
        };
        let accounted = ProcessSample {
            max_rss_bytes: Some(2048),
            user_us: Some(10),
            system_us: Some(20),
            ..ProcessSample::default()
        };

        let merged = merge_samples(live, accounted);

        assert_eq!(merged.max_rss_bytes, Some(2048));
        assert_eq!(merged.rss_bytes, Some(512));
        assert_eq!(merged.threads, Some(4));
        assert_eq!(merged.user_us, Some(10));
        assert_eq!(
            merged.io_read_bytes, None,
            "a measurement nobody read stays absent"
        );
    }

    #[test]
    fn a_declared_duration_no_instant_can_hold_is_clamped_rather_than_fatal() {
        // The shape a manifest can actually produce: a backoff declared as u64::MAX seconds,
        // which no monotonic `Instant` can name. Before this clamp it panicked the loop whose
        // whole job is to witness deaths.
        let before = Instant::now();
        let clamped = deadline_after(Duration::from_secs(u64::MAX));
        assert!(clamped > before, "the clamp is still a future deadline");
        assert!(
            clamped <= Instant::now() + OVERFLOW_DEADLINE,
            "an unrepresentable backoff falls back to the hour, not to forever"
        );

        // `patience_ms` of u64::MAX IS representable (some 584 million years), so it is
        // honoured rather than clamped: an absurd budget is a rung that never times out, and
        // the loop still leaves it on the child's death or on a shutdown.
        assert!(
            deadline_after(Duration::from_millis(u64::MAX)) > Instant::now() + OVERFLOW_DEADLINE
        );
    }

    #[test]
    fn a_representable_duration_is_honoured_exactly() {
        let now = Instant::now();
        let deadline = deadline_after(Duration::from_secs(10));
        assert!(deadline >= now + Duration::from_secs(10));
        assert!(
            deadline < now + OVERFLOW_DEADLINE,
            "nothing clamps a real budget"
        );
    }

    #[test]
    fn a_panicking_supervision_thread_raises_the_shutdown_flag() {
        let flag = Arc::new(AtomicBool::new(false));
        let unwound = Arc::clone(&flag);
        // A panic on the thread is the only way this guard is ever left armed in production.
        let _ = thread::spawn(move || {
            let _guard = PanicGuard::new(unwound);
            panic!("supervision of a child panicked");
        })
        .join();
        assert!(
            flag.load(Ordering::SeqCst),
            "siblings must be asked to stop"
        );

        let clean = Arc::new(AtomicBool::new(false));
        {
            let mut guard = PanicGuard::new(Arc::clone(&clean));
            guard.disarm();
        }
        assert!(
            !clean.load(Ordering::SeqCst),
            "a clean return — GaveUp included — leaves the siblings running"
        );
    }

    #[test]
    fn the_system_clock_reads_a_plausible_wall_clock() {
        // Any epoch beyond 2020 proves this is milliseconds since the epoch, not seconds.
        assert!(SystemClock.now_epoch_ms() > 1_577_836_800_000);
    }
}
