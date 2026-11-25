//! `epr flow enrich`: immutable target, closed kind vocabulary, tuple idempotence and derived
//! verdict fields through the binary surface.
#![cfg(feature = "peer")]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Output};

use elohim_epr_cli::{device_key::DeviceKey, flow::misses::open_misses};
use elohim_epr_rea::{
    atom_cid, record_signing_message, ActorRecord, ActorStore, ActorWitness, AgentRef, FlowEvent,
    FlowRecord, FlowStore, Magnitude, ParticipantRow, ReaVerb, RecordSignature, SidecarActorStore,
    SidecarFlowStore, SidecarRoster,
};
use tempfile::TempDir;

const PROJECT: &str = "eprfs";
const CLOSURE: &str = "closure-a";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

struct Fixture {
    root: TempDir,
    keys: TempDir,
    did: String,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".epr-meta")).unwrap();
        std::fs::create_dir_all(root.path().join(".claude/epr-meta")).unwrap();
        std::fs::write(
            root.path().join(".epr-meta/repository.yaml"),
            "version: 1\nagent: repo:test/enrichment\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join(".claude/epr-meta/measures.yaml"),
            "measures:\n  - id: attestation-enrichment\n    version: 1\n    family: gate\n    unit: enrichments\n",
        )
        .unwrap();
        std::fs::write(root.path().join("README.md"), "fixture\n").unwrap();
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["config", "user.name", "Enrichment Test"]);
        git(
            root.path(),
            &["config", "user.email", "enrichment@example.test"],
        );
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-q", "-m", "fixture"]);

        let keys = tempfile::tempdir().unwrap();
        let key_file = keys.path().join("elohim/device/ed25519.seed");
        let key = DeviceKey::load_or_generate(&key_file).unwrap();
        let did = key.did_key();
        Self { root, keys, did }
    }

    fn records(&self) -> Vec<(cid::Cid, FlowRecord)> {
        SidecarFlowStore::open(self.root.path())
            .unwrap()
            .records()
            .unwrap()
    }

    fn append(&self, record: FlowRecord) -> cid::Cid {
        SidecarFlowStore::open(self.root.path())
            .unwrap()
            .append(record)
            .unwrap()
    }

    fn verdict(&self) -> cid::Cid {
        self.append(gate_verdict(PROJECT, CLOSURE, COMMIT, true, None))
    }

    fn enrich(&self, verdict: &str, outcome: &str, extra: &[&str]) -> Output {
        self.enrich_with(&["--verdict", verdict, "--outcome", outcome], extra)
    }

    fn enrich_selector(&self, project: &str, closure: &str, from_device: Option<&str>) -> Output {
        let mut selector = vec!["--project", project, "--closure", closure];
        if let Some(device) = from_device {
            selector.extend(["--from-device", device]);
        }
        self.enrich_with(&selector, &["--outcome", "green"])
    }

    fn enrich_with(&self, target: &[&str], extra: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .args([
                "flow",
                "enrich",
                "--root",
                self.root.path().to_str().unwrap(),
                "--json",
                "--kind",
                "re-executed",
                "--method",
                "gate-v3/judge-v1",
            ])
            .args(target)
            .args(extra)
            .env("XDG_CONFIG_HOME", self.keys.path())
            .env_remove("ELOHIM_DEVICE_KEY_FILE")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID");
        command.output().unwrap()
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.root.path().join(".eprfs/status/flows.jsonl")).unwrap()
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn success_json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn gate_verdict(
    project: &str,
    closure: &str,
    commit: &str,
    success: bool,
    via: Option<&str>,
) -> FlowRecord {
    let mut event = FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("human:fixture".into()),
        receiver: AgentRef("repo:test/enrichment".into()),
        resource: atom_cid(&"gate-verdict-resource".to_string()).unwrap(),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "run-note".into(),
        },
        process: None,
        in_scope_of: atom_cid(&"scope".to_string()).unwrap(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-25T00:00:00Z".into(),
        classified_as: vec![
            "run:observation".into(),
            ".".into(),
            "reason:verdict".into(),
            "measure:gate-verdict@1".into(),
            "value:1".into(),
            format!("env:project={project}"),
            format!("env:closure={closure}"),
            format!("env:commit={commit}"),
            format!("env:success={success}"),
            "env:seconds=12".into(),
        ],
    };
    if let Some(device) = via {
        event.classified_as.push(format!("via:{device}"));
    }
    FlowRecord::Event(event)
}

