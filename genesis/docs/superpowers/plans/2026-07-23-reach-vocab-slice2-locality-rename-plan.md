---
title: "Reach Reconciliation Slice 2 — locality rename + ReachClass schema-8 alignment + stragglers"
id: reach-vocab-slice2-locality-rename-plan
status: Landed
landed: 2026-08-05
verified_by: |
  Verified against the tree 2026-08-05 during the worktree commit sweep. The
  graduation-trigger's conditions are met: LocalityLevel is canonical in
  elohim/sdk/storage-client-ts/src/protocol-core.model.ts and consumed across the app;
  app/lamad/src/app/models/trust-badge.model.ts carries the "slice 2, 2026-07-23" provenance
  for replacing the inlined ReachLevel copy; and elohim-service/src/cache/types.ts carries the
  LOCALITY_LEVEL_VALUES keep-in-sync note this plan prescribes. Steps below were never ticked
  as the work landed — treat them as an authoring record, not a queue.
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once slice 2 lands (locality rename live, ReachClass schema-8, gap-matrix updated)
created: 2026-07-23
topic: [reach, locality, vocabulary-drift, reach-class, distribution, slice-2]
cites:
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:d0303e0209f57b76 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
  - elohim/elohim-views/src/infrastructure.rs
  - elohim/elohim-storage/src/services/distribution_view.rs
---

# Reach Reconciliation Slice 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the TS geographic-8 out of "reach" into a **locality** vocabulary (deprecated aliases keep all 37 consumers compiling); align `ReachClass` (views/distribution) to the canonical schema-8 — fixing the live replica-target degradation where schema-8 declared-reach strings fall to `Private`/2-replicas; migrate the retired-6 stragglers the slice-1 final review found; update the resilience README gap-matrix + custody vocabulary naming.

**Architecture:** Spec §1 dispositions (`2026-07-22-reach-ontology-vocabulary-split-spec.md`): geographic-8 → locality/placement vocabulary (single SDK edit point, other sites re-export); `ReachClass` is a *distribution* projection of DECLARED reach and must speak schema-8 (View Schema Contract: schema JSON first → Rust → contract test → codegen TS); retired-6 inline stragglers re-point to canonical `Reach`; data values migrate per pinned mapping. **Out of scope:** verdict surface, announcement slot, steward/node's dormant 6-value enum (own crate, `#[allow(dead_code)]` — next slice), burning down the deprecated `ReachLevel` aliases across the 37 consumers (slice 3).

**Tech Stack:** TypeScript (SDK storage-client-ts, elohim-app, lamad, elohim-library), Rust (elohim-views, elohim-storage), JSON view schemas + codegen, plain cargo test.

## Global Constraints

- Storage/views cargo (from `/projects/elohim/elohim/elohim-storage`): `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev`. Plain `cargo test` (no nextest). fmt + clippy `-D warnings` before commit.
- Branch: continue on `shift/reach-vocab-slice1` is WRONG — create `shift/reach-vocab-slice2` from the slice-1 head. Commit-only; never push/amend; path-limited commits.
- Canonical schema-8 (order): `private, self, intimate, trusted, familiar, community, public, commons`.
- Legacy→canonical mapping (same table as slice 1): personal→self · household→trusted · neighborhood→familiar · collective→community · district→public · (stored-data-only) old-public→commons.
- Retired-6→canonical mapping (NEW, pinned here): private→private · invited→intimate · local→trusted · community→community · **federated→public** · commons→commons. (`federated` sat between community and commons: visible beyond the community boundary but not commons-held ⇒ `public`.) Non-schema stray `agent-private`→`private`.
- Locality vocabulary values are UNCHANGED (`private/invited/local/neighborhood/municipal/bioregional/regional/commons`) — only the *names* (`ReachLevel`→`LocalityLevel` etc.) move; values are locality semantics and never collided with declared reach.
- Replica-target ladder for schema-8 `ReachClass` (pinned): private 2 · self 2 · intimate 4 · trusted 6 · familiar 8 · community 12 · public 14 · commons 16.

---

### Task 0: Branch

- [ ] **Step 1:** `cd /projects/elohim && git checkout shift/reach-vocab-slice1 && git checkout -b shift/reach-vocab-slice2`. Expected: `Switched to a new branch`.

