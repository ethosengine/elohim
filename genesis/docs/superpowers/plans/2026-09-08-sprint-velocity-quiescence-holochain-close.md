---
id: sprint-velocity-quiescence-holochain-close
title: Sprint 2026-09-08 → 09-14 — velocity, peer-driven quiescence, Holochain integration close
status: IN_PROGRESS
class: protocol-canonical
actor: agent:orchestrator@fable-5.1
habits: [dataplane-convergence, runtime-upgrade-propagation, operator-runtime-surface, runtime-death-witnessed, happ-lineage-migration]
commits: []
cites:
  - "holons-are-spaces-how-we-use-holochain | Architecture grounding for the sprints Holochain integration close (D-D/D-C) — spaces/holons framing this sprint budgets and contracts against. | sha256:ac1de36d2423be82 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "ai-stewarded-commons-reimplementation-plan | T12 appends the chosen decision-experiment budgets to this plans section 8, finishing the re-evaluated Holochain integration strategy. | sha256:0d5f1300b5d615dc | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "accountable-correction-contract | T7a/T7b implement stations 1-2 and 4 of this contract; T13 designs a companion D6 contract beside it. | sha256:691e8b89f394214c | path: genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md"
  - "ratchet-to-delivery-dataplane-sdk-lanes | Execution-scaffold ratchet spec this sprints batch/push-gate cadence and habit-delta discipline follow. | sha256:162f2cde07f0de8e | path: genesis/docs/superpowers/specs/2026-08-28-ratchet-to-delivery-dataplane-sdk-lanes-design.md"
  - genesis/data/timeline/backlog/runtime-steward-adoption-of-fleet-cut-release.md
  - genesis/data/timeline/backlog/feedback-discovery-sweep-is-o-n-in-history.md
  - genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md
  - genesis/data/timeline/backlog/rung5-p2p-propagation-of-tonights-coordinators.md
  - genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md
  - elohim/holochain/.epr-meta/happ-lineage-migration.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - elohim/elohim-storage/.epr-meta/operator-runtime-surface.habit.md
  - elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md
  - elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md
---

Runtime copy of the approved plan at /projects/.claude-config/plans/melodic-riding-goblet.md; this file is the repo home.

# Sprint 2026-09-08 → 09-14: velocity, peer-driven quiescence, Holochain integration close

## Context

The overnight shifts of 2026-09-06/07 landed a large batch (accountable correction slice 1,
delegated compute, fixtures-clone harness, toolchain 0.7 closure) and left ten unpushed
commits on `dev`. The hand-off note records the mesh dying to background-task reaping; the
RAM guard shed six `cargo test` runs in the same window; one whole mesh round was lost to a
missing binary discovered only after `start` had begun. The sealed Holochain decisions
(D0–D10) fix the first slice as "accountable correction converges" and defer the
keep/deepen/replace decision to a budgeted experiment. Rung-5 p2p upgrade propagation works,
but the workspace peer cannot cleanly adopt a fleet-cut release and could not run its next leg
because the household mesh held ports 8090/8888 all night.

Operator ranking criteria (2026-09-07): (1) increase development velocity; (2) leverage peer
relationships to drive high-performance p2p quiescence and DHT efficiency, finishing the
re-evaluated Holochain integration strategy; (3) reduce wall-clock for every later session.

Operating posture: Fable plans, delegates, reviews composition; Opus/Sonnet/Codex do the
labor; every batch ends at a harvestable push (one CI pass per batch, never during a rolling
edge deploy). Tasks are written so a fresh agent executes one without this conversation.
This plan was fact-checked by an Opus reviewer against the tree on 2026-09-07; the
corrections are folded in below (marked ⟲ where they changed a task).

## Ranking method (evidence only)

| Axis | Question |
|---|---|
| Compounds | pays back on every later session, or once? |
| Habit | which red it moves, or whose delta it writes (WIP fence: `dataplane-convergence`, `runtime-death-witnessed` active) |
| Unblocks | how many sprint items sit behind it |
| Owner | agent-doable now, or operator-owned |

## Decisions taken here (executors do not re-litigate)

- **D-A. Fleet-cut adoption:** cure **(b)** from
  `genesis/data/timeline/backlog/runtime-steward-adoption-of-fleet-cut-release.md`: "I have
  adopted" means "I run the target bytes". Cure (a) binds adoption to a mode flip; rejected.
