// Python parity table: each row names the native test replacing that Python function.
// Installation/guard cases exercise the native installation boundary; fail-open hook cases
// also retain Python adapter tests. Launcher implementation changes from Python to native.
// test_session_key_uses_nearest_claude_ancestor_and_starttime -> test_session_key_uses_nearest_claude_ancestor_and_starttime
// test_session_key_falls_back_to_declared_environment -> test_session_key_falls_back_to_declared_environment
// test_resolution_order_is_candidate_explicit_session_then_current -> test_resolution_order_is_candidate_explicit_session_then_current
// test_exec_resolution_checks_executable_without_hashing -> test_exec_resolution_checks_executable_without_hashing
// test_client_reports_the_resolved_executable_cid_without_spawning -> test_client_reports_the_resolved_executable_cid_without_spawning
// test_client_uses_the_shared_bin_store_override -> test_client_uses_the_shared_bin_store_override
// test_launcher_ignores_an_unrelated_malformed_registry_entry -> test_launcher_ignores_an_unrelated_malformed_registry_entry
// test_launcher_alias_is_refused -> test_launcher_alias_is_refused
// test_launcher_executes_the_python_resolver -> test_launcher_executes_the_python_resolver
// test_every_stored_exec_refreshes_the_usage_lease -> test_every_stored_exec_refreshes_the_usage_lease
// test_atomic_pin_creation_recovers_an_empty_pin -> test_atomic_pin_creation_recovers_an_empty_pin
// test_two_launcher_processes_publish_one_complete_pin -> test_two_launcher_processes_publish_one_complete_pin
// test_ingest_hashes_staged_bytes_and_publishes_read_only -> test_ingest_hashes_staged_bytes_and_publishes_read_only
// test_bootstrap_repairs_a_damaged_existing_artifact_from_verified_staging -> test_bootstrap_repairs_a_damaged_existing_artifact_from_verified_staging
// test_land_updates_heads_under_lock -> test_land_updates_heads_under_lock
// test_reingest_preserves_primary_provenance_and_appends_sources -> test_reingest_preserves_primary_provenance_and_appends_sources
// test_land_ancestry_uses_current_json_not_mutable_ingest_provenance -> test_land_ancestry_uses_current_json_not_mutable_ingest_provenance
// test_health_runs_outside_lock_and_checkout_is_revalidated_afterward -> test_health_runs_outside_lock_and_checkout_is_revalidated_afterward
// test_land_refuses_when_current_moves_during_health -> test_land_refuses_when_current_moves_during_health
// test_land_refuses_failed_health_without_moving_current -> test_land_refuses_failed_health_without_moving_current
// test_land_checks_record_head_and_checkout_cleanliness -> test_land_checks_record_head_and_checkout_cleanliness
// test_land_requires_descent_unless_revert -> test_land_requires_descent_unless_revert
// test_gc_holds_lock_and_keeps_heads_live_pins_and_grace -> test_gc_holds_lock_and_keeps_heads_live_pins_and_grace
// test_gc_keeps_an_old_artifact_used_within_fourteen_days -> test_gc_keeps_an_old_artifact_used_within_fourteen_days
// test_exec_pin_lock_timeout_falls_back_to_current_with_one_warning -> test_exec_pin_lock_timeout_falls_back_to_current_with_one_warning
// test_other_lock_waits_refuse_with_a_clear_bound -> test_other_lock_waits_refuse_with_a_clear_bound
// test_bootstrap_environment_uses_gate_toolchain_and_resource_caps -> test_bootstrap_environment_uses_gate_toolchain_and_resource_caps
// test_headless_bootstrap_resumes_an_existing_candidate -> test_headless_bootstrap_resumes_an_existing_candidate
// test_guard_installs_only_after_a_verified_head_into_persistent_device_bin -> test_guard_installs_only_after_a_verified_head_into_persistent_device_bin
// test_guard_uses_the_shared_bin_store_override -> test_guard_uses_the_shared_bin_store_override
// test_guard_registry_initialization_failure_is_fail_open -> test_guard_registry_initialization_failure_is_fail_open
// test_guard_expands_relative_install_paths_to_absolute -> test_guard_expands_relative_install_paths_to_absolute
// test_guard_restores_previous_link_when_installed_verification_fails -> test_guard_restores_previous_link_when_installed_verification_fails
// test_guard_leaves_existing_path_entry_untouched_when_bootstrap_refuses -> test_guard_leaves_existing_path_entry_untouched_when_bootstrap_refuses
// test_cleanliness_is_judged_over_epr_sources_not_hook_ledgers -> test_cleanliness_is_judged_over_epr_sources_not_hook_ledgers
// test_source_paths_fall_back_to_the_whole_tree_without_a_cargo_answer -> test_source_paths_fall_back_to_the_whole_tree_without_a_cargo_answer
// test_shadows_lists_every_epr_reached_before_the_launcher -> test_shadows_lists_every_epr_reached_before_the_launcher
// test_guard_keeps_a_hand_installed_binary_and_repoints_stale_shadows -> test_guard_keeps_a_hand_installed_binary_and_repoints_stale_shadows
// test_guard_preserves_distinct_plain_alias_bytes_as_separate_candidates -> test_guard_preserves_distinct_plain_alias_bytes_as_separate_candidates
// test_follow_adopts_a_newer_published_dev_commit_the_checkout_holds -> test_follow_adopts_a_newer_published_dev_commit_the_checkout_holds
// test_follow_leaves_the_head_for_an_unpublished_or_unchanged_checkout -> test_follow_leaves_the_head_for_an_unpublished_or_unchanged_checkout
// test_follow_does_nothing_when_epr_sources_did_not_change -> test_follow_does_nothing_when_epr_sources_did_not_change
// test_brit_multibin_ingest_uses_canonical_sums_rcid -> test_brit_multibin_ingest_uses_canonical_sums_rcid
// test_rakia_alias_execs_brit_with_rakia_as_argv_zero -> test_rakia_alias_execs_brit_with_rakia_as_argv_zero
// test_session_pins_are_scoped_per_tool -> test_session_pins_are_scoped_per_tool
// test_brit_follow_refuses_unpublished_head -> test_brit_follow_refuses_unpublished_head
// test_direct_brit_land_requires_witnessed_success_before_health -> test_direct_brit_land_requires_witnessed_success_before_health
// test_brit_follow_refuses_unreachable_or_red_attestation_for_every_candidate_source -> test_brit_follow_refuses_unreachable_or_red_attestation_for_every_candidate_source
// test_brit_follow_refuses_a_pin_that_does_not_descend -> test_brit_follow_refuses_a_pin_that_does_not_descend
// test_brit_follow_adopts_verified_fake_release -> test_brit_follow_adopts_verified_fake_release
// test_brit_follow_names_both_cures_when_no_bytes_source_exists -> test_brit_follow_names_both_cures_when_no_bytes_source_exists
// test_brit_headless_bootstrap_requires_attestation_and_uses_the_same_release_source -> test_brit_headless_bootstrap_requires_attestation_and_uses_the_same_release_source
// test_the_brit_registry_credential_prefers_the_cargo_token_then_npm_token -> test_the_brit_registry_credential_prefers_the_cargo_token_then_npm_token
// test_a_daemon_follows_the_device_head_not_the_launching_session -> test_a_daemon_follows_the_device_head_not_the_launching_session
// test_a_daemon_lease_is_owned_by_the_daemon_process -> test_a_daemon_lease_is_owned_by_the_daemon_process

