use eprfs_core::BlobCid;
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

struct Reader {
    dir: tempfile::TempDir,
    output: String,
}

impl Reader {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("binary"), b"binary").unwrap();
        fs::write(
            dir.path().join("epr"),
            r##"#!/bin/sh
set -eu
[ "$1 $2 $3 $5 $7" = 'artifact requires --output --write --json' ]
base="$(dirname "$0")"
case "${MODE:-ok}" in
  missing) echo "error: unrecognized subcommand 'requires'" >&2; exit 2 ;;
  error) echo 'reader failed' >&2; exit 1 ;;
  ok) cp "$base/requires" "$6" ;;
esac
sed "s|WRITE_PATH|$6|g" "$base/reply"
case "${MODE:-ok}" in
  unknown) exit 2 ;;
  unavailable|gap) exit 3 ;;
  conflict) exit 4 ;;
esac
"##,
        )
        .unwrap();
        fs::set_permissions(dir.path().join("epr"), fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            dir,
            output: BlobCid::compute_bytes(b"binary").to_string(),
        }
    }
    fn record(&self, clauses: &[Value], own: bool) {
        fs::write(
            self.dir.path().join("requires"),
            serde_json::to_vec(clauses).unwrap(),
        )
        .unwrap();
        fs::write(self.dir.path().join("reply"), json!({"known":true, "sources":[], "output":self.output, "requires":BlobCid::compute_bytes(&serde_json::to_vec(clauses).unwrap()).to_string(), "self":own, "produce":BlobCid::compute_bytes(b"Produce").to_string(), "path":"WRITE_PATH"}).to_string()).unwrap();
    }
    fn run(&self, mode: &str) -> (i32, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_ark"))
            .env("EPR_BIN", self.dir.path().join("epr"))
            .env("MODE", mode)
            .args(["host", "admit", "--output", &self.output, "--binary"])
            .arg(self.dir.path().join("binary"))
            .args(["--name", "fixture"])
            .output()
            .unwrap();
        assert!(output.stderr.is_empty());
        let line = String::from_utf8(output.stdout).unwrap();
        assert_eq!(line.lines().count(), 1);
        (output.status.code().unwrap(), line)
    }
}

#[test]
fn recorded_output_requires_verified_bytes_before_reader_lookup() {
    let reader = Reader::new();
    reader.record(&floor(), false);
    assert_eq!(
        reader.run("ok"),
        (0, "ok compatibility fixture: admitted\n".into())
    );
    fs::write(reader.dir.path().join("binary"), b"wrong bytes").unwrap();
    let (exit, line) = reader.run("gap");
    assert_eq!(exit, 1);
    assert!(line.contains("recorded CID does not match bytes"));
}

#[test]
#[cfg(target_arch = "x86_64")]
fn reader_alone_supplies_own_build_eligibility() {
    let reader = Reader::new();
    let mut clauses = floor();
    clauses.push(json!({"axis":"isa.x86", "predicate":{"kind":"subset", "value":["hostcompat_impossible_feature"]}, "source":"extracted"}));
    reader.record(&clauses, true);
    assert_eq!(reader.run("ok"), (0, "ok compatibility fixture: admitted under the own-build rule (waived: isa.x86 hostcompat_impossible_feature)\n".into()));
    reader.record(&clauses, false);
    assert_eq!(reader.run("ok").0, 1);
    for source in ["declared", "proven"] {
        clauses[2]["source"] = json!(source);
        reader.record(&clauses, true);
        assert_eq!(reader.run("ok").0, 1);
    }
}