- **D-B. Feedback discovery sweep:** cures **(a)+(b)** from
  `feedback-discovery-sweep-is-o-n-in-history.md` (cold-retire after K clean sweeps, re-arm on
  notification or new act reference, hot-first order). `MAX_MEMBERS_PER_SWEEP` stays 8. This is
  the "peer relationships drive quiescence" item: a peer's notification is what re-arms
  discovery, and cold peers cost nothing.
- **D-C. D6 (cell-qualified projection/sync)** is design-only this sprint (a contract, gated by
  `p2p-design-gate`). Implementation waits for slice 1 (D10).
- **D-D. Holochain decision-experiment budgets** get chosen numbers this sprint from measured
  household evidence, recorded in the reimplementation plan §8, operator-ratified. Choosing
  them is what "finishes" the re-evaluated strategy at this stage.
- **D-E. First "Jenkins stage on peers"** = an a2o scoped feature run packaged as a compute
  task, executed by a household provider, result attested and read back. Cargo gate on a peer
  is the stretch.
- **D-F ⟲. DNA hash path-independence is held.** `--remap-path-prefix` in CI moves every DNA
  hash (the DNA justfile records the 2026-09-02 lesson: a CI RUSTFLAG "moved every DNA hash in
  CI and was removed"). Per sealed D5, hash-moving changes fold into the next integrity crossing
  already needed. This sprint only reproduces and files the atom (T9a).
- **D-G ⟲. Velocity items bind to the habit whose measurement they gate** (`dataplane-
  convergence` for mesh ergonomics), never to `dev-system-equilibrium` (its checks are REA
  commitment-stock rates; its own guard names wall-clock deltas as the over-claim).

---

## Batch 1 (days 1–2): unblockers, local-gate verifiable, one push

Harvest: every later mesh session starts detached and fails fast; the seeder writes; the
workspace peer adopts fleet-cut releases; the T3 peer runs beside the mesh.

### T1 ⟲ — `just mesh preflight`, detached `start`, fail-fast `wait`
- **Tier:** Sonnet. **Habit:** `dataplane-convergence` (delta: start→ready wall-clock).
  **Compounds:** every mesh session; the 2026-09-07 round `mesh-20260907000336Z` was lost
  entirely to a missing doorway binary found only after start began.
- **Evidence:** `app/elohim-app/scripts/hc-mesh.sh:2630 start_all()` setsid-nohups each child
  but blocks inline on four readiness ladders (mongod 20×1 s, doorways 20×1 s, conductors
  90×3 s, storage 30×2 s); ~6 refusal paths use bare `exit 1`; pid files are listener-derived
  (`record_listener_pid`/`refresh_mesh_pidfiles`, `$MESH_DIR/pids`), so detaching is pid-safe.
  No `wait`/`ready`/`preflight` verb exists (`justfile:236-249`).
- **Files:** `hc-mesh.sh`, `justfile` (mesh recipe).
- **Do:** (1) `preflight`: assert the four binaries (holochain fork pair, elohim-storage,
  doorway, iroh-relay), fork-pair pin = submodule pin12, transport/iroh feature marker, and
  free ports (4444+10i, 4445+10i, 8090+i, DOORWAY_PORT, DOORWAY_B_PORT, 3340, 27017); print
  each as `ok`/`REFUSED <reason>`; exit 1 on any refusal. (2) `start` runs `preflight`, then
  `setsid nohup start_all > $MESH_DIR/logs/start.log 2>&1 &` and returns ≤ 5 s printing the
  log path and `just mesh wait`. (3) `wait [--timeout 900]` polls the same ladders AND exits 1
  immediately when `start.log` gains a `REFUSED|exit 1|missing` line, printing that line.
  `MESH_FOREGROUND=1` keeps today's inline behavior.
- **Verify:** from a 60 s tool call, `start` returns; `wait` reaches ready; kill the calling
  shell's process group, `just mesh status` still UP; rename the doorway binary, `preflight`
  refuses in < 3 s and `wait` (if forced) exits 1 in < 10 s. Record start→ready seconds.
- **Reviewer:** Haiku reads the four logs.

### T2 ⟲ — T3 workspace peer runs beside the household mesh
- **Tier:** Sonnet. **Habit:** `runtime-upgrade-propagation`. **Unblocks:** T9.
- **Evidence ⟲:** `hc-start.sh` accepts `STORAGE_PORT` (:93, default 8090) and
  `DOORWAY_PORT` (:97, default 8888); `CONDUCTOR_APP_PORT` (:199) already defaults to 4485 for
  `join-alpha`; the admin port is discovered from the sandbox log (`get_admin_port`, :355), so
  it does not collide. Sandboxes root at the in-repo `elohim/holochain/local-dev` (:79-80),
  shared with the mesh peers; `MESH_DIR` is not read by hc-start.sh.
- **Files:** `hc-start.sh`, `justfile` (`dev conductor`).
- **Do:** when the mesh pid dir shows peers UP, `just dev conductor alpha` defaults
  `STORAGE_PORT=8095 DOORWAY_PORT=8898` and a sandbox name `t3-<profile>` under `local-dev/`
  distinct from any mesh peer; print the chosen ports and sandbox; refuse on collision naming
  the port. No new admin-port plumbing.
- **Verify:** mesh UP, `just dev conductor alpha` joins alpha (agent-info listed via
  doorway-alpha `/db/p2p/conductor-diagnostics`), `just mesh status` still three UP. Receipt
  line under `genesis/a2o/reports/workspace-release/2026-09-08/`.
- **Reviewer:** Haiku.

### T3 ⟲ — Seeder sends `reach`
- **Tier:** Sonnet. **Habit:** `dataplane-convergence`. **Unblocks:** prologue seed, the
  fixtures-clone seeder leg (not T7; see below).
- **Evidence ⟲:** payload at `genesis/seeder/src/seed-production.ts:431-447`, nine fields, no
  `reach`; `CreateContentInput.reach: String` with no serde default at
  `elohim/sdk/domains/lamad/types/src/lib.rs:39`.
- **Do:** carry `reach` from the seed row (default = the corpus manifest's declared reach,
  never a literal). Failing→passing seeder unit test.
- **Verify:** `just seed validate`; `just mesh prologue` on the running mesh writes the base
  corpus with zero `WasmError Deserialize` lines; success count in the habit delta.
- **Reviewer:** Sonnet (different agent) reruns the prologue.

### T3b ⟲ — `step` relationship vocabulary (schema-owned, multi-site)
- **Tier:** Sonnet, same agent as T3, sequential (shared seeder tree).
- **Evidence ⟲:** `prologue-seed-step-relationship-type-invalid.md`; sites: `seed.ts`
  ~:1723-1897 (~10 uses), `relationship-vocabulary.ts`, `VALID_RELATIONSHIP_TYPES` in
  `elohim/elohim-storage/src/.../relationship_service.rs:313`. The atom says schema-owned.
- **Do:** add the vocabulary at the schema source of truth (`elohim/sdk/domains/lamad/
  manifest.json` relationships, then `pnpm run lamad:codegen`), so Rust and TS both accept it;
  do not hand-edit generated files.
- **Verify:** codegen freshness (pre-push check), storage gate, prologue with zero
  `relationship type invalid` lines.
- **Reviewer:** Haiku diff-checks that only generated files changed downstream.

### T4 ⟲ — Fleet-cut adoption: "runs the target bytes" counts as adopted (D-A)
- **Tier:** Sonnet (TS) + Sonnet (storage route), or one Sonnet sequentially.
  **Habit:** `runtime-upgrade-propagation`. **Unblocks:** T9.
- **Evidence ⟲:** the publish-time check is TypeScript: `genesis/a2o/scripts/
  release-ceremony.ts:424-473, 611-631` (`not_adopted`, reads `appliedRelease.cid` from
  `/admin/adoption`). `already_runs_target` exists in Rust at
  `elohim/elohim-storage/src/services/release_adoption/verify.rs:761` but is unreachable from
  the ceremony.
- **Do:** expose `installedCoordinatorHashes` (or reuse `already_runs_target` as a boolean
  `runsTarget` for the candidate CID) on `/admin/adoption`; the ceremony's adopted check
  accepts `runsTarget === true` for a release whose `appliesTo` was cut for other peers.
  Unit test both sides (Rust route; TS ceremony with a fixture from
  `genesis/a2o/reports/workspace-release/2026-09-06/` it8 JSON).
- **Verify:** `just gate elohim-storage` (`EXIT=$?`), a2o script tests; replaying it8 no longer
  emits `coordinator_lineage_mismatch`.
- **Reviewer:** Opus (rust-architect) reads for a false-positive adoption path.
- **Write-set note ⟲:** touches `genesis/a2o/scripts/`; T9/T10 (batch 3) also live in
  `genesis/a2o` and `genesis/agentic`; batches are sequential so no collision, but T10 must
  rebase on T4.

### T5 — Delegated compute small bugs
- **Tier:** Sonnet. **Habit:** `operator-runtime-surface`.
- **Evidence:** `compute-executor-cid-refuses-null-retention-field.md`
  (`elohim/rakia/rakia-executor/src/{main.rs:59,contract.rs:142}`, raw `serde_json::Value`
  hits `ipld-core serialize_unit`); `compute-payload-store-expiry-test-flake.md`
  (`compute_payload_store.rs:260`, one-off `NotFound` after `put`).
- **Do:** (a) CID the typed contract; null → absent before hashing; test both spellings hash
  equal. (b) name the put→get race mechanism; fix or record it in the atom; no `#[ignore]`.
- **Note:** (a) edits inside the `elohim/rakia` submodule: commit there, do not push, list the
  pin bump for the operator (see operator list).
- **Verify:** executor tests; `cargo test --test compute_payload_native` 15/15 three runs.
- **Reviewer:** Haiku (a); Sonnet (b).

**Batch 1 push gate:** all tasks green locally with `just gate` per touched project; habits
re-projected (`.claude/scripts/habits-project.py`); one push; ci-observer summarizes the
pass. Not while an edge deploy rolls.

---

## Batch 2 (days 2–4): the quiescence and wall-clock measures, one push

### T6 — Feedback discovery sweep: cold-retire, notification re-arm, hot-first (D-B)
- **Tier:** Opus (`rust-architect`). **Habit:** `dataplane-convergence`. **Compounds:** every
  mesh round (baseline 107→333 s across 2026-09-07 rounds).
- **Evidence ⟲:** `elohim/elohim-storage/src/services/feedback_projector.rs`:
  `SWEEP_INTERVAL_SECS = 60` (:63), `MAX_MEMBERS_PER_SWEEP = 8` (:70), `publish_generation`
  (:617), rotation :780, publish :963; re-arm entry point `admit_notified_signal` (:1389);
  tests in `feedback_projector/tests.rs`.
- **Do:** member state {hot, warm, cold}; no open/unapplied acts after K=3 clean sweeps →
  cold, out of rotation; `admit_notified_signal` naming the member, or a new act reference,
  re-arms hot; order = last-new-act desc. Unit tests per transition plus the race (signal
  arriving mid-sweep). Budget constants unchanged.
- **Verify:** `just gate elohim-storage`; mesh run of
  `genesis/a2o/features/dataplane/accountable-correction.feature` with receipt; sweep-to-
  convergence seconds before/after on the same corpus; `get_links` per sweep not increased.
- **Reviewer:** Opus (different agent) on the re-arm race.

### T7a ⟲ — Accountable correction stations 1–2: link-only coordinator extern + notification-driven wake
- **Tier:** Opus (`rust-architect`). **Habit:** `dataplane-convergence` (top red).
  **Depends:** T6 (wake reuses `admit_notified_signal`). Does **not** depend on T3.
- **Evidence ⟲:** `grep -c 'pending()'` in the step defs = 0; stations 1–3 stop on named
  product gaps, not missing steps: no link-only coordinator extern
  (`elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs:168-192`), no
  notification-driven wake, no named dependent view (station 3). Receipts live in the
  gitignored worktree `.claude/worktrees/accountable-correction/` and must be copied under
  `genesis/a2o/reports/` for the delta.
- **Do:** add the link-only extern in `content_store` (coordinator-only: DNA hash must not
  move; verify with `hc dna hash` before/after); wire the wake path so a feedback signal
  re-arms the member (T6) and station 2's "notification accelerates, never substitutes"
  measures the delta between notified and swept discovery.
- **Verify:** stations 1 and 2 PASS on the mesh in one receipt; station 1 with notifications
  disabled; DNA hash unchanged.
- **Reviewer:** Sonnet reruns from a clean mesh.

### T7b ⟲ — Station 4 (crash window) after T6 re-budgets the harness deadline
- **Tier:** Sonnet. **Habit:** `dataplane-convergence`. **Depends:** T6.
- **Evidence ⟲:** best round `mesh-20260907T045232Z/run.log:36`: `projector consumes crash
  arm: deadline exceeded` (`helpers.ts:154`, `steps.ts:394`), a harness poll deadline derived
  from `MAX_MEMBERS_PER_SWEEP`/`SWEEP_INTERVAL_SECS`, not the outbox.
- **Do:** derive the station 4/7 deadlines from the live peer env after T6 (a cold member's
  worst case is now bounded by the re-arm, not the rotation); assert the crash window against
  that bound.
- **Verify:** station 4 PASS twice in a row. Sprint target: **7/8** (station 3's dependent view
  is a frontend deliverable, see stretch). 8/8 is stretch.
- **Stretch:** T7c station 3 dependent view (angular-architect, Sonnet) if a slot frees.

### T8 ⟲ — `cargo test` memory shed: measure the phase, then cap it via a manifest key
- **Tier:** Opus measure (read-only) → Sonnet implement. **Habit:** `dataplane-convergence`
  (its checks run the storage gate); numbers also filed as a backlog atom.
- **Evidence ⟲:** RAM guard shed six tier-1 cargo runs at 7.6–13.7 GB; `pool-policy.json`
  `max_concurrent_heavy: 1`, `default_jobs: 4`; `gate-runner.mjs:39-49` passes exactly four
  cargo args and `manifest.schema.json:155-165` allows only those keys, so a new key needs
  `manifest.schema.json`, `gate-runner.mjs`, `run-local-gate.sh:42`, `gate-runner.test.mjs`.
- **Do:** measure one storage gate with `/usr/bin/time -v` per phase under `CARGO_BUILD_JOBS`
  4 vs 2 and `RUST_TEST_THREADS` 4 vs 2; add a `cargo.env` map key (schema + runner + local
  script + test) and set the winning values in the storage manifest cargo block.
- **Verify:** three consecutive `just gate elohim-storage` runs with no shed line; peak RSS
  and wall-clock recorded; wall-clock regression ≤ 15 %.
- **Reviewer:** Haiku reads the three logs.

### T9a ⟲ — DNA hash path-dependence: reproduce and file, do not fix (D-F)
- **Tier:** Sonnet. **Habit:** `happ-lineage-migration` (atom cites it).
- **Evidence ⟲:** run note only (flows.jsonl); no `.cargo/config.toml` under
  `dna/elohim/`; `dna/elohim/justfile:7` and the DNA `Jenkinsfile:154,857` export RUSTFLAGS
  as env, so config.toml would never be read, and a CI RUSTFLAG change moves every hash.
- **Do:** pack `dna/elohim` from `/projects/elohim` and from a worktree; diff `hc dna hash` and
  integrity wasm sha256; file `genesis/data/timeline/backlog/dna-hash-path-dependent.md` with
  the numbers, the remap remedy, and "rides the next integrity crossing (D5)".
- **Verify:** the atom exists with both hashes; no build config touched.

**Batch 2 push gate:** T6/T7a/T7b receipts under `genesis/a2o/reports/`; T8 numbers; one
push. No `[build:dna]` unless T7a's extern is confirmed hash-neutral (it should be, and the
coordinator hot-swap heals the fleet without a DNA reinstall).