### Task 1: SDK locality rename + deprecated aliases + site consolidation

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` (~lines 36–124: type, const, `reachEncompasses`)
- Modify: `app/elohim-app/src/app/elohim/models/protocol-core.model.ts` (duplicate definition ~lines 50–72)
- Modify: `app/lamad/src/app/models/trust-badge.model.ts` (~lines 20–28 inlined copy)
- Modify: `app/elohim-library/projects/elohim-service/src/cache/types.ts` (~lines 19–40 numeric const 0–7)

**Interfaces:**
- Produces (later tasks + consumers rely on): `export type LocalityLevel` (same 8 string values) · `export const LOCALITY_LEVEL_VALUES: Record<LocalityLevel, number>` (same 0–7 ordinals) · `export function localityEncompasses(source, target): boolean` · deprecated aliases `ReachLevel`/`REACH_LEVEL_VALUES`/`reachEncompasses` re-exporting the new names.

- [ ] **Step 1 (SDK — the single edit point):** In `elohim/sdk/storage-client-ts/src/protocol-core.model.ts`, rename `ReachLevel`→`LocalityLevel`, `REACH_LEVEL_VALUES`→`LOCALITY_LEVEL_VALUES`, `reachEncompasses`→`localityEncompasses` (update internal references, including the `reach: ReachLevel` field type annotations in this file — field NAMES stay `reach` for wire-compat; only the TYPE is renamed). Rewrite the doc comment header:

```ts
/**
 * LocalityLevel — geographic/placement scope (dataplane: replication, eviction,
 * caching). RENAMED from "ReachLevel" 2026-07-23: this vocabulary is NOT
 * declared content reach (that is the schema-8 `Reach` enum, generated from
 * elohim/sdk/schemas/v1/enums/reach.schema.json). See spec:
 * reach-ontology-vocabulary-split-spec §1.
 */
```

Then append the compat block:

```ts
/** @deprecated Renamed 2026-07-23 — use LocalityLevel. Burn-down tracked in reach-vocabulary-frontend-strand. */
export type ReachLevel = LocalityLevel;
/** @deprecated Renamed 2026-07-23 — use LOCALITY_LEVEL_VALUES. */
export const REACH_LEVEL_VALUES = LOCALITY_LEVEL_VALUES;
/** @deprecated Renamed 2026-07-23 — use localityEncompasses. */
export const reachEncompasses = localityEncompasses;
```

- [ ] **Step 2 (app duplicate):** In `app/elohim-app/src/app/elohim/models/protocol-core.model.ts`, delete the duplicated type/const/function bodies and re-export from the SDK IF the app already imports `@elohim/storage-client` (check `package.json`/existing imports — it does, via generated types). Replace the deleted block with:

```ts
export {
  LocalityLevel,
  LOCALITY_LEVEL_VALUES,
  localityEncompasses,
  // deprecated aliases pass through for existing consumers:
  ReachLevel,
  REACH_LEVEL_VALUES,
  reachEncompasses,
} from '@elohim/storage-client';
```

If a type-only export is required by lint (`isolatedModules`), split into `export type { LocalityLevel, ReachLevel }` + value exports. If the file defines OTHER local members referencing the old names, update them to the new names.
- [ ] **Step 3 (trust-badge inline copy):** In `app/lamad/src/app/models/trust-badge.model.ts`, delete the inlined copy and `import type { LocalityLevel } from '@elohim/storage-client';` (lamad already imports the SDK — see `content-node.model.ts:31`), updating local references. Keep exported names this file's consumers use as thin aliases if any exist (grep first).
- [ ] **Step 4 (cache numeric const):** In `app/elohim-library/projects/elohim-service/src/cache/types.ts`: elohim-service does NOT depend on `@elohim/storage-client` — do a rename-in-place (`ReachLevel`→`LocalityLevel` in names/comments, same numeric values) and add a one-line drift-comment: `// Mirrors LOCALITY_LEVEL_VALUES in @elohim/storage-client/protocol-core.model.ts — keep in sync (no dep edge; see reach-vocabulary-frontend-strand).` Update in-package references (grep `ReachLevel` within `projects/elohim-service`).
- [ ] **Step 5: Gates**

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm test 2>&1 | tail -4
cd /projects/elohim/app/elohim-app && pnpm run lint 2>&1 | tail -3 && pnpm exec vitest run --config vite.config.ts src/app/elohim 2>&1 | tail -4
cd /projects/elohim/app/lamad && pnpm exec tsc --noEmit -p tsconfig.json 2>&1 | tail -5
```
Expected: green / pre-existing-only reds (stash-compare any new red and EMBED the before/after outputs in the report).
- [ ] **Step 6: Commit** `git add elohim/sdk/storage-client-ts/src/protocol-core.model.ts app/elohim-app/src/app/elohim/models/protocol-core.model.ts app/lamad/src/app/models/trust-badge.model.ts app/elohim-library/projects/elohim-service/src/cache/types.ts && git commit -m "refactor(reach): geographic-8 renamed to LocalityLevel — locality is placement, not declared reach (slice 2; deprecated ReachLevel aliases keep consumers compiling)" -- <same four paths>`

### Task 2: ReachClass → schema-8 (View Schema Contract order; fixes live replica-target degradation)

**Files:**
- Modify: `elohim/sdk/schemas/v1/views/distribution-summary.schema.json` (+ `placement-gap-row.schema.json` if it enumerates reach_class values)
- Modify: `elohim/elohim-views/src/infrastructure.rs:1600–1615` (`ReachClass` enum)
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs` (`replica_target_for`, `parse_reach_class`)
- Regenerate: `elohim/sdk/storage-client-ts/src/generated/ReachClass.ts` (+ `DistributionSummary.ts` if changed) via `cargo test export_bindings`; consumer codegen via `pnpm run schema:codegen:ts`
- Test: existing `elohim/elohim-storage/tests/schema_contract.rs` + `distribution_view` unit tests

