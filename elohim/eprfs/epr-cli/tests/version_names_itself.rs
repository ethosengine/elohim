//! `--version` names the exact executable without requiring repository discovery.

use std::{fs, process::Command};

use eprfs_core::BlobCid;
use serde_json::Value;

#[test]
fn version_names_the_binary_from_outside_a_worktree() {
    let binary = env!("CARGO_BIN_EXE_epr");
    let outside = tempfile::tempdir().expect("temporary non-git directory");
    let out = Command::new(binary)
        .arg("--version")
        .current_dir(outside.path())
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(
        out.status.code(),
        Some(0),
        "epr --version exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stderr.is_empty(), "epr --version wrote stderr: {stderr}");
    assert_eq!(
        stdout.lines().count(),
        1,
        "expected one JSON line: {stdout}"
    );

    let identity: Value = serde_json::from_str(stdout.trim()).expect("version is JSON");
    let bytes = fs::read(binary).expect("read the tested executable");
    let expected = BlobCid::compute_raw(&bytes).to_string();
    assert_eq!(identity["id"], "elohim-epr-cli");
    assert_eq!(identity["cid"], expected);

    let short = Command::new(binary)
        .arg("-V")
        .current_dir(outside.path())
        .output()
        .expect("epr -V runs");
    assert!(short.status.success(), "epr -V exited non-zero");
    assert_eq!(short.stdout, out.stdout, "-V and --version must agree");
}