---

## Batch 3 (days 4–6): fleet harvest runs and the first peer-executed stage

### T9 — Rung-5 leg: T3 peer publishes a lamad single-role candidate; james adopts by election; rollback returns baseline
- **Tier:** fork (needs context) or Sonnet with the recipe. **Habit:**
  `runtime-upgrade-propagation`. **Depends:** T2, T4, edge deploy quiet, and T7a's coordinator
  bytes on `dev` (a real delta to propagate).
- **Recipe:** memory `project_workspace_to_fleet_first_crossing_2026_09_06` +
  `rung5-p2p-propagation-of-tonights-coordinators.md`: `just dev conductor alpha` (fork bin),
  `coordinator-candidate.ts --role lamad --applies-to-from-adoption
  <doorway>/db/p2p/adoption?peer=james`, publish, expect `coordinator hot-swap applied …
  drifted=1 applied=1` on james (~60 s), attestation (+86 s), then a rollback-shaped release;
  receipts `genesis/a2o/reports/workspace-release/2026-09-08/`.
- **Verify:** both attestations read back; the workspace peer's own adoption resolves (T4).
  Delta with both wall-clocks. Preflight the quiesce gate's four legs first (CLAUDE.md CI/CD).
- **Never** during a rolling edge deploy.

### T10 — First peer-executed stage: an a2o feature run as a compute task, attested by the provider (D-E)
- **Tier:** Opus design → Sonnet implement. **Habit:** `operator-runtime-surface`.
  Rebases on T4 (shared `genesis/a2o` tree).
