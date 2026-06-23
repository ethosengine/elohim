# HANDOFF — Resilience cards + dual-doorway EPR sprint (post-leak-unblock), 2026-06-18

## North star & definition of done

Carry forward the protocol's resilience / content-mobility thesis on the now-stable live dataplane: (1) flip the household **resilience cards** from "0 of X" to **real peer counts** — peers actually providing/hosting content, the card counting how many; and (2) serve the **same EPR through BOTH doorways** (matthew-fronted `doorway-alpha` / `alpha.elohim.host` and adam-fronted apex `elohim.host`), because the content is replicated across peers — not centralized. The conductor leak that prevented a stable mesh is mitigated; this sprint does the *real* provide-pipeline + identity work the stable mesh now makes runnable.

**Definition of done — split into what the sprint controls vs. what the operator fork gates:**

- **(i) Sprint-controlled, household-provable code (the committed deliverable):**
  - Seeded content is DHT-anchored at ingest and passes `require_provenance` (provide-able, dual-doorway-serveable).
  - HTTP content-read path honors reach (`check_reach_authorization` wired on the serve handlers).
  - The resilience proof-suite is green at the Layer-1 / protocol-chaos baseline (`cargo test --test household_resilience` 28 tests + `--test chaos_dataplane` 4 tests), and the F-COHERENCE + F-EDGE doorway detectors compile and pass `--lib`.
  - F-BOOTSTRAP verified live (both doorways report `backend: "mongo"` + identical spaces/agents).

