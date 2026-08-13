---
title: Agentic-harness borrows — run-plane write path, equilibrium stocks, projection emitter, and the cluster's mintable rows — Implementation Plan
id: agentic-harness-borrows-implementation-plan
status: Draft
class: process-meta
domain: D9
sprint: agentic-harness-borrows
requires_env: [household-nodes]
cites:
  - run-plane-projection-observation-events | Spec A — the write leg (epr flow note) and the per-turn projection emitter this plan implements as Tasks 1 and 3 | sha256:0781aecc53d8a0d0 | path: genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md
  - dev-system-equilibrium-stocks | Spec B — the drain-vs-inflow equilibrium fold and --check verdict this plan implements as Task 2, and whose red is Task 4 first_move | sha256:5306c437d02200f2 | path: genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md
  - commitment-dispatch-puller | Spec C — cluster row 4, deliberately NOT built here: sequenced after the write path, cited so the plan names its own boundary | sha256:608803ebc8811e4a | path: genesis/docs/superpowers/specs/2026-08-13-commitment-dispatch-puller-design.md
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
  - genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md
  - genesis/manifests/habits.yaml
  - genesis/research/elohim-as-viable-system-2026-06-04.md
---

# Agentic-harness borrows — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or
> superpowers:subagent-driven-development where Tasks are independent) to implement this plan
> task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the eight rows of
[agentic-harness-borrows-backlog](genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md)
as one composed slice: a **write path** into the EPR plane (`epr flow note`) so a mid-run correction
survives compaction, a **read-only equilibrium fold** (`epr flow stocks`) so every stock in the
development system can be asked "is drain ≥ inflow?", a **per-turn projection emitter** that
re-derives a short state block from registers we already keep, and the four bounded edits the
cluster names as mintable now (ceremony gate de-escalation, Codex instruction-budget repair, the
Beer amendment, the habits delta).

**Architecture:** The survey's finding sets the shape — *the gap is projective, not
representational*. We already hold the durable workflow object: `FlowRecord::{Intent | Commitment |
Event | Process | Spec | Edge}` appended CID-deduped to `.eprfs/status/flows.jsonl`
(`elohim/epr-rea/src/store.rs:22-31`, `:201-222`, CID re-verified on every read at `:246-252`), and
the dimensional measure primitives `MeasureKind::{Level, Rate{per}, Ratio}`
(`elohim/epr/src/measure.rs:30-34`) with `Stock{level, inflow, outflow}` and its
construction-time refusals (`elohim/epr-rea/src/stock.rs:114-118`, `:121-148`). What is missing is
(a) any CLI leg that *writes a run-scale observation* — the six existing legs are
`project | walk | status | seal | reseal | hold | fulfill` (`elohim/eprfs/epr-cli/src/flow/mod.rs:74-164`,
usage at `:306-319`), none of which accepts an authored note; (b) any caller of
`stock_over_window_within` (`stock.rs:343-468`) — the fold exists and is tested but no command
invokes it; and (c) any per-turn injection point — `UserPromptSubmit` carries exactly one hook
today (`.claude/settings.json:64-74`).

**Tech stack:** Rust (the `elohim/eprfs` native workspace for `epr-cli`; the `elohim` virtual
workspace for `epr-rea`), Python 3 (the `.claude/hooks` transport layer), Node (the
`elohim-agent` package-projection tooling), YAML (`habits.yaml`, seam registries, `.epr-meta`).

**Env:** `household-nodes`. Nothing in this plan needs the live alpha fleet, a conductor, or shem —
every gate is a local build, a local test, or a projection-fidelity check.

**Cargo target pool (container discipline).** `gate_pool_slot` composes
`$CARGO_TARGET_POOL_ROOT/family/<family>/<flattened-workspace-rel>/dev` where `<family>` is
branch-derived (`.husky/pre-push.bash:97-114`, `:116-125`), and `run_gate` maps the `elohim-epr`
project to workspace root `elohim` (`.husky/pre-push.bash:703-710`). On branch `dev` that resolves
to:

```
# epr / epr-rea (members of the elohim virtual workspace)
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev
# eprfs (its OWN workspace, excluded from the monorepo workspace — pre-push.bash:321-336)
export CARGO_TARGET_DIR=/tmp/eprfs-gate-target
```

On a shift branch re-derive the family segment rather than pasting these; never let a native cargo
build fall back to an in-tree `target/`. `RUSTFLAGS=""` for both (the eprfs gate sets it explicitly
at `.husky/pre-push.bash:333-335`). **No `cargo nextest` in this container** — plain `cargo test`,
never piped (echo `EXIT=$?` on its own line).

**Compose, don't reinvent.** Row 1 adds **no new register** (survey LEAVE-11: a `current.md` is a
fifth register with no reader); row 4's puller is `/delivery-stasis` refined, not a new scheduler;
row 6 is a bounded edit to an existing skill through package authority; row 8 is an amendment, never
an in-place edit of the 2026-06-04 text.

---

## File structure (locked)

