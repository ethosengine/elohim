---
title: "Wave 1.3 — Light the resilience card honestly: live distribute_shards household observation (verify-track)"
id: live-distribute-shards-household-observation-plan
status: Draft
class: substrate
domain: D5
sprint: substrate-validation   # durability-arc rung (M1→substrate-validation); de-risks roadmap Sprint 2 (grandma-standard recovery)
topic: [resilience-card, distribute_shards, shard_locations, stewarding, household-mesh, observation, verify-track, p2p-dataplane, a2o, dataplane-actuation]
refines:
  - genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md
cites:
  - resiliency-card-p2p-weave-sprint-plan | the parent sprint; this plan executes its Wave 1.3 (the single highest-leverage first slice) | sha256:834716e333f5b01f | path: genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - epr-durability-replication-arc-plan | the distribute_shards home — Workstreams A (custody/DHT leg) + D (stewarded aggregates), the Phase-0 observe-first discipline, and the /p2p/status counter contract | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - resilience-card-lighting-plan | the card-lighting arc this continues (commitmentBacked already lit; this lights stewarding) | sha256:be6dfb65e5e8a433 | path: genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md
  - resilience-facings-select-fold-aggregate-design | the fold layer this observes — §8 clean-read-projection P2P-gate verdict + §9 slices; the lens is built, this feeds it | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - che-live-peer-dev-loop-design | the local-runner x live-mesh observation pattern Task 1 uses to capture the green run | sha256:f976477c2f2baba0 | path: genesis/docs/superpowers/specs/2026-06-10-che-live-peer-dev-loop-design.md
  - qahal-epr-household-lattice-design | the household/hub topology the holder-relation groups by | sha256:ed5c1d3d2698b567 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - dht-is-a-notary-not-a-byte-store | the binding constraint — shard_locations is gossip+projection (Category-C), never a DHT entry | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - genesis/a2o/features/resilience/observable-distribution.feature
  - genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature
  - genesis/a2o/features/federation/peer-recovery.feature
  - genesis/a2o/steps/resilience.steps.ts
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - genesis/scripts/ci/substrate-verify.sh
informed-by:
  - genesis/a2o/features/resilience/
  - genesis/docs/content/elohim-protocol/architecture/cluster-topology.md
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env. The household-mesh
# observation leg (Tasks 0–5) is testable on a local hc:start:seed M/J/J stack NOW. Only the
# live-alpha durable proof (Task 6, sustained soak) carries an inline @requires — leak-gated, NOT
# a cluster-capability gate. Every task below is single-host-or-local-mesh, household-nodes class.
---

# Wave 1.3 — Light the resilience card honestly: live distribute_shards household observation

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans, task-by-task, two-stage review. This is a **VERIFY-TRACK plan** —
> the scenarios, step-defs, and runtime path already exist (verified file:line below). The work is to
> **run them green against a real mesh and capture the honest observation**, fixing whatever Phase 0
> reveals as the actual break — NOT to re-author coverage. The **p2p-design-gate is already satisfied**
> (no new entity — see below); do not re-run it. Story-first: the a2o that proves each leg already exists;
> extend it in the same commit if a leg needs a new assertion.

**Goal:** the resilience card reads `stewardingCollectives ≥ 2` (and non-zero `diversityScore` + real
`regionOccupancy`) as a genuine **observation of a live `distribute_shards` push-ack** on a 3-household
mesh — not a seeded claim — captured green by the existing `observable-distribution.feature`.

**Architecture:** `POST /db/content` with **blob-backed** content already triggers `distribute_shards`
(on-by-default), which fans RS shards to contract-diverse households over libp2p and writes
`shard_locations` rows (`status="announced"`). The resiliency lens (`elohim-facings`) folds
`shard_locations ⋈ humans` into the card. Every fold is built and proof-gated; the card is dark only
because **no live run has populated `shard_locations`**. This plan drives that run on a local M/J/J
household stack, observes where the manifest→reality chain breaks (durability-arc Phase 0), repairs the
break, and captures the green observation + honest `/p2p/status` counters as evidence.

**Tech Stack:** elohim-storage (Rust, libp2p 0.54.1, diesel/SQLite), `sharding.rs` (rs-4-7), a2o
(Cucumber/TypeScript, `@local` profile), `hc:start:seed` local stack, `/p2p/status` counters.

## Global Constraints

