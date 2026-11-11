//! Private retained feedback is evidence for its original intent, never acceptance authority.
use std::{collections::BTreeMap, path::Path};

use cid::Cid;
use elohim_epr_cli::{
    actor,
    flow::{self, acceptance::EvidenceResources, reconciliation, FlowError, FlowResult},
};
use elohim_epr_rea::{
    feedback::{Entry, Feedback, Occurrence, Outcome, Policy, Response, Source},
    feedback_capacity::Capacity,
    feedback_store::FeedbackStore,
    AgentRef, Commitment, CommitmentState, FlowRecord, FlowStore, Intent, MemoryFlowStore, ReaVerb,
    ResourceSpec, SidecarFlowStore,
};
use serde_json::json;

fn policy() -> Policy {
    Policy {
        id: "fixture@1".into(),
        authority: "human:fixture".into(),
        participant: Some("human:fixture".into()),
        producers: vec!["brit".into()],
        automatic_capture: true,
        deliberate_evidence: true,
        max_bytes: 1_000_000,
        max_record_bytes: 12_000,
        capacity: None,
        response: Response::Repair,
        sharing: "separately-authorized".into(),
    }
}

fn occurrence(run: &str, outcome: Outcome) -> Feedback {
    Feedback::Occurrence(Occurrence {
        source: Source {
            producer: "brit".into(),
            run: run.into(),
            item: "log".into(),
        },
        observed_at: "2020-01-01T00:00:00Z".into(),
        participant: Some("human:fixture".into()),
        subject: "brit:log".into(),
        outcome,
        count: 1,
        duration_ms: None,
        impact: None,
        message: None,
        detail: json!(null),
    })
}

fn workspace(policy: &Policy) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("fixture");
    std::fs::create_dir(dir.path().join(".epr-meta")).expect("governance");
    std::fs::write(
        dir.path().join(".epr-meta/feedback-policy.json"),
        serde_json::to_vec(policy).expect("policy bytes"),
    )
    .expect("policy");
    std::fs::write(
        dir.path().join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:fixture\n",
    )
    .expect("repository identity");
    // Actor claims are dated against an actual repository revision, as in flow_acceptance.
    for args in [
        vec!["init", "-q"],
        vec!["add", ".epr-meta"],
        vec!["commit", "-qm", "fixture"],
    ] {
        let result = std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
            .output()
            .expect("fixture repository");
        assert!(result.status.success(), "{result:?}");
    }
    dir
}

fn work(root: &Path, scope: Cid, source: &str) -> (Vec<(Cid, FlowRecord)>, Cid, Cid, String) {
    let claim =
        actor::claim(root, "agent:implementer@gpt-6", "fixture-session").expect("registered actor");
    // A supplied historical snapshot may outlive its source. Do not fabricate admission
    // of an absent source into the live store; the retained test admits its records below.
    let mut store = MemoryFlowStore::new();
    let intent = store
        .append(FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            raised_by: AgentRef("human:fixture".into()),
            in_scope_of: scope,
            resource_spec: ResourceSpec {
                classified_as: vec![source.into(), scope.to_string()],
                quantity: None,
            },
        }))
        .expect("intent");
    let commitment = store
        .append(FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("agent:implementer@gpt-6".into()),
            receiver: AgentRef("repo:fixture".into()),
            in_scope_of: scope,
            resource_spec: ResourceSpec {
                classified_as: vec![format!("actor-claim:{}", claim.record_cid)],
                quantity: None,
            },
            valid_from: None,
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: vec![intent],
            bound: None,
        }))
        .expect("commitment");
    (
        store.records().expect("records"),
        intent,
        commitment,
        claim.record_cid,
    )
}

