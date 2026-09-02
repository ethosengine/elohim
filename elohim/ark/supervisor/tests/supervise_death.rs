//! The supervision loop against real children.
//!
//! `/bin/sh` is the artifact for every child here, hashed in the test exactly as the driver
//! hashes it before spawn, so a passing run exercises the whole path a conductor takes: a
//! declared child becomes a process, its death is learned without being consumed, a witness
//! reaches the spool before anything is decided, and the decision is recorded against the
//! same incident.
//!
//! Timing is deliberate rather than incidental. The flapper's backoff is two seconds so that
//! its third death — and therefore the run's last verdict — is reliably LATER than the
//! sigkilled child's single restart verdict, which the test waits for explicitly before it
//! waits for the give-up. Every test here stays well inside the plan's fifteen seconds.
//!
//! Two of these tests are about what the loop must NOT do: it must not let a shutdown request
//! swallow a death that already happened, and it must not leave siblings running when the
//! thread supervising one child stops existing.

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

use ark_core::{
    berth::Berth,
    manifest::{
        ArtifactRef, Backoff, ChildPolicy, ChildSpec, Intensity, Probe, ProcessKind,
        RuntimeManifest, Shutdown,
    },
    ExitClass, GiveUpReason, Incident, IncidentClose, Intent, IntentAction, Passport,
    ProcessSample, RestartVerdict, RuntimeScope, WitnessSink, UNIT_PROCESS_MS,
};
use ark_supervisor::{
    driver::{Driver, DriverError, Fingerprint, Started},
    native::{sha256_file, NativeDriver},
    reaper::wait_nowait,
    spool::{Spool, WitnessSummary},
    supervisor::{RunOutcome, Supervisor, SupervisorError, SystemClock},
};
use elohim_epr_rea::{fold::resource_state, store::FlowStore, store::SidecarFlowStore, ReaVerb};

const SHELL: &str = "/bin/sh";

fn shell_artifact() -> ArtifactRef {
    ArtifactRef::Pinned {
        cid: None,
        sha256: sha256_file(&PathBuf::from(SHELL)).unwrap(),
        bytes: None,
    }
}

fn policy(backoff_s: u64, grace_ms: u64) -> ChildPolicy {
    ChildPolicy {
        intensity: Intensity {
            max_deaths: 10,
            window_s: 300,
        },
        backoff: Backoff {
            min_s: backoff_s,
            max_s: backoff_s,
            steps: 0,
        },
        same_cause_limit: 3,
        shutdown: Shutdown {
            signal: 2,
            grace_ms,
        },
        ..ChildPolicy::default()
    }
}

fn child(name: &str, script: &str, readiness: Vec<Probe>, policy: ChildPolicy) -> ChildSpec {
    ChildSpec {
        name: name.to_string(),
        kind: ProcessKind::Native,
        artifact: shell_artifact(),
        argv: vec!["{artifact}".into(), "-c".into(), script.to_string()],
        readiness,
        policy,
        ..ChildSpec::default()
    }
}

fn berth_for(manifest: &RuntimeManifest, data_root: PathBuf, processes: &[&str]) -> Berth {
    Berth {
        manifest: manifest.cid().unwrap(),
        data_root,
        artifacts: processes
            .iter()
            .map(|name| (name.to_string(), PathBuf::from(SHELL)))
            .collect::<BTreeMap<_, _>>(),
        ..Berth::default()
    }
}

fn supervisor_for(manifest: &RuntimeManifest, berth: &Berth) -> Supervisor {
    supervisor_with_driver(manifest, berth, Box::new(NativeDriver))
}

fn supervisor_with_driver(
    manifest: &RuntimeManifest,
    berth: &Berth,
    driver: Box<dyn Driver>,
) -> Supervisor {
    let scope = Supervisor::scope_for(manifest, berth).unwrap();
    let spool = Spool::open(&berth.data_root, scope).unwrap();
    Supervisor::new(
        manifest.clone(),
        berth.clone(),
        driver,
        Box::new(spool),
        Box::new(SystemClock),
    )
    .unwrap()
}

/// Starts every declared child except one, which it panics on part-way through the run.
///
/// The sleep is what makes the failure a panic MID-RUN rather than a race at startup: the
/// sibling is spawned, ready, and being polled by the time this thread stops existing.
struct PanickingDriver {
    inner: NativeDriver,
    panic_on: String,
}

impl Driver for PanickingDriver {
    fn fingerprint(&self) -> Fingerprint {
        self.inner.fingerprint()
    }

