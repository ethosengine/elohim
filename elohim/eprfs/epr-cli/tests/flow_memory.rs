//! Local memory contracts, not peer acceptance. Synthetic roots own every write.
use std::path::Path;

use elohim_epr_cli::{
    actor,
    flow::{memory, note},
};
use serde_json::{json, Value};
use tempfile::TempDir;

fn write(root: &Path, path: &str, value: &Value) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn pin(root: &Path, path: &str) -> Value {
    memory::execute(root, "pin", Some(path), None).unwrap()["resource"].clone()
}
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let declaration: Value =
        serde_json::from_str(include_str!("../../../../.epr-meta/collective.json")).unwrap();
    write(root, ".epr-meta/collective.json", &declaration);
    write(
        root,
        "genesis/evidence.json",
        &json!({"observed":"Only narrow claims are supported"}),
    );
    for args in [
        vec!["init", "-q"],
        vec!["add", ".epr-meta/collective.json", "genesis/evidence.json"],
        vec!["commit", "-qm", "fixture"],
    ] {
        let status = elohim_epr_cli::process::build_command("git", &args, root, &[])
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    actor::claim(root, "agent:investigator@fixture", "one").unwrap();
    actor::claim(root, "agent:reviewer@fixture", "two").unwrap();
    dir
}
fn assertion(root: &Path, reach: &str) -> Value {
    json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "author":"agent:investigator@fixture","steward":"repo:ethosengine/elohim",
        "scope":"workspace","reach":reach,"concern":"qualified evidence",
        "claim":"Only the measured scope is supported","uncertainty":["Wider use remains untested"],
        "sources":[{"resource":pin(root,"genesis/evidence.json"),"reach":reach}],"supersedes":[],"contradicts":[]})
}
fn contribute(root: &Path, reach: &str) -> Value {
    write(root, "genesis/assertion.json", &assertion(root, reach));
    memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one"),
    )
    .unwrap()
}
fn projection(root: &Path, inputs: Vec<Value>, audience: &str) -> Value {
    json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),"purpose":"Recover narrow purpose",
        "audience":audience,"inputs":inputs,"omissions":["No population audit or external ranking"]})
}

#[test]
fn two_agents_replay_projection_feedback_and_independent_local_graduation() {
    let dir = fixture();
    let root = dir.path();
    let first = contribute(root, "repository");
    assert!(first["actorClaim"].is_string());
    assert_eq!(first["actorClaim"], first["event"]["actor_claim"]);
    assert_eq!(first["author"], first["event"]["actor"]);
    let repeated = memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one"),
    )
    .unwrap();
    assert_eq!(
        first["event"]["record_cid"],
        repeated["event"]["record_cid"]
    );
    assert_eq!(repeated["event"]["appended"], false);
    let input = pin(root, "genesis/assertion.json");
    write(
        root,
        "genesis/project-request.json",
        &projection(root, vec![input.clone()], "workspace"),
    );
    let projected =
        memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    assert_eq!(projected["retained"], false);
    assert_eq!(
        projected["receipt"]["items"][0]["assertion"]["uncertainty"][0],
        "Wider use remains untested"
    );
    write(root, "genesis/projection.json", &projected);
    write(
        root,
        "genesis/feedback.json",
        &json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "target":pin(root,"genesis/projection.json"),"kind":"misleading-projection",
        "passage":"Only the measured scope is supported","reason":"Do not omit the qualification"}),
    );
    let feedback =
        memory::execute(root, "feedback", Some("genesis/feedback.json"), Some("two")).unwrap();
    assert_eq!(feedback["author"], "agent:reviewer@fixture");
    assert_eq!(feedback["event"]["kind"], "run:correction");
    let review = note::note(
        root,
        "genesis/assertion.json",
        "verdict",
        "Approved only this narrow scope",
        None,
        Some("approved"),
        &note::NoteActor {
            as_ref: None,
            session: Some("two".into()),
        },
    )
    .unwrap();
    write(
        root,
        "genesis/graduation.json",
        &json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "contribution":input,"review":review.record_cid,"audience":"repository"}),
    );
    let graduate =
        memory::execute(root, "graduate", Some("genesis/graduation.json"), None).unwrap();
    assert_eq!(graduate["allowed"], true);
    assert!(graduate["standing"]
        .as_str()
        .unwrap()
        .contains("rehearsal only"));
}

