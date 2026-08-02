//! # `elohim-seam-contracts` — the concern canon, as compile shapes and harnesses
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (Design surface 3, task P1.1). The plan's canon is the authority for what
//! each concern id (`C0`–`C14`) guarantees; this crate is one of its two
//! executable surfaces (the other is the decision-point registry, P3).
//!
//! ## What this crate is
//!
//! A **leaf** crate carrying the concern classes that have a *type* or a
//! *property* form:
//!
//! | Concern | Shape here |
//! |---|---|
//! | C4 honest absence | [`Answer<T>`] — `Present` / `Absent` / `Unreachable` |
//! | C8 observability-per-decision | [`ReasonLabel`] + its conformance checks |
//! | C2 monotonic authority | [`harness::arbitrated`] — permutation-invariance + tiebreak determinism |
//! | C6b idempotent effect | [`harness::quiescent`] — replay against settled state mints nothing |
//!
//! Everything here is a **side-effect-free plain-data function or type**.
//! Services wire them; the crate never reaches for a connection, a socket, or a
//! clock. That is the D1 inversion-of-control seam the integrator-compatibility
//! contract names (`genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-epr-integrator-compatibility-contract.md`).
//!
//! ## The boundary is the dependency graph, not a lint
//!
//! Zero first-party dependencies; `std` only by default. The two optional
//! dependencies (`serde`, `ts-rs`) are third-party and feature-gated. A reach
//! for `elohim-views`, `diesel`, `tokio`, `hdi`/`hdk`, `reqwest`, `libp2p`, or
//! `iroh` puts a package in this crate's own `Cargo.lock`, and
//! [`boundary_tests::no_heavy_deps_in_dep_tree`] fails — pointing at the broken
//! boundary instead of at a mystery link error or a bloated WASM artifact. The
//! same boundary is what keeps the zome (WASM) path open: CI asserts
//! `cargo build --target wasm32-unknown-unknown --no-default-features`.
//!
//! ## Doc convention (Design surface 3's closing rule)
//!
//! Every public item carries, in its rustdoc:
//!
//! - **Concerns:** the concern ids it answers (`C4`, `C8`, …);
//! - **Forcing incident:** the dated production event that made it necessary —
//!   a contract with no forcing incident is a guess;
//! - **Contract test:** where its own conformance is proven.
//!
//! `#![deny(missing_docs)]` below makes the first half of that convention
//! structural rather than aspirational.
//!
//! ## What this crate deliberately does NOT do
//!
//! - **No wire contract.** The `serde` feature is a convenience derive for
//!   internal and test use. The monomorphic, schema-first `answer` envelope
//!   (`elohim/sdk/schemas/v1/objects/answer.schema.json`) is plan task P1.4 and
//!   is the source of truth for anything that crosses the HTTP boundary — the
//!   View Schema Contract order is schema → Rust conforms → `schema_contract.rs`
//!   catches drift → TS generated.
//! - **No adoption.** Rewiring `LocalResolve`, `head_adoption`,
//!   `projection_reconcile`, or doorway's `ReconcileDecision` onto these types
//!   is P1.2/P1.3 — behavior-neutral, separately verified.
//! - **No liveness table.** The `Liveness` harness (C3) is P2.1; it is the one
//!   genuinely new artifact and it earns its own regression demonstration
//!   against two independently-dated historical predicate sets.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod answer;
#[cfg(feature = "harness")]
pub mod harness;
pub mod reason;

pub use answer::{Answer, AnswerState};
pub use reason::{
    assert_reason_labels_conformant, assert_reason_labels_discriminating,
    assert_reason_labels_stable, check_reason_labels, ReasonLabel, ReasonLabelViolation,
    ReasonLabelViolationClass,
};

