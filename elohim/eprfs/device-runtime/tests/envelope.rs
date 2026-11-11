use elohim_device_runtime::{
    envelope::{self, Envelope, Store},
    session::{Registry, SessionIdentity, Source},
};
use serde_json::json;
use std::path::Path;

fn binding(home: &Path, id: &str) -> Envelope {
    Envelope {
        generation: 1,
        predecessor: None,
        identity: SessionIdentity {
            runtime: "codex".into(),
            session: Some(id.into()),
            owner_pid: Some(7),
            owner_start_time: Some(42),
            source: Source::RuntimeAncestor,
        },
        owner_pid_namespace: envelope::owner_context(home).0,
        owner_boot_id: envelope::owner_context(home).1,
        device: "did:key:fixture".into(),
        steward: "human:matthew".into(),
        model: None,
        repository: home.join("repo"),
        berth_store: None,
        opened_at: envelope::now(),
        closed_at: None,
        covenant_seal: None,
        witness: None,
        owner_holder: None,
        release_receipt: None,
        retirement: None,
    }
}
#[test]
fn binding_is_idempotent_private_and_cannot_be_reused_or_traverse() {
    if !in_succession_fixture("binding_is_idempotent_private_and_cannot_be_reused_or_traverse") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("self/ns")).unwrap();
    std::fs::write(dir.path().join("self/ns/pid"), "namespace").unwrap();
    std::fs::create_dir_all(dir.path().join("sys/kernel/random")).unwrap();
    std::fs::write(dir.path().join("sys/kernel/random/boot_id"), "boot").unwrap();
    std::fs::create_dir_all(dir.path().join("7")).unwrap();
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[19] = "42";
    std::fs::write(
        dir.path().join("7/stat"),
        format!("7 (codex) {}", fields.join(" ")),
    )
    .unwrap();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let record = store
        .open(binding(dir.path(), "codex-7-42"), dir.path())
        .unwrap();
    assert_eq!(
        store
            .open(binding(dir.path(), "codex-7-42"), dir.path())
            .unwrap()
            .opened_at,
        record.opened_at
    );
    let mut other = record.clone();
    other.identity.owner_start_time = Some(43);
    assert!(store.open(other, dir.path()).is_err());
    for id in ["../escape", ".", "..", "", "a/b"] {
        assert!(store.open(binding(dir.path(), id), dir.path()).is_err());
    }
    let mut closed = record;
    closed.closed_at = Some(envelope::now());
    store.save(&closed).unwrap();
    assert!(store
        .open(binding(dir.path(), "codex-7-42"), dir.path())
        .is_err());
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(store.path("codex-7-42").unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!dir.path().join("key").exists());
}
#[test]
fn liveness_uses_both_pid_and_start_and_unknown_is_not_dead() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("self/ns")).unwrap();
    std::fs::write(dir.path().join("self/ns/pid"), "namespace").unwrap();
    std::fs::create_dir_all(dir.path().join("sys/kernel/random")).unwrap();
    std::fs::write(dir.path().join("sys/kernel/random/boot_id"), "boot-one").unwrap();
    let mut record = binding(dir.path(), "codex-7-42");
    assert!(record.owner_dead(dir.path()));
    std::fs::create_dir(dir.path().join("7")).unwrap();
    // A present PID with missing, malformed, or unreadable stat is unknown.
    assert!(!record.owner_dead(dir.path()));
    std::fs::write(dir.path().join("7/stat"), "malformed").unwrap();
    assert!(!record.owner_dead(dir.path()));
    std::fs::remove_file(dir.path().join("7/stat")).unwrap();
    std::fs::create_dir(dir.path().join("7/stat")).unwrap();
    assert!(!record.owner_dead(dir.path()));
    std::fs::remove_dir(dir.path().join("7/stat")).unwrap();
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[19] = "42";
    std::fs::write(
        dir.path().join("7/stat"),
        format!("7 (codex) {}", fields.join(" ")),
    )
    .unwrap();
    assert!(!record.owner_dead(dir.path()));
    fields[19] = "43";
    std::fs::write(
        dir.path().join("7/stat"),
        format!("7 (codex) {}", fields.join(" ")),
    )
    .unwrap();
    assert!(record.owner_dead(dir.path()));
    // Same pid evidence in a different namespace cannot prove this owner dead.
    record.owner_pid_namespace = Some("other-namespace".into());
    assert!(!record.owner_dead(dir.path()));
    // A different boot proves all original owners dead even across namespaces.
    std::fs::write(dir.path().join("sys/kernel/random/boot_id"), "boot-two").unwrap();
    assert!(record.owner_dead(dir.path()));
    std::fs::remove_file(dir.path().join("sys/kernel/random/boot_id")).unwrap();
    assert!(!record.owner_dead(dir.path()));
    record.identity.owner_pid = None;
    assert!(!record.owner_dead(dir.path()));
}
#[test]
fn explicit_model_precedes_registered_exports_and_unknown_stays_absent() {
    let record = binding(Path::new("/fixture"), "s");
    let registry: Registry = serde_json::from_value(json!({"runtimes":[{"id":"codex","comm":["codex"],"sessionEnv":[],"modelEnv":["TEST_MODEL"]}]})).unwrap();
    let env = |_: &str| Some("exported".into());
    assert_eq!(
        envelope::model(Some("explicit"), &record.identity, &registry, &env).as_deref(),
        Some("explicit")
    );
    assert_eq!(
        envelope::model(None, &record.identity, &registry, &env).as_deref(),
        Some("exported")
    );
    assert_eq!(
        envelope::model(None, &record.identity, &registry, &|_| None),
        None
    );
}
#[test]
fn runtime_registry_and_envelope_view_match_their_schemas() {
    let registry: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../genesis/agentic/agent-runtimes.json"
    ))
    .unwrap();
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/manifest/agent-runtimes.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&registry)
        .unwrap();
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/views/session-envelope.schema.json"
    ))
    .unwrap();
    let view = json!({"live":true,"envelope":binding(Path::new("/fixture"), "s"),"berth":{},"claims":[],"covenant":{},"standing":{},"capabilityTier":"unknown","recallSession":"s","pinnedTools":{}});
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&view)
        .unwrap();
}

