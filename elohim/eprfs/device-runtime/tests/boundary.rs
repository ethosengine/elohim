//! The launcher and session primitives cannot pull in the peer/runtime stack.
#[test]
fn dependency_boundary() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "tokio",
        "reqwest",
        "hyper",
        "iroh",
        "eprfs-peer",
        "cid =",
        "sha2",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "unexpected dependency: {forbidden}"
        );
    }
    assert!(manifest.contains("eprfs-core"));
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin_store")).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap() == "tests.rs" {
            continue;
        }
        let source = std::fs::read_to_string(path).unwrap();
        for forbidden in [
            "tokio::",
            "reqwest::",
            "iroh::",
            "eprfs_peer::",
            ".unwrap()",
        ] {
            assert!(!source.contains(forbidden), "unexpected {forbidden}");
        }
    }
}
