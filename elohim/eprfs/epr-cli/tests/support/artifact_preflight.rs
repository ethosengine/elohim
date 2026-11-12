//! Copied mesh recognition facts never waive the uniform checklist.
use super::*;

#[test]
fn artifact_copied_mesh_recognition_facts_cannot_bypass_preflight() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "mesh");
    let scratch = TempDir::new().unwrap();
    let root = device.root();
    let crate_dir = "elohim/elohim-storage";
    std::fs::create_dir_all(root.join(crate_dir).join("src")).unwrap();
    std::fs::create_dir_all(root.join("elohim/holochain")).unwrap();
    std::fs::write(
        root.join(crate_dir).join("Cargo.toml"),
        "[package]\nname=\"elohim-storage\"\nversion=\"0.1.0\"\n[dependencies]\nserde=\"1\"\n",
    )
    .unwrap();
    std::fs::write(root.join(crate_dir).join("Cargo.lock"), "version = 4\n[[package]]\nname=\"serde\"\nversion=\"1.0.0\"\nsource=\"registry+https://github.com/rust-lang/crates.io-index\"\n").unwrap();
    std::fs::write(root.join(crate_dir).join("build.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join(crate_dir).join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        root.join("elohim/holochain/build-manifest.json"),
        r#"{"steps":{"cargo-build-storage":{"inputs":{"sources":["elohim/elohim-storage/**"]}}}}"#,
    )
    .unwrap();
    git(
        root,
        &["add", crate_dir, "elohim/holochain/build-manifest.json"],
    );
    git(root, &["commit", "-qm", "existing mesh recipe"]);
    let tools = scratch.path().join("tools");
    fake_cc(&tools, "cc mesh fixture");
    let path = format!("{}:/bin", tools.display());
    let (method, descriptor) = method_file_with(
        scratch.path(),
        &[crate_dir, "elohim/holochain/build-manifest.json"],
        &[("crate", crate_dir), ("PATH", &path)],
    );
    let key = elohim_epr_cli::artifact::closure_key(root, &descriptor).unwrap();
    let (brit, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("storage");
    std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    let out = device
        .command()
        .envs(&descriptor.non_path_inputs)
        .env("BRIT_BUILD_REF_BIN", brit)
        .args(["artifact", "put", "--method"])
        .arg(method)
        .args(["--input", &key.input.to_string(), "--bytes"])
        .arg(bytes)
        .args(["--project", "elohim-storage", "--repo"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("registry dependency serde has no lockfile checksum"),
        "{}",
        text(&out)
    );
    assert!(text(&out).contains("compiler inputs unverifiable (no dep-info)"));
    assert!(events(root).is_empty());
    assert!(!log.exists());
    let flow = SidecarFlowStore::open(root).unwrap();
    assert!(flow.events().unwrap().is_empty());
}