fn miss() -> FlowRecord {
    FlowRecord::Event(FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("human:fixture".into()),
        receiver: AgentRef("repo:test/enrichment".into()),
        resource: atom_cid(&"landing-intent".to_string()).unwrap(),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "landings".into(),
        },
        process: None,
        in_scope_of: atom_cid(&"scope".to_string()).unwrap(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-25T00:00:00Z".into(),
        classified_as: vec![
            "dev-landing:misses".into(),
            "I0123456789abcdef0123456789abcdef01234567".into(),
            "reason:misses".into(),
            "measure:dev-landing@1".into(),
            "value:1".into(),
            "env:change=I0123456789abcdef0123456789abcdef01234567".into(),
            format!("env:commit={COMMIT}"),
            "env:ref=refs/heads/self/enrichment-test".into(),
            "env:miss=other-project=other-closure".into(),
        ],
    })
}

fn enrichment_events(records: &[(cid::Cid, FlowRecord)]) -> Vec<(cid::Cid, &FlowEvent)> {
    records
        .iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Event(event)
                if event
                    .classified_as
                    .iter()
                    .any(|slot| slot == "measure:attestation-enrichment@1") =>
            {
                Some((*cid, event))
            }
            _ => None,
        })
        .collect()
}

fn env<'a>(event: &'a FlowEvent, name: &str) -> Option<&'a str> {
    let prefix = format!("env:{name}=");
    event
        .classified_as
        .iter()
        .find_map(|slot| slot.strip_prefix(&prefix))
}

fn signature_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn enrichment_appends_without_mutating_records_or_gate_decisions() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict();
    fixture.append(miss());
    let records_before = fixture.records();
    let cids_before: Vec<_> = records_before.iter().map(|(cid, _)| *cid).collect();
    let atoms_before: BTreeSet<_> = cids_before.iter().copied().collect();
    let misses_before = open_misses(&records_before, &|_| false, Some(&fixture.did));
    assert_eq!(misses_before.len(), 1);
    let bytes_before = fixture.log();

    let written = success_json(fixture.enrich(&verdict.to_string(), "green", &[]));
    assert_eq!(written["appended"], true);

    let bytes_after = fixture.log();
    assert!(bytes_after.starts_with(&bytes_before));
    let records_after = fixture.records();
    let preserved: BTreeSet<_> = records_after
        .iter()
        .map(|(cid, _)| *cid)
        .filter(|cid| atoms_before.contains(cid))
        .collect();
    assert_eq!(preserved, atoms_before);
    assert_eq!(
        records_after
            .iter()
            .take(cids_before.len())
            .map(|(cid, _)| *cid)
            .collect::<Vec<_>>(),
        cids_before
    );
    assert_eq!(
        open_misses(&records_after, &|_| false, Some(&fixture.did)),
        misses_before
    );
}

#[test]
fn identical_enrichment_is_idempotent() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict().to_string();
    let first = success_json(fixture.enrich(&verdict, "green", &["--seconds", "12"]));
    let count = fixture.records().len();
    let replay = success_json(fixture.enrich(&verdict, "green", &["--seconds", "99"]));
    assert_eq!(first["cid"], replay["cid"]);
    assert_eq!(replay["appended"], false);
    assert_eq!(fixture.records().len(), count);
    assert_eq!(enrichment_events(&fixture.records()).len(), 1);
}

