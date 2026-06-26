---
title: Blob Custody P3-8 slice 1b — thread household into the salvage pool (humans-join) + swap to diversity placement
id: blob-custody-p3-8-1b-salvage-household-plumbing-handoff
status: Draft
class: implementation-handoff
domain: D5
sprint: substrate-validation
cites:
  - blob-custody-phase3-xor-salvage-placement-design | The anchor spec — its P3-8 row names household co-location as salvage's sharpest blind spot; this handoff implements that placement-diversity step (1b) | sha256:f4c139eee8478b9a | path: genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md
  - che-keyless-peer-client-slice1-governance-spine-plan | Sibling dataplane slice — the op-gate authorizes THAT a node distributes; this ensures WHERE salvage re-places keeps diversity real (the same lens, governance vs substrate) | sha256:f49a28709387179a | path: genesis/docs/superpowers/plans/2026-06-26-che-keyless-peer-client-slice1-governance-spine-plan.md
requires_env: [household-nodes]
---

# Handoff: Blob Custody P3-8 — slice 1b (salvage household plumbing)

> **For agentic workers:** this is a START-HERE handoff for ONE bounded implementation slice. Read it,
> then execute with `superpowers:subagent-driven-development` (or directly with TDD). Slice **1a is already
> landed** — do NOT re-build the strategy.

**Goal:** make the already-landed `DiversityAwarePlacementStrategy` (slice 1a) actually act on real
households during autonomous salvage re-placement, by (1) populating each salvage candidate's
`household_id` from the `humans` projection at pool-build time, and (2) swapping the hard-coded
`XorDistanceStrategy` for the diversity strategy behind a config knob.

**Why this exists:** salvage re-placement is currently **household-blind** — it builds its candidate pool
with `household_id: None` and hard-selects XOR distance, so when it heals an under-replicated blob it can
silently co-locate multiple replicas in one household, eroding the diversity that ingest establishes
(invisible to the op-gate / resilience card). 1a built the *decision logic*; 1b gives it the *data* and
*turns it on*.

## Prior context (what's done)

- **Slice 1a — LANDED, commit `9e0f84f4b`** (`feat/frontend-eyes-sprint`):
  `elohim/elohim-storage/src/reconcile/placement.rs` now has `DiversityAwarePlacementStrategy` — a pure,
  deterministic, diversity-first multi-pass greedy (maximize distinct `household_id` coverage first, XOR
  distance as the within/across-domain tiebreak), behind the existing sealed `PlacementStrategy` trait. 21
  unit tests. **Key property you can rely on:** with no household data (all `household_id: None`) it
  degrades **EXACTLY** to `XorDistanceStrategy` — so wiring it in is *never worse than today*, and strictly
  better once households are known. An adversarial review already caught + fixed an order-dependence break
  (conflicting-household duplicate `agent_cid`), so the strategy is order-robust by construction.

## The decision this handoff encodes: humans-JOIN (option B), not gossip-ad (option A)

Slice 1a's review surfaced two ways to give the salvage pool each peer's household. **Option B was chosen
(operator decision, 2026-06-26).** Do NOT implement option A.

- **B (CHOSEN) — join `humans.household_id` at pool-build.** Mirrors the already-shipped *ingest* selector
  (`services/peer_selection.rs:184-203`). No migration, no wire-format change, and **no new trust surface**:
  household comes from the notarized `humans` projection, not peer self-report.
- **A (REJECTED) — add `household_id` to the `SalvageCapacityAd` gossip ad.** Would make failure-domain
  identity *self-reported* in gossip (a peer could lie about its household to manipulate placement) — a
  trust anti-pattern the protocol avoids — and it's a wire-format change needing the p2p-design-gate.

## p2p-design-gate — pre-answered (confirm before coding)

Option B adds **no new data entity**, so the gate is satisfied by construction; confirm these still hold:
- **No new DHT entry type / table / route / sync message.** B reads existing `humans.household_id` and
  `salvage_capacity` rows into the existing `PlacementCandidate` struct (whose `household_id` field already
  exists, doc-commented "Failure-domain key for diversity-aware strategies (P3-8)"). Category **C
  (operational)** — a local projection join.
