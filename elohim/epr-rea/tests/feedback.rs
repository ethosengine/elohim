#![cfg(feature = "sidecar")]
use elohim_epr_rea::{
    atom_cid,
    feedback::{
        experiences, Entry, Feedback, Occurrence, Outcome, Policy, Relation, Response, Source,
    },
    feedback_store::FeedbackStore,
};

fn policy() -> Policy {
    Policy {
        id: "fixture@1".into(),
        authority: "human:fixture".into(),
        participant: Some("human:fixture".into()),
        producers: vec!["human".into()],
        automatic_capture: true,
        deliberate_evidence: true,
        capacity: None,
        max_bytes: 1_000_000,
        max_record_bytes: 50_000,
        response: Response::Repair,
        sharing: "separately-authorized".into(),
    }
}
fn occurrence(run: &str, outcome: Outcome) -> Feedback {
    Feedback::Occurrence(Occurrence {
        source: Source {
            producer: "human".into(),
            run: run.into(),
            item: "feedback".into(),
        },
        observed_at: "2026-11-08T15:00:00.000Z".into(),
        participant: Some("human:fixture".into()),
        subject: "history".into(),
        outcome,
        count: 7,
        duration_ms: None,
        impact: None,
        message: None,
        detail: serde_json::Value::Null,
    })
}

#[test]
fn feedback_offline_restart_idempotence_and_recurrence() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = FeedbackStore::new(dir.path(), policy())?;
    assert!(store.read()?.is_empty());
    assert!(
        !dir.path().join(".eprfs").exists(),
        "reading absent evidence creates nothing"
    );
    let cid = store.append(occurrence("first", Outcome::Reported), true)?;
    drop(store);
    let store = FeedbackStore::new(dir.path(), policy())?;
    let mut retry = occurrence("first", Outcome::Reported);
    if let Feedback::Occurrence(o) = &mut retry {
        o.observed_at = "2026-11-09T15:00:00.000Z".into();
    }
    assert_eq!(store.append(retry, true)?, cid);
    assert!(store
        .append(occurrence("first", Outcome::Failed), true)
        .is_err());
    store.append(occurrence("second", Outcome::Reported), true)?;
    let entries = store.read()?;
    let views = experiences(&entries);
    assert_eq!(views.len(), 2);
    assert_eq!(views[0].observed_runs, 2, "retries are not recurring runs");
    assert_eq!(
        views[0].occurrence.count, 7,
        "aggregate count is retained without invented observations"
    );
    assert_eq!(views[0].occurrence.duration_ms, None);
    Ok(())
}

#[test]
fn feedback_concurrent_append_and_duplicate_repair() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::thread::scope(|scope| {
        let mut handles = vec![];
        for i in 0..12 {
            let root = dir.path();
            handles.push(scope.spawn(move || {
                FeedbackStore::new(root, policy())
                    .expect("fixture store")
                    .append(occurrence(&i.to_string(), Outcome::Reported), true)
                    .expect("concurrent append")
            }));
        }
        for handle in handles {
            handle.join().expect("writer completed");
        }
    });
    let store = FeedbackStore::new(dir.path(), policy())?;
    let entries = store.read()?;
    assert_eq!(entries.len(), 12);
    let target = entries[0].cid;
    store.append(
        Feedback::CommitmentLink {
            occurrence: target,
            commitment: atom_cid(&"repair-a")?,
        },
        true,
    )?;
    assert!(store
        .append(
            Feedback::CommitmentLink {
                occurrence: target,
                commitment: atom_cid(&"repair-b")?
            },
            true
        )
        .is_err());
    Ok(())
}

#[test]
fn feedback_corruption_limits_and_missing_policy_preserve_bytes(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = FeedbackStore::new(dir.path(), policy())?;
    store.append(occurrence("first", Outcome::Reported), true)?;
    let path = dir.path().join(".eprfs/status/feedback.jsonl");
    let original = std::fs::read(&path)?;
    let mut small = policy();
    small.max_record_bytes = 1;
    small.max_bytes = 1;
    assert!(FeedbackStore::new(dir.path(), small)?
        .append(occurrence("next", Outcome::Reported), true)
        .is_err());
    assert_eq!(std::fs::read(&path)?, original);
    let corrupted = String::from_utf8(original)?.replace("history", "tampered");
    std::fs::write(&path, &corrupted)?;
    assert!(store.read().is_err());
    assert!(store
        .append(occurrence("next", Outcome::Reported), true)
        .is_err());
    assert_eq!(std::fs::read_to_string(&path)?, corrupted);
    std::fs::write(&path, "{truncated")?;
    assert!(store.read().is_err());
    assert_eq!(std::fs::read_to_string(&path)?, "{truncated");
    let mut absent = policy();
    absent.authority.clear();
    assert!(FeedbackStore::new(dir.path(), absent).is_err());
    Ok(())
}

