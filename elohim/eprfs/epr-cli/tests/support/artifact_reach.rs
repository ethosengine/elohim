//! K12 packet 3, the writer: every custody Produce `put` records carries exactly one
//! `env:reach=<level>` slot derived from its inputs; nothing a caller passes names the level; a
//! `private` record does not cross; `reach` is no longer a declared gap.
use super::*;

/// One recordable mesh build on one device, keyed and ready to put.
struct MeshPut {
    _keys: TempDir,
    a: Device,
    _scratch: TempDir,
    method: PathBuf,
    descriptor: MethodDescriptor,
    bytes: PathBuf,
    stub: PathBuf,
}

impl MeshPut {
    /// The fixture of `put_and_mount`, with `edit` applied to the descriptor before it is keyed.
    fn new(edit: impl FnOnce(&mut MethodDescriptor)) -> MeshPut {
        let keys = TempDir::new().unwrap();
        let a = Device::new(&keys, "a");
        let scratch = TempDir::new().unwrap();
        let (method, mut descriptor) = method_file(scratch.path(), &["crate/src", "README.md"]);
        let bytes = scratch.path().join("elohim-storage");
        std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
        recordable_mesh_fixture(a.root(), scratch.path(), &method, &mut descriptor);
        edit(&mut descriptor);
        std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        let (stub, _log) = brit_stub(scratch.path());
        MeshPut {
            _keys: keys,
            a,
            _scratch: scratch,
            method,
            descriptor,
            bytes,
            stub,
        }
    }

    /// The device's command under the descriptor's recorded build environment.
    fn command(&self) -> Command {
        let mut command = self.a.command();
        command.envs(
            self.descriptor
                .non_path_inputs
                .iter()
                .filter(|(name, _)| name.as_str() != "crate"),
        );
        command
    }

    fn input(&self) -> String {
        let out = self
            .command()
            .args(["artifact", "key", "--method"])
            .arg(&self.method)
            .args([
                "--repo",
                self.a.root().to_str().unwrap(),
                "--print",
                "input",
            ])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    fn put_with(&self, extra: &[&str], env: &[(&str, &str)]) -> Output {
        let input = self.input();
        let mut command = self.command();
        command
            .env("BRIT_BUILD_REF_BIN", &self.stub)
            .envs(env.iter().copied())
            .args(["artifact", "put", "--method"])
            .arg(&self.method)
            .args(["--input", &input, "--bytes"])
            .arg(&self.bytes)
            .args([
                "--project",
                "elohim-storage",
                "--repo",
                self.a.root().to_str().unwrap(),
                "--json",
            ])
            .args(extra);
        command.output().unwrap()
    }

    fn produces(&self) -> Vec<FlowEvent> {
        events(self.a.root())
            .into_iter()
            .filter(|e| e.action == ReaVerb::Produce)
            .collect()
    }
}

/// Every slot whose key reads as a reach slot in any case, as the gate counts them.
fn reach_slots(event: &FlowEvent) -> Vec<&str> {
    event
        .classified_as
        .iter()
        .filter(|slot| {
            slot.get(.."env:reach".len())
                .is_some_and(|key| key.eq_ignore_ascii_case("env:reach"))
        })
        .map(String::as_str)
        .collect()
}

fn recorded(out: &Output) -> serde_json::Value {
    assert!(out.status.success(), "{}", text(out));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn repo_allow_list() -> CrossingAllowList {
    let policy =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../genesis/agentic/pool-policy.json");
    CrossingAllowList::from_policy_file(&policy).unwrap()
}

#[test]
fn a_mesh_build_put_records_reach_self() {
    let f = MeshPut::new(|_| {});
    let out = f.put_with(&["--cross"], &[]);
    let json = recorded(&out);
    assert_eq!(json["reach"], "self");
    assert_eq!(json["crosses"], true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("reach: self\n"), "{stderr}");
    assert!(!stderr.contains("reach: private"), "{stderr}");
    let produces = f.produces();
    assert_eq!(produces.len(), 1);
    assert_eq!(reach_slots(&produces[0]), ["env:reach=self"]);
    // A self record crosses as before.
    assert!(produces[0]
        .classified_as
        .contains(&"measure:artifact-produce@1".to_string()));
    assert!(repo_allow_list().admits(&produces[0]));
    // The gate reads exactly this slot as the person's own devices.
    assert_eq!(
        eprfs_peer::event_reach(&produces[0]),
        Some(eprfs_peer::DeclaredReach::OwnDevices)
    );
}

#[test]
fn a_sysroot_put_records_reach_self() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "floor");
    let scratch = TempDir::new().unwrap();
    let input = scratch.path().join("input");
    std::fs::create_dir(&input).unwrap();
    std::fs::write(input.join("header.h"), b"/* content-pinned header */").unwrap();
    let archive = scratch.path().join("sysroot.eprsys");
    let packed = a.epr(&[
        "artifact",
        "sysroot",
        "pack",
        "--from",
        input.to_str().unwrap(),
        "--triple",
        "x86_64-unknown-linux-gnu",
        "--out",
        archive.to_str().unwrap(),
    ]);
    assert!(packed.status.success(), "{}", text(&packed));
    let (stub, _log) = brit_stub(scratch.path());
    let put = a
        .command()
        .env("BRIT_BUILD_REF_BIN", &stub)
        .args([
            "artifact",
            "sysroot",
            "put",
            "--archive",
            archive.to_str().unwrap(),
            "--repo",
            a.root().to_str().unwrap(),
            "--cross",
            "--json",
        ])
        .output()
        .unwrap();
    let json = recorded(&put);
    assert_eq!(json["reach"], "self");
    assert_eq!(json["crosses"], true);
    assert!(String::from_utf8_lossy(&put.stderr).contains("reach: self\n"));
    let produces: Vec<_> = events(a.root())
        .into_iter()
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(produces.len(), 1);
    assert_eq!(reach_slots(&produces[0]), ["env:reach=self"]);
    assert!(produces[0]
        .classified_as
        .contains(&"basis-key:sysroot-manifest".to_string()));
}