#[test]
fn final_k13_answers_are_distinct_and_single_line() {
    let reader = Reader::new();
    for (mode, extra, exit, detail) in [
        (
            "unmeasured",
            json!({"unmeasured":"no recorded requirement"}),
            0,
            "unmeasured (no recorded requirement)",
        ),
        (
            "unavailable",
            json!({"unavailable":[BlobCid::compute_bytes(b"absent").to_string()]}),
            0,
            "unmeasured (requirement not held locally)",
        ),
        (
            "gap",
            json!({"gap":"document does not decode"}),
            0,
            "unmeasured (document does not decode)",
        ),
        (
            "gap",
            json!({"gap":"unreadable slot\nnext line"}),
            0,
            "unmeasured (unreadable slot\\nnext line)",
        ),
        (
            "missing",
            json!({}),
            0,
            "unmeasured (epr has no requirement reader yet)",
        ),
        ("error", json!({}), 0, "unmeasured (artifact reader failed)"),
        (
            "unknown",
            json!({"known":false}),
            1,
            "epr does not know output",
        ),
        (
            "conflict",
            json!({"conflict":"incompatible clauses"}),
            1,
            "conflicting recorded requirements (MergeError) — rebuild with just mesh build fixture",
        ),
    ] {
        let mut reply = json!({"output":reader.output, "self":true, "produce":null,
            "known":true, "requires":null, "path":null});
        reply
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        fs::write(reader.dir.path().join("reply"), reply.to_string()).unwrap();
        let (actual, line) = reader.run(mode);
        assert_eq!(actual, exit, "{mode}: {line}");
        assert!(line.contains(detail), "{mode}: {line}");
        assert!(!line.contains("admitted"));
        if mode == "unknown" {
            assert!(line.contains("— rebuild with just mesh build fixture"));
        }
    }
    reader.record(&floor(), true);
    fs::write(reader.dir.path().join("reply"), "{}").unwrap();
    let (exit, line) = reader.run("ok");
    assert_eq!(exit, 1);
    assert!(line.starts_with("REFUSED compatibility fixture: malformed"));
}

#[test]
#[cfg(target_arch = "x86_64")]
fn null_or_invalid_local_produce_never_waives() {
    let reader = Reader::new();
    let mut clauses = floor();
    clauses.push(json!({"axis":"isa.x86", "predicate":{"kind":"subset", "value":["hostcompat_impossible_feature"]}, "source":"extracted"}));
    for (local, produce, exit, detail) in [
        (
            true,
            json!(null),
            1,
            "isa.x86 requires hostcompat_impossible_feature",
        ),
        (
            true,
            json!("invalid"),
            1,
            "REFUSED compatibility fixture: malformed requirement record: invalid local Produce CID",
        ),
        (
            false,
            json!("invalid"),
            1,
            "isa.x86 requires hostcompat_impossible_feature",
        ),
    ] {
        reader.record(&clauses, local);
        let path = reader.dir.path().join("reply");
        let mut reply: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        reply["produce"] = produce;
        fs::write(path, reply.to_string()).unwrap();
        let (actual, line) = reader.run("ok");
        assert_eq!(actual, exit);
        assert!(line.contains(detail), "{line}");
        assert!(!line.contains("admitted"));
    }
}

#[test]
fn cli_separates_record_and_diagnostic_forms() {
    for args in [
        vec!["--own-produce", "cid", "--requires", "file"],
        vec!["--output", "cid"],
        vec!["--binary", "file"],
        vec!["--requires", "file", "--output", "cid", "--binary", "file"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_ark"))
            .args(["host", "admit"])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(64));
    }
    let help = Command::new(env!("CARGO_BIN_EXE_ark"))
        .args(["host", "admit", "--help"])
        .output()
        .unwrap();
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("nothing was admitted and nothing was refused"));
}

fn floor() -> Vec<Value> {
    vec![
        json!({"axis":"arch", "predicate":{"kind":"equal", "value": std::env::consts::ARCH}, "source":"declared"}),
        json!({"axis":"os", "predicate":{"kind":"equal", "value": std::env::consts::OS}, "source":"declared"}),
    ]
}

