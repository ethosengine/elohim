//! Every follow attempt leaves one durable outcome in `follow.json`, and recording it never
//! changes what the follow does.
use super::*;
use crate::bin_store::follow_record::{sanitize_detail, sanitize_location, Attempt};
use ark_core::declaration::{Follow, FollowMode};

fn last(f: &Fixture) -> LastFollow {
    f.store.last_follow(&crate::session::proc_root())
}
fn recorded(f: &Fixture) -> FollowRecord {
    match last(f) {
        LastFollow::Recorded(record) => record,
        other => panic!("expected a recorded follow, got {other:?}"),
    }
}
fn follow_json(f: &Fixture) -> PathBuf {
    f.store.root.join("follow.json")
}
/// A pending record owned by a fixture process, for liveness cases.
fn pending_for(f: &Fixture, pid: u32, start: &str) -> FollowRecord {
    let mut record = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    record.owner_pid = pid;
    record.owner_start_time = Some(start.into());
    atomic_json(&follow_json(f), &record).unwrap();
    record
}
/// A fake process table holding one process in `state` started at tick 777.
fn proc_with(f: &Fixture, pid: u32, state: &str) -> PathBuf {
    let root = f.path("proc");
    fs::create_dir_all(root.join(pid.to_string())).unwrap();
    fs::write(
        root.join(pid.to_string()).join("stat"),
        format!("{pid} (epr) {state} {}777", "0 ".repeat(18)),
    )
    .unwrap();
    root
}

#[test]
fn follow_record_absent_reads_as_never_attempted() {
    let f = Fixture::new("epr");
    assert_eq!(last(&f), LastFollow::Never);
    // Bootstrap is not a follow and leaves no follow record.
    f.release("a");
    f.store.bootstrap(f.tmp.path(), None).unwrap();
    assert_eq!(last(&f), LastFollow::Never);
}

#[test]
fn follow_record_goes_from_pending_to_terminal() {
    let f = Fixture::new("epr");
    let mut attempt = Attempt::begin(&f.store, f.tmp.path(), "apply");
    let LastFollow::InProgress(live) = last(&f) else {
        panic!("a running attempt reads as in progress: {:?}", last(&f));
    };
    assert_eq!(live.outcome, FollowOutcome::Pending);
    assert!(live.ended_at.is_none());
    attempt.finish(&Err(Refusal::new("build-failed", "fixture")));
    let done = recorded(&f);
    assert_eq!(done.attempt, live.attempt);
    assert_eq!(done.outcome, FollowOutcome::Refused);
    assert_eq!(done.kind.as_deref(), Some("build-failed"));
    assert!(done.ended_at.is_some());
}

// Review item 1: a record that cannot be written never changes the follow's result.
#[test]
fn follow_record_failure_never_changes_what_follow_returns() {
    let reference = Fixture::new("epr");
    let a = reference.release("a");
    reference.land(&a).unwrap();
    let expected = reference.store.follow(reference.tmp.path(), None).unwrap();

    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    // follow.json obstructed by a non-empty directory: neither pending nor terminal writes.
    fs::create_dir_all(follow_json(&f).join("obstruction")).unwrap();
    assert_eq!(f.store.follow(f.tmp.path(), None).unwrap(), expected);

    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("b");
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap()["followed"],
        true
    );
    assert_eq!(f.store.head("current"), Some(b.cid));
    assert!(follow_json(&f).join("obstruction").is_dir());
}

// Review item 2: an unwind still records a terminal outcome.
#[test]
fn follow_record_unwind_records_a_terminal_outcome() {
    let f = Fixture::new("epr");
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        f.store
            .recorded_follow(f.tmp.path(), "apply", |_| panic!("fixture worker panic"))
    }));
    assert!(unwound.is_err());
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Refused);
    assert_eq!(record.kind.as_deref(), Some("follow-unwound"));
}

