---
id: "backlog-head-authority-carried-with-content-sync-unit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Head authority should be carried WITH its content sync unit — the adoption-trigger ladder is the conservative slice, not the destination"
slug: "head-authority-carried-with-content-sync-unit"
written: "2026-09-18"
author: "doorway-overnight-20260914 shift (doorway-failover pickup)"
status: "backlog"
priority: "high"
tags: [dataplane, head-adoption, trust-gradient, sync, elohim-storage, holochain]
cites:
  - elohim/elohim-storage/src/services/head_adoption_trigger.rs
  - elohim/elohim-storage/src/chain_write_gate.rs
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/final/household-gossip-wedge-notes.md
---

# Head authority carried with the content sync unit

## Principle (operator, 2026-09-17)

Verification cost is paid once, up front, next to the human judgment; a head should not quiesce
as its own property separately from the content it belongs to. Target shape: the sync unit
becomes `{content, signed head record}`, verified once by the receiving peer and projected
atomically ("AdoptPeer by verified record"), with a `trust_path` label on every adoption so the
per-edge price is measurable.

## What shipped 2026-09-17 as the conservative slice

Commits `d599b8587`, `8128a1475`, `cf280f8fd`:
`elohim/elohim-storage/src/services/head_adoption_trigger.rs` — sync-triggered adoption with a
re-probe ladder at 1/2/4/8/15/30s.

Measured on the household mesh: jessica adopted at 47.7s on attempt 3, james at 58.8s on
attempt 5 — versus 4m54s / 7m25s before the trigger existed. The scenario deadline is 75s, so the
margin is thin, and the ladder being *needed at all* is the evidence that DHT walkability lags
content sync by ~45–60s. That measurement is the case for the carried-record slice below, not a
sign the ladder is sufficient on its own.

## The scenario that pins the destination (measured 2026-09-18, final binaries)

`doorway-apex-transition.feature` "A current governed version crosses withdrawal and recovery",
step `doorway "elohim.host" serves authority B's exact head…` (`apex-transition.steps.ts:1252`)
reads the surviving doorway's head **once**, immediately after the canonical author publishes
authority B — no polling, by design (`assertExactPublishedAuthority`). On a healthy fresh household
the adoption trigger adopts on attempt 1 in ~1.0–1.1 s; the single read lands inside that second
and sees authority A (`governed action diverged`, runs 20260918T230005Z and 20260918T231158Z). The
ladder cannot close a zero-lag oracle; only a head that arrives WITH the content can. This step is
the acceptance check for item (1) below.

## Open items

1. **Carried-record adoption** — the successor of the ladder: sync delivers `{content, signed
   head}` as one unit; the receiver verifies once and adopts atomically instead of polling.
2. **`ChainTopOrdering::Relaxed` for head declares** — all declares are `Strict` today; `Relaxed`
   needs a rework of the `canonical_declared_at` / chain-head assertion in the content_store
   coordinator zome (`content_store/src/lib.rs` ~2942–2965). Until then, storage serializes cell
   writes through `elohim/elohim-storage/src/chain_write_gate.rs` (commits `08c619982`,
   `0ff361417`) and retries on `HeadMoved`.
3. **Pure-iroh transport path does not raise the adoption trigger** — only the libp2p/dual path
   does.
4. **Ordered `ContentHeadDeclared` delivery observed 0-for-176**, with gossipsub
   `InsufficientPeers`. Needs a diagnostic that names which conjunct of the delivery precondition
   failed, rather than a bare count.
5. **Trigger cooldown (60s claim) and the re-probe ladder are compile-time consts**, not
   runtime-config — no lever to retune without a rebuild.
6. **Defect, 2026-09-18**: james's `adopt-before-author` CONTESTED path re-mints a canonical
   declaration for the SAME 28 content ids every ~20 minutes (11× each so far) without checking
   that its earlier contested declaration already stands — a non-converging chain-write loop.
7. **Defect, 2026-09-18**: `reanchor_backfill` re-probes the same 29–43 `held` dead candidates
   every ~70s, forever — each probe a failing `declare_canonical_head` zome call, with no backoff
   for a held candidate.
8. **Parked draft**, uncompiled, on branch `sprint/2026-09-17-candidate-head-single-call`
   (commits `5e4dbaa1e` single coordinator extern `resolve_staging_candidate_head`, `462e87498`
   candidate-byte prefetch) — collapses the candidate-head resolve from ~7 zome calls to one.

## Missing node (mintable)

chain / between "sync delivers content" → "peer adopts head" / missing node "the sync unit
carries a signed head record verified once at receipt": assertion — the receiver's adoption
decision needs no re-probe ladder because the head arrived pre-verified with the content; probe —
adoption latency collapses to sync latency (no 1/2/4/8/15/30s stepping), measured against the
47.7s/58.8s baseline above. State: **not built** — item 1 above is exactly this node.

## Current decision

**Captured, not started** beyond the shipped conservative slice. Owner: next dataplane/head-
adoption shift. Items 6 and 7 are live non-converging loops on the household mesh right now and
are the cheapest wins (bounded, no design decision required) — a "does the contested declaration
already stand" check and a backoff on held reanchor candidates. Items 1–2 are the design-owning
work; item 8 is a ready-to-review draft that shortens the eventual carried-record path.
