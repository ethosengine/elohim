---
title: "Reach Reconciliation Slice 3 — retired-6 library strand, alias burn-down, policy canonicalization, content.reach backfill"
id: reach-vocab-slice3-burn-down-plan
status: In-progress
class: substrate
landing_state: |
  Partially landed as of 2026-08-05 (verified against the tree during the worktree commit
  sweep, not by ticking steps). DONE: the SDK alias burn-down (ReachLevel survives only as a
  rename comment) and bootstrap-standing-policy.json canonicalization to the 8-value vocab.
  OPEN: the retired-6 library strand — app/lamad/src/app/services/trust-badge.service.ts:439
  still defines getNextReachLevel over ContentReach. Pick from the open strand, not the
  checkboxes, which were never maintained.
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once slice 3 lands (aliases deleted, library strand canonical, bootstrap policy canonical, backfill migration shipped)
created: 2026-07-23
topic: [reach, locality, vocabulary-drift, alias-burn-down, backfill-migration, slice-3]
cites:
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:d0303e0209f57b76 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
  - elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json
  - elohim/elohim-storage/src/services/distribution_view.rs
  - elohim/elohim-storage/src/services/manifest_registry.rs
---

# Reach Reconciliation Slice 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the vocabulary burn-down: migrate elohim-library's self-contained retired-6 strand (`ContentReach` + trust.service `ReachLevel`) to canonical schema-8; burn down all consumers of the deprecated `ReachLevel`/`REACH_LEVEL_VALUES`/`reachEncompasses` aliases and DELETE the aliases; canonicalize `bootstrap-standing-policy.json` reachThresholds (currently missing four canonical keys → silent fallthrough to code defaults); ship the one-time `content.reach` backfill migration (legacy `public`→`commons` + all legacy/retired values → canonical) and retire the parse divergence it resolves; complete `reachIcon()` schema-8 cases; document steward's dormant enum as the locality/placement-engine seed (NOT deleted, NOT renamed).

**Architecture:** Spec §1 dispositions + §7 DoD items 1/2/5. Data-aware migration rule (spec §7.5): migrate rows first, then retire ambiguity. Drift-prevention law: locality's source-of-record is declared at the SDK edit point. Out of scope (slice 4+): verdict surface, fixture harness, announcement slot, doorway `can_serve_at_reach` re-keying, `reach: LocalityLevel`-named wire fields, app/lamad's own generated-Reach surfaces beyond alias compilation fixes.

**Tech Stack:** TypeScript (elohim-library Jest+Vitest, elohim-app Vitest, SDK tsc), Rust (elohim-storage, diesel embedded migrations), plain cargo test.

## Global Constraints

- **Branch:** create `shift/reach-vocab-slice3` from the current head of `shift/reach-vocab-slice2`. Commit-only; never push/amend; path-limited commits (`git commit -m … -- <paths>`); the worktree carries unrelated in-flight changes from concurrent sessions — never `git add -A`.
- **Canonical schema-8 (order):** `private, self, intimate, trusted, familiar, community, public, commons`.
- **Retired-6→canonical mapping (pinned, from slice 2):** private→private · invited→intimate · local→trusted · community→community · **federated→public** · commons→commons.
- **Legacy-kebab→canonical mapping (pinned, from slice 1):** personal→self · household→trusted · neighborhood→familiar · collective→community · district→public · (stored-data-only) old-public→commons.
- **Storage cargo env** (from `/projects/elohim/elohim/elohim-storage`): `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/shift/elohim__elohim-storage/dev`. Plain `cargo test` (no nextest here). `cargo fmt` + `cargo clippy -- -D warnings` before commit.
- **dist-freshness trap:** app/elohim-app and app/elohim-library resolve `@elohim/storage-client` via its compiled `dist/`. After ANY edit to `elohim/sdk/storage-client-ts/src/`, run `pnpm run build` in `elohim/sdk/storage-client-ts` BEFORE type-checking consumers, or the edit is invisible.
- **Six unrelated same-name symbols — DO NOT TOUCH:** `elohim/sdk/src/types.ts:1211,1222` (`ReachLevels`/`ReachLevel`, schema-8-valued, independent); `app/elohim-app/.../elohim-client.provider.ts:47`; `app/elohim-library/.../client/types.ts:148-165` (numeric enum `Commons=0..Private=7`) and its consumers `client/elohim-client.ts`, `client/angular-provider.ts`, `client/index.ts:110`. Also unrelated: policy-console `setReachLevel`, imagodei `ProfileReachLevels`, doorway `maxReachLevel`, shefa `ReachLevelStorage`, generated `ReachLevelStorageView`.
- **Spec-file gate blind spot (slice-1 harvest):** elohim-library `**/*.spec.ts` are BOTH eslint-ignored and tsconfig-excluded, but Jest (`pnpm run test:service`) still runs them — always run the test suite, never trust lint/tsc alone.

