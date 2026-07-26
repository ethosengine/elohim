---
id: "sprint-result-2026-07-26-resiliency-saga-overnight-cure"
---

# Resiliency-Saga Overnight Cure Sprint Result — 2026-07-26

**Branch:** `dev` (integrator authority granted; six batched pushes)
**Commits:** `45c613482` (cure wave 1+2) · `f60b7ae8b` (heal legibility + pins surface + ch05 timeout) · `7cfc41c8c` (provide-tick prefix fix) · `b9864aaf3` (mishpat cell routing) · `cb86de206` (GetCommitmentOutput typed HoloHash) · `0cb247196` (bounds.epr_scope)
**Builds:** edge #1233–#1239 (+ app #1642, orchestrator-dispatched) — all deployed to alpha
**Orchestration:** chief-agent (Fable) + 2 Opus rust-architect waves + 3 Sonnet legs + 2 read-only evidence agents
**Status:** the resilience card for `elohim-host-landing` moved from all-zeros to its first honest non-zero stats; the remaining reds are named, diagnosed, and each has a documented next actuation.

> **Objective (operator, overnight grant):** drive the resiliency-saga valueflow toward valid delivery — interesting stats and A/B doorway convergence on the `elohim-host-landing` EPR by morning.

---

## The card, before → after

| Gauge (alpha-A) | Yesterday | This morning |
|---|---|---|
| stewardingCollectives | 0 | **1** (household-dowell — first ever) |
| diversityScore | 0.0 | **0.14** |
| commitmentBackedReplication | not on the wire | **served** (`{dwelling:0, collective:0, commons:0, pledgedBytes:0}` — honest zeros) |
| feltStatus headline | "No household is holding these yet" | **"Held by only 1 household — invite another to help hold these"** |
| distribution | measured | measured |
| protection | at-risk | at-risk (honest: needs ≥2 households / ≥1 online steward peer) |

`SAGA resiliency` line: 3/10 → 2/10 (ch06 regression *caught*, correctly) with ch03/ch04 stable green and the red set now fully diagnosed.

## What landed (all deployed)