#[test]
fn missing_stale_unknown_and_impersonated_inputs_refuse_before_append() {
    let dir = fixture();
    let root = dir.path();
    let mut value = assertion(root, "workspace");
    write(root, "genesis/assertion.json", &value);
    assert!(memory::execute(root, "contribute", Some("genesis/assertion.json"), None).is_err());
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("two")
    )
    .unwrap_err()
    .to_string()
    .contains("author differs"));
    value["extraAuthority"] = json!(true);
    write(root, "genesis/assertion.json", &value);
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .is_err());
    value.as_object_mut().unwrap().remove("extraAuthority");
    write(root, "genesis/assertion.json", &value);
    write(root, "genesis/evidence.json", &json!({"new":"contrary"}));
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .unwrap_err()
    .to_string()
    .contains("version changed"));
    assert!(
        std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
            .unwrap_or_default()
            .is_empty()
    );
    std::fs::remove_file(root.join("genesis/evidence.json")).unwrap();
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .is_err());
}

#[test]
fn contradictory_versions_survive_explicit_selection_and_replay() {
    let dir = fixture();
    let root = dir.path();
    contribute(root, "workspace");
    let first = pin(root, "genesis/assertion.json");
    let mut other = assertion(root, "workspace");
    other["author"] = json!("agent:reviewer@fixture");
    other["claim"] = json!("This evidence does not support the proposed inference");
    other["contradicts"] = json!([first.clone()]);
    write(root, "genesis/alternative.json", &other);
    memory::execute(
        root,
        "contribute",
        Some("genesis/alternative.json"),
        Some("two"),
    )
    .unwrap();
    write(
        root,
        "genesis/project-request.json",
        &projection(
            root,
            vec![first, pin(root, "genesis/alternative.json")],
            "workspace",
        ),
    );
    let one = memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    let two = memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    assert_eq!(one["receiptCid"], two["receiptCid"]);
    assert_eq!(one["receipt"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        one["receipt"]["items"][1]["assertion"]["contradicts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn source_policy_and_projection_metadata_prevent_reach_laundering() {
    let dir = fixture();
    let root = dir.path();
    let mut collective: Value =
        serde_json::from_slice(&std::fs::read(root.join(".epr-meta/collective.json")).unwrap())
            .unwrap();
    collective["sourceRules"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"genesis/private","maxReach":"private"}));
    write(root, ".epr-meta/collective.json", &collective);
    write(
        root,
        "genesis/private/evidence.json",
        &json!({"private":"must remain private"}),
    );
    let mut c = assertion(root, "repository");
    c["sources"][0]["resource"] = pin(root, "genesis/private/evidence.json");
    write(root, "genesis/assertion.json", &c);
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .unwrap_err()
    .to_string()
    .contains("policy forbids"));
    contribute(root, "workspace");
    write(
        root,
        "genesis/project-request.json",
        &projection(root, vec![pin(root, "genesis/assertion.json")], "workspace"),
    );
    let p = memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    write(root, "genesis/projection.json", &p);
    let mut wider = assertion(root, "repository");
    wider["sources"][0]["resource"] = pin(root, "genesis/projection.json");
    write(root, "genesis/wider.json", &wider);
    assert!(
        memory::execute(root, "contribute", Some("genesis/wider.json"), Some("one"))
            .unwrap_err()
            .to_string()
            .contains("metadata retains")
    );
}

