---
id: "backlog-ci-genesis-agent-bindings-conductor-fanin"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Seed Agent Peer Bindings fails 7/7 humans: first-reachable-wins fans every human's bindings onto adam's conductor (the #1119 affinity fix was never backported)"
slug: "ci-genesis-agent-bindings-conductor-fanin"
written: "2026-07-29"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [db7030259d93]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, genesis, seeding, agent-peer-bindings, conductor-affinity, imagodei, alpha]
cites:
  - genesis/seeder/src/seed-agent-bindings.ts
  - genesis/seeder/src/seed-conductor-identities.ts
  - genesis/seeder/src/__tests__/seed-agent-bindings.spec.ts
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/agent_peer_binding.rs
  - genesis/data/timeline/backlog/ci-genesis-household-founder-binding.md
  - genesis/data/timeline/backlog/ci-alpha-cluster-degraded-substrate.md
  - genesis/data/timeline/backlog/app-port-4445-auth-timeout-fleet-wide.md
---

# Agent Peer Bindings fans every human onto one conductor

## The failure

`Seed Agent Peer Bindings` reported total failure in every overnight
elohim-genesis run of 2026-07-28→29 (#1380–#1386, all UNSTABLE):

```
=== Results: 0 bindings written, 0 humans succeeded, 7 humans failed ===
```

Every per-human line names the **same** conductor — adam's — regardless of
whose bindings are being written (#1386):

```
[X] Adam     node    [desktop]              created=0/1 ws://elohim-adam-alpha…:4445 (desktop: Request timed out in 60000 ms: call_zome)
[X] Matthew  doorway [desktop+node+steward] created=0/3 ws://elohim-adam-alpha…:4445 (desktop: … ; node: … ; steward: …)
[X] Eve      device  [desktop]              created=0/1 ws://elohim-adam-alpha…:4445 (desktop: Request timed out in 60000 ms: call_zome)
[X] James    device  [desktop]              created=0/1 ws://elohim-adam-alpha…:4445 (desktop: Request timed out in 60000 ms: call_zome)
[X] Jessica  device  [desktop+mobile]       created=0/2 ws://elohim-adam-alpha…:4445 (desktop: … ; mobile: …)
[X] Pete     device  [desktop]              created=0/1 ws://elohim-adam-alpha…:4445 (desktop: Request timed out in 60000 ms: call_zome)
[X] Terrance device  [desktop]              created=0/1 ws://elohim-adam-alpha…:4445 (desktop: Request timed out in 60000 ms: call_zome)
```

Occurrence evidence: folded into ledger fingerprint `db7030259d93`
(`red build, stage:Seed Substrate`, seen 52, first_build 1262,
last_build 1386) — whose note already recorded `Agent Peer Bindings 0/6`
back at **#1262 (2026-07-06)**.

Jenkins has pruned this job to `#714` plus a contiguous `#1367–#1386`. Over
that whole retained window the stage is **intermittent, not always-zero** —
and the shape is diagnostic:

| Build | Results line | Note |
|---|---|---|
| 1370 | `0 bindings written, 0 humans succeeded, 6 humans failed` | |
| 1372 | `7 bindings written, 4 humans succeeded, 2 humans failed` | Jessica hit a source-chain-head race, not a timeout |
| 1373 | `9 bindings written, 6 humans succeeded, 0 humans failed` | **full success — all 9 written on adam** |
| 1374–1375 | `0 bindings written, 0 humans succeeded, 6 humans failed` | |
| 1376 | `9 bindings written, 6 humans succeeded, 0 humans failed` | **full success — all 9 written on adam** |
| 1377–1382 | 4–8 bindings, 2–5 humans succeeded | partial |
| **1383–1386** | `0 bindings written, 0 humans succeeded, 7 humans failed` | unbroken; roster grew 6→7 (James) at #1383 |

So the *overnight* 7/7 window is real and new, but it is the tail of a
long-running intermittency — and the "successful" runs are the more
alarming ones (see Root cause §2).

## Verdict: REAL — and the "4445 auth-timeout class" is a costume

The **control** is in the same build, seconds earlier. `Seed Conductor
Identities` routes per-human and gets mixed results at the same instant
(#1386):

```
[C] Matthew  doorway ws://elohim-matthew-alpha…:4445 (conductor already embodies '378e2c66-…')
[X] Adam     node    ws://elohim-adam-alpha…:4445    (Request timed out in 60000 ms: call_zome)
[X] Eve      device  ws://elohim-eve-alpha…:4445     (Request timed out in 60000 ms: call_zome)
[=] James    device  ws://elohim-james-alpha…:4445
[=] Jessica  device  ws://elohim-jessica-alpha…:4445
[-] Pete     device  (none) (no conductor deployed for this human …)
[-] Terrance device  (none) (no conductor deployed for this human …)
```

James's and Jessica's conductors answered `call_zome` fine. Matthew's
conductor answered too (it returned a *conflict*, which requires a
completed round-trip). So the fleet was not down — **only adam and eve were
saturated**, and the bindings stage fanned all seven humans into adam.

Against the museum trap list
(`2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`): this is
not #1 (NOT_BUILT/superseded — the builds ran), not #5 (sccache), not #6
(`#[ignore]`), not #7 (cucumber parse). It is a *new* instance of the
**#1119 first-reachable-wins** class already canonicalized in
`ci-genesis-household-founder-binding.md` — the sibling seeder that never
got the backport. The lesson generalizes and is recorded below.

**The `app-port-4445-auth-timeout` background class is NOT the cause.**
That class (`app-port-4445-auth-timeout-fleet-wide.md`) is a *handshake*
failure: the conductor logs `timed out while awaiting authentication.
Dropping connection`. Here the client completes the handshake — it opens
the admin WS, lists apps, authorizes signing credentials, issues an auth
token and opens the app WS — and then `call_zome` times out at 60 s. Same
port number, different phase. It is a costume: adam's conductor is
saturated (the write-path half of `ci-alpha-cluster-degraded-substrate`),
and the fan-in defect converts that one saturated pod into a 100 % stage
failure.

## Root cause

`genesis/seeder/src/seed-agent-bindings.ts` walked `CONDUCTOR_URLS` in order
and stopped at the first conductor whose installed app matched the
`INSTALLED_APP_ID` prefix (`elohim`). **Every** alpha conductor installs an
app with that prefix, so the walk always stopped at index 0 — adam.

`seed-conductor-identities.ts` had this exact bug and it was fixed in
`e9396d164` ("conductor-identity affinity fix") with the name-affine
resolver `urlsAreNameAffine` / `conductorUrlForHuman` — a human seeds ONLY
onto their own `elohim-<name>-<env>` pod. The bindings seeder predates that
fix (`fd8ab161e`) and was never backported.

Two consequences, only the first of which shows in CI:

1. **Availability.** One saturated conductor = 7/7 stage failure, and each
   attempt burns the full 60 s client timeout (matthew's 3-archetype plan
   burned 3 minutes alone).
2. **Correctness — the silent one, and the reason the GREEN runs are
   worse.** `seed-agent-bindings.ts`'s own header asserted the signer-match
   precondition holds "because each human's seeder call runs on their own
   conductor whose agent IS the human's agent." That premise was false.
   The zome gate in `imagodei/src/agent_peer_binding.rs` rejects a caller
   whose pubkey does not match the Agent EPR's `holochain_agent_key` — but
   it carries an explicit Stage-1 carve-out: if the Agent EPR is *not
   found*, the gate is skipped entirely. A log sweep of the retained window
   finds **zero** occurrences of `signer mismatch`, and #1373/#1376 wrote
   9 bindings for 6 humans with 0 failures — all on adam. So the carve-out
   path is the one being taken: the writes SUCCEED, on adam's source chain,
   with the forward link `create_link(caller_pubkey → binding)` anchored on
   **adam's** pubkey for every human. The per-human topology aggregation
   this seeder exists to feed is therefore silently wrong on exactly the
   builds that reported success. The saturation was masking a provenance
   bug, and a green stage was the failure mode with no alarm on it.

3. **Single-chain contention — visible in the logs.** Fanning ten zome
   calls onto one agent's source chain serializes them and makes them race.
   #1372 caught it directly on Jessica's mobile binding:

   ```
   Source chain error: Attempted to commit a bundle to the source chain, but the
   source chain head has moved since the bundle began. Bundle head: Some(ActionHash(
   uhCkkkld-SAeP4PYwX_4Gu1W3NQKawI6oSJBPHHNie60kIlZJdKgK)), Current head: Some(
   HeadInfo { action: ActionHash(uhCkkzaUZC_aydUMNGfznKP34xkeUJqhsGXyj91uvgAYj0FkQuTMk), seq: 7806, … })
   ```

   Seven humans writing to one chain at seq ~7806 is contention the design
   never intended; per-human routing removes it by construction.

Secondary: Pete and Terrance have no `elohim-pete-*` / `elohim-terrance-*`
conductor at all. Identities skips them softly (`[-]`); bindings charged
them as hard failures against adam's pod, so "7 of 7 humans" was an
unreachable denominator even on a perfectly healthy fleet.

**On the #1383 inflection:** the roster grew 6→7 (James's Human profile was
created at #1383) in the same build the stage went stably zero. Adding one
human does not by itself flip 5-succeed to 0-succeed; the more likely
reading is that adam's conductor degraded further around then, and the
extra call simply pushed an already-marginal serialized batch past the 60 s
wall. Treat the inflection as substrate, the amplification as this entry.
Not independently confirmed — no pod-side restart/OOM evidence was pulled
for `elohim-adam-alpha` around #1383, and that is the open question if the
residue persists.

## Current decision

Bounded fix LANDED and locally verified — awaiting disappearance
confirmation from the genesis pipeline (`ci_status: in-progress`). The
residual red on this stage after the fix deploys will be **adam-only**, and
that residue belongs to `ci-alpha-cluster-degraded-substrate` (adam
conductor saturation), not here. If a non-adam human still fails after the
next genesis run, reopen this entry.

Not blocked on anything: no cluster access, no pipeline-architecture
change, no operator move required.

**Follow-on this fix does NOT do — mis-provenanced bindings already on the
DHT.** The green runs (#1372/#1373/#1376/#1377/#1378/#1379/#1380/#1382)
wrote on the order of 50 AgentPeerBinding entries onto adam's source chain,
each linked from adam's pubkey regardless of whose `agent_cid` it carries.
The seeder has no Stage-1 dedup and no supersede path, so correct bindings
written after this fix will coexist with the wrong ones rather than replace
them. Reconciling that is a separate, data-shaped concern (supersede via
`superseded_by`, or an agent-scoped sweep) and wants a rust-architect
decision on whether the `AgentToPeerBinding` link anchor or the entry's
`agent_cid` is authoritative for the topology join. Filed here rather than
split out because it has no CI fingerprint of its own yet; promote to its
own entry if it grows a work plan.

## Fix trail

`genesis/seeder/src/seed-agent-bindings.ts`:

- Imports `urlsAreNameAffine` / `conductorUrlForHuman` / `humanShortName`
  **from** `seed-conductor-identities.ts` rather than re-copying them —
  re-copying is precisely how this seeder drifted back to
  first-reachable-wins. New exported pure helper
  `candidateConductorUrls(humanId, urls)` wraps the resolution.
- A human with no deployed pod is now `skipped` (`[-]`), not `failed`, and
  is out of the success denominator (`counts.total` = seedable;
  `counts.targeted` = all).
- Fail-fast: a `call_zome` timeout aborts the remaining archetypes for that
  human (same cell — they would only re-prove the same fact at 60 s each)
  and reports them as `not attempted`.
- Lossy-measure guard (museum anti-pattern #1): all-targeted-humans-skipped
  now exits 1 with the offending `CONDUCTOR_URLS`, instead of exiting 0
  with zero bindings written.
- Header contract corrected — it asserted a precondition the code did not
  establish.

`genesis/seeder/src/__tests__/seed-agent-bindings.spec.ts` (new, 8 tests):
pins the regression directly — with adam FIRST in the URL list, no non-adam
human may resolve to adam; humans without a pod resolve to `[]` (skip); the
legacy walk survives for non-affine local-dev URL sets; `isZomeTimeout`
matches the CI line verbatim without swallowing `signer mismatch`.

Verified locally: `pnpm exec vitest run` in `genesis/seeder` — 387 passed,
9 skipped, 28 files. `pnpm run typecheck` reports zero errors in the
touched files (the 18 remaining `@elohim/storage-client` export errors are
pre-existing, in untouched files, and unrelated).

## The generalized lesson (candidate for the museum)

**A prefix-matched service walk is not a routing strategy.** When every
member of a pool satisfies the match predicate, "first that matches wins"
silently degenerates to "index 0 always wins" — which reads as a
fleet-wide outage whenever index 0 is the unhealthy member, and as correct
behavior whenever index 0 happens to be healthy. This has now bitten the
genesis seeders twice (#1119 identities, #1380–#1386 bindings) on the same
`CONDUCTOR_URLS` list. The fix both times is name-affinity plus an explicit
`skipped` state for "no pod for this member" — never a broader walk.
Diagnostic tell: **every failing row names the same target**.
