---
title: Resilience card lighting — uniform-list classification + per-pod membership, finishing the dark-card arc
id: resilience-card-lighting-plan
status: Draft
class: substrate
domain: D-substrate-distribution
sprint: composes 2026-06-13-non-commons-provide-commitments-design §11 (the set→list decision) and 2026-06-18-genesis-seed-stabilization-postleakfix-plan (supersedes its pre-probe premise with the U1/U2 findings)
cites:
  - non-commons-provide-commitments-design | §11 addendum decides uniform JSON-list + typed accessor; this plan implements it | path: genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md
  - genesis-seed-stabilization-postleakfix-plan | the pre-probe RCA/plan this supersedes — U1/U2 replace its agent_pub_key-never-populated premise with the serialization root cause | sha256:68252229e83056bc | path: genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md
  - genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md
  - genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md
  - genesis/data/timeline/backlog/qahal-collective-cid-formation-projection-gap.md
  - epr-durability-replication-arc-plan | Sprint 4 owner — stewardingCollectives via real P2P distribute_shards lands in this arc, not here | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/elohim-storage/src/db/rea_commitments.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/services/distribution_view.rs
  - elohim/elohim-storage/src/views_convert/shefa.rs
  - elohim/elohim-storage/src/services/genesis_self_heal.rs
  - genesis/seeder/src/seed-provide-rows.ts
requires_env: [household-nodes]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
---

# Resilience card lighting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:test-driven-development per task, and
> superpowers:subagent-driven-development / executing-plans for the arc. The **p2p-design-gate is
> MANDATORY** before Sprint 2's per-pod identity write surface and before any work on the weighted-
> affinity model (non-commons spec §11.3 — do NOT auto-build it). Story-first: land the a2o
> scenario with the implementation, same commit.

**Goal:** take `GET /api/v1/resilience/evolution-of-trust/household` from all-zeros to a *real*
(not fabricated) `commitmentBackedCollectives ≥ 1` on alpha, by implementing the §11 decision
(uniform JSON-list classification + typed accessor) and then the per-pod membership work that
raises the count to the topology ceiling — with the honest ceiling stated up front so the
"deploy+reseed is the whole lever" walk-back does not recur a third time.

## Honest ceiling (read FIRST — this calibrates every "done")

The current seed is **3 humans, 2 households** (dowell={matthew, jessica}, eden={adam}).

| Metric | Reachable | By |
|---|---|---|
| `commitmentBackedCollectives = 1` (matthew) | ✅ | Sprint 1 — storage redeploy + reseed to clean list-form data |
| `commitmentBackedCollectives = 2` (matthew + adam) | ✅ | Sprint 2 — per-pod seeding + self-heal (design-gated) |
| `commitmentBackedCollectives ≥ 3` / `protectionStatus: protected` | ❌ | structurally impossible — needs ≥3 seeded households |
| `stewardingCollectives > 0` | ❌ (this arc) | Sprint 4 — live P2P `distribute_shards`; no seed path |

So this arc lights the card from dark to **partial (real non-zero commitment count, capped at
2)**. "Protected / stewarding" is a separate, larger arc (Sprint 4 + more seeded households).

## Verified root cause (operator cluster probes U1–U4, 2026-06-19 — supersedes the pre-probe RCA)

- **U1:** matthew's provide row is present, `active`, `provider == humans.agent_pub_key`,
  `h_app_id='lamad'`, content reach `commons` — **perfect except** `resource_classified_as` is the
  JSON-array string `["content:commons"]` while the card join does scalar `.eq("content:commons")`
  → no match → 0. The column is inconsistently serialized across the table (the action-polymorphism
  in non-commons §11.2).
- **U2:** matthew's `humans` row is fully healed (`agent_pub_key` non-NULL, `household_id` non-NULL).
  The humans side and self-heal are **not** the blocker; economic attribution is internally
  consistent.
- **U3:** adam's `humans` table = 0 rows — the seed single-targets matthew via doorway; adam's pod
  never received its own `register` INSERT, so adam's self-heal `NotFound`-skips (no session → 401).
- **U4:** content reach `commons` both sides — confirms the U1 mismatch is format-only.

## Architecture / composition