**Interfaces:**
- Produces: `ReachClass` with variants `Private, SelfScope (#[serde(rename = "self")] under rename_all snake_case — verify emitted string is exactly "self"), Intimate, Trusted, Familiar, Community, Public, Commons`; `replica_target_for` per the pinned ladder (private 2 · self 2 · intimate 4 · trusted 6 · familiar 8 · community 12 · public 14 · commons 16); `parse_reach_class` accepting schema-8 strings AND legacy aliases per the Global-Constraints mapping (reuse slice-1 semantics; legacy "public"→`Commons` here IS correct because this parses STORED declared-reach data written under the old vocabulary — document this divergence from `parse_reach_key` in a comment).

- [ ] **Step 1 (schema first):** Update the `reach_class`/`reachClass` enum arrays in the view schema JSON(s) to `["private","self","intimate","trusted","familiar","community","public","commons"]`.
- [ ] **Step 2 (failing contract):** Run the schema contract test — expect FAIL (Rust enum still kebab-8): `cd /projects/elohim/elohim/elohim-storage && <env> cargo test --test schema_contract 2>&1 | tail -5`.
- [ ] **Step 3 (Rust):** Migrate the `ReachClass` enum variants in `infrastructure.rs`; update `replica_target_for` to the pinned ladder; update `parse_reach_class` (schema-8 + legacy aliases, stored-data reading: `"public"`→`Commons`, `"district"`→`Public`); fix compile fallout inside elohim-views/elohim-storage ONLY (compiler-driven, mapping table verbatim). The `unwrap_or(ReachClass::Private)` conservative fallback STAYS.
- [ ] **Step 4 (regenerate):** `cargo test export_bindings` (regenerates `ReachClass.ts` etc.); then `cd /projects/elohim && pnpm run schema:codegen:ts`. Verify `git diff --stat` on generated files shows the expected vocabulary change (schema-8 strings) and note the known Prettier oscillation on unrelated generated files — do NOT commit unrelated oscillation; path-limit the add.
- [ ] **Step 5 (test the degradation fix):** Add/extend a `distribution_view` unit test proving a content row with declared reach `"trusted"` yields `replica_target == 6` (pre-fix it fell to Private/2), and `"commons"` yields 16. Full gate: `cargo fmt && cargo build && cargo test --lib && cargo test --test schema_contract && cargo test --test reach_vocabulary_contract && cargo clippy -- -D warnings`.
- [ ] **Step 6: Commit** (schema + rust + regenerated TS + tests; message: `"fix(distribution): ReachClass speaks schema-8 — declared 'trusted/familiar/commons' no longer degrade to 2-replica Private floor (slice 2; view-schema-first, legacy stored values alias-parsed)"`).

### Task 3: Retired-6 stragglers + data hygiene

