#![cfg(feature = "sidecar")]
use chrono::{Duration, Utc};
use elohim_epr_rea::{
    atom_cid,
    feedback::{Entry, Feedback, Occurrence, Outcome, Policy, Relation, Response, Source},
    feedback_capacity::Capacity,
    feedback_store::FeedbackStore,
    AgentRef, FlowRecord, FlowStore, Intent, ReaVerb, ResourceSpec, SidecarFlowStore,
};
use serde_json::json;

fn policy() -> Policy {
    let parameters = json!({"totalBytes":180000,"activeBytes":50000,"replacementBytes":50000,
        "controlBytes":80000,"recordBytes":12000,"triggerPercent":85,"resetPercent":70,
        "routineDays":7,"resolvedDays":30,"recentReceipts":2,"dwellSeconds":0});
    let row = json!({"id":"fixture-retention","version":1,"binding":"binding-local", "permits":["reclaim-eligible-feedback"], "parameter-bounds":{"routineDays":{"min":7},"resolvedDays":{"min":30},"controlBytes":{"min":65536},"recentReceipts":{"min":1,"max":32}},
        "established_by":"operator-fixture","why":"bounded private evidence", "supersedes":null,
        "scope":{"path":".eprfs/status/feedback"},"measure":parameters,
        "permitted-local-parameters":parameters.as_object().unwrap().keys().collect::<Vec<_>>()});
    Policy {
        id: "fixture@2".into(),
        authority: "operator-fixture".into(),
        participant: Some("human:f".into()),
        producers: vec!["human".into(), "brit".into()],
        automatic_capture: true,
        deliberate_evidence: true,
        max_bytes: 0,
        max_record_bytes: 0,
        capacity: Some(Capacity::resolve("fixture-retention@1".into(), row, json!({})).unwrap()),
        response: Response::Report,
        sharing: "separately-authorized".into(),
    }
}
fn event(run: &str, outcome: Outcome, days: i64) -> Feedback {
    Feedback::Occurrence(Occurrence {
        source: Source {
            producer: "brit".into(),
            run: run.into(),
            item: "status".into(),
        },
        observed_at: (Utc::now() - Duration::days(days)).to_rfc3339(),
        participant: Some("human:f".into()),
        subject: "brit:status".into(),
        outcome,
        count: 1,
        duration_ms: None,
        impact: None,
        message: None,
        detail: json!(null),
    })
}
fn setup() -> (tempfile::TempDir, FeedbackStore) {
    let root = tempfile::tempdir().unwrap();
    let store = FeedbackStore::new(root.path(), policy()).unwrap();
    store.migrate(true).unwrap();
    (root, store)
}
fn fill(store: &FeedbackStore, days: i64) {
    for i in 0..100 {
        if store.capacity().unwrap().retained_bytes >= 43000 {
            break;
        }
        store
            .append(event(&format!("success-{i}"), Outcome::Passed, days), false)
            .unwrap();
    }
}