// Review item 2: a completion that cannot be written leaves no "in progress" behind.
#[test]
fn follow_record_unwritable_completion_reads_as_interrupted() {
    let f = Fixture::new("epr");
    let mut record = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    record.outcome = FollowOutcome::Refused;
    record.ended_at = Some("2026-11-15T00:00:00Z".into());
    f.store.complete_follow(&record, &|_, _| {
        Err(Refusal::new("io", "No space left on device"))
    });
    assert!(!follow_json(&f).exists());
    let LastFollow::Interrupted {
        record: left,
        detail,
    } = last(&f)
    else {
        panic!("expected interrupted, got {:?}", last(&f));
    };
    assert_eq!(left.attempt, record.attempt);
    assert!(detail.contains("could not be recorded"));
    // A later attempt supersedes the marker.
    Attempt::begin(&f.store, f.tmp.path(), "apply")
        .finish(&Err(Refusal::new("build-failed", "fixture")));
    assert_eq!(recorded(&f).kind.as_deref(), Some("build-failed"));
    assert!(!f.store.root.join("follow.unfinished.json").exists());
}

#[test]
fn follow_record_names_a_landed_release() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("b");
    f.store.follow(f.tmp.path(), None).unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Landed);
    assert_eq!(record.mode, "apply");
    assert_eq!(record.commit, Some("b".repeat(40)));
    assert_eq!(record.prior_cid, Some(a.cid));
    assert_eq!(record.candidate_cid, Some(b.cid.clone()));
    assert_eq!(record.result_cid, Some(b.cid));
    assert_eq!(record.source, f.tmp.path().display().to_string());
    assert!(record.kind.is_none());
}

// Review item 3: the authoritative head decides, not Ok/Err alone.
#[test]
fn follow_record_a_committed_head_is_landed_even_when_link_repair_fails() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("b");
    let link = f.store.root.join("current");
    fs::remove_file(&link).unwrap();
    fs::create_dir_all(link.join("obstruction")).unwrap();
    let error = f.store.follow(f.tmp.path(), None).unwrap_err();
    assert_eq!(f.store.head("current"), Some(b.cid.clone()));
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Landed);
    assert_eq!(record.kind.as_deref(), Some(error.kind.as_str()));
    assert!(record.reason.unwrap().starts_with("head committed, then:"));
    assert_eq!(record.result_cid, Some(b.cid));
}

// Review item 3: the same release landed from a new source commit is a landing.
#[test]
fn follow_record_same_release_from_a_new_commit_is_landed() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let again = f.release("a");
    assert_eq!(again.cid, a.cid);
    f.store.follow(f.tmp.path(), None).unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Landed);
    assert_eq!(record.prior_cid, record.result_cid);
    assert_eq!(record.commit, Some("b".repeat(40)));
}

#[test]
fn follow_record_keeps_an_early_refusal() {
    // brit refuses with an error; epr declines with a reason. Both leave the outcome.
    let f = brit_follow();
    f.state.lock().unwrap().descended = false;
    let prior = f.store.head("current");
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap_err().kind,
        "non-descendant"
    );
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Refused);
    assert_eq!(record.kind.as_deref(), Some("non-descendant"));
    assert_eq!(record.commit, Some("b".repeat(40)));
    assert_eq!(record.prior_cid, prior);
    assert_eq!(record.result_cid, prior);
    assert!(record.candidate_cid.is_none());

    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    {
        let mut s = f.state.lock().unwrap();
        s.head = "b".repeat(40);
        s.descended = false;
    }
    let value = f.store.follow(f.tmp.path(), None).unwrap();
    assert_eq!(value["followed"], false);
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Refused);
    assert_eq!(record.kind.as_deref(), Some("non-descendant"));
    assert_eq!(record.result_cid, Some(a.cid));
}