**Files:**
- Modify: `app/lamad/src/app/models/knowledge-map.model.ts:~501` (inline retired-6 union on `reach`)
- Modify: `app/lamad/src/app/models/content-node.model.ts:~150` (comment referencing `federated`)
- Modify: `app/elohim-library/projects/elohim-service/src/models/content-node.model.spec.ts:~81` (asserts `invited < federated` ordering)
- Modify: `genesis/data/lamad/attestations/index.json` (`"reachGranted": "federated"` row)
- Modify: whatever `grep -rn "agent-private" genesis/data --include="*.json"` finds (3 rows expected)

- [ ] **Step 1:** knowledge-map inline union → the canonical declared-reach type: `import type { Reach } from '../<path>/generated/schema-enums'` (find lamad's generated schema-enums location via `grep -rn "schema-enums" app/lamad/src | head -3`; use whatever canonical schema-8 `Reach`/union type lamad already consumes; if none is importable, inline the schema-8 union with a `// generated-from reach.schema.json — do not hand-extend` comment). Fix the `content-node.model.ts` comment likewise.
- [ ] **Step 2:** The elohim-library spec asserting retired-6 ordering: read the surrounding describe — if it tests a still-live retired-6 structure, migrate its fixture to the locality or schema-8 vocabulary the model NOW carries; if the structure it tests died in slice 1/this task, delete the case (justify in report).
- [ ] **Step 3 (data):** `"federated"`→`"public"`; `"agent-private"`→`"private"` per the pinned retired-6 mapping. Then validate: `cd /projects/elohim && pnpm run schema:test 2>&1 | tail -4` (or the seeder validation the repo exposes — `pnpm run schema:validate`; run whichever exists per root package.json and report which).
- [ ] **Step 4:** Gates: lamad `tsc --noEmit`, elohim-service `pnpm test`, plus Task 1's app gates if any shared file was touched. Commit path-limited: `"refactor(reach): retire the last inline retired-6 strand — knowledge-map/spec/data migrate to canonical vocabularies (slice 2; federated→public, agent-private→private)"`.

### Task 4: Docs — gap-matrix, custody naming, strand disposition

**Files:**
- Modify: `genesis/docs/content/elohim-protocol/resilience/README.md` (gap-matrix row "Reach enum drift — Rust … divergent from schema enum | GAP"; Part-V custody vocabulary text)
- Modify: `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md` (append slice-2 disposition)

- [ ] **Step 1:** Rewrite the gap-matrix GAP row to name all five strands + dispositions: schema-8 CANONICAL (slice-1 contract-tested) · services kebab-8 RETIRED (slice 1) · TS `VALID_REACH_LEVELS`-6 RETIRED (slice 1) · TS geographic-8 RENAMED→LocalityLevel (slice 2, aliases pending burn-down) · Part-V 5 = **custody vocabulary** (named, distinct from reach). Status GAP→PARTIAL (remaining: alias burn-down, steward dormant enum, verdict surface).
- [ ] **Step 2:** In the Part-V custody-example region, add one clarifying sentence where the 5-value ladder appears: these are **custody** tiers (who holds/replicates for whom — `CustodianCommitment`/`Mishpat::Commitment` lineage), not reach levels — per spec §1.
- [ ] **Step 3:** Append slice-2 disposition to the strand doc (locality rename live w/ deprecated aliases + 37-consumer burn-down open; ReachClass schema-8 + replica-fix; stragglers/data migrated; steward dormant enum still open).
- [ ] **Step 4:** Commit: `"docs(reach): gap-matrix five-strand dispositions, custody vocabulary named, slice-2 strand disposition"`.

---

## Self-review notes
- Spec coverage: DoD item 2 fully; item 5 extended (ReachClass consumer). Verdict surface (item 3), fixtures harness (4), a2o composition-law scenarios (6) remain later slices. P2P design gate: no new entity/table/route — vocabulary alignment on existing views; view-schema change follows the View Schema Contract process.
- Judgment pins: federated→public; agent-private→private; replica ladder extends old 16-cap to commons with public 14; `parse_reach_class` stored-data "public"→Commons DIVERGES from `parse_reach_key` deliberately (reading old stored data vs parsing canonical wire keys) — both sides documented at the definition.
- Risk: generated-file churn (Prettier oscillation memory) — path-limit adds; lamad has no test suite wired (`tsc --noEmit` is the gate there).