use super::files::*;
use super::*;
use crate::session::{SessionIdentity, Source};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, UNIX_EPOCH},
};

#[derive(Default)]
struct State {
    head: String,
    dirty: bool,
    descended: bool,
    published: bool,
    unchanged: bool,
    health_fail: bool,
    health_dirty: bool,
    health_move: Option<String>,
    verdict: Option<Value>,
    judge_calls: usize,
    calls: Vec<CommandSpec>,
    install_fail: bool,
    metadata: Option<Value>,
}
struct Fake {
    state: Arc<Mutex<State>>,
    root: PathBuf,
    tool: String,
}
impl Runner for Fake {
    fn run(&self, c: &CommandSpec) -> Result<CommandOutput> {
        let mut s = self.state.lock().unwrap();
        s.calls.push(c.clone());
        let mut out = String::new();
        let mut success = true;
        if c.program == "git" {
            let a = &c.args[2..];
            match a.first().map(String::as_str).unwrap_or("") {
                "rev-parse" => {
                    out = if a[1] == "--show-toplevel" {
                        self.root.display().to_string()
                    } else {
                        s.head.clone()
                    }
                }
                "status" => {
                    out = if s.dirty {
                        " M src/lib.rs".into()
                    } else {
                        String::new()
                    }
                }
                "ls-tree" => out = format!("160000 commit {}\telohim/brit", s.head),
                "merge-base" => {
                    success = if a.last().unwrap().starts_with("refs/") {
                        s.published
                    } else {
                        s.descended
                    }
                }
                "diff" => success = s.unchanged,
                "cat-file" | "worktree" => {}
                _ => success = false,
            }
        } else if c.program == "cargo" {
            if c.args[0] == "metadata" {
                if let Some(m) = &s.metadata {
                    out = m.to_string();
                } else {
                    success = false;
                }
            } else {
                success = false;
            }
        } else if c.program == "node" {
            s.judge_calls += 1;
            out = s
                .verdict
                .clone()
                .unwrap_or(json!({"outcome":"pass","tier":"witnessed","conclusion":"success"}))
                .to_string();
        } else if c.args == ["flow", "report", "scope"] {
            let lock = Lock::acquire(&self.root.join("store"), Duration::from_millis(20)).unwrap();
            drop(lock);
            if s.health_dirty {
                s.dirty = true;
            }
            if let Some(cid) = s.health_move.take() {
                atomic_link(&self.root.join("store/current"), Path::new(&cid)).unwrap();
                atomic_json(
                    &self.root.join("store/current.json"),
                    &json!({"cid":cid,"commit":s.head}),
                )
                .unwrap();
            }
            success = !s.health_fail;
        } else if c.args == ["--version"] {
            if c.program.contains("cargo-bin") {
                success = !s.install_fail;
            } else if self.tool == "brit" {
                out = format!("brit fixture\nsource HEAD: {}\n", s.head);
            } else {
                out=json!({"id":"elohim-epr-cli","version":"fixture","cid":file_cid(Path::new(&c.program))?}).to_string();
            }
        } else {
            success = false;
        }
        Ok(CommandOutput {
            success,
            stdout: out,
            stderr: String::new(),
        })
    }
}
struct Fixture {
    tmp: tempfile::TempDir,
    store: Store,
    state: Arc<Mutex<State>>,
    registry: Registry,
}
impl Fixture {
    fn new(tool: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry: Registry = serde_json::from_str(include_str!(
            "../../../../../genesis/agentic/bin-tools.json"
        ))
        .unwrap();
        let state = Arc::new(Mutex::new(State {
            head: "a".repeat(40),
            descended: true,
            published: true,
            ..Default::default()
        }));
        let runner = Arc::new(Fake {
            root: tmp.path().into(),
            state: state.clone(),
            tool: tool.into(),
        });
        let store = Store::new(tmp.path().join("store"), tool, &registry)
            .unwrap()
            .with_runner(runner);
        Self {
            tmp,
            store,
            state,
            registry,
        }
    }
    fn path(&self, p: &str) -> PathBuf {
        self.tmp.path().join(p)
    }
    fn commit(&self) -> String {
        self.state.lock().unwrap().head.clone()
    }
    fn release(&self, label: &str) -> Release {
        let source = self.path(label);
        fs::create_dir_all(&source).unwrap();
        let mut sums = BTreeMap::new();
        for name in &self.store.config.bins {
            atomic_bytes(
                &source.join(name),
                format!("#!/bin/sh\n# {label} {name}\nexit 0\n").as_bytes(),
                0o755,
            )
            .unwrap();
            sums.insert(name.clone(), file_cid(&source.join(name)).unwrap());
        }
        for (a, t) in &self.store.config.aliases {
            sums.insert(a.clone(), sums[t].clone());
        }
        atomic_bytes(
            &source.join("sums.cbor"),
            &sums_manifest(&sums).unwrap(),
            0o444,
        )
        .unwrap();
        self.store
            .ingest(
                &source,
                &IngestOptions {
                    source_commit: Some(self.commit()),
                    tree_clean: Some(true),
                    gate_run: Some("test".into()),
                    ..Default::default()
                },
            )
            .unwrap()
    }
    fn head(&self, r: &Release) {
        atomic_link(&self.store.root.join("current"), Path::new(&r.cid)).unwrap();
        atomic_json(
            &self.store.root.join("current.json"),
            &json!({"cid":r.cid,"commit":self.commit()}),
        )
        .unwrap();
    }
    fn land(&self, r: &Release) -> Result<Release> {
        self.store
            .land(&r.cid, &self.commit(), self.tmp.path(), false)
    }
    fn install(&self) -> Result<Value> {
        atomic_bytes(&self.path("native"), b"#!/bin/sh\nexit 0\n", 0o755).unwrap();
        self.store.install_launcher(
            &self.path("native"),
            &self.path("cargo-bin"),
            &self.registry,
            self.tmp.path(),
        )
    }
}
fn identity(key: &str) -> SessionIdentity {
    SessionIdentity {
        runtime: "declared".into(),
        session: Some(key.into()),
        owner_pid: None,
        owner_start_time: None,
        source: Source::Declared,
    }
}
fn proc_entry(root: &Path, pid: u32, parent: u32, comm: &str, start: u64) {
    let mut fields = vec!["0".to_owned(); 20];
    fields[0] = "S".into();
    fields[1] = parent.to_string();
    fields[19] = start.to_string();
    atomic_bytes(
        &root.join(pid.to_string()).join("stat"),
        format!("{pid} ({comm}) {}", fields.join(" ")).as_bytes(),
        0o644,
    )
    .unwrap();
}
fn age(f: &Fixture, r: &Release, at: &str) {
    let mut r = r.clone();
    r.provenance.ingested_at = at.into();
    f.store.save(&r).unwrap();
}
#[test]
fn test_session_key_uses_nearest_claude_ancestor_and_starttime() {
    let t = tempfile::tempdir().unwrap();
    proc_entry(t.path(), 400, 300, "rust", 40);
    proc_entry(t.path(), 300, 200, "bash", 30);
    proc_entry(t.path(), 200, 100, "claude", 20);
    proc_entry(t.path(), 100, 1, "claude", 10);
    let registry = serde_json::from_str(
        r#"{"runtimes":[{"id":"claude","comm":["claude"],"sessionEnv":["NATIVE"]}]}"#,
    )
    .unwrap();
    let i = crate::session::resolve_with(
        &|k| (k == "NATIVE").then(|| "native".into()),
        t.path(),
        400,
        &registry,
    );
    assert_eq!(
        (i.session.as_deref(), i.owner_pid, i.owner_start_time),
        (Some("native"), Some(200), Some(20))
    );
}
#[test]
fn test_session_key_falls_back_to_declared_environment() {
    let i = crate::session::resolve_with(
        &|k| (k == "ELOHIM_SESSION_ID").then(|| "declared".into()),
        Path::new("/absent"),
        1,
        &Default::default(),
    );
    assert_eq!(i.session.as_deref(), Some("declared"));
    assert_eq!(i.source, Source::Declared);
}
#[test]
fn test_resolution_order_is_candidate_explicit_session_then_current() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    let b = f.release("b");
    f.head(&a);
    f.store.pin(&identity("s"), None, false, None).unwrap();
    f.head(&b);
    let env = |k: &str| match k {
        "EPR_BIN" => Some(f.path("a/epr").display().to_string()),
        "EPR_PIN_CID" => Some(b.cid.clone()),
        _ => None,
    };
    assert_eq!(
        f.store
            .resolve(None, Some(&identity("s")), &env, false)
            .unwrap()
            .source,
        "candidate"
    );
    assert_eq!(
        f.store
            .resolve(
                None,
                Some(&identity("s")),
                &|k| (k == "EPR_PIN_CID").then(|| b.cid.clone()),
                false
            )
            .unwrap()
            .source,
        "explicit-pin"
    );
    assert_eq!(
        f.store
            .resolve(None, Some(&identity("s")), &|_| None, false)
            .unwrap()
            .cid,
        Some(a.cid)
    );
    assert_eq!(
        f.store.resolve(None, None, &|_| None, false).unwrap().cid,
        Some(b.cid)
    );
}
#[test]
fn test_exec_resolution_checks_executable_without_hashing() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(
        &f.store.root.join(&r.cid).join("epr"),
        b"changed bytes",
        0o111,
    )
    .unwrap();
    assert!(!f.store.verified(&r.cid));
    assert_eq!(
        f.store.resolve(None, None, &|_| None, true).unwrap().cid,
        Some(r.cid)
    );
    assert!(f
        .store
        .resolve(
            None,
            None,
            &|k| (k == "EPR_BIN").then(|| f.path("a/epr").display().to_string()),
            true
        )
        .unwrap()
        .cid
        .is_none());
}
#[test]
fn test_client_reports_the_resolved_executable_cid_without_spawning() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    f.state.lock().unwrap().calls.clear();
    let s = f
        .store
        .status(&identity("s"), None, false, &|_| None)
        .unwrap();
    assert_eq!(s.resolved_cid, Some(r.cid));
    assert!(f.state.lock().unwrap().calls.is_empty());
}
#[test]
fn test_client_uses_the_shared_bin_store_override() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let s = f
        .store
        .status(&identity("s"), None, false, &|_| None)
        .unwrap();
    assert_eq!(s.store, f.store.root);
    assert!(s.resolved.unwrap().starts_with(f.store.root()));
}
#[test]
fn test_launcher_ignores_an_unrelated_malformed_registry_entry() {
    let r: Registry = serde_json::from_value(
        json!({"version":1,"tools":{"epr":{"bins":["epr"]},"brit":{"bins":"brit"}}}),
    )
    .unwrap();
    assert_eq!(
        r.invoked(Path::new("/x/epr")).unwrap(),
        ("epr".into(), "epr".into())
    );
}
#[test]
fn test_launcher_alias_is_refused() {
    let f = Fixture::new("epr");
    atomic_bytes(&f.path("bin/bin-launch"), b"launcher", 0o755).unwrap();
    symlink(f.path("bin/bin-launch"), f.path("candidate")).unwrap();
    assert_eq!(
        f.store
            .resolve(
                None,
                None,
                &|k| (k == "EPR_BIN").then(|| f.path("candidate").display().to_string()),
                true
            )
            .unwrap_err()
            .kind,
        "launcher-alias"
    );
}
#[test]
fn test_launcher_executes_the_python_resolver() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let resolved = f.store.resolve(None, None, &|_| None, true).unwrap();
    assert_eq!(resolved.path, f.store.root.join(r.cid).join("epr"));
    assert!(!resolved.path.ends_with("bin-launch"));
}
#[test]
fn test_every_stored_exec_refreshes_the_usage_lease() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    for mode in 0..3 {
        let env = |k: &str| (mode == 1 && k == "EPR_PIN_CID").then(|| r.cid.clone());
        let id = identity("s");
        let resolved = f
            .store
            .resolve(None, (mode == 2).then_some(&id), &env, true)
            .unwrap();
        f.store
            .touch_used(resolved.cid.as_deref().unwrap())
            .unwrap();
        assert!(f.store.root.join(&r.cid).join(".used").exists());
    }
}
#[test]
fn test_atomic_pin_creation_recovers_an_empty_pin() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(&f.store.root.join("sessions/s"), b"", 0o600).unwrap();
    assert_eq!(
        f.store.pin(&identity("s"), None, false, None).unwrap().cid,
        r.cid
    );
    let v: Value = read_json(&f.store.root.join("sessions/s")).unwrap();
    assert!(v["cid"].is_string());
}
#[test]
fn test_two_launcher_processes_publish_one_complete_pin() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let s = f.store.clone();
            std::thread::spawn(move || s.pin(&identity("same"), None, false, None).unwrap().cid)
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), r.cid);
    }
    let pin: Pin = read_json(&f.store.root.join("sessions/same")).unwrap();
    assert_eq!(pin.cid, r.cid);
}
#[test]
fn test_ingest_hashes_staged_bytes_and_publishes_read_only() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    let source = f.path("a/epr");
    fs::write(source, b"changed after staging").unwrap();
    let path = f.store.root.join(&r.cid).join("epr");
    assert_eq!(file_cid(&path).unwrap(), r.cid);
    assert_eq!(path.metadata().unwrap().permissions().mode() & 0o222, 0);
    assert!(children(&f.store.root).iter().all(|p| !p
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("staging")));
}
#[test]
fn test_bootstrap_repairs_a_damaged_existing_artifact_from_verified_staging() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(&f.store.root.join(&r.cid).join("epr"), b"damaged", 0o755).unwrap();
    let repaired = f.store.bootstrap(f.tmp.path(), Some(&f.path("a"))).unwrap();
    assert_eq!(repaired.cid, r.cid);
    assert!(f.store.verified(&r.cid));
}
#[test]
fn test_land_updates_heads_under_lock() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    let b = f.release("b");
    f.land(&a).unwrap();
    f.land(&b).unwrap();
    assert_eq!(f.store.head("previous"), Some(a.cid));
    assert_eq!(f.store.head("current"), Some(b.cid.clone()));
    let v: Value = read_json(&f.store.root.join("current.json")).unwrap();
    assert_eq!(v["cid"], b.cid);
    assert_eq!(v["commit"], f.commit());
}
#[test]
fn test_reingest_preserves_primary_provenance_and_appends_sources() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("a");
    assert_eq!(b.cid, a.cid);
    assert_eq!(b.provenance.source_commit, a.provenance.source_commit);
    assert_eq!(b.sources.len(), 2);
    f.land(&b).unwrap();
}
#[test]
fn test_land_ancestry_uses_current_json_not_mutable_ingest_provenance() {
    let f = Fixture::new("epr");
    let mut a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("b");
    a.provenance.source_commit = f.commit();
    f.store.save(&a).unwrap();
    f.state.lock().unwrap().descended = false;
    assert_eq!(f.land(&b).unwrap_err().kind, "non-descendant");
    let s = f.state.lock().unwrap();
    assert!(s
        .calls
        .iter()
        .any(|c| c.args.contains(&"merge-base".into()) && c.args.contains(&"a".repeat(40))));
}
#[test]
fn test_health_runs_outside_lock_and_checkout_is_revalidated_afterward() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.state.lock().unwrap().health_dirty = true;
    assert_eq!(f.land(&r).unwrap_err().kind, "checkout-dirty");
    assert!(f.store.head("current").is_none());
}
#[test]
fn test_land_refuses_when_current_moves_during_health() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    let b = f.release("b");
    let c = f.release("c");
    f.land(&a).unwrap();
    f.state.lock().unwrap().health_move = Some(c.cid.clone());
    assert_eq!(f.land(&b).unwrap_err().kind, "current-moved");
    assert_eq!(f.store.head("current"), Some(c.cid));
}
#[test]
fn test_land_refuses_failed_health_without_moving_current() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    let b = f.release("b");
    f.state.lock().unwrap().health_fail = true;
    assert_eq!(f.land(&b).unwrap_err().kind, "health-flow-failed");
    assert_eq!(f.store.head("current"), Some(a.cid));
}
#[test]
fn test_land_checks_record_head_and_checkout_cleanliness() {
    let f = Fixture::new("epr");
    let mut r = f.release("a");
    r.sources[0].tree_clean = false;
    f.store.save(&r).unwrap();
    assert_eq!(f.land(&r).unwrap_err().kind, "dirty-build");
    r.sources[0].tree_clean = true;
    f.store.save(&r).unwrap();
    assert_eq!(
        f.store
            .land(&r.cid, "missing", f.tmp.path(), false)
            .unwrap_err()
            .kind,
        "commit-mismatch"
    );
    f.state.lock().unwrap().head = "b".repeat(40);
    assert_eq!(
        f.store
            .land(&r.cid, &r.provenance.source_commit, f.tmp.path(), false)
            .unwrap_err()
            .kind,
        "head-mismatch"
    );
    f.state.lock().unwrap().head = r.provenance.source_commit.clone();
    f.state.lock().unwrap().dirty = true;
    assert_eq!(f.land(&r).unwrap_err().kind, "checkout-dirty");
}
#[test]
fn test_land_requires_descent_unless_revert() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    let b = f.release("b");
    f.state.lock().unwrap().descended = false;
    assert_eq!(f.land(&b).unwrap_err().kind, "non-descendant");
    assert_eq!(
        f.store
            .land(&b.cid, &f.commit(), f.tmp.path(), true)
            .unwrap()
            .ring,
        "reverted"
    );
    assert_eq!(f.store.head("previous"), Some(a.cid));
}
#[test]
fn test_gc_holds_lock_and_keeps_heads_live_pins_and_grace() {
    let f = Fixture::new("epr");
    let rows: Vec<_> = [
        "current",
        "previous",
        "live",
        "ownerless",
        "recent",
        "garbage",
    ]
    .into_iter()
    .map(|s| f.release(s))
    .collect();
    for r in &rows {
        age(&f, r, "2026-01-01T00:00:00Z");
    }
    f.head(&rows[0]);
    atomic_link(&f.store.root.join("previous"), Path::new(&rows[1].cid)).unwrap();
    proc_entry(&f.path("proc"), 200, 1, "codex", 20);
    for (key, r, pid, start) in [
        ("live", &rows[2], Some(200), Some("20")),
        ("ownerless", &rows[3], None, None),
        ("dead", &rows[5], Some(200), Some("10")),
    ] {
        atomic_json(
            &f.store.root.join("sessions").join(key),
            &Pin {
                cid: r.cid.clone(),
                session: key.into(),
                pinned_at: "2026-10-23T00:00:00Z".into(),
                owner_pid: pid,
                owner_start_time: start.map(str::to_owned),
                provisional: false,
            },
        )
        .unwrap();
    }
    age(&f, &rows[4], "2026-10-23T00:00:00Z");
    let result = f
        .store
        .gc(&f.path("proc"), parse_time("2026-10-24T00:00:00Z"))
        .unwrap();
    assert_eq!(result["kept"].as_array().unwrap().len(), 5);
    assert_eq!(result["removed"], json!([rows[5].cid]));
    assert_eq!(result["removedSessions"], json!(["dead"]));
}
#[test]
fn test_gc_keeps_an_old_artifact_used_within_fourteen_days() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    age(&f, &r, "2026-01-01T00:00:00Z");
    f.store.touch_used(&r.cid).unwrap();
    let at = parse_time("2026-10-24T00:00:00Z").unwrap();
    fs::File::open(f.store.root.join(&r.cid).join(".used"))
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(at - 13 * 86400))
        .unwrap();
    assert_eq!(
        f.store.gc(&f.path("proc"), Some(at)).unwrap()["removed"],
        json!([])
    );
}
#[test]
fn test_exec_pin_lock_timeout_falls_back_to_current_with_one_warning() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let _lock = Lock::acquire(&f.store.root, Duration::ZERO).unwrap();
    let result = f
        .store
        .resolve(None, Some(&identity("waiting")), &|_| None, true)
        .unwrap();
    assert_eq!(result.source, "current-lock-timeout");
    assert_eq!(result.cid, Some(r.cid));
    assert!(!f.store.root.join("sessions/waiting").exists());
}
#[test]
fn test_other_lock_waits_refuse_with_a_clear_bound() {
    let t = tempfile::tempdir().unwrap();
    let _lock = Lock::acquire(t.path(), Duration::ZERO).unwrap();
    let Err(e) = Lock::acquire(t.path(), Duration::from_millis(10)) else {
        panic!("lock must refuse")
    };
    assert_eq!(e.kind, "lock-timeout");
    assert!(e.detail.contains("0.01s"));
}
#[test]
fn test_bootstrap_environment_uses_gate_toolchain_and_resource_caps() {
    let f = Fixture::new("epr");
    atomic_json(&f.path("genesis/agentic/pool-policy.json"),&json!({"cargo_env_overrides":{"*":{"RUSTUP_TOOLCHAIN":"1.96.1"},"eprfs":{"CARGO_BUILD_JOBS":"1","RUST_TEST_THREADS":"1"}}})).unwrap();
    let e = f.store.build_environment(f.tmp.path(), &f.path("target"));
    assert_eq!(e["RUSTUP_TOOLCHAIN"].as_deref(), Some("1.96.1"));
    assert_eq!(e["CARGO_BUILD_JOBS"].as_deref(), Some("1"));
    assert_eq!(e["RUSTFLAGS"].as_deref(), Some(""));
}
#[test]
fn test_headless_bootstrap_resumes_an_existing_candidate() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    assert_eq!(f.store.bootstrap(f.tmp.path(), None).unwrap().cid, r.cid);
    assert_eq!(f.store.head("current"), Some(r.cid));
    assert!(!f
        .state
        .lock()
        .unwrap()
        .calls
        .iter()
        .any(|c| c.program == "cargo" && c.args[0] == "build"));
}
#[test]
fn test_guard_installs_only_after_a_verified_head_into_persistent_device_bin() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    assert_eq!(f.install().unwrap_err().kind, "unverified-current");
    assert!(!f.path("cargo-bin/epr").exists());
    f.head(&r);
    let result = f.install().unwrap();
    assert_eq!(result["head"], r.cid);
    assert_eq!(
        f.path("cargo-bin/epr").canonicalize().unwrap(),
        f.path("bin/bin-launch")
    );
}
#[test]
fn test_guard_uses_the_shared_bin_store_override() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    f.install().unwrap();
    let s = f.state.lock().unwrap();
    let check = s
        .calls
        .iter()
        .find(|c| c.program.contains("cargo-bin"))
        .unwrap();
    assert_eq!(check.env["EPR_STORE"].as_deref(), f.store.root.to_str());
}
#[test]
fn test_guard_registry_initialization_failure_is_fail_open() {
    let t = tempfile::tempdir().unwrap();
    atomic_bytes(&t.path().join("registry"), b"invalid", 0o600).unwrap();
    assert_eq!(
        Registry::load(&t.path().join("registry")).unwrap_err().kind,
        "registry-invalid"
    );
    assert_eq!(children(t.path()).len(), 1);
}
#[test]
fn test_guard_expands_relative_install_paths_to_absolute() {
    let p = absolute(Path::new("relative/../bin")).unwrap();
    assert!(p.is_absolute());
    assert_eq!(p, std::env::current_dir().unwrap().join("bin"));
}
#[test]
fn test_guard_restores_previous_link_when_installed_verification_fails() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    fs::create_dir_all(f.path("cargo-bin")).unwrap();
    symlink("/old/epr", f.path("cargo-bin/epr")).unwrap();
    f.state.lock().unwrap().install_fail = true;
    assert_eq!(
        f.install().unwrap_err().kind,
        "launcher-verification-failed"
    );
    assert_eq!(
        fs::read_link(f.path("cargo-bin/epr")).unwrap(),
        Path::new("/old/epr")
    );
}
#[test]
fn test_guard_leaves_existing_path_entry_untouched_when_bootstrap_refuses() {
    let f = Fixture::new("epr");
    atomic_bytes(&f.path("cargo-bin/epr"), b"sentinel", 0o755).unwrap();
    assert!(f.install().is_err());
    assert_eq!(fs::read(f.path("cargo-bin/epr")).unwrap(), b"sentinel");
}
#[test]
fn test_cleanliness_is_judged_over_epr_sources_not_hook_ledgers() {
    let f = Fixture::new("epr");
    f.state.lock().unwrap().metadata =
        Some(json!({"packages":[{"source":null,"manifest_path":f.path("src/Cargo.toml")}]}));
    assert_eq!(f.store.source_state(f.tmp.path()).unwrap()["clean"], true);
    let s = f.state.lock().unwrap();
    let c = s
        .calls
        .iter()
        .find(|c| c.args.contains(&"status".into()))
        .unwrap();
    assert!(c.args.contains(&"src".into()));
    assert!(!c.args.contains(&".".into()));
    drop(s);
    f.state.lock().unwrap().dirty = true;
    assert_eq!(f.store.source_state(f.tmp.path()).unwrap()["clean"], false);
}
#[test]
fn test_source_paths_fall_back_to_the_whole_tree_without_a_cargo_answer() {
    let f = Fixture::new("epr");
    assert_eq!(f.store.source_paths(f.tmp.path()), ["."]);
}
#[test]
fn test_shadows_lists_every_epr_reached_before_the_launcher() {
    let f = Fixture::new("epr");
    atomic_bytes(&f.path("bin/bin-launch"), b"launcher", 0o755).unwrap();
    for name in ["stale", "foreign", "ours", "after"] {
        fs::create_dir_all(f.path(name)).unwrap();
    }
    symlink("/gone/epr", f.path("stale/epr")).unwrap();
    atomic_bytes(&f.path("foreign/epr"), b"foreign", 0o755).unwrap();
    symlink(f.path("bin/bin-launch"), f.path("ours/epr")).unwrap();
    atomic_bytes(&f.path("after/epr"), b"later", 0o755).unwrap();
    let path = ["stale", "foreign", "ours", "after"]
        .map(|s| f.path(s).display().to_string())
        .join(":");
    let rows = f.store.shadows(&path);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["superseded"], true);
    assert_eq!(rows[1]["superseded"], false);
}
#[test]
fn test_guard_keeps_a_hand_installed_binary_and_repoints_stale_shadows() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(&f.path("cargo-bin/epr"), b"hand-built", 0o755).unwrap();
    let cid = file_cid(&f.path("cargo-bin/epr")).unwrap();
    let result = f.install().unwrap();
    assert_eq!(
        result["adopted"]["epr"],
        f.store.root.join(&cid).join("epr").display().to_string()
    );
    let r = f.store.record(&cid).unwrap();
    assert!(!r.provenance.tree_clean);
    assert_eq!(r.provenance.gate_run.as_deref(), Some("adopted-from-path"));
}
#[test]
fn test_guard_preserves_distinct_plain_alias_bytes_as_separate_candidates() {
    let f = Fixture::new("brit");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(&f.path("cargo-bin/brit"), b"plain brit", 0o755).unwrap();
    atomic_bytes(&f.path("cargo-bin/rakia"), b"plain alias", 0o755).unwrap();
    let result = f.install().unwrap();
    let a = Path::new(result["adopted"]["brit"].as_str().unwrap());
    let b = Path::new(result["adopted"]["rakia"].as_str().unwrap());
    assert_ne!(a.parent(), b.parent());
    assert_eq!(fs::read(a).unwrap(), b"plain brit");
    assert_eq!(fs::read(b).unwrap(), b"plain alias");
}
#[test]
fn test_follow_adopts_a_newer_published_dev_commit_the_checkout_holds() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    f.state.lock().unwrap().head = "b".repeat(40);
    let b = f.release("b");
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap()["followed"],
        true
    );
    assert_eq!(f.store.head("current"), Some(b.cid));
}
#[test]
fn test_follow_leaves_the_head_for_an_unpublished_or_unchanged_checkout() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap()["followed"],
        false
    );
    {
        let mut s = f.state.lock().unwrap();
        s.head = "b".repeat(40);
        s.published = false;
    }
    assert!(f.store.follow(f.tmp.path(), None).unwrap()["why"]
        .as_str()
        .unwrap()
        .contains("not published"));
    {
        let mut s = f.state.lock().unwrap();
        s.published = true;
        s.dirty = true;
    }
    assert!(f.store.follow(f.tmp.path(), None).unwrap()["why"]
        .as_str()
        .unwrap()
        .contains("dirty"));
    assert_eq!(f.store.head("current"), Some(a.cid));
}
#[test]
fn test_follow_does_nothing_when_epr_sources_did_not_change() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.land(&a).unwrap();
    {
        let mut s = f.state.lock().unwrap();
        s.head = "b".repeat(40);
        s.unchanged = true;
    }
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap()["why"],
        "no epr source change since the landed head"
    );
}
#[test]
fn test_brit_multibin_ingest_uses_canonical_sums_rcid() {
    let f = Fixture::new("brit");
    let r = f.release("a");
    assert!(r.cid.starts_with("bafyrei"));
    assert_eq!(r.cid, r.rcid);
    assert_eq!(r.bins.len(), 4);
    assert_eq!(r.bins["rakia"], r.bins["brit"]);
    assert!(f.store.verified(&r.cid));
    let bytes = fs::read(f.store.root.join(&r.cid).join("sums.cbor")).unwrap();
    assert_eq!(bytes[0], 0xa4);
    assert_eq!(&bytes[1..6], b"\x64brit");
    for name in r.bins.keys() {
        assert_eq!(
            f.store
                .root
                .join(&r.cid)
                .join(name)
                .metadata()
                .unwrap()
                .permissions()
                .mode()
                & 0o222,
            0
        );
    }
}
#[test]
fn test_rakia_alias_execs_brit_with_rakia_as_argv_zero() {
    let f = Fixture::new("brit");
    assert_eq!(
        f.registry.invoked(Path::new("/bin/rakia")).unwrap(),
        ("brit".into(), "brit".into())
    );
    // Actual argv[0] and non-UTF8 execution are asserted by tests/launcher.rs.
    let r = f.release("alias");
    f.head(&r);
    assert!(f
        .store
        .resolve(Some("brit"), None, &|_| None, true)
        .unwrap()
        .path
        .ends_with("brit"));
}
#[test]
fn test_session_pins_are_scoped_per_tool() {
    let a = Fixture::new("epr");
    let b = Fixture::new("brit");
    let ar = a.release("a");
    let br = b.release("b");
    a.head(&ar);
    b.head(&br);
    let i = identity("shared");
    assert_eq!(a.store.pin(&i, None, false, None).unwrap().cid, ar.cid);
    assert_eq!(b.store.pin(&i, None, false, None).unwrap().cid, br.cid);
}
fn brit_follow() -> Fixture {
    let f = Fixture::new("brit");
    let a = f.release("old");
    f.head(&a);
    f.state.lock().unwrap().head = "b".repeat(40);
    f
}
#[test]
fn test_brit_follow_refuses_unpublished_head() {
    let f = brit_follow();
    f.state.lock().unwrap().published = false;
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap_err().kind,
        "unpublished"
    );
    assert_eq!(f.state.lock().unwrap().judge_calls, 0);
}
#[test]
fn test_direct_brit_land_requires_witnessed_success_before_health() {
    let f = Fixture::new("brit");
    let r = f.release("a");
    f.state.lock().unwrap().verdict =
        Some(json!({"outcome":"pass","tier":"claimed","conclusion":"unreachable"}));
    assert_eq!(f.land(&r).unwrap_err().kind, "attestation-unreachable");
    assert!(f.store.head("current").is_none());
}
#[test]
fn test_brit_follow_refuses_unreachable_or_red_attestation_for_every_candidate_source() {
    for cached in [false, true] {
        for conclusion in ["unreachable", "failure"] {
            let f = brit_follow();
            let r = f.release("new");
            if !cached {
                fs::remove_dir_all(f.store.root.join(&r.cid)).unwrap();
            }
            f.state.lock().unwrap().verdict = Some(
                json!({"outcome":"pass","tier":if conclusion=="unreachable"{"claimed"}else{"witnessed"},"conclusion":conclusion}),
            );
            let e = f
                .store
                .follow(f.tmp.path(), Some(&f.path("new")))
                .unwrap_err();
            assert_eq!(
                e.kind,
                if conclusion == "unreachable" {
                    "attestation-unreachable"
                } else {
                    "attestation-refused"
                }
            );
            assert_ne!(f.store.head("current"), Some(r.cid));
        }
    }
}
#[test]
fn test_brit_follow_refuses_a_pin_that_does_not_descend() {
    let f = brit_follow();
    f.state.lock().unwrap().descended = false;
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap_err().kind,
        "non-descendant"
    );
    assert_eq!(f.state.lock().unwrap().judge_calls, 0);
}
#[test]
fn test_brit_follow_adopts_verified_fake_release() {
    let f = brit_follow();
    let r = f.release("new");
    assert_eq!(
        f.store.follow(f.tmp.path(), None).unwrap()["followed"],
        true
    );
    assert_eq!(f.store.head("current"), Some(r.cid));
    assert_eq!(f.state.lock().unwrap().judge_calls, 1);
}
#[test]
fn test_brit_follow_names_both_cures_when_no_bytes_source_exists() {
    let f = brit_follow();
    let e = f.store.follow(f.tmp.path(), None).unwrap_err();
    assert_eq!(e.kind, "bootstrap-unavailable");
    assert!(e.detail.contains("--closure"));
    assert!(e.detail.contains("status"));
    assert!(
        e.detail.contains("bootstrap-pool-unavailable")
            || e.detail.contains("git fetch")
            || e.detail.contains("offline")
    );
}
#[test]
fn test_brit_headless_bootstrap_requires_attestation_and_uses_the_same_release_source() {
    let f = Fixture::new("brit");
    let r = f.release("a");
    let result = f.store.bootstrap(f.tmp.path(), None).unwrap();
    assert_eq!(result.cid, r.cid);
    assert_eq!(f.state.lock().unwrap().judge_calls, 1);
    assert!(result
        .attestations
        .values()
        .any(|v| v["tier"] == "witnessed"));
}
#[test]
fn test_the_brit_registry_credential_prefers_the_cargo_token_then_npm_token() {
    assert!(source::registry_credential(&|_| None).is_empty());
    let e = source::registry_credential(&|k| (k == "NPM_TOKEN").then(|| " t0k ".into()));
    assert_eq!(
        e["CARGO_REGISTRIES_ELOHIM_TOKEN"].as_deref(),
        Some("Bearer t0k")
    );
    let e = source::registry_credential(&|k| match k {
        "NPM_TOKEN" => Some("npm".into()),
        "CARGO_REGISTRIES_ELOHIM_TOKEN" => Some("Bearer explicit".into()),
        _ => None,
    });
    assert_eq!(
        e["CARGO_REGISTRIES_ELOHIM_TOKEN"].as_deref(),
        Some("Bearer explicit")
    );
}
#[test]
fn test_a_daemon_follows_the_device_head_not_the_launching_session() {
    let f = Fixture::new("epr");
    let a = f.release("a");
    f.head(&a);
    f.store.pin(&identity("s"), None, false, None).unwrap();
    let b = f.release("b");
    f.head(&b);
    assert!(f.store.long_running(&["peer".into(), "serve".into()]));
    assert!(!f.store.long_running(&["peer".into(), "status".into()]));
    assert_eq!(
        f.store.resolve(None, None, &|_| None, true).unwrap().cid,
        Some(b.cid)
    );
    assert_eq!(
        f.store
            .resolve(None, Some(&identity("s")), &|_| None, true)
            .unwrap()
            .cid,
        Some(a.cid)
    );
}
#[test]
fn test_a_daemon_lease_is_owned_by_the_daemon_process() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    proc_entry(&f.path("proc"), 4242, 1, "epr", 777);
    let p = f
        .store
        .daemon_lease(&r.cid, 4242, &f.path("proc"))
        .unwrap()
        .unwrap();
    assert_eq!(p.owner_pid, Some(4242));
    assert_eq!(p.owner_start_time.as_deref(), Some("777"));
    assert!(f
        .store
        .daemon_lease(&r.cid, 9999, &f.path("proc"))
        .unwrap()
        .is_none());
}