#[test]
fn liveness_and_guard_agree_at_every_close_checkpoint() {
    for (sealed, closed) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut record = binding(Path::new("/fixture"), "s");
        record.covenant_seal = sealed.then(|| "seal".into());
        record.closed_at = closed.then(|| "closed".into());
        assert_eq!(record.is_live(), !sealed && !closed);
        assert_eq!(Store::require_live(&record).is_ok(), record.is_live());
    }
}

fn process(root: &Path, pid: u32, start: u64) {
    std::fs::create_dir_all(root.join(pid.to_string())).unwrap();
    let mut fields = vec!["0".to_owned(); 20];
    fields[0] = "S".into();
    fields[19] = start.to_string();
    std::fs::write(
        root.join(format!("{pid}/stat")),
        format!("{pid} (codex) {}", fields.join(" ")),
    )
    .unwrap();
}

// Set fixture mode in a child test process, never in the parallel harness environment.
fn in_succession_fixture(name: &str) -> bool {
    if std::env::var("EPR_UNDER_TEST").as_deref() == Ok("1") {
        return true;
    }
    assert!(std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("EPR_UNDER_TEST", "1")
        .status()
        .unwrap()
        .success());
    false
}

// Facet release verified: the receipt a closed predecessor needs before it admits a successor.
fn receipt(store: &Store, record: &mut Envelope) {
    let cid = store
        .save_record(
            record.id(),
            record.generation,
            "release",
            &json!({"session": record.id(), "generation": record.generation}),
        )
        .unwrap();
    record.release_receipt = Some(cid);
}

fn resume_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("self/ns")).unwrap();
    std::fs::write(dir.path().join("self/ns/pid"), "namespace").unwrap();
    std::fs::create_dir_all(dir.path().join("sys/kernel/random")).unwrap();
    std::fs::write(dir.path().join("sys/kernel/random/boot_id"), "boot").unwrap();
    process(dir.path(), 7, 42);
    process(dir.path(), 8, 43);
    dir
}

fn successor(root: &Path) -> Envelope {
    let mut record = binding(root, "resume");
    record.identity.owner_pid = Some(8);
    record.identity.owner_start_time = Some(43);
    record
}