- **Evidence:** `genesis/agentic/compute/workspace.mjs` verbs grant/submit/start/poll/listen;
  envelope ≤ 64 KiB, content-addressed binary/dna, invocation nonce; authority leg measured
  with jessica as provider (3/3 `delegated-sweettest.feature`).
- **Do:** a task whose `binary` is a content-addressed script running `just test mesh
  genesis/a2o/features/dataplane/federation-version-convergence.feature` (`@requires:
  multi-node`, satisfied by the household mesh) on the provider, returning `cucumber.json`
  as a payload lease; requester reads it back with `poll`; the attestation is the
  EconomicEvent the grant flow already mints. Prefer existing envelope fields for "stage name
  + upstream task CID"; any new field goes through `p2p-design-gate` first (expected: class C
  ephemeral inside a content-addressed envelope, identity = task CID, no DHT entry, no route).
- **Verify:** matthew requester, jessica provider; one receipt with task CID, payload CID,
  attestation hash; run twice to show the nonce makes retry idempotent.
- **Stretch (next sprint):** storage cargo gate in the worker image; Adam as provider.
- **Reviewer:** Opus checks the provider cannot claim a result it did not run
  (`api/compute_tasks.rs` verified-performer path).

### T11 ⟲ — Ark station 3b: stranger-refusal receipt and the blob leg (the gate already ships)
- **Tier:** Sonnet. **Habit:** `runtime-death-witnessed` (active, red).
- **Evidence ⟲:** `private_serve_verdict` at `elohim/elohim-storage/src/private_reach.rs:247`,
  wired at `shard_service.rs:147` with the reach short-circuit at :138; plan
  `2026-09-03-ark-s1-station3b-custody-read-gate-plan.md` landed. Remaining: the blob leg and
  the a2o `@wip` flip.
