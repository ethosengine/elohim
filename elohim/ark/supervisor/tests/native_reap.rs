//! The Native driver and the reaper against a real child process.
//!
//! `/bin/sh` is the artifact for the death tests: it is present on every host this runs on,
//! and hashing it in the test is the same act the driver performs before spawn, so a passing
//! run proves the passport-grade hash is computed over the bytes actually executed. The
//! refusal and relative-path tests use a purpose-written script instead, because they need an
//! artifact whose execution leaves a mark on the filesystem — a refusal that is only observed
//! as an `Err` proves the driver returned early, not that nothing ran.

use std::path::{Component, Path, PathBuf};

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

/// Writes an executable script whose only effect is to create `marker`, and returns its path.
///
/// The marker is what makes a refusal observable: an artifact that never runs leaves the
/// filesystem exactly as it found it.
fn marker_script(dir: &Path, marker: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("artifact.sh");
    std::fs::write(&path, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A path that reaches `target` only when resolved from the test process's own directory.
///
/// Built by walking up to the root and back down, which is what makes it a real relative
/// path: the child is spawned with `current_dir(data_root)`, so this resolves to `target`
/// for the supervisor and to nothing at all for the child.
fn relative_from_cwd(target: &Path) -> PathBuf {
    let normal = |path: &Path| -> PathBuf {
        path.components()
            .filter(|component| matches!(component, Component::Normal(_)))
            .collect()
    };
    let cwd = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    let target = std::fs::canonicalize(target).unwrap();

    let mut relative = PathBuf::new();
    for _ in 0..normal(&cwd).components().count() {
        relative.push("..");
    }
    relative.join(normal(&target))
}

/// Polls without consuming until the child dies, then reaps it and returns how it ended.
fn await_exit(pid: u32) -> ExitClass {
    for _ in 0..100 {
        if let WaitEvent::Exited { .. } = wait_nowait(pid).unwrap() {
            return reap_with_rusage(pid).unwrap().0;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("child {pid} never exited");
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
    // The recorded path is the file the kernel opened, symlinks resolved: `/bin/sh` names a
    // shell, and the passport's claim is about the bytes behind that name.
    assert_eq!(
        started.artifact_path,
        std::fs::canonicalize(shell()).unwrap()
    );
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
    let artifacts = tempfile::tempdir().unwrap();
    let data_root = tempfile::tempdir().unwrap();
    let marker = data_root.path().join("the-artifact-ran");
    let artifact = marker_script(artifacts.path(), &marker);
    let truth = sha256_file(&artifact).unwrap();

    let refused = "00".repeat(32);
    let mut spec = spec(ArtifactRef::Pinned {
        cid: None,
        sha256: refused.clone(),
        bytes: None,
    });
    spec.argv = vec!["{artifact}".into()];
    let mut berth = berth(data_root.path().into());
    berth
        .artifacts
        .insert("child".to_string(), artifact.clone());

    match NativeDriver.start(&spec, &berth) {
        Err(DriverError::ArtifactHashMismatch {
            expected,
            actual,
            path,
        }) => {
            assert_eq!(expected, refused);
            assert_eq!(actual, truth);
            assert_eq!(path, std::fs::canonicalize(&artifact).unwrap());
        }
        other => panic!("expected ArtifactHashMismatch, got {other:?}"),
    }

    // The refusal is observable, not merely returned: the artifact leaves a mark when it
    // runs, and that mark is absent. A mismatch is exit 66 before any process exists.
    assert!(
        !marker.exists(),
        "the refused artifact ran anyway: {}",
        marker.display()
    );

    // The control, so the assertion above cannot pass vacuously: the same script, pinned to
    // its true digest, does run and does leave the mark.
    spec.artifact = ArtifactRef::Pinned {
        cid: None,
        sha256: truth,
        bytes: None,
    };
    let started = NativeDriver.start(&spec, &berth).unwrap();
    assert_eq!(await_exit(started.pid), ExitClass::Exited { code: 0 });
    assert!(marker.exists(), "the control artifact never ran");
}

#[test]
fn a_relative_artifact_is_hashed_and_executed_as_the_same_file() {
    let artifacts = tempfile::tempdir().unwrap();
    let data_root = tempfile::tempdir().unwrap();
    let marker = data_root.path().join("the-artifact-ran");
    let artifact = marker_script(artifacts.path(), &marker);

    // Relative to the supervisor's directory, and meaningless from the berth's `data_root` —
    // which is where the child is `chdir`ed before `exec`. Absolutising before the hash is
    // what keeps the hashed file and the executed file the same file.
    let declared = relative_from_cwd(&artifact);
    assert!(declared.is_relative());
    let sha = sha256_file(&declared).unwrap();

    let mut spec = spec(ArtifactRef::Pinned {
        cid: None,
        sha256: sha.clone(),
        bytes: None,
    });
    spec.argv = vec!["{artifact}".into()];
    let mut berth = berth(data_root.path().into());
    berth.artifacts.insert("child".to_string(), declared);

    let started = NativeDriver.start(&spec, &berth).unwrap();
    assert_eq!(started.artifact_sha256, sha);
    assert!(
        started.artifact_path.is_absolute(),
        "a relative artifact was recorded as {}",
        started.artifact_path.display()
    );
    assert_eq!(
        started.artifact_path,
        std::fs::canonicalize(&artifact).unwrap()
    );

    assert_eq!(await_exit(started.pid), ExitClass::Exited { code: 0 });
    assert!(
        marker.exists(),
        "the hashed file was not the file that ran: {}",
        marker.display()
    );
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