- **Identity join key = `agent_cid`.** `salvage_capacity` is `agent_cid`-keyed (uhCAk…); `humans` stores
  the same namespace in `agent_pub_key` (uhCAk…, e.g. `uhCAkMATTHEW`). Join `salvage_capacity.agent_cid ==
  humans.agent_pub_key` — same namespace, **no cross-namespace string compare** (avoids the all-zeros
  resilience-card class of bug). The config knob is operational state, not notarized.

## File map

- **Modify** `elohim/elohim-storage/src/services/salvage_commitment_author.rs` — `run_salvage_pass`
  (fn at `:131`). Today: builds `candidates` from `salvage_capacity::list_fresh` mapping each row to
  `PlacementCandidate { household_id: None, … }` (`:148-158`, the `None` at `:152`), pushes self via
  `from_agent_cid` (`:160`, also `None`), and hard-selects `let strategy = XorDistanceStrategy;` (`:163`).
  This is the whole 1b surface.
- **Reference (mirror its join)** `elohim/elohim-storage/src/services/peer_selection.rs:184-203` — the
  ingest household enrichment: `humans::table.filter(h_app_id).filter(agent_pub_key.eq_any(&ids)).select((agent_pub_key, household_id))`
  → `HashMap<agent_cid, Option<household_id>>`. Copy this shape.
- **Modify** `elohim/elohim-storage/src/reconcile/custody.rs:310` — `SalvageConfig` struct (the config-knob
  home; add the strategy-selection field here).
- **Modify** `elohim/elohim-storage/src/main.rs:1556` — the salvage tick task that calls `run_salvage_pass`.
  Thread `h_app_id` (the installed app id — capture it into the spawned task closure the same way
  `salvage_self_cid` is captured) and the config-knob value into the call.
- **Reuse (do NOT modify)** `elohim/elohim-storage/src/reconcile/placement.rs` — `DiversityAwarePlacementStrategy`
  (1a). `db/humans.rs` (columns `agent_pub_key`, `household_id`), `db/salvage_capacity.rs` (`list_fresh`,
  `agent_cid`-keyed).

## Steps (TDD; each ends green before the next)

