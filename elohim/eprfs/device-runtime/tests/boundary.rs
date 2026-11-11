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
    let mut pending = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin_store"
    ))];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                std::fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
            continue;
        }
        if path.file_name() == Some(std::ffi::OsStr::new("tests.rs"))
            || path.extension() != Some(std::ffi::OsStr::new("rs"))
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        for forbidden in [
            "tokio::",
            "reqwest::",
            "iroh::",
            "eprfs_peer::",
            ".unwrap()",
        ] {
            assert!(
                !source.contains(forbidden),
                "{}: unexpected {forbidden}",
                path.display()
            );
        }
    }
}