#[test]
fn disagreement_is_a_second_record_and_preserves_verdict() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict();
    let verdict_record = fixture
        .records()
        .into_iter()
        .find(|(cid, _)| cid == &verdict)
        .unwrap()
        .1;
    let green = success_json(fixture.enrich(&verdict.to_string(), "green", &[]));
    let red = success_json(fixture.enrich(&verdict.to_string(), "red", &[]));
    assert_ne!(green["cid"], red["cid"]);
    let records = fixture.records();
    assert_eq!(enrichment_events(&records).len(), 2);
    assert_eq!(
        records.iter().find(|(cid, _)| cid == &verdict).unwrap().1,
        verdict_record
    );
}

#[test]
fn reserved_unknown_kinds_and_unknown_verdict_append_nothing() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict().to_string();
    let count = fixture.records().len();
    for (kind, expected) in [
        (
            "household-witnessed",
            "kind household-witnessed is reserved, not implemented",
        ),
        ("accepted", "kind accepted is reserved, not implemented"),
        ("invented", "unknown kind `invented`"),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        let output = command
            .args([
                "flow",
                "enrich",
                "--root",
                fixture.root.path().to_str().unwrap(),
                "--verdict",
                &verdict,
                "--kind",
                kind,
                "--outcome",
                "green",
                "--method",
                "judge-v1",
            ])
            .env("XDG_CONFIG_HOME", fixture.keys.path())
            .env_remove("ELOHIM_DEVICE_KEY_FILE")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let unknown = atom_cid(&"unknown-verdict".to_string()).unwrap();
    let output = fixture.enrich(&unknown.to_string(), "green", &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no gate-verdict@1 with CID"));
    assert_eq!(fixture.records().len(), count);
}

#[test]
fn enrichment_derives_project_closure_and_commit_from_verdict() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict();
    let run = atom_cid(&"reexecution-run".to_string()).unwrap();
    let forged = fixture.enrich(
        &verdict.to_string(),
        "green",
        &["--project", "forged-project"],
    );
    assert!(!forged.status.success());
    assert!(String::from_utf8_lossy(&forged.stderr)
        .contains("--verdict cannot be combined with --project, --closure, or --from-device"));

    let result = success_json(fixture.enrich(
        &verdict.to_string(),
        "green",
        &["--run", &run.to_string(), "--seconds", "19"],
    ));
    assert_eq!(result["project"], PROJECT);
    assert_eq!(result["closure"], CLOSURE);
    assert_eq!(result["device"], fixture.did);
    let records = fixture.records();
    let (_, event) = enrichment_events(&records).pop().unwrap();
    assert_eq!(event.resource, verdict);
    assert_eq!(event.provider.0, fixture.did);
    assert_eq!(env(event, "project"), Some(PROJECT));
    assert_eq!(env(event, "closure"), Some(CLOSURE));
    assert_eq!(env(event, "commit"), Some(COMMIT));
    assert_eq!(env(event, "verdict"), Some(verdict.to_string().as_str()));
    assert_eq!(env(event, "run"), Some(run.to_string().as_str()));
    assert_eq!(env(event, "seconds"), Some("19"));
}

#[test]
fn selector_resolves_own_newest_green_in_store_order() {
    let fixture = Fixture::new();
    let older = fixture.append(gate_verdict(
        PROJECT,
        CLOSURE,
        "1111111111111111111111111111111111111111",
        true,
        None,
    ));
    let newer = fixture.append(gate_verdict(
        PROJECT,
        CLOSURE,
        "2222222222222222222222222222222222222222",
        true,
        None,
    ));
    fixture.append(gate_verdict(
        PROJECT,
        CLOSURE,
        "3333333333333333333333333333333333333333",
        false,
        None,
    ));

    let result = success_json(fixture.enrich_selector(PROJECT, CLOSURE, None));
    assert_eq!(result["verdict"], newer.to_string());
    assert_ne!(result["verdict"], older.to_string());
    let records = fixture.records();
    let (_, event) = enrichment_events(&records).pop().unwrap();
    assert_eq!(event.resource, newer);
    assert_eq!(env(event, "verdict"), Some(newer.to_string().as_str()));
}

