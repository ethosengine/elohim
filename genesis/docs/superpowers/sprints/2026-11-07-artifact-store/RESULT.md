---
id: artifact-store-sprint-2026-11-07-result
status: partial
habits: [sprint-lands-each-closure-once, blob-durability, push-gate-pays-once]
---

# Sprint 2 result: the artifact store on the p2p dataplane

Written 2026-11-09 during the overnight shakeout, by the orchestrator (Claude Opus 5.5, session
a8efbf5a on shem). The plan is `plan.md`; the minute-by-minute record is `STATE.md`; raw logs are
in `evidence/`.

## Outcome in one paragraph

Every code track except E4 landed on dev, and the hardening closure was accepted on the shem
household and rolled through alpha green. The sprint's headline receipt, a sibling workspace
fetching the 454 MB storage binary instead of rebuilding it (F2r), was **not run**. It needs
ethosengine's tier daemon at the same time as shem's. So the sibling-fetch code (F2) is landed
and gated, but has no live two-device receipt, and the `sprint-lands-each-closure-once` habit has
no DELTA. E4, one authoritative payload store per device, was not started.

## What landed on dev

| Closure | dev commit | Content | Verified by |
|---|---|---|---|
| L0 | hub | sprint hub, plan, design | docs |
| L1 + L2 | 3b266e41d | B1 notes skip in the pre-push ref loop; B2 seeder retry on `400 deferred`; B4/B5 cites and backlog; E3 `serve-blob` mount with per-root and per-object scope; S stories (`@wip`, two blind-reader rounds) | `evidence/push-L12.log`, astra E3 re-review |
| gospel | 3134f517f | brit build receipts travel (`refs/notes/brit/build/*`, 29 refs pushed, V5) | `evidence/push-notes-build.log` |
| L4 | 79acec4eb | F2 sibling fetch (resolve record by key, fetch over the tier, verify per chunk and whole, install) | delta review `evidence/f2-delta-review.md`, `evidence/push-L4.log` |
| L5 | 0bef0b84f | gate preflight `checkCargoLocks` (an unlocked Cargo.lock refuses in seconds) | replay against the incident tree, `evidence/push-L5-lockcheck.log` |
| survey | 364da4e09..4a1f41a69 | Radicle/Heartwood survey v2 | docs |
| L3 | 1b76c2327 | H hardening passes 1–5 (cross-signed, DHT-confirmed holder bindings; bounded confirmer), E2 tolerant reader, P policy epoch, R1.4 receipts | combined gate `evidence/gate-L3-storage.log`; household chaos-peer-churn 3/3, blob-durability lane 8/0/3/3, epr-app-deliverability 7/7; edge #1840/#1841 SUCCESS; fleet quiesce sustained |
| shakeout | 683b10ce3, 88f95922c | backlog rows from L3; the alpha orphaned-binding incident and its proof | genesis #1757 |
| shakeout | 0452dc85e | gate key: a habit DELTA no longer re-keys its crate's gate; blob-durability and push-gate-pays-once DELTAs | `gate-evidence.test.mjs` 72/72; real-commit receipt on 2c413084f; pre-push ALL CLEAR 1258 s |
| shakeout | (pending) | seeder reports the identifier the doorway stores | `seed-humans-binding.test.ts` 7/7, seeder 753 passed |

The pending row lands during this shakeout; STATE.md records its commit.

## Owed (what the next sprint picks up)

1. **F2r, the 454 MB receipt** (runbook: `f2r-runbook.md`). On ethosengine run `just mesh build storage` (a Produce). On shem
   the same build should resolve the record and fetch the binary instead of compiling it. Then run
   the reverse direction for doorway, then a second successive build to measure reuse. Compare
   both devices' `tool:` lines first (the key carries the resolved cc/ld paths and versions). The
   figures go into the `sprint-lands-each-closure-once` DELTA. F2's key is still the interim
   `basis-key:git-tree`; a receipt on it does not close Track A's projected-view frontier.
2. **F1r.** A live two-device sibling gate-reuse receipt (`independence: sibling` row plus its Use).
3. **E4, one store per device.** This was gated on H's storage closure, which has now landed.
   Interface anchors are in `design.md` §2.4 (`PeerConfig.payload_store`, `BlobStoreTier`). It
   needs the never-held-twice test.
4. **B3, mesh staleness keyed by closure.** It was never briefed. hc-mesh.sh still judges binary
   freshness by `src/` mtime, so a path-dependency, Cargo.lock or feature change passes as fresh.
5. **E2's review fix never landed.** Branch `artifact-store/e2-null-ext` (d77171ffd) re-applies
   d4cf5763b: an explicit null `ext` is refused, not normalized into a different CID. It needs an
   `eprfs-storage` 0.3.7 bump and both lockfiles, which dispatches edge, so it lands with the
   storage closure below. Details are in `task-as-e2-report.md`.
6. **Debt:** rename `blob_hash` to `blob_cid` in `elohim-storage/src/reconcile/custody.rs`. The
   address-profile-lint ratchet baseline was raised by three for L3. This is a storage source
   change, so it dispatches edge.
7. **Frontiers kept from the plan (§9):** bao range reads; the FsStore ruling (R1.4 receipts are
   its input); libp2p/dual tier modes (typed gaps); F5 release-plane custody gate; source
   materialization by projected view.

## Operator items

- Close dialogue-stasis incidents :270, :288, :303 and :314. Each parked an effect after it had
  already completed; nothing was retried.
- Rule on publishing covenant notes (`refs/notes/brit/covenant/*`, still local).
- The main checkout is at 6d994a931, far behind dev, and every worktree push runs its stale
  `.husky/_` hook. The pushes so far succeeded with it, but it predates B1.
- Alpha doorway: build the live "orphaned bindings = 0" invariant, fix the hosted reclaim's 18
  HTTP 500s, and write the re-genesis rebind procedure. Tracked in
  `genesis/data/timeline/backlog/doorway-orphaned-binding-after-regenesis.md`.

## Lessons

- **A check only sees what is checked out where it runs.** The eprfs dead-cite red came from a
  submodule CI never initialized. The first orphaned-binding repair went to the identifier the
  seeder sent rather than the one the doorway stored. Two worktree pushes failed preflight on a
  missing `node_modules`. Each was cheap to catch at the boundary and costly after it.
- **Wall-clock is in the key's slots.** One docs-only DELTA re-ran a 25–86 minute gate because the
  project tree slot covered the habit ledger. The fix keys the tree without habit atoms.
- **Session envelopes survive a disconnect badly.** The pre-disconnect envelope kept its dead
  owner pid, so berth claims and commit attribution refused until `epr session gc` sealed it and a
  successor opened. The degraded `epr` on PATH also lacks `commit-trailers`. `epr device arrive`
  landed a newer head (`bafyreiexj56…`), but `epr bin pin` refused both that build and the stored
  working one (`candidate-absent: explicit pin requires verified bytes`). So this session's commits
  carry their trailers in the message, and the pin refreshes at the next session start.