    fn start(&self, spec: &ChildSpec, berth: &Berth) -> Result<Started, DriverError> {
        if spec.name == self.panic_on {
            thread::sleep(Duration::from_millis(500));
            panic!("driver panicked starting {}", spec.name);
        }
        self.inner.start(spec, berth)
    }

    fn signal(&self, pid: u32, signal: i32) -> Result<(), DriverError> {
        self.inner.signal(pid, signal)
    }

    fn stats(&self, pid: u32) -> Option<ProcessSample> {
        self.inner.stats(pid)
    }
}

/// A second handle on the same spool, for reading what the running supervisor writes.
fn reader(berth: &Berth, manifest: &RuntimeManifest) -> Spool {
    Spool::open(
        &berth.data_root,
        RuntimeScope::from_berth(berth).unwrap_or_else(|_| panic!("berth {manifest:?} has no CID")),
    )
    .unwrap()
}

/// Polls `probe` every 50 ms until it yields a value, or panics naming what never happened.
fn wait_until<T>(what: &str, timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn passport_of(spool: &Spool) -> Option<Passport> {
    WitnessSink::load_passport(spool).unwrap()
}

fn ready_pid(spool: &Spool, process: &str) -> Option<u32> {
    passport_of(spool)?
        .processes
        .into_iter()
        .find(|entry| entry.name == process && entry.ready)
        .and_then(|entry| entry.pid)
}

fn witnesses_of(spool: &Spool, process: &str) -> Vec<WitnessSummary> {
    let mut all: Vec<WitnessSummary> = spool
        .list_witnesses()
        .unwrap()
        .into_iter()
        .filter(|summary| summary.process == process)
        .collect();
    // `list_witnesses` is newest-first; observation order reads better in assertions.
    all.reverse();
    all
}

fn decided(spool: &Spool, process: &str) -> Vec<WitnessSummary> {
    witnesses_of(spool, process)
        .into_iter()
        .filter(|summary| summary.verdict.is_some())
        .collect()
}

fn intents_of(root: &std::path::Path, process: &str) -> Vec<Intent> {
    let contents = std::fs::read_to_string(root.join("ark/intents.log")).unwrap_or_default();
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Intent>(line).unwrap())
        .filter(|intent| intent.process == process)
        .collect()
}

fn incidents_of(spool: &Spool, process: &str) -> Vec<Incident> {
    spool
        .list_incidents()
        .unwrap()
        .into_iter()
        .filter(|incident| incident.process == process)
        .collect()
}

fn sigkill(pid: u32) {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    )
    .unwrap();
}

