use elohim_device_runtime::{
    envelope::{
        self,
        stage::{self, NetworkStage},
        Envelope, Store,
    },
    session::{Registry, SessionIdentity, Source},
};
use serde_json::json;
use std::{ffi::OsString, path::Path};

fn binding(home: &Path, id: &str) -> Envelope {
    Envelope {
        generation: 1,
        predecessor: None,
        stage: NetworkStage::Enforced,
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
    }
}

fn stage_declaration(device_key: &Path, declared: &str) {
    let home = device_key.parent().unwrap();
    std::fs::create_dir_all(home).unwrap();
    std::fs::write(
        home.join("stage.json"),
        serde_json::to_vec(&json!({
            "stage": declared,
            "declaredBy": "test:device-runtime",
            "reason": "isolated stage contract",
            "futureField": "ignored by older readers",
        }))
        .unwrap(),
    )
    .unwrap();
}

fn stage_env(value: Option<&str>, dev_mode: Option<&str>) -> impl Fn(&str) -> Option<OsString> {
    let value = value.map(OsString::from);
    let dev_mode = dev_mode.map(OsString::from);
    move |key| match key {
        stage::DEVICE_STAGE_ENV => value.clone(),
        "DEV_MODE" => dev_mode.clone(),
        _ => None,
    }
}

#[test]
fn a_simulacra_session_in_an_enforced_home_is_refused_naming_the_isolated_home() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("device/ed25519.seed");
    let error = stage::resolve_with(&key, &stage_env(Some("simulacra"), None)).unwrap_err();
    assert_eq!(error.kind, "stage-mismatch");
    assert!(error
        .detail
        .contains("simulacra session in a enforced device home"));
    assert!(error
        .detail
        .contains("isolated XDG_CONFIG_HOME declared simulacra"));
}

#[test]
fn an_undeclared_process_in_a_simulacra_home_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("device/ed25519.seed");
    stage_declaration(&key, "simulacra");
    let error = stage::resolve_with(&key, &stage_env(None, None)).unwrap_err();
    assert_eq!(error.kind, "stage-mismatch");
    assert!(error
        .detail
        .contains("enforced session in a simulacra device home"));
}

#[test]
fn matching_explicit_simulacra_declarations_are_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("device/ed25519.seed");
    stage_declaration(&key, "simulacra");
    assert_eq!(
        stage::resolve_with(&key, &stage_env(Some("simulacra"), None)).unwrap(),
        NetworkStage::Simulacra
    );
}

#[test]
fn bootstrap_and_coordinated_are_refused_as_device_plane_declarations() {
    for declared in ["bootstrap", "coordinated"] {
        for source in ["process", "home"] {
            let dir = tempfile::tempdir().unwrap();
            let key = dir.path().join("device/ed25519.seed");
            let process = if source == "process" {
                Some(declared)
            } else {
                stage_declaration(&key, declared);
                None
            };
            let error = stage::resolve_with(&key, &stage_env(process, None)).unwrap_err();
            assert_eq!(error.kind, "stage-invalid");
            assert!(error.detail.contains(declared));
        }
    }
}

#[test]
fn stage_never_derives_from_dev_mode() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("device/ed25519.seed");
    assert_eq!(
        stage::resolve_with(&key, &stage_env(None, Some("1"))).unwrap(),
        NetworkStage::Enforced
    );
}

#[test]
fn device_stage_strings_round_trip_through_the_shared_vocabulary() {
    for stage in NetworkStage::ALL {
        assert_eq!(stage.label().parse::<NetworkStage>(), Ok(stage));
    }
}

#[test]
fn an_enforced_envelope_is_byte_identical_to_the_pre_stage_record() {
    const PRE_STAGE_ENVELOPE: &str = concat!(
        r#"{"generation":1,"predecessor":null,"identity":{"runtime":"codex","#,
        r#""session":"historic","ownerPid":7,"ownerStartTime":"42","#,
        r#""source":"runtimeAncestor"},"ownerPidNamespace":null,"ownerBootId":null,"#,
        r#""device":"did:key:fixture","steward":"human:matthew","model":null,"#,
        r#""repository":"/fixture/repo","berthStore":null,"#,
        r#""openedAt":"2026-10-27T00:00:00Z","closedAt":null,"#,
        r#""covenantSeal":null,"witness":null}"#,
    );
    const PRE_STAGE_RECORD_CID: &str =
        "bafkreicjqumvehgp2p5vq7vrq47otft7uovyybjxfdzc54ghdweehh6hd4";

    let historic = PRE_STAGE_ENVELOPE.as_bytes();
    let decoded: Envelope = serde_json::from_slice(historic).unwrap();
    assert_eq!(decoded.stage, NetworkStage::Enforced);
    let rewritten = serde_json::to_vec(&decoded).unwrap();
    assert_eq!(rewritten, historic);
    assert_eq!(
        eprfs_core::BlobCid::compute_raw(historic).to_string(),
        PRE_STAGE_RECORD_CID
    );
    assert_eq!(
        eprfs_core::BlobCid::compute_raw(&rewritten).to_string(),
        PRE_STAGE_RECORD_CID
    );
}

#[test]
fn a_simulacra_envelope_carries_the_shared_stage_string() {
    let mut record = binding(Path::new("/fixture"), "simulacra");
    record.stage = NetworkStage::Simulacra;
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["stage"], "simulacra");
    assert_eq!(
        serde_json::from_value::<Envelope>(value).unwrap().stage,
        NetworkStage::Simulacra
    );
}

#[test]
fn simulacra_sessions_use_the_stage_partition_without_moving_enforced_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("device/ed25519.seed");
    let enforced = Store::lock_for_stage(&key, NetworkStage::Enforced).unwrap();
    assert_eq!(
        enforced.path("enforced").unwrap(),
        dir.path().join("device/sessions/enforced.json")
    );
    drop(enforced);
    let simulacra = Store::lock_for_stage(&key, NetworkStage::Simulacra).unwrap();
    assert_eq!(
        simulacra.path("simulacra").unwrap(),
        dir.path()
            .join("device/stages/simulacra/sessions/simulacra.json")
    );
    let stage_less = serde_json::to_vec(&binding(dir.path(), "simulacra")).unwrap();
    std::fs::write(simulacra.path("simulacra").unwrap(), stage_less).unwrap();
    let error = simulacra.read("simulacra").unwrap_err();
    assert_eq!(error.kind, "stage-mismatch");
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
    for checkpoint in ["closed", "sealed", "both"] {
        let dir = resume_fixture();
        let store = Store::lock(&dir.path().join("key")).unwrap();
        let mut old = store
            .open(binding(dir.path(), "resume"), dir.path())
            .unwrap();
        let seal = store
            .seal_covenant(&json!({"clauses": ["original"]}))
            .unwrap();
        old.closed_at = (checkpoint != "sealed").then(envelope::now);
        old.covenant_seal = (checkpoint != "closed").then(|| seal.clone());
        old.berth_store = Some(dir.path().join("old-berth"));
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
            assert!(!dir.path().join("sessions/resume.gen").exists(), "{change}");
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
    store.save(&old).unwrap();
    let bytes = std::fs::read(store.path("resume").unwrap()).unwrap();
    std::fs::remove_dir_all(dir.path().join("7")).unwrap();
    let directory = dir.path().join("sessions/resume.gen");
    std::fs::create_dir(&directory).unwrap();
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
