# elohim-seam-contracts

Executable seam contracts for the Elohim Protocol **concern canon** — the small
set of concern classes (`C0`–`C14`) that keep being rediscovered bespoke at every
new boundary. This crate carries the ones that have a *type* form or a *property*
form, so a concern solved once at one seam becomes a compile shape or a harness
that every other seam can adopt.

**Who this is for:** a Rust developer writing a decision point — any function that
answers "is it there," "who wins," or "should this apply again" — in a peer runtime,
a service, or a Holochain zome. You do not need to run an Elohim peer to use this
crate; it is plain data and has no runtime.

**The one idea to hold:** *a boundary answer is not a boolean, and an outcome is not a
string.* This crate gives each a type, so the distinctions survive review, refactors,
and dashboards instead of being re-litigated after the next incident.

**Prerequisites:** Rust 1.83+. Nothing else — no async runtime, no network, no
database.

```toml
[dependencies]
elohim-seam-contracts = { version = "0.1.0", registry = "elohim" }

[dev-dependencies]
elohim-seam-contracts = { version = "0.1.0", registry = "elohim", features = ["harness"] }
```

The registry `elohim` is a Nexus hosted index; add it to `~/.cargo/config.toml`
(anonymous read is enabled, so no token is needed to build):

```toml
[registries.elohim]
index = "sparse+https://nexus.ethosengine.com/repository/cargo-internal/"
```

The library name is `seam_contracts` — that is what you `use`.

Design:
`genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
(Design surface 3). The plan is the authority for what each concern guarantees;
this crate is one of its two executable surfaces — the other is the
decision-point registry (`seam-registry.yaml` per crate).

| Concern | Shape here |
|---|---|
| **C4** honest absence | `Answer<T>` — `Present` / `Absent` / `Unreachable` |
| **C8** observability-per-decision | `ReasonLabel` + its conformance / stability checks |
| **C2** monotonic authority | `harness::arbitrated` — permutation-invariance + tiebreak determinism |
| **C6b** idempotent effect | `harness::quiescent` — replay against settled state mints nothing |

## Leaf by construction

- **Zero first-party dependencies.** No `elohim-views`, no `elohim-storage`, no
  `hdi`/`hdk`. A predicate here cannot reach a connection, a socket, or a clock.
- **`std`-only default.** `default = []` pulls nothing; `serde` and `ts-rs` are
  optional and third-party.
- **WASM-buildable.** `cargo build --target wasm32-unknown-unknown
  --no-default-features` is part of the gate, so the zome path stays open.
- **The lockfile is the boundary.** `boundary_tests::no_heavy_deps_in_dep_tree`
  reads this crate's own `Cargo.lock` and fails if a denied class, a first-party
  crate, or an unexplained subtree arrives — pointing at the broken boundary
  instead of at a mystery link error.

Everything public is a side-effect-free plain-data type or function. Services
wire them; that inversion of control is the D1 seam the integrator-compatibility
contract names.

## Features

| Feature | Default | What it adds |
|---|---|---|
| `serde` | off | `Serialize`/`Deserialize` derives on `Answer<T>` and `AnswerState`. **Convenience only — not the wire contract** (see below). |
| `ts` | off | ts-rs derives on the non-generic vocabulary types. Implies `serde`. |
| `harness` | off | The `Arbitrated` and `Quiescent` property harnesses. Enable in `[dev-dependencies]`; a runtime consumer never links them. |

## `Answer<T>` — the full-arc law

```rust
use seam_contracts::Answer;

// The responder answered "no such thing" — absence was OBSERVED.
let a = Answer::observed_absence(responder_result);

// A conductor-local get on a FULL-ARC fleet. A miss means gossip has not
// delivered the record, NOT that it does not exist.
let b = Answer::from_local_get(local_get_result);   // None => Unreachable
```

There is deliberately **no** `impl From<Option<T>>`: a blanket conversion has to
pick one mapping for `None`, and whichever it picks becomes the ergonomic default
at the call sites where it is wrong. On a full-arc fleet, `None → Absent` is
wrong for every conductor-local read.

`Answer::into_option()` is the named, greppable collapse for boundaries that have
not adopted the type yet — searching for it is searching for the remaining C4
debt.

## `ReasonLabel` — outcomes you can count

```rust
use seam_contracts::ReasonLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContestFailure { NoLocalChain, NotRetrievable, StampNotNewer }

impl ReasonLabel for ContestFailure {
    const ALL: &'static [Self] = &[
        ContestFailure::NoLocalChain,
        ContestFailure::NotRetrievable,
        ContestFailure::StampNotNewer,
    ];
    fn label(&self) -> &'static str {
        match self {
            ContestFailure::NoLocalChain  => "no_local_chain",
            ContestFailure::NotRetrievable => "not_retrievable",
            ContestFailure::StampNotNewer  => "stamp_not_newer",
        }
    }
}

#[test]
fn labels_are_a_dashboard_contract() {
    seam_contracts::assert_reason_labels_stable::<ContestFailure>(
        &["no_local_chain", "not_retrievable", "stamp_not_newer"],
    );
}
```

Label strings are a contract with every panel, alert, and recording rule keyed on
them. `assert_reason_labels_stable` turns "unchanged" from a claim in a commit
message into a test.

A reason that wants to carry context is not a label — it is a *capsule*, and its
home is C14's residual channel. Mixing the two produces an unbounded-cardinality
metric, which is a different way of having no metric.

## Harnesses (`--features harness`)

```rust
// C2: the winner is a function of the candidate SET, not of arrival order.
seam_contracts::harness::assert_arbitration(&candidates, select_canonical_winner);

// C6b: replay against settled state mints nothing — while reconciliation stays EAGER.
seam_contracts::harness::assert_quiescent(initial_state, decide, apply);
```

Both harnesses declare a budget, respect it, and report whether the check was
exhaustive or sampled — an instrument that silently degraded from proof to
evidence would be committing the advertise/serve asymmetry it exists to catch.

**Quiescence is about effect, not effort.** It never licenses lazy acceptance:
elohim-storage remains an eagerly-reconciling controller. The property is that a
tick where nothing changed mints nothing — not that the tick may skip the work.

## What this crate deliberately does not do

- **No wire contract.** The `serde` derives are for internal and test use. The
  monomorphic `answer` envelope
  (`elohim/sdk/schemas/v1/objects/answer.schema.json`) is authored schema-first
  and is the source of truth for anything crossing the HTTP boundary.
- **No adoption.** Rewiring `LocalResolve`, `head_adoption`,
  `projection_reconcile`, or doorway's `ReconcileDecision` onto these types is a
  separate, behavior-neutral step.
- **No liveness table.** The `Liveness` harness (C3) earns its own regression
  demonstration against two independently-dated historical predicate sets.

## Verification

```bash
export RUSTFLAGS=""
cargo test                                   # default features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --check
cargo build --target wasm32-unknown-unknown --no-default-features
```

## License

AGPL-3.0