- **Sprint 1** implements non-commons §11 Option A: `rea_commitments.resource_classified_as` becomes
  **uniformly a JSON list**, accessed through a typed seam; the three internal scalar-`.eq()` readers
  (card, peer-selection, distribution) move to membership. Removes the action-polymorphism that
  caused the bug, lights matthew's column on existing data, forward-fits the weighted model.
- **Sprint 2** raises the count to the topology ceiling (adam) by making seeding per-pod so each
  pod's already-deployed `genesis_self_heal` can run — **carries a p2p-design-gate** (the
  self-asserted cell-key→economic-attribution stopgap; durable cross-signed replacement deferred).
- **Sprint 3** is the non-commons spec's own Stage A/B (household/community content shows commitment-
  backed) — DNA-hash-moving, operator-owned ceremony. Referenced, not duplicated.
- **Sprint 4** is the EPR durability arc (`stewardingCollectives` via real P2P) — referenced.

## Global Constraints (verbatim — every task inherits)

- elohim-storage builds keep ambient `RUSTFLAGS=--cfg getrandom_backend="custom"`; set
  `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/elohim__elohim-storage/dev` (fall
  back to a `/tmp` target dir on a fingerprint ENOENT). Plain `cargo test` (NO nextest in this
  container). Disk is at the 85% soft watermark — prefer `--lib` targeted tests over full builds;
  reclaim per policy if a build needs room.
- **Commit-only; the integrator pushes/merges/deploys.** Autonomous mode never `git push`, never
  `kubectl`. The repo manifests are the cleanup surface.
- **Do NOT relax any verify/snapshot assertion to force green.** The card reading 0 is a real
  condition to fix at the source, never a gate to loosen.
- Shared worktree — selective staging; never bulk-revert ambient mods. Push needs `--no-verify`
  (the pre-push husky hook crashes on eslint-plugin-sonarjs load — tooling, not our code).
- `feat/*` / `sprint/*` are NOT orchestrator-indexed — none of these stages are CI-verifiable from
  this session; CI is the operator dev-merge + redeploy. State that honestly per task.

---

## SPRINT 1 — Light the card for matthew (count = 1). Storage repo work, TDD, household-nodes-testable.

> **✅ LANDED 2026-06-19, commit `27746ce6e`** (committed on `feat/frontend-eyes-sprint`, integrator
> merges/deploys). Verified: 1688 lib + 32+5+19 integration tests pass, 0 failed; fmt clean;
> clippy-clean in scope (3 pre-existing crate-wide clippy errors in untouched files —
> `integration-prepush-preexisting-gate-debt.md`). Accessor + all readers (incl. the latent-buggy
> `custody.rs:145`) + writer convergence landed. Lights `commitmentBackedCollectives=1` on operator
> deploy; matthew's existing `["content:commons"]` row matches via membership (no reseed needed for
> count=1, though a reseed cleans the historical drift to uniform list per §11).

### Task 1.1 — Typed classification accessor on the commitment row (the seam)

**Why:** §11 Option A requires that no reader touches the raw `resource_classified_as` string; all
access goes through one tolerant seam so the column can be uniformly a JSON list without every
caller re-implementing parse logic.

**Files:** `elohim/elohim-storage/src/db/rea_commitments.rs` (the `ReaCommitment` model + an `impl`).
Reuse `crate::rea_projection::parse_json_strings` (`rea_projection.rs:401`) — confirm in Step 1 it
returns `vec![s]` for a bare non-JSON string and `[]` for NULL/empty (adjust the accessor's fallback
if it returns `[]` on a bare string — wrap bare → single-element).

**Interface (produces):**
- `ReaCommitment::classifications(&self) -> Vec<String>` — parse `resource_classified_as` via the
  tolerant seam (JSON array → elements; bare string → one element; NULL/empty → `[]`).
- `ReaCommitment::has_classification(&self, c: &str) -> bool` — membership over `classifications()`.
- `ReaCommitment::primary_classification(&self) -> Option<String>` — `classifications().into_iter().next()`.

- [ ] **Step 1:** Read `parse_json_strings` and write a failing unit test asserting all four shapes
  (`["a","b"]` → `["a","b"]`; bare `a` → `["a"]`; `null`/`None` → `[]`; `["content:commons"]` →
  `has_classification("content:commons") == true`).