#[test]
fn policy_pins_variance_authority_and_historical_addresses() {
    let p = policy();
    let c = p.capacity.as_ref().unwrap();
    let mut corrupt_parameters = c.precedent.clone();
    corrupt_parameters["measure"] = json!(7);
    assert!(Capacity::resolve(
        c.policy.clone(),
        corrupt_parameters,
        json!({"routineDays":8})
    )
    .is_err());
    assert!(
        Capacity::resolve("fixture-retention@2".into(), c.precedent.clone(), json!({})).is_err()
    );
    assert!(Capacity::resolve(
        c.policy.clone(),
        c.precedent.clone(),
        json!({"routineDays":1})
    )
    .is_err());
    assert!(Capacity::resolve(
        c.policy.clone(),
        c.precedent.clone(),
        json!({"undeclared":1})
    )
    .is_err());
    let changed = Capacity::resolve(
        c.policy.clone(),
        c.precedent.clone(),
        json!({"routineDays":8}),
    )
    .unwrap();
    assert_ne!(changed.policy_cid, c.policy_cid);
    let mut observation_only = c.precedent.clone();
    observation_only["permits"] = json!([]);
    assert!(Capacity::resolve(c.policy.clone(), observation_only, json!({})).is_err());
    // The next policy version can deliberately change defaults and permitted floors.
    let mut next_policy = c.precedent.clone();
    next_policy["version"] = json!(2);
    next_policy["measure"]["routineDays"] = json!(5);
    next_policy["parameter-bounds"]["routineDays"]["min"] = json!(5);
    let revised = Capacity::resolve("fixture-retention@2".into(), next_policy, json!({})).unwrap();
    assert_eq!(revised.parameters.routine_days, 5);
    let mut missing = c.precedent.clone();
    missing["established_by"] = json!(null);
    assert!(Capacity::resolve(c.policy.clone(), missing, json!({})).is_err());
    let mut old = p;
    old.capacity = None;
    old.max_bytes = 100000;
    old.max_record_bytes = 12000;
    let record = event("old", Outcome::Failed, 100);
    let entry = Entry::new(old.clone(), record.clone()).unwrap();
    assert_eq!(entry.cid, atom_cid(&(&old, &record)).unwrap());
    let encoded = serde_json::to_vec(&entry).unwrap();
    let decoded: Entry = serde_json::from_slice(&encoded).unwrap();
    decoded.verify().unwrap();
    assert!(!String::from_utf8(encoded)
        .unwrap()
        .contains("\"automatic\":"));
}

#[test]
fn oldest_successes_reclaim_but_protected_bytes_and_addresses_survive() {
    let (root, store) = setup();
    let protected = store
        .append_protected(event("protected", Outcome::Passed, 100), true, true)
        .unwrap();
    fill(&store, 8);
    let before = store.capacity().unwrap();
    assert!(before.observation.is_some());
    let bytes = std::fs::read(root.path().join(".eprfs/status/feedback/state.json")).unwrap();
    let dry = store.reclaim(true).unwrap();
    assert!(dry.selected_bytes > 0);
    assert_eq!(
        bytes,
        std::fs::read(root.path().join(".eprfs/status/feedback/state.json")).unwrap()
    );
    let after = store.reclaim(false).unwrap();
    println!(
        "capacity demonstration: {}",
        serde_json::to_string(
            &json!({"before":before,"after":after,"protectedCid":protected.to_string()})
        )
        .unwrap()
    );
    assert!(after.retained_bytes <= 35000);
    assert!(after.total_managed_file_bytes <= 180000);
    assert!(store.read().unwrap().iter().any(|e| e.cid == protected));
    assert!(after.history.contains("permanently unknown"));
    let signal: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.path().join(".eprfs/status/feedback/signal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(signal["open"], json!([]));
}

#[test]
fn age_future_human_and_unmeasured_are_not_routine() {
    let (_root, store) = setup();
    for (run, outcome, days, deliberate) in [
        ("young", Outcome::Passed, 6, false),
        ("future", Outcome::Passed, -8, false),
        ("unknown", Outcome::Unmeasured, 100, false),
        ("human", Outcome::Passed, 100, true),
    ] {
        store.append(event(run, outcome, days), deliberate).unwrap();
    }
    assert_eq!(store.capacity().unwrap().reclaimable_bytes, 0);
}

#[test]
fn automatic_success_with_dependencies_needs_experienced_recovery() {
    let (_root, store) = setup();
    let original = store
        .append(event("original", Outcome::Passed, 100), false)
        .unwrap();
    let mut dependent = event("dependent", Outcome::Passed, 99);
    if let Feedback::Occurrence(o) = &mut dependent {
        o.detail = json!({"evidence": original.to_string()});
    }
    let dependent = store.append(dependent, false).unwrap();
    let report = store.capacity().unwrap();
    assert_eq!(report.reclaimable_bytes, 0);
    assert_eq!(report.protected_bytes, report.retained_bytes);
    store.reclaim(false).unwrap();
    assert!(store.read().unwrap().iter().any(|e| e.cid == dependent));
    assert!(store.read().unwrap().iter().any(|e| e.cid == original));
}

#[test]
fn recovery_group_requires_experience_and_thirty_days() {
    for days in [29, 31] {
        let (_root, store) = setup();
        let failed = store
            .append(event("failed", Outcome::Failed, 50), true)
            .unwrap();
        let recovered = store
            .append(event("recovered", Outcome::Passed, days), false)
            .unwrap();
        let link = |relation| Feedback::EvidenceLink {
            occurrence: failed,
            evidence: recovered,
            relation,
            observer: "human:f".into(),
            at: (Utc::now() - Duration::days(days)).to_rfc3339(),
            outcome: Outcome::Passed,
        };
        store.append(link(Relation::VerifiedFix), true).unwrap();
        assert_eq!(store.capacity().unwrap().reclaimable_bytes, 0);
        store
            .append(link(Relation::ExperiencedRecovery), true)
            .unwrap();
        assert_eq!(store.capacity().unwrap().reclaimable_bytes > 0, days == 31);
    }
}

#[test]
fn native_flow_reference_protects_evidence_and_corruption_is_unknown() {
    let (root, store) = setup();
    let id = store
        .append(event("referenced", Outcome::Passed, 8), false)
        .unwrap();
    let flows = SidecarFlowStore::open(root.path()).unwrap();
    let mut tx = flows.transaction().unwrap();
    assert_eq!(tx.feedback_records(policy()).unwrap()[0].cid, id);
    tx.append(FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        raised_by: AgentRef("human:f".into()),
        in_scope_of: id,
        resource_spec: ResourceSpec {
            classified_as: vec!["feedback:repair".into()],
            quantity: None,
        },
    }))
    .unwrap();
    drop(tx);
    assert_eq!(store.capacity().unwrap().reclaimable_bytes, 0);
    std::fs::write(root.path().join(".eprfs/status/flows.jsonl"), "broken\n").unwrap();
    assert!(store.capacity().is_err());
    assert!(store.reclaim(false).is_err());
    assert_eq!(store.read().unwrap()[0].cid, id);
    std::fs::remove_file(root.path().join(".eprfs/status/flows.jsonl")).unwrap();
    assert!(SidecarFlowStore::open(root.path()).is_err());
    assert!(store.capacity().is_err());
    assert!(store.reclaim(false).is_err());
    assert_eq!(store.read().unwrap()[0].cid, id);
}