- **No new DHT entity, no new table, no new route, no new sync dialect.** `distribute_shards` /
  `shard_locations` are pre-classified **Category-C** (gossip + projection) per
  `dht-is-a-notary-not-a-byte-store` and the resilience-facings §8 P2P-gate verdict. This plan ADDS no
  entity — it exercises and observes existing ones.
- **Honest-observation bar (parent sprint Decision A, resolved 2026-06-22):** `stewardingCollectives > 0`
  is legitimate ONLY from a real `distribute_shards` push-ack. `seed_shard_manifest`
  (`ALLOW_SEED_SHARD_MANIFEST=1`, `status="seeded"`) is **acceptance/demo only — never the
  production-lit path.** The card is a declared trust surface (`distributionState: measured|unmeasured`,
  "never a fake at-risk verdict").
- **`distribute_shards` is blob-gated.** It fires on `POST /db/content` only when `blob_hash` is present
  (`http.rs:4301`). Plain-markdown content will NOT trigger distribution — the observation MUST ingest
  **blob-backed** content (album/photo/video).
- **Join key is `agent_cid` (`uhCAk…`).** `shard_locations.peer_id` already holds `agent_cid` (misnamed
  column, verified `seed_shard_manifest.rs:55-58`, `peer_selection.rs:253-255`); `humans.agent_pub_key`
  is `agent_cid`. The INNER JOIN drops rows when `agent_pub_key` is NULL — that is the live data gap, NOT
  a namespace bug. Never raw-string-compare against a transport id; the transport-id resolver is blocked
  and NOT needed here.
