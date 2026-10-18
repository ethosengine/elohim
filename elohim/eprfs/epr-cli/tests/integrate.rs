//! Local cooperative acceptance -> gated atomic fast-forward, through the public CLI.
//! BRIT_INTEGRATION_BIN selects a required installed Brit executable for the same
//! suite; its PATH must contain installed epr. EPR_INTEGRATION_BIN selects that
//! installed evaluator for the report-pinned contrary-evidence gate as well.
#![cfg(unix)]
use elohim_epr_cli::{
    actor,
    flow::{
        self,
        acceptance::AcceptanceOptions,
        note::{note, note_with_options, NoteActor},
    },
    process,
};
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    ResourceSpec, SidecarFlowStore,
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Output};

fn fixture_command(root: &Path, program: &str, args: &[&str]) -> std::process::Command {
    let mut command = process::build_command(program, args, root, &[]);
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test");
    command
}
fn command(root: &Path, program: &str, args: &[&str]) -> Output {
    fixture_command(root, program, args)
        .output()
        .expect("fixture process")
}
fn git(root: &Path, args: &[&str]) -> String {
    let result = command(root, "git", args);
    assert!(
        result.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout)
        .expect("git output")
        .trim()
        .into()
}
struct Fixture {
    dir: tempfile::TempDir,
    acceptance: String,
    source: String,
    target: String,
}
fn evaluator_bin() -> String {
    std::env::var("EPR_INTEGRATION_BIN").unwrap_or_else(|_| env!("CARGO_BIN_EXE_epr").into())
}
impl Fixture {
    fn run(&self) -> Output {
        let program = std::env::var("BRIT_INTEGRATION_BIN").unwrap_or_else(|_| evaluator_bin());
        assert!(
            Path::new(&program).is_absolute() && Path::new(&program).is_file(),
            "integration executable must be an existing absolute path: {program}"
        );
        command(
            self.dir.path(),
            &program,
            &["integrate", "--acceptance", &self.acceptance],
        )
    }
    fn target(&self) -> String {
        git(self.dir.path(), &["rev-parse", "refs/heads/target"])
    }
}
fn fixture(mode: &str) -> Fixture {
    let dir = tempfile::tempdir().expect("isolated repo");
    let root = dir.path();
    git(root, &["init", "-q", "--initial-branch=source"]);
    fs::create_dir_all(root.join(".epr-meta")).unwrap();
    fs::create_dir_all(root.join(".githooks")).unwrap();
    fs::write(
        root.join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:fixture/standalone\n",
    )
    .unwrap();
    fs::write(
        root.join(".epr-meta/manifest.md"),
        "---\nepr-meta-version: 1\nid: integration-fixture\nroot: true\n---\n",
    )
    .unwrap();
    fs::write(
        root.join("AGENTS.md"),
        "Local governed integration fixture.\n",
    )
    .unwrap();
    fs::write(
        root.join("CLAUDE.md"),
        "Local governed integration fixture.\n",
    )
    .unwrap();
    // Harness governance activation only. The pinned quality gate below really checks
    // the candidate diff; this smoke hook does not claim production quality evidence.
    fs::write(root.join(".githooks/pre-push"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(
        root.join(".githooks/pre-push"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    git(root, &["config", "core.hooksPath", ".githooks"]);
    fs::write(root.join(".gitignore"), ".eprfs/\n").unwrap();
    fs::write(root.join("scope.md"), "The intended feature\n").unwrap();
    git(
        root,
        &[
            "add",
            ".epr-meta",
            ".githooks",
            "AGENTS.md",
            "CLAUDE.md",
            ".gitignore",
            "scope.md",
        ],
    );
    git(root, &["commit", "-qm", "base"]);
    let target = git(root, &["rev-parse", "HEAD"]);
    git(root, &["branch", "target"]);
    fs::write(root.join("feature.txt"), "Delivered feature\n").unwrap();
    git(root, &["add", "feature.txt"]);
    git(root, &["commit", "-qm", "feature"]);
    let source = git(root, &["rev-parse", "HEAD"]);
    let implementer = actor::claim(root, "agent:implementer@gpt-6", "implementation").unwrap();
    let acceptor = actor::claim(root, "agent:acceptor@gpt-6", "acceptance").unwrap();
    let mut store = SidecarFlowStore::open(root).unwrap();
    let scope = flow::body_cid("The intended feature");
    let promise = store
        .append(FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("agent:implementer@gpt-6".into()),
            receiver: flow::repository_agent(root).unwrap(),
            resource_spec: ResourceSpec {
                classified_as: vec![format!("actor-claim:{}", implementer.record_cid)],
                quantity: None,
            },
            in_scope_of: scope,
            valid_from: None,
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: vec![],
            bound: None,
        }))
        .unwrap();
    let fulfillment = store
        .append(FlowRecord::Event(FlowEvent {
            action: ReaVerb::Produce,
            provider: flow::repository_agent(root).unwrap(),
            receiver: flow::repository_agent(root).unwrap(),
            resource: flow::body_cid("Exercised actual integration"),
            quantity: Magnitude::Count {
                value: 1.,
                unit: "artifact".into(),
            },
            process: None,
            in_scope_of: scope,
            fulfills: vec![promise],
            satisfies: vec![],
            classified_as: vec![],
            occurred_at: "2026-10-12T00:00:00Z".into(),
        }))
        .unwrap();
    let on = promise.to_string();
    let review = note(
        root,
        &on,
        "verdict",
        "Independent technical review",
        None,
        Some("approved"),
        &NoteActor::default(),
    )
    .unwrap();
    let appointment = note_with_options(
        root,
        &on,
        "ruling",
        "Appoint acceptance",
        None,
        None,
        &NoteActor::default(),
        &AcceptanceOptions {
            appoint: Some(acceptor.record_cid),
            ..Default::default()
        },
    )
    .unwrap();
    fs::write(
        root.join(".eprfs/status/labels.json"),
        serde_json::json!({scope.to_string(): "scope.md"}).to_string(),
    )
    .unwrap();
    fs::write(
        root.join(".eprfs/status/evidence.txt"),
        "Exercised actual integration",
    )
    .unwrap();
    let gate: Vec<String> = match mode {
        "disable-hooks" => vec![
            "git".into(),
            "config".into(),
            "core.hooksPath".into(),
            "/dev/null".into(),
        ],
        "fail" => vec!["git".into(), "diff".into(), "--invalid-gate-option".into()],
        "timeout" => vec!["sleep".into(), "30".into()],
        "source-moved" => vec![
            "git".into(),
            "update-ref".into(),
            "refs/heads/source".into(),
            target.clone(),
        ],
        "target-moved" => vec![
            "git".into(),
            "update-ref".into(),
            "refs/heads/target".into(),
            source.clone(),
        ],
        "changed-evidence" => vec![
            "sh".into(),
            "-c".into(),
            "printf changed >.eprfs/status/evidence.txt".into(),
        ],
        "contrary" => vec![
            evaluator_bin(),
            "flow".into(),
            "note".into(),
            "--on".into(),
            on.clone(),
            "--kind".into(),
            "correction".into(),
            "--reason".into(),
            "Regression discovered during gate".into(),
        ],
        _ => vec![
            "git".into(),
            "diff".into(),
            "--check".into(),
            target.clone(),
            source.clone(),
        ],
    };
    let report = serde_json::json!({"intent":"Integrate delivered feature", "revision":source, "environment":"disposable local Linux repository",
        "actions":["Inspect candidate and run diff check"],"observations":["Candidate meets scoped assertion"],"limitations":["Local explicit approval, no cryptographic identity claim"],
        "evidence":[{"path":".eprfs/status/evidence.txt","cid":flow::body_cid("Exercised actual integration").to_string()}],
        "integration":{"sourceRef":"refs/heads/source","sourceCommit":source,"targetRef":"refs/heads/target","targetCommit":target,"gate":gate,"gateTimeoutSeconds":1}});
    fs::write(
        root.join(".eprfs/status/acceptance.json"),
        report.to_string(),
    )
    .unwrap();
    let acceptance = note_with_options(
        root,
        &on,
        "verdict",
        "Used candidate independently",
        None,
        Some("approved"),
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        &AcceptanceOptions {
            purpose: Some("acceptance".into()),
            appointment: Some(appointment.record_cid),
            fulfillment: Some(fulfillment.to_string()),
            review: Some(review.record_cid),
            report: Some(".eprfs/status/acceptance.json".into()),
            ..Default::default()
        },
    )
    .unwrap()
    .record_cid;
    Fixture {
        dir,
        acceptance,
        source,
        target,
    }
}

#[test]
fn accepted_integration_moves_only_target_and_replay_refuses() {
    let f = fixture("pass");
    let index = fs::read(f.dir.path().join(".git/index")).unwrap();
    let result = f.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(f.target(), f.source);
    assert_eq!(git(f.dir.path(), &["rev-parse", "HEAD"]), f.source);
    assert_eq!(fs::read(f.dir.path().join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(f.dir.path().join("feature.txt")).unwrap(),
        "Delivered feature\n"
    );
    assert!(
        !f.run().status.success(),
        "old approval cannot replay onto changed target"
    );
}

#[test]
fn gate_failures_movement_and_contrary_evidence_refuse() {
    for mode in [
        "fail",
        "timeout",
        "source-moved",
        "target-moved",
        "changed-evidence",
        "contrary",
        "disable-hooks",
    ] {
        let f = fixture(mode);
        let result = f.run();
        assert!(!result.status.success(), "{mode} unexpectedly integrated");
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        let expected = match mode {
            "fail" => "quality gate failed",
            "timeout" => "quality gate timed out",
            "source-moved" | "target-moved" => "moved",
            "changed-evidence" => "evidence changed",
            "contrary" => "does not currently accept",
            "disable-hooks" => "ready gate failed after",
            _ => unreachable!(),
        };
        assert!(
            diagnostic.contains(expected),
            "{mode} refused for wrong reason: {diagnostic}"
        );
        assert_eq!(
            f.target(),
            if mode == "target-moved" {
                f.source
            } else {
                f.target
            },
            "integration must not overwrite the post-gate target: {mode}"
        );
    }
}

#[test]
fn reference_transaction_hooks_refuse_without_execution() {
    for configured in [true, false] {
        let f = fixture("pass");
        let dir = if configured {
            ".githooks"
        } else {
            ".git/hooks"
        };
        if !configured {
            git(f.dir.path(), &["config", "--unset", "core.hooksPath"]);
        }
        let path = f.dir.path().join(dir).join("reference-transaction");
        fs::write(&path, "#!/bin/sh\nprintf ran > .eprfs/hook-ran\nsleep 30\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let started = std::time::Instant::now();
        let result = f.run();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("reference-transaction"),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        assert!(!f.dir.path().join(".eprfs/hook-ran").exists());
        assert_eq!(f.target(), f.target);
    }
}

#[test]
fn superseded_acceptance_cannot_authorize_integration() {
    use elohim_epr_cli::flow::acceptance::slot;
    let f = fixture("pass");
    let records = SidecarFlowStore::open(f.dir.path())
        .unwrap()
        .records()
        .unwrap();
    let event = records
        .iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Event(event) if cid.to_string() == f.acceptance => Some(event),
            _ => None,
        })
        .unwrap();
    let options = AcceptanceOptions {
        purpose: Some("acceptance".into()),
        appointment: slot(event, "appointment:").map(str::to_owned),
        fulfillment: slot(event, "fulfillment:").map(str::to_owned),
        review: slot(event, "review:").map(str::to_owned),
        report: slot(event, "report-path:").map(str::to_owned),
        supersedes: Some(f.acceptance.clone()),
        ..Default::default()
    };
    note_with_options(
        f.dir.path(),
        &event.resource.to_string(),
        "verdict",
        "A later independent decision",
        None,
        Some("approved"),
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        &options,
    )
    .unwrap();
    let result = f.run();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("superseded"));
    assert_eq!(f.target(), f.target);
}

#[test]
fn dirty_source_checked_out_target_and_changed_report_refuse() {
    for mode in ["dirty", "checked-out", "report"] {
        let f = fixture("pass");
        match mode {
            "dirty" => fs::write(f.dir.path().join("feature.txt"), "unreviewed").unwrap(),
            "checked-out" => {
                git(
                    f.dir.path(),
                    &["worktree", "add", ".eprfs/target-worktree", "target"],
                );
            }
            _ => fs::write(f.dir.path().join(".eprfs/status/acceptance.json"), "{}").unwrap(),
        }
        assert!(!f.run().status.success(), "{mode} must refuse");
        assert_eq!(f.target(), f.target);
    }
}

#[test]
fn a_target_changed_to_a_symbolic_ref_cannot_redirect_the_transaction() {
    let f = fixture("pass");
    let root = f.dir.path();
    git(root, &["branch", "neighbor", &f.target]);
    let index_before = fs::read(root.join(".git/index")).unwrap();
    let discovery = command(root, "sh", &["-c", "command -v git"]);
    assert!(discovery.status.success(), "real Git must be discoverable");
    let real_git = String::from_utf8(discovery.stdout).unwrap();
    let real_git = real_git.trim();
    assert!(Path::new(real_git).is_absolute());
    let shims = root.join(".eprfs/git-race-shim");
    fs::create_dir_all(&shims).unwrap();
    let shim = shims.join("git");
    // The native preflight sees direct refs. Only the final transaction launch
    // replaces target with a same-OID symref, recreating the check/use race.
    fs::write(&shim, b"#!/bin/sh\nif [ \"$1\" = update-ref ]; then\n  \"$INTEGRATION_REAL_GIT\" symbolic-ref refs/heads/target refs/heads/neighbor || exit\n  : > .eprfs/target-raced\nfi\nexec \"$INTEGRATION_REAL_GIT\" \"$@\"\n").unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![shims];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let path = std::env::join_paths(paths).unwrap();
    let program = std::env::var("BRIT_INTEGRATION_BIN").unwrap_or_else(|_| evaluator_bin());
    let result = fixture_command(
        root,
        &program,
        &["integrate", "--acceptance", &f.acceptance],
    )
    .env("PATH", path)
    .env("INTEGRATION_REAL_GIT", real_git)
    .output()
    .unwrap();
    assert!(
        root.join(".eprfs/target-raced").is_file(),
        "test must reach the production Git ref transaction: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        git(root, &["rev-parse", "refs/heads/neighbor"]),
        f.target,
        "a raced symbolic target must never redirect the accepted update into its neighbor"
    );
    assert_eq!(git(root, &["rev-parse", "HEAD"]), f.source);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index_before);
    if result.status.success() {
        assert_eq!(
            f.target(),
            f.source,
            "a successful transaction updates only its named target"
        );
        assert!(
            !command(root, "git", &["symbolic-ref", "-q", "refs/heads/target"])
                .status
                .success(),
            "the accepted target update must be direct"
        );
    }
}
