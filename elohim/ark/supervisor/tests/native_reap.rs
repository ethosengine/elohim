//! The Native driver and the reaper against a real child process.
//!
//! `/bin/sh` is the artifact under test: it is present on every host this runs on, and
//! hashing it in the test is the same act the driver performs before spawn, so a passing
//! run proves the passport-grade hash is computed over the bytes actually executed.

use std::path::PathBuf;

use ark_core::berth::Berth;
use ark_core::{
    exit::ExitClass,
    manifest::{ArtifactRef, ChildSpec, ProcessKind},
};
use ark_supervisor::{
    driver::{Driver, DriverError},
    native::{sha256_file, NativeDriver},
    reaper::{reap_with_rusage, wait_nowait, WaitEvent},
};

fn shell() -> PathBuf {
    PathBuf::from("/bin/sh")
}

fn spec(artifact: ArtifactRef) -> ChildSpec {
    ChildSpec {
        name: "child".into(),
        kind: ProcessKind::Native,
        artifact,
        argv: vec![
            "{artifact}".into(),
            "-c".into(),
            "echo booted; sleep 30".into(),
        ],
        ..Default::default()
    }
}

fn berth(data_root: PathBuf) -> Berth {
    Berth {
        manifest: "x".into(),
        data_root,
        artifacts: [("child".to_string(), shell())].into(),
        ..Default::default()
    }
}

#[test]
fn sigkilled_child_is_witnessed_as_signaled_9_with_rusage() {
    let dir = tempfile::tempdir().unwrap();
    let sha = sha256_file(&shell()).unwrap();
    let spec = spec(ArtifactRef::Pinned {
        cid: None,
        sha256: sha.clone(),
        bytes: None,
    });
    let berth = berth(dir.path().into());

    let started = NativeDriver.start(&spec, &berth).unwrap();
    assert_eq!(started.artifact_sha256, sha);
    assert_eq!(started.artifact_path, shell());
    assert!(started.started_at_epoch_ms > 0);
    assert!(matches!(
        wait_nowait(started.pid).unwrap(),
        WaitEvent::StillRunning
    ));

    NativeDriver.signal(started.pid, 9).unwrap();

    // Poll for at most 2 s: the death must become visible WITHOUT being consumed, which is
    // the whole reason the supervisor uses waitid(WNOWAIT) before it reaps.
    let mut seen = false;
    for _ in 0..40 {
        if let WaitEvent::Exited { .. } = wait_nowait(started.pid).unwrap() {
            seen = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(seen, "waitid(WNOWAIT) never saw the death");

    // Still visible: WNOWAIT left the zombie in place for the reaper below.
    let (class, sample) = reap_with_rusage(started.pid).unwrap();
    assert_eq!(
        class,
        ExitClass::Signaled {
            signal: 9,
            core_dumped: false
        }
    );
    assert!(
        sample.max_rss_bytes.unwrap_or(0) > 0,
        "rusage carried no maxrss"
    );

    // The child is consumed: nothing is left for anyone else to reap.
    assert!(wait_nowait(started.pid).is_err());
}

#[test]
fn hash_mismatch_refuses_to_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let expected = "00".repeat(32);
    let spec = spec(ArtifactRef::Pinned {
        cid: None,
        sha256: expected.clone(),
        bytes: None,
    });
    let berth = berth(dir.path().into());

    // No process is spawned: `start` returns before `Command::spawn` is reached, so there is
    // no pid to leak and no `Started` handle to drop.
    match NativeDriver.start(&spec, &berth) {
        Err(DriverError::ArtifactHashMismatch {
            expected: declared,
            actual,
            path,
        }) => {
            assert_eq!(declared, expected);
            assert_ne!(actual, expected);
            assert_eq!(actual, sha256_file(&shell()).unwrap());
            assert_eq!(path, shell());
        }
        other => panic!("expected ArtifactHashMismatch, got {other:?}"),
    }
}

#[test]
fn channel_artifact_is_refused_in_s0_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let spec = spec(ArtifactRef::Channel {
        channel_id: "conductor/stable".into(),
    });
    let berth = berth(dir.path().into());

    match NativeDriver.start(&spec, &berth) {
        Err(DriverError::ChannelUnresolvedInS0 { channel_id }) => {
            assert_eq!(channel_id, "conductor/stable");
        }
        other => panic!("expected ChannelUnresolvedInS0, got {other:?}"),
    }
}
