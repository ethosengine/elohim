//! Real OS-process coordination proofs; child gates are fixture files, not sleeps used as
//! evidence that a race happened. Timeouts only bound a broken test's wait.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use elohim_epr_rea::{
    ActorClaim, ActorRecord, ActorStore, FlowRecord, FlowStore, SidecarActorStore, SidecarFlowStore,
};

fn actor(worker: &str) -> ActorRecord {
    ActorRecord::Claim(
        ActorClaim::new("agent:worker@gpt-6", worker, "2026-09-05T00:00:00Z", None).unwrap(),
    )
}

fn flow(worker: &str) -> FlowRecord {
    let mut event = super::event(
        super::ReaVerb::Use,
        &super::resource(worker),
        super::count(1.0, "test"),
        &super::resource("scope"),
        None,
        vec![],
    );
    event.classified_as = vec![worker.to_string()];
    FlowRecord::Event(event)
}

fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(15);
    while !path.exists() {
        assert!(
            Instant::now() < until,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn spawn(root: &Path, mode: &str, id: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "concurrent_sidecars::subprocess", "--nocapture"])
        .env("SIDECAR_TEST_ROOT", root)
        .env("SIDECAR_TEST_MODE", mode)
        .env("SIDECAR_TEST_ID", id)
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
}

fn finish(mut child: Child) {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "child failed: {status}");
            return;
        }
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("sidecar child deadlocked");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn subprocess() {
    let Ok(root) = std::env::var("SIDECAR_TEST_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let mode = std::env::var("SIDECAR_TEST_MODE").unwrap();
    let id = std::env::var("SIDECAR_TEST_ID").unwrap();
    fs::write(root.join(format!("ready-{id}")), "ready").unwrap();
    wait_for(&root.join("start"));
    match mode.as_str() {
        "append" => {
            let mut actors = SidecarActorStore::open(root).unwrap();
            let mut flows = SidecarFlowStore::open(root).unwrap();
            for n in 0..12 {
                let key = format!("{id}-{n}");
                actors.append(actor(&key)).unwrap();
                flows.append(flow(&key)).unwrap();
            }
        }
        "hold-flow" => {
            let mut tx = SidecarFlowStore::open(root).unwrap().transaction().unwrap();
            tx.append(flow("before-pause")).unwrap();
            fs::write(root.join("held"), "held").unwrap();
            wait_for(&root.join("release"));
            tx.append(flow("after-pause")).unwrap();
        }
        "read-flow" => {
            fs::write(root.join("reading"), "reading").unwrap();
            let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
            assert_eq!(records.len(), 2);
            fs::write(root.join("read-complete"), "complete").unwrap();
        }
        "crash-flow" | "crash-actor" => {
            let name = if mode == "crash-flow" {
                "flows.jsonl"
            } else {
                "actors.jsonl"
            };
            let path = root.join(".eprfs/status").join(name);
            let mut file = OpenOptions::new().append(true).open(path).unwrap();
            file.lock().unwrap();
            file.write_all(b"{\"cid\":\"interrupted").unwrap();
            file.sync_all().unwrap();
            fs::write(root.join("held"), "partial bytes synced").unwrap();
            wait_for(&root.join("never-release"));
        }
        other => panic!("unknown child mode {other}"),
    }
}

#[test]
fn independent_processes_first_open_and_append_preserve_every_record() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let children: Vec<_> = (0..8)
        .map(|n| spawn(root, "append", &n.to_string()))
        .collect();
    for n in 0..8 {
        wait_for(&root.join(format!("ready-{n}")));
    }
    assert!(
        !root.join(".eprfs").exists(),
        "all processes race the first open"
    );
    fs::write(root.join("start"), "start").unwrap();
    for child in children {
        finish(child);
    }
    let actors = SidecarActorStore::open(root).unwrap().records().unwrap();
    let flows = SidecarFlowStore::open(root).unwrap().records().unwrap();
    assert_eq!(actors.len(), 96);
    assert_eq!(flows.len(), 96);
    for n in 0..8 {
        for item in 0..12 {
            let key = format!("{n}-{item}");
            assert!(actors.contains(&(actor(&key).cid().unwrap(), actor(&key))));
            assert!(flows.contains(&(flow(&key).cid().unwrap(), flow(&key))));
        }
    }
}

#[test]
fn flow_reader_observes_a_complete_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("start"), "start").unwrap();
    let writer = spawn(root, "hold-flow", "writer");
    wait_for(&root.join("held"));
    let reader = spawn(root, "read-flow", "reader");
    wait_for(&root.join("reading"));
    // The writer has deliberately stopped between appends. Reader cannot accept the prefix.
    std::thread::sleep(Duration::from_millis(50));
    assert!(!root.join("read-complete").exists());
    fs::write(root.join("release"), "release").unwrap();
    finish(writer);
    finish(reader);
}

#[test]
fn killed_writer_releases_lock_but_preserves_and_refuses_partial_evidence() {
    for mode in ["crash-flow", "crash-actor"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut actors = SidecarActorStore::open(root).unwrap();
        let mut flows = SidecarFlowStore::open(root).unwrap();
        actors.append(actor("accepted")).unwrap();
        flows.append(flow("accepted")).unwrap();
        fs::write(root.join("start"), "start").unwrap();
        let mut child = spawn(root, mode, "crash");
        wait_for(&root.join("held"));
        child.kill().unwrap();
        child.wait().unwrap();
        let path = if mode == "crash-flow" {
            flows.log_path().to_path_buf()
        } else {
            actors.log_path().to_path_buf()
        };
        let before = fs::read(&path).unwrap();
        if mode == "crash-flow" {
            assert!(flows
                .records()
                .unwrap_err()
                .to_string()
                .contains("unterminated"));
            assert!(flows.append(flow("must-refuse")).is_err());
        } else {
            assert!(actors
                .records()
                .unwrap_err()
                .to_string()
                .contains("unterminated"));
            assert!(actors.append(actor("must-refuse")).is_err());
        }
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn complete_json_without_terminator_is_not_accepted_as_a_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SidecarActorStore::open(dir.path()).unwrap();
    store.append(actor("accepted")).unwrap();
    let mut bytes = fs::read(store.log_path()).unwrap();
    assert_eq!(bytes.pop(), Some(b'\n'));
    fs::write(store.log_path(), &bytes).unwrap();
    assert!(store.records().is_err());
    assert!(store.append(actor("retry")).is_err());
    assert_eq!(fs::read(store.log_path()).unwrap(), bytes);
}

#[test]
fn representative_log_read_check_and_batch_cost() {
    let dir = tempfile::tempdir().unwrap();
    let store = SidecarFlowStore::open(dir.path()).unwrap();
    let start = Instant::now();
    let mut tx = store.transaction().unwrap();
    for n in 0..1000 {
        tx.append(flow(&n.to_string())).unwrap();
    }
    let batch = start.elapsed();
    drop(tx);
    let start = Instant::now();
    let records = store.records().unwrap();
    let found = records
        .iter()
        .any(|(cid, _)| *cid == flow("999").cid().unwrap());
    assert!(found);
    eprintln!(
        "SIDECAR_COST records={} bytes={} batch={batch:?} read_check={:?}",
        records.len(),
        fs::metadata(store.log_path()).unwrap().len(),
        start.elapsed()
    );
}
