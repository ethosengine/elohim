---
title: elohim-storage decomposes into cohesive crates — bottom layer first, each born governed
id: storage-crate-decomposition-design
status: Draft
class: architecture
serves: dev-system-equilibrium
date: 2026-09-19
context-tier: disclosed
steward: agent:orchestrator@claude-fable-5-1
graduation-trigger: the first three extractions (error, settings, blob) have landed byte-identical and the measured full-gate and focused-test cycle times are recorded beside the 2026-09-19 baseline; then the db extraction is planned from §5 and the services/p2p seam (§7) is opened as its own design
cites:
  - genesis/data/timeline/backlog/arch-workspace-discipline-backlog.md
  - genesis/data/timeline/backlog/edge-buildmanifest-sibling-crate-source-globs.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/projection-reconcile-loc-ceiling-decomposition.md
  - "serving-edge-failover-balance-stream-campaign-plan | Serving edge campaign | sha256:c52d3b58876c3304 | path: genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md"
  - "elohim-storage-gospel | CLAUDE | sha256:cc601a081fc94151 | path: elohim/elohim-storage/CLAUDE.md"
  - elohim/elohim-facings/CLAUDE.md
---

# elohim-storage decomposes into cohesive crates

## 1. Why now — measured, 2026-09-19

`elohim/elohim-storage` is one crate: 593 files, 357 315 lines (`http.rs` 21 411, `p2p/mod.rs` 11 133,
`services/` 109 960, `db/` 48 225). A crate is rustc's unit of compilation and of memory, so:

| cycle | cost | why |
|---|---|---|
| one focused `cargo test --lib <filter>` after a one-line edit | 1 m 44 s – 3 m 54 s | the whole lib test target recompiles |
| `just gate elohim-storage` | ~33 min | clippy 1 m 23 s · test build 12 m 11 s (168 integration-test binaries each link the whole lib) · 3 912 lib tests 4 m 06 s · the 168 binaries |
| parallelism | `CARGO_BUILD_JOBS=1` | rustc peaked at 15.4 GB on this crate at default jobs and the RAM guard shed it; debuginfo was already cut to `line-tables-only` for the same reason |

Three small storage stories on 2026-09-19 each paid about forty minutes of machine time, serially, because they
share one compile unit and one target directory. Two earlier capacity workarounds (the job cap, the debuginfo
cut) treated a design signal as a resource problem.

