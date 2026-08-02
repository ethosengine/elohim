# elohim-seam-contracts

Executable seam contracts for the Elohim Protocol **concern canon** — the small
set of concern classes (`C0`–`C14`: a fixed vocabulary of cross-cutting
correctness properties such as C4 honest-absence and C8 observability-per-decision,
defined in `.claude/epr-meta/{policies,concerns}.yaml` and detailed in the
seam-concern plan linked below) that keep being rediscovered bespoke at every
new boundary. This crate carries the ones that have a *type* form or a *property*
form, so a concern solved once at one **seam** (a seam is a crate or module locus
where a decision predicate answers one of these concerns) becomes a compile
shape or a harness that every other seam can adopt.

**Who this is for:** a Rust developer writing a decision point — any function that
answers "is it there," "who wins," or "should this apply again" — in a peer runtime,
a service, or a Holochain zome. You do not need to run an Elohim peer to use this
crate; it is plain data and has no runtime.

**The one idea to hold:** *a boundary answer is not a boolean, and an outcome is not a
string.* This crate gives each a type, so the distinctions survive review, refactors,
and dashboards instead of being re-litigated after the next incident.

**Prerequisites (to use the crate):** Rust 1.83+. Nothing else — no async
runtime, no network, no database. Building the crate's *own* gate (its tests,
clippy, and WASM check) has one more prerequisite; see
[Maintainer gate](#maintainer-gate) — it does not apply to consuming the crate.

```toml
[dependencies]  # Answer<T> / ReasonLabel at runtime — always add this one.
elohim-seam-contracts = { version = "0.1.0", registry = "elohim" }

[dev-dependencies]  # ONLY if you also want the property harnesses in your own
                     # tests (a runtime consumer never needs this second entry).
elohim-seam-contracts = { version = "0.1.0", registry = "elohim", features = ["harness"] }
```

The registry `elohim` is a Nexus hosted index; add it to `~/.cargo/config.toml`
(anonymous read is enabled, so no token is needed to build):

```toml
[registries.elohim]
index = "sparse+https://nexus.ethosengine.com/repository/cargo-internal/"
```

Registry reachability is not assumed. Inside this monorepo, skip the registry
and depend on the crate by path instead (adjust the relative path to your
crate's own location — this example is from a crate two directories under a
top-level workspace such as `elohim/` or `doorway/`):

```toml
[dependencies]
elohim-seam-contracts = { path = "../../crates/seam-contracts" }
```

The library name is `seam_contracts` — that is what you `use`.

**Vocabulary, once, up front:** a **conductor** is Holochain's local peer
process; **"fleet"** throughout this crate just means whatever runs a check
without a human in the loop (a peer set, a service, a single process); on a
**full-arc fleet** every node holds the whole DHT arc, so a local read has no
network fetch behind it — a miss means gossip hasn't delivered the record yet,
not that it doesn't exist.

## First use

Create `tests/first_use.rs` in a crate with the `[dependencies]` entry above
(no `harness` feature needed for this one):

```rust
use seam_contracts::Answer;

#[test]
fn first_use() {
    // A conductor-local miss on a full-arc fleet is Unreachable, not Absent.
    let miss: Option<u8> = None;
    assert_eq!(Answer::from_local_get(miss), Answer::Unreachable);

    let hit: Option<u8> = Some(7);
    assert_eq!(Answer::from_local_get(hit), Answer::Present(7));
}
```

<!-- Mirrors `from_local_get_maps_miss_to_unreachable` in
     crates/seam-contracts/src/answer.rs (mod answer_tests) — same constructor,
     same two cases, same assertions, wrapped in a test fn the same way that
     module wraps its own. -->

**Expected pass signal:** `cargo test first_use` — green, same as
`from_local_get_maps_miss_to_unreachable` inside this crate's own suite.
Nothing here talks to a peer, a database, or the network; the crate is plain
data (see [Leaf by construction](#leaf-by-construction) below).

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
| **C3** liveness | `harness::liveness` — every reachable non-terminal state has an *automated* move |

These five rows are the **complete set** this crate carries today. Not every
`C0`–`C14` concern class has a type or property form (some are process or
review disciplines instead); the plan linked above is the map of which is
which.

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
wire them; that inversion of control is the seam named `D1` by the
integrator-compatibility contract
(`genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-epr-integrator-compatibility-contract.md`)
— a distinct numbering from the plan's "Design surface N" cited above, not the
same series despite the resemblance.

## Features

| Feature | Default | What it adds |
|---|---|---|
| `serde` | off | `Serialize`/`Deserialize` derives on `Answer<T>` and `AnswerState` (the non-generic, payload-free sibling of `Answer<T>` — same three states, for call sites that need the discriminant without the generic). **Convenience only — not the wire contract:** these derives are for internal/test use; the monomorphic, schema-first `answer` envelope (`elohim/sdk/schemas/v1/objects/answer.schema.json`) is what a service should map to at its own HTTP boundary — this crate ships no (de)serializer for that envelope. |
| `ts` | off | ts-rs derives on `AnswerState` (the only non-generic vocabulary type today). Implies `serde`. |
| `harness` | off | The `Arbitrated`, `Quiescent`, and `Liveness` harnesses. Enable in `[dev-dependencies]`; a runtime consumer never links them. |

## `Answer<T>` — the full-arc law

The two constructors — the **complete** set today:

```rust
fn observed_absence<T>(responder_said: Option<T>) -> seam_contracts::Answer<T> { unimplemented!() }
fn from_local_get<T>(local_store_said: Option<T>) -> seam_contracts::Answer<T> { unimplemented!() }
```

```rust
use seam_contracts::Answer;

// The responder answered — either "here it is" or "no such thing," and
// either way absence (if any) was OBSERVED, not merely unheard.
assert_eq!(Answer::observed_absence(Some(7)), Answer::Present(7));
let responder_result: Option<u8> = None;
let a = Answer::observed_absence(responder_result);
assert_eq!(a, Answer::Absent);

// A conductor-local get on a FULL-ARC fleet — every node holds the whole DHT
// arc, so there is no network fetch behind a local read. A miss here means
// gossip has not delivered the record, NOT that it does not exist.
let local_get_result: Option<u8> = None;
let b = Answer::from_local_get(local_get_result);
assert_eq!(b, Answer::Unreachable);
```

**Picking a constructor:**

- **A responder answered you at all** (HTTP, a zome remote call, any peer
  that replied): use `observed_absence`. Its `None` means "the responder said
  no such thing."
- **A local, full-arc read** — a store that is supposed to already hold
  everything reachable: use `from_local_get`.
- **Your local store is NOT full-arc** (a partial arc, a cache, a projection
  that may lag): do not route through either constructor above — both would
  mint a claim you have not earned (`observed_absence` would falsely claim
  `Absent` on a stale miss; `from_local_get` presumes full-arc coverage you do
  not have). Construct **`Answer::Unreachable` directly** — it is a plain
  public variant, not gated behind a constructor — and let a higher layer that
  *can* ask an authoritative responder decide `Present`/`Absent`.

The reason to hold the type instead of collapsing back to `Option` is that the
two non-present states demand different responses, and the type makes that
un-skippable:

```rust
use seam_contracts::Answer;

fn handle(b: Answer<u8>) {
    match b {
        Answer::Present(v) => println!("got {v}"),
        Answer::Absent => { /* record it: the boundary confirmed nothing is there */ }
        Answer::Unreachable => { /* retry or heal: nothing is established either way */ }
    }
}
```

Propagate `Answer<T>` upward through your own call stack; collapse it exactly
once, at your outermost boundary, by hand-mapping to the schema-first `answer`
envelope (below) — the crate ships no serializer for that envelope on purpose,
so adoption stays incremental instead of an all-at-once rewrite.

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
message into a test — it runs `assert_reason_labels_conformant` first (unique,
non-empty, metric-safe labels; ≥2 variants) and then compares `ALL`, in order,
against the slice you pass, so **reordering your variants is itself a failure**
along with adding, removing, or renaming one. `ALL` is what both checks
iterate; the compiler cannot verify it lists every variant, so a forgotten one
is the one mistake these checks cannot catch — review `ALL` by eye against the
enum whenever you touch either.

A reason that wants to carry context is not a label — it is a *capsule*, and its
home is **C14's residual channel** (the canon's diagnostic/detail channel for
context that must never become a metric dimension — not itself carried by this
crate). Mixing the two produces an unbounded-cardinality metric, which is a
different way of having no metric.

## Harnesses (`--features harness`)

Each harness comes in two forms sharing one name stem: `check_*` returns
`Result<Report, Failure>` for your own error-propagating code, and `assert_*`
calls the matching `check_*` and panics with every finding — reach for
`assert_*` inside a `#[test]`. Every form is callable two ways — at
`seam_contracts::harness::assert_arbitration(..)` (root) or
`seam_contracts::harness::arbitrated::assert_arbitration(..)` (its own
submodule) — pick whichever reads better at the call site; both are the same
function. (The three spellings you'll see for one harness — module `arbitrated`,
functions `assert_arbitration`/`check_arbitration`, and "the `Arbitrated`
harness" in its own doc comment — are not typos: same C2 property, named by
grammatical role.) The snippet below is illustrative (`candidates`,
`select_canonical_winner`, etc. are stand-ins for your own types); each
harness's own module doc (`arbitrated`, `quiescent`, `liveness`) carries a
runnable example.

```rust,ignore
// C2: the winner is a function of the candidate SET, not of arrival order.
seam_contracts::harness::assert_arbitration(&candidates, select_canonical_winner);

// C6b: replay against settled state mints nothing — while reconciliation stays EAGER.
seam_contracts::harness::assert_quiescent(initial_state, decide, apply);

// C3: every state the fleet can be in has a move the FLEET can take.
seam_contracts::harness::assert_liveness(&states, classify, transitions);
```

Every harness declares a budget and respects it, but "exhaustive or sampled"
is not one uniform knob — check `Report::exhaustive` where it applies:

- **Arbitration** trades exhaustive permutation enumeration for a sampled walk
  once `n!` exceeds `ArbitrationBudget::max_permutations` (default 720, i.e.
  exhaustive through 6 candidates) — `ArbitrationReport::exhaustive: bool` is
  the field that tells you which ran. The sample is a **deterministic**
  pseudo-random walk with a fixed seed (not flaky across runs). `assert_*`
  always uses the default budget; to raise it, call
  `check_arbitration_with(&candidates, select, ArbitrationBudget { max_permutations: 5040, ..Default::default() })`
  yourself and `.expect(..)` the result — there is no `assert_*_with` sibling.
- **Quiescence**'s budget is just a round count (`QuiescenceBudget::rounds`,
  default 4) — round 1 may legally mint, rounds 2+ must not; there is no
  sampled mode, only "how many rounds did we check."
- **Liveness** is exhaustive **by construction** over whatever table you hand
  it — that is the whole reason it is a table harness instead of a
  `proptest`-style search; its budget guards the table's *size*, not a
  proof/sample tradeoff.

An instrument that silently degraded from proof to evidence — reporting green
while actually having sampled, i.e. **advertising more coverage than it
delivers** — would be committing the exact C7 failure (advertise/serve
asymmetry) it exists to catch in the code it checks.

**Quiescence is about effect, not effort.** It never licenses lazy acceptance:
elohim-storage remains an eagerly-reconciling controller. The property is that a
tick where nothing changed mints nothing — not that the tick may skip the work.

**Liveness counts moves the fleet can take.** You enumerate the states, classify
each one (`Live` / `Terminal` / `Unreachable`, the latter two carrying a written
justification), and list the transitions out of each with their *agency*. This
is `check_liveness` (the `Result`-returning form — its sibling `assert_liveness`
is what the harness overview above uses), over a small illustrative state space
— a blob-acquisition "pin" waiting for bytes, not `std::pin::Pin`:

```rust
use seam_contracts::harness::liveness::{check_liveness, StateClass, Transition};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Pin { Queued, InFlight, Exhausted, Filled }

let states = [Pin::Queued, Pin::InFlight, Pin::Exhausted, Pin::Filled];

let report = check_liveness(
    &states,
    |s| match s {
        Pin::Filled => StateClass::Terminal("the bytes are here; nothing further is owed"),
        _ => StateClass::Live,
    },
    |s| match s {
        Pin::Queued => vec![Transition::automated("dial_advertiser").to(Pin::InFlight)],
        Pin::InFlight => vec![
            Transition::automated("bytes_verified").to(Pin::Filled),
            Transition::automated("attempts_exhausted").to(Pin::Exhausted),
        ],
        // Without this arm the only exit was `operator DELETE /pins/{id}`.
        Pin::Exhausted => vec![Transition::automated("backoff_retry").to(Pin::Queued)],
        Pin::Filled => vec![],
    },
)
.expect("every state has an automated move that eventually ends");

assert!(report.progress_checked);
```

<!-- Byte-for-byte the `# Example` doctest on `check_liveness` in
     crates/seam-contracts/src/harness/liveness.rs — a real, currently-passing
     `cargo test --doc` case, not illustrative pseudocode. -->

Swap that `Exhausted` arm for `Transition::human("operator DELETE /pins/{id}")` and
the check fails, naming the state: the machine is *complete* and the fleet is
still stuck. **A human hand — or a deploy — as the only exit is a liveness hole,
not an escape hatch.** When every transition declares where it lands, the harness
also proves the moves *end*: a cycle of perfectly legal automated transitions
that never reaches a terminal state is the failure that reads as health on a
dashboard.

The failure lists **every** dead-ended state at once, not the first — a stuck
state machine usually has more than one wall, and finding them serially is what
turns a bug into a week.

## What this crate deliberately does not do

- **No wire contract.** The `serde` derives are for internal and test use. The
  monomorphic `answer` envelope
  (`elohim/sdk/schemas/v1/objects/answer.schema.json`) is authored schema-first
  and is the source of truth for anything crossing the HTTP boundary.
- **No adoption.** Rewiring `LocalResolve`, `head_adoption`,
  `projection_reconcile`, or doorway's `ReconcileDecision` onto these types is a
  separate, behavior-neutral step.
- **No state spaces.** The liveness harness never enumerates a predicate's
  states for you. The table is authored beside the predicate and reviewed with
  it, because what the table omits is exactly what the check cannot see.

## Maintainer gate

The dev environment exports a WASM-targeted `RUSTFLAGS`
(`--cfg getrandom_backend="custom"`, needed elsewhere for Holochain WASM
builds) globally; it breaks this crate's native `cargo test`/`clippy` runs, so
clear it first. The last line needs the WASM target installed once per
toolchain:

```bash
rustup target add wasm32-unknown-unknown     # one-time, for the last line below
export RUSTFLAGS=""
cargo test                                   # default features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --check
cargo build --target wasm32-unknown-unknown --no-default-features
```

## License

AGPL-3.0