#[cfg(test)]
mod boundary_tests {
    /// The leaf boundary, asserted transitively.
    ///
    /// **Concerns:** C7 (advertise/serve symmetry — the crate advertises "zero
    /// first-party deps, std-only default"; this is the check that it serves
    /// that); C6a (bounded work — a leaf crate is a declared budget on the
    /// dependency graph).
    ///
    /// **Forcing incident:** the `elohim-facings`
    /// `no_transitive_diesel_in_dep_tree` precedent — a direct `use diesel;`
    /// fails to compile, but a TRANSITIVE pull through the one path dep is
    /// invisible until link time. This crate has no path deps at all, so the
    /// equivalent risk is a third-party dep quietly acquiring a heavy subtree
    /// (or someone "temporarily" adding `elohim-views` for one type).
    ///
    /// **Contract test:** this function.
    ///
    /// Denylist over allowlist, deliberately: an allowlist of the exact package
    /// set would churn on every `ts-rs` patch bump and train people to widen it
    /// reflexively. The denylist names the *classes* that would falsify the
    /// crate's claim, plus a first-party guard (no `elohim-*`, no
    /// `doorway-client`) and a total-package ceiling so a heavy dep cannot
    /// arrive under a name nobody thought to deny.
    #[test]
    fn no_heavy_deps_in_dep_tree() {
        let lock = include_str!("../Cargo.lock");

        // (package name, why it falsifies the leaf claim)
        const DENIED: &[(&str, &str)] = &[
            ("diesel", "a contract predicate must never be able to touch a connection"),
            ("rusqlite", "same as diesel — persistence is not a contract concern"),
            ("tokio", "contracts are side-effect-free plain data; no runtime belongs here"),
            ("reqwest", "an HTTP client in a leaf contract crate means a predicate can do I/O"),
            ("hyper", "server/client stack — see reqwest"),
            ("hdi", "the WASM zome path must stay open; the HDK/HDI surface is the adapter's job, not the contract's"),
            ("hdk", "same as hdi"),
            ("libp2p", "transport belongs behind the service seam"),
            ("iroh", "transport belongs behind the service seam"),
            ("automerge", "CRDT state is a service concern"),
            ("holochain_integrity_types", "zome types would make this crate DNA-coupled"),
        ];

        for (pkg, why) in DENIED {
            assert!(
                !lock.contains(&format!("name = \"{pkg}\"")),
                "elohim-seam-contracts pulled `{pkg}` into its dependency tree — {why}. \
                 The leaf boundary (zero first-party deps, std-only default, WASM-buildable) \
                 is broken; this crate is no longer safe to link from a zome or an external \
                 peer runtime."
            );
        }

        // First-party guard: the crate claims ZERO first-party dependencies.
        for marker in [
            "name = \"elohim-",
            "name = \"doorway-client\"",
            "name = \"seam",
        ] {
            let occurrences = lock.matches(marker).count();
            let allowed = usize::from(marker == "name = \"elohim-"); // our own package entry
            assert!(
                occurrences <= allowed,
                "elohim-seam-contracts gained a first-party dependency (matched `{marker}` \
                 {occurrences}× in Cargo.lock, allowed {allowed}). A contract crate that \
                 depends on the substrate cannot be the substrate's contract — invert the \
                 dependency: put the type here and let the consumer wire it."
            );
        }

        // Ceiling: catches a heavy subtree arriving under an unforeseen name.
        // Measured 18 at authoring (2026-08-02): this crate + serde (3) +
        // ts-rs (2) + their proc-macro/diagnostic chain. The lockfile records
        // OPTIONAL deps too, so this count is feature-independent. Raise
        // deliberately with a note, never reflexively — the headroom is the
        // point.
        let packages =
            lock.matches("\n[[package]]").count() + usize::from(lock.starts_with("[[package]]"));
        assert!(
            packages <= 20,
            "elohim-seam-contracts' lockfile grew to {packages} packages (ceiling 20). \
             A leaf crate's whole dependency tree should be serde + ts-rs + their proc-macro \
             chain. Something heavy arrived under a name the denylist does not know."
        );
    }
}