#[test]
fn malformed_paths_changed_collective_and_byte_budget_refuse() {
    let dir = fixture();
    let root = dir.path();
    for path in [
        "../outside",
        "/etc/passwd",
        "genesis/../genesis/evidence.json",
    ] {
        assert!(memory::execute(root, "pin", Some(path), None).is_err());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            root.join("genesis/evidence.json"),
            root.join("genesis/link.json"),
        )
        .unwrap();
        assert!(
            memory::execute(root, "pin", Some("genesis/link.json"), None)
                .unwrap_err()
                .to_string()
                .contains("symlink")
        );
    }
    write(
        root,
        "genesis/assertion.json",
        &assertion(root, "workspace"),
    );
    let mut collective: Value =
        serde_json::from_slice(&std::fs::read(root.join(".epr-meta/collective.json")).unwrap())
            .unwrap();
    collective["charter"] = json!("Changed policy");
    write(root, ".epr-meta/collective.json", &collective);
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .unwrap_err()
    .to_string()
    .contains("collective declaration version"));
    std::fs::write(root.join("genesis/large"), vec![b'x'; 262145]).unwrap();
    assert!(memory::execute(root, "pin", Some("genesis/large"), None)
        .unwrap_err()
        .to_string()
        .contains("byte budget"));
}

#[test]
fn unrecorded_contribution_and_missing_saved_projection_do_not_become_context() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/assertion.json",
        &assertion(root, "workspace"),
    );
    write(
        root,
        "genesis/project-request.json",
        &projection(root, vec![pin(root, "genesis/assertion.json")], "workspace"),
    );
    assert!(memory::execute(root, "project", Some("genesis/project-request.json"), None).is_err());
    contribute(root, "workspace");
    let projected =
        memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    write(root, "genesis/projection.json", &projected);
    write(
        root,
        "genesis/feedback.json",
        &json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "target":pin(root,"genesis/projection.json"),"kind":"omitted-contradiction","passage":"Only the measured scope is supported","reason":"Inspect an alternative"}),
    );
    std::fs::remove_file(root.join("genesis/projection.json")).unwrap();
    assert!(memory::execute(root, "feedback", Some("genesis/feedback.json"), Some("two")).is_err());
}

#[test]
fn concurrent_identical_contributions_have_one_effect_and_discoverable_contract() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/assertion.json",
        &assertion(root, "workspace"),
    );
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            memory::execute(
                root,
                "contribute",
                Some("genesis/assertion.json"),
                Some("one"),
            )
            .unwrap()
        });
        let b = scope.spawn(|| {
            memory::execute(
                root,
                "contribute",
                Some("genesis/assertion.json"),
                Some("one"),
            )
            .unwrap()
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(
        results.0["event"]["record_cid"],
        results.1["event"]["record_cid"]
    );
    assert_ne!(
        results.0["event"]["appended"],
        results.1["event"]["appended"]
    );
    let collective = memory::execute(root, "collective", None, None).unwrap();
    assert_eq!(collective["inputGuide"]["graduate"]["readOnly"], true);
    assert!(collective["inputGuide"]["contribute"]["example"]["collective"]["cid"].is_string());
}

#[test]
fn self_review_restricted_reach_and_later_red_refuse_local_graduation() {
    let dir = fixture();
    let root = dir.path();
    contribute(root, "repository");
    let actor = |session: &str| note::NoteActor {
        as_ref: None,
        session: Some(session.into()),
    };
    let own = note::note(
        root,
        "genesis/assertion.json",
        "verdict",
        "I approve my own assertion",
        None,
        Some("approved"),
        &actor("one"),
    )
    .unwrap();
    let request = |review: String| {
        json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "contribution":pin(root,"genesis/assertion.json"),"review":review,"audience":"repository"})
    };
    write(root, "genesis/graduation.json", &request(own.record_cid));
    assert!(
        memory::execute(root, "graduate", Some("genesis/graduation.json"), None)
            .unwrap_err()
            .to_string()
            .contains("independent")
    );
    let good = note::note(
        root,
        "genesis/assertion.json",
        "verdict",
        "Narrow approval",
        None,
        Some("approved"),
        &actor("two"),
    )
    .unwrap();
    write(root, "genesis/graduation.json", &request(good.record_cid));
    note::note(
        root,
        "genesis/assertion.json",
        "verdict",
        "New contrary evidence",
        None,
        Some("changes-requested"),
        &actor("two"),
    )
    .unwrap();
    assert!(
        memory::execute(root, "graduate", Some("genesis/graduation.json"), None)
            .unwrap_err()
            .to_string()
            .contains("later contrary")
    );
    contribute(root, "workspace");
    write(
        root,
        "genesis/graduation.json",
        &request("unavailable".into()),
    );
    assert!(
        memory::execute(root, "graduate", Some("genesis/graduation.json"), None)
            .unwrap_err()
            .to_string()
            .contains("restriction forbids")
    );
}