#[test]
fn put_takes_no_reach_from_arguments_or_environment() {
    let f = MeshPut::new(|_| {});
    // No argument names a level: each spelling is refused before anything is recorded.
    for flag in ["--reach", "--env-reach", "--set-reach"] {
        for level in ["private", "public", "commons"] {
            let out = f.put_with(&[flag, level], &[]);
            assert!(!out.status.success(), "{flag} {level}: {}", text(&out));
            assert!(
                text(&out).contains("unknown artifact argument"),
                "{flag}: {}",
                text(&out)
            );
        }
    }
    assert!(f.produces().is_empty(), "a refused put records nothing");
    // Neither does the environment, nor a stray `--env` pair (a describe flag put never reads).
    let out = f.put_with(
        &["--env", "env:reach=commons", "--env", "reach=public"],
        &[
            ("EPR_REACH", "public"),
            ("REACH", "commons"),
            ("ELOHIM_REACH", "public"),
            ("EPR_ARTIFACT_REACH", "private"),
        ],
    );
    assert_eq!(recorded(&out)["reach"], "self");
    let produces = f.produces();
    assert_eq!(produces.len(), 1);
    assert_eq!(reach_slots(&produces[0]), ["env:reach=self"]);
}

#[test]
fn a_descriptor_env_named_reach_does_not_change_the_slot() {
    let f = MeshPut::new(|descriptor| {
        for (name, value) in [
            ("reach", "public"),
            ("REACH", "commons"),
            ("env:reach", "private"),
        ] {
            descriptor.non_path_inputs.insert(name.into(), value.into());
        }
    });
    let out = f.put_with(&[], &[]);
    // A declared build-environment value is an input of class `self`, whatever it is named.
    assert_eq!(recorded(&out)["reach"], "self");
    let produces = f.produces();
    assert_eq!(produces.len(), 1);
    assert_eq!(reach_slots(&produces[0]), ["env:reach=self"]);
}

#[test]
fn put_writes_exactly_one_reach_slot() {
    let f = MeshPut::new(|_| {});
    for _ in 0..2 {
        let out = f.put_with(&["--cross"], &[]);
        assert_eq!(recorded(&out)["reach"], "self");
    }
    // The second put is the same record: no second event, so no second slot anywhere.
    let produces = f.produces();
    assert_eq!(produces.len(), 1);
    assert_eq!(reach_slots(&produces[0]), ["env:reach=self"]);
    assert_eq!(
        produces[0]
            .classified_as
            .iter()
            .filter(|slot| slot.to_ascii_lowercase().contains("reach"))
            .count(),
        1,
        "{:?}",
        produces[0].classified_as
    );
}

#[test]
fn a_private_put_does_not_cross_even_with_cross() {
    // A method field with no class makes the record private (§2.6 item 3).
    let f = MeshPut::new(|descriptor| descriptor.embedder = Some("fixture-embedder".into()));
    let out = f.put_with(&["--cross"], &[]);
    let json = recorded(&out);
    assert_eq!(json["reach"], "private");
    assert_eq!(json["crosses"], false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("reach: private\n"), "{stderr}");
    assert!(
        stderr.contains("reach: private because of the method's embedder\n"),
        "{stderr}"
    );
    // Loud, on its own line, so a classifier gap is visible here and not only as "no holder".
    assert!(
        stderr.lines().any(|line| line
            == "reach: private records do not cross: --cross was given, and this Produce stays \
                on this device"),
        "{stderr}"
    );
    let produces = f.produces();
    assert_eq!(produces.len(), 1);
    let produce = &produces[0];
    assert_eq!(reach_slots(produce), ["env:reach=private"]);
    for crossing in ["measure:artifact-produce@1", "reason:produce"] {
        assert!(
            !produce.classified_as.contains(&crossing.to_string()),
            "{crossing} in {:?}",
            produce.classified_as
        );
    }
    assert!(
        !repo_allow_list().admits(produce),
        "a private Produce never crosses"
    );
    // It is still this device's custody record, which the gate refuses as private.
    assert!(produce.classified_as.contains(&"copy:self".to_string()));
    assert_eq!(
        eprfs_peer::event_reach(produce),
        Some(eprfs_peer::DeclaredReach::Private)
    );
}