- **Do:** confirm the blob fetch path honors the same verdict; flip the scenario's `@wip`;
  produce the stranger receipt.
- **Verify:** `just mesh join-peer stranger`; stranger fetches a private row and its blob by
  CID → typed refusal, zero private rows visible; `just mesh recovery warm james` still PASS.
  Delta on the habit.
- **Reviewer:** red-team (read-only) tries the fetch another way.

### T12 — Holochain integration close, part 1: choose the decision-experiment budgets (D-D)
- **Tier:** Opus gathers measured numbers; Codex (GPT-6) adjudicates as an outside skeptic; I
  decide; operator ratifies.
- **Evidence:** clone cost under gossip (+1.62 MiB RSS, +2.11 MiB disk, +21 FDs, +10 threads
  per clone); warm recovery 258 s (2026-08-28); restart churn ≈ 20 min; quiesce gate legs;
  conductor RSS ∝ corpus at full arc; T6 convergence numbers; T8 gate numbers.
- **Do:** append a "chosen budgets" table to §8 of
  `genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md`:
  local-save latency, healthy-network publication, working memory, disk growth, idle
  bandwidth, cold recovery, witness unavailability, each with source receipt and threshold
  for the intended household hardware. Managed surface: edit through the cite tooling; record
  `epr flow note --kind ruling`.