#[test]
fn private_feedback_and_request_metadata_cannot_launder_repository_reach() {
    let dir = fixture();
    let root = dir.path();
    contribute(root, "repository");
    // Public assertions do not authorize the workspace-only request's purpose and omissions.
    write(
        root,
        ".claude/memory-kit/request.json",
        &projection(
            root,
            vec![pin(root, "genesis/assertion.json")],
            "repository",
        ),
    );
    assert!(memory::execute(
        root,
        "project",
        Some(".claude/memory-kit/request.json"),
        None
    )
    .unwrap_err()
    .to_string()
    .contains("policy forbids"));
    let mut collective: Value =
        serde_json::from_slice(&std::fs::read(root.join(".epr-meta/collective.json")).unwrap())
            .unwrap();
    collective["sourceRules"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"genesis/private","maxReach":"private"}));
    write(root, ".epr-meta/collective.json", &collective);
    write(
        root,
        "genesis/private/target.json",
        &json!({"claim":"A private passage"}),
    );
    write(
        root,
        "genesis/feedback-request.json",
        &json!({"version":1,"collective":pin(root,".epr-meta/collective.json"),
        "target":pin(root,"genesis/private/target.json"),"kind":"stale-source","passage":"A private passage","reason":"Needs qualification"}),
    );
    let feedback = memory::execute(
        root,
        "feedback",
        Some("genesis/feedback-request.json"),
        Some("two"),
    )
    .unwrap();
    assert_eq!(feedback["effectiveReach"], "private");
    write(root, "genesis/saved-feedback.json", &feedback);
    for source in [
        "genesis/saved-feedback.json",
        "genesis/feedback-request.json",
    ] {
        let mut c = assertion(root, "repository");
        c["sources"][0]["resource"] = pin(root, source);
        write(root, "genesis/laundered.json", &c);
        assert!(memory::execute(
            root,
            "contribute",
            Some("genesis/laundered.json"),
            Some("one")
        )
        .unwrap_err()
        .to_string()
        .contains("metadata retains"));
    }
}

#[test]
fn file_references_use_raw_codec_and_preserve_frontmatter_bytes() {
    let dir = fixture();
    let root = dir.path();
    let first = b"---\nrestriction: private\n---\nbody\n";
    std::fs::write(root.join("genesis/raw.md"), first).unwrap();
    let reference = pin(root, "genesis/raw.md");
    assert_eq!(
        reference["cid"],
        eprfs_core::BlobCid::compute_raw(first).to_string()
    );
    let cid: cid::Cid = reference["cid"].as_str().unwrap().parse().unwrap();
    assert_eq!(cid.codec(), 0x55);
    std::fs::write(
        root.join("genesis/raw.md"),
        b"---\nrestriction: repository\n---\nbody\n",
    )
    .unwrap();
    assert_ne!(reference["cid"], pin(root, "genesis/raw.md")["cid"]);
}

#[test]
fn contribution_bounds_existing_native_log_before_note_reader_runs() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/assertion.json",
        &assertion(root, "workspace"),
    );
    std::fs::File::create(root.join(".eprfs/status/flows.jsonl"))
        .unwrap()
        .set_len(33554433)
        .unwrap();
    assert!(memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("one")
    )
    .unwrap_err()
    .to_string()
    .contains("sidecar exceeds"));
}
