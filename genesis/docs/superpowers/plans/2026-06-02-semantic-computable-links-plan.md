---
title: "Semantic-Computable Links — Bootstrap Implementation Plan"
id: semantic-computable-links-plan
status: Draft
created: 2026-06-02
class: process-meta
process_subdomain: memory
sprint: bootstrap
cites:
  - semantic-computable-links-design | the design spec this plan implements — slug+desc+fingerprint envelope cites the tools generate, resolve, and verify | sha256:405c25775e06a985
  - .claude/scripts/memory-kit/memory-coherence-audit.py
  - .claude/scripts/_lib/frontmatter.py
  - .claude/scripts/memory-kit/decompose.py
  - .claude/workflows/memory-stasis-loop.js
derived_from:
  - .claude/skills/epr-content-addressing/SKILL.md   # the dogfood: slug identity + CID fingerprint + envelope (lineage breadcrumb)
requires_env: []   # pure dev-tooling (python/js/markdown) — testable on household-nodes
---

# Semantic-Computable Links — Bootstrap Implementation Plan

Implements `2026-06-02-semantic-computable-links-design.md`. **Foundational decisions resolved** (the spec's open questions): **slug** = readable, derived from title, collision-guarded against the slug-index; **fingerprint** = `sha256(content-body)` (frontmatter EXCLUDED, so a metadata edit doesn't trip `STALE-CANDIDATE`); **back-compat** = legacy path-string cites keep resolving via `p.exists()` until the migration completes. Verification is process-meta-shaped (test passes / script exits 0). Order is load-bearing: the data model + slug-index (P1) before the tool (P2) and the audit (P3); the migration (P7) only after the audit is back-compat; the dissolution gate (P5) hardens only after migration.

This plan is the second link in the chain `subject-routed-decomposition → semantic-computable-links → scope-tree-reconciler` — and `scope-tree-reconciler`'s held-tree moves are unsafe until P1–P3 here make cites resolve by slug, not path.

## Phase 1 — Data model: doc `id:` slug + cites envelope + slug-index

- [ ] `.claude/scripts/_lib/cite_graph.py` (shared `_lib`, ≥2 callers): `slugify(title)`; `allocate_slug(title, index)` (collision-guarded); `fingerprint(path)` = `sha256` of the content-body (frontmatter stripped via `_lib.frontmatter`), short-prefixed; `build_slug_index(roots)` → `{slug: path}` across live (and later `held/`) trees; `parse_cites(fm)` → list of `{ref|path, desc?, fingerprint?, status?}` accepting BOTH the envelope form and a legacy path-string (back-compat); `serialize_cites(entries)`. Covers spec gap-items 1, 2, 3.
- [ ] `.claude/scripts/_lib/__tests__/cite_graph_test.py`: slugify is stable + collision-guarded; fingerprint is invariant under a frontmatter-only edit but changes on a body edit; slug-index round-trips; `parse_cites` accepts envelope AND legacy path-string; serialize↔parse idempotent.
- [ ] **Check:** `python3 .claude/scripts/_lib/__tests__/cite_graph_test.py` exits 0.

## Phase 2 — `cite-gen` (the friction-killer tool)

