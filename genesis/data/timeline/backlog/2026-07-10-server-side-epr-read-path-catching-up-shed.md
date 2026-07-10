---
id: backlog-server-side-epr-read-path-catching-up-shed
kind: backlog
title: server-side /epr read-path sheds 503 catching-up under sustained load (E2E flake + real read degradation)
created: 2026-07-10
status: OPEN
domain: D-dataplane
source: genesis #1272 E2E evidence + doorway-client catching-up seeder fix (ec5f0f522) — the read-path twin
severity: high
tags: [projector, admission-control, backpressure, p2p-dataplane, catching-up]
---

**Context.** genesis #1272 (UNSTABLE) fails in two families. The *write* family —
`Seed Stewardship Allocations` + `Seed Projections` 503ing on
`{"status":"catching-up","retryAfter":30}` — is now cured client-side: the seeder retries
the shed until the projector drains (`ec5f0f522`, catching-up retry hoisted into
`DoorwayClient.fetch()`). This item is the **read-path twin**, which that fix does NOT touch.

**The read-path shed.** During `E2E Verification`, browser requests to serve EPR content shed
under sustained load:

```
Failed to load resource: 503 (https://alpha.elohim.host/epr/foundations-christian-technology)
Failed to load resource: 503 (https://alpha.elohim.host/epr/unit-core-principles)
```

followed by post-shed 400/500 instability. These are the SAME projector-backpressure admission
shed, but on GET `/epr/*` reads — and browsers do not carry the seeder's catching-up retry, so a
real user (not just the E2E harness) sees a 503 on a content read whenever the projector is behind.
This is genuine read degradation, not merely a test flake.

**Why a client retry is the wrong home here.** Retrying on the seeder is right (a shed *write* was
rejected, retry lands it). But shedding *reads* that could be served from already-projected state is
the deeper defect: admission control should not shed a read the node can already answer. The cure is
server-side — one or more of:

- **Don't shed reads on `catching-up`** — reads of already-projected content are safe during
  catch-up; scope the shed to writes (or to reads of not-yet-projected entities only).
- **Raise/segment the admission threshold** so expected seed/test bursts don't trip the read shed.
- **Pace the projector drain** (closed-loop) so it keeps up under burst — see prior art
  `project_closed_loop_ingest_drain_prior_art` (`drain_publish_queue` + wait-for-drain) and the
  inbound-admission-backpressure plan (`genesis/docs/superpowers/plans/2026-06-13-inbound-admission-backpressure-plan.md`).

**Where it lives.** doorway `/epr/*` serve path + elohim-storage admission/projector
(rust-architect). Design gate: this is a serve-path behavior change, not a new entity — no
p2p-design-gate entity classification needed, but confirm the shed decision reads projection state
(is-this-CID-projected) rather than a global catching-up flag.

**Acceptance.** Under a seed/E2E burst, GET `/epr/<already-projected-cid>` returns 200 (served from
projection), never 503; only reads of genuinely-unprojected entities may 503. genesis E2E `/epr/*`
503 count → 0.
