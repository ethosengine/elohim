//! Authoring acceptance: one native classifier, two harnesses, private reconstructable evidence.
use elohim_epr_cli::seam::{self, adapters, declarations};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}
fn put(root: &Path, path: &str, body: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
const DECLARATIONS: &[&str] = &[
    "elohim/sdk/working-version.seam.json",
    "elohim/elohim-storage/.epr-meta/publication-authority.seam.json",
    "elohim/elohim-storage/.epr-meta/peer-serving.seam.json",
    "elohim/eprfs/epr-cli/developer-composition.seam.json",
];
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let method = ".epr-meta/elohim/algorithms/seam-guidance.json";
    put(
        dir.path(),
        method,
        &fs::read_to_string(repo().join(method)).unwrap(),
    );
    for path in DECLARATIONS {
        let raw = fs::read_to_string(repo().join(path)).unwrap();
        put(dir.path(), path, &raw);
        let d: Value = serde_json::from_str(&raw).unwrap();
        for source in d["authoritativeContracts"]
            .as_array()
            .unwrap()
            .iter()
            .chain(d["reusableInterfaces"].as_array().unwrap())
        {
            let path = source["path"].as_str().unwrap();
            put(
                dir.path(),
                path,
                &fs::read_to_string(repo().join(path)).unwrap(),
            );
        }
    }
    let refs = DECLARATIONS
        .iter()
        .map(|p| format!("        - {p}\n"))
        .collect::<String>();
    let root_body = fs::read_to_string(repo().join(".epr-meta/manifest.md")).unwrap();
    let body = root_body.splitn(3, "---").nth(2).unwrap();
    put(dir.path(), ".epr-meta/manifest.md", &format!("---\nepr-meta-version: 1\nid: fixture\nroot: true\nrules:\n  - id: seams\n    class: inject\n    when: {{write: '*'}}\n    validator: epr:validator-native-seam-guidance\n    parameters:\n      algorithm: .epr-meta/elohim/algorithms/seam-guidance.json\n      declarations:\n{refs}---\n{body}"));
    dir
}
fn event(path: &str, content: &str) -> Value {
    json!({"session_id": "fixture-session", "tool_name": "Write", "tool_input": {"file_path": path, "content": content}})
}
fn classify(root: &Path, path: &str, content: &str) -> Value {
    seam::evaluate(
        root,
        &json!({"harness":"claude", "event":event(path,content)}),
    )
}
fn codex(path: &str, content: &str) -> Value {
    let lines = content
        .lines()
        .map(|l| format!("+{l}\n"))
        .collect::<String>();
    json!({"session_id":"fixture-session", "tool_name":"apply_patch", "tool_input":{"command":format!("*** Begin Patch\n*** Add File: {path}\n{lines}*** End Patch")}})
}

#[test]
fn labeled_corpus_and_harness_parity() {
    let dir = fixture();
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/seam-guidance-corpus.json")).unwrap();
    for row in corpus.as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let body = row["content"].as_str().unwrap();
        let a = classify(dir.path(), path, body);
        assert_eq!(a["coverageGaps"], json!([]), "{}: {a}", row["label"]);
        let b = seam::evaluate(
            dir.path(),
            &json!({"harness":"codex","event":codex(path,body)}),
        );
        assert_eq!(a, b, "{}", row["label"]);
        let actual = a["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == row["family"])
            .map(|c| c["kind"].clone())
            .unwrap_or(Value::Null);
        assert_eq!(actual, row["expected"], "{}: {a}", row["label"]);
        assert!(a["packet"].as_str().unwrap().len() <= 8192);
    }
    assert!(
        !dir.path().join(".eprfs").exists(),
        "classification is read-only and private"
    );
    assert!(!dir.path().join("app").exists(), "patch never executes");
}