- [ ] `.claude/scripts/memory-kit/cite-gen.py` (imports `_lib.cite_graph`): given a target (slug or path) → emit the envelope block (`ref` from the target's `id:`, `desc` auto from its title, `fingerprint` computed); `--into <doc>` rewrites that doc's `cites:` in place (path-strings → envelopes); `--verify <doc>` exits non-zero if any cite is non-envelope or unresolvable (the gate mode for P5). Covers gap-item 4.
- [ ] Tests: emit produces a parseable envelope; `--into` is idempotent (re-run = no-op); `--verify` passes on an envelope doc, fails on a legacy-only doc.
- [ ] **Check:** tests exit 0; `cite-gen.py <a-real-spec-slug>` prints a valid envelope.

## Phase 3 — The audit upgrade (resolve-by-slug + verdicts)

- [ ] Patch `.claude/scripts/memory-kit/memory-coherence-audit.py` `cite_resolves()`: build the slug-index once; resolve a cite by `ref` across live + `held/`; emit `HELD-CITE` (in held/ — informational, NOT dead), `DEAD-CITE` (resolves nowhere), `STALE-CANDIDATE` (target fingerprint ≠ cited), `CITE-FORMAT-CANDIDATE` (legacy path-string — migrate). Legacy path-strings still resolve via `p.exists()` (back-compat). Covers gap-item 5.
- [ ] **Check:** a fixture doc citing a held-tree slug → `HELD-CITE` not `DEAD-CITE`; a fingerprint-drifted cite → `STALE-CANDIDATE`; a path-string cite → `CITE-FORMAT-CANDIDATE`; a real envelope cite → OK. Run the audit over the repo → no false `DEAD-CITE` storm.

## Phase 4 — The propagation surface (the epr-head edge)

- [ ] `.claude/scripts/_lib/cite_graph.py` += `reverse_index(slug_index)` (target → every citer) + `propagate_status(target, verdict)` — materialize/remove the optional inline `status:` field on each citing block (`status: held — needs remote-compute` on degradation; removed on recovery). Covers gap-items 6, 7.
- [ ] `.claude/scripts/memory-kit/cite-propagate.py` (or an audit `--apply` mode): for each target whose scope/fingerprint changed, stamp/clear `status:` on its citers; emit a reviewable summary (the commit is the link-health ledger). Triggered by a reconciler move, audit, or the stasis loop.
- [ ] **Check:** moving a fixture target to `held/` then running propagate → its citers gain `status: held`; restoring it → `status:` removed. Healthy cites never carry the field.

## Phase 5 — The dissolution gate (hard at the BACK-fire, after migration)

- [ ] `decompose.py` (the BACK-fire) calls `cite-gen --verify` on a spec being dissolved/graduated: block graduation until every `cites:` is envelope-form and resolves. **Soft-warn until P7 migration completes** (legacy cites tolerated mid-migration), then hard. Covers gap-item 8.
- [ ] **Check:** a fully-migrated spec passes the gate; a legacy-cite spec warns (pre-migration) / blocks (post-migration).

## Phase 6 — The authoring template (born-correct at the FRONT)

- [ ] `.claude/skills/semantic-links/SKILL.md` — the canonical envelope format + the doc `id:` convention + how to author (run `cite-gen`, never hand-write slugs/fingerprints). Wire a one-line pointer into the `/brainstorm` + compaction POST steps (which already write `cites:`), so new cites are born content-addressed. Covers gap-item 9.
- [ ] **Check:** the skill is discoverable; the POST-step pointer references it.

## Phase 7 — The corpus migration (one-time, fan-out)

- [ ] `.claude/workflows/cites-migration.js` (Workflow), two passes: (1) allocate `id: <slug>` to every doc lacking one (deterministic from title, collision-guarded); (2) fan out `cite-gen --into <doc>` over the corpus, path-strings → envelopes. Idempotent. Covers gap-item 10.
- [ ] **Check:** dry-run on a sample dir converts cites to envelopes losslessly; re-run = no-op; the audit's `CITE-FORMAT-CANDIDATE` count drops toward 0.

## Phase 8 — Maintenance wiring (stasis loop)

- [ ] `memory-stasis-loop.js` += `cites_legacy` scoreboard dimension (from the audit's `CITE-FORMAT-CANDIDATE` count) + a `cites` discipline (librarian goal: run `cite-gen --into` on a batch; lower `cites_legacy`→0). Add a `cites:` line to `placement-audit.py --headline`. Covers gap-item 11.
- [ ] **Check:** the loop's MEASURE step reports `cites_legacy`; the discipline is dispatchable.

## Phase 9 — Memory-team agent awareness

- [ ] Update `librarian` / `historian` / `cartographer` / `storyteller` agent defs: the envelope convention + `cite-gen` (never hand-write); the verdicts (`HELD-CITE` ≠ `DEAD-CITE`; `STALE-CANDIDATE`; `CITE-FORMAT-CANDIDATE`); `held/` moves are safe (don't "fix" a `HELD-CITE` by deleting the link); the dissolution gate. Per-agent emphasis per spec §8. Covers gap-item 12. **(Coordinate: `.claude/agents/librarian.md` had operator in-flight edits this session — rebase onto current.)**
- [ ] **Check:** each agent def references the envelope + the verdicts.

## Phase 10 — Close the loop

- [ ] End-to-end: author a new doc → `cite-gen` makes a born-correct envelope cite → move the target to `held/` → propagation stamps `status: held` on the citer → audit reports `HELD-CITE` (no false DEAD) → restore → `status:` clears. The citation graph is content-addressed, self-describing, and survives moves.
- [ ] **Check:** the full cycle runs with no false dead-link; `scope-tree-reconciler`'s held-tree moves (the next chain link) are now safe to build on top.

## Sequencing & rollback

- **Order:** P1 → P2 → P3 → P4 → P5(soft) → P6 → P7(migration) → P5(harden) → P8 → P9 → P10.
- **Back-compat is the safety net:** P3's legacy `p.exists()` path keeps the corpus working through the migration; the dissolution gate stays soft until P7 completes.
- **Blast radius:** dev-tooling + doc frontmatter only; no product code, CI, or cluster. Each phase is a new file or a localized patch, independently revertable. The propagation `status:` writes are reviewable commits (the link-health ledger).
