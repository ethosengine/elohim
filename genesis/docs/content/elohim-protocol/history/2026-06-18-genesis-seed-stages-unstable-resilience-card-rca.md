---
title: "RCA: Genesis seeding/verify stages Unstable + all-zeros resilience card"
id: genesis-seed-stages-unstable-resilience-card-rca
type: history-gotcha
status: noted
tier: history
created: 2026-06-18
author: session (feat/frontend-eyes-sprint)
topic: [genesis-pipeline, seeding, resilience-card, humans-junction, agent-pub-key, rca]
# Verified RCA that grounds the post-leak-fix stabilization plan; point-in-time live-alpha evidence.
# Routed out of .claude/data 2026-07-02 (machine-ledger law).
derived_from:
  - .claude/data/genesis-seed-stages-rca-2026-06-18.md  # original home; routed 2026-07-02
canonical:
  - genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md
---

# Genesis seeding/verify stages Unstable + all-zeros resilience card — RCA

**Date:** 2026-06-18 · **Branch:** feat/frontend-eyes-sprint · **Pipeline:** `genesis/Jenkinsfile` (job `elohim-genesis`, runs against live alpha)

## Ask
Drive five genesis stages from **Unstable → Success**: Seed Substrate, Seed Custody Commitments,
Seed REA Commitments, Verify Delivery Events, Verify Projection Sync. Bonus: the **resilience card**
for EPR `elohim-host-landing` (on elohim.host + alpha.elohim.host) should show peers + sync instead of
`at-risk / 0 stewarding / 0 commitment-backed / 0% diversity / no region / 1 placement gap`.

## The two halves are ONE system
The card lights ONLY through the "humans junction" in
`elohim/elohim-storage/src/services/household_resilience.rs` (`compute()`/`snapshot()`):
1. shard manifest must exist for the content (gate; else immediate all-zeros + `distributionState=unmeasured`).
2. `households_stewarding` = distinct `humans.household_id` where `shard_locations.peer_id == humans.agent_pub_key`
   (both `agent_cid`/`uhCAk…`) AND `humans.household_id IS NOT NULL` AND `shard_locations.shard_hash ∈ manifest`.
3. `commitment_backed` = distinct `humans.household_id` where `rea_commitments.provider == humans.agent_pub_key`
   AND `action ∈ {provide,replicates-content,replicates-commons}` AND `state='active'`
   AND `resource_classified_as='content:<reach>'`.
- Gospel (`elohim/elohim-storage/CLAUDE.md` → Identity & Transport-Identity Coherence): **never join `agent_cid`
  to a libp2p `12D3Koo…` transport id** — that silently empties the join and is the documented cause of the
  all-zeros card; the usual live cause is a **NULL `humans.agent_pub_key`**, fixed by *populating* it.
- `humans.household_id` is set by `ReconcileController` (controller.rs ~1110) **only where `agent_pub_key` already
  matches** → agent_pub_key must be populated FIRST (conductor seeding), then household_id (household formation).

So the same Seed Substrate sub-steps that feed the card are the stages going Unstable.

## Live alpha probes (2026-06-18, doorway uptime ~46min — freshly restarted PRE-FIX conductor)
- `/health`: conductor.connected, 14/14 pools healthy, `p2p.peerCount=13`, `caughtUp=true`, projection.writer=true.
- `/db/humans?limit=50` → **`{"items":[],"count":0}`** (zero humans rows).
- `/db/stats` → contentCount=3710.
- `/api/v1/commitments` → custody-blob, `state=active`, **`provider="12D3KooWFhAP…"` (libp2p, NOT agent_cid)**.
- `/api/v1/resilience/elohim-host-landing/household` → `distributionState=measured` (manifest exists),
  stewarding=0, commitment-backed=0, diversity=0, `placementGaps=[{requestedStewardCount:1, achievedStewardCount:0,
  gapKind:"contracts-short", shardHash:"sha256-9d981f99…"}]`, `protectionStatus=at-risk`.