#[test]
fn retained_feedback_establishes_original_source_without_granting_acceptance() {
    let policy = policy();
    let dir = workspace(&policy);
    let feedback = FeedbackStore::new(dir.path(), policy.clone()).expect("feedback store");
    let scope = feedback
        .append(occurrence("retained", Outcome::Failed), false)
        .expect("capture");
    let (records, intent, promise, actor) = work(dir.path(), scope, "feedback:repair");
    let store = SidecarFlowStore::open(dir.path()).expect("flow store");
    let mut transaction = store
        .transaction()
        .expect("reference admission transaction");
    transaction
        .feedback_records(policy)
        .expect("retained evidence under lock");
    for (_, record) in &records {
        transaction
            .append(record.clone())
            .expect("lawful reference admission");
    }
    drop(transaction);
    let before =
        std::fs::read(dir.path().join(".eprfs/status/feedback.jsonl")).expect("retained bytes");
    // A misleading descriptive label cannot replace native source verification.
    let labels = BTreeMap::from([(scope.to_string(), "not-a-file".into())]);
    let result =
        reconciliation::reconcile_with(dir.path(), &scope, None, &records, &labels, json!({}));
    let row = &result.assertions[0];
    assert!(row.issues.is_empty(), "{:?}", row.issues);
    assert_eq!(result.scope_cid, scope.to_string());
    assert_eq!(row.intent.as_deref(), Some(intent.to_string().as_str()));
    assert_eq!(row.commitments[0].commitment, promise.to_string());
    assert!(row.commitments[0].attribution.eligible);
    assert_eq!(row.commitments[0].attribution.actor_claim, Some(actor));
    assert_eq!(row.acceptance, "acceptance-unestablished");
    assert!(row.commitments[0].fulfillments.is_empty());
    let context =
        flow::context::context(dir.path(), &scope.to_string()).expect("occurrence context");
    assert_eq!(context.reconciliation.assertions[0].intent, row.intent);
    assert!(context.reconciliation.assertions[0].issues.is_empty());
    assert_eq!(
        before,
        std::fs::read(dir.path().join(".eprfs/status/feedback.jsonl")).expect("unchanged bytes")
    );
}

#[test]
fn missing_and_corrupt_feedback_remain_unestablished_despite_a_file_label() {
    let policy = policy();
    let dir = workspace(&policy);
    let entry = Entry::new(policy.clone(), occurrence("missing", Outcome::Failed)).expect("entry");
    let (records, _, _, _) = work(dir.path(), entry.cid, "feedback:repair");
    let path = dir.path().join(".eprfs/status/feedback.jsonl");
    for (contents, expected) in [
        (None, "not retained"),
        (Some("not-json\n".to_owned()), "fabric: decode"),
    ] {
        if let Some(contents) = contents {
            std::fs::write(&path, contents).expect("corrupt fixture");
        }
        let result = reconciliation::reconcile(
            dir.path(),
            &entry.cid,
            Some(".epr-meta/feedback-policy.json"),
            &records,
        );
        assert_eq!(result.assertions[0].acceptance, "revalidation-required");
        assert!(
            result.assertions[0].issues.join(" ").contains(expected),
            "{:?}",
            result.assertions[0].issues
        );
    }
    let mut corrupt = entry;
    if let Feedback::Occurrence(o) = &mut corrupt.record {
        o.count += 1;
    }
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string(&corrupt).expect("entry")),
    )
    .expect("tampered atom");
    let result = reconciliation::reconcile(dir.path(), &corrupt.cid, None, &records);
    assert!(result.assertions[0].issues.join(" ").contains("integrity"));
}

struct Captured {
    entry: Option<Entry>,
}

impl EvidenceResources for Captured {
    fn read_text(&self, _path: &str) -> FlowResult<String> {
        Err(FlowError::Unavailable("no captured text".into()))
    }
    fn actor_identity(&self, _id: &Cid) -> FlowResult<String> {
        Err(FlowError::Unavailable("no captured actor".into()))
    }
    fn feedback_entry(&self, _id: &Cid) -> FlowResult<Entry> {
        self.entry
            .clone()
            .ok_or_else(|| FlowError::Unavailable("private feedback resource not captured".into()))
    }
}