1. **Add the config knob.** Add a field to `SalvageConfig` (custody.rs:310) selecting the placement
   strategy — e.g. `pub diversity_placement: bool` (or a `PlacementMode` enum if you prefer explicitness).
   Decide the default: the strategy is *never worse than XOR*, so defaulting **on** is defensible, but a
   knob lets ops roll out deliberately — recommend default **on**, env/config-overridable. Wire the value
   from `main.rs` config (add a `config.*` field + env). Update all `SalvageConfig { … }` literals
   (there's a test stub in custody.rs ~`:1009`/`:1048` and the prod build).
2. **Thread `h_app_id` into `run_salvage_pass`.** Add an `h_app_id: &str` param; capture the installed app
   id into the salvage task closure at `main.rs:1556` and pass it. (Needed for the humans filter — the
   ingest join filters `humans::h_app_id.eq(input.h_app_id)`.)
3. **Write the failing test (join populates households).** In `salvage_commitment_author.rs` tests (or a
   `tests/` file), seed an in-memory SQLite with: 2+ `salvage_capacity` fresh rows (distinct `agent_cid`s),
   matching `humans` rows with populated `agent_pub_key` + distinct `household_id`s, and one human with
   **NULL** `agent_pub_key` (dormant). Assert the built candidate set carries the expected `household_id`
   per cid, and that the dormant/unmatched cid stays `household_id: None`. (Extract the candidate-build into
   a testable helper if `run_salvage_pass` is awkward to call directly — keep the join logic unit-covered.)
4. **Implement the join.** After `list_fresh`, collect the candidate `agent_cid`s (incl. `self_cid`),
   query `humans` (mirror peer_selection.rs:184-203) into a `HashMap<String, Option<String>>`, and set each
   `PlacementCandidate.household_id` from the map (`None` when absent — dormant humans simply don't match,
   which is correct: the strategy treats `None` as its own domain → XOR fallback). Populate the self
   candidate's household too. Make step 3 pass.
5. **Swap the strategy behind the knob.** Replace `let strategy = XorDistanceStrategy;` (`:163`) with a
   branch on the config knob: `DiversityAwarePlacementStrategy` when on, `XorDistanceStrategy` when off.
   Both impl the same trait, so the call to `salvage_pass` is unchanged.
6. **Write the behavioral tests.** With households seeded (step 3 fixture) and the knob **on**: assert the
   salvage selection spans distinct households when available (reuse the 1a property shape). With the knob
   **off**: assert it matches `XorDistanceStrategy`. With **all-dormant** humans (all `None`): assert it
   equals XOR regardless of knob (the safety property).
7. **Run gates + commit.** See build env below. `cargo fmt`, `cargo clippy -- -D warnings` on the touched
   files, the targeted tests. Selective-stage ONLY the files you edited. Commit-only — the integrator
   pushes (never `git push`).

## Build & test environment (read — this bites)

- **elohim-storage KEEPS the ambient `RUSTFLAGS`** (`--cfg getrandom_backend="custom"`, Holochain WASM) —
  do NOT clear it for this crate.
- **The cargo-target-pool slot currently hits a fingerprint ENOENT** (`failed to write
  …/.fingerprint/…/invoked.timestamp — No such file or directory`). Use the sanctioned fallback: a `/tmp`
  target + disable the wrapper:
  `export CARGO_TARGET_DIR=/tmp/<scratch>/storage-target; export RUSTC_WRAPPER=""` (first build is cold,
  ~2.5 min; incremental ~30s).
- **Plain `cargo test` (no nextest in this container).** Targeted run:
  `cargo test --lib salvage_commitment_author` (and `reconcile::custody` / `reconcile::placement`).
- Do NOT touch/stage the ambient-dirty storage files (pre-existing): `api/contributors.rs`,
  `api/identity.rs`, `conductor/process_manager.rs`, `db/contributor_presences.rs`, `db/epr_atoms.rs`,
  `services/epr_nav_context_view.rs`. `main.rs` is yours to edit for step 2 — stage ONLY your salvage hunks.

## Done criteria (1b)

- Salvage candidates carry real `household_id` from the `humans` join (fixture-verified); dormant/unmatched
  peers stay `None` and degrade to XOR (verified).
- With the knob on + households available, salvage placement spans distinct households (no silent
  co-location); with the knob off it matches XOR; all-dormant matches XOR either way.
- fmt + clippy clean on touched files; targeted tests green; committed (not pushed).

## Caveats & what stays deferred

- **Dormancy gate (the honest limitation).** 1b lights only for `humans` rows with a populated
  `agent_pub_key` + `household_id`. The same `humans` NULL-`agent_pub_key` dormancy that keeps
  `region_occupancy` dark applies here — until humans are populated, the salvage pool sees `None` and the
  strategy correctly degrades to XOR. Fixing the dormancy at its source (the humans projection) is a
  SEPARATE, shared piece of work — do not try to route around it inside salvage.
- **Held (needs the live mesh, NOT this slice):** the P3-7 cross-peer "replica count rises" end-to-end
  proof — `distribute_shards`/`salvage_pass` report `distributed == 0` on a single node, so the live
  byte-movement verification needs a real multi-peer Jenkins mesh. Unit/fixture coverage of the *decision*
  is the whole of 1b; the live proof is its own held leg.
- **Composition:** purely additive behind the strategy seam + a config knob — do NOT rework `salvage_pass`,
  the commitment author, or the gossip ad (that was option A, rejected).

## Pointers

- Anchor spec: `genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md`
  (the **P3-8** row + §Intentional placement — household co-location is named salvage's "sharpest blind spot").
- Memory: `project_dataplane_next_lens_diversity_placement` (the lens decision + 1a/1b state).
- 1a commit: `9e0f84f4b`. Ingest join to mirror: `services/peer_selection.rs:184-203`.
