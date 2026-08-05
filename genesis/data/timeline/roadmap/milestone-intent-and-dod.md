---
id: "roadmap-milestone-intent-and-dod"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "Milestone intent & Definition of Done (the authored-intent layer)"
slug: "milestone-intent-and-dod"
written: "2026-06-25"
created: "2026-06-25"
author: "Matthew Dowell"
status: "active"
class: "authored-intent"
topic: [roadmap, milestones, definition-of-done, vision, authored-intent, derived-state]
target_window: "open-ended"
themes: [milestone-intent, definition-of-done, authored-vs-derived, vision]
tags: [roadmap, authored-intent, definition-of-done, milestones, hand-tended, durable]
cites:
  - genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md
---

# Milestone intent & Definition of Done

> **This is the AUTHORED-INTENT layer — the durable *why* and the *done* line, hand-tended.**
> It carries only what a human authors and keeps stable: the vision, the milestone map (M1–M6
> names + intent + ordering), and the per-milestone Definition of Done. It carries **no live
> status** — no completion percentages, no sprint task lists, no velocity, no risk register.
>
> **Live status lives in the generated roadmap:**
> `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md` — that is DERIVED STATE,
> regenerated each cartographer ceremony from the placement-audit ledger (`--ledger`/`--focus`)
> intersected with the vision axis. Never hand-edit it; never copy a percentage back here.
>
> The split (the one move applied repeatedly across this repo): **separate authored intent
> (durable, hand-tended, source-of-truth) from derived state (generated, never hand-edited)** —
> one home per fact, generate the rest. This doc is the authored home for *intent + DoD*; the
> generated roadmap is the derived home for *where we are*. The DoD rungs below are the top
> rung of the witnessed reach-chain (`2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md` §6).

---

## Vision Statement

The Elohim Protocol is distributed civilization infrastructure organized around love, encompassing
five pillars: identity (imagodei), learning (lamad), community (qahal), economy (shefa), and core
protocol coordination (elohim). Lamad is the first demonstrable vertical — people learn the protocol
by using it to learn. The initial deliverable: a person can register an account, establish an
identity, navigate curated learning paths, complete assessments that reveal something about
themselves, and track their mastery over time — all through a gateway (doorway) that custodially
manages their Holochain identity. The second person can join, and both can eventually run offline
desktop nodes that sync peer-to-peer. The economics layer (shefa) enables cooperative resource
stewardship, built on REA patterns, but only becomes relevant once the learning experience works
end-to-end for real people.

"Done" means: two humans can independently establish identity, log in, learn through paths with
assessments, see their progress persist, and eventually sync through an offline-capable node. The
economic and governance layers activate as the community forms.

---

## Milestone Map

The milestones are ordered and dependent: each builds on the one before. The names carry the intent;
the ordering carries the strategy (a demonstrable single-learner vertical first, then breadth, then
sovereignty, then community economics). **Current readiness, sprint breakdown, and per-milestone
completion live in the generated roadmap — not here.**

### M1: "Show Someone"
One person can register, log in, navigate a learning path, read content, and see their progress
tracked. You can demo this to another human being.

### M2: "Two Learners"
Two separate accounts on the same doorway, each with independent progress. Assessment flow works
end-to-end for at least one discovery instrument. Learner dashboard shows real data.

### M3: "Know Thyself"
All five discovery assessments completable. Results appear on profile as attestations. Path
adaptation responds to assessment outcomes. A learner can take the "Know Thyself" path from start to
finish.

### M4: "Take It With You"
Tauri desktop app packages the experience. elohim-node runs locally and syncs content. A learner can
work offline and rejoin. Doorway-app provides operator visibility.

### M5: "Community"
Economic events recorded through REA patterns. Stewardship commitments tracked. Basic request/offer
matching. Governance proposals readable (read path before write path).

### M6: "Cooperative"
Token generation from economic events. Insurance mutual claim filing. P2P sync between two family
nodes. Governance deliberation write path.

---

## Definition of Done

The DoD is the authored "done" line for each milestone — the predicate a milestone must satisfy to
be called complete. It is the top rung of the witnessed reach-chain: a milestone is *done* only when
its checkboxes are earned bottom-up by witnesses (scenarios, the ledger), never asserted top-down.
**The checkbox state itself is derived** — read the generated roadmap (or the placement-audit ledger)
for what is currently satisfied; this doc owns the *criteria*, not the *current tally*.

### M1: "Show Someone"
- A new user can register an account through the browser
- The user can navigate to a learning path and view content at each step
- Step completion is tracked and persists across page refresh
- Path overview shows progress percentage
- Learner dashboard shows active paths
- Content seeding completes without errors
- All images and blob content load correctly
- Seed verification script passes

### M2: "Two Learners"
- Two accounts on one doorway have independent, isolated progress
- Logging out and back in preserves all progress
- At least one discovery assessment is completable end-to-end
- Assessment result appears as an attestation on the user's profile
- Cross-device login shows consistent progress

### M3: "Know Thyself"
- All 5 discovery assessments are completable
- Assessment results drive path adaptation
- Bloom's mastery levels advance through quiz completion
- The "Know Thyself" path is completable start-to-finish
- Cypress E2E test suite covers the core learning journey
- Learner dashboard shows real mastery, activity, and attestation data

### M4: "Take It With You"
- Tauri desktop app installs and launches on macOS/Linux
- Desktop app connects to doorway and functions identically to browser
- elohim-node runs as a local sidecar
- Content is cached locally for offline access
- Offline progress syncs when connectivity is restored
- Two nodes on the same LAN discover each other via mDNS

### M5: "Community"
- Learning engagement, compute contribution, and content creation generate economic events
- Shefa dashboard shows real economic activity
- Requests and offers can be created and viewed
- Stewardship commitments are tracked
- Doorway-app shows operator metrics

### M6: "Cooperative"
- Token generation from economic events
- Insurance claims can be filed and adjudicated
- Governance proposals can be read and voted on
- Two family nodes on different networks can sync via bootstrap peers
- Token balances are visible and accurate

---

## What lives where (the authored/derived split)

| Concern | Home | Edited by |
|---|---|---|
| Vision + milestone intent + ordering | **this doc** (authored-intent) | a human, deliberately |
| Per-milestone Definition of Done (criteria) | **this doc** (authored-intent) | a human, deliberately |
| Current completion %, sprint breakdown, velocity, risk register, env-holds | `vision-readiness-sprint-roadmap.md` (derived) | the cartographer ceremony — never by hand |
| Which DoD checkboxes are currently satisfied | derived (placement-audit `--ledger` / generated roadmap) | regenerated, never copied here |

The prior root `ROADMAP.md` fused both layers (intent + volatile status) and drifted stale. This doc
keeps the durable half; the generated roadmap keeps the live half. The full rationale is the
doc-lifecycle framing spec
(`genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md`,
§6 the witnessed Definition of Done, §12 P0).