---

### Task 1: elohim-library retired-6 strand → canonical schema-8

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/models/content-node.model.ts` (~line 46 `ContentReach`, line ~168 usage)
- Modify: `app/elohim-library/projects/elohim-service/src/models/content-node-builder.ts` (lines 5, 23, 36)
- Modify: `app/elohim-library/projects/elohim-service/src/services/trust.service.ts` (line 14 `ReachLevel` def; ordinal map 19-26; default line 48; usages 59, 78, 137, 142, 185, 281)
- Modify: `app/elohim-library/projects/elohim-service/src/cli/import.ts` (lines 41-44) if it passes retired-6 literals
- Modify (test fixtures — Jest runs these even though lint/tsc skip them): `content-node.model.spec.ts`, `trust.service.spec.ts`, `content-node-builder.spec.ts`, `conductor-normalizer.spec.ts` (only where retired-6 literals appear)

**Interfaces:**
- Consumes: `Reach`, `ALL_REACH_LEVELS`, `REACH_OPENNESS`, `reachOpenness()`, `isReach()` from the library's OWN generated file `app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts` (lines ~319-354). Do NOT import from `@elohim/storage-client`.
- Produces: `ContentReach` stays exported from the barrel but becomes `export type ContentReach = Reach;` (alias to canonical). trust.service's local `ReachLevel` type is REPLACED by canonical `Reach` (delete the local union).

**Steps:**

- [ ] **Step 1:** In `content-node.model.ts`, replace the retired-6 `ContentReach` union with `import type { Reach } from '../generated/schema-enums';` + `export type ContentReach = Reach;`. Remove the 2026-07-23 dev note in `content-node.model.spec.ts:55-64` that pre-announces this migration (it is now done), and update spec fixtures using `invited/local/federated` per the retired-6→canonical mapping.
- [ ] **Step 2:** In `content-node-builder.ts`, keep default `'commons'` (already canonical); fix any retired-6 literals in its spec.
- [ ] **Step 3:** In `trust.service.ts`, delete the local `ReachLevel` union; import canonical `Reach` from `../generated/schema-enums`. Rewrite the ordinal map to cover all 8 canonical values in schema order (private=0, self=1, intimate=2, trusted=3, familiar=4, community=5, public=6, commons=7) — or reuse generated `REACH_OPENNESS`/`reachOpenness()` if it provides the same ordering (prefer reuse; check first). Map defaults: old `'local'` default → `'trusted'`; any `'federated'` logic → `'public'`.
- [ ] **Step 4:** WIRE-FORMAT NOTE (this is the point of the task): `enrichContentDirectory`/`updateContentIndexWithTrust` write `reach` values into seed-data JSON consumed downstream by the DHT-seed pipeline whose contract (`CreateContentInput.reach`, generated) is canonical schema-8. After this task the CLI emits canonical values — this FIXES a latent wire mismatch. Verify `cli/import.ts:41-44` passes canonical literals.
- [ ] **Step 5:** Update `trust.service.spec.ts` fixtures (~24 retired-6 literals, lines 232-559) per the mapping.
- [ ] **Step 6:** Run `pnpm run test:service` (Jest) and `pnpm test` from `app/elohim-library`; run `pnpm run lint`. All green.
- [ ] **Step 7:** Commit path-limited: `git commit -m "refactor(reach): migrate elohim-library retired-6 strand (ContentReach, trust.service) to canonical schema-8" -- app/elohim-library`

### Task 2: Alias burn-down + deletion (SDK + app + library cache)

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` — DELETE deprecated aliases at lines ~78-81 (`ReachLevel`, `REACH_LEVEL_VALUES`) and ~136-137 (`reachEncompasses`)
- Modify: `app/elohim-app/src/app/elohim/models/protocol-core.model.ts:43-55` — re-export `LocalityLevel`, `LOCALITY_LEVEL_VALUES`, `localityEncompasses` instead of the old names (keep NO deprecated re-exports)
- Modify: `app/elohim-app/src/app/elohim/models/trust-badge.model.ts:20,40` — import `LocalityLevel`; note its local `export type ContentReach = ReachLevel` is an app-side homonym of Task 1's library type: retype to `LocalityLevel`, and rename this app-local alias `ContentReach` → `ContentLocality` ONLY if no template/type consumer breaks — otherwise keep the name and add a one-line comment flagging the homonym (reviewer adjudicates)
- Modify: `app/elohim-app/src/app/elohim/models/coordination-envelope.model.ts:25,124` — `LocalityLevel`
- Modify: `app/elohim-app/src/app/shefa/models/resilience-profile.model.ts:24,125` and `resilience-profile.model.typetest.ts` (8 refs) — `LocalityLevel` direct from `@elohim/storage-client`
- Modify: `app/elohim-library/projects/elohim-service/src/cache/types.ts:20-44` — delete the deprecated `export const ReachLevel = LocalityLevel;` (line 42) and `export type ReachLevelType = LocalityLevelType;` (line 44)
- Modify: `app/elohim-library/projects/elohim-service/src/cache/index.ts:11,24,32,41-42` — barrel + JSDoc to `LocalityLevel` names
- Modify: `app/elohim-library/projects/elohim-service/src/cache/reach-aware-cache.ts:16` — JSDoc only
- Modify: any `app/lamad` (inside app/elohim-app) consumer of `REACH_LEVEL_VALUES` or cache's `ReachLevel` re-export found by the Step-3 sweep (scout flagged these exist)
- Modify: `.claude/data/deprecations.jsonl` — DELETE the two ledger lines for fingerprints `cad8d5f51f6f` and `247dc16fb9d5` (per the deprecation-triage disposition: deleting the aliases must delete the ledger lines so the sentinel re-fires as a regression signal if the tags return)