- `/api/v1/peer-statuses` → 1 record `uhCAkaPrIWRPvkgo6…` status **degraded**.
- `/api/v1/network/posture` → totalPeers=1, activePeers=1, **householdsReciprocating=0**.
- `https://doorway.elohim.host` (apex) → **502 Bad Gateway** (elohim.host half of the card cannot render — operator).

## CI evidence (elohim-genesis builds 1170/1171/1172, all on `dev`, persistent — not flaky)
1. **Seed Conductor Identities** UNSTABLE: `CellDisabled(CellId(DnaHash(uhC0kcvrXsiO4Caub…), AgentPubKey(uhCAkunY…)))`
   on `elohim-adam-alpha`/`elohim-jessica-alpha` (rotates; DNA hash constant). matthew "2 exists". Downstream
   provide-rows all SKIPPED: `/auth/me → 401 … no agent_pub_key — conductor seeding must run first`.
2. **Household Formation** UNSTABLE: 1/3 affirmed; adam CellDisabled; jessica Wasm guest error
   `qahal_coordinator.rs:412 Guest("caller is not a current Steward of collective:…")`.
3. **Seed Custody Commitments** exit 1: `james-son→matthew-manager HTTP 503 {"status":"catching-up","retryAfter":30}`.
4. **Seed REA Commitments** (operator-bindings + stewardship + projections) — all three exit 1 with the same
   `503 catching-up`. Doorway is restarted (`restart-doorway-epr.sh`) AFTER these fail.
5. **Verify Delivery Events** FAIL: `0 serve-blob events` on all 3 pods — **though Verify Substrate Propagation
   PASSES** (bytes moved, replica on disk). The serve-blob REA EconomicEvent (blob_fetch atomic pair) isn't emitted.
6. **Verify Projection Sync** FAIL: matthew `pull=false` (consistent); adam `pull=idle/false` or unreachable;
   jessica `pull=idle` (WARN). Cursor lag checks all pass.

## ⚠ BLOCKING DISCOVERY (advisor-prompted)
The leak-fix commits **`9c034e8f8` (tx5 zombie leak fix) + `7747f3ec8` (embed patched conductor) are NOT in
`dev`/`origin/dev`** — only on `feat/frontend-eyes-sprint`. Alpha deploys from dev → **alpha runs the pre-fix
(leaking) conductor.** Builds 1170-1172 are pre-fix evidence. CellDisabled / catching-up / pull=false are
textbook unfixed-leak symptoms (OOM → cell disabled → projection reset → never catches up → sheds writes).

