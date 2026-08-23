---
id: "backlog-swarm-parity-aware-completion"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shard swarm: an erasure-coded fetch is COMPLETE once data_shards have landed — stop racing the rest, report reconstructible-vs-incomplete honestly"
slug: "swarm-parity-aware-completion"
written: "2026-08-23"
author: "fable-5 fork 2026-08-23 (operator-requested Codex queue — sharded blob distribution)"
status: "wip"
priority: "high"
area: "dataplane/blob-swarm"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:blob-durability"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
tags: [dataplane, blob-swarm, erasure-coding, bounded-code-fix, codex-claimable, agent-agnostic]
---

# Parity-aware swarm completion

**Why.** `elohim/elohim-storage/src/p2p/blob_swarm.rs` (`fetch_shards_via_swarm`, ~:133-300) races
every shard named by the manifest and reports `ManifestPersistedIncomplete { missing_shards }` if ANY
failed. For an `rs-4-7` manifest that is wrong twice: a blob with 4 of 7 shards landed is already
servable (since 2026-08-23 `reassemble_from_local_shards` reconstructs through parity with
≥ `data_shards`), and racing the remaining 3 parity shards after 4 have landed spends holder
bandwidth the requester does not need — the opposite of the torrent curve the module exists for.

## Scope (elohim-storage only — `p2p/blob_swarm.rs` + one test file)
1. Read `manifest.encoding` / `data_shards` (`sharding.rs` `ShardManifest`); for erasure-coded
   manifests, treat the fetch as COMPLETE when `landed >= data_shards` and cancel/skip the
   still-pending shard races (the `futures::stream` fan-out already bounds in-flight; add a shared
   `AtomicUsize` landed-counter checked before each race starts, and do not start new races past
   the threshold). `"chunked"`/`"none"` keep all-or-nothing.
2. Split the outcome: `SwarmRaceOutcome::Reconstructible { landed, missing }` (servable now; the
   background salvage/heal fills the rest) vs `ManifestPersistedIncomplete` (below `data_shards`).
   Callers in `http.rs` (`race_fetch_with_swarm` consumer ~:3664-3760) treat `Reconstructible`
   like success for serving; the named-404 branch keeps `Incomplete`.
3. Metric: `inc_blob_swarm_shard_fetched("parity_skipped")` (match the existing label style at
   `blob_swarm.rs:243`).

## DoD / verification
- Unit tests beside `plan_shard_holders_rotates_composite_fallback_across_shards`: (a) rs-4-7
  manifest, 4 shards land first → outcome `Reconstructible`, ≤ 4 races started after threshold;
  (b) 3 land → `Incomplete`; (c) chunked manifest with one miss → `Incomplete` (unchanged).
- `just gate elohim-storage` → `GATE_EXIT=0` echoed on its own line. Plain cargo (no nextest);
  `CARGO_TARGET_DIR` = the pool slot.
- Commit path-limited: `git commit -m "…" -- <paths>`; never `--amend` (shared worktree).

## Disjointness
Do not touch `http.rs` `put_blob_bytes`/`reassemble_from_local_shards` (landed today), `sharding.rs`,
`p2p/blob_fetch.rs`, or anything under `p2p_iroh/` (Lane T2, Opus).

## Live evidence (2026-08-23, orchestrator on the owned 3-peer mesh)

Codex landed the implementation as `4009362f0` (gate green, 2,968 lib tests) without a
live mesh. Re-verified here: storage rebuilt at `4009362f0` with `p2p-iroh`, all three
peers re-exec'd in `dual`; `features/resilience/app-blob-heal-on-read.feature` 2/2
(first-request heal races peers for locally-missing bytes and serves; the >64 MiB RS
artifact ingests and serves whole) and `features/dataplane/doorway-failover.feature`
10/10 as the regression guard. Not yet measured: the curve itself (roadmap S3 — fetch
one RS blob with 1/2/3 holders and assert wall-clock falls) and a parity-shard-missing
heal through the doorway; those are the evidence that flips blob-durability, so this
row moves to `wip`-verified, not `done`.