#[test]
fn successor_preserves_exact_legacy_bytes_seal_and_current_enumeration() {
    if !in_succession_fixture("successor_preserves_exact_legacy_bytes_seal_and_current_enumeration")
    {
        return;
    }
    for checkpoint in ["closed", "both"] {
        let dir = resume_fixture();
        let store = Store::lock(&dir.path().join("key")).unwrap();
        let mut old = store
            .open(binding(dir.path(), "resume"), dir.path())
            .unwrap();
        let seal = store
            .seal_covenant(&json!({"clauses": ["original"]}))
            .unwrap();
        old.closed_at = Some(envelope::now());
        old.covenant_seal = (checkpoint != "closed").then(|| seal.clone());
        old.berth_store = Some(dir.path().join("old-berth"));
        receipt(&store, &mut old);
        // Keep the predecessor in legacy, pretty-printed form: reserialization changes its CID.
        let mut legacy = serde_json::to_value(&old).unwrap();
        legacy.as_object_mut().unwrap().remove("generation");
        legacy.as_object_mut().unwrap().remove("predecessor");
        let bytes = serde_json::to_vec_pretty(&legacy).unwrap();
        std::fs::write(store.path("resume").unwrap(), &bytes).unwrap();
        let seal_path = dir.path().join(format!("sessions/seals/{seal}.json"));
        let sealed_bytes = std::fs::read(&seal_path).unwrap();
        std::fs::remove_dir_all(dir.path().join("7")).unwrap();
        let new = store.open(successor(dir.path()), dir.path()).unwrap();
        assert_eq!(new.generation, 2);
        assert!(new.is_live());
        assert!(new.berth_store.is_none());
        let predecessor = new.predecessor.as_ref().unwrap();
        assert_eq!(
            predecessor.record_cid,
            eprfs_core::BlobCid::compute_raw(&bytes).to_string()
        );
        assert_eq!(predecessor.covenant_seal, old.covenant_seal);
        assert_eq!(
            std::fs::read(dir.path().join("sessions/resume.gen/1.json")).unwrap(),
            bytes
        );
        assert_eq!(std::fs::read(&seal_path).unwrap(), sealed_bytes);
        assert_eq!(store.records().unwrap().len(), 1);
        assert_eq!(store.read("resume").unwrap().unwrap().generation, 2);
        assert_eq!(
            store
                .open(successor(dir.path()), dir.path())
                .unwrap()
                .generation,
            2
        );
        assert_eq!(new.covenant_session(), "resume/generations/2");
        assert_eq!(old.covenant_session(), "resume");
    }
}

#[test]
fn successor_may_resume_in_another_checkout_but_never_on_another_device_or_steward() {
    if !in_succession_fixture(
        "successor_may_resume_in_another_checkout_but_never_on_another_device_or_steward",
    ) {
        return;
    }
    for change in ["device", "steward", "checkout"] {
        let dir = resume_fixture();
        let store = Store::lock(&dir.path().join("key")).unwrap();
        let mut old = store
            .open(binding(dir.path(), "resume"), dir.path())
            .unwrap();
        old.closed_at = Some(envelope::now());
        receipt(&store, &mut old);
        store.save(&old).unwrap();
        std::fs::remove_dir_all(dir.path().join("7")).unwrap();
        let mut proposed = successor(dir.path());
        match change {
            "device" => proposed.device = "did:key:another-device".into(),
            "steward" => proposed.steward = "human:someone-else".into(),
            _ => proposed.repository = dir.path().join("worktree"),
        }
        let before = std::fs::read(store.path("resume").unwrap()).unwrap();
        let result = store.open(proposed, dir.path());
        if change == "checkout" {
            // Checkout carries no authority: the successor inherits nothing from it.
            let new = result.unwrap();
            assert_eq!(new.generation, 2);
            assert_eq!(new.repository, dir.path().join("worktree"));
            assert_eq!(
                std::fs::read(dir.path().join("sessions/resume.gen/1.json")).unwrap(),
                before
            );
        } else {
            assert_eq!(result.unwrap_err().kind, "session-conflict", "{change}");
            assert_eq!(
                std::fs::read(store.path("resume").unwrap()).unwrap(),
                before,
                "{change}"
            );
            assert!(
                !dir.path().join("sessions/resume.gen/1.json").exists(),
                "{change}"
            );
        }
    }
}