- [ ] **Step 2:** Run it, verify FAIL (accessor doesn't exist).
- [ ] **Step 3:** Implement the three accessor methods.
- [ ] **Step 4:** Run, verify PASS. `cargo test --lib db::rea_commitments -- --nocapture`.

### Task 1.2 — Card join uses the accessor (the keystone — lights count=1)

**Why:** U1 — the lone blocker. The scalar `.eq(&scope)` at `household_resilience.rs:189` misses
matthew's `["content:commons"]`. Move the scope predicate from SQL scalar-eq to a Rust membership
test via the accessor.

**Files:** `elohim/elohim-storage/src/services/household_resilience.rs` (`snapshot`, ~159–198).

**Approach:** keep the SQL filters that are cheap and correct (h_app_id, `action IN (...)`,
`state='active'`, `humans` join, `humans.household_id IS NOT NULL`) but **drop the
`resource_classified_as.eq(&scope)` SQL filter**; instead select the candidate rows' `(household_id,
resource_classified_as)`, and in Rust keep only rows whose `has_classification(&scope)` holds, then
count `DISTINCT household_id` (collect into a `HashSet`). Row counts here are small (provide rows per
content); the load+filter is trivial cost and removes the encoding dependency.

- [ ] **Step 1:** Write a failing test (beside existing `household_resilience` tests, or a new
  `#[cfg(test)]` with `test_pool`): seed a healed `humans` row (`agent_pub_key`, `household_id`) +
  a `provide`/`active`/`h_app_id='lamad'` `rea_commitments` row with `resource_classified_as =
  ["content:commons"]`; assert `snapshot(...).commitment_backed_collectives == 1`. (RED today —
  scalar eq misses the array.)
- [ ] **Step 2:** Run, verify FAIL.
- [ ] **Step 3:** Restructure the join to load candidates + Rust membership + distinct count.
- [ ] **Step 4:** Run, verify PASS. Add regression cases in the same module: bare `content:commons`
  still counts (1); `["content:household"]` with scope `content:commons` counts 0 (non-member);
  `["content:household","content:commons"]` counts (multi-element membership); a provider with NULL
  `household_id` counts 0 (correct-but-dormant honesty).
- [ ] **Step 5:** `cargo test --lib household_resilience -- --nocapture` all PASS.

### Task 1.3 — Peer-selection and distribution readers use the accessor (consistency, behavior-preserving)

**Why:** the other two scalar-`.eq()` readers must parse the now-uniform list, or they silently
empty on array rows (the same bug class, in the distribution/selection paths). Behavior-preserving on
bare rows; newly-correct on array rows.

**Files (complete production-reader census 2026-06-19):** `peer_selection.rs:122` (scalar `.eq`);
`distribution_view.rs:556` (scalar `.eq`); **`reconcile/custody.rs:145`** — reads the raw column as
a bare blob hash (`commitment.resource_classified_as.as_ref()`); it is **latent-buggy TODAY** on the
array-wrapped custody rows the U1 probe found, so route it through `primary_classification()`.
(`conductor_writes.rs:387` is a `#[test]` round-trip — form-symmetric, safe; no change.)

> **Safe order (advisor 2026-06-19):** migrate EVERY reader (1.2 + this task incl. custody) to the
> accessor BEFORE converging any writer (1.4). Correctness lives in the readers, not the stored
> form; once all readers are form-agnostic the writer change cannot break anyone. The custody read
> is the integration trap a per-task test misses — it needs its own custody test.

- [ ] **Step 1:** For each, write a failing test: a candidate commitment with the scope/blob_hash
  stored as a one-element JSON list is selected/matched (custody: a `["sha256-…"]` row is acted on).
  (RED — scalar eq / raw `.as_ref()` misses it.)