- **Verify:** every number cites a receipt path; Codex read attached.

### T13 — Holochain integration close, part 2: the D6 cell-qualified projection/sync contract (D-C)
- **Tier:** Opus (`rust-architect`), gated by `p2p-design-gate`; design-only.
- **Evidence:** `clone-content-escapes-via-shared-projection.md`; projection keyed by
  `h_app_id`, sync docs in the `elohim` namespace, re-author path crosses cells.
- **Do:** a contract beside the correction contract: the cell qualifier on every projected row
  and sync doc (DNA hash + cell id), the re-author refusal rule, partition behavior, rebuild
  path, migration shape for existing rows. Cites the atom and the habit. No code.
- **Verify:** blind-reader pass; Codex readability edit; the atom's `done when` maps to the
  contract's stations.

**Batch 3 push gate:** T9 and T10 receipts, T11 delta, T12/T13 sealed with cite-gen; one push.

---

## Operator-owned (agents cannot do these; listed so they are not lost)

1. `elohim/rakia`: commit `rakia-executor/` in the rakia repo, push, bump the pin here
   (`rakia-executor-untracked-in-submodule-pin.md`); T5(a) rides the same commit. Blocks any
   fresh clone or CI from building the compute worker.
2. `elohim/brit` shows untracked despite a registered gitlink; re-init or re-add so the pin
   cannot drift silently.
