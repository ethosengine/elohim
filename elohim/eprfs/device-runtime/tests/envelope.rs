use elohim_device_runtime::{
    envelope::{self, Envelope, Store},
    session::{Registry, SessionIdentity, Source},
};
use serde_json::json;
use std::path::Path;

fn binding(home: &Path, id: &str) -> Envelope {
    Envelope {
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
#[test]
fn binding_is_idempotent_private_and_cannot_be_reused_or_traverse() {
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