fn run(clauses: &[Value]) -> (bool, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requires.json");
    fs::write(&path, serde_json::to_vec(clauses).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ark"));
    command
        .args(["host", "admit", "--requires"])
        .arg(&path)
        .args(["--name", "fixture"]);
    let output = command.output().unwrap();
    assert!(output.stderr.is_empty());
    let line = String::from_utf8(output.stdout).unwrap();
    assert_eq!(line.lines().count(), 1);
    (output.status.success(), line)
}

#[test]
fn admits_the_stored_clause_array() {
    assert_eq!(
        run(&floor()),
        (
            true,
            "ok compatibility fixture: admitted (diagnostic: requirement from file, not from the record)\n".into()
        )
    );
}

#[test]
#[cfg(target_arch = "x86_64")]
fn unmet_extracted_isa_clause_refuses_from_the_cli() {
    // Deliberately nonexistent CPUID feature makes this independent of the runner's CPU.
    let mut clauses = floor();
    clauses.push(json!({"axis":"isa.x86", "predicate":{"kind":"subset", "value":["hostcompat_impossible_feature"]}, "source":"extracted"}));
    for source in ["extracted", "declared", "proven"] {
        clauses[2]["source"] = json!(source);
        let (ok, line) = run(&clauses);
        assert!(!ok, "{line}");
        assert!(line.starts_with(
            "REFUSED compatibility fixture: isa.x86 requires hostcompat_impossible_feature"
        ));
        assert!(!line.contains("observed"));
        assert!(!line.contains("waived"));
    }
}

#[test]
fn unknown_missing_and_non_isa_requirements_refuse() {
    for axis in ["foreign.axis", "arch"] {
        let mut clauses = floor();
        if axis == "arch" {
            clauses.remove(0);
        }
        clauses.push(json!({"axis":axis,"predicate":{"kind":"equal","value":"foreign"},"source":"extracted"}));
        let (ok, line) = run(&clauses);
        assert!(!ok);
        assert!(line.contains(&format!("{axis} requires foreign")));
    }
    assert!(!run(&[]).0);
}

#[test]
fn malformed_reader_answers_and_written_bytes_refuse_in_one_line() {
    let reader = Reader::new();
    for (field, wrong, detail) in [
        (
            "output",
            json!(BlobCid::compute_bytes(b"other").to_string()),
            "output mismatch",
        ),
        ("known", json!(false), "inconsistent reader response"),
        (
            "requires",
            json!(BlobCid::compute_bytes(b"other").to_string()),
            "requirement CID does not match written bytes",
        ),
    ] {
        reader.record(&floor(), false);
        let path = reader.dir.path().join("reply");
        let mut reply: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        reply[field] = wrong;
        fs::write(path, reply.to_string()).unwrap();
        let (exit, line) = reader.run("ok");
        assert_eq!(exit, 1);
        assert!(line.starts_with("REFUSED compatibility fixture:"));
        assert!(line.contains(detail), "{line}");
    }
}

#[test]
#[cfg(target_arch = "x86_64")]
fn refusal_names_non_waivable_blocker_with_or_without_own_produce() {
    let reader = Reader::new();
    for own in [false, true] {
        for (axis, required, source, reason) in [
            ("lib", "libhostcompat_missing.so.1", "extracted", "unmet"),
            ("zzz.unknown", "missing", "extracted", "unknown-axis"),
            ("isa.x86", "amx_tile", "extracted", "unobserved"),
            (
                "isa.x86",
                "hostcompat_declared_feature",
                "declared",
                "unmet",
            ),
            ("isa.x86", "hostcompat_proven_feature", "proven", "unmet"),
        ] {
            let mut clauses = floor();
            clauses.push(json!({"axis":"isa.x86", "predicate":{"kind":"subset", "value":["hostcompat_impossible_feature"]}, "source":"extracted"}));
            clauses.push(json!({"axis":axis, "predicate":{"kind":if axis == "isa.x86" {"subset"} else {"present"}, "value":[required]}, "source":source}));
            reader.record(&clauses, own);
            let (exit, line) = reader.run("ok");
            assert_eq!(exit, 1);
            assert!(
                line.starts_with(&format!(
                    "REFUSED compatibility fixture: {axis} requires {required} —"
                )),
                "{line}"
            );
            assert!(line.contains(&format!("({reason})")), "{line}");
            assert!(!line.contains("waived"));
        }
    }
}