#[test]
fn migration_removes_legacy_copy_refuses_old_writers_and_can_close_old_loops() {
    let root = tempfile::tempdir().unwrap();
    let mut old = policy();
    old.capacity = None;
    old.max_bytes = 50000;
    old.max_record_bytes = 12000;
    let legacy = FeedbackStore::new(root.path(), old.clone()).unwrap();
    let id = legacy
        .append(event("legacy-success", Outcome::Passed, 10), false)
        .unwrap();
    let original = legacy.read().unwrap()[0].clone();
    let store = FeedbackStore::new(root.path(), policy()).unwrap();
    assert_eq!(
        store.read().unwrap()[0],
        original,
        "historical evidence is readable before explicit cutover"
    );
    assert!(store.migrate(false).is_err());
    assert!(
        store.migrate(true).is_err(),
        "missing native references block cutover"
    );
    assert_eq!(legacy.read().unwrap()[0], original);
    // The fixture's reference inventory is known empty; restore that information explicitly.
    std::fs::write(root.path().join(".eprfs/status/flows.jsonl"), "").unwrap();
    store.migrate(true).unwrap();
    assert_eq!(store.read().unwrap()[0], original);
    assert!(legacy
        .append(event("old-writer", Outcome::Failed, 0), true)
        .is_err());
    assert!(
        std::fs::metadata(root.path().join(".eprfs/status/feedback.jsonl"))
            .unwrap()
            .len()
            < 100
    );
    store
        .append(
            Feedback::EvidenceLink {
                occurrence: id,
                evidence: id,
                relation: Relation::RoutineSuccess,
                observer: "human:f".into(),
                at: Utc::now().to_rfc3339(),
                outcome: Outcome::Passed,
            },
            true,
        )
        .unwrap();
    assert!(store.capacity().unwrap().reclaimable_bytes > 0);
}

