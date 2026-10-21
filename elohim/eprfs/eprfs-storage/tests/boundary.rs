//! Rail R-E3 (interfaces spec §1.3): the eprfs adapter crates never depend on `elohim-storage`.
//! The dependency runs the other way: elohim-storage may depend on eprfs-core and eprfs-serve
//! (eprfs-serve's `testing` feature as a dev-dependency), never on eprfs-storage or eprfs-local.
//! That half lives in elohim-storage's own boundary test.

/// The manifest reader rail R-E1 uses too, so both rails read every form of a dependency.
#[path = "../../eprfs-core/src/boundary/manifest_deps.rs"]
mod manifest_deps;

/// Every package any dependency table names (normal, dev, build, target-specific, dotted), by
/// the package Cargo resolves, so a rename cannot hide one.
fn dependency_names(manifest: &str) -> Vec<String> {
    manifest_deps::dependencies(manifest)
        .iter()
        .map(|dep| dep.package_name().to_string())
        .collect()
}

#[test]
fn eprfs_adapters_never_depend_on_elohim_storage() {
    for (crate_name, manifest) in [
        ("eprfs-storage", include_str!("../Cargo.toml")),
        ("eprfs-local", include_str!("../../eprfs-local/Cargo.toml")),
        ("eprfs-serve", include_str!("../../eprfs-serve/Cargo.toml")),
        ("eprfs-agent", include_str!("../../eprfs-agent/Cargo.toml")),
    ] {
        let names = dependency_names(manifest);
        assert!(
            !names.is_empty(),
            "{crate_name}: the manifest parser found no dependency"
        );
        for denied in ["elohim-storage", "elohim-storage-client"] {
            assert!(
                !names.iter().any(|n| n == denied),
                "{crate_name} depends on {denied}: storage is the data plane and never a dependency of eprfs (rail R-E3)"
            );
        }
    }
}

#[test]
fn the_parser_sees_every_form_of_a_denied_dependency() {
    for manifest in [
        "[dependencies]\nstore = { package = \"elohim-storage\", path = \"x\" }\n",
        "[dependencies]\nelohim-storage.workspace = true\n",
        "[dependencies.elohim-storage]\npath = \"x\"\n",
        "[dependencies.store]\npackage = \"elohim-storage\"\npath = \"x\"\n",
        "[target.'cfg(unix)'.dependencies]\nelohim-storage = \"1\"\n",
        "[target.'cfg(unix)'.dev-dependencies.elohim-storage]\nversion = \"1\"\n",
        "[build-dependencies]\nstore.package = \"elohim-storage\"\nstore.version = \"1\"\n",
    ] {
        assert!(
            dependency_names(manifest).contains(&"elohim-storage".to_string()),
            "the R-E3 reader missed elohim-storage in:\n{manifest}"
        );
    }
    assert!(
        dependency_names("[dependencies]\ntokio.workspace = true\n").contains(&"tokio".to_string())
    );
}
