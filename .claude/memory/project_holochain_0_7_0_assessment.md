---
name: project_holochain_0_7_0_assessment
title: Holochain 0.7.0 assessment
description: "Holochain 0.7.0 shipped 2026-07-30; assessed 2026-09-02 vs our 0.6.3 fork — tx5 gone, Action preimage changes, no data migration, kitsune2 0.5 strict relay match; steers Wave 3"
metadata: 
  node_type: memory
  title: Holochain 0.7.0 assessment vs our 0.6.3 fork
  type: project
  originSessionId: 0b015666-05d9-4a1d-bf90-6e1e121f70b3
  modified: 2026-09-02T21:19:13.485Z
---

Holochain 0.7.0 released 2026-07-30 (tag `holochain-0.7.0` = 84cdce7d4, already fetched in the
`elohim/holochain-conductor` submodule; no 0.7.x patch tags as of 2026-09-02). Assessed 2026-09-02.

**Upstream facts that matter to us:** hdk 0.7.0 / hdi 0.8.0 / kitsune2 0.5.0 / lair 0.7.1 / wasmer 7.1 /
`@holochain/client` 0.21 / `holochain_client` 0.9.0. tx5 + SBD signal + `webrtc_config` + `signal_url`
are REMOVED (iroh is the only transport). `Action` became `{header, data}` (10-variant closed
`ActionData`, `#[serde(tag="type")]`) — the hash/signature preimage changes. DB + wire incompatible
with 0.6; DNA hashes change for identical DNAs; NO data migration path. New: `wasm_backend`
(`cranelift|LLVM|wasmi` — wasmi = iOS App-Store path), `db_sync_level` replaces `db_sync_strategy`,
`request_timeout_s` moves under `network`, `chc_url` gone, `restore_chain_quorum` exists but is
unused in the 0.7.0 tree, `MigrationTarget` on Close/OpenChain + `InitProperties` (upgrade rails).
`update_coordinators` survives (rung-1 hot-swap vehicle intact). Upstream publishes an AI upgrade
skill: github.com/holochain/ai-tools/skills/upgrade-holochain-0.7.

**None of our 9 fork commits are upstream at 0.7.0** (jemalloc pair, store_slice_hash change-check,
sys-validation backoff, sqlite saturation log, tx5 pin, iroh cross-relay preflight fallback).
**kitsune2 0.5.0 PR #479 codified the OPPOSITE of our cross-relay patch** ("exact relay URL match,
no fallback to global relay", one relay per space) — our per-doorway relay topology (design D2)
partitions again on 0.7 unless the patch is re-carried or the topology collapses to one relay.

**Our exposure (counted 2026-09-02):** zomes small (55 rs files; 24 `Action::` sites in 4 files,
16 `FlatOp::Store|Register` in 6, 0 `EntryCreationAction`/`get_agent_activity`/`ChainFilter`);
JS client trivial (only websockets + hash helpers imported); shared `holo_hash =0.6.0` in 6 sdk
domain crates + sweettest + doorway-client is the interlock that forces one atomic family move;
doorway already on 0.7.0-dev.23 pre-restructure pins; steward `tauri-plugin-holochain` has NO
main-0.7 branch yet (blocker for desktop). Fleet conductor is ALREADY iroh (alpha, since Wave-2
Stage 1); tx5 remains only as the rollback line, so 0.7 retires ~1150 lines of doorway signal
server, 2 coturn manifests, the `elohim/tx5` submodule and the `<hc12>-<tx512>` tag scheme.

**Why:** the 2026-08-04 convergence campaign staged 0.7 as Wave 3 "re-genesis"; the release is now
real, and the 2026-09-02 alpha re-key already spent the "chains are disposable" cost once.
**How to apply:** treat 0.7 as a planned re-genesis event, not a dep bump — one atomic family move
(zomes + sdk types + sweettest + doorway + storage client pins + conductor rebase + config templates)
with `[dna:migrate]` + baseline, both bootstrap peers together. Decide the relay topology BEFORE
rebasing. See [[project_upgrade_authority_constitutional_elohim]],
[[feedback_upgrade_propagation_north_star_wall_clock]], [[project_alpha_dna_migration_2026_09_02]].

