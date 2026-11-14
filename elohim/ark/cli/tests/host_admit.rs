use eprfs_core::BlobCid;
use serde_json::{json, Value};
use std::{fs, process::Command};

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
            "ok compatibility fixture: admitted (2 clauses)\n".into()
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
fn recorded_cids_are_checked_before_admission() {
    let dir = tempfile::tempdir().unwrap();
    let req = dir.path().join("requires.json");
    let bin = dir.path().join("binary");
    let bytes = serde_json::to_vec(&floor()).unwrap();
    fs::write(&req, &bytes).unwrap();
    fs::write(&bin, b"binary").unwrap();
    let req_cid = BlobCid::compute_bytes(&bytes).to_string();
    let output_cid = BlobCid::compute_bytes(b"binary").to_string();
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_ark"))
            .args(["host", "admit", "--requires"])
            .arg(&req)
            .args([
                "--requires-cid",
                &req_cid,
                "--output",
                &output_cid,
                "--binary",
            ])
            .arg(&bin)
            .output()
            .unwrap()
    };
    assert!(call().status.success());
    fs::write(&bin, b"foreign binary").unwrap();
    let output = call();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("output requires"));
    fs::write(&bin, b"binary").unwrap();
    fs::write(&req, b"[]").unwrap();
    assert!(!call().status.success());
}

#[test]
#[cfg(target_arch = "x86_64")]
fn own_produce_waives_only_extracted_isa_after_byte_verification() {
    let dir = tempfile::tempdir().unwrap();
    let req = dir.path().join("requires.json");
    let bin = dir.path().join("binary");
    fs::write(&bin, b"binary").unwrap();
    let output = BlobCid::compute_bytes(b"binary").to_string();
    let produce = BlobCid::compute_bytes(b"local Produce record").to_string();
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_ark"))
            .args(["host", "admit", "--requires"])
            .arg(&req)
            .args(["--name", "fixture", "--binary"])
            .arg(&bin)
            .args(["--output", &output, "--own-produce", &produce])
            .output()
            .unwrap()
    };
    let mut clauses = floor();
    clauses.push(json!({"axis":"isa.x86", "predicate":{"kind":"subset", "value":["hostcompat_impossible_feature"]}, "source":"extracted"}));
    fs::write(&req, serde_json::to_vec(&clauses).unwrap()).unwrap();
    let admitted = call();
    assert!(admitted.status.success());
    assert_eq!(String::from_utf8(admitted.stdout).unwrap(), "ok compatibility fixture: admitted under the own-build rule (waived: isa.x86 hostcompat_impossible_feature)\n");
    fs::write(&bin, b"foreign bytes").unwrap();
    let refused = call();
    assert!(!refused.status.success());
    assert!(String::from_utf8(refused.stdout)
        .unwrap()
        .contains("recorded CID does not match bytes"));
    fs::write(&bin, b"binary").unwrap();
    for source in ["declared", "proven"] {
        clauses[2]["source"] = json!(source);
        fs::write(&req, serde_json::to_vec(&clauses).unwrap()).unwrap();
        let refused = call();
        assert!(!refused.status.success());
        let line = String::from_utf8(refused.stdout).unwrap();
        assert!(line.starts_with("REFUSED compatibility fixture: isa.x86 requires"));
        assert!(!line.contains("waived"));
    }
}

#[test]
fn own_produce_requires_binary_and_output() {
    for extra in [vec![], vec!["--binary", "binary"], vec!["--output", "cid"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_ark"))
            .args([
                "host",
                "admit",
                "--requires",
                "requires.json",
                "--own-produce",
                "cid",
            ])
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(64));
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("required arguments"));
    }
}