#[test]
fn protected_store_rejects_without_spill_and_retries_remain_idempotent() {
    let (_root, store) = setup();
    let first = event("first", Outcome::Failed, 100);
    let id = store.append(first.clone(), false).unwrap();
    let mut refused = false;
    for i in 0..100 {
        if store
            .append(event(&format!("fail-{i}"), Outcome::Failed, 100), false)
            .is_err()
        {
            refused = true;
            break;
        }
    }
    assert!(refused);
    let count = store.read().unwrap().len();
    assert_eq!(store.append(first, false).unwrap(), id);
    assert_eq!(store.read().unwrap().len(), count);
    let report = store.capacity().unwrap();
    assert_eq!(report.rejected_capture, 1);
    assert_eq!(report.reclaimable_bytes, 0);
    assert!(report.total_managed_file_bytes <= 180000);
}

#[test]
fn concurrent_reference_admission_append_and_reclaim_are_serialized() {
    let (root, store) = setup();
    fill(&store, 8);
    let candidate = store.read().unwrap()[0].cid;
    let flows = SidecarFlowStore::open(root.path()).unwrap();
    let mut tx = flows.transaction().unwrap();
    assert!(tx
        .feedback_records(policy())
        .unwrap()
        .iter()
        .any(|e| e.cid == candidate));
    std::thread::scope(|scope| {
        let cleanup = scope.spawn(|| {
            FeedbackStore::new(root.path(), policy())
                .unwrap()
                .reclaim(false)
        });
        tx.append(FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            raised_by: AgentRef("human:f".into()),
            in_scope_of: candidate,
            resource_spec: ResourceSpec {
                classified_as: vec!["feedback:repair".into()],
                quantity: None,
            },
        }))
        .unwrap();
        drop(tx);
        let path = root.path();
        let writers: Vec<_> = (0..6)
            .map(|i| scope.spawn(move || store_ref_append(path, i)))
            .collect();
        cleanup.join().unwrap().unwrap();
        for writer in writers {
            writer.join().unwrap();
        }
    });
    assert!(store.read().unwrap().iter().any(|e| e.cid == candidate));
    assert!(store.capacity().unwrap().total_managed_file_bytes <= 180000);
}
fn store_ref_append(root: &std::path::Path, i: usize) {
    let store = FeedbackStore::new(root, policy()).unwrap();
    let _result = store.append(event(&format!("concurrent-{i}"), Outcome::Failed, 0), false);
}

#[test]
fn exact_fit_is_admitted_at_capacity_and_one_byte_more_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut record = event("exact", Outcome::Failed, 0);
    let mut p = policy();
    let c = p.capacity.as_ref().unwrap();
    p.capacity = Some(
        Capacity::resolve(
            c.policy.clone(),
            c.precedent.clone(),
            json!({
                "activeBytes":4096,"replacementBytes":4096,"recordBytes":4096,"totalBytes":88192
            }),
        )
        .unwrap(),
    );
    let mut padding = 0;
    let mut exact = false;
    for attempt in 0..1000 {
        if let Feedback::Occurrence(o) = &mut record {
            o.source.run = format!("exact-{attempt:04}");
            o.observed_at = "2026-01-01T00:00:00Z".into();
            o.detail = json!("x".repeat(padding));
        }
        let n =
            serde_json::to_vec(&Entry::captured(p.clone(), record.clone(), true, false).unwrap())
                .unwrap()
                .len()
                + 1;
        if n == 4096 {
            exact = true;
            break;
        }
        padding = (padding as isize + 4096 - n as isize) as usize;
    }
    assert!(
        exact,
        "serialized fixture fills exactly 4096 bytes, including CID/policy/newline"
    );
    let store = FeedbackStore::new(root.path(), p).unwrap();
    store.migrate(true).unwrap();
    let id = store.append(record.clone(), false).unwrap();
    assert!(store.capacity().unwrap().at_capacity);
    assert!(store.capacity().unwrap().observation.is_some());
    assert_eq!(store.append(record, false).unwrap(), id);
    assert!(store
        .append(event("extra", Outcome::Passed, 0), false)
        .is_err());
    assert_eq!(store.read().unwrap().len(), 1);
}

