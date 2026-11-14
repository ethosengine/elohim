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
  gap) exit 3 ;;
  unknown) exit 2 ;;
  missing) echo "error: unrecognized subcommand 'requires'" >&2; exit 2 ;;
esac
cp "$base/requires" "$6"
sed "s|WRITE_PATH|$6|g" "$base/reply"
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
        fs::write(self.dir.path().join("reply"), json!({"output":self.output, "requires":BlobCid::compute_bytes(&serde_json::to_vec(clauses).unwrap()).to_string(), "self":own, "produce":BlobCid::compute_bytes(b"Produce").to_string(), "path":"WRITE_PATH"}).to_string()).unwrap();
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
fn gaps_are_unmeasured_but_unknown_output_is_refused() {
    let reader = Reader::new();
    reader.record(&floor(), true);
    for mode in ["gap", "missing"] {
        let (exit, line) = reader.run(mode);
        assert_eq!(exit, 0);
        assert!(line.contains("unmeasured ("));
    }
    let (exit, line) = reader.run("unknown");
    assert_eq!(exit, 1);
    assert!(line.starts_with("REFUSED compatibility fixture: epr does not know output"));
    fs::write(reader.dir.path().join("reply"), "{}").unwrap();
    let (exit, line) = reader.run("ok");
    assert_eq!(exit, 0);
    assert!(line.contains("unmeasured (malformed"));
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