#[test]
fn successor_admission_refuses_unclosed_live_unknown_and_non_runtime_owners_without_writes() {
    if !in_succession_fixture(
        "successor_admission_refuses_unclosed_live_unknown_and_non_runtime_owners_without_writes",
    ) {
        return;
    }
    for reason in [
        "unclosed",
        "live",
        "namespace",
        "stat",
        "declared",
        "tty",
        "none",
    ] {
        let dir = resume_fixture();
        let store = Store::lock(&dir.path().join("key")).unwrap();
        let mut old = store
            .open(binding(dir.path(), "resume"), dir.path())
            .unwrap();
        if reason != "unclosed" {
            old.closed_at = Some(envelope::now());
        }
        if reason == "namespace" {
            old.owner_pid_namespace = Some("unobserved-namespace".into());
        }
        store.save(&old).unwrap();
        if !matches!(reason, "live" | "stat") {
            std::fs::remove_dir_all(dir.path().join("7")).unwrap();
        }
        if reason == "stat" {
            std::fs::remove_file(dir.path().join("7/stat")).unwrap();
            std::fs::create_dir(dir.path().join("7/stat")).unwrap();
        }
        let mut proposed = successor(dir.path());
        proposed.identity.source = match reason {
            "declared" => Source::Declared,
            "tty" => Source::TtyLeader,
            "none" => Source::None,
            _ => Source::RuntimeAncestor,
        };
        let before = std::fs::read(store.path("resume").unwrap()).unwrap();
        let error = store.open(proposed, dir.path()).unwrap_err();
        assert_eq!(error.kind, "envelope-owner-mismatch", "{reason}");
        assert_eq!(
            std::fs::read(store.path("resume").unwrap()).unwrap(),
            before,
            "{reason}"
        );
        assert!(!dir.path().join("sessions/resume.gen").exists(), "{reason}");
        // A blanket refusal must not satisfy this test: restore the three admission facts.
        old.closed_at = Some(envelope::now());
        old.owner_pid_namespace = envelope::owner_context(dir.path()).0;
        receipt(&store, &mut old);
        store.save(&old).unwrap();
        if dir.path().join("7").exists() {
            std::fs::remove_dir_all(dir.path().join("7")).unwrap();
        }
        assert_eq!(
            store
                .open(successor(dir.path()), dir.path())
                .unwrap()
                .generation,
            2,
            "{reason} positive control"
        );
    }
}

#[test]
fn successor_archive_retry_is_idempotent_and_conflicting_history_refuses() {
    if !in_succession_fixture(
        "successor_archive_retry_is_idempotent_and_conflicting_history_refuses",
    ) {
        return;
    }
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let mut old = store
        .open(binding(dir.path(), "resume"), dir.path())
        .unwrap();
    old.closed_at = Some(envelope::now());
    receipt(&store, &mut old);
    store.save(&old).unwrap();
    let bytes = std::fs::read(store.path("resume").unwrap()).unwrap();
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    let directory = dir.path().join("sessions/resume.gen");
    std::fs::create_dir_all(&directory).unwrap();
    let archive = directory.join("1.json");
    std::fs::write(&archive, b"different history").unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap_err()
            .kind,
        "envelope-generation-conflict"
    );
    assert_eq!(std::fs::read(store.path("resume").unwrap()).unwrap(), bytes);
    assert_eq!(std::fs::read(&archive).unwrap(), b"different history");
    std::fs::write(&archive, &bytes).unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap()
            .generation,
        2
    );
    assert_eq!(std::fs::read(&archive).unwrap(), bytes);
}

