---
id: "backlog-adam-pull-loop-wedged-at-boot"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam's projector pull loop is wedged at boot (total=0, fetched=0, caughtUp=false for hours) — doorway B answers catching-up on every head-record/EPR read"
slug: "adam-pull-loop-wedged-at-boot"
written: "2026-08-10"
author: "batch-3 ghost-declaration diagnosis session"
status: "backlog"
priority: "high"
tags: [dataplane, projector, pull-loop, catching-up-shed, adam, doorway-b, self-heal-exhaustion, saga]
cites:
  - genesis/data/timeline/backlog/2026-07-10-server-side-epr-read-path-catching-up-shed.md
  - genesis/data/timeline/backlog/adopt-before-author-evidence-starvation.md
---

# adam's pull loop never completes its first pass — B-plane reads shed forever

## Evidence (2026-08-10, live)

- `https://elohim.host/p2p/status` → `pull: {total: 0, fetched: 0, pending: 0,
  failed: 0, caughtUp: false}` — hours after the last restart, the puller has
  not completed (or started) a first pass. Not backlog lag: **zero** items
  have ever entered the window.
- Every `GET /db/content/{id}/head-record` on doorway B answers
  `{"status":"catching-up","retryAfter":30}` — the catching-up shed is
  permanent, so adam cannot serve head records over HTTP, and B-caughtUp read
  false for the whole of validate-only run #1339's 45-minute gate window
  (telemetry-only for the gate, but a standing self-heal exhaustion).
- Contrast matthew: `pull: {total: 2, fetched: 2, caughtUp: true}`.

## Why it matters

adam is the genesis-pair supplier peer with the largest anchored corpus; a
permanently-shedding B plane removes one of two doorway testimonies (the
guide-star convergence bar is two doorways testifying the same footprint) and
hides adam's conductor behind a shed for every HTTP-path read. This is a
distinct defect from the ghost-declaration deadlock (cured on branch
feat/angular22-node24, 2026-08-10) — the p2p-plane view-federation responder
still answers, so record supply is not gated on this, but the HTTP surface and
the B-side caughtUp telemetry are.

## Causal correction (operator Codex probe + live check, 2026-08-10 evening)

- **`pull.caughtUp=false` does NOT cause the doorway shedding.** Source
  tracing (read-only Codex probe): the doorway's catching-up shed reads
  `projectionReconcile.caughtUp`, not `pull.caughtUp`. The two symptoms may
  share startup pressure, but that relationship is unproven — do not fold
  them into one defect.
- **The startup-hydration suspect is FALSIFIED live.** Hypothesis was that
  hydration (full content rows + a per-row tag query where only IDs are
  needed) blocks creation of the acquisition ticker. Loki, adam, every boot
  in the last 24h: "P2P node started" → "Loaded local content IDs for
  replication state" (count≈4454–4462) lands **~1–2 seconds** later
  (05:01:34→05:01:36, 09:50:52→09:50:54, 12:09:00→12:09:01). Hydration
  completes fast; the ticker is not materially delayed. (The
  IDs-only-but-loads-full-rows inefficiency is still real as a cleanup, just
  not this wedge's cause.)
- **Also observed:** adam restarted 4× in ~10h (boots 02:09, 05:01, 09:50,
  12:09) — restart cadence itself deserves an explanation before any
  single-boot theory.

## Next probes

1. Why does the pull loop report total=0 — never scheduled (init-order gate
   waiting on a bridge/condition that never fires on adam), or first query
   hung on the conductor with no timeout? Find the pull loop's boot path in
   `elohim/elohim-storage` and its gating condition; check adam's boot logs
   for the loop's first log line vs its absence. (Hydration is NOT the
   blocker — see the causal correction above.)
2. If the first pull query hangs on the conductor: does it use a bounded call
   (cf. the head-record responder's 5s budget) or an unbounded
   `HcClient::call_zome` (~60s ws timeout, retried forever)?
3. What flips `projectionReconcile.caughtUp` on adam, and why it stays false —
   that (not `pull`) is the doorway-shed input.
4. Why adam restarts every few hours (crashloop? OOM? deploys?) — read the
   pod's restart reason before theorizing about any single boot.

## Scoped follow-ups (separately claimable)

- **Status honesty (`pull: null`)**: publish the already-schema-valid
  `pull: null` until the first reconcile completes, instead of presenting
  default zeroes as if work had begun and finished empty (C4: "never started"
  vs "started and behind" are currently collapsed).
- **Direct `/head-record` HTTP budget**: the HTTP route lacks the p2p
  responder's response budget and can exceed doorway's 12s request limit.
  Note: merely wrapping `HcClient::call_zome` in a timeout is NOT a complete
  fix — it does not cancel conductor work (same reason
  `HealPacing::batch_extern_budget` stays strictly below `attempt_timeout`).