**Steps:**

- [ ] **Step 1:** Migrate every inventoried consumer to the `LocalityLevel` family, then delete the three SDK aliases and the two cache/types.ts aliases.
- [ ] **Step 2:** `cd elohim/sdk/storage-client-ts && pnpm run build` (dist-freshness), then `pnpm test` there.
- [ ] **Step 3:** **Repo-wide sweep (the real gate):** `grep -rn "ReachLevel\|REACH_LEVEL_VALUES\|reachEncompasses" app/ elohim/sdk/ --include='*.ts'` must return ONLY the six unrelated same-name symbol families listed in Global Constraints (plus Task-1's now-deleted refs = none). Fix every other hit, including app/lamad.
- [ ] **Step 4:** From `app/elohim-app`: `pnpm run build` (AOT catches typetest/strictTemplates breaks that vitest misses) + `pnpm test` + `pnpm run lint`. From `app/elohim-library`: `pnpm test` + `pnpm run lint`.
- [ ] **Step 5:** Delete the two deprecations.jsonl lines.
- [ ] **Step 6:** Commit path-limited: `git commit -m "refactor(reach): burn down deprecated ReachLevel/REACH_LEVEL_VALUES/reachEncompasses aliases; delete aliases + sentinel ledger lines" -- elohim/sdk/storage-client-ts app/elohim-app app/elohim-library .claude/data/deprecations.jsonl`

### Task 3: distribution-badge reachIcon() — complete schema-8 cases + first test

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.ts:145-150`
- Test: `distribution-badge.component.spec.ts` (add `reachIcon` coverage — currently zero)

**Steps:**

- [ ] **Step 1:** Replace the partial if-chain with a complete record over the schema-8 union (fallthrough returns the raw string only for future-proofing):

```ts
private static readonly REACH_ICONS: Record<string, string> = {
  private: '🔒 private',
  self: '🔒 self',
  intimate: '🔒 peer-only',
  trusted: '🤝 trusted',
  familiar: '🤝 familiar',
  community: '🏘️ community',
  public: '🌐 public',
  commons: '🌍 commons',
};

reachIcon(reach: DistributionSummary['reachClass']): string {
  return DistributionBadgeComponent.REACH_ICONS[reach] ?? reach;
}
```

- [ ] **Step 2:** Add a spec test asserting every one of the 8 canonical values returns a non-raw labeled string (i.e., result !== input), and an unknown string falls through raw.
- [ ] **Step 3:** Run the library test suite + lint; commit path-limited: `git commit -m "fix(distribution-badge): complete reachIcon schema-8 cases + coverage" -- app/elohim-library`

### Task 4: bootstrap-standing-policy canonicalization + reach_threshold normalization

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json` (reachThresholds, lines ~43-52)
- Modify: `elohim/elohim-storage/src/services/manifest_registry.rs:135-143` (`reach_threshold`) + in-module tests (~440-582)

**Ordering (hard precondition from slice 1):** canonicalize the JSON in the SAME commit as (and logically before) wiring any canonicalization into the lookup — never wire `parse_reach_key` against the legacy-keyed policy (district/public collision).

**Steps:**

- [ ] **Step 1:** Replace `reachThresholds` with the canonical 8-key set (values follow slice-1's fixture semantics — floor bands "any", community "neutral", public/commons "high"):

```json
"reachThresholds": {
  "private": "any",
  "self": "any",
  "intimate": "any",
  "trusted": "any",
  "familiar": "any",
  "community": "neutral",
  "public": "high",
  "commons": "high"
}
```

This removes the dead `"district": "neutral"` and the four missing-canonical-key fallthroughs (private/trusted/familiar/commons currently absent → silent code-default).

- [ ] **Step 2:** In `reach_threshold`, normalize the incoming key before lookup so legacy manifest payloads still resolve:

```rust
pub fn reach_threshold(&self, reach: &str) -> Option<String> {
    let payload = ...; // unchanged
    let thresholds = payload.get("reachThresholds")?.as_object()?;
    let canonical = crate::services::epr_kind::parse_reach_key(reach)
        .map(|r| r.as_manifest_key())
        .unwrap_or(reach);
    thresholds.get(canonical)?.as_str().map(|s| s.to_string())
}
```

(Adjust paths/trait imports to match `epr_kind.rs`'s actual exports — `ReachStandingExt::as_manifest_key`.)

- [ ] **Step 3:** Tests: extend the in-module tests to assert (a) every canonical schema-8 key resolves against the updated JSON, (b) legacy inputs `"household"`, `"district"` resolve via normalization to the `trusted`/`public` thresholds, (c) unknown strings return None.
- [ ] **Step 4:** Check whether any schema governs bootstrap-standing-policy.json (`pnpm run schema:test` / `schema:validate` from repo root) — run it if it exists; fix drift.
- [ ] **Step 5:** cargo test (storage env from Global Constraints), fmt, clippy. Commit path-limited: `git commit -m "fix(reach): canonicalize bootstrap-standing-policy reachThresholds to schema-8; normalize reach_threshold lookup" -- elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json elohim/elohim-storage`

### Task 5: content.reach backfill migration + retire the stored-"public" divergence + shefa fallback fix

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-07-23-140000_content_reach_canonicalize/up.sql` + `down.sql`
  (**timestamp trap:** verify with `ls elohim/elohim-storage/migrations/` that no other dir shares the `2026-07-23-140000` prefix — the repo already contains a live collision pair at `2026-04-19-000001_*`; diesel silently keeps only one)
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs:290-330` (divergence doc-comment + `parse_reach_class`)
- Modify: `elohim/elohim-storage/src/graph_views/shefa/distribution.rs:104-118` (`reach_str_to_class`)
- Test: in-module + `tests/distribution_view.rs`; new migration test

**Steps:**

- [ ] **Step 1:** `up.sql` — ordered UPDATEs on `content.reach` (order is load-bearing: old-top-rung remaps run FIRST so later remaps into `public` mean canonical Public):

```sql
-- One-time canonicalization of pre-reconciliation reach values (spec §7.5 data-aware migration).
-- Order matters: 'public' (old top rung) must move to 'commons' BEFORE any legacy value maps INTO 'public'.
UPDATE content SET reach = 'commons'  WHERE reach = 'public';
UPDATE content SET reach = 'public'   WHERE reach IN ('district', 'federated');
UPDATE content SET reach = 'self'     WHERE reach = 'personal';
UPDATE content SET reach = 'trusted'  WHERE reach IN ('household', 'local');
UPDATE content SET reach = 'familiar' WHERE reach = 'neighborhood';
UPDATE content SET reach = 'community' WHERE reach = 'collective';
UPDATE content SET reach = 'intimate' WHERE reach = 'invited';
```

`down.sql`: comment-only no-op (`-- irreversible data canonicalization; no down migration`).

- [ ] **Step 2:** Migration test: a Rust test that opens a scratch in-memory SQLite connection, creates a minimal `content(id, reach)` table, inserts one row per legacy/retired value plus one canonical `'public'`-era row, `batch_execute`s the up.sql (read from the migrations dir via `include_str!`), and asserts the resulting value set is exactly canonical with `public→commons` and `district/federated→public`.
- [ ] **Step 3:** Retire the divergence: read BOTH `parse_reach_class` (distribution_view.rs) and `epr_kind::parse_reach_key` stored-"public" handling and unify — post-migration, stored `'public'` unambiguously means canonical Public in BOTH parsers. Delete the lines-290-311 divergence-rationale comment; replace with one line stating stored data is canonical as of the 2026-07-23 migration. **KEEP the legacy match arms** in both parsers (adjudicated: they remain correct ingest-tolerance for stale seed JSON; removing them would re-create the silent `unwrap_or(Private)` degradation slice 2 fixed). Update `tests/distribution_view.rs` legacy round-trip expectations if any change.
- [ ] **Step 4:** `reach_str_to_class` (shefa/distribution.rs): add explicit arms for all 8 canonical + 5 legacy values (mirroring `parse_reach_class` — delegate to it if visibility allows, else duplicate with a sync comment), and change the catch-all from `ReachClass::Public` to `ReachClass::Private` (conservative). NOTE for reviewer: this flips the empty-string/no-Cozo-row default from most-open to most-closed in a placeholder composition path — intended.
- [ ] **Step 5:** cargo test + fmt + clippy (storage env). Commit path-limited: `git commit -m "feat(reach): one-time content.reach canonicalization migration; unify stored-public reading; conservative shefa fallback" -- elohim/elohim-storage`

### Task 6: steward disposition + locality source-of-record + strand ledger closeout

**Files:**
- Modify: `steward/node/src/storage/reach.rs` — module doc-comment ONLY (no rename, no deletion — steward/node/CLAUDE.md records "dormant definition site, do not canonize"; `sync/coordinator.rs` has one live construction `Reach::Local`)
- Modify: `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` — source-of-record declaration comment on the `LocalityLevel` block
- Modify: `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md` — slice-3 dispositions
- Modify (if a schema-8 case is warranted): `genesis/a2o/features/` per story-harvest

**Steps:**

- [ ] **Step 1:** steward `reach.rs` header comment: this 6-value enum + `replication_policy` matrix (FullSync/MetadataOnly/OnDemand/Skip over content-locality × peer-trust) is the **locality/placement axis** — the seed of the locality-driven placement engine the reach spec sequences behind the reconciliation (`genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md` §1/§7-out-of-scope). It is NOT the declared-reach vocabulary; do not migrate it to schema-8; align it to the SDK `LocalityLevel` vocabulary when the placement engine is built.
- [ ] **Step 2:** SDK `LocalityLevel` block comment: declare this file the source-of-record for the locality vocabulary (drift-prevention law, spec §1); list the known projections (app re-export, cache/types.ts mirror, steward's future alignment). Rebuild dist (`pnpm run build`) since the SDK file changed.
- [ ] **Step 3:** Strand ledger: record slice-3 dispositions; add the two newly discovered same-name vocabularies (library `client/types.ts` numeric enum; `elohim/sdk/src/types.ts` hand-written schema-8 `ReachLevel` — candidate for codegen alignment in a later pass) so they cannot hide; list what remains for slice 4 (verdict surface, fixture harness, doorway `can_serve_at_reach` re-keying, `reach:`-named LocalityLevel wire fields, composition-law a2o scenarios).
- [ ] **Step 4:** Commit path-limited: `git commit -m "docs(reach): steward locality-seed disposition, LocalityLevel source-of-record, slice-3 strand ledger" -- steward/node/src/storage/reach.rs elohim/sdk/storage-client-ts genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`