#[test]
fn commit_decoration_declines_unknown_closed_and_injectable_context() {
    let registry: Registry = serde_json::from_value(json!({"runtimes": [{
        "id": "codex", "comm": ["codex"], "sessionEnv": [],
        "commitAttribution": {"email": "noreply@openai.com", "modelSuffix": " (Codex)",
                              "sessionTrailer": "Codex-Session"}
    }]}))
    .unwrap();
    let mut record = binding(Path::new("/fixture"), "session-1");
    let derive =
        |record: &Envelope| elohim_device_runtime::commit_attribution::trailers(record, &registry);
    assert!(derive(&record).unwrap_err().contains("model is unmeasured"));
    record.model = Some("gpt-fixture".into());
    assert_eq!(
        derive(&record).unwrap(),
        vec![
            (
                "Co-Authored-By".into(),
                "gpt-fixture (Codex) <noreply@openai.com>".into()
            ),
            ("Codex-Session".into(), "session-1".into()),
        ]
    );
    for model in [
        "gpt-fixture\nOther: forged",
        "gpt-fixture <forged@example.test>",
        " ",
    ] {
        record.model = Some(model.into());
        assert!(derive(&record).is_err());
    }
    record.model = Some("gpt-fixture".into());
    record.identity.runtime = "unknown".into();
    assert!(derive(&record).unwrap_err().contains("no declared"));
    record.identity.runtime = "codex".into();
    record.closed_at = Some("2026-11-01T00:00:00Z".into());
    assert!(derive(&record).unwrap_err().contains("closed"));
}

// ---- release-first admission, owner holder, retirement ----

fn holder_fixture(dir: &Path) -> (Store, Envelope, std::path::PathBuf) {
    let store = Store::lock(&dir.join("key")).unwrap();
    let mut record = binding(dir, "resume");
    let lock = envelope::holder_lock(&dir.join("sessions"), "resume", 1);
    std::fs::create_dir_all(dir.join("sessions")).unwrap();
    std::fs::write(&lock, b"").unwrap();
    use std::os::unix::fs::MetadataExt;
    record.owner_holder = Some(envelope::OwnerHolder {
        lock: lock.clone(),
        generation: 1,
        armed_at: envelope::now(),
        inode: std::fs::metadata(&lock).unwrap().ino(),
    });
    (store, record, lock)
}

#[test]
fn foreign_namespace_reads_the_holder_lock_and_unknown_is_not_dead() {
    let dir = resume_fixture();
    let (_store, mut record, lock) = holder_fixture(dir.path());
    // The observer sits in another pid namespace on the same boot.
    record.owner_pid_namespace = Some("elsewhere".into());
    assert!(record.owner_dead(dir.path()), "free lock proves death");
    // Held lock: the owner's holder lives.
    let held = std::fs::File::open(&lock).unwrap();
    rustix::fs::flock(&held, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();
    assert!(!record.owner_dead(dir.path()));
    drop(held);
    // Missing file, replaced inode, wrong generation, symlink, legacy record: all unknown.
    let saved = record.owner_holder.clone();
    record.owner_holder.as_mut().unwrap().inode += 1;
    assert!(!record.owner_dead(dir.path()));
    record.owner_holder = saved.clone();
    record.owner_holder.as_mut().unwrap().generation = 2;
    assert!(!record.owner_dead(dir.path()));
    record.owner_holder = saved.clone();
    std::fs::remove_file(&lock).unwrap();
    assert!(!record.owner_dead(dir.path()));
    let target = dir.path().join("target");
    std::fs::write(&target, b"").unwrap();
    std::os::unix::fs::symlink(&target, &lock).unwrap();
    assert!(!record.owner_dead(dir.path()));
    record.owner_holder = None;
    assert!(!record.owner_dead(dir.path()));
    // A changed boot still proves death regardless.
    std::fs::write(dir.path().join("sys/kernel/random/boot_id"), "boot-two").unwrap();
    assert!(record.owner_dead(dir.path()));
}

#[test]
fn same_namespace_keeps_the_pid_check_over_the_lock() {
    let dir = resume_fixture();
    let (_store, record, _lock) = holder_fixture(dir.path());
    // Same namespace, owner pid 7 present with the recorded start: alive even with a free lock.
    assert!(!record.owner_dead(dir.path()));
    assert!(record.owner_observably_alive(dir.path()));
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    assert!(record.owner_dead(dir.path()));
    assert!(!record.owner_observably_alive(dir.path()));
}

#[test]
fn hold_owner_keeps_the_lock_until_the_owner_is_gone() {
    let dir = resume_fixture();
    let lock = dir.path().join("owner.lock");
    let (l, root) = (lock.clone(), dir.path().to_owned());
    let handle = std::thread::spawn(move || envelope::hold_owner(&l, 7, 42, &root));
    let probe = |lock: &Path| {
        let file = std::fs::File::open(lock).ok()?;
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared) {
            Ok(()) => Some(false),
            Err(_) => Some(true),
        }
    };
    for _ in 0..100 {
        if probe(&lock) == Some(true) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(probe(&lock), Some(true));
    // A second holder refuses while the first holds.
    assert_eq!(
        envelope::hold_owner(&lock, 7, 42, dir.path())
            .unwrap_err()
            .kind,
        "owner-holder-held"
    );
    // Unreadable stat is unknown: the holder keeps holding.
    std::fs::remove_file(dir.path().join("7/stat")).unwrap();
    std::fs::create_dir(dir.path().join("7/stat")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1300));
    assert_eq!(probe(&lock), Some(true));
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    handle.join().unwrap().unwrap();
    assert_eq!(probe(&lock), Some(false));
}