#[test]
fn witnessed_attestation_is_cached_and_provisional_never_lands() {
    let f = Fixture::new("brit");
    let r = f.release("a");
    f.land(&r).unwrap();
    f.state.lock().unwrap().verdict = Some(json!({"conclusion":"unreachable"}));
    f.land(&r).unwrap();
    assert_eq!(f.state.lock().unwrap().judge_calls, 1);
    let b = f.release("b");
    let pin = f
        .store
        .pin(
            &identity("provisional"),
            Some(&b.cid),
            true,
            Some(f.tmp.path()),
        )
        .unwrap();
    assert!(pin.provisional);
    assert_eq!(f.store.head("current"), Some(r.cid));
    assert_eq!(f.land(&b).unwrap_err().kind, "attestation-unreachable");
}
#[test]
fn wrong_sum_and_old_cid_readers_refuse() {
    let f = Fixture::new("brit");
    f.release("a");
    fs::write(f.path("a/brit"), b"changed").unwrap();
    let e = f
        .store
        .ingest(
            &f.path("a"),
            &IngestOptions {
                source_commit: Some(f.commit()),
                tree_clean: Some(true),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(e.kind, "checksums-mismatch");
    assert!(require_cid(&format!("sha256:{}", "a".repeat(64))).is_err());
}
#[test]
fn migration_is_idempotent_and_preserves_provenance_and_pins() {
    let f = Fixture::new("epr");
    let bytes = b"#!/bin/sh\nexit 0\n";
    let raw = eprfs_core::BlobCid::compute_raw(bytes);
    let old = format!(
        "sha256:{}",
        raw.as_cid()
            .hash()
            .digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let dir = f.store.root.join(&old);
    atomic_bytes(&dir.join("epr"), bytes, 0o555).unwrap();
    let provenance = json!({"cid":old,"source_commit":"old-commit","tree_clean":true,"gate_run":"old-gate","ingested_at":"2026-01-01T00:00:00Z","custom":{"proof":"keep"}});
    atomic_json(&dir.join("record.json"), &provenance).unwrap();
    atomic_link(&f.store.root.join("current"), Path::new(&old)).unwrap();
    atomic_link(&f.store.root.join("previous"), Path::new(&old)).unwrap();
    atomic_json(
        &f.store.root.join("current.json"),
        &json!({"cid":old,"commit":"old-commit","previous":old}),
    )
    .unwrap();
    atomic_json(&f.store.root.join("sessions/s"),&json!({"cid":old,"session":"s","pinned_at":"2026-10-23T00:00:00Z","owner_pid":5,"owner_starttime":"123"})).unwrap();
    // An already-native release may have newer custody evidence than the old record.
    let native = f.store.root.join(raw.to_string());
    fs::create_dir_all(&native).unwrap();
    touch(&native.join(".used")).unwrap();
    let recent = fs::metadata(native.join(".used"))
        .unwrap()
        .modified()
        .unwrap();
    touch(&dir.join(".used")).unwrap();
    fs::File::open(dir.join(".used"))
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    let first = f.store.migrate().unwrap();
    assert_eq!(
        fs::metadata(native.join(".used"))
            .unwrap()
            .modified()
            .unwrap(),
        recent
    );
    assert_eq!(f.store.migrate().unwrap(), first);
    let cid = raw.to_string();
    assert_eq!(f.store.head("current"), Some(cid.clone()));
    assert_eq!(f.store.head("previous"), Some(cid.clone()));
    assert!(f.store.verified(&cid));
    let r = f.store.record(&cid).unwrap();
    assert_eq!(r.provenance.gate_run.as_deref(), Some("old-gate"));
    assert_eq!(r.extra["legacyProvenance"], provenance);
    let p: Pin = read_json(&f.store.root.join("sessions/s")).unwrap();
    assert_eq!(p.cid, cid);
    assert_eq!(p.owner_start_time.as_deref(), Some("123"));
}
#[test]
fn no_session_gets_no_pin() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let i = crate::session::resolve_with(&|_| None, Path::new("/absent"), 1, &Default::default());
    assert_eq!(
        f.store
            .resolve(None, Some(&i), &|_| None, true)
            .unwrap()
            .source,
        "current"
    );
    assert!(!f.store.root.join("sessions").exists());
}
#[test]
fn status_schema_contract() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../sdk/schemas/v1/views/bin-status.schema.json"
    ))
    .unwrap();
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    let status = serde_json::to_value(
        f.store
            .status(&identity("s"), None, false, &|_| None)
            .unwrap(),
    )
    .unwrap();
    let object = status.as_object().unwrap();
    assert_eq!(
        object.len(),
        schema["properties"].as_object().unwrap().len()
    );
    for key in schema["required"].as_array().unwrap() {
        assert!(object.contains_key(key.as_str().unwrap()));
    }
    for (key, value) in object {
        let kind = if value.is_null() {
            "null"
        } else if value.is_string() {
            "string"
        } else if value.is_boolean() {
            "boolean"
        } else if value.is_array() {
            "array"
        } else {
            panic!("unexpected view type")
        };
        let declared = &schema["properties"][key]["type"];
        assert!(
            declared == kind
                || declared
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(kind))),
            "{key}"
        );
    }
    let release = &status["candidates"][0];
    for key in schema["definitions"]["Release"]["required"]
        .as_array()
        .unwrap()
    {
        assert!(release.get(key.as_str().unwrap()).is_some(), "{key}");
    }
    assert_eq!(release["cid"], r.cid);
    assert!(release.get("source_commit").is_none());
    assert_eq!(status["verifiedCurrent"], true);
}

