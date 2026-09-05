use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use super::*;

fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(20);
    while !path.exists() {
        assert!(
            Instant::now() < until,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn finish(mut child: Child) {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "child failed: {status}");
            return;
        }
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("flow child deadlocked");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn subprocess() {
    let Ok(root) = std::env::var("FLOW_RACE_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let id = std::env::var("FLOW_RACE_ID").unwrap();
    let session = std::env::var("FLOW_RACE_SESSION").unwrap();
    let mode = std::env::var("FLOW_RACE_MODE").unwrap();
    fs::write(root.join(format!("ready-{id}")), "ready").unwrap();
    wait_for(&root.join("start"));
    let who = NoteActor {
        as_ref: None,
        session: Some(session),
    };
    let result = match mode.as_str() {
        "claim" => claim::claim(root, &request("epic#1", &who))
            .map(|result| serde_json::to_value(result).unwrap()),
        "note" => note::note(
            root,
            "plans/epic.md",
            "observation",
            "same retried observation",
            None,
            None,
            &who,
        )
        .map(|result| serde_json::to_value(result).unwrap()),
        "fulfill" => fulfill::fulfill_on(root, &fulfil_request("epic#1", "DONE", &who))
            .map(|result| serde_json::to_value(result).unwrap()),
        "project" => project::project(root, &root.join(".claude/epr-meta/recipes.yaml"))
            .map(|_| serde_json::json!({"projected": true})),
        _ => panic!("unknown mode"),
    };
    let output = match result {
        Ok(value) => value,
        Err(error) => serde_json::json!({"error": error.to_string()}),
    };
    fs::write(
        root.join(format!("result-{id}")),
        serde_json::to_vec(&output).unwrap(),
    )
    .unwrap();
}

fn race(root: &Path, mode: &str, sessions: &[String]) -> Vec<serde_json::Value> {
    let start = root.join("start");
    if start.exists() {
        fs::remove_file(&start).unwrap();
    }
    let tx = SidecarFlowStore::open(root).unwrap().transaction().unwrap();
    let children: Vec<_> = sessions
        .iter()
        .enumerate()
        .map(|(n, session)| {
            let id = format!("{mode}-{n}");
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "concurrent_flows::subprocess", "--nocapture"])
                .env("FLOW_RACE_ROOT", root)
                .env("FLOW_RACE_ID", id)
                .env("FLOW_RACE_SESSION", session)
                .env("FLOW_RACE_MODE", mode)
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    for n in 0..sessions.len() {
        wait_for(&root.join(format!("ready-{mode}-{n}")));
    }
    fs::write(start, "start").unwrap();
    drop(tx);
    for child in children {
        finish(child);
    }
    (0..sessions.len())
        .map(|n| {
            serde_json::from_slice(&fs::read(root.join(format!("result-{mode}-{n}"))).unwrap())
                .unwrap()
        })
        .collect()
}

#[test]
fn competing_same_role_workers_have_one_winner_and_preserve_its_exact_pin() {
    let dir = valueflow_fixture();
    let root = dir.path();
    let sessions: Vec<_> = (0..8).map(|n| format!("worker-{n}")).collect();
    let claims: Vec<_> = sessions
        .iter()
        .map(|s| elohim_epr_cli::actor::claim(root, "agent:implementer@gpt-6", s).unwrap())
        .collect();
    let outcomes = race(root, "claim", &sessions);
    let winners: Vec<_> = outcomes
        .iter()
        .enumerate()
        .filter(|(_, out)| out["appended"] == true)
        .collect();
    assert_eq!(winners.len(), 1, "{outcomes:?}");
    for out in &outcomes {
        if let Some(error) = out["error"].as_str() {
            assert!(error.contains("already claimed"), "{error}");
        }
    }
    assert_eq!(
        outcomes
            .iter()
            .filter(|out| out.get("error").is_some())
            .count(),
        7
    );
    let (winner, outcome) = winners[0];
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    let (_, FlowRecord::Commitment(commitment)) = records
        .iter()
        .find(|(cid, _)| cid.to_string() == outcome["commitment_cid"].as_str().unwrap())
        .unwrap()
    else {
        panic!("commitment");
    };
    assert!(commitment
        .resource_spec
        .classified_as
        .contains(&format!("actor-claim:{}", claims[winner].record_cid)));
}

#[test]
fn concurrent_exact_claim_note_and_fulfillment_retries_append_once() {
    let dir = valueflow_fixture();
    let root = dir.path();
    elohim_epr_cli::actor::claim(root, "agent:implementer@gpt-6", "same-worker").unwrap();
    let sessions = vec!["same-worker".to_string(); 8];
    for mode in ["claim", "note", "fulfill"] {
        let before = SidecarFlowStore::open(root)
            .unwrap()
            .records()
            .unwrap()
            .len();
        let outcomes = race(root, mode, &sessions);
        assert!(
            outcomes.iter().all(|out| out.get("error").is_none()),
            "{mode}: {outcomes:?}"
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|out| out["appended"] == true)
                .count(),
            1,
            "{mode}: {outcomes:?}"
        );
        assert_eq!(
            SidecarFlowStore::open(root)
                .unwrap()
                .records()
                .unwrap()
                .len(),
            before + 1
        );
    }
}

#[test]
fn concurrent_projection_preserves_record_and_semantic_deduplication() {
    let dir = valueflow_fixture();
    let root = dir.path();
    write(
        root,
        "gap-items/extra.json",
        r#"{"doc":"plans/epic.md","items":[{"id":"epic#3","state":"OPEN"}]}"#,
    );
    let outcomes = race(root, "project", &vec!["projection-worker".to_string(); 6]);
    assert!(outcomes.iter().all(|out| out.get("error").is_none()));
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    assert_eq!(records.iter().filter(|(_, r)| matches!(r, FlowRecord::Intent(i) if i.resource_spec.classified_as.get(1).is_some_and(|id| id == "epic#3"))).count(), 1);
    let cids: std::collections::HashSet<_> = records.iter().map(|(cid, _)| *cid).collect();
    assert_eq!(cids.len(), records.len());
}