#[test]
fn follow_record_keeps_a_refused_candidate() {
    let f = brit_follow();
    let r = f.release("new");
    f.state.lock().unwrap().verdict =
        Some(json!({"outcome":"pass","tier":"witnessed","conclusion":"failure"}));
    f.store.follow(f.tmp.path(), None).unwrap_err();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Refused);
    assert_eq!(record.kind.as_deref(), Some("attestation-refused"));
    assert_eq!(record.candidate_cid, Some(r.cid));
}

#[test]
fn follow_record_reports_unchanged() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.store.follow(f.tmp.path(), None).unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Unchanged);
    assert_eq!(record.kind.as_deref(), Some("head-is-checkout"));
    {
        let mut s = f.state.lock().unwrap();
        s.head = "b".repeat(40);
        s.unchanged = true;
    }
    f.store.follow(f.tmp.path(), None).unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Unchanged);
    assert_eq!(record.kind.as_deref(), Some("no-source-change"));
    assert_eq!(record.result_cid, Some(a.cid));
}

#[test]
fn follow_record_reports_observed_and_a_pinned_current_as_unchanged() {
    let f = Fixture::new("epr");
    let old = f.release("old");
    f.land(&old).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let new = f.release("new");
    let mut follow = Follow {
        channel: "bin:epr".into(),
        mode: FollowMode::Observe,
        pin: None,
    };
    f.store
        .follow_declared(f.tmp.path(), None, &follow)
        .unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Observed);
    assert_eq!(record.mode, "observe");
    assert_eq!(record.candidate_cid, Some(new.cid.clone()));
    assert_eq!(record.result_cid, Some(old.cid));

    follow.mode = FollowMode::Canary;
    follow.pin = Some(new.cid.clone());
    f.store
        .follow_declared(f.tmp.path(), None, &follow)
        .unwrap();
    assert_eq!(recorded(&f).outcome, FollowOutcome::Landed);
    f.store
        .follow_declared(f.tmp.path(), None, &follow)
        .unwrap();
    let record = recorded(&f);
    assert_eq!(record.outcome, FollowOutcome::Unchanged);
    assert_eq!(record.kind.as_deref(), Some("already-current"));
    assert_eq!(record.result_cid, Some(new.cid));
}

#[test]
fn follow_record_pending_without_its_owner_reads_as_interrupted() {
    let f = Fixture::new("epr");
    let mut pending = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    // The same PID started at another time is a different process.
    pending.owner_start_time = Some("1".into());
    atomic_json(&follow_json(&f), &pending).unwrap();
    assert!(
        matches!(last(&f), LastFollow::Interrupted { record, .. } if record.attempt == pending.attempt)
    );
    pending.owner_pid = u32::MAX;
    atomic_json(&follow_json(&f), &pending).unwrap();
    assert!(matches!(last(&f), LastFollow::Interrupted { .. }));
}

// Review item 6: a zombie is not running, and an unreadable process table is not "exited".
#[test]
fn follow_record_liveness_reads_zombies_and_unknowns_honestly() {
    let f = Fixture::new("epr");
    pending_for(&f, 4242, "777");
    let running = proc_with(&f, 4242, "S");
    assert!(matches!(
        f.store.last_follow(&running),
        LastFollow::InProgress(_)
    ));
    let zombie = proc_with(&f, 4242, "Z");
    let LastFollow::Interrupted { detail, .. } = f.store.last_follow(&zombie) else {
        panic!("a zombie owner is not in progress");
    };
    assert!(detail.contains("zombie"));

    // Unreadable metadata (here: stat is a directory) leaves the attempt pending, unknown.
    let unreadable = f.path("proc-unreadable");
    fs::create_dir_all(unreadable.join("4242/stat")).unwrap();
    assert!(matches!(
        f.store.last_follow(&unreadable),
        LastFollow::Unknown { .. }
    ));
    assert!(matches!(
        f.store.last_follow(&f.path("no-proc")),
        LastFollow::Unknown { .. }
    ));
}