#[test]
fn open_arms_new_generation_before_save_and_rearms_a_live_record() {
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let armed = std::cell::RefCell::new(Vec::new());
    let arm = |_: &Store, record: &Envelope| {
        armed.borrow_mut().push(record.generation);
        Some(envelope::OwnerHolder {
            lock: "/fixture/lock".into(),
            generation: record.generation,
            armed_at: "t".into(),
            inode: 1,
        })
    };
    let first = store
        .open_armed(binding(dir.path(), "resume"), dir.path(), &arm)
        .unwrap();
    assert!(first.owner_holder.is_some());
    assert_eq!(armed.borrow().as_slice(), &[1]);
    // Persisted with the holder: armed before the record was saved.
    assert!(store
        .read("resume")
        .unwrap()
        .unwrap()
        .owner_holder
        .is_some());
    // The holder is not held (inode never matches): the live owner re-arms on re-open.
    store
        .open_armed(binding(dir.path(), "resume"), dir.path(), &arm)
        .unwrap();
    assert_eq!(armed.borrow().as_slice(), &[1, 1]);
    // Arming failure leaves the record exactly as it was.
    let before = std::fs::read(store.path("resume").unwrap()).unwrap();
    store
        .open_armed(binding(dir.path(), "resume"), dir.path(), &|_, _| None)
        .unwrap();
    assert_eq!(
        std::fs::read(store.path("resume").unwrap()).unwrap(),
        before
    );
}

#[test]
fn closed_without_receipt_refuses_and_interrupted_close_does_not_admit() {
    if !in_succession_fixture("closed_without_receipt_refuses_and_interrupted_close_does_not_admit")
    {
        return;
    }
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let mut old = store
        .open(binding(dir.path(), "resume"), dir.path())
        .unwrap();
    old.closed_at = Some(envelope::now());
    store.save(&old).unwrap();
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    let before = std::fs::read(store.path("resume").unwrap()).unwrap();
    let error = store.open(successor(dir.path()), dir.path()).unwrap_err();
    assert_eq!(error.kind, "envelope-release-unverified");
    assert!(error.detail.contains("session retire"));
    assert_eq!(
        std::fs::read(store.path("resume").unwrap()).unwrap(),
        before
    );
    // A receipt that does not hash to the recorded CID is not verification.
    old.release_receipt = Some("bafkr4ifake".into());
    store.save(&old).unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap_err()
            .kind,
        "envelope-release-unverified"
    );
    // Sealed but not closed: an interrupted close admits nothing, receipt or not.
    receipt(&store, &mut old);
    old.closed_at = None;
    old.covenant_seal = Some("seal".into());
    store.save(&old).unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap_err()
            .kind,
        "envelope-owner-mismatch"
    );
    assert!(!dir.path().join("sessions/resume.gen/1.json").exists());
    // Closed with a verified receipt admits generation two.
    old.closed_at = Some(envelope::now());
    store.save(&old).unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap()
            .generation,
        2
    );
}