#[test]
fn session_keys_cannot_escape_or_collide() {
    let f = Fixture::new("epr");
    for key in [".", "..", "../outside", "/absolute", "a/b", "a_b"] {
        let path = f.store.pin_path(key).unwrap();
        assert_eq!(path.parent(), Some(f.store.root.join("sessions").as_path()));
        assert!(!matches!(
            path.file_name().unwrap().to_str(),
            Some("." | "..")
        ));
    }
    assert_ne!(
        f.store.pin_path("a/b").unwrap(),
        f.store.pin_path("a_b").unwrap()
    );
}

#[test]
fn migration_renames_unsafe_and_long_session_keys() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    for (old_name, key) in [("unsafe", "x/y".to_owned()), ("long", "x".repeat(300))] {
        let old = f.store.root.join("sessions").join(old_name);
        atomic_json(
            &old,
            &json!({"cid": r.cid, "session": key, "pinned_at": "2026-10-23T00:00:00Z"}),
        )
        .unwrap();
        f.store.migrate().unwrap();
        assert!(!old.exists());
        let pin: Value = read_json(&f.store.pin_path(&key).unwrap()).unwrap();
        assert_eq!(pin["session"], key);
        assert_eq!(pin["cid"], r.cid);
        assert!(f.store.migrate().is_ok());
    }
}