#[test]
fn a_sigkilled_child_leaves_a_witness_then_restarts_then_gives_up_on_same_cause() {
    let data_root = tempfile::tempdir().unwrap();
    let manifest = RuntimeManifest {
        processes: vec![
            child(
                "child",
                "echo booted; exec sleep 300",
                vec![Probe::StdoutLine {
                    contains: "booted".to_string(),
                    patience_ms: 5_000,
                }],
                policy(0, 1_000),
            ),
            // No readiness ladder: a child that declares no probe never claims to be ready,
            // so its death window is never reset and three identical fast deaths accumulate.
            child("flapper", "exit 7", Vec::new(), policy(2, 1_000)),
        ],
        ..RuntimeManifest::default()
    };
    let berth = berth_for(&manifest, data_root.path().into(), &["child", "flapper"]);
    let spool = reader(&berth, &manifest);

    let supervisor = supervisor_for(&manifest, &berth);
    let shutdown = supervisor.shutdown_flag();
    let running = thread::spawn(move || supervisor.run());

    // The child boots, is witnessed dying by signal 9, and comes back.
    let first_pid = wait_until("the child to become ready", Duration::from_secs(10), || {
        ready_pid(&spool, "child")
    });
    sigkill(first_pid);

    let restart = wait_until(
        "the child's restart verdict",
        Duration::from_secs(10),
        || decided(&spool, "child").into_iter().next(),
    );
    assert_eq!(
        restart.exit,
        ExitClass::Signaled {
            signal: 9,
            core_dumped: false
        }
    );
    assert_eq!(
        restart.verdict,
        Some(RestartVerdict::Restart {
            after_s: 0,
            attempt: 0
        })
    );
    assert_eq!(restart.pid, first_pid);

    // Two writes, one death: the write-ahead copy carries no verdict, and both address the
    // same incident — the first CID is the record that survives a crash mid-decision.
    let both = witnesses_of(&spool, "child");
    assert_eq!(
        both.len(),
        2,
        "one death leaves a write-ahead witness and a decided one"
    );
    let write_ahead = both
        .iter()
        .find(|summary| summary.verdict.is_none())
        .expect("the write-ahead witness is durable before anything is decided");
    assert_eq!(write_ahead.incident, restart.incident);
    assert_ne!(write_ahead.cid, restart.cid);

    let second_pid = wait_until(
        "the child to become ready again",
        Duration::from_secs(10),
        || ready_pid(&spool, "child").filter(|pid| *pid != first_pid),
    );

    let child_incident = wait_until(
        "the child's incident to close",
        Duration::from_secs(10),
        || {
            incidents_of(&spool, "child")
                .into_iter()
                .find(|incident| incident.closed.is_some())
        },
    );
    assert!(matches!(
        child_incident.closed,
        Some(IncidentClose::ReadyAgain { .. })
    ));
    assert_eq!(child_incident.id, restart.incident);
    assert_eq!(
        child_incident.witnesses,
        vec![write_ahead.cid.clone(), restart.cid.clone()],
        "the incident holds both addresses of the one death, in write order"
    );
    assert_eq!(
        child_incident.restarts.len(),
        1,
        "the restart intent is an incident output"
    );

    // Write-ahead intents, in order, before the actions they name.
    let child_intents = intents_of(data_root.path(), "child");
    assert_eq!(
        child_intents
            .iter()
            .map(|intent| intent.action.clone())
            .collect::<Vec<_>>(),
        vec![
            IntentAction::Spawn,
            IntentAction::Restart {
                attempt: 0,
                after_s: 0
            },
        ]
    );

    // The flapper dies on its own, three times, with one cause — and is given up on.
    let give_up = wait_until("the flapper to give up", Duration::from_secs(12), || {
        decided(&spool, "flapper")
            .into_iter()
            .find(|summary| matches!(summary.verdict, Some(RestartVerdict::GiveUp { .. })))
    });
    let Some(RestartVerdict::GiveUp {
        reason: GiveUpReason::SameCause { key, count },
    }) = give_up.verdict.clone()
    else {
        panic!("expected a same-cause give-up, got {:?}", give_up.verdict);
    };
    assert_eq!(count, 3);
    assert!(key.contains("exited:7"), "give-up key was {key}");

    let flapper_decided = decided(&spool, "flapper");
    assert_eq!(flapper_decided.len(), 3, "three deaths, three decisions");
    assert_eq!(
        witnesses_of(&spool, "flapper").len(),
        6,
        "each death is written twice"
    );
    assert_eq!(
        flapper_decided[0].exit,
        ExitClass::Exited { code: 7 },
        "the flapper exits 7 every time"
    );

    shutdown.store(true, Ordering::SeqCst);
    let outcome: RunOutcome = running
        .join()
        .expect("the supervisor thread panicked")
        .expect("the supervisor returned an error");

    assert_eq!(
        outcome.exit_code, 3,
        "a process was permanently abandoned, so the run did not deliver the manifest"
    );
    assert!(matches!(
        outcome.passport.last_verdict,
        Some(RestartVerdict::GiveUp { .. })
    ));
    assert!(
        outcome.passport.processes.iter().all(|entry| !entry.ready),
        "no process is still running once the run has ended"
    );

    // Nothing is left for anyone else to reap: every pid this run created is consumed.
    for pid in [first_pid, second_pid]
        .into_iter()
        .chain(flapper_decided.iter().map(|summary| summary.pid))
    {
        assert!(
            wait_nowait(pid).is_err(),
            "pid {pid} was left unreaped by the supervision loop"
        );
    }

    // A6 — the runtime's records ARE valueflow records: three deaths, three `Consume` events
    // denominated in process-ms, folded by the same mechanism `epr flow` reads.
    let flows = SidecarFlowStore::open(&data_root.path().join("ark")).unwrap();
    let events: Vec<_> = flows
        .events()
        .unwrap()
        .into_iter()
        .map(|(_, event)| event)
        .collect();
    let flapper_deaths: Vec<_> = events
        .iter()
        .filter(|event| {
            event.classified_as.iter().any(|tag| tag == "runtime:death")
                && event.classified_as.iter().any(|tag| tag == "flapper")
        })
        .cloned()
        .collect();
    assert_eq!(
        flapper_deaths.len(),
        3,
        "one death event per death, never per write"
    );

    let mut consumed = 0u64;
    for event in &flapper_deaths {
        assert_eq!(event.action, ReaVerb::Consume);
        let state = resource_state(&event.resource, &events);
        assert_eq!(state.event_count, 1, "each witness is its own resource");
        assert!(
            state.total(ReaVerb::Consume, UNIT_PROCESS_MS) > 0.0,
            "a death event carries the process-ms it actually consumed"
        );
        consumed += state.event_count;
    }
    assert_eq!(consumed, 3);

    assert!(
        flows.open_pain().unwrap().is_empty(),
        "under manifest policy there is no promise to be in pain about — honest absence, not a zero"
    );
}

