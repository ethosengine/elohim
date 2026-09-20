//! The settings-layer boundary, asserted transitively over this crate's own
//! lockfile.
//!
//! A direct `use tokio;` in `src/` fails to compile, which makes the *direct*
//! boundary self-enforcing. A **transitive** pull — one convenient dependency
//! that drags a runtime, a transport or the conductor client into the graph —
//! is invisible until link time, and by then the settings layer is no longer
//! beneath the stack it configures.
//!
//! Shape borrowed from `elohim-error/tests/boundary.rs` and, before it,
//! `crates/seam-contracts` (`boundary_tests::no_heavy_deps_in_dep_tree`).
//!
//! Denylist over allowlist, deliberately: an allowlist of the exact package set
//! would churn on every `serde` patch bump and train people to widen it
//! reflexively. The denylist names the *classes* that would falsify this
//! crate's claim to sit beneath the things it configures.
//!
//! `serde`, `serde_json`, `toml`, `dirs` and `tracing` are NOT denied — they
//! are the point. A `Config` that cannot be deserialised from TOML, and a
//! registry that cannot log a live flag flip, would not be this module.

/// (package name, why its presence falsifies this crate's claim)
const DENIED: &[(&str, &str)] = &[
    (
        "tokio",
        "THE load-bearing one for this crate. The runtime-config watcher's decisions are \
         synchronous std by construction and its CLOCK lives one layer up, in \
         elohim_storage::runtime_config_watch. A runtime here would colour every crate above \
         with an async choice made for the sake of one poll loop",
    ),
    (
        "libp2p",
        "transport is an upper layer; the settings that select a transport backend cannot \
         depend on one",
    ),
    (
        "iroh",
        "same as libp2p — both transport stacks read this crate's config, never the reverse",
    ),
    (
        "diesel",
        "settings are read before any pool exists and by code paths that hold no connection; a \
         database here would make the boot config depend on the thing it configures",
    ),
    (
        "reqwest",
        "an HTTP client in the settings crate means loading a config could do network I/O",
    ),
    (
        "hyper",
        "server/client stack — the HTTP boundary is the top layer, not this one",
    ),
    (
        "axum",
        "same as hyper — routing is above services, which are above this",
    ),
];

/// No runtime, transport, database or HTTP stack in this crate's graph.
#[test]
fn no_heavy_deps_in_dep_tree() {
    let lock = include_str!("../Cargo.lock");

    for (pkg, why) in DENIED {
        assert!(
            !lock.contains(&format!("name = \"{pkg}\"")),
            "elohim-settings pulled `{pkg}` into its dependency tree — {why}. \
             Every layer above reads this crate's config; a heavy subtree here \
             is paid for by all of them."
        );
    }

    // Holochain is a family, not one package (holochain_client, holochain_types,
    // holochain_zome_types, …), so it is matched by prefix.
    assert!(
        !lock.contains("name = \"holochain"),
        "elohim-settings pulled a `holochain*` package into its dependency tree. \
         DHT truth is the layer above; a node's settings must not be DNA-coupled, \
         or a conductor version bump recompiles the whole stack."
    );
}

/// Exactly ONE first-party crate sits beneath this one, and it is named here.
///
/// `elohim-cache-core` supplies `ExtractionCacheConfig`, which IS a field of
/// `Config` — the dependency is the type, not a convenience. It is taken with
/// DEFAULT features (`default = []`) precisely so its optional `native`
/// feature, which pulls tokio, stays out of this graph; `elohim-storage`
/// enables `native` on the same single package.
///
/// A second first-party dep appearing here is the signal that something is
/// being configured by the thing it configures — invert it instead.
#[test]
fn exactly_one_first_party_dep() {
    let lock = include_str!("../Cargo.lock");

    let elohim: Vec<&str> = lock
        .lines()
        .filter_map(|l| l.strip_prefix("name = \"elohim-"))
        .filter_map(|r| r.strip_suffix('"'))
        .collect();
    assert_eq!(
        elohim,
        vec!["cache-core", "settings"],
        "elohim-settings' first-party dependency set changed. Only elohim-cache-core may sit \
         beneath it (it supplies a Config FIELD type). Anything else means the settings layer \
         is depending upward — put the type here and let the consumer wire it."
    );

    for marker in ["name = \"doorway-", "name = \"eprfs-"] {
        assert!(
            !lock.contains(marker),
            "elohim-settings gained a `{marker}…` dependency — the settings layer cannot depend \
             on a gateway or a filesystem runtime."
        );
    }
}