This spec is the design pass for backlog `arch-workspace-discipline` **item 5** ("Extraction sequence — services
out of building blocks") and supplies the evidence its **item 8** (a crate-tier ceiling policy) was waiting for.
It composes with that backlog; it does not replace it.

## 2. What the coupling says

Counted `crate::X` references between top-level modules, then read every cut (rust-architect, 2026-09-19):

- **Below everything:** `error.rs` (122 lines, zero outward references, 35 dependants) and `epr_codec.rs` (320,
  zero outward) are true leaves. `config.rs` + `runtime_config.rs` are mutually cyclic and move as one unit with a
  single outward cut (`config.rs:432` → `services::head_adoption::ALTERNATE_ADVERTISER_CAP`).
- **`db/`** (89 files, 48 225 lines, 84 migration dirs) is used by 36 modules. Of its 116 upward lines, 85 are
  `use crate::error::StorageError` and vanish once the error crate sits beneath it; 31 remain, none needing more
  than a moved pure function, a moved wire struct, or one injected policy (`identity_namespace`, four call sites).
- **The blob family** (`blob_store`, `sharding`, `dag_store`, `compute_payload_store`, `content_server`, ~3.1 k
  lines) leaves the family at exactly two points — `content_server.rs:31` `ConductorClient` and `:35`
  `NodeIdentity`. `db/shard_manifests.rs` depends on it, so **blob lands before db**.
- **`metrics.rs` is not a foundation.** 6 201 lines with 21 upward cuts: it aggregates label vocabularies owned
  by upper layers. Only a zero-cut registry core could sit low; the instruments stay until their vocabularies
  have homes.
- **Nothing depends on `api/`, `http.rs`, `main.rs`** (~43 k lines) — a top layer.
- **`services` ↔ `p2p` is a cycle (111 / 194), and it is three strands wearing one name** — see §7.
- No candidate below contains a `#[cfg(feature = …)]`: the bottom layer sits under the feature surface.
- Visibility is no obstacle (14 `pub(crate)` items across every candidate) — which also means nothing today
  prevents the upward coupling being cut, so **a boundary test must hold each line afterwards**.

## 3. Principles

1. **One extraction per bounded shift, never a mid-edit refactor** (item 5). Each is its own commit series, its
   own gate, its own valueflow commitment.
2. **Byte-identical first, better second.** An extraction moves code; it does not improve it. Proof is a golden:
   identical generated TypeScript (sha256 over `elohim/sdk/storage-client-ts/src/generated/`), identical
   `/metrics` exposition where metrics are touched, the same test count passing. The method `elohim-facings`
   proved. Refactors ride separately, after.
3. **The re-export shim is the tool.** `elohim-storage/src/views.rs:271-278` already does
   `pub use elohim_views::…::*`, which is why ~122 `crate::views::X` call sites never changed. Every extraction
   keeps its old path alive the same way (`pub use elohim_error::StorageError;` in `lib.rs`), so an extraction
   touches a handful of files, not hundreds. Shims are removed later, one crate at a time, when convenient.
4. **The absence of the dependency is the boundary.** A lower crate cannot `use` an upper one — cargo enforces the
   layering, so no governance rule restates it. Each new crate carries a lockfile boundary test in the
   `seam-contracts` shape (item 6) asserting what it must never depend on (`libp2p`, `iroh`, `holochain_*`, and
   for the lowest, `diesel`).
5. **Idiomatic Rust, concretely:** `thiserror` enums in library crates, never `anyhow` in a public signature
   (item 7's lesson); traits at seams are small and named for the capability (`GossipPublisher`, `OutboundSink` —
   the crate already has ~30 such); no new process-global state, and existing `OnceLock`/`LazyLock` statics move
   unchanged; `pub` surface is what upper crates use today and nothing more; every crate has `#![forbid(unsafe_code)]`
   unless it already needs otherwise; features stay in the crates that need them.
6. **ts-rs moves atomically.** 80 `#[derive(TS)]` occurrences remain in storage, 17 of them in `db/`. ts-rs
   computes generated import paths from the source crate, so every TS type in a moved module moves in one commit
   with the sha256 golden, and no other codegen work is in flight while it does ("codegen is global and races").
7. **Every new crate is born governed** — §6.
8. **The deploy path is fixed before it is widened.** Backlog `edge-buildmanifest-sibling-crate-source-globs` is
   OPEN: the edge `cargo-build-storage` step watches only `elohim/elohim-storage/**`, and the Dockerfile COPY set
   is the same list maintained twice. A change to a sibling path-dep crate alone ships a **stale storage binary
   silently**. Every extraction widens that hole, so it closes in step 0.

## 4. The crates, in order

```
elohim-error ◄───────────────┬──────────────┐
     ▲            ▲          │              │
elohim-epr-codec  │    elohim-settings      │
     ▲            │   (config+runtime_config)│
elohim-blob ──────┘                          │
     ▲                                       │
elohim-db ──────────────────────────────────►┘   (+ elohim-views, existing)
     ▲
elohim-storage  (services, p2p, reconcile, sync, trust, api, http, main — for now)
```

| # | crate | what moves | cuts to make | riskiest step |
|---|---|---|---|---|
| 0 | — | nothing moves | derive the edge manifest's storage source globs and the Dockerfile COPY set from the crate's path-dep closure, in one place | getting it wrong ships stale binaries; prove with a sibling-only change on a branch build |
| 0b | — | 168 `tests/*.rs` → ~8 binaries grouped by area | tests that share process globals (metrics registry, runtime-config statics) must not share a binary with tests that assume a fresh one | hidden order dependence; run each group with `--test-threads=1` and default once |
| 1 | `elohim-error` | `error.rs` verbatim | none | none in code; step 0 must be done |
| 2 | `elohim-epr-codec` **or fold into `elohim-epr`** | `epr_codec.rs` verbatim | none | deciding which — a fifth EPR-shaped crate needs a reason; **open question Q1** |
| 3 | `elohim-settings` | `config.rs` + `runtime_config.rs` together | `config.rs:432` (move the constant down) | ~10 `publish_boot_*` setters become cross-crate; a missed call degrades to a silent default — add a boot assertion that every key was published |
| 4 | `elohim-blob` | blob_store, sharding, dag_store, compute_payload_store, content_server | two traits at `content_server.rs:31,35` (announce sink, node identity source) | the habit `blob-durability` moves with it (§6) |
| 5 | `elohim-db` | `db/`, `migrations/`, `diesel.toml`, the `migration-hygiene` justfile recipe, plus `generated_enums`, `identity_root`, `identity_namespace`, `binding_proof_wire`, `custody_announce`, `test_pool` (behind a `testing` feature) | the 31 residual lines (§2); rewrite 38 stray `embed_migrations!("migrations")` sites to the one canonical `MIGRATIONS` — which also collapses 39 expansions of an 84-directory tree into one | the 17 ts-rs types: one atomic commit with the sha256 golden |
| later | `elohim-storage-http` | `api/`, `http.rs` | its own design (21 k-line `http.rs` splits by route family first) | — |
| later | the services/p2p seam | §7 | — | — |

`metrics.rs` stays put. If `db`'s five self-instrumentation call sites force the question, a zero-cut
`elohim-metrics-registry` (registry, `Once`, register helpers) goes beneath db and the 6 k of label-aware
instruments stay where their vocabularies live — decided at step 5, not before.

Steps 0 and 0b change no architecture and pay back first: 0 removes a silent-staleness hazard, 0b is expected to
take roughly ten minutes off every gate.

## 5. Proof for each extraction

- `just gate elohim-storage` green with the **same lib test count** as before (3 912 on 2026-09-19) — tests that
  move are counted in the new crate's gate and the sum is reported.
- sha256 of generated TS unchanged (steps that touch a `TS` type: 5 only).
- The new crate's own gate green, and its boundary test green.
- Cycle times recorded beside the §1 baseline: full gate, and one focused test in the new crate and in storage.
  The decomposition is justified by these numbers moving, and is re-examined if they do not.
- Fleet: a storage binary built from the decomposed tree is content-equivalent in behaviour; the first edge
  build after each extraction is the reading. Extractions batch with other storage pushes — they do not earn
  their own fleet roll.

### Readings

| after | storage full gate | storage lib tests | new crate gate (cold / warm) | storage no-op `cargo test --lib` |
|---|---|---|---|---|
| baseline, 2026-09-19 | ~1 980 s | 3 912 | — | — |
| step 1 `elohim-error` (c63079ac2) | 1 368 s | 3 912 (+2 in the new crate) | 13 s / 7 s | 1 s |
| step 0b test binaries 168 → 14 | 1 048 s | 3 913 | — | — |
| step 2 head codec folded into `elohim-epr` | over the ceiling — a cold rebuild, `elohim-epr` sits under everything | 3 904 (−9, now 10 in `elohim-epr` with the golden) | `elohim-epr` gate green | — |
| step 3 `elohim-settings` | 971 s | 3 880 (−30 net: 31 moved, 1 kept, 1 added; +38 in the new crate) | 17 s / <1 s | — |

The 1 980 → 1 368 s drop is cache warmth, not the extraction — `error.rs` was 122 lines. It is recorded so that
later rows are read against an honest neighbour rather than against the cold baseline.

Step 0b moved no code out of the crate; it grouped 165 single-file integration-test binaries into 11 by domain
(`tests/<group>/main.rs`), leaving three standalone. Same tests, same verdicts — 821 passed, 0 failed, 41 ignored
before and after — and `cargo test --no-run` fell from 7 m 42 s to 3 m 49 s because 154 links disappeared. It is the
first storage gate under the 1 200 s ceiling; step 3's 971 s is the second, which is the finding's closing condition. One thing changed shape and not
coverage: the 68 files that carried a crate-root `#![cfg(feature = …)]` now sit behind `#[cfg(…)] mod x;` in their
group's `main.rs`, so under default features they are no longer even tokenised. They were already empty binaries
there. The 46 gated on `p2p-iroh` run only under `just test-iroh`, which no gate or pipeline calls —
`genesis/data/timeline/backlog/storage-gate-skips-iroh-two-peer-test.md` carries that gap, widened on this date.

The step-1 gate was also the first run under the gate-cycle ceiling (`gate-cycle-full-ceiling@1`, hard 1 200 s,
`.claude/epr-meta/measures.yaml`). It fired: finding `984ccc2c18d8` in `.claude/data/architecture-findings.jsonl`,
charter "does this make sense — can this be modularized?". **This spec is that finding's design pass**; no second
review is dispatched for it. The finding closes itself when two consecutive storage gates come in under the
ceiling, which is this decomposition's own success reading.

## 6. Born governed — `.epr-meta` in every new crate

None of the four crates already carved out (`elohim-views`, `elohim-facings`, `elohim-peer-fabric`,
`elohim-cache-core`) has a `.epr-meta/`, a `build-manifest.json` gate entry or a justfile. Planting governance is
new practice here, so it is stated once:

```
elohim/<crate>/
  Cargo.toml
  CLAUDE.md                      what this crate is, what it must never depend on, how to gate it
  justfile                       `gate` = fmt-check · clippy -D warnings · test · boundary test
  .epr-meta/
    manifest.md                  purpose · covers: subtree · cites this spec · the LoC ceiling policy binding
    <habit>.habit.md             only when a habit's concern lives here (see below)
  src/ …
  tests/boundary.rs              the lockfile boundary test
```

`manifest.md` is rules-light on purpose: `purpose:` says what the crate is for and what it refuses to know;
`covers: subtree` claims the tree; `cites:` points at this spec and at the crate's row in §4 — that cite is the
spec↔code link the valueflow projection follows; the one binding is `policy: source-file-loc-ceiling@1`, because
god-file growth is the drift this tree has actually produced. No layering rule: cargo is the layering rule.

**Habits move with their concern.** `elohim-storage/.epr-meta/` holds nine habit atoms. A habit is declared where
its behaviour lives, so `blob-durability.habit.md` moves to `elohim-blob/.epr-meta/` in step 4 (one `git mv`, then
`.claude/scripts/habits-project.py`); the rest stay until their concern gets a crate.

**The gate.** Each crate gets a `gate.projects` entry in `elohim/holochain/build-manifest.json` — the manifest
that owns storage's gate today — so `just gate` and pre-push pick up a changed crate and its dependants without a
second detector.

**On the valueflows.** Each row of §4 is one commitment (`epr flow claim` → implement → verify → `fulfill`),
citing this spec; the crate's manifest cites the same spec; `epr flow project` re-mints the edges. A reader
following a crate back to its reason, or this spec forward to its code, walks one chain.

## 7. The services ↔ p2p seam — named here, designed separately

The cycle is asymmetric and the asymmetry is the seam. Of ~111 `services → p2p` references, 47 are the single
concrete type `p2p::P2PHandle`, concentrated in five files. The rest of both arrows is:

1. a **wire-type stratum** that belongs beneath both (`binding_proof_wire`, `custody_announce`, `feedback_signal`,
   `topics`, `reconcile_rails`, `sync_state`) — extracted downward, a large share of both arrows disappears with
   no trait at all;
2. a **decision-logic stratum** (`head_adoption` and its neighbours: `advertiser_health`, `head_batch_resolver`,
   `reanchor_backfill`, `heal_backoff`, …) that p2p calls but that owns no I/O — it sits beside services and p2p
   depends on it one way. This is the code the 2026-09-19 stories touched, and the crate whose extraction would
   make such stories compile in seconds;
3. the genuine two-way edge, **`P2PHandle`** — the one place a trait is needed, an outbound-capability trait in
   the shape `services/gossip_flood.rs:72 GossipPublisher` and `services/back_prop.rs:77 OutboundSink` already
   are, supplied by the libp2p and iroh adapters (item 5: "retrofit libp2p onto the 7 existing iroh-side
   `*Backend` traits — compose with, don't duplicate").

That is a design of its own, opened when this spec graduates.

## 8. Non-goals

- No behaviour change, no API change, no schema change, no renamed metric.
- No workspace-wide restructuring: the existing extracted crates' membership conventions are followed as found.
- No extraction rides inside a feature story, and no feature story waits on an extraction.

## 8a. Settled by the step-2 / step-3 groundwork (2026-09-19)

**Q1 is answered: `epr_codec` FOLDS into `elohim-epr`, as `elohim/epr/src/head.rs`.** Folding adds zero
dependencies — every crate `epr_codec.rs` uses is already declared by `elohim-epr` at matching versions — and
`elohim/epr/src/cid.rs:9` already defines the identical `DAG_CBOR_CODEC = 0x71` and the same CIDv1 construction, so a
separate crate would be a fifth EPR crate whose dependency set is a subset of an existing one's. No consumer of
`elohim-epr` is wasm-compiled (its own `flake.nix` says "Native crate"), and it is already a path dep of storage,
already COPY'd by the Dockerfile and already in the edge manifest's globs, so no build wiring is needed.
**Licence — decided provisionally, 2026-09-19 (operator delegated the call "for now, we're still in
development"; the stated ideal: encourage re-use that supports a commons while discouraging borrowers from breaking
the base primitives).** `elohim-epr` is CAL-1.0 and `elohim-storage` is AGPL-3.0. The rule for this decomposition:

- code that FOLDS into an existing crate takes that crate's licence — so the head codec becomes CAL-1.0;
- a crate carved out as a storage INTERNAL (`elohim-error`, `elohim-settings`, `elohim-blob`, `elohim-db`) keeps
  storage's AGPL-3.0;
- the protocol's base primitives — what others build ON — are CAL-1.0. It is the closest existing fit to the stated
  ideal: share-alike, so re-use stays in the commons, and its distinctive obligation is that whoever runs the
  software for others must give those users their own data and the keys to it — "borrow this, but do not break
  the primitive that makes it trustworthy", in licence form. OSI-approved.

This creates no new situation: AGPL storage already depends on CAL `elohim-epr`. Copyleft incompatibility binds
downstream redistributors, not the copyright holder, and all of this code has one. Who may redistribute the
combined binary, and the 20 unlicensed crates, remain the workspace-discipline backlog's item 2 — the sweep that
should be done with legal advice before the first outside contributor or the first published release. Not legal
advice; an engineering default, recorded so it can be revisited rather than rediscovered. Step 2 is unblocked.

**`elohim-settings` (step 3) — the cut list holds, with three additions.**
- Exactly one outward reference, as predicted (`config.rs:432` → `ALTERNATE_ADVERTISER_CAP`); move the constant down
  and leave an alias at `services/head_adoption.rs:335`. `config.rs:592 DEFAULT_HEAL_RESOLVE_FANOUT` is `pub(crate)`
  and consumed from `p2p/projection_reconcile.rs:9000`, so it becomes `pub`.
- **The file watcher stays in storage.** `runtime_config::spawn_watcher` (`:1037-1095`) is the one place the pair
  touches tokio (`tokio::spawn` + `tokio::time::interval`); everything else — registry, parse, apply, `reload_now`,
  `set_watched_key` — is synchronous `std`. The watcher has one call site (`main.rs:762`). It moves to a thin
  `src/runtime_config_watch.rs`, and the settings crate's boundary test denies `tokio`.
- Two module-scoped shims, not one glob: `src/config.rs` → `pub use elohim_settings::config::*;` and
  `src/runtime_config.rs` → `pub use elohim_settings::runtime_config::*;` (a flattened namespace would collide
  `Key` / `Kind` / `config_path`).
- 31 unit tests (13 + 18) move verbatim; none reaches another storage module. No integration test targets the pair.

**The spec's named risk was already live.** §4 row 3 warned that a missed `publish_boot_*` "degrades to a silent
default". The groundwork found it had happened the same day, before any move: `set_reanchor_held_backoff_seconds` and
`set_contest_remint_window_seconds` (serving-edge stories 1.1 / 1.2) shipped with **zero call sites** — the keys are
registered and read live, but the operator's env var is never published. Fixed in `main.rs`, and
`config::tests::every_boot_publisher_is_called_from_main` now fails the gate on any publisher without a call site.
That static check is the first half of the boot assertion; the runtime half (a `published` flag per setting, asserted
before the watcher starts at `main.rs:761`) lands in step 3, because `provenance` is seeded `BootEnv` at construction
and cannot tell "published" from "never touched".

Order for step 3, each independently landable: publishers fixed + static guard (done) → runtime boot assertion →
invert the one cut and widen the one `pub(crate)` → split the watcher out → create the crate on the `elohim-error`
template → two shims → build wiring → **verify at runtime**: diff `GET /admin/runtime-config` against a pre-move
capture (9 `SPECS` + 1 `TEXT_SPECS` + 5 `BOOT_ONLY`, provenance included) — a silent default shows up there and
nowhere else.

## 9. Open questions

- **Q1** — answered in §8a: fold into `elohim-epr` under CAL-1.0 (provisional licence rule recorded there).
- **Q2** — step 0b's grouping: by directory of the code under test, or by which process globals a test touches?
  Decided by a dry run that reports which tests fail when co-located.
- **Q3** — whether `elohim-db`'s self-instrumentation justifies `elohim-metrics-registry` — decided at step 5.