#[test]
fn selector_resolves_an_admitted_siblings_green_by_device() {
    let fixture = Fixture::new();
    let own_key = DeviceKey::load(&fixture.keys.path().join("elohim/device/ed25519.seed")).unwrap();
    let sibling_key =
        DeviceKey::load_or_generate(&fixture.keys.path().join("sibling.seed")).unwrap();
    let witness = ActorRecord::Witness(
        ActorWitness::new(
            "human:fixture",
            "agent:test@fixture-model",
            "flow-enrich-test",
            "operator present for the enrichment fixture",
            "2026-11-25T00:00:00Z",
        )
        .unwrap(),
    );
    let witness_cid = witness.cid().unwrap();
    let signed = ActorRecord::Signed(
        RecordSignature::new(
            &witness_cid,
            &own_key.did_key(),
            &own_key.sign(&record_signing_message(&witness_cid.to_string())),
        )
        .unwrap(),
    );
    let mut actor_store = SidecarActorStore::open(fixture.root.path()).unwrap();
    actor_store.append(witness).unwrap();
    actor_store.append(signed).unwrap();

    let own = own_key.did_key();
    let sibling = sibling_key.did_key();
    let mut genesis = ParticipantRow::Genesis {
        handle: "fixture".into(),
        chain_root: own.clone(),
        record_cid: witness_cid.to_string(),
        signer: own.clone(),
        signature: String::new(),
    };
    let genesis_signature = signature_hex(&own_key.sign(&genesis.signing_message().unwrap()));
    let ParticipantRow::Genesis { signature, .. } = &mut genesis else {
        unreachable!()
    };
    *signature = genesis_signature;
    let roster = SidecarRoster::open(fixture.root.path(), "fixture").unwrap();
    roster.append(genesis).unwrap();

    let mut binding = ParticipantRow::Binding {
        handle: "fixture".into(),
        chain_root: own.clone(),
        controller: sibling.clone(),
        authorized_by: own,
        sig_a: String::new(),
        sig_b: String::new(),
        nonce: "flow-enrich-sibling".into(),
    };
    let binding_message = binding.signing_message().unwrap();
    let sig_a = signature_hex(&own_key.sign(&binding_message));
    let sig_b = signature_hex(&sibling_key.sign(&binding_message));
    let ParticipantRow::Binding {
        sig_a: row_sig_a,
        sig_b: row_sig_b,
        ..
    } = &mut binding
    else {
        unreachable!()
    };
    *row_sig_a = sig_a;
    *row_sig_b = sig_b;
    roster.append(binding).unwrap();
    let verdict = fixture.append(gate_verdict(PROJECT, CLOSURE, COMMIT, true, Some(&sibling)));

    let result = success_json(fixture.enrich_selector(PROJECT, CLOSURE, Some(&sibling)));
    assert_eq!(result["verdict"], verdict.to_string());
    let records = fixture.records();
    let (_, event) = enrichment_events(&records).pop().unwrap();
    assert_eq!(event.resource, verdict);
    assert_eq!(env(event, "verdict"), Some(verdict.to_string().as_str()));
}

#[test]
fn selector_refuses_when_no_green_matches_and_appends_nothing() {
    let fixture = Fixture::new();
    fixture.append(gate_verdict(PROJECT, CLOSURE, COMMIT, false, None));
    let count = fixture.records().len();

    let output = fixture.enrich_selector(PROJECT, CLOSURE, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&format!(
        "no green gate-verdict@1 for {PROJECT} {CLOSURE} from {} in this device's fold",
        fixture.did
    )));
    assert_eq!(fixture.records().len(), count);
}

#[test]
fn verdict_and_project_selector_are_mutually_exclusive() {
    let fixture = Fixture::new();
    let verdict = fixture.verdict();
    let count = fixture.records().len();

    let output = fixture.enrich(
        &verdict.to_string(),
        "green",
        &["--project", PROJECT, "--closure", CLOSURE],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--verdict cannot be combined with --project, --closure, or --from-device"));
    assert_eq!(fixture.records().len(), count);
}