1. **Manifest re-stamp on blob rotation** (the stewarding=0 root cause): stored shard manifests stamped from a previous SSR bundle diverged from `content.blob_hash`; distribution wrote locations under recomputed hashes nobody read. The healing sweep now detects divergence, re-stamps from local bytes, prioritizes divergent rows within the per-boot cap, and prunes superseded orphan locations (only when unreferenced by any manifest). **Live-verified: alpha's stewarding count went 0→1 on the very next boot.**
2. **Snapshot wire truth (ch09/ch10)**: `ResilienceSnapshotView` gains optional `commitmentBackedReplication` (schema-first, absent≡not-selected, graph branch omits rather than fakes); ch10 re-aimed to the served `stewardingCollectives` vocabulary; ch09 re-aimed to `commonsCommitments` (content commitments pledge 0 bytes *by design* — pledged-bytes needs a capacity-tier producer that doesn't exist yet; documented residue).
3. **The ch05 deadlock chain — EIGHT stacked defects, each invisible until the one above it was cured** (the trust-contract's "stacked invisible defects" pattern, at record depth): (a) retry glue exhausted its 60s window in ~21s (`maxAttempts` default); (b) cucumber's 30s step timeout killed the 60s poll; (c) commitments route hid fresh rows behind a 300s cache; (d) acquisition retries re-probed the same peer every time (batch-position modulo); (e) `ShardResponse::Error` leaked in-flight budget until dispatch wedged silently at 0; (f) the provide tick matched `epr:`-prefixed pin head_refs against bare content ids — **the desired set was empty for every pin that ever existed**; (g) every mishpat zome call targeted the lamad cell (`ZomeNotFound: mishpat` on every live conductor, forever — the notarize path was stillborn); (h) `GetCommitmentOutput` declared String hash fields against msgpack byte arrays (the 2026-06-13 decode class) and the author's payload omitted `bounds.epr_scope`, which validator check 4b requires.

   **The chain's end state, live-verified 2026-07-26 ~11:55 UTC**: commitment `uhCEkd2ZZTOht…` — `replicates-commons`, provider matthew (`uhCAkYi1…`), **state: active** in the mishpat ledger (`/api/v1/commitments/facing/rea`) — the first co-steward agreement ever notarised, announced, and graduated on the fabric, produced by the full designed loop: explicit pin (consent) → provide tick → mishpat notarization → bounds-validated ProvideAnnounce → graduation. **One link remains un-lit**: the mishpat→rea mirror never minted the `rea_commitments` row (no mirror/signal log lines), so ch05's probe (which reads rea) still measures 0 — next session's first diagnostic: whether the app-signal subscription actually receives mishpat-cell signals, or the mirror keys on a variant graduation doesn't emit.
4. **The pins consent surface is now reachable**: `/api/v1/pins` declared in storage's `build_manifest()` (doorway auto-discovers; no doorway code — the card's "invite a household" CTA finally has a wired lever). **Pin #433 performed**: household-dowell committed to provide the commons landing EPR via the explicit consent API.
5. **Heal-plane legibility**: the content heal was logging ~1578 `HEALED`/12h on matthew for *no-op same-head refreshes* (own-conductor resolution cannot heal a two-root split). `StampOutcome::Refreshed` now splits spin from cure; four new `elohim_projection_heal_outcomes_total` labels (`refreshed`, `refused_declared`, `refused_stale`, `no_row`) make starved/erroring/refusing/spinning answerable from metrics. StampMode invariants untouched.

## A/B convergence — where it truly stands

The blocker is **not** deploy lag and **not** the heal plane. App #1642's canonical-head propagation (DECLARE_ONLY, full 24-attempt ladder) fired at elohim.host and **B's conductor refused it**: `declare_canonical_head: no content found for id` — B's content rows are db-projections with **no conductor entry** (the DHT-anchor gap class), and DHT gossip of A's entry isn't reaching B (the spine's standing `notary-authority` red). Until B's conductor holds an entry for the id — via gossip heal or a local anchor step — no canonical channel can move B's head. That is the day-scale arc for A/B convergence; everything reachable around it was cured tonight.

## Security finding (morning priority #1)

`.auth_required()` on manifest-declared routes is stored on `CompiledRoute` and **never enforced by the doorway forwarder** — `POST /db/content/{id}/canonical-head` (Declare-mode, may move any head) reached the zome unauthenticated on BOTH doorways. Not patched blind overnight: the App pipeline's own propagation leg rides that route and needs its X-API-Key path verified against enforcement first. Canonical backlog entry: `genesis/data/timeline/backlog/security-doorway-auth-required-unenforced.md`.

## Residue (ranked)

0. **mishpat→rea mirror** (one bounded diagnostic from ch05 green): the active commitment exists in mishpat_commitments; rea_commitments never received it. Check the signal subscription's cell coverage and the mirror's triggering variant.
1. **Security**: enforce `auth_required` at the doorway forwarder (backlog entry above).
2. **A/B convergence**: B-side conductor entry gap → the notary-authority spine red; the mechanism, evidence, and refusal error are documented in `project_local_stack_dht_anchor_gap` memory + this doc.
3. **ch02/07/08 measurement timing**: Dataplane Validation probes gauges ~2min post-restart, before the 5-min sweeps populate — chronically `pending`. Either delay the gauge probes or emit-at-boot.
4. **ch06 assertion semantics**: `divergentAnchor <= 0` watches an oscillating 2000-row windowed sample (160→2207 in 2min), and `/p2p/status` `healedTotal` describes only the REA arm. Re-aim to the new heal-outcome labels.
5. **Pin retirement policy** (operator decision): retry-exhausted pins live forever — occupy `MAX_ACTIVE_PIN_ROWS`, hold `pull.caughtUp` false (alpha's 36 `e2e-*` phantoms). TTL vs auditable `abandoned` state vs source-side exclusion.
6. **Capacity-tier pledge producer**: nothing in the codebase can pledge bytes; `totalPledgedBytes` stays 0 until it exists (saga README residue).
7. **Environment**: a `/tmp` wipe left 5 dangling cargo target symlinks that cost three gate runs; the pool preflight should verify symlink targets. `grandma-album-1974` reanchor loops on invalid content_type `album` (vocabulary drift). `transportStats` deser (`missing field is_direct`) blanks a diagnostics field.

## Meta-verdict: is the valueflow helpful? (operator's standing question)

**Yes, with named limits.** What it earned tonight: the frontier line + `epr flow fulfill` turned "where are we?" from an hour of archaeology into one command; born-red chapters worked exactly as designed — every card zero decomposed onto a chapter, and the regression (ch06) was *caught and recorded as a Dismiss event* rather than silently overwritten. What it cost/missed: `flow walk` can't traverse the saga (walk-by-recipe is backlogged — the story is readable only through the frontier reader); fulfillments are binary, so the quantitative story (stewarding count, pledged bytes, protection deltas over time) isn't yet in the ledger — the natural evolution is fulfillment events carrying measured quantities at flip time; and the loop only re-measures when CI runs, so the frontier can lag live truth by a deploy cycle (tonight's actual frontier was ch05's producer, not the pending-env ch02 the line named). Value axes that matter (vs story points): risk retired (protection deltas), pledged-vs-witnessed bytes (REA reciprocity), and energy spent per delta — all already denominated in the schema.

## Verification evidence

- Gates per push: storage lib 2162→2163 passed, schema_contract 221, household_resilience 37, views export_bindings 380, fmt/clippy clean; a2o tsc/eslint/cucumber dry-run clean; final pre-push `ALL CLEAR`.
- Live probes (2026-07-26 morning): alpha snapshot stewarding=1/diversity=0.14/cbr-served; pins route 200 via doorway; pin #433 active with `pull caughtUp:true`.
- Saga measurement: edge #1233/#1234 dataplane reports consumed via `jenkins-sync`; ch10's step now correctly asserts the real divergence (`alpha-A=1 vs elohim.host=0`).