#[test]
fn shutdown_sends_policy_signal_then_kills_after_grace() {
    let data_root = tempfile::tempdir().unwrap();
    // `trap "" INT` sets SIG_IGN, which survives `exec` — so the single `sleep` process the
    // supervisor holds ignores the policy signal and must be killed after the grace period.
    let manifest = RuntimeManifest {
        processes: vec![child(
            "stubborn",
            "trap \"\" INT; echo booted; exec sleep 300",
            vec![Probe::StdoutLine {
                contains: "booted".to_string(),
                patience_ms: 5_000,
            }],
            policy(0, 300),
        )],
        ..RuntimeManifest::default()
    };
    let berth = berth_for(&manifest, data_root.path().into(), &["stubborn"]);
    let spool = reader(&berth, &manifest);

    let supervisor = supervisor_for(&manifest, &berth);
    let shutdown = supervisor.shutdown_flag();
    let running = thread::spawn(move || supervisor.run());

    let pid = wait_until("the child to become ready", Duration::from_secs(10), || {
        ready_pid(&spool, "stubborn")
    });

    let requested = Instant::now();
    shutdown.store(true, Ordering::SeqCst);
    let outcome = running
        .join()
        .expect("the supervisor thread panicked")
        .expect("the supervisor returned an error");

    assert!(
        requested.elapsed() < Duration::from_secs(1),
        "the child outlived its grace period by {:?}",
        requested.elapsed()
    );
    assert_eq!(outcome.exit_code, 0, "a clean shutdown is not a failure");
    assert!(wait_nowait(pid).is_err(), "pid {pid} was left unreaped");

    let intents = intents_of(data_root.path(), "stubborn");
    let actions: Vec<_> = intents.iter().map(|intent| intent.action.clone()).collect();
    assert_eq!(
        actions,
        vec![
            IntentAction::Spawn,
            IntentAction::Stop {
                signal: 2,
                grace_ms: 300
            },
            IntentAction::Kill,
        ],
        "the policy signal is recorded with the policy's own grace, then the kill that followed"
    );

    // A stop is not a death: nothing was witnessed, and the incident closed as stopped.
    assert!(witnesses_of(&spool, "stubborn").is_empty());
}

#[test]
fn a_death_during_the_poll_sleep_is_witnessed_and_never_laundered_into_a_stop() {
    let data_root = tempfile::tempdir().unwrap();
    // One same-cause death is enough to give up, so the run ends ON the death rather than on a
    // restart the raised flag would then stop — which would close the incident as `Stopped`
    // for an honest reason and hide the dishonest one this test is about.
    let mut policy = policy(0, 300);
    policy.same_cause_limit = 1;
    let manifest = RuntimeManifest {
        processes: vec![child(
            "child",
            "echo booted; exec sleep 300",
            vec![Probe::StdoutLine {
                contains: "booted".to_string(),
                patience_ms: 5_000,
            }],
            policy,
        )],
        ..RuntimeManifest::default()
    };
    let berth = berth_for(&manifest, data_root.path().into(), &["child"]);
    let spool = reader(&berth, &manifest);

    let supervisor = supervisor_for(&manifest, &berth);
    let shutdown = supervisor.shutdown_flag();
    let running = thread::spawn(move || supervisor.run());

    let pid = wait_until("the child to become ready", Duration::from_secs(10), || {
        ready_pid(&spool, "child")
    });

    // Both happen inside one poll interval, so the loop wakes owing two answers at once: a
    // child that is already dead, and a flag that says stop. The death is the one that cannot
    // be recovered later, so the death is the one that must be read first.
    sigkill(pid);
    shutdown.store(true, Ordering::SeqCst);

    let outcome = running
        .join()
        .expect("the supervisor thread panicked")
        .expect("the supervisor returned an error");

    let witnesses = witnesses_of(&spool, "child");
    assert!(
        !witnesses.is_empty(),
        "the SIGKILL was laundered into a clean stop: no witness reached the spool"
    );
    assert_eq!(
        witnesses[0].exit,
        ExitClass::Signaled {
            signal: 9,
            core_dumped: false
        },
        "the witness records the signal that actually killed the child"
    );

    let incident = wait_until("the child's incident", Duration::from_secs(5), || {
        incidents_of(&spool, "child").into_iter().next()
    });
    assert!(
        !matches!(incident.closed, Some(IncidentClose::Stopped { .. })),
        "a death was closed as if the ark had meant it: {:?}",
        incident.closed
    );
    assert!(
        matches!(incident.closed, Some(IncidentClose::GaveUp { .. })),
        "the incident closes on the verdict the death earned: {:?}",
        incident.closed
    );

    let actions: Vec<_> = intents_of(data_root.path(), "child")
        .into_iter()
        .map(|intent| intent.action)
        .collect();
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, IntentAction::Stop { .. })),
        "nothing was stopped — the child was already dead: {actions:?}"
    );

    assert_eq!(
        outcome.exit_code, 3,
        "the child was permanently abandoned after its death"
    );
    assert!(wait_nowait(pid).is_err(), "pid {pid} was left unreaped");
}