- **Encryption-ordering edge (do not trip):** `content_reach:"commons"` is hardcoded at
  `p2p/mod.rs:1492`. Do NOT resolve that reach-derivation TODO in this plan — private content would
  plaintext-leak to custodians before blob encryption (weave #4) lands.
- **`genesis/Jenkinsfile` CPS cap:** the pipeline block is at ~63.8KB of a 65KB hard limit — if Task 6 is
  taken, NO new inline `sh """…"""` heredocs; bash bodies go in `genesis/scripts/ci/*.sh`.
- **Commit-only; integrator pushes.** Shared worktree — selective staging. Never `kubectl` from the dev
  session. elohim-storage builds keep ambient `RUSTFLAGS` (getrandom custom); `CARGO_TARGET_DIR` = pool
  slot (fall back `/tmp` on fingerprint ENOENT); plain `cargo test` (no nextest in container).

---

## Provenance & framing

Surfaced 2026-06-26 from the operator's "what's the next lens" framing while watching the live alpha card
read `stewardingCollectives 0 / commitmentBacked 2 / diversity 0% / no region / contracts-short 0 of 1`.
The grounding verdict (atlas-grounding `wf_1f1364ff`, 2 adversarial verifiers, `refuted=true` both): **the
darkness is a missing dataplane ACTUATION, not a missing lens.** All five facings shipped to `dev`
2026-06-22; the resiliency lens + folds are built and the Slice-0 proof-gate (`46baef5e5`) passes with
coherent data. This is the parent sprint's **Wave 1.3** — its own "single highest-leverage first slice."

**Composes from (born-linked, lexical floor):** the parent sprint plan (Wave 1.3), the durability-arc
plan (Workstreams **A** custody/DHT-leg + **D** stewarded-commitment aggregates; its **Phase 0
observe-first** discipline and `/p2p/status` counter contract), the card-lighting plan, resilience-facings
§9, and `che-live-peer-dev-loop` (the local-runner × live-mesh observation pattern).
**Semantic lens (MemPalace) was UNAVAILABLE + index known-stale (2026-06-11, 73 files behind) → degraded
to lexical-only; the 8-match lexical floor stands.**

## Ground truth (verified file:line 2026-06-26 — do NOT re-derive)

| Claim | Verdict | Evidence |
|---|---|---|
| `distribute_shards` wired + on-by-default | TRUE | `p2p/mod.rs:1489` (writes `shard_locations` `status="announced"` at `:1550-1559`); called from `http.rs:4301` (`POST /db/content`) + `:4487` (bulk); `p2p` in `default` Cargo features |
| Trigger is **blob-gated** | TRUE | `http.rs:4301` success branch only fires when `blob_hash` present |
| Resiliency lens + folds built & proof-gated | TRUE | `elohim-facings/` pure crate (`da0282201`); Slice-0 proof-gate `populated_relation_lights_stewarding_regional_and_intra_hub` (`46baef5e5`) green |
| `observable-distribution.feature` step-defs REAL | TRUE | `resilience.steps.ts:186` (2-household precondition), `:303` (ingest), `:403` (`stewardingCollectives ≥ 2`), `:428/:443` (placement/protection) |
| …but **NEVER RUN in CI** | TRUE | feature tagged `@local` (line 1); `e2e-verify-api.sh` filter is `@e2e and not @local and not @wip`; authored Apr-2026 (`dev-intent.jsonl:4,6`), no green CI run recorded; only `pnpm test:local` runs it (not in Jenkins) |
| Content-viewer tooltip step is a stub | TRUE | `resilience.steps.ts:326` returns `'pending'`; browser-mode `assert.fail(NO_PW_DEVICE)` |
| Honest counters exist | TRUE | `/p2p/status` exposes `reconcilePassesTotal`, `kicksFiredTotal`, `placementGapsEmittedTotal` (durability-arc Phase 0); `kicksFiredTotal>0` on a receiver = manifest→reality chain alive |
| `grandma-photos:98` is a ready blob-backed pilot | TRUE | `grandma-photos-survive-node-loss.feature:98` tagged `@local @resilience-p1`, blob-backed album across household mesh |

**P2P-design-gate status: ALREADY SATISFIED.** resilience-facings §8 ran it → "clean read-projection,
no new DHT entry type, no new identity, no new commitment"; the weave-epic index forecloses any
capacity/location DHT entry. This plan introduces no entity, so the gate does not re-fire. (Recorded here
so a downstream worker does not stall re-running it.)

---

## Phase 0 — observe before building (FIRST move; gates everything after)

### Task 0: Stand up the local household mesh and locate the break

**Files:** none (observation). **Interfaces produced:** the Phase-0 verdict (one of the three branches
below) that selects which of Tasks 2–3 fire.

- [ ] **Step 1: Start the seeded local stack.** From `app/elohim-app/`:

```bash
pnpm run hc:start:seed
```

Expected: conductor + storage (:8090) + doorway (:8888) come up; seeder runs household-formation +
provide-row seeding. (Skill: `hc-dev-orchestrator` if any service is unreachable.)

- [ ] **Step 2: Confirm the scenario precondition is REAL (2 households, active commons commitments).**

```bash
curl -s localhost:8090/api/v1/commitments | python3 -c "import sys,json; r=json.load(sys.stdin); \
print('active commons providers:', sorted({c['provider'] for c in r if c.get('action') in ('provide','replicates-content','replicates-commons') and c.get('state')=='active'}))"
```

Expected: ≥ 2 distinct `agent_cid` providers across ≥ 2 households. If empty → the precondition itself is
unseeded (resolve in Task 3 before any ingest).

- [ ] **Step 3: Snapshot the receiver counters BEFORE ingest** (pick two non-ingesting nodes; matthew is
  the ingest node, james + jessica the receivers):

```bash
for n in james jessica; do echo "== $n =="; curl -s localhost:8090/p2p/status \
  | python3 -c "import sys,json;d=json.load(sys.stdin);print({k:d.get(k) for k in ('reconcilePassesTotal','kicksFiredTotal','placementGapsEmittedTotal')})"; done
```

(In the local stack each node has its own storage port; substitute the per-node port. Record the
baseline `kicksFiredTotal`.)

- [ ] **Step 4: Ingest BLOB-BACKED content and watch `distribute_shards` fire.** Post a blob-backed item
  (album/photo) via the doorway, then check the writer and the table:

```bash
# (use an existing seeded blob-backed item, e.g. the grandma album fixture, or POST one)
curl -s "localhost:8090/api/v1/resilience/<contentId>/household" | python3 -m json.tool
sqlite3 <storage content.db> "select content_cid,peer_id,status from shard_locations limit 20;"
```

Expected: `shard_locations` has rows with `status="announced"`; `/api/v1/resilience/<id>/household`
reports `stewardingCollectives ≥ 2`, `distributionState:"measured"`.

- [ ] **Step 5: Branch on what you observe** (the decision tree — record the verdict, it selects Tasks 2–3):

  - **(α) Card lights, `stewardingCollectives ≥ 2`, counters rose** → chain is ALIVE. Skip Tasks 2–3; go
    straight to **Task 1** (lock it green) → **Task 5** (capture).
  - **(β) `shard_locations` populated but `stewardingCollectives` still 0** → the **`agent_pub_key`
    junction** drops rows (resilience-facings §8). Fire **Task 2**.
  - **(γ) `distribute_shards` never fired / `shard_locations` empty after a blob ingest** → either content
    wasn't blob-backed, no contract-diverse peer was selectable, or the custody/DHT leg is broken
    (durability-arc Phase-0 γ: `reconcilePassesTotal>0, kicksFiredTotal==0`). Fire **Task 3**.

- [ ] **Step 6: Commit the Phase-0 evidence** (a short note appended to `.claude/data/dev-intent.jsonl`
  capturing the branch + counters), so the chosen remediation is auditable.

---

## Task 1: Run the core observation scenario green (VERIFY)

**Files:** Test: `genesis/a2o/features/resilience/observable-distribution.feature` (no change unless a leg
needs a new assertion). **Interfaces consumed:** Phase-0 verdict α. **Produces:** a green
`@resilience-p1` run capturing `stewardingCollectives ≥ 2`.

- [ ] **Step 1: Run the ready blob-backed pilot first** (lightest path). From `genesis/a2o/`:

```bash
E2E_MODE=local E2E_DOORWAY_ALPHA=http://localhost:8888 E2E_STORAGE_URL=http://localhost:8090 \
  pnpm test:local --tags '@resilience-p1 and @local and not @wip and not @browser-only'
```

Expected: `grandma-photos-survive-node-loss.feature:98` (blob album across the mesh) PASSES.

- [ ] **Step 2: Run the two core observable-distribution scenarios.**

Run (same env): the suite above already includes them (`@resilience-p1 @local`). Confirm in output:
`Full placement across two households` PASS (asserts `stewardingCollectives >= 2`), `Placement gap when
commitments are short` PASS (asserts a `contracts-short` row).
Expected: both green. If red → return to Phase-0 branch β/γ and fire Task 2/3, then re-run.

- [ ] **Step 3: Capture `/p2p/status` counters AFTER the green run** (the honest evidence the card is an
  observation, not a claim):

```bash
for n in james jessica; do curl -s localhost:8090/p2p/status \
  | python3 -c "import sys,json;d=json.load(sys.stdin);print('$n kicksFiredTotal=',d.get('kicksFiredTotal'))"; done
```

Expected: `kicksFiredTotal` ROSE above the Task-0 baseline on at least one receiver.

- [ ] **Step 4: Commit** (evidence note only; no code change if α):

```bash
git add genesis/a2o/features/resilience/observable-distribution.feature   # only if an assertion was tightened
git commit -m "test(resilience): observable-distribution green on local M/J/J mesh — stewardingCollectives>=2 as live observation"
```

---

## Task 2 (CONDITIONAL — Phase-0 branch β): repair the `agent_pub_key` junction

Fires ONLY if `shard_locations` is populated but `stewardingCollectives` stays 0 — the INNER JOIN
`humans.agent_pub_key = shard_locations.peer_id` drops rows because `agent_pub_key` is NULL.

**Files:** Modify (verify-then-fix): `elohim/elohim-storage/src/db/humans.rs` (`heal_human_identity`
NULL-fill path), `genesis/seeder/src/seed-provide-rows.ts` (the `/api/v1/identity/heal` call before
provide-row writes). Test: `elohim/elohim-storage/tests/household_resilience.rs`.

- [ ] **Step 1: Confirm the NULL with the existing diagnostic test as the spec.** Read
  `tests/household_resilience.rs` `stopgap_heal_lights_commitment_backed_via_direct_join` — it asserts the
  join lights after heal. Run it:

```bash
cd elohim/elohim-storage && CARGO_TARGET_DIR=/tmp/wave13-target cargo test --lib --test household_resilience stopgap_heal -- --nocapture
```

Expected: PASS (proves the fold is correct given non-NULL keys; the gap is data population).

- [ ] **Step 2: Verify the seed path actually heals the M/J/J humans.** Query each node:

```bash
sqlite3 <storage content.db> "select id, (agent_pub_key is not null) as has_key, household_id from humans;"
```

Expected after fix: every steward human row has `agent_pub_key` non-NULL. If NULL → the seeder's
`/api/v1/identity/heal` call did not run for that node; ensure `seed-provide-rows.ts` invokes heal before
writing provide rows for each pod (the heal is gated on `/auth/me`; on the local stack the cell key
resolves).

- [ ] **Step 3: Re-run Task 1 Step 2.** Expected: `stewardingCollectives ≥ 2` now lights.

- [ ] **Step 4: Commit** the seeder/heal fix WITH the green scenario:

```bash
git add genesis/seeder/src/seed-provide-rows.ts elohim/elohim-storage/src/db/humans.rs
git commit -m "fix(resilience): heal agent_pub_key on the steward path so the holder-relation join lights"
```

---

## Task 3 (CONDITIONAL — Phase-0 branch γ): make `distribute_shards` actually fan

Fires ONLY if a blob ingest produced no `shard_locations` rows. Diagnose in order (cheapest first):

**Files:** none new — this is configuration/precondition repair + (if the custody/DHT leg is the break)
the durability-arc **Workstream A** concern, canonicalized not rebuilt here.

- [ ] **Step 1: Confirm the content was blob-backed.** A markdown item has no `blob_hash` → no
  distribution by design. Re-ingest a blob-backed item (album/photo fixture). Expected: `distribute_shards`
  INFO log fires.

- [ ] **Step 2: Confirm peer-selection found contract-diverse households.** If the log shows a
  `placement_gap` (`contracts-short`), there were < 2 households with an active `commons` provide
  commitment at ingest time → resolve via Phase-0 Task-0 Step-2 (seed the commitments) and re-ingest.
  Expected: selection picks ≥ 2 distinct households.

- [ ] **Step 3: If rows still absent with selection succeeding → the push-ack/custody leg is the break**
  (`reconcilePassesTotal>0, kicksFiredTotal==0`). This is durability-arc **Workstream A** (the
  CommitmentCommitted signal subscription / DHT-anchor→gossip→projection leg). **Do NOT rebuild it inline**
  — open ONE canonical concern (timeline-CONVENTIONS, status-documented) under
  `genesis/data/timeline/backlog/`, mark this plan's live-observation leg BLOCKED-ON it, and proceed with
  the local-stack short-run path (where matthew↔james↔jessica are directly connected, so the
  connected-fallback push path applies). Capture the blocker; do not silently truncate.

- [ ] **Step 4: Re-run Task 1.** Commit precondition/seed fixes with the green scenario.

---

## Task 4: De-stub OR honestly defer the content-viewer tooltip leg

The one real code gap surfaced by verification: `resilience.steps.ts:326` returns `'pending'`; the
tooltip scenario is CLAIMED-but-not-executable.

**Files:** Modify `genesis/a2o/steps/resilience.steps.ts:326`; Test:
`genesis/a2o/features/resilience/observable-distribution.feature:33`.

- [ ] **Step 1: Decide implement-vs-defer.** The API-level observation (Task 1) is the priority; the
  tooltip is the UI echo and needs a rendered content-viewer (Playwright/`pnpm look`). **Recommended:
  DEFER** — tag the scenario `@wip` honestly and file a one-line backlog item
  (`genesis/data/timeline/backlog/resilience-tooltip-step-destub.md`, domain D5, links this plan) so the
  `'pending'` stub stops masquerading as coverage. (Implement only if the operator wants the UI leg this
  cycle — then drive it via `looking-at-frontend` against the rendered content-viewer.)

- [ ] **Step 2: Apply the chosen path** (defer = add `@wip` + backlog note; do not leave a silent
  `'pending'` that reads as green).

- [ ] **Step 3: Commit.**

```bash
git add genesis/a2o/features/resilience/observable-distribution.feature genesis/data/timeline/backlog/resilience-tooltip-step-destub.md
git commit -m "test(resilience): honestly mark the tooltip leg @wip + backlog (no silent pending stub)"
```

---

## Task 5: Capture the observation as durable evidence + story-harvest

**Files:** `.claude/data/dev-intent.jsonl`; new `genesis/data/timeline/` evidence note if warranted.

- [ ] **Step 1: Record the green observation** — append to `.claude/data/dev-intent.jsonl` a 3–4 sentence
  note: what ran green (scenario + node topology), the lit values (`stewardingCollectives`, `diversityScore`,
  `regionOccupancy`), the counter delta (`kicksFiredTotal` baseline→after), and that it was a real
  `distribute_shards` push-ack (not `seed_shard_manifest`).

- [ ] **Step 2: Run `story-harvest`** to preserve the engineering constraints this leg proved — the
  **blob-only trigger**, the **2-household-active-commitment precondition**, the **`agent_pub_key`
  non-NULL join requirement** — as a2o regression guards / operator presets.

- [ ] **Step 3: Run `/close-loop`** to reconcile dev-intent → a2o scenario updates.

---

## Task 6 (OPEN DECISION — operator): durable gating of the household-mesh E2E

The `@local` CI-exclusion was **deliberate** (local stacks are dev-only). Two honest endpoints — operator
picks:

- **Option A (recommended): keep it a dev-loop observation + lean on the existing CI backstop.** The
  household-mesh E2E stays `@local` / dev-loop (`che-live-peer-dev-loop` pattern); the **live-alpha durable
  proof** rides the EXISTING `genesis/scripts/ci/substrate-verify.sh` *Verify Resilience Signals* stage
  (CI, inside-cluster, build-unique probe blob) — extend that stage to assert `stewardingCollectives ≥ 1`
  once conductor seeding is applied. **No new Jenkins stage; no CPS-cap risk.** `@requires:` the
  sustained-alpha leak-gate (conductor OOM soak) — that leg HELD.
- **Option B: promote to a gated CI lane.** Add a "Local/Household Resilience E2E" stage to
  `genesis/Jenkinsfile` running `@e2e and @local and not @wip` against a bootstrapped mesh
  (`hc:start:seed`). **Cost:** the CPS cap (bash body → `genesis/scripts/ci/resilience-local-e2e.sh`,
  never inline heredoc) + a local-mesh reference environment in CI. Heavier; only if a permanent
  regression gate is wanted.

- [ ] **Step 1: Operator confirms A or B** (default A). Do not build B speculatively.
- [ ] **Step 2 (if A):** extend `substrate-verify.sh` Verify-Resilience-Signals assertion; commit.
- [ ] **Step 2 (if B):** author `genesis/scripts/ci/resilience-local-e2e.sh` + the thin Jenkins stage call;
  commit (CPS-cap respected).

---

## Open Decisions (resolve at review; defaults stand if silent)

| # | Decision | Recommendation |
|---|---|---|
| A | Capture env: local `hc:start:seed` M/J/J stack vs live-alpha pods | **Local stack** — full dev-session control over POST + per-node `/p2p/status`; live-alpha writes + per-pod counters aren't cleanly dev-reachable (proxy reads matthew-only; no kubectl) → live-alpha is the CI/substrate-verify path (Task 6A) |
| B | Tooltip leg: implement now vs defer `@wip`+backlog | **Defer** (Task 4) — API observation is the priority; UI echo is follow-on |
| C | CI gating | **Option A** (Task 6) — dev-loop observation + extend the existing substrate-verify stage; promote to B only on demand |

## Land-now vs Held

- **LAND-NOW (local household mesh, dev-session):** Tasks 0–5 (observe, run green, repair the Phase-0
  break, capture). `household-nodes` class.
- **HELD — leak-gated (`@requires` inline, Task 6A):** the *sustained* live-alpha soak proving the
  observation holds across restarts — waits on the conductor-OOM soak verdict (jemalloc swap landed
  2026-06-19, not soak-confirmed).
- **HELD — backlog-blocked (only if Phase-0 γ Step 3):** the custody/DHT-leg break → durability-arc
  Workstream A concern, canonicalized not rebuilt here.
- **NOT IN SCOPE:** `seed_shard_manifest` as a production-lit path; blob encryption (weave #4); the
  `content_reach` derivation TODO (encryption-ordering edge); cross-doorway breadth (`@requires:shem`).

## Done (stability-gated, not single-green)

- `observable-distribution.feature` "Full placement across two households" + the `grandma-photos:98` pilot
  GREEN on the local M/J/J mesh across **two consecutive runs**, `stewardingCollectives ≥ 2`,
  `distributionState:"measured"`, with `kicksFiredTotal` risen on a receiver (push-ack, not seed-claim).
- `diversityScore > 0` and `regionOccupancy` non-empty fall out of the same lit relation (one root).
- The `agent_pub_key` junction (if it was the break) heals on the steward seed path; the diagnostic
  direct-join test passes.
- The tooltip leg is honestly `@wip`+backlogged (no silent `'pending'`).
- The observation captured to dev-intent + story-harvest; the Task-6 gating decision recorded.

## Non-goals

- Does NOT add a DHT entity, table, route, or sync dialect (Category-C exercise only).
- Does NOT fabricate `shard_locations` via `seed_shard_manifest` for a production-lit card.
- Does NOT rebuild the custody/DHT leg inline (that is durability-arc Workstream A — canonicalize if hit).
- Does NOT touch blob encryption or the `content_reach` derivation TODO.
- Does NOT promise live-alpha sustained lighting (leak-gated) — only the household-mesh observation +
  the CI substrate-verify hook.