#[test]
fn feedback_full_store_preserves_evidence_and_accepts_idempotent_retries(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let mut capacity = policy();
    capacity.max_bytes = 4096;
    capacity.max_record_bytes = 2048;
    let store = FeedbackStore::new(dir.path(), capacity.clone())?;
    let path = dir.path().join(".eprfs/status/feedback.jsonl");
    let first = occurrence("0", Outcome::Reported);
    let first_cid = store.append(first.clone(), true)?;
    let mut reached_limit = false;
    for run in 1..12 {
        let before = std::fs::read(&path)?;
        let record = occurrence(&run.to_string(), Outcome::Reported);
        let bytes = serde_json::to_vec(&Entry::new(capacity.clone(), record.clone())?)?.len() + 1;
        assert!(bytes as u64 <= capacity.max_record_bytes);
        let result = store.append(record, true);
        if before.len() as u64 + bytes as u64 <= capacity.max_bytes {
            result?;
            assert_eq!(
                std::fs::metadata(&path)?.len(),
                (before.len() + bytes) as u64
            );
        } else {
            assert!(result
                .expect_err("full store refuses")
                .to_string()
                .contains("store retention limit"));
            assert_eq!(
                std::fs::read(&path)?,
                before,
                "no eviction or partial append"
            );
            reached_limit = true;
            break;
        }
    }
    assert!(
        reached_limit,
        "fixture actually fills the total-byte allowance"
    );
    let before = std::fs::read(&path)?;
    drop(store);
    let reopened = FeedbackStore::new(dir.path(), capacity)?;
    assert!(
        !reopened.read()?.is_empty(),
        "full evidence remains readable after restart"
    );
    assert_eq!(reopened.append(first, true)?, first_cid);
    assert_eq!(std::fs::read(&path)?, before, "retries consume no capacity");
    Ok(())
}

#[test]
fn feedback_racing_writers_cannot_exceed_carry_capacity() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let mut capacity = policy();
    capacity.max_bytes = 4096;
    capacity.max_record_bytes = 2048;
    let barrier = std::sync::Barrier::new(12);
    let outcomes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..12)
            .map(|run| {
                let capacity = capacity.clone();
                let barrier = &barrier;
                let root = dir.path();
                scope.spawn(move || {
                    let store = FeedbackStore::new(root, capacity).expect("fixture store");
                    barrier.wait();
                    store.append(occurrence(&run.to_string(), Outcome::Reported), true)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("writer completed"))
            .collect::<Vec<_>>()
    });
    let accepted = outcomes.iter().filter(|r| r.is_ok()).count();
    assert!(
        accepted > 0 && accepted < 12,
        "writers reach the capacity boundary"
    );
    for error in outcomes.into_iter().filter_map(Result::err) {
        assert!(error.to_string().contains("store retention limit"));
    }
    let entries = FeedbackStore::new(dir.path(), capacity.clone())?.read()?;
    assert_eq!(
        entries.len(),
        accepted,
        "every acknowledged record is retained"
    );
    assert!(
        std::fs::metadata(dir.path().join(".eprfs/status/feedback.jsonl"))?.len()
            <= capacity.max_bytes
    );
    Ok(())
}

#[test]
fn feedback_build_green_is_not_recovery_and_recurrence_reopens(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = FeedbackStore::new(dir.path(), policy())?;
    let target = store.append(occurrence("first", Outcome::Failed), true)?;
    let link = |evidence, relation| Feedback::EvidenceLink {
        occurrence: target,
        evidence,
        relation,
        observer: "human:fixture".into(),
        at: "2026-11-08T16:00:00.000Z".into(),
        outcome: Outcome::Passed,
    };
    store.append(link(atom_cid(&"green-build")?, Relation::VerifiedFix), true)?;
    let attempt = atom_cid(&"one-observed-attempt")?;
    store.append(link(attempt, Relation::Attempt), true)?;
    let mut second_witness = link(attempt, Relation::Attempt);
    if let Feedback::EvidenceLink { observer, .. } = &mut second_witness {
        *observer = "human:second-observer".into();
    }
    store.append(second_witness, true)?;
    assert_eq!(
        experiences(&store.read()?)[0].attempts,
        1,
        "two witnesses do not create two attempts"
    );
    assert!(!experiences(&store.read()?)[0].experienced_recovery);
    assert!(store
        .append(
            link(atom_cid(&"green-build")?, Relation::ExperiencedRecovery),
            true
        )
        .is_err());
    let recovered = store.append(occurrence("second", Outcome::Passed), true)?;
    store.append(link(recovered, Relation::ExperiencedRecovery), true)?;
    assert!(experiences(&store.read()?)[0].experienced_recovery);
    let again = store.append(occurrence("third", Outcome::Failed), true)?;
    assert!(
        !experiences(&store.read()?)
            .iter()
            .find(|v| v.cid == again)
            .expect("recurrence")
            .experienced_recovery
    );
    Ok(())
}

#[test]
fn feedback_policy_and_recorder_io_failure() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let mut denied = policy();
    denied.automatic_capture = false;
    let store = FeedbackStore::new(dir.path(), denied)?;
    assert!(store
        .append(occurrence("first", Outcome::Failed), false)
        .is_err());
    assert!(!dir.path().join(".eprfs").exists());
    std::fs::write(dir.path().join(".eprfs"), "not a directory")?;
    let store = FeedbackStore::new(dir.path(), policy())?;
    assert!(store
        .append(occurrence("first", Outcome::Failed), true)
        .is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".eprfs"))?,
        "not a directory"
    );
    Ok(())
}

#[test]
fn feedback_recovery_uses_instants_and_refuses_invalid_times(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = FeedbackStore::new(dir.path(), policy())?;
    let target = store.append(occurrence("first", Outcome::Failed), true)?;
    let mut before = occurrence("second", Outcome::Passed);
    if let Feedback::Occurrence(o) = &mut before {
        o.observed_at = "2026-11-08T16:00:00+02:00".into();
    }
    let before = store.append(before, true)?;
    assert!(store
        .append(
            Feedback::EvidenceLink {
                occurrence: target,
                evidence: before,
                relation: Relation::ExperiencedRecovery,
                observer: "human:fixture".into(),
                at: "2026-11-08T17:00:00Z".into(),
                outcome: Outcome::Passed
            },
            true
        )
        .is_err());
    let mut invalid = occurrence("third", Outcome::Passed);
    if let Feedback::Occurrence(o) = &mut invalid {
        o.observed_at = "not a time".into();
    }
    assert!(store.append(invalid, true).is_err());
    Ok(())
}