#[test]
fn a_panicking_supervision_thread_stops_its_siblings_and_fails_the_run() {
    let data_root = tempfile::tempdir().unwrap();
    let manifest = RuntimeManifest {
        processes: vec![
            child(
                "sibling",
                "echo booted; exec sleep 300",
                vec![Probe::StdoutLine {
                    contains: "booted".to_string(),
                    patience_ms: 5_000,
                }],
                policy(0, 300),
            ),
            child("exploder", "exec sleep 300", Vec::new(), policy(0, 300)),
        ],
        ..RuntimeManifest::default()
    };
    let berth = berth_for(&manifest, data_root.path().into(), &["sibling", "exploder"]);
    let spool = reader(&berth, &manifest);

    let supervisor = supervisor_with_driver(
        &manifest,
        &berth,
        Box::new(PanickingDriver {
            inner: NativeDriver,
            panic_on: "exploder".to_string(),
        }),
    );
    let running = thread::spawn(move || supervisor.run());

    let pid = wait_until(
        "the sibling to become ready",
        Duration::from_secs(10),
        || ready_pid(&spool, "sibling"),
    );

    // Without a guard on the unwind this join never returns: the panicked thread asked for
    // nothing, and the sibling would go on sleeping for its declared three hundred seconds.
    let error = running
        .join()
        .expect("the test's own thread")
        .expect_err("a run with an unsupervised child is not a success");
    assert!(
        matches!(&error, SupervisorError::Panicked { process } if process == "exploder"),
        "the run names the thread that stopped existing, got {error:?}"
    );

    let actions: Vec<_> = intents_of(data_root.path(), "sibling")
        .into_iter()
        .map(|intent| intent.action)
        .collect();
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, IntentAction::Stop { .. })),
        "the sibling was never asked to stop: {actions:?}"
    );
    assert!(
        witnesses_of(&spool, "sibling").is_empty(),
        "the sibling was stopped, not killed off — a stop is not a death"
    );
    assert!(
        wait_nowait(pid).is_err(),
        "the sibling {pid} outlived the run that could no longer supervise it"
    );
}

#[test]
fn a_berth_that_does_not_match_its_manifest_is_refused_before_anything_runs() {
    let data_root = tempfile::tempdir().unwrap();
    let manifest = RuntimeManifest {
        processes: vec![child("child", "exit 0", Vec::new(), policy(0, 100))],
        ..RuntimeManifest::default()
    };

    let mut wrong_cid = berth_for(&manifest, data_root.path().into(), &["child"]);
    wrong_cid.manifest = RuntimeManifest::default().cid().unwrap();
    assert!(matches!(
        Supervisor::scope_for(&manifest, &wrong_cid),
        Err(SupervisorError::BerthManifestMismatch { .. })
    ));

    let mut not_a_cid = berth_for(&manifest, data_root.path().into(), &["child"]);
    not_a_cid.manifest = "not-a-cid".to_string();
    assert!(matches!(
        Supervisor::scope_for(&manifest, &not_a_cid),
        Err(SupervisorError::BerthManifestNotACid { .. })
    ));

    let mut no_root = berth_for(&manifest, data_root.path().into(), &["child"]);
    no_root.data_root = PathBuf::new();
    assert!(matches!(
        Supervisor::scope_for(&manifest, &no_root),
        Err(SupervisorError::BerthWithoutDataRoot)
    ));

    let mut unplaced = berth_for(&manifest, data_root.path().into(), &[]);
    unplaced.artifacts.clear();
    assert!(matches!(
        Supervisor::scope_for(&manifest, &unplaced),
        Err(SupervisorError::ArtifactNotPlaced { process }) if process == "child"
    ));

    // The one shape that is accepted, so the refusals above cannot pass vacuously.
    let placed = berth_for(&manifest, data_root.path().into(), &["child"]);
    let scope = Supervisor::scope_for(&manifest, &placed).unwrap();
    assert_eq!(scope.scope.to_string(), placed.manifest);
}