- **(ii) Live-alpha card-lights — CONTINGENT on the operator probe fork (NOT in the sprint's unilateral control):**
  - `pnpm look` the resilience card on live alpha shows a real **"N of X"** (N ≥ 1), AND the storage resilience snapshot route returns `commitmentBackedCollectives ≥ 1` / `householdsStewarding ≥ 1` with `distributionState: "measured"`.
  - The same EPR **CID** (`bafy…`, not a deploy-version SHA) resolves and serves through BOTH doorways (verify via `GET /api/v1/federation/coherence` reporting `in_agreement: true` over content heads).

DoD (ii) is real and is the north star — but it depends on the `GET /auth/me` fork below resolving on the 200 branch (or an operator-gated design landing on the 401 branch). The sprint must deliver (i) and *light* (ii) where the fork allows; it must NOT promise (ii) as if it owns it.

## The blocker is cleared — do NOT re-chase it

The conductor off-heap leak (native glibc-malloc arena retention; go-pion exonerated) is **mitigated** as of 2026-06-18 by switching the conductor to jemalloc, deployed fleet-wide on alpha via dev commit **`b8481f090`** (a TEMP profiling build). Conductors are now stable (0 OOM-restarts, anon bounded ~2GB, releasing memory) and the genesis seed **survives** the Database step that used to kill it — the live dataplane can finally run. **Rule: this sprint does NOT work the leak.** If OOM-flapping returns, the path is the non-prof jemalloc conductor (or an optional `jeprof` dump read), and you escalate — you do not re-diagnose arenas mid-sprint. **Two leak-track open loops are NON-sprint cleanup, kept off the plate:** (1) revert the temp prof deploy `b8481f090` after a non-prof jemalloc conductor ships; (2) optional `jeprof` dump read to confirm zero residual code-leak. Both are leak-track, not the goal.

## The real gates to a non-zero card (the frontier)

A stable conductor is **necessary but not sufficient.** Three gates stand between a healthy mesh and a non-zero card. Three independent cluster maps (provide-durability, resilience-cards, seed-gates) separately traced gate (a) with line numbers and converged on the same finding — it is the crux.

**(a) Identity / join population — THE structural hole, and it is the sprint's fork.**
The resilience snapshot joins on the canonical Holochain `agent_cid` (`uhCAk…`) across two surfaces in `elohim/elohim-storage/src/services/household_resilience.rs`: `compute()` (lines 71–84, `shard_locations.peer_id == humans.agent_pub_key`, filtered `household_id IS NOT NULL`), `snapshot()` (lines 168–198, `rea_commitments.provider == humans.agent_pub_key`), and `compute_regional_distribution` (lines 446–452). **Both joins need `humans.agent_pub_key` populated with the `agent_cid`.** Traced writers: `on_membership_projected` (`elohim/elohim-storage/src/reconcile/controller.rs:1114`) sets **only `household_id`**, never `agent_pub_key`; the ONLY writer of `agent_pub_key` is `heal_human_identity` via `POST /identity/heal` (`elohim/elohim-storage/src/api/identity.rs:149`), gated on `GET /auth/me` returning a key, which requires a `LocalSession` that **no genesis/seed/boot path on a headless server pod creates**. So on a live pod, `agent_pub_key` stays **NULL → both joins empty → all-zeros card, independent of the leak and the mesh.** The identity-namespace trap compounds it: raw-string equality between an `agent_cid` (`uhCAk…`) and a libp2p transport id (`12D3Koo…`) silently empties any join — and the provide-loop's `resolve_provider` (`conductor_commitment_author.rs:180`) falls back to `self_cid` (the transport id) when no session `agent_cid` exists.
  - **Provider side is now correct on the tree** (`agent_cid` written as `rea_commitments.provider`, commit `8c217137c`; `self_cid` derive-at-startup landed, `main.rs:400-513`). **The humans side (`agent_pub_key`) is still the open structural hole.** Do NOT inherit the dataplane map's "largely closed by Workstream D" — that map did not trace the humans-side writer; the others did, with line numbers.
  - **The 2026-06-15 "identity contract" in `resilience-card-self-cid-provide-loop-gate.md` (`agent_pub_key` set by `MembershipProjected`) is FALSE / superseded.** Treat the doc's **2026-06-18 contradiction block** as current.

**(b) Live content provide rows (Epic B).** `content:<reach>` provide rows are written today only in `test_util`, NOT on the live plane. Owner: `genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2b-provide-loop-plan.md` (all 14 tasks landed on branch; commons reach) + `genesis/docs/superpowers/plans/2026-06-13-non-commons-provide-implementation-plan.md` (extends to all reaches; Stage A landed, **Stage B unlanded** — `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:836` still hard-codes commons). Bulk-seeded content can't even *reach* a provide row until it passes the provenance gate — see the bulk-seed anchor item below.

**(c) Active (not proposed) commitments.** `POST commitments` inserts `state: "proposed"` (`elohim/elohim-storage/src/db/rea_commitments.rs:432`); the snapshot counts only `state == 'active'`, and `find_active_operator_binding` resolves only `ACTIVE_PROVIDE_STATES = active/accepted/in-progress`. Commitments-only seeding lights NOTHING. The slice-2a graduation rail (`proposed→active`) is the landed path; the graduation-audit sweep (slice-2b follow-on #1) hardens it.

## Deferred-plan landscape — what to execute, in order

Sequenced so measurement is re-enabled first, then the live data, then the proofs, then the two axes. Each item: plan path · one-line next action · dependencies.

### Stage 0 — Re-enable measurement (seed / identity gates)

1. **Dev-merge the branch + genesis re-run.** `genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md`. Next action: integrator dev-merges this branch (carries `7afa03337` projection-sync + `83a0669e3` delivery-events, both landed on branch, NOT dev) so the next genesis run exercises Tasks 1 & 2 against the stable jemalloc conductor; confirm `ci-genesis-household-founder-binding`'s `DepMissingFromDht` layer clears on the stable conductor. Deps: operator deploy + reseed.

2. **THE OPERATOR FORK — run `GET /auth/me` on a healthy pod after jemalloc deploy + reseed.** `genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md` (2026-06-18 block, the keystone). This decides whether north-star #1 is one operator action away or needs a gated design:
   - **200 + agentPubKey** → deploy+reseed is the whole lever; `agent_pub_key` populates; flip the gate resolved; the card can light.
   - **401** → the structural finding holds; a session/key-population path is needed — **and that path is p2p-design-gate + security-owned (economic attribution), blocked by `genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`. DO NOT auto-build the session/key path.** Build the *stopgap* design only through the gate; the resolver itself is §0-demoted to speculative.

### Stage 1 — Autonomous, leak-independent, household-provable code (the sprint's committed work)

3. **Bulk-seed anchor step (ONE fix, two backlog docs).** `genesis/data/timeline/backlog/seed-provenance-anchor-gap.md` + `genesis/data/timeline/backlog/ci-seeder-stamp-conductor-anchor-circularity.md`. Next action: settle the p2p-design-gate question (should `require_provenance` exempt creator-scoped reads?), then add `dhtAnchorHash` to `CreateContentInputView`/`UpdateContentInputView` at `elohim/elohim-views/src/lamad.rs:141` (verified absent) so seeded content is DHT-authored at ingest and passes the provenance gate. Top code lever toward both axes — provenance-gated content is exactly what the card can't count and the second doorway can't serve. Leak-independent, household-provable.

4. **HTTP reach enforcement.** `genesis/data/timeline/backlog/http-reach-enforcement-gap.md`. Next action: route the HTTP content-read handlers (`elohim/elohim-storage/src/http.rs:3743-3756` `GET /db/content/{id}`; `http.rs:6807` `GET /epr-head/{id}`) through `check_reach_authorization` (`epr_service.rs:311`, currently called only from the P2P path `epr_service.rs:100`), deriving `agent_cid` from the doorway-proxied `X-Agent-Cid`; flip the two `@wip` scenarios in `intimate-reach-household.feature`. Correctness gate for dual-doorway serve. Leak-independent.

5. **Non-commons Stage B (DNA-hash-moving integrity arm).** `genesis/docs/superpowers/plans/2026-06-13-non-commons-provide-implementation-plan.md`. Next action: implement B1 reach generalization at `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:836` (accept `replicates-content|replicates-commons`; content branch → `reach ∈ REACH_LEVELS` ∧ `idx(ceiling) ≥ idx(reach)`; capacity stays commons-pinned) with the inline `REACH_LEVELS` const and a `hc dna hash` before/after proof-of-exactly-one-move; then B2 limit arm + B3 sweettest/a2o. Extends card-lighting from the commons demo blob to the *majority* of real content. Commit-only-ready; **DEPLOY needs the adam+matthew reinstall ceremony (operator)** — both genesis peers must get `ALLOW_DNA_REINSTALL` or they land on different DNA hashes → P2P partition. Smaller alt: graduation-audit sweep (`epr-slice2b-provide-loop-followons.md` #1).

### Stage 2 — Dataplane proofs (peers actually replicate)

6. **Proof-suite green baseline + re-observe the durability arc.** `genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md` + the arc `genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md` (+ `-11` pickup, `-12` overnight handoff). Next action: run `cargo test --test household_resilience` (28) + `--test chaos_dataplane` (4) to establish the green Layer-1 / protocol-chaos baseline now (no mesh dependency; container quirks: `/tmp` target-dir, custom-getrandom RUSTFLAGS). Then run the arc plan's **own Phase 0 re-observation on the now-stable mesh** — pull latest `substrate-verify-*.json` + per-pod `/p2p/status` counters (`kicksFiredTotal`, `reconcilePassesTotal`, `placementGapsEmittedTotal`) and re-baseline the frozen `propagation.custody-convergence` measure (last read **1** under the flapping mesh — that trace is STALE; a non-converging DHT leg is what an unstable conductor produces). Pick the workstream from fresh counters. The pure combinatorial proofs (P-PROOFS Task 1, `tests/rs_reconstruct_property.rs`, `genesis/docs/superpowers/plans/2026-06-14-dataplane-proofs-plan.md`) are zero-dep and directly prove the replication-durability thesis. Mesh-hardening roots (both leak-independent, additive): P-DEFENSE Task 1 `jittered` backoff (`2026-06-14-dataplane-defense-plan.md`); P-TRANSPORT Task 1/3 `connection_limits` floor (`2026-06-14-dataplane-transport-plan.md`).

### Stage 3 — Resilience card lights (north-star #1)

7. **Card honesty + un-wip the live proof.** `genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md` (Part 1). Next action: land the `onlinePeerCount → {live, known}` num/denominator pair on `ResilienceSnapshotView` (schema → Rust struct → `schema_contract` test → `INTERFACE_FILES` codegen → snapshot component; extend D1/D2 fixtures) — pure, CI-green-now (the `distributionState` half already shipped, `household_resilience.rs:150-155`). Then, **once the Stage-0 fork resolves on the 200 branch**, un-wip the D3 row (`commitmentBackedCollectives ≥ 1`) against live alpha — the first true dataplane card-light.

### Stage 4 — Dual-doorway (north-star #2)

8. **Verify F-BOOTSTRAP live (zero new code).** `genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md` (landed). Next action: `GET https://alpha.elohim.host/admin/bootstrap-coherence` + `https://elohim.host/admin/bootstrap-coherence`; assert both report `backend: "mongo"` + identical spaces/agents, then confirm matthew↔adam appear in each other's peer list (`/p2p/status` / Loki). Highest-information cheap check — tells you if the genesis pair converged on the stable mesh. (The `#[ignore]` cross-pod test has never run against live mongo.)

9. **F-COHERENCE detector (standalone).** `genesis/docs/superpowers/plans/2026-06-14-federation-coherence-plan.md`. Next action: create `doorway/doorway-service/src/routes/coherence.rs` (`EprHeadFingerprint` + `CoherenceManifest` + pure `router_fingerprint` over sorted `(url_path, epr_id)`), add `head_fingerprints()` to `epr_router.rs`, TDD `cargo test --lib coherence` (`RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test`). The only "two heads" ever observed were **deploy-version SHAs, not content CIDs** — live content divergence is currently **unmeasured**; this builds the instrument.

10. **F-EDGE peer-count honesty.** `genesis/docs/superpowers/plans/2026-06-14-federation-edge-plan.md`. Next action: in `doorway/doorway-service/src/routes/federation.rs`, extract pure `project_p2p_peers(&Value)` reading `connectedPeers`, add `connected_peer_count: Option<usize>` to `P2PPeersResponse`, retire the dead `headless_service_base` StatefulSet branch (`:231`) and `total = peers.len()` (`:256`); TDD `cargo test --lib federation`. **CRITICAL: this is NOT the resilience card** — it makes the doorway's `/p2p-peers` count truthful (1→~13), a different surface in a different crate. Do not read "F-EDGE → resilience cards." F-DEPLOY (`2026-06-14-federation-deploy-plan.md`) Task 1 (`ALLOW_COORDINATOR_UPDATE` env + manifest, commit-only — edge Jenkinsfile is operator-applied) is the deploy-coherence backstop, gated on F-COHERENCE's endpoint.

### SUPERSEDED / stale — do NOT send the sprint at these

- **P-DIAGNOSTIC** (`2026-06-14-dataplane-diagnostic-plan.md`) — its core (surface the anchor gap) already shipped as `elohim/elohim-storage/src/services/provide_loop_status.rs` (`selfCidSource`/`active`/`reanchor`) on `/p2p/status`, richer than the plan's `AnchorView`. Only residual: thread the *existing* status through doorway's composed `/admin/self-healing` view. Don't build `AnchorView`.
- **P-ACTUATION contract** (`2026-06-14-dataplane-actuation-plan.md` Tasks 1/4) — superseded by landed `trait Governor` + `Refusal`/`LimitOwner` (`elohim-compute/src/actuation.rs`, commit `732fdaa69`); names `Actuation`/`ScopeId`/`GrantBounds` don't match reality. Only the S13 `sets-authority-arc` dead-path projection arm is a live residual, and must be re-verified against the evolved `mishpat_projection.rs` first.
- **Landing-page ROOT_APP_SLUG plan** (`genesis/docs/superpowers/plans/2026-05-23-landing-page-epr-dual-doorway.md`) — central mechanism abandoned (B14: drop `ROOT_APP_SLUG`; doorway now consults live projection); seeder + `stageSpaBlob` work landed/repaired (`832855d5b`, `ae9869d37`). Verify-live (`pnpm look` both surfaces), don't re-run Tasks 0-8. Carries the gate-(c) trap (operator binding lands `proposed`, inert).
- **`epr-blob-replication-direction.md`** — resolved-by-implementation (substrate-P2P direction chosen + built). Mark superseded; rationale record only.
- **`ci-genesis-doorway-503-seed-phase-wedge.md`** — cure already in tree (`4dc862748`+`54d2bb737`); deploy-propagation-blocked; and it is a **DIFFERENT bug** (doorway warm_stream wedge, `project_doorway_wedge_unbounded_mongo_await`), NOT the conductor leak — do not conflate.
- **`ci-genesis-epr-lens-composite-outline-not-deployed.md`** — source coherent/unit-green; app-bundle deploy-coherence gap; needs a clean app rebuild+redeploy, not a code fix.
- **`ci-genesis-seeding-suspended-peer-unstable.md`** — fix landed 2026-06-08; only confirm-by-CI-disappearance remains.
- **Off-goal entirely:** `atproto-lexicon-projection` (web2 interop axis), `household-mobility-seams` (lifecycle *design*, not the `agent_pub_key` fix), `doorway-recovery-reconstruction-residue` (self-described low-priority cleanup), `resilience-tier-content-declared-floor` (needs `/brainstorm`; refines a denominator on a card that doesn't light yet — lowest urgency), `arch-dataplane-sdk-proposal` (blocked on `/brainstorm`).

## Recommended sprint Objective (/shift-ready)

> **Objective:** Land the leak-independent, household-provable substrate that lets the resilience card and dual-doorway content equality become *measurable and lightable* on the now-stable jemalloc mesh — specifically: (1) DHT-anchor seeded content at ingest (`dhtAnchorHash` on `CreateContentInputView`/`UpdateContentInputView`) so it passes `require_provenance`; (2) enforce reach on the HTTP content-read serve path; (3) establish the green resilience proof-suite baseline (`household_resilience` 28 + `chaos_dataplane` 4) and the F-COHERENCE + F-EDGE doorway detectors; (4) verify F-BOOTSTRAP live. **Done when:** those land green in CI **and are verified on live alpha** (provenance-gated content serves; `/api/v1/federation/coherence` measures content heads; both doorways report shared mongo bootstrap), **AND** the operator `GET /auth/me` probe has been run to decide the card-lighting fork. The live card showing real **"N of X"** is the goal but is **gated on the 200 branch of that probe** — deliver it where the fork allows; surface the 401 branch to the operator as a security/design-gated blocker, do not auto-build the session path.

**Guardrails:**
1. **Don't work the leak.** It's mitigated (jemalloc, `b8481f090`). Reverting the temp prof deploy + optional `jeprof` read are NON-sprint cleanup. If OOM-flapping returns, escalate to the non-prof conductor path — don't re-diagnose arenas.
2. **Run the `GET /auth/me` fork BEFORE writing live provide rows.** On 401, **do NOT auto-build the session/key-population path** — it's p2p-design-gate + security-owned (economic attribution), blocked by the coherent-transport-identity-resolver spec. Surface it; don't build it.
3. **Measure on live alpha, not just CI.** "Landed on dev" ≠ "visible on alpha." Use `pnpm look` the card and the storage snapshot route / `/admin/bootstrap-coherence` / `/api/v1/federation/coherence` — CI green is necessary, not the DoD.
4. **household-nodes is the stable floor.** Deep-prove on the M/J/J live multi-peer mesh; only matthew↔adam cross-node convergence (dual-doorway, F-BOOTSTRAP live) needs the live genesis pair / shem.
5. **snake_case never leaves Rust.** New view fields go schema → Rust `#[serde(rename_all = "camelCase")]` → `schema_contract` test → codegen; no TS-side transforms.
6. **Edge Jenkinsfile + cluster ops are operator-owned.** F-DEPLOY / DNA-reinstall / coordinator-update manifest work is commit-only; never `kubectl`, never apply. Stage-B DNA deploy needs the adam+matthew reinstall ceremony (both peers, or P2P partition).
7. **F-EDGE peer-count ≠ resilience card.** Two distinct surfaces/crates — don't conflate the doorway `/p2p-peers` honesty fix with the household card flip.

## First 3 moves

1. **Establish the green proof baseline (no mesh, no fork needed):**
   `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/hr-test cargo test --test household_resilience` and `--test chaos_dataplane` in `elohim/elohim-storage/` — confirm 28 + 4 green. This is the executable acceptance gate everything else hangs off.

2. **Run the operator fork probe** (gates north-star #1 card-lights): after the integrator dev-merges this branch and a reseed runs on the jemalloc-stable alpha, `GET /auth/me` on matthew's `elohim-storage` pod. Record 200+agentPubKey (→ deploy+reseed is the lever; proceed to light D3) vs 401 (→ surface the security-gated session-path blocker to the operator; do NOT build it). Reference: `genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md` (2026-06-18 block, lines ~286-294).

3. **Start the top autonomous code lever — the bulk-seed anchor:** open `elohim/elohim-views/src/lamad.rs:141`, settle the `require_provenance` creator-read-exemption design-gate question, then add `dhtAnchorHash` to `CreateContentInputView`/`UpdateContentInputView` so seed/import writes a content-derived anchor at ingest (one fix closing `seed-provenance-anchor-gap.md` + `ci-seeder-stamp-conductor-anchor-circularity.md`).

## Key references

**Gate (a) — identity / join population (the fork):**
- `genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md` (keystone; 2026-06-18 contradiction block is current, 2026-06-15 contract is FALSE)
- `genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md` (BLOCKED; §0-demoted — build stopgap only, through the gate)
- Seams: `elohim/elohim-storage/src/services/household_resilience.rs:71-84,150-155,168-198,446-452` · `reconcile/controller.rs:1114` (sets only `household_id`) · `api/identity.rs:149` (`heal_human_identity`, sole `agent_pub_key` writer) · `conductor_commitment_author.rs:180` (`resolve_provider`) · `main.rs:400-513` (`self_cid` derive)

**Gate (b) — live provide rows / provenance:**
- `genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2b-provide-loop-plan.md` (commons, landed) + `specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md`
- `genesis/docs/superpowers/plans/2026-06-13-non-commons-provide-implementation-plan.md` (Stage A landed; Stage B unlanded) + `specs/2026-06-13-non-commons-provide-commitments-design.md`
- `genesis/data/timeline/backlog/seed-provenance-anchor-gap.md` + `ci-seeder-stamp-conductor-anchor-circularity.md` (one fix) + `http-reach-enforcement-gap.md`
- Seams: `elohim/elohim-views/src/lamad.rs:141` (no `dhtAnchorHash`) · `mishpat_integrity/src/lib.rs:836` (commons hard-code) · `http.rs:3743-3756,6807` · `epr_service.rs:100,311` (`check_reach_authorization`)

**Gate (c) — active commitments:**
- `genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md` (graduation rail) · `genesis/data/timeline/backlog/epr-slice2b-provide-loop-followons.md` (#1 graduation-audit sweep)
- Seam: `elohim/elohim-storage/src/db/rea_commitments.rs:432` (`proposed`), `ACTIVE_PROVIDE_STATES`

**Proofs / durability (north-star #1 evidence):**
- `genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md` · `2026-05-29-durability-topology-felt-resilience.md`
- `genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md` (+ `-11`, `-12`) · `2026-06-14-dataplane-proofs-plan.md` · `-defense-plan.md` · `-transport-plan.md`
- `genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md` (Part 1, card honesty)
- Tests: `elohim/elohim-storage/tests/household_resilience.rs` · `tests/chaos_dataplane.rs` · `genesis/a2o/features/resilience/resilience-dimensions.feature` · `chaos-peer-churn.feature`

**Dual-doorway (north-star #2):**
- `genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md` (landed) · `-coherence-plan.md` (not started) · `-edge-plan.md` (not started) · `-deploy-plan.md` (not started)
- `genesis/docs/superpowers/plans/2026-05-23-landing-page-epr-dual-doorway.md` (SUPERSEDED — B14 live-projection)
- Seams: `doorway/doorway-service/src/routes/federation.rs:231,256,277` · `epr_router.rs` (`head_fingerprints`) · `_edgenode-consolidated.template.yaml:262` (`ALLOW_COORDINATOR_UPDATE` comment only)

**Seed stabilization / CI:**
- `genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md` (Tasks 1/2 on branch: `7afa03337`, `83a0669e3`)
- `genesis/data/timeline/backlog/ci-genesis-household-founder-binding.md` (re-verify `DepMissingFromDht` on stable conductor)

**Leak-track (NON-sprint cleanup):** revert temp prof deploy `b8481f090` after non-prof jemalloc conductor ships; optional `jeprof` confirmation.