#[test]
fn reviews_bind_exact_proposal_declarations_contracts_and_citations() {
    let dir = fixture();
    let input = json!({"harness":"claude","event":event("app/new.ts", "const db = indexedDB.open('draft');\n")});
    let report = seam::evaluate(dir.path(), &input);
    let citation = report["candidates"][0]["reuseCandidates"][0].clone();
    for decision in ["keep", "route", "split", "abstain"] {
        let mut request = input.clone();
        request["review"] = json!({"proposalId":report["proposalId"], "decision":decision,"rationale":"Evidence supports this bounded judgment", "limitations":"Fixture judgment is not architectural proof", "citations":[citation]});
        let result = seam::evaluate(dir.path(), &request);
        assert_eq!(result["review"]["status"], "validated", "{result}");
        assert_eq!(result["packet"], "");
        request["event"]["tool_input"]["content"] = json!("const db = indexedDB.open('other');\n");
        assert_eq!(
            seam::evaluate(dir.path(), &request)["review"]["status"],
            "invalid"
        );
    }
    let mut request = input;
    request["review"] = json!({"proposalId":report["proposalId"], "decision":"route","rationale":"route", "limitations":"limited", "citations":[citation]});
    request["review"]["citations"][0]["startByte"] = json!(999);
    assert_eq!(
        seam::evaluate(dir.path(), &request)["review"]["status"],
        "invalid"
    );
    request["review"]["citations"][0] = citation;
    let source = request["review"]["citations"][0]["path"].as_str().unwrap();
    let body = fs::read_to_string(dir.path().join(source)).unwrap();
    put(
        dir.path(),
        source,
        &format!("{body}\n// contract changed\n"),
    );
    assert_eq!(
        seam::evaluate(dir.path(), &request)["review"]["status"],
        "invalid"
    );
    let new = seam::evaluate(dir.path(), &request);
    request["review"]["proposalId"] = new["proposalId"].clone();
    let dpath = DECLARATIONS[0];
    let mut d: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(dpath)).unwrap()).unwrap();
    d["responsibility"] = json!("changed declaration");
    put(dir.path(), dpath, &d.to_string());
    assert_ne!(
        seam::evaluate(dir.path(), &request)["proposalId"],
        new["proposalId"]
    );
}

#[test]
fn invalid_declarations_boundaries_and_nearest_rule() {
    let dir = fixture();
    put(dir.path(), DECLARATIONS[0], "{}");
    let report = classify(dir.path(), "app/x.ts", "indexedDB.open('draft');\n");
    assert!(!report["coverageGaps"].as_array().unwrap().is_empty());
    fs::create_dir_all(dir.path().join("nested/.git")).unwrap();
    assert!(adapters::normalize(dir.path(), "claude", &event("nested/x.ts", "hi")).is_err());
    assert!(adapters::normalize(dir.path(), "claude", &event("../x.ts", "hi")).is_err());
    put(dir.path(), "app/.epr-meta", "---\nepr-meta-version: 1\nid: nested\nrules:\n  - id: seams\n    class: inject\n    when: {write: '*'}\n    validator: epr:validator-native-seam-guidance\n    parameters: {algorithm: .epr-meta/elohim/algorithms/seam-guidance.json, declarations: [local.seam.json]}\n---\n");
    let d = fs::read_to_string(repo().join(DECLARATIONS[0])).unwrap();
    put(dir.path(), "local.seam.json", &d);
    let report = classify(dir.path(), "app/x.ts", "indexedDB.open('draft');\n");
    assert_eq!(
        report["candidates"][0]["declaration"], "local.seam.json",
        "{report}"
    );
    assert_eq!(report["coverageGaps"], json!([]));
}

#[test]
fn patch_operations_unicode_and_stale_prior() {
    let dir = fixture();
    put(dir.path(), "a.ts", "const title = 'שלום';\nconst n = 1;\n");
    put(dir.path(), "delete.ts", "gone\n");
    let patch = "*** Begin Patch\n*** Update File: a.ts\n*** Move to: b.ts\n@@\n const title = 'שלום';\n-const n = 1;\n+const n = indexedDB.open('draft');\n*** Delete File: delete.ts\n*** Add File: c.ts\n+ok\n*** End Patch";
    let event = json!({"session_id":"s","tool_name":"apply_patch","tool_input":{"command":patch}});
    let proposal = adapters::normalize(dir.path(), "codex", &event).unwrap();
    assert_eq!(proposal.changes.len(), 3);
    assert_eq!(proposal.changes[0].operation, "rename");
    assert!(dir.path().join("a.ts").exists());
    assert!(!dir.path().join("b.ts").exists());
    let r = seam::evaluate(dir.path(), &json!({"proposal":proposal}));
    let span = &r["candidates"][0]["evidence"][0];
    let text = proposal.changes[0].proposed.as_ref().unwrap();
    assert_eq!(
        &text[span["startByte"].as_u64().unwrap() as usize
            ..span["endByte"].as_u64().unwrap() as usize],
        "indexedDB.open("
    );
    put(dir.path(), "a.ts", "changed\n");
    assert!(adapters::validate(dir.path(), &proposal).is_err());
    for patch in [
        "bad",
        "*** Begin Patch\n*** Update File: a.ts\n@@\n-absent\n+new\n*** End Patch",
        "*** Begin Patch\n*** Add File: ../escape\n+x\n*** End Patch",
    ] {
        let e = json!({"session_id":"s","tool_name":"apply_patch","tool_input":{"command":patch}});
        assert!(adapters::normalize(dir.path(), "codex", &e).is_err());
    }
}