#[test]
fn install_preserves_plain_persistent_bin_as_candidate() {
    let f = Fixture::new("epr");
    let r = f.release("a");
    f.head(&r);
    atomic_bytes(&f.path("bin/epr"), b"persistent plain", 0o755).unwrap();
    atomic_bytes(&f.path("cargo-bin/epr"), b"destination plain", 0o755).unwrap();
    let result = f.install().unwrap();
    let persistent = Path::new(result["persistentAdopted"]["epr"].as_str().unwrap());
    let destination = Path::new(result["adopted"]["epr"].as_str().unwrap());
    assert_ne!(persistent, destination);
    assert_eq!(fs::read(persistent).unwrap(), b"persistent plain");
    assert_eq!(fs::read(destination).unwrap(), b"destination plain");
}

#[test]
fn brit_build_tries_offline_then_anonymous_then_credentials() {
    for token in [None, Some("test-secret")] {
        let f = Fixture::new("brit");
        let mut attempts = Vec::new();
        let result =
            f.store
                .build_release(f.tmp.path(), &f.commit(), "test", &mut attempts, &|key| {
                    (key == "NPM_TOKEN")
                        .then(|| token.map(str::to_owned))
                        .flatten()
                });
        assert!(result.is_err());
        let state = f.state.lock().unwrap();
        let builds: Vec<_> = state
            .calls
            .iter()
            .filter(|c| c.program == "cargo" && c.args[0] == "build")
            .collect();
        assert_eq!(builds.len(), if token.is_some() { 3 } else { 2 });
        assert!(builds[0].args.iter().any(|a| a == "--offline"));
        assert!(!builds[1].args.iter().any(|a| a == "--offline"));
        for build in &builds[..2] {
            assert_eq!(build.env["CARGO_REGISTRIES_ELOHIM_TOKEN"], None);
            assert_eq!(build.env["NPM_TOKEN"], None);
        }
        if token.is_some() {
            assert_eq!(
                builds[2].env["CARGO_REGISTRIES_ELOHIM_TOKEN"].as_deref(),
                Some("Bearer test-secret")
            );
        }
        assert!(attempts.iter().all(|a| !a.contains("test-secret")));
    }
}

#[test]
fn explicit_offline_mode_never_attempts_network_build_or_download() {
    let f = Fixture::new("brit");
    let mut attempts = Vec::new();
    assert!(f
        .store
        .build_release(f.tmp.path(), &f.commit(), "test", &mut attempts, &|key| {
            (key == "CARGO_NET_OFFLINE").then(|| "true".into())
        })
        .is_err());
    let state = f.state.lock().unwrap();
    assert_eq!(
        state
            .calls
            .iter()
            .filter(|c| c.program == "cargo" && c.args[0] == "build")
            .count(),
        1
    );
    assert!(!state.calls.iter().any(|c| c.program == "gh"));
}