3. Adam compute worker: provision the Secret and pin the image digest so
   `genesis/orchestrator/data/adam-compute-worker.json` can enable (T10 stretch).
4. Three pushes (one per batch), never during a rolling edge deploy.
5. Optional: pin the devfile image to a versioned tag instead of `:latest`.

## Delegation map

| Task | Tier / agent | Write-set | Verifier |
|---|---|---|---|
| T1 | Sonnet | `hc-mesh.sh`, `justfile` mesh recipe | Haiku |
| T2 | Sonnet (after T1, same agent) | `hc-start.sh`, `justfile` dev recipe | Haiku |
| T3, T3b | Sonnet (sequential) | `genesis/seeder/src/*`, lamad manifest + codegen | Sonnet rerun |
| T4 | Sonnet | `genesis/a2o/scripts/release-ceremony.ts`, storage `/admin/adoption` handler | Opus |
| T5 | Sonnet | rakia executor (submodule), `compute_payload_store.rs` | Haiku / Sonnet |
| T6 | Opus | `feedback_projector.rs` + `tests.rs` | Opus |
| T7a | Opus (after T6) | `content_store/feedback_signal.rs`, wake path in storage | Sonnet rerun |
| T7b | Sonnet (after T6) | a2o `helpers.ts`, `steps.ts` deadlines | Haiku |
| T8 | Opus→Sonnet | `manifest.schema.json`, `gate-runner.mjs`, `run-local-gate.sh`, storage manifest | Haiku |
| T9a | Sonnet | one backlog atom | Haiku |
| T9 | fork / Sonnet | receipts only | me |
| T10 | Opus→Sonnet (after T4) | `genesis/agentic/compute/*`, maybe envelope schema | Opus |
| T11 | Sonnet | a2o feature tag, blob path if needed | red-team |
| T12 | Opus + Codex | reimplementation plan §8 | me + operator |
| T13 | Opus | new contract doc | blind-reader + Codex |

Collisions handled: T1/T2 share `justfile` (sequential); T3/T3b share the seeder (sequential);
T6/T7a/T7b share the feedback plane and T7's deadlines derive from T6's budgets (T6 first);
T4/T10 share `genesis/a2o` (batch order).

## Verification (sprint level)

- Per batch: `just gate` per touched project with `EXIT=$?` echoed; habits re-projected; one
  push; ci-observer summarizes; no new fingerprint in `.claude/data/ci-findings.jsonl`.
- Sprint close: `habits-status.py --full` shows deltas on `dataplane-convergence`,
  `runtime-upgrade-propagation`, `operator-runtime-surface`, `runtime-death-witnessed`,
  `happ-lineage-migration`; three wall-clock numbers (mesh start→ready, storage gate,
  sweep-to-convergence) each improved against their baseline; two harvest receipts (T9 rung-5,
  T10 peer-executed stage) exist; accountable correction ≥ 7/8.
- `story-harvest` at close for parameter-bearing discoveries (T6 K, T8 jobs, T12 budgets).

## Out of scope (held)

- D6 implementation (after slice 1); DNA hash remap (next integrity crossing, D5); cargo gate
  as a compute task on Adam (operator item 3); election read-lag diagnosis on two of seven
  alpha peers (after T6); group/household clone spaces; station 3 dependent view unless a
  slot frees.