- [ ] **Step 2:** Run, verify FAIL.
- [ ] **Step 3:** Route each through the accessor (`has_classification` for the filters,
  `primary_classification()` for custody's bare-hash read). Preserve all other predicates exactly.
- [ ] **Step 4:** Run, verify PASS, including a bare-form regression for each (still matches).

### Task 1.4 — Converge producers on the JSON-list form + document the contract

**Why:** stop the drift at the source so the column is uniformly a list going forward (the
flip-both-together discipline). The seeder already writes a list (`seed-provide-rows.ts:241`
`resourceClassifiedAs: [scope]`); the **side-projection writes a bare scalar** and must converge.

**Files:** `elohim/elohim-storage/src/db/rea_commitments.rs:380`
(`record_provide_from_content_commitment` — `resource_classified_as: Some(&scope)`); audit the
custody writer (`reconcile/custody.rs`) for the same; the column doc comments at
`rea_commitments.rs:351` and the model field.

- [ ] **Step 1:** Write a failing test: `record_provide_from_content_commitment(... reach="commons")`
  persists `resource_classified_as` such that `row.classifications() == ["content:commons"]` (a JSON
  list, not bare). (RED — it writes bare today.)
- [ ] **Step 2:** Run, verify FAIL.
- [ ] **Step 3:** Change the writer to persist `serde_json::to_string(&vec![scope])` (one-element
  JSON list). Audit and converge any sibling single-valued writer (custody) the same way. Leave
  `operate-doorway` (already a list) untouched.
- [ ] **Step 4:** Run, verify PASS. Update the doc comments (`= [content:<reach>]`, a JSON list) and
  the model field comment to state the uniform-list contract + point at the accessor.
- [ ] **Step 5:** Confirm the output `ReaCommitmentView` (`views_convert/shefa.rs:119`) and the
  doorway auth path (`shefa.rs:181`) are unaffected (they already parse a list) — add an assertion
  that a side-projection row now round-trips through `ReaCommitmentView` with a non-None
  `resourceClassifiedAs` (it nulled on the old bare form per `api/rea_commitments.rs:110`).

### Task 1.5 — a2o scenario + commit (story-first)

- [ ] Extend `genesis/a2o/features/resilience/` (or the durability-arc Workstream-E coverage) with a
  scenario asserting: a healed household with an active commons provide commitment renders a non-zero
  `commitmentBackedCollectives`. Household floor (M/J/J); no `@requires:shem`.
- [ ] Commit (commit-only; integrator merges):
  ```
  fix(storage): uniform JSON-list resource_classified_as + typed accessor — lights commitment-backed card

  Per 2026-06-13-non-commons-provide-commitments-design §11 (Option A). The resilience
  card read 0 because the provide row stored ["content:commons"] (JSON list) while the
  join did scalar equality (the action-polymorphic column drift). Make the column
  uniformly a JSON list, accessed via ReaCommitment::{classifications,has_classification,
  primary}; move the 3 internal scalar-eq readers (card, peer_selection, distribution) to
  membership; converge the side-projection writer on the list form. No reseed. Removes the
  polymorphism that misled the live probe; forward-fits weighted affinity (§11.3).

  Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
  ```

### Sprint 1 operator step (keystone, not code)

Redeploy alpha storage from dev (image rebuild + rolling restart — **NOT** `ALLOW_DNA_REINSTALL`;
no DNA change). **No reseed needed** (matthew's existing row matches via membership). Verify:
`GET /api/v1/resilience/evolution-of-trust/household → commitmentBackedCollectives == 1`.

---

## SPRINT 2 — Raise the count to the ceiling (adam → count = 2). Seeder-to-matthew orchestration. **(REVISED 2026-06-19 on a verified cross-pod trace — `sprint2-crosspod-count2-trace` workflow.)**

> **The original framing ("per-pod registration so adam's self-heal runs") was WRONG — discarded.**
> The card reads **matthew's** local `content.db` only (`household_resilience.rs:134-144` — one pooled
> local SqliteConnection, no peer/DHT read; doorway single-targets matthew, `route_registry.rs:704-709`).
> Healing adam on *adam's* pod puts nothing on matthew. So count=2 requires adam's rows **physically in
> matthew's db**, and per-pod self-heal can't deliver that.
>
> **This is NOT economic-attribution-gated** (corrects the prior gate note). The join
> `humans.agent_pub_key = rea_commitments.provider` is `agent_cid = agent_cid`, intra-namespace,
> consuming ZERO `AgentPeerBinding`s — the gospel's transport-binding gate (`elohim-storage/CLAUDE.md`)
> does **not** fire here. It's an orchestration/wiring fix, not a security-gated design change.

### The real mechanism (verified)
Two rows must land in **matthew's** content.db, both via the **seeder POSTing through the doorway** (so
matthew's conductor authors them — `create_rea_commitment` has no `provider==author` guard, integrity
`_ => Ok(Valid)`):
- **(a) adam `humans` row** — `id=human-adam-firstman`, `household_id='household-eden'`,
  `agent_pub_key=`adam's real `uhCAk…`. `seed-humans.ts:270-280` already deposits the row but with
  `agent_pub_key=NULL`; `seed-provide-rows.ts:281` `POST /api/v1/identity/heal` stamps adam's fetched key.