#[test]
fn retirement_admits_a_successor_only_when_it_says_so_and_never_proves_death() {
    if !in_succession_fixture(
        "retirement_admits_a_successor_only_when_it_says_so_and_never_proves_death",
    ) {
        return;
    }
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let mut old = store
        .open(binding(dir.path(), "resume"), dir.path())
        .unwrap();
    // Owner pid 7 stays alive: only a retirement can admit.
    old.closed_at = Some(envelope::now());
    receipt(&store, &mut old);
    for (admit, expected) in [(false, "envelope-owner-mismatch"), (true, "")] {
        old.retirement = Some(
            store
                .save_record(
                    "resume",
                    1,
                    "retirement",
                    &json!({"session": "resume", "generation": 1, "admitSuccessor": admit}),
                )
                .unwrap(),
        );
        store.save(&old).unwrap();
        assert!(!old.owner_dead(dir.path()), "retirement is not death");
        let result = store.open(successor(dir.path()), dir.path());
        if admit {
            let new = result.unwrap();
            assert_eq!(new.generation, 2);
            assert_eq!(new.covenant_session(), "resume/generations/2");
            assert!(new.retirement.is_none() && new.release_receipt.is_none());
        } else {
            assert_eq!(result.unwrap_err().kind, expected);
        }
    }
}

#[test]
fn production_admission_reads_the_real_proc_not_the_fixture_root() {
    if std::env::var("EPR_UNDER_TEST").as_deref() == Ok("1") {
        return;
    }
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let mut old = store
        .open(binding(dir.path(), "resume"), dir.path())
        .unwrap();
    // The fixture root says pid 7 is gone; the real /proc says this very process lives.
    let (namespace, boot) = envelope::owner_context(Path::new("/proc"));
    old.owner_pid_namespace = namespace;
    old.owner_boot_id = boot;
    old.identity.owner_pid = Some(std::process::id());
    old.identity.owner_start_time =
        elohim_device_runtime::session::process_start_time(Path::new("/proc"), std::process::id());
    old.closed_at = Some(envelope::now());
    receipt(&store, &mut old);
    store.save(&old).unwrap();
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    assert_eq!(
        store
            .open(successor(dir.path()), dir.path())
            .unwrap_err()
            .kind,
        "envelope-owner-mismatch"
    );
}

#[test]
fn arm_holder_never_adopts_a_holder_watching_another_owner() {
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let lock = envelope::holder_lock(&dir.path().join("sessions"), "resume", 1);
    let (l, root) = (lock.clone(), dir.path().to_owned());
    // A's holder watches owner pid 7 / start 42 and holds the generation's lock.
    let handle = std::thread::spawn(move || envelope::hold_owner(&l, 7, 42, &root));
    for _ in 0..200 {
        if std::fs::read_to_string(&lock).is_ok_and(|t| t.trim() == "7 42") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // B (owner 8 / 43) retries the same generation: A's held lock is not B's holder.
    let b = successor(dir.path());
    assert!(envelope::arm_holder(&store, &b, Path::new("/bin/false")).is_none());
    // A's own owner does adopt it, without spawning anything.
    let a = binding(dir.path(), "resume");
    let adopted = envelope::arm_holder(&store, &a, Path::new("/bin/false")).unwrap();
    assert_eq!(adopted.generation, 1);
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    handle.join().unwrap().unwrap();
}

#[test]
fn a_superseding_retirement_is_a_new_file_and_never_replaces_the_cited_one() {
    let dir = resume_fixture();
    let store = Store::lock(&dir.path().join("key")).unwrap();
    let first = store
        .save_record("resume", 1, "retirement", &json!({"admitSuccessor": false}))
        .unwrap();
    let second = store
        .save_record(
            "resume",
            1,
            "retirement",
            &json!({"admitSuccessor": true, "supersedes": first}),
        )
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        store
            .read_record("resume", 1, "retirement", &first)
            .unwrap()["admitSuccessor"],
        false
    );
    assert_eq!(
        store
            .read_record("resume", 1, "retirement", &second)
            .unwrap()["supersedes"],
        json!(first)
    );
    // Same bytes again is write-once idempotent.
    assert_eq!(
        store
            .save_record("resume", 1, "retirement", &json!({"admitSuccessor": false}))
            .unwrap(),
        first
    );
    assert!(store
        .read_record("resume", 1, "retirement", "../x")
        .is_none());
}