| File | Responsibility |
|---|---|
| Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` | `note` + `stocks` arms in the `run()` dispatch table; `usage()` lines; `pub mod note; pub mod stocks;` |
| Create `elohim/eprfs/epr-cli/src/flow/note.rs` | the write leg — resolve → append one `FlowRecord::Event`, two-phase (resolve-all-then-append) per `seal.rs::reseal` |
| Create `elohim/eprfs/epr-cli/src/flow/stocks.rs` | the read-only fold leg — build `Window`s, call `stock_over_window_within`, render + `--check` verdict |
| Create `elohim/eprfs/epr-cli/tests/flow_note.rs` | unit/integration over a fixture `flows.jsonl` |
| Create `elohim/eprfs/epr-cli/tests/flow_stocks.rs` | two-window fixture tests + `--check` exit semantics |
| Create `elohim/eprfs/epr-cli/seam-registry.yaml` | FIRST registry for this crate — the note-admission and equilibrium-verdict decision points, with an honesty clause for the pre-existing unregistered surface |
| Create `.claude/hooks/run-projection.py` | the per-turn emitter (`UserPromptSubmit` plain stdout / `SessionStart` JSON wrapper) |
| Modify `.claude/settings.json` | register the emitter on `UserPromptSubmit` + `SessionStart` with an honest timeout |
| Modify `.claude/hooks/.epr-meta` | the emitter's governance row, carrying `retire-when:` |
| Modify `genesis/manifests/habits.yaml` | **operator-gated** — park `declarative-desired-state`, admit `dev-system-equilibrium` unwired |
| Modify `.epr-meta/elohim/packages/skills/memory-ceremony.json` | Phase 1 pick de-escalation (package authority, then replant) |
| Modify `elohim/sdk/domains/elohim-agent/scripts/mcp-packages.mjs` + `.epr-meta/elohim/packages/mcp-profiles/elohim-project.json` | emit `project_doc_max_bytes` into the generated `.codex/config.toml` |
| Modify `genesis/research/elohim-as-viable-system-2026-06-04.md` | **append-only** amendment section |
| Create `genesis/a2o/features/…` scenarios (Task 8) | the `@concern:` scenarios Specs A/B decompose to |

No DHT entry types, no HTTP routes, no schema files, no new registers.

---

### Task 1: `epr flow note` — the write leg (Spec A mechanism 1, cluster row 2)

**Files:** Modify `elohim/eprfs/epr-cli/src/flow/mod.rs`; create
`elohim/eprfs/epr-cli/src/flow/note.rs`, `elohim/eprfs/epr-cli/tests/flow_note.rs`,
`elohim/eprfs/epr-cli/seam-registry.yaml`.

**Context:** Row 2 is the **precondition for row 1** — "a projection loop over state nobody writes
to projects nothing." Survey §4.4 names the hole precisely: an operator's mid-run correction lands
in the conversation, compaction is lossy summarization of exactly that region, and nothing
re-injects it because habits/commitments were never told. `epr flow note` is that channel's repair,
and it is also row 5's run-scale failed-approaches field (`Tried X, it failed because Y, switched to
Z`) — one leg, two rows. It mints a `FlowRecord::Event` (`elohim/epr-rea/src/store.rs:22-31`) rather
than a new record kind: the observation is an economic event in the dev valueflow, and adding a
seventh variant would move protocol vocabulary, which is not an implementer's call.

- [x] **Step 1: Read the two idioms before writing.** `run()`'s dispatch table and `parse_global`
      (`elohim/eprfs/epr-cli/src/flow/mod.rs:74-164`, `:263-286` — this crate hand-rolls its
      subcommand table; `--root`/`--json` are stripped before leg-specific parsing) and the
      **two-phase append** at `elohim/eprfs/epr-cli/src/flow/seal.rs:251-287`: Phase 1 resolves
      *every* candidate and collects failures, returning `FlowError::InvalidArguments("reseal
      aborted, nothing appended — …")` if any failed; Phase 2 appends only once all resolved. Copy
      that shape — a `note` that half-appends is worse than one that refuses.

- [x] **Step 2: Add the dispatch arm** in `mod.rs`'s `match sub` alongside `"seal"`/`"hold"`, and
      the matching `usage()` line at `mod.rs:306-319` — **the surface is Spec A §3's, verbatim**:

```
epr flow note --on <commitment-cid-or-path> --kind failed-approach|correction|observation
              --reason TEXT [--switched-to TEXT] [--json] [--root DIR]
```

`--on` is the resource the note is about (resolved through the same label/tree lookup that raises
`FlowError::UnknownResource`, `mod.rs:56-57`); `--reason` is the authored body; `--switched-to` is
the optional consequence half of a failed-approach (the clax idiom: "system is too stiff. Switched
to Kvaerno5"); `--kind` is the closed triad `failed-approach|correction|observation` and lands in
`classified_as` as `run:<kind>` (mirroring how `derive_scenario` classifies at `project.rs:572-605`
— no fourth kind may be minted here without a Spec A amendment). Refuse an empty `--reason` and an
unresolvable `--on` with `FlowError::InvalidArguments` — refusal is the honest default this crate
already uses.

- [x] **Step 3: Add the additive `classified_as` field to `FlowEvent`** — Spec A §2's "one
      mechanical decision the gate forced": `FlowEvent` (`elohim/epr-rea/src/model.rs:375-392`) has
      no `classified_as` today (it lives on `ResourceSpec`, `model.rs:74-77`). Add
      `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub classified_as: Vec<String>` so
      every pre-existing record's CID is byte-stable, and pin it with a test mirroring
      `an_unbounded_commitment_keeps_its_pre_bound_cid` (`model.rs:701` precedent). Source of truth:
      the repo-plane `.eprfs/status/flows.jsonl` sidecar (append-only, CID-verified on read, local
      and reprojectable — NOT a DHT entity, NOT a SQLite table; per the spec's P2P-gate section).
      This touches **epr-rea, not epr-cli** — run the `elohim-epr` gate clauses for it
      (`CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev`) and include
      `elohim/epr-rea/src/model.rs` in Step 8's commit.

- [x] **Step 3b: Implement `note.rs`.** Two phases:
      **Phase 1 (resolve, append nothing)** — open `SidecarFlowStore` (`store.rs:201-216`), resolve
      `--on` to its body CID, resolve the actor `AgentRef`, and build the `FlowEvent`
      (`elohim/epr-rea/src/model.rs:375-392`) with **empty `fulfills`/`satisfies`** — a note
      annotates via `in_scope_of` + `classified_as`; it never discharges the commitment (Spec A §3's
      load-bearing rule: the discharged set is derived from `fulfills` at `walk.rs:170-179`).
      This dating choice — the git head commit, below — **resolves Spec A open question Q1 for v1**
      (recorded back in the spec); its known coarseness is that notes authored against the same head
      share a timestamp, with ordering carried by append order. `occurred_at` comes from the **git head commit**, the
      way every other timestamp on this path does (`project.rs:351-364` — "Deterministic and
      history-derived like every other timestamp on this path — never `now()`"); a note authored
      mid-run is dated by the tree it was authored against, not by wall clock. **Phase 2 (append)** —
      one `store` append, CID-deduped like every other record. A second identical note is a no-op,
      not a duplicate row.

- [x] **Step 4: Unit tests over a fixture `flows.jsonl`** in
      `elohim/eprfs/epr-cli/tests/flow_note.rs`, mirroring the fixture style of the existing
      `flow_fulfill.rs` / `flow_edges.rs`: (a) a note against a known resource appends exactly one
      line and the record round-trips through `store.records()` with its CID re-verified
      (`store.rs:246-252`); (b) the same note twice appends once (dedup); (c) an unresolvable `--on`
      appends **nothing** and returns `UnknownResource`; (d) empty `--reason` is refused before the
      store is opened; (e) `--json` emits the appended record's CID; (f) **empty-`fulfills`
      invariant** — a note never discharges its commitment: after a note, `walk::status` still
      counts the commitment unfulfilled; (g) the `classified_as` **CID-stability pin** from Step 3 —
      a record without the field round-trips to the same CID after the field lands.

- [x] **Step 5: `seam-registry.yaml` for `epr-cli` — FIRST registry for this crate.** Create
      `elohim/eprfs/epr-cli/seam-registry.yaml` conforming to
      `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`, mirroring the structure of
      `elohim/epr-rea/seam-registry.yaml` (`seamRegistryVersion`/`crate`/`crateRoot` at `:19-21`,
      `decisionPoints:` at `:31`). Register the new decision surfaces at birth — note admission
      (what makes an `--on` resolvable; what makes a note refusable) and, after Task 2, the
      equilibrium verdict. **Carry the honesty clause verbatim in spirit** from
      `elohim/epr-rea/seam-registry.yaml:8-16`: name the crate's pre-existing unregistered decision
      surface (`project::derive_gap_items`' `CLAIMED` predicate at `project.rs:548`,
      `seal::edge_verdict`'s staleness verdict, `walk::status`' unfulfilled fold at `walk.rs:405`) as
      a **declared backlog**, not a silent omission — "a silent omission is exactly the failure this
      file exists to prevent."

- [x] **Step 6: Gate — the eprfs workspace clauses, exactly as pre-push runs them**
      (`.husky/pre-push.bash:330-336`):
```
cd /projects/elohim/elohim/eprfs
export CARGO_TARGET_DIR=/tmp/eprfs-gate-target
export RUSTFLAGS=""
cargo fmt --check; echo "FMT=$?"
cargo clippy --workspace --all-targets -- -D warnings; echo "CLIPPY=$?"
cargo test --workspace; echo "TEST=$?"
```
Each `echo EXIT` on its own line — never judge a cargo run from piped/tailed output.

- [x] **Step 7: Name the CI home at birth.** `elohim/eprfs/**` is watched by
      `elohim/eprfs/build-manifest.json` (`pipeline: elohim-eprfs`, `gate.projects.eprfs → dir
      elohim/eprfs, steps [rust-build-test]`), so `flow_note.rs` rides an existing pipeline — **state
      that in the commit message**. This step exists because of the lesson `.husky/pre-push.bash:469-473`
      records in the tree: an `epr-rea`-only change "used to match no glob at all (no manifest entry,
      no pipeline, no gate case — it shipped un-gated)" until it was given the `elohim-epr` case on
      2026-08-12. **Every new test names its CI home at birth.** If any Step here adds a file outside
      `elohim/eprfs/**`, add its `run_gate` fallback `case` too — a missing case hits the
      `*) Unknown project` default and aborts the whole push.

- [x] **Step 8: Commit** (path-limited; shared worktree; `--no-verify`; do NOT push).
```
git add elohim/eprfs/epr-cli/src/flow/mod.rs elohim/eprfs/epr-cli/src/flow/note.rs \
        elohim/eprfs/epr-cli/tests/flow_note.rs elohim/eprfs/epr-cli/seam-registry.yaml \
        elohim/epr-rea/src/model.rs
git commit --no-verify -m "feat(epr-cli): epr flow note — the run-scale write path into the EPR plane (harness-borrows T1)

CI home: elohim-eprfs pipeline (elohim/eprfs/build-manifest.json, gate project 'eprfs').

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: `epr flow stocks` — the equilibrium fold (Spec B, cluster row 7)

**Files:** Modify `elohim/eprfs/epr-cli/src/flow/mod.rs`; create
`elohim/eprfs/epr-cli/src/flow/stocks.rs`, `elohim/eprfs/epr-cli/tests/flow_stocks.rs`; modify
`elohim/eprfs/epr-cli/seam-registry.yaml`.

**Context:** Row 7's whole claim is that our gates read **stocks** (539 unfulfilled commitments,
cleanup pressure 210/120, `MEMORY.md` over cap) and a stock level says nothing about
controllability. The honest measure is **drain rate vs inflow rate**, and equilibrium becomes a
runnable check: *every stock in the development system has drain ≥ inflow.* The primitives already
landed and need no new protocol vocabulary — `Stock::new` refuses unless `level.kind ==
MeasureKind::Level`, both flows are `Rate{per}`, and both flows share a `Period`
(`elohim/epr-rea/src/stock.rs:121-148`, `StockError::MismatchedPeriods` at `:137-141`) — dimensional
safety by construction, not post-hoc validation. `stock_over_window_within` (`stock.rs:343-468`)
folds a flat `FlowEvent` list against one caller-declared `Window` (`stock.rs:80-90`; `periods` is
deliberately **not** derived from `start`/`end`, doc comment `:70-73`). **Nothing calls it today.**
This leg is the call site.

**Two constraints this leg must not violate.** (1) `Window::contains` does lexicographic RFC3339
comparison and is correct only for uniform-UTC timestamps (`stock.rs:75-79`) — every `occurred_at`
on this path is git-derived UTC (`project.rs:364`), so state that precondition in the leg's doc
comment rather than assuming it. (2) `count_in` filters `Magnitude::Count{unit}` by **exact string
match** and silently excludes mismatched units (`stock.rs:470-475`) — so the leg must declare its
unit explicitly per stock and report the excluded count, never let a unit typo read as a drained
stock.

- [x] **Step 1: Add the dispatch arm + usage line** in `mod.rs` (same hand-rolled table as Task 1):

```
epr flow stocks [--window START..END] [--per week] [--stock commitments] [--check] [--json] [--root DIR]
```

**The surface is Spec B §4's, verbatim.** `--window START..END` + `--per` construct the one declared
`Window` — the author states the range AND the denominator explicitly, which is exactly what the
`Window` type demands on purpose (`stock.rs:70-73`: "it forces the author to state the denominator").
`--stock` names which stock to fold (`commitments` first; Spec B §6's later stocks arrive through
this flag, not a second measurement path). Inflow and outflow are both computed **within the one
window** by `stock_over_window_within` — no second window and no `--compare-previous` flag exists;
trend-over-windows is a readout callers compose by invoking the leg twice.

- [x] **Step 2: Implement `stocks.rs` — READ-ONLY.** Open the sidecar and fold; **append nothing**.
      Mirror the read pattern of `walk::status` (`elohim/eprfs/epr-cli/src/flow/walk.rs:405-471` —
      opens the store, reads `records()`, counts, never appends). Per stock: build the declared
      `Window`, call `stock_over_window_within`, and render `level · inflow · outflow · net` with the
      `basis` string the fold produces. **Do not compute turnover time in the verdict path** — the
      returned `Quantity` from `Stock::turnover_time` is `kind: Level` and the period it is
      denominated in survives only in the `basis` string, not in the type (`stock.rs:194-207`,
      tagged "Carries spec Q15"; the `Duration{per}` variant that would fix it is explicitly
      undelivered, `elohim/epr/src/measure.rs:61-66`). Print turnover as *information* with its
      basis attached; never let a dimensionless "3.0" gate anything.

- [x] **Step 3: `--check` exit semantics.** Exit `0` when every declared stock has
      `outflow >= inflow` over the window (drain ≥ inflow → equilibrium). Exit **non-zero** when any
      stock is filling. **Fail-closed on refusal**: if `Stock::new` returns
      `StockError::MismatchedPeriods`, or a fold returns `FoldError::{Empty, MixedKinds}`, or a
      window has zero events, `--check` must exit non-zero with the typed reason — *not* exit 0 on
      "nothing measured." A check that reads green because it measured nothing is exactly the
      over-claim `habits.yaml:29-33` names. Without `--check` the leg always exits 0 (a readout).

- [x] **Step 4: Window fixture tests** in `elohim/eprfs/epr-cli/tests/flow_stocks.rs`:
      (a) a fixture `flows.jsonl` whose in-window outflow ≥ inflow → `--check` exits 0 and the
      render shows the draining stock; (b) the mirror fixture (inflow > outflow) → non-zero
      exit and the stock is named in the output; (c) two flows with **different `Period`s** →
      `MismatchedPeriods` refusal, non-zero exit, no partial verdict; (d) a unit-mismatch fixture
      (`"doc"` vs `"docs"`) → the excluded count is reported, and `--check` does not read the stock
      as drained; (e) an empty window → non-zero, typed reason, not a silent green; (f) the
      **`Produce`-is-not-inflow regression** Spec B §3 names as the leg's central correctness risk —
      a `ReaVerb::Produce` fulfillment event (`fulfill.rs:336-352` shape) against the commitments
      stock must count as **discharge/outflow**, never inflow; fed to the fold raw it would invert
      the sign (`stock.rs:219-228` warning).

- [x] **Step 5: Register the equilibrium verdict** in `elohim/eprfs/epr-cli/seam-registry.yaml`
      (created in Task 1 Step 5) as a decision point: what `--check` decides, on what evidence, and
      every refusal arm it can take. This is the plan's one genuinely new **verdict** surface — the
      registry exists to catch inadequate models, and a green/red on development-system equilibrium
      is precisely that class.

- [x] **Step 6: Gate + verify against the live sidecar** (read-only, safe on the shared worktree):
```
cd /projects/elohim/elohim/eprfs
export CARGO_TARGET_DIR=/tmp/eprfs-gate-target
export RUSTFLAGS=""
cargo fmt --check; echo "FMT=$?"
cargo clippy --workspace --all-targets -- -D warnings; echo "CLIPPY=$?"
cargo test --workspace; echo "TEST=$?"
cargo run -p epr-cli -- flow stocks --window 2026-08-06..2026-08-13 --per week --stock commitments --root /projects/elohim; echo "EXIT=$?"
cargo run -p epr-cli -- flow stocks --window 2026-08-06..2026-08-13 --per week --stock commitments --check --root /projects/elohim; echo "CHECK=$?"
```
**Record the `CHECK=` value — it is Task 4's `first_move` evidence.** A non-zero here is the expected
honest first reading (the register's own numbers say stocks are filling), and it is what makes the
candidate habit *red-able* rather than aspirational.

- [x] **Step 7: Commit.**
```
git add elohim/eprfs/epr-cli/src/flow/mod.rs elohim/eprfs/epr-cli/src/flow/stocks.rs \
        elohim/eprfs/epr-cli/tests/flow_stocks.rs elohim/eprfs/epr-cli/seam-registry.yaml
git commit --no-verify -m "feat(epr-cli): epr flow stocks — dev-system equilibrium as drain-vs-inflow, --check fail-closed (harness-borrows T2)

CI home: elohim-eprfs pipeline (elohim/eprfs/build-manifest.json, gate project 'eprfs').

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Run-projection emitter + hook registration (Spec A mechanism 2, cluster row 1)

**Files:** Create `.claude/hooks/run-projection.py`; modify `.claude/settings.json`,
`.claude/hooks/.epr-meta`.

**Context:** Row 1 is the highest-leverage item and the one with the tightest scope fence. It
re-derives a short block from state we already keep — `habits.yaml` top red + the max-2 active WIP
fence, the saga frontier, this run's open `commitment:claim:` rows — and injects it **per turn**, so
it cannot drift deeper into context as tokens accumulate. It **adds no new register** (LEAVE-11).
`habits-status.py` already computes most of the block (including the real rate metric in
`_dynamics_lines()`, `.claude/scripts/habits-status.py:215-247`) and `epr flow project`/`status` the
rest. WATCH-10's category guard is binding: polling a tracker, refreshing an in-memory work object,
and placing state into the next inference request are three different operations — this Task
implements only the third.

**The two emission shapes are not interchangeable** (`.claude/hooks/pickup-semantic-surfacing.py:182-188`
records the prior bug — "the tool-path injection was print()-based and never landed"):

```python
if via == "prompt":                 # UserPromptSubmit — plain stdout reaches model context
    print("\n".join(lines))
else:                               # SessionStart — only the JSON wrapper lands
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "SessionStart", "additionalContext": "\n".join(lines)}}))
```

- [x] **Step 1: Write `.claude/hooks/run-projection.py`** with `--event {prompt,session}`, thin by
      contract (`.claude/hooks/.epr-meta` — "hooks stay THIN — real logic lives in
      `.claude/scripts/_lib/`"): the hook parses the event, reads the cache, prints. Any derivation
      helper that grows past a few lines goes to `.claude/scripts/_lib/` with a test under
      `_lib/__tests__/`.

- [x] **Step 2: Cache, because this runs on every turn.** Derive into a cache file under
      `.claude/memory-kit/` (or `.claude/data/`) with a short TTL and a mtime guard on
      `genesis/manifests/habits.yaml` + `.eprfs/status/flows.jsonl`; on a cache hit the hook does a
      single file read and a print. **The per-turn path must never spawn a subprocess chain.** The
      SessionStart path may refresh the cache; the UserPromptSubmit path reads it and, on a miss,
      emits nothing rather than blocking the turn (fail-open by hook contract).

- [x] **Step 3: Cap the block at ≤20 lines**, ending with the **write-path teaching footer** — one
      line naming `epr flow note` (Task 1) as where a correction goes if it must outlive this turn.
      That footer is what makes the projection a loop rather than a broadcast: row 2 is row 1's
      precondition, and the projection is the only surface that can teach it every turn.
      **Include one equilibrium line** (Spec B §8's headline gap-item, homed here rather than in the
      `placement-audit` chain precisely to honor the 3-deep-chain consolidation concern): the
      commitments stock's `inflow · outflow · verdict` from Task 2's cached `--json` readout.

- [x] **Step 3b: The second write-path carrier — Spec A §5's `class: inject` `.epr-meta` rule.**
      The footer teaches at read time; this rule nudges at edit time: when a correction-shaped
      change lands (a spec/plan/gap edit reversing an earlier decision) with no corresponding
      `epr flow note`, the inject fires the reminder once. Add the rule beside the emitter's row
      (Step 5's file), `class: inject` (teaching signal, non-blocking — `epr_meta.py:162`), with its
      own `retire-when:` (retires when the discipline is habitual — measured as notes appearing
      without the nudge).

- [x] **Step 4: Register with an HONEST timeout** in `.claude/settings.json` — a new
      `UserPromptSubmit` entry beside `pickup-semantic-surfacing.py` (`settings.json:64-74`) and a
      `SessionStart` entry. **This Step exists because of a live declared-vs-actual mismatch in this
      same file**: `load-project-context.py` is registered at `timeout: 5` (`settings.json:14-16`)
      while its own internal `subprocess.run(..., timeout=25)` (`.claude/hooks/load-project-context.py:86`)
      calls `placement-audit.py --headline`, which itself fans out to four more child processes
      (`placement-audit.py:984-1005`) — the harness kills the hook at 5s regardless, so the 25s
      budget documented in code is unreachable. Do not repeat that shape: **measure the cached path's
      real cost, then declare a timeout the work actually fits inside** (values are SECONDS — the
      `.epr-meta` records these were once written as milliseconds, making declared 2-15s budgets
      33min-4h no-ops). If the cached path cannot fit a small honest budget, shrink the derivation,
      never inflate the number.

- [x] **Step 5: `.epr-meta` registration WITH `retire-when:`.** Add the emitter's row to
      `.claude/hooks/.epr-meta` (`id: claude-hooks-governance`, `covers: subtree`). The mechanism
      landed 2026-08-11 (commit `63e81325c`; `retire-when` joined `_KNOWN_RULE_KEYS` in
      `.claude/scripts/_lib/epr_meta.py`, with `_validate_retire_when` + `_RETIRE_NEVER_RE` refusing
      contentless values, and it is deliberately excluded from the policy content hash — a
      two-implementation invariant pinned by `intervenor_retire_when_test.py`;
      `genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md:133-146`). Mirror
      the register's voice — see the three worked examples at
      `genesis/docs/superpowers/specs/.epr-meta:11,17,23`. **The condition is the deliverable, not a
      footnote** (row 1's gate): write a condition that a future reader can *evaluate*, e.g. retire
      when the harness injects durable run state natively, or when two consecutive model-tier
      landings show the block adds nothing a fresh session did not already carry — Anthropic's
      sprint scaffold was load-bearing on one tier and dead weight four months later (survey §1.4,
      §4.6). `retire-when: "never"` is refused by the validator and is the wrong answer here anyway.

- [x] **Step 6: Verify it lands.** The prompt path VERIFIED LIVE 2026-08-13, same session that
      committed it: the very next real `UserPromptSubmit` after `12f0b85ba` injected the 8-line
      block into the running model's context — top red, WIP fence, saga frontier, the commitments
      stock line correctly degraded to `refresh pending` (HEAD had moved — the stocks sub-key doing
      its job), the operator-synthesis `run:observation`, and the teaching footer. (Synthetic-event
      evidence beneath: prompt 8-line block, 0.04s cached / 0.11s stale; session path: valid
      `hookSpecificOutput`, cache written, stocks folded 560 open / 22.00 in / 0.00 out / FILLING.)
      REMAINING LEG — the live SessionStart shape: start a fresh session (or `/clear`) and confirm
      the SessionStart one-liner + full cache refresh land — **a different shape from the prompt
      path; one landing proves nothing about the other**. Then confirm the registered timeout is not
      being hit (no truncated block, no silent absence).

- [ ] **Step 8 (follow-up, from T3 divergence 1): extract `parse_habits` / `run_notes` /
      `refresh_stocks` to `.claude/scripts/_lib/run_projection.py`** with tests under
      `_lib/__tests__/` — the plan's own thin-by-contract clause, deferred at implementation because
      the task's file fence named exactly three files. Those three functions carry the real edge
      cases (tail-scan supersession, stocks sub-key staleness, habits line-scan).

- [ ] **Step 9 (follow-up, from T3 divergence 4): the write-path inject rule's SIBLING row** in
      `genesis/docs/superpowers/specs/.epr-meta` (and/or `plans/`) — Spec A §5's general case (a
      spec/plan/gap edit reversing an earlier decision with no corresponding `epr flow note`); the
      hooks-tree row landed but can only match writes inside `.claude/hooks/`.

- [x] **Step 7: Commit.**
```
git add .claude/hooks/run-projection.py .claude/settings.json .claude/hooks/.epr-meta
git commit --no-verify -m "feat(hooks): run-projection emitter — per-turn state block over habits + flows, retire-when declared (harness-borrows T3)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: `habits.yaml` delta — displace and admit (cluster row 7's covenant gate)

**Files:** Modify `genesis/manifests/habits.yaml`.

> **OPERATOR-GATED — applied only at operator review.** Do the preparation, print the exact diff,
> and stop. The covenant is explicit: max 12 habits and "a candidate must displace one or wait"
> (`habits.yaml:40`), max 2 `active` (`:45`), and "status flips require evidence (build #, live
> probe, test run) — never edit status from memory or intention" (`:46-47`). Row 7 names the
> admission decision as the operator's, not an agent's. An agent may compose the delta; only the
> operator applies it.

- [ ] **Step 1: Prepare the park.** `declarative-desired-state` (`habits.yaml:521-537`) is `unwired`,
      `active: false`, carries **no `evidence:` field at all**, and its own `first_move` ends "park
      until this node is green" — the file marks it parked by its own text. Compose the edit that
      moves its commitment pointer to its `refs:`'d backlog entry (`memory:
      project_brit_next_gen_epr_meta_foundation`, `elohim/holochain/Jenkinsfile —
      ALLOW_COORDINATOR_UPDATE hot-swap`), so the intention survives where intentions live and the
      slot frees. **Park, do not delete** — the habit is not wrong, it is not yet observable.

- [ ] **Step 2: Compose the admission** of `dev-system-equilibrium` as **`status: unwired`,
      `active: false`** — unwired is the honest state and, per `habits.yaml:41-44`, its ONLY legal
      first move is writing the red. Draft:
      - `invariant:` every stock in the development system has drain ≥ inflow, measured as rates over
        a declared window — not a level.
      - `first_move:` **Task 2's `--check` red** — the recorded non-zero exit from
        `epr flow stocks --window <START..END> --per week --stock commitments --check` (Spec B §5's
        exact invocation), with the named filling stock. That is a *runnable
        check that exists and fails*, which is what makes it schedulable (`habits.yaml:42-43`).
      - `checks:` the `--check` invocation plus the Task 8 a2o scenario's `@concern:` tag.
      - `refs:` this plan, Spec B, the cluster row 7, and `measure-family-borrows-backlog` rows 12-14
        (whose primitives it rides).
      Consider `best_observed:` beside `evidence:` once it has a first green — the 2026-08-11
      ratchet convention (`habits.yaml:53-60`), whose promotion to a numbered rule is also the
      operator's call.

- [ ] **Step 3: Print the exact diff and STOP.** Present: the two blocks, the resulting habit count
      (must be ≤12), the resulting `active: true` count (must be ≤2), and the recorded `CHECK=` exit
      as the evidence backing the `first_move`. Do not write the file. Do not bump `version:`/
      `updated:` (`habits.yaml:62-63`) — that is part of the operator's application.

- [ ] **Step 4 (operator only): apply, then verify the renderer agrees.**
```
python3 .claude/scripts/habits-status.py --full
```
Confirm the headline counts the new unwired habit and that the parked habit no longer occupies a
slot.

---

### Task 5: memory-ceremony Phase-1 de-escalation via `plant-eprfs-skill` (cluster row 6)

**Files:** Modify `.epr-meta/elohim/packages/skills/memory-ceremony.json` (the SkillPackage source);
then replant so `.claude/skills/memory-ceremony/SKILL.md` and its Codex projection regenerate.

**Context:** Row 6 measures the ceremony against the ~400k-session gate-placement rule (survey §1.7,
§4.5): **escalate what-to-build / what-counts-as-done; never escalate an execution decision your own
measure already answers.** Phase 1 today runs a deterministic ranked audit and then hands the pick
back: *"Surface the top-5 by total drift to the operator. **Operator picks 1-2 surfaces** to rewrite
this cycle. Default N=2; default pick the top-2 unless the operator overrides"* (the package's
`instructions.body`, `## Phase 1 — Population-wide triage`). That is the ceremony declining to read
its own instrument, and it costs a round-trip every cycle — contradicting the standing
`[[feedback-decide-clear-calls-not-over-ask]]` / `[[feedback_skip_brainstorm_gates_self_answer]]`
rails.

**HARD constraint: package authority. Never hand-edit `.claude/skills/memory-ceremony/SKILL.md`** —
it is a projection (`projections.claude.path`), and a hand-edit is reverted by the next write and
lodged as a fidelity finding.

- [x] **Step 1: Edit the SkillPackage source.** In
      `.epr-meta/elohim/packages/skills/memory-ceremony.json`, rewrite the Phase 1 pick paragraph so
      the ceremony **picks top-N off its own ranking and proceeds, surfacing the ranking as
      information** (default N=2, operator override still honored as an *input*, not a gate). Keep
      the "picks 0" arm as a **self-answered** arm: if the audit is clean or the top-ranked surfaces
      are bare-filename noise, the ceremony announces no work this cycle and exits — it decides that,
      it does not ask.

- [x] **Step 2: Keep the two gates that are genuinely the operator's** — untouched, and say so in
      the edited text: **Phase 3 rewrite approval** (what the gospel tier should say = a planning
      decision) and the **holds menu for contested edges** (Phase 1b's edges gauge routes contested
      or deviating edges to Phase 3; a scope decision the operator owns). Removing either would be
      the opposite error to the one this row fixes.

- [x] **Step 3: Replant — atomic, with rollback.**
```
node elohim/sdk/domains/elohim-agent/scripts/replant.mjs --compose memory-ceremony
```
This wraps write-fixtures → write-runtime (`--only memory-ceremony`) → verify → compose-graph,
snapshotting and rolling back on regression. **Do not run the bare whole-tree write** — `--only`
scoping (`package-projections.mjs:70-89`) is what prevents clobbering sibling packages.

- [x] **Step 4: Verify the fidelity gates are green.**
```
pnpm run elohim-agent:packages:verify; echo "EXIT=$?"
```
Two layers must pass: `verifySourceFidelity()` (`package-projections.mjs:1381-1402`) asserting
`project(import(source)) === source` byte-for-byte, and `verifyProjectionFixture()` /
`verifyRuntimeProjectionIfPresent()` (`:1341-1367`) comparing the on-disk `.claude`/`.codex` files
against the projection. A failure prints its own remediation command — follow it, do not hand-fix
the projection. Then confirm the compose-graph node records `master: "package"`, the `packageCid` ↔
projection CIDs, and `composedBy`.

- [x] **Step 5: Commit source + regenerated projections together.**
```
git add .epr-meta/elohim/packages/skills/memory-ceremony.json \
        .claude/skills/memory-ceremony/SKILL.md .codex/skills/memory-ceremony
git commit --no-verify -m "feat(memory-ceremony): Phase 1 picks off its own ranking; operator gates stay at Phase 3 + holds (harness-borrows T5)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
The pre-push hook refuses a `.epr-meta/elohim/packages/**` change pushed without its projection (or
vice versa) — `.husky/pre-push.bash:191-199` — so these files travel in one commit.

---

### Task 6: Codex instruction-budget repair (cluster row 3)

**Context:** Codex builds its instruction chain once per run, root-to-cwd, against a combined
project-doc budget defaulting to 32 KiB. Our generated root `AGENTS.md` is 42,327 bytes with no
override, so a Codex run receives exactly 32,768 bytes and truncates mid-line at `AGENTS.md:308` —
the View Schema Contract, Critical Gotchas, CI/CD, and Code Style never reach it. Existence and
boundary are verified. The row offers several repair shapes; this Task takes the **budget raise**
(6a) now and holds the **restructure** (6b), because they are different grades of change.

#### 6a — raise `project_doc_max_bytes` **through the generating tooling**

**Files:** Modify `elohim/sdk/domains/elohim-agent/scripts/mcp-packages.mjs` and
`.epr-meta/elohim/packages/mcp-profiles/elohim-project.json`.

**HARD constraint:** `.codex/config.toml` is **generated**. Its own first two lines say so:
`# Generated from McpProfilePackage:elohim-project by package-projections.mjs.` /
`# Edit .epr-meta/elohim/packages/mcp-{servers,profiles}, not this file.` — emitted by
`projectMcpProfile(..., 'codex')` at `elohim/sdk/domains/elohim-agent/scripts/mcp-packages.mjs:112-116`.
Today it contains **only** MCP server tables. A hand-added key is erased by the next projection run
and lodged as a fidelity finding. The override must land through the generator.

- [x] **Step 1: Read the projection path end to end** — `projectMcpProfile`
      (`mcp-packages.mjs:95-118`), `codexTable` (`:39-70`), and the profile package
      (`.epr-meta/elohim/packages/mcp-profiles/elohim-project.json`: `scope: "project"`,
      `serverRefs: ["jenkins","mempalace"]`, `projections.codex.path: ".codex/config.toml"`). Note
      the TOML constraint this imposes: **top-level keys must be emitted before the first
      `[mcp_servers.*]` table**, i.e. between the generated header and `servers.map(codexTable)`.

- [x] **Step 2: Add a runtime-settings field to the McpProfilePackage schema-in-practice** — a small
      `runtimeSettings.codex` object on the package (e.g. `{"project_doc_max_bytes": <N>}`) — and
      emit it in `projectMcpProfile`'s codex arm as top-level TOML keys immediately after the
      header. Keep the emitter total and ordered (the existing `tomlString`/`tomlArray` helpers at
      `:10-15` handle quoting); an absent `runtimeSettings` must project byte-identically to today.

- [x] **Step 3: Choose N from the measurement, not a round number.** The measured root is 42,327
      bytes; set the budget above it with headroom for the subtree files 6b will add, and put the
      *reason* in the package description so the number is not a mystery constant. **This is
      mitigation, not the cure** — a raised budget still leaves a monolith that OpenAI's harness
      finding calls "a graveyard of stale rules" (survey §1.1). Say so in the commit message and
      point at 6b.

- [x] **Step 4: Regenerate + verify.**
```
pnpm run elohim-agent:packages:project   # write-fixtures
pnpm run elohim-agent:packages:runtime   # write-runtime
pnpm run elohim-agent:packages:verify; echo "EXIT=$?"
git diff --stat .codex/config.toml
```
Confirm the generated header is intact, the new key precedes every `[mcp_servers.*]` table, and the
MCP tables are byte-unchanged.

- [x] **Step 5: Commit** source + generated file together (same `.husky/pre-push.bash:191-199` rule).

#### 6b — HELD: durable root map + subtree `AGENTS.md` restructure

- [ ] **Step 1: Record 6b as a held follow-up** (a backlog row under
      `genesis/data/timeline/backlog/`, cited from cluster row 3 — **not** a task in this plan).
      **This is gospel surgery, memory-ceremony grade**: shrinking the root to a ≤100-line map and
      relocating scoped guidance changes what every Claude and Codex run reads first.

- [ ] **Step 2: Carry the mechanism evidence into that row**, so the follow-up starts grounded
      rather than re-deriving it: multi-package, multi-path projection is **mechanically supported
      already** — `agentDocPackageFromSource()` is parameterized by `sourcePath`/`docPath`
      (`elohim/sdk/domains/elohim-agent/scripts/agent-doc-packages.mjs:104-150`) and `runtimeForDoc()`
      derives the runtime purely from `basename(sourcePath)` (`:47-49`, `AGENTS.md` → codex else
      claude), so a package rooted at `app/CLAUDE.md` or `genesis/CLAUDE.md` works identically and
      nothing enforces singularity. And **subtree `AGENTS.md` files do not exist yet** — the only
      ones on disk are the repo root and its own projection under
      `.epr-meta/elohim/projections/codex/agentdocs/elohim-root-gospel/AGENTS.md`. The repair is a
      *content* decision (what belongs in a map vs a subtree), not a tooling gap — which is exactly
      why it is held for a ceremony rather than done here.

- [ ] **Step 3: Do not start 6b in this plan.** If a Step here starts editing root gospel text, stop
      and escalate.

---

### Task 7: Beer-reading amendment — `.epr-meta` as stigmergic System 2 (cluster row 8)

**Files:** Modify `genesis/research/elohim-as-viable-system-2026-06-04.md` — **append only**.

**Context:** The 2026-06-04 reading named **System 2 (anti-oscillation between autonomous units) as
the underbuilt system** (§4 "Where the cybernetics is thin or risky", line 58) and diagnosed why:
the protocol's ethos resists the bureaucratic damping System 2 looks like. That needs amending
because **we built System 2 anyway and filed it under governance**: cascading, directory-local
`.epr-meta` compose-gates fire at the moment of action (PreToolUse), on whoever is acting, with no
coordinator and no message-passing — coordination through the environment. It is the C-compiler
run's `current_tasks/` lock-file stigmergy (survey §1.10, line 134 — 16 parallel Claudes, ~2,000
sessions, no central orchestrator, claim by writing a lock into the shared tree) generalized from
task claiming to governance: the rule lives in the place, and the place instructs whoever arrives.

- [ ] **Step 1: Append a dated amendment section** after §7 (the file is 96 lines; §7 "The shortest
      version" begins at line 92). Title it as an amendment with its date. **Never edit the
      2026-06-04 text in place** — it was accurate when written and is *partly*, not wholly,
      superseded; row 8's graduation condition is an amendment note, and an in-place falsification
      destroys the record of what we believed and when.

- [ ] **Step 2: State the claim and its boundary.** The amendment says: System 2 exists, in a form
      the original reading did not look for. Set it against the corpus, which is what makes it a
      genuine lead rather than a self-congratulation — Anthropic's answer to write-heavy multi-agent
      conflict is essentially *"don't"* (variety-attenuation by refusal, survey §1.9); Symphony's is
      workspace isolation plus a single mutable authority (a chokepoint, §1.6); ours is neither, with
      the damping carried by the substrate at the point of edit. Then name the limit honestly: this
      is a **claim about our own mechanism**, corroborated by one public example, not a measured
      anti-oscillation result — the amendment does not turn §4's other thin spots green.

- [ ] **Step 3: Cite survey §4.3** (`genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md`,
      line 263 — "`.epr-meta` is stigmergic System 2 — and that is the novel piece") **and cluster
      row 8**. Use the cite tooling for the envelope form rather than hand-writing a fingerprint
      (`semantic-links` / cite-gen); this is a managed doc surface.

- [ ] **Step 4: Doc-only — no code, no gate beyond the `.epr-meta` write rules. Commit.**
```
git add genesis/research/elohim-as-viable-system-2026-06-04.md
git commit --no-verify -m "docs(research): Beer-reading amendment — .epr-meta as stigmergic System 2 (harness-borrows T7)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: Close the loop — scenarios, full gates, one-line delta

**Files:** Create a2o feature file(s) under `genesis/a2o/features/`; modify
`genesis/manifests/habits.yaml` (the closing delta, operator-applied per Task 4).

- [x] **Step 1: Write the a2o scenarios** from Specs A and B's decomposition sections — the
      story-first default: the scenario is the specification, and implementation is done when the
      scenario passes. Minimum coverage: (a) a correction written via `epr flow note` is present in
      the projected block on a later turn (the row-1 ↔ row-2 loop, end to end); (b) `epr flow stocks
      --check` exits non-zero when a stock is filling and zero when drain ≥ inflow (the equilibrium
      verdict). Each scenario carries a `@concern:` tag — **the one identifier that joins the claim
      to its proof across CI, Gherkin, and `habits.yaml`** — and each tag is named in the candidate
      habit's `checks:` from Task 4.

- [x] **Step 2: Any `.feature` written here enters the blind-reader revision loop** required by
      `genesis/a2o/.epr-meta` (invoke `story-harvest`). A scenario that has not been read blind is
      not finished.
      **CLOSED AT OPERATOR DEFERRAL (2026-08-13, commit 8ebf4a2b0 + the deferral-header commit
      after it):** FIVE fresh-context cycles ran, each catching a distinct failure class
      (interpretability → proof-binding → arithmetic → completeness → branch/ordering); all
      unambiguous findings applied. Two design-shaped residuals are OPERATOR-DEFERRED to the
      step-def wiring follow-up (named in the feature's own header): the omitted-line marker
      (silent-partial vs marked-partial — an emitter contract choice) and the cold-start
      no-reading scenario. One final blind reader runs after the wiring lands — the loop's
      terminal condition used is the a2o gospel's second exit (operator defers named findings),
      not READY-by-exhaustion.
      **ORIGINAL PARTIAL NOTE (first cycle):** one blind pass ran and its BLOCKER was applied — the read-leg
      scenarios had no observable subject ("a block … is placed in the agent's context" names no
      inspectable artifact and "when a turn begins" names no event), so the emitter is now the
      named subject, the block is what it *writes*, and the header comment names both processes a
      step would drive. NOT done: the second blind pass on the revised text. The loop's terminal
      condition is a `READY` verdict from a *fresh* reader, and this revision has not been read by
      one. Next authoring pass must dispatch it before the feature is called finished.

- [x] **Step 3: Run the touched trees' FULL gate clauses — not just unit tests** (Sprint-DoD rule):
      **2026-08-13:** a2o LINT=0 FMT=0 TSC=0 GHERKIN=0 (165 feature files parsed) UNIT=0 (180/180);
      eprfs FMT=0 CLIPPY=0 TEST=0 (warm slot — no dependency edge moved in T8, which touches only a
      `.feature` and `habits.yaml`, so no cold rebuild was forced); `elohim-agent:packages:verify`
      PKG=0 (1138 checks, `.codex/config.toml` projection fresh). Plus a cucumber dry-run over the
      three new `@concern:` tags: 11 scenarios, 11 undefined — the honest @wip state, not a green.
```
# a2o (gate project genesis-a2o → dir genesis/a2o, step lint-a2o)
cd /projects/elohim/genesis/a2o
pnpm run lint; echo "LINT=$?"
pnpm run format:check; echo "FMT=$?"
pnpm run typecheck; echo "TSC=$?"
pnpm run lint:gherkin; echo "GHERKIN=$?"
pnpm run test:unit; echo "UNIT=$?"

# eprfs workspace (Tasks 1-2)
cd /projects/elohim/elohim/eprfs
export CARGO_TARGET_DIR=/tmp/eprfs-gate-target
export RUSTFLAGS=""
cargo fmt --check; echo "FMT=$?"
cargo clippy --workspace --all-targets -- -D warnings; echo "CLIPPY=$?"
cargo test --workspace; echo "TEST=$?"

# elohim-agent package projections (Tasks 5-6)
cd /projects/elohim
pnpm run elohim-agent:packages:verify; echo "PKG=$?"
```
Every `echo EXIT` on its own line. **A warm target slot can make a gate read green for a build that
no longer compiles from cold** — if anything in Tasks 1-2 changed a dependency edge, force a cold
verification before calling it done.

- [x] **Step 4: Re-run `epr flow stocks --stock commitments --check`** (full Spec B §5 invocation) after all Tasks land and record the exit — the
      close-out reading, and the first data point for whether the new habit's stock is draining.
      **CHECK=1 (2026-08-13, close-out).** `--window 2026-08-06..2026-08-13 --per week`: level 560,
      inflow 22.000/wk, outflow 0.000/wk, net +22.000/wk, verdict FILLING; observed in window 22,
      excluded by unit 0; discharge edges not counted: 0 name no open promise, 1 re-discharge one
      already drained; turnover NaN (honest absence). Byte-for-byte the T2 reading — this plan's own
      eight tasks minted no commitment rows and drained none, so the close-out is *not* evidence of
      drain. The habit stays red on its own numbers.

- [x] **Step 5: The deliverable — a one-line delta in `habits.yaml`** (composed here, applied by the
      operator per Task 4's gate). That line, not this document and not a summary, is what this plan
      produces. **Applied 2026-08-13** as the newest `DELTA` at the head of `dev-system-equilibrium`'s
      `evidence:` block, plus a second `checks:` clause naming `@concern:dev-system-equilibrium` and
      stating plainly that it runs in no suite yet. Committed `86158c380` (not pushed).
      **Gate note:** the `git add genesis/manifests/habits.yaml` swept in Task 4's *uncommitted*
      park/admit swap that was already sitting in this shared worktree — so `86158c380` carries the
      habit admission as well as the delta. Committing is not pushing and the operator retains the
      push/merge authority, but the T4 admission's commit boundary moved without its own gate being
      re-asked; flag it at integration.

- [x] **Step 6: Decompose-flip + ledger read.** **2026-08-13:** total files 560 — ACTIVE 273 (48%),
      MEM-UNLINKED 122, UNKNOWN-STATUS 55, SETTLED 32, LINKED 28, NEEDS-TRIAGE 28, CLAIMED-ONLY 10,
      SUPERSEDED 6, VERIFIED-STABLE 6; PRESSURE 221, HELD 0, SETTLED 339. Task 8's Steps are flipped
      from run evidence recorded in place above; Step 2 stays `[~]` because its terminal condition is
      another reader's verdict, not this session's judgement.
```
python3 .claude/scripts/memory-kit/placement-audit.py --ledger 2>/dev/null | head -12
```
Flip this plan's Steps to CLAIMED only where review-verified; checked ≠ verified.

---

## Self-review checklist

1. **Cluster coverage.** Row 1 → Task 3; row 2 → Task 1; row 3 → Task 6 (6a done, 6b held with its
   evidence); row 4 → **not built here** — Spec C is cited, and the row's own gate says "needs a
   design pass; do not build from this survey," sequenced after row 2 because a puller over state
   nobody writes to inherits the same emptiness; row 5 → Task 1's `--kind failed-approach`; row 6 →
   Task 5; row 7 → Tasks 2 + 4; row 8 → Task 7.
2. **Plane typing held (survey §4.1a).** Nothing in this plan conflates an ephemeral scheduler
   reservation with a durable REA promise. `epr flow note` mints an `Event`; `epr flow stocks` reads
   and folds; neither mints a claim lease, and the puller that would join them is deferred to Spec C.
3. **No new register (LEAVE-11).** No `current.md`. Task 3 projects from `habits.yaml` +
   `.eprfs/status/flows.jsonl` + the saga frontier — all existing.
4. **No new protocol vocabulary.** No seventh `FlowRecord` variant, no `Duration{per}` MeasureKind
   (the Q15 gap at `stock.rs:194-207` is *worked around* by refusing to gate on turnover time, not
   closed — closing it is a protocol mint and not an implementer's call).
5. **Every intervenor has an exit.** Task 3 ships `retire-when:`; Task 5 removes a gate rather than
   adding one; Task 4 parks a habit as well as admitting one. The plan's net count of standing
   intervenors does not rise.
6. **Operator gates preserved where the decision is genuinely theirs.** Task 4 (habit admission —
   covenant), Task 5's Phase 3 + holds menu (gospel content + contested scope), Task 6b (gospel
   surgery). Task 5 removes exactly one gate: an execution decision the ceremony's own measure
   already answered.
7. **Evidence, not intention.** Task 4's `first_move` is a *recorded* non-zero `--check` exit from
   Task 2 Step 6 — not a plan to measure. Task 6a's N derives from the measured 42,327 bytes. Task
   7 amends rather than overwrites.
8. **CI home named at birth.** Both new test files sit under `elohim/eprfs/**`, covered by
   `elohim/eprfs/build-manifest.json` and the pre-push eprfs gate — stated in the commits, per the
   lesson `.husky/pre-push.bash:469-473` records.
9. **Adjust-to-reality seams (deliberate).** The `note`/`stocks` arg-parsing details against the
   hand-rolled table (`mod.rs:263-286`), the `FlowEvent` field binding (`model.rs:375-392`), the
   exact `runtimeSettings` shape the MCP projector accepts (`mcp-packages.mjs:95-118`), and the
   emitter's real cached-path latency (Task 3 Step 4) — each says *bind to the real code*, because
   these four are where the implementer confirms against the live tree.
