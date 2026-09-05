use super::*;
use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(15);
    while !path.exists() {
        assert!(Instant::now() < until, "timeout: {}", path.display());
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn child(root: &Path, mode: &str, id: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "concurrent_actors::subprocess", "--nocapture"])
        .env("ACTOR_RACE_ROOT", root)
        .env("ACTOR_RACE_MODE", mode)
        .env("ACTOR_RACE_ID", id)
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn subprocess() {
    let Ok(root) = std::env::var("ACTOR_RACE_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let id = std::env::var("ACTOR_RACE_ID").unwrap();
    let mode = std::env::var("ACTOR_RACE_MODE").unwrap();
    fs::write(root.join(format!("ready-{id}")), "ready").unwrap();
    wait_for(&root.join("start"));
    if mode == "hold" {
        let _tx = SidecarActorStore::open(root)
            .unwrap()
            .transaction()
            .unwrap();
        fs::write(root.join("held"), "held").unwrap();
        wait_for(&root.join("release"));
    } else {
        let outcome = claim(root, CLAIMED, SESSION).unwrap();
        fs::write(
            root.join(format!("result-{id}")),
            serde_json::to_vec(&outcome).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn concurrent_actor_retries_are_one_current_claim() {
    let dir = fixture();
    let root = dir.path();
    let children: Vec<_> = (0..8)
        .map(|n| child(root, "claim", &n.to_string()))
        .collect();
    for n in 0..8 {
        wait_for(&root.join(format!("ready-{n}")));
    }
    fs::write(root.join("start"), "start").unwrap();
    for n in 0..8 {
        wait_for(&root.join(format!("result-{n}")));
    }
    for mut process in children {
        assert!(process.wait().unwrap().success());
    }
    let appended = (0..8)
        .filter(|n| {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join(format!("result-{n}"))).unwrap())
                    .unwrap();
            value["appended"] == true
        })
        .count();
    assert_eq!(appended, 1);
    assert_eq!(
        SidecarActorStore::open(root)
            .unwrap()
            .records()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn busy_actor_store_does_not_block_governance_or_fabricate_a_claim() {
    let dir = fixture();
    let root = dir.path();
    claim(root, CLAIMED, SESSION).unwrap();
    fs::write(root.join("start"), "start").unwrap();
    let mut holder = child(root, "hold", "holder");
    wait_for(&root.join("held"));
    let started = Instant::now();
    assert!(current(root, SESSION).is_err(), "busy is not unclaimed");
    let decision = elohim_epr_cli::govern::evaluate(
        root,
        &[
            "--path".into(),
            "README.md".into(),
            "--session".into(),
            SESSION.into(),
        ],
    )
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(decision["actor"]["source"], "unclaimed");
    assert!(decision["actor"]["claimCid"].is_null());
    fs::write(root.join("release"), "release").unwrap();
    assert!(holder.wait().unwrap().success());
    assert!(current(root, SESSION).unwrap().claim.is_some());
}

#[test]
fn cli_refuses_an_unterminated_actor_record_without_altering_evidence() {
    let dir = fixture();
    let root = dir.path();
    claim(root, CLAIMED, SESSION).unwrap();
    let path = root.join(".eprfs/status/actors.jsonl");
    let mut bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.pop(), Some(b'\n'));
    fs::write(&path, &bytes).unwrap();
    // The override permits a deterministic red probe of an already-built prior CLI,
    // without mutating shared source or treating a probabilistic race as reproduced.
    let binary = std::env::var_os("EPR_SIDECAR_PROBE_BIN")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_epr").into());
    let output = Command::new(binary)
        .args([
            "actor",
            "claim",
            "--as",
            CLAIMED,
            "--session",
            SESSION,
            "--json",
            "--root",
        ])
        .arg(root)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "unterminated evidence was accepted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("unterminated"));
    assert_eq!(fs::read(path).unwrap(), bytes);
}