#[test]
fn put_no_longer_declares_the_reach_gap() {
    let f = MeshPut::new(|_| {});
    let out = f.put_with(&[], &[]);
    recorded(&out);
    assert!(
        !text(&out).contains("reach: pending"),
        "the pre-K12 banner is gone: {}",
        text(&out)
    );
    let produces = f.produces();
    let gaps: Vec<_> = produces[0]
        .classified_as
        .iter()
        .filter_map(|slot| slot.strip_prefix("env:gaps="))
        .collect();
    assert_eq!(gaps.len(), 1, "{:?}", produces[0].classified_as);
    assert!(
        gaps[0].split(',').all(|gap| gap != "reach"),
        "reach is derived, not a gap: {}",
        gaps[0]
    );
    assert!(gaps[0]
        .split(',')
        .any(|gap| gap == "build-inputs.compiler-inputs-unattested"));
}

#[test]
fn a_private_put_warns_unless_the_serving_daemon_is_confirmed_to_gate_reach() {
    // T16 residual: a pre-K12 daemon ignores the slot. The daemon exposes no gate version, but its
    // head table records the digest of the rules it loaded, which covers the gate semantics.
    const WARNING: &str = "reach: warning: cannot confirm the serving daemon reads reach slots";
    let f = MeshPut::new(|descriptor| descriptor.embedder = Some("fixture-embedder".into()));
    let peer_dir = f.a.config.path().join("epr").join("peer");
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    std::fs::create_dir_all(f.a.payload()).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(f.a.payload(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let ours = std::fs::canonicalize(f.a.payload()).unwrap();
    std::fs::write(
        peer_dir.join(elohim_epr_cli::peer::PAYLOAD_STORE_RECORD),
        format!("{}\n", ours.display()),
    )
    .unwrap();
    // A serving daemon whose head table records no loaded rules (one that predates recording
    // them) is no evidence of a gate: warn.
    eprfs_peer::HeadTable::load(&peer_dir)
        .unwrap()
        .save(&peer_dir)
        .unwrap();
    assert!(eprfs_peer::HeadTable::load(&peer_dir)
        .unwrap()
        .rules
        .is_none());
    let out = f.put_with(&[], &[]);
    assert_eq!(recorded(&out)["reach"], "private");
    assert!(text(&out).contains(WARNING), "{}", text(&out));
    assert!(
        text(&out).contains("its head table records no loaded rules"),
        "{}",
        text(&out)
    );
    let policy = f.a.root().join("pool-policy.json");
    std::fs::write(
        &policy,
        r#"{"berth":{"artifacts":{"serves":{"mesh-build":["steward"]}}}}"#,
    )
    .unwrap();
    let digest_of = |text: &str| {
        let other = f.a.root().join("other-policy.json");
        std::fs::write(&other, text).unwrap();
        eprfs_peer::Rules::from_policy_file(&other)
            .unwrap()
            .digest()
    };
    let record = |digest: eprfs_peer::RulesDigest| {
        let mut table = eprfs_peer::HeadTable::load(&peer_dir).unwrap();
        table.rules = Some(eprfs_peer::RulesState {
            digest,
            policy: policy.clone(),
            loaded_at: 0,
            pending: None,
        });
        table.save(&peer_dir).unwrap();
    };
    // A daemon whose loaded digest differs from this binary's digest of the same policy.
    record(digest_of(r#"{"berth":{"artifacts":{"serves":{}}}}"#));
    let out = f.put_with(&[], &[]);
    assert_eq!(recorded(&out)["reach"], "private");
    assert!(text(&out).contains(WARNING), "{}", text(&out));
    // One that loaded exactly this binary's digest of it is confirmed: no warning.
    record(
        eprfs_peer::Rules::from_policy_file(&policy)
            .unwrap()
            .digest(),
    );
    let out = f.put_with(&[], &[]);
    assert_eq!(recorded(&out)["reach"], "private");
    assert!(!text(&out).contains(WARNING), "{}", text(&out));
}