#[test]
fn follow_record_unreadable_is_corrupt_not_never() {
    let f = Fixture::new("epr");
    fs::create_dir_all(&f.store.root).unwrap();
    fs::write(follow_json(&f), b"{not json").unwrap();
    assert!(matches!(last(&f), LastFollow::Corrupt(_)));
    fs::write(follow_json(&f), br#"{"outcome":"landed"}"#).unwrap();
    assert!(matches!(last(&f), LastFollow::Corrupt(_)));
}

// Review item 5: a syntactically valid but wrong record reads as corrupt.
#[test]
fn follow_record_validates_version_mode_cids_and_end_time() {
    let f = Fixture::new("epr");
    let good = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    type Damage = Box<dyn Fn(&mut FollowRecord)>;
    let cases: [(&str, Damage); 5] = [
        ("version", Box::new(|r: &mut FollowRecord| r.version = 0)),
        (
            "mode",
            Box::new(|r: &mut FollowRecord| r.mode = "sideways".into()),
        ),
        (
            "cid",
            Box::new(|r: &mut FollowRecord| r.prior_cid = Some("not-a-cid".into())),
        ),
        (
            "start",
            Box::new(|r: &mut FollowRecord| r.owner_start_time = Some("12a".into())),
        ),
        (
            "end",
            Box::new(|r: &mut FollowRecord| r.ended_at = Some("2026-11-15T00:00:00Z".into())),
        ),
    ];
    for (name, damage) in cases {
        let mut record = good.clone();
        damage(&mut record);
        atomic_json(&follow_json(&f), &record).unwrap();
        assert!(matches!(last(&f), LastFollow::Corrupt(_)), "{name}");
    }
}

// Review item 7: completion fails closed on a record it cannot read.
#[test]
fn follow_record_completion_never_replaces_an_unreadable_record() {
    let f = Fixture::new("epr");
    let mut record = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    fs::write(follow_json(&f), b"{unreadable newer").unwrap();
    record.outcome = FollowOutcome::Refused;
    record.ended_at = Some("2026-11-15T00:00:00Z".into());
    f.store
        .complete_follow(&record, &|path, record| atomic_json(path, record));
    assert_eq!(fs::read(follow_json(&f)).unwrap(), b"{unreadable newer");
}

#[test]
fn follow_record_completion_never_replaces_a_newer_attempt() {
    let f = Fixture::new("epr");
    let mut first = Attempt::begin(&f.store, f.tmp.path(), "apply");
    let second = f.store.begin_follow(f.tmp.path(), "apply").unwrap();
    first.finish(&Ok(json!({})));
    let LastFollow::InProgress(record) = last(&f) else {
        panic!("the newer pending attempt stands");
    };
    assert_eq!(record.attempt, second.attempt);
}

// Review item 8: no URL credentials, query strings or secret assignments are recorded.
#[test]
fn follow_record_sanitises_locations_and_details() {
    assert_eq!(
        sanitize_location("https://user:t0ken@example.invalid/repo.git?access=t0ken#x"),
        "https://example.invalid/repo.git"
    );
    assert_eq!(sanitize_location("/projects/elohim"), "/projects/elohim");
    let detail = sanitize_detail(
        "fetch https://u:p@host/x?sig=1 failed\nNPM_TOKEN=abc CARGO_REGISTRIES_ELOHIM_TOKEN=def ok",
    );
    assert_eq!(
        detail,
        "fetch https://host/x failed NPM_TOKEN=<redacted> CARGO_REGISTRIES_ELOHIM_TOKEN=<redacted> ok"
    );
    let long = sanitize_detail(&"é".repeat(500));
    assert_eq!(long.chars().count(), 240);
    assert!(long.ends_with('…'));

    let f = Fixture::new("epr");
    Attempt::begin(&f.store, f.tmp.path(), "apply").finish(&Err(Refusal::new(
        "build-failed",
        "gh release download --repo https://x:secret@github.invalid/r failed",
    )));
    let reason = recorded(&f).reason.unwrap();
    assert!(!reason.contains("secret"), "{reason}");
}