**STATUS 2026-09-03 16:2xZ — F2 DONE, push staged.** Lanes A–E, I folded on `upgrade/holochain-0.7` (rebased
onto dev 502ff2765, 26 commits, tip carries `[dna:migrate]`). Household mesh on the STOCK 0.7.0 conductor +
local iroh-relay 1.0.3 (kitsune2 0.5 makes the relay load-bearing — 0 connections without it): relay
connectivity ✓, cross-conductor DHT heal ✓, Act I 203/31/7/37 of 278 (every red traced to apparatus or
standing storage bugs — findings atom `2026-09-03-hc07-f2-mesh-findings.md`), **rung-5 stations 1–5 5/5**
(receipt 20260903T154932Z-fcb81456; `sync_coordinators` hot-swap applied on 0.7). Fleet relay must roll to
1.0.3 in the same wave (E10). Push = one `HUSKY=0 git push origin upgrade/holochain-0.7:dev` after the
tevah/ark session's Task-6 storage commit lands (lane gates green; 18 pre-push gates would take hours and
their compile writes are today's pod-swap shape; CI is the backstop). Then F3/F4 dispatch chain
conductor → dna → edge, baseline from CI `DNA-HASH` lines, F5 = the operator runbook
`2026-09-03-holochain-0-7-fleet-cutover-runbook.md` (full fleet wipe authorized).
**PUSHED 2026-09-03 16:5xZ: origin/dev 3c29b39d7 → a78904105** (`HUSKY=0`, storage `cargo check` on the 0.7 family
green after the rebase). Watch: orchestrator → elohim-conductor (`conductor-25dd2d0be144`) → DNA (`DNA-HASH` lines →
baseline) → edge; then F5 via the runbook.
**F3/F4 DONE 2026-09-03 ~21:00Z:** conductor image `conductor-25dd2d0be144` (edgenode #29), DNA #1427 green (55/55,
hApp `1.0.0-dev-6ce1fff6` = dev-latest), edge #1426 rolled alpha with the 0.7 storage image + relay 1.0.3. Seven CI
rounds were spent on dispatch defects, none on the substrate (fabricated gitlink SHA, che main not fast-forwarded,
Jenkins param-default lag, plain-helper rendezvous, shard evictions, missing perl, floating hApp + fire-and-forget
ordering) — all in `project_pipeline_dispatch_ordering`. NEXT = F5: operator runbook
`2026-09-03-holochain-0-7-fleet-cutover-runbook.md` (wipe after roll, bootstrap pair together, DNA_MIGRATION_INTENT
from the baseline, re-seed once); then F6 evidence (dataplane-convergence atom delta, cycle-time row).
**Concession → DESIGN (2026-09-03):** the 0.7 cutover was a WIPE + re-genesis, not upgrade propagation — rung 5 covers
coordinator-only releases on a fixed line. The missing class is now DESIGNED as the Holochain Evolution Epic: spec
`genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md` (the path = the elohim's mishpat `migrates-lineage`
commitment, never a consent act; v2 installed beside v1 under the same key; NotarizationWitness carries v1 action+signature, re-verified by v2
— entry hashes survive, only action hashes move; dual-cell bridge; free revert until a notarized sunset closes v1 chains;
Lane B = conductor-fork spike toward one-DHT chain continuation). Story `happ-lineage-migration.feature` Stations 1–9,
habit `elohim/holochain/.epr-meta/happ-lineage-migration.habit.md` born red, rehearsal = node_registry on the mesh.
Never count a wipe as rung-5 or Holochain-Evolution-Epic evidence. See [[feedback_delegate_research_to_opus_sonnet_codex]].
**LANDED 2026-09-04 00:3xZ:** alpha fleet on holochain 0.7.0 fresh genesis (edge #1428 after the operator wipe); both
doorways caughtUp+converged, 35 agents / 5 spaces across BOTH relays (D2 holds live). Re-seed via `[build:genesis]`.
