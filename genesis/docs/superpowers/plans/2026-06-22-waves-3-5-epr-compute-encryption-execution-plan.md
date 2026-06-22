---
title: "Waves 3–5 execution — EPR-head drill-downs · REA compute-contracts facing · private-replica encryption proof"
id: waves-3-5-epr-compute-encryption-execution-plan
status: Draft
class: protocol-canonical
domain: D5
sprint: follow-on
topic: [epr-heads, epr-composite-renderer, rea, compute-contracts, delegates-compute, mishpat-rea-bridge, compute-fulfilled, private-replica, encryption, tiered-weave]
refines:
  - genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - genesis/docs/superpowers/plans/2026-06-08-epr-slice1-lens-complete-resolver-plan.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
cites:
  - resiliency-card-p2p-weave-sprint-plan | the parent sprint plan; this drains its FOLLOW-ON Waves 3–5 (gap-items #16/#17/#18), inheriting its binding constraints + honest-verification boundary | path: genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - epr-slice1-lens-complete-resolver-plan | Wave 3 drains its Tasks 2/3/4 (Task 1 claims-302 demotion already landed); the epr-composite renderer is the keystone | path: genesis/docs/superpowers/plans/2026-06-08-epr-slice1-lens-complete-resolver-plan.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law Wave 3's relationships-population serves (knowledge leg from existing head relations, NOT the held ClusterClosure walk) | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - rea-economic-facing-lens-design | Wave 4 lands its slice sequence (REA folds proof-gate → MishpatCommitmentView + route → mishpat→rea delegates-compute bridge + compute-fulfilled) | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - weave-epic-arc-design | Wave 5 lands #4's Slice-0 single-host encryption round-trip proof; live encryption (KeyEnvelope + ShardManifest field-add + X25519 substrate) HELD | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - elohim/elohim-storage/src/epr_head.rs
  - elohim/elohim-storage/src/services/sealed_against_self.rs
  - elohim/elohim-storage/src/sharding.rs
  - elohim/elohim-storage/src/services/rea_commitment_service.rs
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env — each gap
# inherits household-nodes; only the cross-doorway / live-alpha legs carry inline @requires.
---

# Waves 3–5 execution

> **EXECUTION plan, not a new design.** Every wave `refines:`/`cites:` a settled spec/plan — the
> design lives there. This doc's value-add is the **slice → task → test** breakdown, the **sequence**,
> and the **honest-verification boundary** (locally-verified-and-committed vs alpha/a2o-held → operator)
> carried forward from the parent sprint. Branch `feat/frontend-eyes-sprint`; integration target `dev`.

## Binding constraints (carried forward — violating these re-treads dead paths)

- **Wave 5 LANDMINE — do NOT touch `p2p/mod.rs:1492`.** The `content_reach:"commons"` hardcode is safe
  *only* while encryption is absent. If the reach-derivation TODO there resolves before live encryption,
  private content plaintext-leaks to every custodian. The Slice-0 proof is a self-contained new file;
  it must leave that TODO untouched (parent constraint §"Encryption-ordering correctness edge").
- **Wave 4 new-view recipe is non-negotiable** (route-shadow trap is runtime-only): schema → Rust struct
  (`#[serde(rename_all="camelCase")]`+`#[derive(TS)]`) → `schema_contract` test → `INTERFACE_FILES` →
  `pnpm run schema:codegen:ts` + `cargo test export_bindings` → declarative `Route` manifest entry **+ the
  `test_manifest_builds` path-coverage assertion** (a unit test passing without it still shadows at runtime,
  the `/auth/portal` incident shape — `project_doorway_main_route_needs_is_service_path`).
- **DHT is a notary, not a byte-store; no third sync dialect; no new DHT entry type** except the HELD
  `KeyEnvelope` (Wave 5 live, not in scope). The mishpat→rea bridge (Wave 4) is **projection only**.
- **p2p-design-gate already cleared** in the source specs (rea §"P2P Design Gate output", weave §"#4");
  do not re-run it. Commit-only; integrator pushes. **Another session interleaves commits on this branch**
  — sequence the storage-touching waves in the main worktree; selective-stage; no parallel worktrees on
  `elohim-storage`.
- **Native build env:** `RUSTFLAGS=""` `RUSTC_WRAPPER=""` `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/elohim__elohim-storage/dev`.
  ts-rs `export_bindings` is the one place the WASM flag is correct: `RUSTFLAGS='--cfg getrandom_backend="custom"'`.

## Sequence — by verifiability-now, not size: **5.1 → 4 → 3**

5.1 is fully self-contained and fully closeable today. 4 follows the WeaveView recipe just run in Wave 2.
3 is the most cross-cutting (Rust + TS renderer + a2o) and its a2o-acceptance leg is alpha-held — but it
serves the user's original *"epr links on the card → drill-downs"* want most directly, so all its Rust/TS
units land this sprint (only the live a2o run defers).

---

## Wave 5.1 — private-replica encryption Slice-0 proof (land-now, DB-free)
**Goal (household-felt):** prove a private blob can be encrypted, erasure-coded, scattered, reconstructed,
and decrypted back to byte-identical plaintext — the cryptographic floor under "your data is held by peers
but unreadable to them," before any live key substrate exists.

| Slice | Files | PR shape (TDD) | Land/Held |
|---|---|---|---|
| 5.1 round-trip proof | NEW `elohim/elohim-storage/src/services/private_replica.rs` (+ `services/mod.rs` reg) | RED test first: `plaintext → random DEK → encrypt → RS `create_shards` (reuse `sharding.rs:249`) → drop a shard → reconstruct ciphertext → decrypt → assert `plaintext_cid` (sha256/blake3 of original) byte-identical`. DEK sealed/unsealed via dryoc `crypto_box_seal`/`_open` with generated keypairs (reuse `sealed_against_self.rs:32` imports). | **LAND-NOW** |
| — live encryption | `ShardManifest` field-add (`encryption`+`plaintext_cid`); new `KeyEnvelope` entry; X25519 reader-key resolver | (not in scope) | **HELD** — substrate-blocked (X25519 not sourced from `agent_cid`) + conductor-leak-gated + ordering-edge |

**Verify:** `cargo test --lib private_replica` (pool slot) + clippy -D warnings + fmt. No DB, no network.
**Honesty boundary:** fully locally-verified-and-committed. Nothing alpha-held.

---

## Wave 4 — REA compute-contracts facing (land-now; cross-doorway verification alpha-held)
**Goal (household-felt):** the economic lens shows who *promises* to host (intent) vs what was *realized*
(observed), and surfaces reciprocal `delegates-compute` agreements — mutual hosting reads as observation,
not intent.

| Slice | Files | PR shape (TDD) | Land/Held |
|---|---|---|---|
| 4.1 REA folds proof-gate | NEW `elohim/elohim-facings/src/folds/rea.rs` (+ `folds/mod.rs`) | DB-free pure folds + tests: `commitment_backed(rows)`, `by_action(rows)→BTreeMap`, `mutual_compute(rows)→reciprocal delegates-compute pairs`. Hand-built `CommitmentRow` fixtures; assert non-zero/buckets/reciprocity. Mirror `operational_weave.rs` fold style. | **LAND-NOW** |
| 4.2 `MishpatCommitmentView` + route | `elohim-views/src/infrastructure.rs` (view); `sdk/schemas/v1/views/mishpat-commitment-view.schema.json`; `codegen-ts.mjs` `INTERFACE_FILES`; storage adapter + declarative `Route` (`GET /api/v1/commitments?facing=rea` or `/api/v1/weave`-sibling) | **full new-view recipe** (see binding constraints) incl. `schema_contract` test + `test_manifest_builds` assertion (RED→GREEN). | **LAND-NOW** |
| 4.3 mishpat→rea bridge + compute-fulfilled | mirror existing content-provide bridge (`mishpat_projection.rs`/`rea_commitment_service.rs`); projection on `action="delegates-compute"`; `compute-fulfilled` EconomicEvent `bounded_by` the commitment | projection-only (no new DHT entry); TDD the projection mapping + the `bounded_by` linkage so `mutual_compute` reads observed. | **LAND-NOW** (impl) |
| — cross-doorway mutual-compute | live two-doorway reciprocal hosting | `@requires:shem` + running stack | **HELD** — alpha degraded (verification, not impl) |

**Verify:** `cargo nextest`/`cargo test --lib` (facings + storage, pool slots) + clippy + fmt; codegen freshness;
`pnpm look` the route once alpha recovers.
**Honesty boundary:** 4.1/4.2/4.3 impl locally-verified-and-committed; live cross-doorway mutual-compute → operator (alpha).

---

## Wave 3 — EPR-head drill-downs read honest (Rust/TS land-now; a2o-acceptance alpha-held)
**Goal (household-felt):** the resilience card's "epr links to drill-downs" (the user's original want) reach
**real destinations** — a path-as-EPR renders a clean lens-complete outline, not the raw-JSON fallback, with
its knowledge leg populated and an "Open in {pillar}" affordance.