- **(b) adam provide-row** — `provider=`adam's `uhCAk…`, `action='provide'`, **`state='active'`**,
  `h_app_id='lamad'`, `resource_classified_as ⊇ content:commons`. `seed-provide-rows.ts:257`
  `POST /api/v1/commitments`.

NOT via the live signal path (`post_commit` is author-local, `content_store/src/lib.rs:10616`) NOR
membership projection (`on_membership_projected` is a NULL-only `household_id` UPDATE that never sets
`agent_pub_key` / never inserts, `controller.rs:1105-1115`). Both confirmed-absent for this purpose.

### Task 2.0 — PROBE FIRST (operator, blocking — decides "confirm" vs "fix")
Query matthew's `content.db`:
- `SELECT id, agent_pub_key, household_id, h_app_id FROM humans WHERE id='human-adam-firstman' OR household_id='household-eden';`
- `SELECT provider, action, state, resource_classified_as, h_app_id FROM rea_commitments WHERE provider LIKE 'uhCAk%' AND action IN ('provide','replicates-content','replicates-commons');`
Expected: adam humans row present, `household_id=eden`, `agent_pub_key=NULL` (→ Gap 1 live). A
`provider=adam` commitment absent (seeder didn't reach matthew for adam) OR present-but-`proposed` (→ Gap 2 live).

### Task 2.1 — Confirm seed order + target (seed-humans → seed-provide-rows, both to matthew)
Verify `seed-humans` runs before `seed-provide-rows`, both POST through `RESOLVED_DOORWAY_HOST`
(= matthew single-target). TDD: after `seed-humans`, matthew's db has the adam `household-eden` row
(`agent_pub_key=NULL`). The heal 404s if the row is absent (`seed-provide-rows.ts:281-287,440-444`).

### Task 2.2 — Stamp adam's real key (Gap 1 — plain wiring)
Ensure `seed-provide-rows.ts` `fetchAgentPubKey` (adam's `/auth/me`) + `healHumanIdentity` run **for
adam against matthew**. TDD: matthew's adam row ends with `agent_pub_key` = the fetched key (non-NULL,
byte-identical to the provide-row `provider`).

### Task 2.3 — Provide-commitment state = `active` (Gap 2 — **THE silent-failure gate**, verify-first)
**This is the gap that silently fails count=2 even with everything else correct:** adam's key stamped
right, the humans row present, the provide-row authored with the right `provider` — and the card STILL
reads 1, not 2, if that row landed `state='proposed'`. No amount of identity wiring lights eden past a
`proposed` row. Treat Task 2.0's state probe as blocking, not advisory.
`household_resilience.rs:192` filters `state='active'`. DHT wire projections force `proposed`; the
`POST /api/v1/commitments` → `rea_commitment_service.rs:247-250` path must land `active` (or add an
activation/graduate step). **Confirm the actual written state (Task 2.0) before choosing the fix** —
memory `project_resilience_snapshot_humans_junction` warns POST commitments inserts `'proposed'`.
TDD: matthew's `rea_commitments` has `provider=adam, action='provide', state='active', resource ⊇ content:commons`.

### Task 2.4 — End-to-end card assertion (the Sprint 2 done-gate)
Through doorway: `GET /api/v1/resilience/evolution-of-trust/household → commitment_backed_collectives == 2`.
Story-harvest a regression scenario capturing the two-row precondition.

### Task 2.5 — (FLAG, not block) name the self-asserted `provider` seam
`provider` is a self-asserted payload field with no `provider==author` integrity check
(`mishpat_integrity/src/lib.rs:766-804`). Acceptable for low-stakes card display; **must be named**
before this join feeds anything weightier. Record as a p2p-design-gate note (distinct from the
transport-binding gate) — not a Sprint 2 blocker.

### Sprint 2 operator step
Redeploy + reseed (seed-humans → seed-provide-rows reach matthew for adam). Verify
`commitment_backed_collectives == 2`. jessica shares matthew's household (dowell) so she does **not**
raise the count — eden (adam) is the only second household in this seed.

---

## SPRINT 3 — Non-commons content shows commitment-backed. Execute the non-commons spec Stage A/B (DNA-gated).

Out of scope to redesign — this is `2026-06-13-non-commons-provide-commitments-design` Stage A
(storage-side: read the payload reach, eligibility filter via `classify_pre_authorization`) + Stage B
(Mishpat integrity reach-gate generalization — **DNA-hash-moving**, operator-owned reinstall ceremony
for the alpha bootstrap pair). Lights `commitmentBackedCollectives` for household/community content.
Sequenced AFTER Sprint 1 (the accessor + list contract is its substrate). Tracked in that spec; this
plan only names the dependency.

---

## SPRINT 4 — `stewardingCollectives > 0`. The EPR durability arc (real P2P distribute_shards).

Out of scope here — folds into `2026-06-10-epr-durability-replication-arc-plan`. `shard_locations`
(the stewarding join target) is written ONLY by runtime `distribute_shards` over a healthy
multi-peer mesh; no seed path exists. Needs the leak fix deployed (mesh not collapsed) + live
distribution. Named dependency; the card's `stewarding`/`protected` columns wait on it.

---

## Complementary captures (do NOT absorb — one-line backlog items, linked)

- **`/db/humans` read-scope (imagodei vs lamad).** `register` writes `h_app_id='imagodei'` but
  `GET /db/humans` hard-forces `lamad` (`http.rs:3637`) → reads empty, which **misled the live
  probe** (matthew's row exists; it was filtered out). Legibility/F-COHERENCE fix; it is the same
  scope-split the qahal gap names ("decide ONE scope, flip both together"). Home:
  `qahal-collective-cid-formation-projection-gap.md`. Not the card blocker; capture, don't bundle.
- **Steward-gate circularity (formation stuck 1/3).** Founder's Steward Membership not DHT-integrated
  when co-members affirm (`DepMissingFromDht`); a seeder formation-ordering/settle-retry fix. Home:
  `qahal-collective-cid-formation-projection-gap.md`. Independent of the card (the snapshot doesn't
  require formation — self-heal sets `household_id` directly).
- **Weighted (fractional) affinity model (non-commons §11.3).** The richer `Map<classification,
  weight>` direction the operator surfaced. Captured born-linked in the spec; **do NOT auto-build** —
  it's a DHT-entry / economic-attribution change behind its own brainstorm → p2p-design-gate → plan,
  operator/security-owned. Sprint 1's uniform-list is its on-ramp (list→map).

---

## Done (stability-gated, not single-green)

- Sprint 1 merged; on the next dev redeploy `commitmentBackedCollectives == 1` (matthew) with no
  reseed; the action-polymorphism is removed (column uniformly a JSON list, accessor-only reads);
  peer-selection/distribution/doorway-auth/output-view all consistent on the list form.
- Sprint 2 merged (post-gate); per-pod seeding lands adam's healed `humans` row + session; count == 2.
- Sprint 3/4 named as dependencies with their owning specs/plans; not claimed done here.
- The honest ceiling held in front: nobody re-asserts "protected/stewarding" without the seeded-
  households + distribute_shards work.

## Self-review

- **Coverage:** U1 → Sprint 1 (the keystone); U2 → already-green (no work, stated); U3 → Sprint 2;
  U4 → guards Sprint 1's scope. The card's four columns each map to a sprint or a stated ceiling.
- **No placeholders:** Sprint 1 carries complete TDD steps + exact files/lines; Sprints 2–4 are
  scoped to their owning specs (intentional, not stubs).
- **Type consistency:** `classifications()/has_classification()/primary()` (1.1) are used exactly as
  defined by 1.2/1.3; the writer change (1.4) produces the form the accessor reads.
- **Gate honesty:** Sprint 2 carries the p2p-design-gate explicitly; the weighted model is captured-
  not-built behind its own gate.