> **CORRECTION (2026-06-18, later session — git-verified; supersedes the framing above):** The leak-fix
> keystone IS now on `origin/dev`. `origin/dev` carries `2af2607e7` ("embed the patched conductor by default
> (fleet-wide)"), **byte-identical to the branch's `7747f3ec8`** (same `git patch-id` `6789a9f2cd1e289f…`),
> sitting atop the canary plumbing (`5a73e400d`/`b33ff524a`). Local `dev` is **NOT diverged** — it is 260
> commits *behind* `origin/dev` (`git merge-base --is-ancestor ebbe201f7 origin/dev` ⇒ yes; fast-forward
> only). The integrator landed the leak fix on dev after this RCA was first written. **The one remaining
> branch-only repo item is Fix #4 (`7afa03337`, the acquisition rollup)** (`merge-base --is-ancestor
> 7afa03337 origin/dev` ⇒ no). Net effect on the runbook below: the "reconcile diverged dev refs / land
> 7747f3ec8" step is **already done**; the live keystone collapses to **redeploy alpha from dev (image
> rebuild + rolling restart) + merge Fix #4 + re-run genesis**. Cause #7's "DIVERGED" classification is the
> only stale cell in the table.

## VERIFIED classification (triage workflow `wf_d1a05815-6eb` + own primary-source confirmation)

| # | Stage / symptom | Classification | Fix home |
|---|---|---|---|
| 1 | `CellDisabled` adam/jessica → conductor-identity + household-formation partial | **LEAK-DEPENDENT** | Deploy leak fix + cycle pods (boot re-enables: `happ_manager.rs:144-150`; no on-demand re-enable) |
| 2 | `503 catching-up` sheds custody + all 3 REA sub-stages | **LEAK-DEPENDENT** | Conductor call fails (`project-epr`=`create_via_conductor` unconditional; `rea_commitment_service.rs:137`) → `StorageError::Conductor`→503 no-Retry-After → doorway maps to `catching-up`, defaults 30s (`storage_proxy.rs:251`), breaker opens (`upstream_health.rs`) → cascades to diesel/SQL writes. retryAfter:30 = `UPSTREAM_CIRCUIT_COOLDOWN_SECS` ≠ admission shed's 2s, so "admission-exempt seed writes" is a NON-FIX. Hardening only: leak-aware preflight canary in `verify-doorway-readiness.sh` (SKIP not UNSTABLE); `StorageError::Conductor` 503 carry short Retry-After |
| 3 | **serve-blob events=0** (Verify Delivery Events) | **GENUINE-REPO-BUG** (leak-indep, HIGH conf) | serve-blob is emitted ONLY by on-demand pulls (`blob_fetch.rs:218-254 finalize_fetch_success`). The proactive quilt-draw path that moves bytes on alpha (`p2p/mod.rs:3756-3777`) calls bare `blob_store.store()` — never books the event. Route it through a `finalize_quilt_draw` helper. ⚠ SEMANTIC: decide `serve-blob` vs distinct `blob-hosted`/`replicates-content` action (p2p-design-gate) + whether `substrate-verify.sh` delivery query widens. `blob_fetch.rs` byte-identical dev↔branch → not a regression |
| 4 | **pull=false** (Verify Projection Sync) | **GENUINE-REPO-BUG** (leak-indep, HIGH conf) | Acquisition rollup counts already-local pinned items in `total` but they never increment `fetched` (only network arrival via `mark_completed` does) → `caughtUp=false` forever on the content-holder (matthew). `acquisition.rs:89/175`; `per_epr` rollup disagrees with `rollup()` on same state = the defect tell. Fix: in `reconcile()` seed already-local want_ids into the tracker `completed` set. Unit-testable |
| 5 | **`humans.agent_pub_key` never populated** → card commitment-backed=0 | **GENUINE-REPO-BUG** (leak-indep, verified — overturns workflow cause E's "leak-dependent") | Only real-key writer is `heal` (POST `/identity/heal`, `api/identity.rs:149`), called by `seed-provide-rows.ts` with the key from `/auth/me`. `/auth/me` needs a `LocalSession` (`http.rs:7385`). **NO `LocalSession` is ever created on a pod**: no `POST /session` in seeding, no boot self-session in `main.rs`, no portal handoff on server pods. `on_membership_projected` (`controller.rs:1105-1114`) only writes `household_id`, never `agent_pub_key`. So `/auth/me` 401s forever → heal skipped → key NULL → both junctions empty, regardless of conductor health. Fix (needs design/p2p-design-gate): boot a `LocalSession` from the conductor cell's `agent_pub_key` (via `cell_discovery`) so `/auth/me` returns it. NOTE `self_cid` is the transport id (`12D3Koo`), NOT the agent_cid — must use the cell key, not self_cid |
| 6 | `shard_locations` no seed path → card stewarding=0 | **STRUCTURAL + leak-degraded mesh** | Written ONLY by runtime `distribute_shards` (`p2p/mod.rs:1505-1532`) over a live multi-peer mesh; no REST seed path. The stale `placementGap` (firstSeen=lastSeen=2026-06-13, contracts-short) is the recorded shadow. Needs healthy mesh (follows from leak fix). No clean repo fix — it's the designed path |
| 7 | leak-fix landing | **DEPLOY/CLUSTER-STATE** | Local `dev`(ebbe201f7, what alpha deploys from) ↔ `origin/dev` DIVERGED; load-bearing commit is `7747f3ec8` (flips Dockerfile default to patched harbor `elohim-edgenode:latest`). Reconcile dev refs, verify che-dw harbor `:latest` is patched (recipe gates: go-pion feature set, kitsune2 0.3.2↔0.3.0-dev.3 wire-compat), canary smaps-flatten, redeploy. Image rebuild + rolling restart — NOT `ALLOW_DNA_REINSTALL` (binary swap, DNA hash unchanged) |
| 8 | apex `doorway.elohim.host` 502 | **DEPLOY/CLUSTER-STATE** | Operator; elohim.host card can't render until back |

### Repo fixes I can land this session (TDD, committed on branch, leak-independent)
- **#3 serve-blob emission** — needs p2p-design-gate (action taxonomy decision) before implementing.
- **#4 pull already-local rollup** — pure counting-bug fix, unit-testable, no new entity, cleanest.
- **#5 boot self-session → agent_pub_key** — genuine bug but new write path + identity semantics → p2p-design-gate + design decision (boot self-session vs seeder POST /session vs admin key endpoint).

### NOT repo-fixable this session (operator/deploy)
- #1, #2, #6-mesh resolve on deploying the leak fix (#7). #8 apex 502. I commit; integrator pushes/deploys.

### Composition: extends `genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md`
The arc's Workstreams A/D/E + its "Done" criteria (kicksFired rising, delivery≥1, projection clean, resilience non-zero) ARE this ask. What changed since 2026-06-10: netpol applied (conductor seeding now runs → reaches CellDisabled); the real blocker was the leaking conductor (now fixed on branch, not deployed); snapshot now accepts replicates-content/commons + active.

## Deliverable shape (honest — I cannot trigger CI or run kubectl, and feat/* isn't orchestrator-indexed)
Repo fixes for genuinely repo-fixable items (TDD'd, committed on branch) + an operator runbook for the
deploy/cluster items. Do NOT relax verify assertions to force green (symptom-fix trap).

## Session outcome (2026-06-18)
- **Plan:** `genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md` (cite-sealed, 13 gap-items; composes the durability arc).
- **Task 1 (#4 pull rollup) — DONE, committed `7afa03337`** (commit-only; integrator merges to dev). TDD: genuine RED (`left:(1,0,0) right:(1,1,0)` at acquisition.rs:307) → GREEN (12/12 `p2p::acquisition` tests; resolved-empty + failed-item guards still pass). Leak-independent; unit-verified on the household-nodes floor. Added `GapTracker::mark_local_wants_satisfied`.
  - Build note: the cargo-pool slot threw repeated fingerprint-ENOENT (disk-pressure reclaim racing the active slot at 83-84%); built in a fresh `/tmp/es-tgt-acq` dir outside the pool, then reclaimed it.
- **Task 2 (#3 serve-blob) — READY, not landed.** Genuine repo bug, high confidence. Blocked on a p2p-design-gate taxonomy decision (serve-blob vs blob-hosted action; whether the verify query widens) — needs the REA-action-taxonomy owner; do not force serve-blob. Deferred implementation (also avoids a 2nd ~15G build under disk pressure).
- **Task 3 (#5 agent_pub_key) — captured, NOT implemented.** Security-gated (self-asserted session bypasses TOFU; gospel forbids consuming the binding for economic attribution). Reconciliation note appended to `resilience-card-self-cid-provide-loop-gate.md` flagging the contradiction + the one discriminating operator test: after leak fix deploys + reseed, `GET /auth/me` on a healthy pod — 200+key = leak-dependent (that doc right), 401 = structural gap (this triage right).
- **Keystone (operator) — UPDATED (leak fix already on `origin/dev` as `2af2607e7`≡`7747f3ec8`):** redeploy alpha from dev (image rebuild + rolling restart; **NOT** ALLOW_DNA_REINSTALL — binary swap, DNA hash unchanged) → merge Fix #4 (`7afa03337`, branch-only) → re-run genesis. Resolves #1/#2/#6-mesh. Plus apex `doorway.elohim.host` 502. (The earlier "land `7747f3ec8` / reconcile diverged dev refs" step is DONE — see the CORRECTION above; local `dev` is merely fast-forward-behind, not diverged.)
- **None of the 5 stages are verifiable-green by this session** (no deploy / CI-trigger / kubectl from here).
