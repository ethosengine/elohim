use super::*;

use std::collections::BTreeMap;

fn descriptor(inputs: &[(&str, &str)]) -> MethodDescriptor {
    MethodDescriptor {
        name: super::super::MESH_BUILD.into(),
        tools: BTreeMap::new(),
        flags: vec![
            "--bin=epr".into(),
            "--target=x86_64-unknown-linux-gnu".into(),
        ],
        params: None,
        exec: None,
        step: Some("mesh-build:eprfs".into()),
        embedder: None,
        view: Vec::new(),
        non_path_inputs: inputs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        deterministic: false,
    }
}

const DECLARED_V1: [(&str, &str); 4] = [
    ("target.isa", "x86-64"),
    ("target.cflags", "-march=x86-64"),
    ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
    ("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
];

#[test]
fn isa_c_accepts_only_a_declaration_the_record_applies() {
    assert!(matches!(declared(&descriptor(&[("CC", "gcc")])), Ok(None)));
    let realized = descriptor(&DECLARED_V1);
    let found = declared(&realized).unwrap().unwrap();
    assert_eq!((found.isa, found.march), ("x86-64", "-march=x86-64"));
    // The C compiler itself (CC) is chosen, not flagged: it may be recorded beside.
    let mut with_cc = DECLARED_V1.to_vec();
    with_cc.push(("CC", "gcc"));
    assert!(declared(&descriptor(&with_cc)).unwrap().is_some());
    for (change, why) in [
        (("target.cflags", "-O2"), "names no single -march"),
        (
            ("target.cflags", "-march=x86-64 -march=x86-64"),
            "names no single -march",
        ),
        (("target.isa", "x86-64-v2"), "not target.isa x86-64-v2"),
        (
            ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64 -O2"),
            "not -march=x86-64",
        ),
        (("CPPFLAGS", "-DX"), "C-family variable CPPFLAGS"),
        (("CC_x86_64_unknown_linux_gnu", "clang"), "CC_x86_64"),
        (("target.libc", "glibc"), "target.libc"),
    ] {
        let mut inputs = DECLARED_V1.to_vec();
        inputs.retain(|(name, _)| *name != change.0);
        inputs.push(change);
        let Err(missing) = declared(&descriptor(&inputs)) else {
            panic!("{change:?} was accepted");
        };
        assert!(
            missing.iter().any(|m| m.contains(why)),
            "{why}: {missing:?}"
        );
    }
}

/// A checkout whose HEAD holds `crate/Cargo.lock` and, when given, `policy` as
/// genesis/agentic/pool-policy.json.
fn checkout(policy: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    let git = |args: &[&str]| {
        let out = crate::git::native_command(root)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    };
    git(&["init", "-q"]);
    std::fs::create_dir_all(root.join("crate")).unwrap();
    std::fs::write(root.join("crate/Cargo.lock"), "version = 4\n").unwrap();
    if let Some(policy) = policy {
        std::fs::create_dir_all(root.join("genesis/agentic")).unwrap();
        std::fs::write(root.join("genesis/agentic/pool-policy.json"), policy).unwrap();
    }
    git(&["add", "."]);
    git(&["commit", "-qm", "fixture", "--no-verify"]);
    dir
}

fn policy(triple: &str, isa: &str, cflags: &str, workspace: &str) -> String {
    serde_json::json!({
        "declared_targets": {
            "_comment": "fixture",
            triple: {"isa": isa, "cflags": cflags, "workspaces": [workspace]},
        }
    })
    .to_string()
}

/// Ruling 3 (L2a rework): the record at HEAD, not the descriptor alone, declares the target, and
/// only for exactly the triple the method was built for.
#[test]
fn isa_c_takes_the_declared_target_from_the_record_at_head_for_the_exact_triple() {
    let gnu = "x86_64-unknown-linux-gnu";
    let with_triple = |triple: &str| {
        let mut d = descriptor(&[("crate", "crate")]);
        d.flags[1] = format!("--target={triple}");
        d
    };
    let target = Declared {
        isa: "x86-64",
        march: "-march=x86-64",
    };
    let held = checkout(Some(&policy(gnu, "x86-64", "-march=x86-64", "crate")));
    let repo = held.path().canonicalize().unwrap();
    assert_eq!(
        policy_declares(&repo, &with_triple(gnu), &target),
        Vec::<String>::new()
    );
    // A same-architecture neighbour the record does not name is undeclared.
    for triple in ["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnux32"] {
        let found = policy_declares(&repo, &with_triple(triple), &target);
        assert!(
            found
                .iter()
                .any(|m| m.contains(&format!("declares no target {triple:?}"))),
            "{triple}: {found:?}"
        );
    }
    // A record whose values differ from the descriptor's, or that names another workspace.
    for (record, why) in [
        (
            policy(gnu, "x86-64-v2", "-march=x86-64", "crate"),
            "isa \"x86-64-v2\"",
        ),
        (
            policy(gnu, "x86-64", "-march=x86-64-v3", "crate"),
            "cflags \"-march=x86-64-v3\"",
        ),
        (
            policy(gnu, "x86-64", "-march=x86-64", "elohim/eprfs"),
            "not the bin's workspace crate",
        ),
    ] {
        let held = checkout(Some(&record));
        let repo = held.path().canonicalize().unwrap();
        let found = policy_declares(&repo, &with_triple(gnu), &target);
        assert!(found.iter().any(|m| m.contains(why)), "{why}: {found:?}");
    }
    // No record at HEAD (a working-tree copy is not HEAD's).
    let bare = checkout(None);
    let repo = bare.path().canonicalize().unwrap();
    std::fs::create_dir_all(repo.join("genesis/agentic")).unwrap();
    std::fs::write(
        repo.join("genesis/agentic/pool-policy.json"),
        policy(gnu, "x86-64", "-march=x86-64", "crate"),
    )
    .unwrap();
    let found = policy_declares(&repo, &with_triple(gnu), &target);
    assert!(
        found.iter().any(|m| m.contains("does not read at HEAD")),
        "{found:?}"
    );
}

/// Ruling 4 (L2a rework): a diagnostic-only switch the gate declares (cc's command log) is
/// recorded and allowed under a declared target; every other C-tool input still refuses, the
/// archiver and its flags included, and so does a switch that changes what cc builds.
#[test]
fn isa_c_allows_the_declared_diagnostic_switch_and_refuses_the_other_c_tools() {
    let mut logged = DECLARED_V1.to_vec();
    logged.push(("CC_ENABLE_DEBUG_OUTPUT", "1"));
    assert!(
        declared(&descriptor(&logged)).is_ok_and(|d| d.is_some()),
        "{:?}",
        declared(&descriptor(&logged)).err()
    );
    for (name, value) in [
        ("HOST_AR", "llvm-ar"),
        ("TARGET_AR", "llvm-ar"),
        ("AR_x86_64_unknown_linux_gnu", "llvm-ar"),
        ("AR_x86_64-unknown-linux-gnu", "llvm-ar"),
        ("ARFLAGS", "-X"),
        ("RANLIBFLAGS", "-D"),
        ("CC_FORCE_DISABLE", "1"),
    ] {
        let mut inputs = logged.clone();
        inputs.push((name, value));
        let Err(missing) = declared(&descriptor(&inputs)) else {
            panic!("{name} was accepted");
        };
        assert!(
            missing
                .iter()
                .any(|m| m.contains(&format!("C-family variable {name};"))),
            "{name}: {missing:?}"
        );
    }
}

/// Ruling 2 (L2a rework): `target.cflags` is exactly `-march=<target.isa>`. A declaration carrying
/// an extra flag refuses even when every recorded variable agrees with it, so a record and a
/// descriptor cannot declare `-mavx2` together.
#[test]
fn isa_c_refuses_declared_cflags_beyond_the_march_even_when_the_record_agrees() {
    for extra in [
        "-march=x86-64 -mavx2",
        "-mavx2 -march=x86-64",
        "-march=x86-64 -O2",
        " -march=x86-64",
    ] {
        let inputs = [
            ("target.isa", "x86-64"),
            ("target.cflags", extra),
            ("CFLAGS_x86_64_unknown_linux_gnu", extra),
            ("CXXFLAGS_x86_64_unknown_linux_gnu", extra),
        ];
        let Err(missing) = declared(&descriptor(&inputs)) else {
            panic!("{extra:?} declared and recorded alike was accepted");
        };
        assert!(
            missing
                .iter()
                .any(|m| m.contains("is not exactly -march=x86-64")),
            "{extra:?}: {missing:?}"
        );
    }
}