#[test]
fn cleanup_receipts_expire_but_retained_retry_identity_does_not() {
    let (_root, store) = setup();
    let protected = event("protected", Outcome::Failed, 80);
    let id = store.append(protected.clone(), true).unwrap();
    for round in 0..6 {
        for i in 0..100 {
            if store.capacity().unwrap().retained_bytes >= 43000 {
                break;
            }
            store
                .append(
                    event(&format!("round-{round}-{i}"), Outcome::Passed, 8),
                    false,
                )
                .unwrap();
        }
        store.reclaim(false).unwrap();
    }
    assert_eq!(store.capacity().unwrap().recent_receipts.len(), 2);
    assert_eq!(store.append(protected, true).unwrap(), id);
}

#[test]
fn automatic_maintenance_reserves_incoming_record_and_protects_new_references() {
    let (_root, store) = setup();
    fill(&store, 8);
    let target = store.read().unwrap()[0].cid;
    store
        .append(
            Feedback::Interpretation {
                occurrences: vec![target],
                author: "human:f".into(),
                at: Utc::now().to_rfc3339(),
                summary: "Retain this experience as evidence".into(),
                objective: None,
            },
            true,
        )
        .unwrap();
    assert!(store.read().unwrap().iter().any(|e| e.cid == target));
    let report = store.capacity().unwrap();
    assert!(
        report.retained_bytes <= 35000,
        "incoming append is included in the reset target"
    );
    assert_eq!(report.recent_receipts.len(), 1);
}

#[test]
fn migration_refuses_an_oversized_legacy_footprint_without_deleting_or_enlarging() {
    let root = tempfile::tempdir().unwrap();
    let mut old = policy();
    old.capacity = None;
    old.max_bytes = 100000;
    old.max_record_bytes = 12000;
    let legacy = FeedbackStore::new(root.path(), old).unwrap();
    for i in 0..100 {
        legacy
            .append(event(&format!("legacy-{i}"), Outcome::Failed, 80), true)
            .unwrap();
        if std::fs::metadata(root.path().join(".eprfs/status/feedback.jsonl"))
            .unwrap()
            .len()
            > 50000
        {
            break;
        }
    }
    let path = root.path().join(".eprfs/status/feedback.jsonl");
    let before = std::fs::read(&path).unwrap();
    assert!(before.len() > 50000);
    let store = FeedbackStore::new(root.path(), policy()).unwrap();
    assert!(store.migrate(true).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(!root
        .path()
        .join(".eprfs/status/feedback/state.json")
        .exists());
}

#[test]
fn corrupted_generation_and_missing_lock_never_authorize_cleanup() {
    let (root, store) = setup();
    fill(&store, 8);
    let state: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.path().join(".eprfs/status/feedback/state.json")).unwrap(),
    )
    .unwrap();
    let path = root.path().join(format!(
        ".eprfs/status/feedback/generation-{}.jsonl",
        state["active"]
    ));
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] = b'!';
    std::fs::write(&path, &bytes).unwrap();
    assert!(store.capacity().is_err());
    assert!(store.reclaim(false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(root.path().join(".eprfs/status/feedback.lock")).unwrap();
    assert!(store.read().is_err());
    assert!(store.reclaim(false).is_err());
    assert!(!root.path().join(".eprfs/status/feedback.lock").exists());
}

#[test]
fn equally_old_groups_are_reclaimed_in_cid_order() {
    let (_root, store) = setup();
    for i in 0..100 {
        if store.capacity().unwrap().retained_bytes >= 43000 {
            break;
        }
        let mut e = event(&format!("tie-{i}"), Outcome::Passed, 8);
        if let Feedback::Occurrence(o) = &mut e {
            o.observed_at = "2020-01-01T00:00:00Z".into();
        }
        store.append(e, false).unwrap();
    }
    let mut before: Vec<_> = store
        .read()
        .unwrap()
        .into_iter()
        .map(|e| e.cid.to_string())
        .collect();
    before.sort();
    store.reclaim(false).unwrap();
    let after: std::collections::BTreeSet<_> = store
        .read()
        .unwrap()
        .into_iter()
        .map(|e| e.cid.to_string())
        .collect();
    let removed: Vec<_> = before
        .iter()
        .filter(|cid| !after.contains(*cid))
        .cloned()
        .collect();
    assert!(!removed.is_empty());
    assert_eq!(removed, before[..removed.len()]);
}
