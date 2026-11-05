//! The wasm-safe leaf boundary, asserted on the lockfile (dev-dependencies included, since
//! Cargo locks them too). The addressing kernel is linked into the DNA zomes and into browser
//! builds: an async runtime, a TypeScript generator or a JSON stack arriving under it — even
//! transitively, even optionally — falsifies that claim, so the build fails here instead of at
//! a zome's link step.

#[test]
fn lockfile_holds_no_runtime_codegen_or_json_stack() {
    let lock = include_str!("../Cargo.lock");
    let packages: Vec<&str> = lock
        .lines()
        .filter_map(|l| l.strip_prefix("name = \""))
        .map(|l| l.trim_end_matches('"'))
        .collect();
    assert!(
        packages.contains(&"elohim-address"),
        "lockfile is this crate's"
    );
    for denied in [
        "tokio",
        "ts-rs",
        "serde_json",
        "mio",
        "reqwest",
        "ipld-core",
    ] {
        assert!(
            !packages.contains(&denied),
            "{denied} reached the elohim-address lockfile"
        );
    }
    for p in &packages {
        assert!(
            *p == "elohim-address" || !p.starts_with("elohim-"),
            "first-party crate {p} reached the leaf"
        );
    }
    assert!(
        packages.len() <= 72,
        "{} packages: a heavy subtree arrived",
        packages.len()
    );
}