#[test]
fn explicit_resources_cannot_substitute_another_occurrence_or_interpretation() {
    let policy = policy();
    let dir = workspace(&policy);
    let source = Entry::new(policy.clone(), occurrence("first", Outcome::Failed)).expect("entry");
    let (records, _, _, _) = work(dir.path(), source.cid, "feedback:repair");
    let other = Entry::new(policy.clone(), occurrence("other", Outcome::Failed)).expect("other");
    for entry in [None, Some(other)] {
        let result = reconciliation::reconcile_with(
            &Captured { entry },
            &source.cid,
            None,
            &records,
            &BTreeMap::new(),
            json!({}),
        );
        assert_eq!(result.assertions[0].acceptance, "revalidation-required");
    }
    let interpretation = Entry::new(
        policy,
        Feedback::Interpretation {
            occurrences: vec![source.cid],
            author: "human:fixture".into(),
            at: "2026-11-11".into(),
            summary: "An interpretation is not the original occurrence".into(),
            objective: None,
        },
    )
    .expect("interpretation");
    let (records, _, _, _) = work(dir.path(), interpretation.cid, "feedback:repair");
    let result = reconciliation::reconcile_with(
        &Captured {
            entry: Some(interpretation.clone()),
        },
        &interpretation.cid,
        None,
        &records,
        &BTreeMap::new(),
        json!({}),
    );
    assert!(result.assertions[0]
        .issues
        .join(" ")
        .contains("exact retained occurrence"));
}

#[test]
fn document_sources_keep_their_original_body_verification() {
    let dir = workspace(&policy());
    let scope = flow::body_cid("document assertion");
    let (records, _, _, _) = work(dir.path(), scope, "gap:open");
    std::fs::write(dir.path().join("scope.md"), "document assertion").expect("document");
    assert!(
        reconciliation::reconcile(dir.path(), &scope, Some("scope.md"), &records).assertions[0]
            .issues
            .is_empty()
    );
    std::fs::write(dir.path().join("scope.md"), "revised").expect("revision");
    assert!(
        reconciliation::reconcile(dir.path(), &scope, Some("scope.md"), &records).assertions[0]
            .issues[0]
            .contains("source missing or revised")
    );
}

#[test]
fn reclaimed_feedback_is_not_retained_with_honestly_unknown_history() {
    let mut policy = policy();
    let parameters = json!({"totalBytes":180000,"activeBytes":50000,"replacementBytes":50000,
        "controlBytes":80000,"recordBytes":12000,"triggerPercent":85,"resetPercent":70,
        "routineDays":7,"resolvedDays":30,"recentReceipts":2,"dwellSeconds":0});
    let precedent = json!({"id":"fixture-retention","version":1,"binding":"binding-local",
        "permits":["reclaim-eligible-feedback"],"parameter-bounds":{"routineDays":{"min":7},"resolvedDays":{"min":30},"controlBytes":{"min":65536},"recentReceipts":{"min":1,"max":32}},
        "established_by":"operator-fixture","why":"bounded private evidence","supersedes":null,
        "scope":{"path":".eprfs/status/feedback"},"measure":parameters,
        "permitted-local-parameters":parameters.as_object().expect("parameters").keys().collect::<Vec<_>>()});
    policy.capacity = Some(
        Capacity::resolve("fixture-retention@1".into(), precedent, json!({})).expect("capacity"),
    );
    policy.max_bytes = 0;
    policy.max_record_bytes = 0;
    let dir = workspace(&policy);
    let feedback = FeedbackStore::new(dir.path(), policy).expect("store");
    feedback.migrate(true).expect("explicit migration");
    let scope = feedback
        .append(occurrence("old-routine", Outcome::Passed), false)
        .expect("capture");
    let absent = Entry::new(
        crate::policy(),
        occurrence("never-recorded", Outcome::Passed),
    )
    .expect("absent")
    .cid;
    assert!(dir.path().feedback_entry(&scope).is_ok());
    for i in 0..100 {
        if feedback.capacity().expect("capacity").retained_bytes >= 43_000 {
            break;
        }
        feedback
            .append(occurrence(&format!("fill-{i}"), Outcome::Passed), false)
            .expect("fill");
    }
    feedback
        .reclaim(false)
        .expect("reclaim eligible routine observations");
    let reclaimed_error = dir
        .path()
        .feedback_entry(&scope)
        .expect_err("reclaimed")
        .to_string();
    let absent_error = dir
        .path()
        .feedback_entry(&absent)
        .expect_err("absent")
        .to_string();
    for error in [reclaimed_error, absent_error] {
        assert!(
            error.contains("not retained")
                && error.contains("absent or reclaimed history is unknown")
        );
    }
    // Only afterward simulate a consumer holding an unavailable historical reference.
    let (records, _, _, _) = work(dir.path(), scope, "feedback:repair");
    let result = reconciliation::reconcile(dir.path(), &scope, None, &records);
    assert_eq!(result.assertions[0].acceptance, "revalidation-required");
}
