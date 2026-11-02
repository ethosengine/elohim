//! Synchronize a real signal after successful output capture and outcome construction.
//! Only the isolated unit-test process contains the checkpoint; the shipped CLI does not.

use super::*;
use std::{
    cell::Cell,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

thread_local! {
    static WAIT_BEFORE_ANSWER: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn before_answer(run: &Path) {
    if WAIT_BEFORE_ANSWER.get() {
        fs::write(
            run.join("before-answer"),
            b"output captured; outcome constructed",
        )
        .expect("checkpoint");
        let deadline = Instant::now() + Duration::from_secs(20);
        while task::interruption().is_none() {
            assert!(Instant::now() < deadline, "no signal at answer boundary");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

// Keep the process-wide handler and subreaper state out of the parallel test harness.
#[test]
fn isolated_native_run() {
    let Some(root) = std::env::var_os("ARK_NATIVE_ANSWER_TEST") else {
        return;
    };
    let root = PathBuf::from(root);
    WAIT_BEFORE_ANSWER.set(true);
    let code = run(RunArgs {
        spec: None,
        request: Some(root.join("request.json")),
        root: Some(fs::read_to_string(root.join("root")).expect("root")),
        store: root.join("store"),
        run_dir: root.join("run"),
        artifact: std::env::current_exe().expect("artifact"),
        device: None,
    })
    .expect("native result");
    std::process::exit(i32::from(code));
}

#[test]
fn output_fixture() {
    let Some(output) = std::env::var_os("ARK_NATIVE_OUTPUT_TEST") else {
        return;
    };
    fs::write(PathBuf::from(output).join("answer"), b"completed output\n").expect("output");
}

struct Running(Option<Child>);

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn native_interrupt_before_answer_is_unanswered() {
    use sha2::{Digest, Sha256};

    let temp = tempfile::tempdir().expect("fixture");
    let source = temp.path().join("source");
    fs::create_dir(&source).expect("source");
    fs::write(source.join("value"), b"candidate A").expect("input");
    let captured = futures::executor::block_on(eprfs_local::capture_tree(
        &source,
        &temp.path().join("store"),
        &["value".into()],
        &TreeLimits::default(),
    ))
    .expect("candidate");
    fs::write(temp.path().join("root"), captured.root.to_string()).expect("root");
    let executable = std::env::current_exe().expect("test executable");
    let digest = hex::encode(Sha256::digest(
        fs::read(&executable).expect("artifact bytes"),
    ));
    let request = serde_json::json!({
        "method": {"name":"brit-gate:answer-interrupt", "deterministic":false},
        "child": {
            "artifact":{"pinned":{"sha256":digest}},
            "argv":["{artifact}", "--exact", "task_native::tests::output_fixture", "--nocapture"],
            "env":{"PATH":"", "HOME":"{data_root}/home", "ARK_NATIVE_OUTPUT_TEST":"{output}"},
            "envScrub":true
        },
        "outputs":["answer"]
    });
    fs::write(temp.path().join("request.json"), request.to_string()).expect("request");
    let mut running = Running(Some(
        Command::new(executable)
            .args([
                "--exact",
                "task_native::tests::isolated_native_run",
                "--nocapture",
            ])
            .env("ARK_NATIVE_ANSWER_TEST", temp.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("isolated native run"),
    ));
    let run_dir = temp.path().join("run");
    let deadline = Instant::now() + Duration::from_secs(20);
    let child = running.0.as_mut().expect("child");
    while !run_dir.join("before-answer").exists() {
        assert!(
            child.try_wait().expect("poll").is_none(),
            "run exited before checkpoint"
        );
        assert!(Instant::now() < deadline, "answer checkpoint timeout");
        thread::sleep(Duration::from_millis(10));
    }
    // The checkpoint is after successful capture and TaskOutcome::new, and holds
    // the run until its actual SIGTERM handler has recorded the signal.
    assert_eq!(
        fs::read(run_dir.join("output/answer")).expect("output"),
        b"completed output\n"
    );
    let before = fs::read_to_string(run_dir.join("status.jsonl")).expect("queued status");
    assert!(!before.contains("\"answered\""));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .expect("signal before answer");
    let output = running
        .0
        .take()
        .expect("child")
        .wait_with_output()
        .expect("reap");
    assert_eq!(output.status.code(), Some(143), "{output:?}");
    let status = fs::read_to_string(run_dir.join("status.jsonl")).expect("terminal status");
    let rows: Vec<serde_json::Value> = status
        .lines()
        .map(|line| serde_json::from_str(line).expect("status JSON"))
        .collect();
    assert_eq!(rows.len(), 2, "{status}");
    assert!(status.contains("unanswered"), "{status}");
    assert!(!status.contains("\"answered\""), "{status}");
    // libtest writes a preamble, but a TaskOutcome JSON must never be emitted.
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(
        !stdout.lines().any(|line| line
            .find('{')
            .is_some_and(|at| { serde_json::from_str::<serde_json::Value>(&line[at..]).is_ok() })),
        "{stdout}"
    );
    assert_eq!(
        fs::read(source.join("value")).expect("source retained"),
        b"candidate A"
    );
}