#[test]
fn unsupported_events_size_limits_and_verified_sources() {
    let dir = fixture();
    assert!(adapters::normalize(
        dir.path(),
        "claude",
        &json!({"session_id":"s","tool_name":"Bash","tool_input":{"command":"echo hi > x"}})
    )
    .is_err());
    assert!(adapters::normalize(dir.path(), "antigravity", &event("a.ts", "x")).is_err());
    assert_eq!(adapters::capabilities("antigravity")["adapter"], "deferred");
    assert!(adapters::normalize(
        dir.path(),
        "claude",
        &event("a.ts", &"x".repeat(declarations::MAX_BYTES + 1))
    )
    .is_err());
    put(
        dir.path(),
        "elohim/sdk/storage-client-ts/src/sync.ts",
        "export class Other {}\n",
    );
    let r = classify(dir.path(), "app/x.ts", "indexedDB.open('draft');\n");
    assert!(r["coverageGaps"]
        .to_string()
        .contains("source anchor absent"));
    assert!(!r["candidates"][0]["reuseCandidates"]
        .to_string()
        .contains("AutomergeSync"));
}

#[test]
fn governed_method_and_application_are_pinned_and_executed() {
    let dir = fixture();
    let input = json!({"harness":"claude", "event":event("app/new.ts", "const db = indexedDB.open('draft');\n")});
    let before = seam::evaluate(dir.path(), &input);
    let path = ".epr-meta/elohim/algorithms/seam-guidance.json";
    let mut method: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(path)).unwrap()).unwrap();
    method["application"]["maxCandidates"] = json!(1);
    method["application"]["packetBytes"] = json!(2048);
    put(dir.path(), path, &method.to_string());
    let after = seam::evaluate(dir.path(), &input);
    assert_ne!(before["proposalId"], after["proposalId"]);
    assert_ne!(
        before["methods"][0]["algorithmId"],
        after["methods"][0]["algorithmId"]
    );
    assert_ne!(
        before["methods"][0]["applicationId"],
        after["methods"][0]["applicationId"]
    );
    assert_eq!(
        before["methods"][0]["implementationId"],
        after["methods"][0]["implementationId"]
    );
    assert!(after["packet"].as_str().unwrap().len() <= 2048);
    method["application"]["mode"] = json!("blocking");
    put(dir.path(), path, &method.to_string());
    assert!(seam::evaluate(dir.path(), &input)["coverageGaps"]
        .to_string()
        .contains("unsupported classifier"));
}

#[test]
fn event_cwd_and_edit_reconstruction_agree_with_patch() {
    let dir = fixture();
    put(dir.path(), "app/existing.ts", "const n = 1;\n");
    let cwd = dir.path().join("app");
    let claude = json!({"session_id":"s","cwd":cwd,"tool_name":"Edit","tool_input":{"file_path":"existing.ts", "old_string":"1", "new_string":"indexedDB.open('draft')"}});
    let codex = json!({"session_id":"s","cwd":cwd,"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Update File: existing.ts\n@@\n-const n = 1;\n+const n = indexedDB.open('draft');\n*** End Patch"}});
    let a = adapters::normalize(dir.path(), "claude", &claude).unwrap();
    let b = adapters::normalize(dir.path(), "codex", &codex).unwrap();
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    assert_eq!(a.changes[0].path, "app/existing.ts");
}

#[test]
fn abstention_cannot_hide_failure_and_invalid_method_revisions_invalidate_review() {
    let dir = fixture();
    let path = ".epr-meta/elohim/algorithms/seam-guidance.json";
    put(dir.path(), path, "{}");
    let mut input =
        json!({"harness":"claude", "event":event("app/new.ts", "indexedDB.open('draft');\n")});
    let before = seam::evaluate(dir.path(), &input);
    input["review"] = json!({"proposalId":before["proposalId"],"decision":"abstain","rationale":"method invalid","limitations":"cannot review","citations":[]});
    let after = seam::evaluate(dir.path(), &input);
    assert_eq!(after["review"]["status"], "validated");
    assert!(!after["packet"].as_str().unwrap().is_empty());
    put(dir.path(), path, "{ }");
    assert_eq!(
        seam::evaluate(dir.path(), &input)["review"]["status"],
        "invalid"
    );
}
