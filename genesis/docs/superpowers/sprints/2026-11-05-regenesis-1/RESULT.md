---
id: regenesis-1-result
title: Re-genesis 1 — the address crossing — sprint result
date: 2026-11-07
status: landed; alpha re-genesis pending (operator act)
author: orchestrator session 184beb01 (Claude Fable 5.1) with shem session 617b9272 (Claude Opus 5.5), Opus/Sonnet implementers, Codex gpt-6-astra read legs
habits: [dataplane-convergence, push-gate-pays-once, pin-attestation, addresses-are-declared-profiles]
---

# Re-genesis 1 — result

**What this sprint did:** moved every byte address in the protocol from a bare `sha256-<hex>` string
to a self-describing CIDv1 (raw + BLAKE3, `bafkr4i…`, FastCDC chunking declared in the profile), as a
clean break with no migration, across the kernel, eprfs, brit, rakia/sdk, DNA integrity zomes,
elohim-storage, doorway, app/SPA, seeder/CI, a2o and steward/node; recast both households; rolled
alpha's storage image and conductors; published `elohim-epr 0.2.0` and `elohim-address 0.1.0`;
moved the brit pin to a build whose notes carry a signature envelope. Journal: `STATE.md`.

## Landed on dev (in order)

| Closure | dev range | What |
|---|---|---|
| L1 kernel | `9c103746e..` (2026-11-05) | `crates/elohim-address` + `elohim-epr` profile/signature envelope |
| L2a/L2b | 2026-11-05/06 | eprfs, schemas/views/generated, storage, doorway, steward/node |
| L2c + L3 | `e85eeeee5..6a31ed270` | a2o/app/seeder/CI crossing, shem's five-DNA series `[dna:migrate]` |
| CI cure | `97a956439..090dcdee1` | rust-style baseline moves to the method bootstrap (`[skip ci]`) |
| shard index | `090dcdee1..2fb2f93c5` | node-registry `MAX_SHARD_INDEX` 6 → 65535 (custody-unit index) |
| Landing 1 | `2fb2f93c5..89d4a22ff` | R1 custody over iroh + relay ruling, epr publish fix, storage image fix, DNA baseline from #1527 |
| Landing 2 | `89d4a22ff..78e0ba08c` | Codex cycle-time harvest, gospel correction, epoch-2 brit note readers (ark, gate-evidence, a2o verifier with a brit-minted vector), brit pin → `10d18eacc` (brit-cli 0.1.13), shem evidence + two hook fixes |
| Landing 3 | `78e0ba08c..6d994a931` | uploader self-copy fix (shem a24504bb), step-983 message, drift-signal hook fix |

CI on the crossing: orchestrator #2233 (cure) and #2236 green; #2235 dispatched the alpha roll —
epr #65 published 0.2.0, DNA #1528 all five roles MATCH the baseline, edge #1821 deployed, edge
#1822 (validation) and genesis #1741 red/unstable as the expected post-crossing amber (below).

## Measured (two households, same head)

| Lane | ethosengine (e77465c37, then 6d994a931) | shem (89d4a22ff, then a24504bb) |
|---|---|---|
| resilience/chaos-peer-churn | 3/3 (uploader unselected on the self-copy run) | 2/3 → 3/3 after the self-copy fix |
| @concern:blob-durability | 8 passed / 0 failed / 3 pending / 3 skipped | 8 / 0 / 3 / 3 |
| dataplane/epr-app-deliverability | 7/7 (135 steps) | 7/7 |
| custody announce refused_* | 0 on every peer | 0 on every peer |
| custody sent/recv/applied | matthew 3/4/3 · james 6/7/5 · jessica 1/0/0 (iroh-gossip topology) | matthew 13/30/15 · jessica 13/30/15 · james 2/52/26 |

Before R1 (dev 6a31ed270, both devices): chaos refused at its baseline step (remote custody did not reach
`shard_locations` over iroh); blob-durability 7/1/3/3.

## Defects found and fixed in flight

1. Custody announcements did not reach iroh (the producer published to libp2p only) and iroh-sourced ones were refused (R1: signed
   `SignedCustodyAnnouncement` through the fan-out; holders claim their own pushes; a relay source
   refuses only when bound to a different agent).
2. node-registry refused striped custody units above shard index 6 (ceiling 65535, index = custody unit).
3. The uploader recorded its own verified copy only when placement happened to select it (self-copy
   ruling: a node that holds and serves verified bytes is a holder).
4. Seeder signed EPR atoms with the retired proof shape; a2o could not load `multiformats` under
   CommonJS; a lint regression in the byte-address helper; a stale loader spec.
5. CI: rust-style lockout (baseline pinned the old method), epr publish refused by an untracked
   `flake.lock`, the storage image lacked the `elohim` registry name, a self-selection test matched the
   pre-R1 producer shape, a handoff doc without an `id:` slug, four eslint errors in a reviewer's files.
6. Brit: `cargo-machete` false positive on `serde_bytes` (ignore list), `brit push` closed-channel and
   end-of-stream failures, worktree `brit commit` refusal, gitlink `brit add` panic (all recorded in memory).

## Follow-ups minted (not done here)

- Custody: re-announce change detection across distributions; verified admission on the SELECTED path;
  multi-chunk custody-unit coverage (`list_cids` sees `blobs/` only); cross-signed agent↔transport
  bindings before custody rows read as verified possession; reject conflicting alias bindings;
  `announced_at` freshness; inventory envelope still signs with the libp2p key; dormant catch-up arm.
- Seeder: retry a 400 "REA projection changed during authority read; deferred".
- Hook: the rust-style reach gate refuses `refs/notes/brit/*`, so validation receipts stay local.
- Cycle time: `briefs/cycle-time-harvest-handoff.md` (three of four items landed by the Codex session;
  receipt narrowing deferred to the import-closure form).
- brit notes: a real brit 0.1.13-minted `@2` note is the a2o verifier's fixture; ark's witness string
  encoding awaits a typed `Witness` in eprfs-core.
- Memory index population (258 unattributed contributions) before the projector can compact it.

## Still owed — the operator's alpha re-genesis (plan §5 A2–A6)

Alpha runs the new storage image and conductors but its cells and landing content predate the crossing:
both doorways report the landing has no `blobCid`. Arm `DNA_MIGRATION_INTENT` with the five BUNDLE
hashes (read from the alpha storage pod's boot ERROR line — happ_manager names each drifted role with its
bundle hash), both genesis peers in the same roll, `RESET_STORAGE=true` on the genesis run, PVC clear after
the roll; then the receipts from here: one landing CID on both doorways, conductor diagnostics on the new
hashes, `X-Content-Address: bafkr4i…`, a `[build:edge] [edge:validate-only]` measure, `pnpm look`.
Next-session device check: the brit session pin moves to 10d18eacc at the next session start.
