//! The leaf boundary, asserted transitively over this crate's own lockfile.
//!
//! A direct `use tokio;` in `src/lib.rs` fails to compile, which makes the
//! *direct* boundary self-enforcing. A **transitive** pull — someone adds one
//! convenient dependency that drags a runtime, a transport or the conductor
//! client into the graph — is invisible until link time, and by then the
//! bottom of the crate stack is no longer the bottom.
//!
//! So the boundary is asserted where it is actually visible: the resolved
//! dependency graph. Shape borrowed from `crates/seam-contracts`
//! (`boundary_tests::no_heavy_deps_in_dep_tree`).
//!
//! Denylist over allowlist, deliberately: an allowlist of the exact package set
//! would churn on every `thiserror` patch bump and train people to widen it
//! reflexively. The denylist names the *classes* that would falsify this
//! crate's claim to sit beneath everything.
//!
//! `diesel`, `sled`, `serde_json` and `thiserror` are NOT denied — they are the
//! point. This crate exists to name the four foreign failure types the storage
//! vocabulary converts, and it cannot do that without depending on them.

/// (package name, why its presence falsifies the bottom-of-the-stack claim)
const DENIED: &[(&str, &str)] = &[
    (
        "libp2p",
        "transport is an upper layer; an error vocabulary that knows a swarm cannot sit beneath one",
    ),
    (
        "iroh",
        "same as libp2p — both transport stacks depend on this crate, never the reverse",
    ),
    (
        "tokio",
        "an error enum is plain data; a runtime here would make the bottom layer async-coloured",
    ),
    (
        "reqwest",
        "an HTTP client in the error crate means a conversion could do I/O",
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

/// No transport, runtime, HTTP stack or DNA coupling in this crate's graph.
#[test]
fn no_heavy_deps_in_dep_tree() {
    let lock = include_str!("../Cargo.lock");

    for (pkg, why) in DENIED {
        assert!(
            !lock.contains(&format!("name = \"{pkg}\"")),
            "elohim-error pulled `{pkg}` into its dependency tree — {why}. \
             This crate is depended on by every layer above it; a heavy subtree \
             here is paid for by all of them."
        );
    }

    // Holochain is a family, not one package (holochain_client, holochain_types,
    // holochain_zome_types, …), so it is matched by prefix.
    assert!(
        !lock.contains("name = \"holochain"),
        "elohim-error pulled a `holochain*` package into its dependency tree. \
         DHT truth is the layer above; the error vocabulary must not be \
         DNA-coupled, or a conductor version bump recompiles the whole stack."
    );
}

/// This crate claims ZERO first-party dependencies — nothing sits beneath it.
#[test]
fn no_first_party_deps_in_dep_tree() {
    let lock = include_str!("../Cargo.lock");

    for marker in ["name = \"elohim-", "name = \"doorway-", "name = \"eprfs-"] {
        let occurrences = lock.matches(marker).count();
        // Our own package entry is the only permitted match.
        let allowed = usize::from(marker == "name = \"elohim-");
        assert!(
            occurrences <= allowed,
            "elohim-error gained a first-party dependency (matched `{marker}` \
             {occurrences}×, allowed {allowed}). The bottom crate of the stack \
             cannot depend on the stack — invert it: put the type here and let \
             the consumer wire it, or reduce the conversion to a `String` variant."
        );
    }
}