Prereq check: resolver-plan **Task 1 (claims-302 demotion) — VERIFY it landed** (ledger says ✅; `http.rs`
`dispatch_epr_universal` Default→`ServeShell`). If not landed, do it first (it gates 3.1–3.4 mattering).

| Slice | Files | PR shape (TDD) | Land/Held |
|---|---|---|---|
| 3.1 `epr-composite` renderer (KEYSTONE) | NEW renderer under `app/lamad/src/app/renderers/`; register in `content-io.module.ts` | Vitest: epr-composite node (sections/items fixture) → navigable outline, one `/epr/{ref}` link per item (via `eprToUniversalHref`, never a literal), root `data-testid`, degrades on empty body. | **LAND-NOW** |
| 3.2 "Open in {pillar}" affordance | `content-viewer.component.{ts,html}` | Vitest: claimed type → affordance present w/ right cross-bundle href; unclaimed → absent. `data-testid`. | **LAND-NOW** |
| 3.3 populate `epr_head.rs:148 relationships` | `epr_head.rs` enrich=true path (mirror how `attestation_requirements` fills from `query_direct_prerequisites`) | fill from existing `EprRelationship` head relations (`query_direct_relationships`-style); **NOT** the transitive `ClusterClosure` walk (design-only, acquisition #11 — HELD). Rust unit test: enrich=true returns head relations; enrich=false stays diesel-only `vec![]`. | **LAND-NOW** |
| 3.4 a2o 302-inversion scenario | `genesis/a2o/features/lamad/deep-link-delivery.feature` + steps + selectors | invert the old 302 assertion → `/epr/{claimed-path}` renders lens-complete (composite outline + knowledge leg + Open-in-pillar affordance), no 302; `@regression`; testid-sync. | impl LAND-NOW; **live run HELD** (alpha) |
| — value-leg substrate | `provide-content` REA action + scorer (acquisition #7–9) | (not in scope) | **HELD** — own slice; value data partly flows from Wave 4 |
| — ClusterClosure walk | transitive typed-relation closure (acquisition §5.1 / #11) | (not in scope) | **HELD** — design-only substrate |

**Verify:** doorway `cargo test --lib --bins` (if Task 1 touched); `pnpm --filter lamad` vitest + route-literal lint;
storage `cargo test` (3.3); a2o `tsc`+`eslint`+testid-sync (3.4). Live a2o render → operator (alpha).
**Honesty boundary:** 3.1/3.2/3.3 + 3.4-authoring locally-verified-and-committed; the live 302-inversion a2o run → operator.

---

## Done when
- **5.1:** `private_replica.rs` round-trip proof green (cargo+clippy+fmt), committed. p2p/mod.rs:1492 untouched.
- **4:** REA folds + `MishpatCommitmentView` + GET route (+ `test_manifest_builds` guard) + delegates-compute
  bridge + compute-fulfilled all green and committed; codegen fresh. Cross-doorway live → operator handoff.
- **3:** epr-composite renderer (no raw fallback) + Open-in-pillar + relationships populated + 302-inversion
  scenario authored, all gates green and committed. Live a2o render → operator handoff.
- Per-wave honest-verification line stated; gap-ledger updated; nothing asserted "done" that only an alpha/a2o
  run can confirm.

## Held / operator-owned (surfaced, not attempted)
- Live encryption substrate (X25519 reader-key resolver; KeyEnvelope; ShardManifest field-add) — security-owned.
- Cross-doorway mutual-compute observation; live EPR 302-inversion a2o render — alpha degraded → operator.
- Value-leg `provide-content` substrate + ClusterClosure transitive walk — own slices (composed, not forked).
- Revert `pool-policy.json` watermarks (88/92 → 75/85) after this sprint — temporary "for now" bump.
