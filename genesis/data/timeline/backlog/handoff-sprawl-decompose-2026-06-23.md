---
id: "backlog-handoff-sprawl-decompose-2026-06-23"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Handoff-doc sprawl: restore the decompose discipline (README de-blessed) + inventory the residual docs for harvest + a2o navigation-track plan"
slug: "handoff-sprawl-decompose-2026-06-23"
written: "2026-06-23"
author: "deployment shakeout sprint (overnight, autonomous) — handoff-sprawl scout + a2o-coverage scout"
status: "resolved"
priority: "medium"
tags: [memory, decompose, handoff, hygiene, discipline, a2o, navigation, console-capture, shakeout]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
  - backlog-a2o-console-error-allowlist
cites:
  - .claude/skills/agentic-developer/SKILL.md
  - .epr-meta/manifest.md
  - genesis/data/timeline/backlog/a2o-console-error-allowlist.md
  - "recovered-doorway-reveals-gap-not-regression | the history record distilled from the 2026-07-27 handoff when this drain closed | sha256:ae674e6d3c29adb3 | path: genesis/docs/content/elohim-protocol/history/2026-07-27-recovered-doorway-reveals-gap-not-regression.md"
---

# Handoff-doc sprawl → decompose discipline (2026-06-23)

## Resolved 2026-09-24: `.claude/handoffs/` drained to zero residue and deleted

The holding pen's own `.epr-meta` set the exit condition: remove the directory once the handoffs are
harvested. That condition is now met. All nine remaining documents were checked against the tree
and git log, not against their own claims. Their signal went to living surfaces, and the files, the
`archive/README.md` and the `.epr-meta` were removed with `git rm`.

| Document | Where its signal lives now |
|---|---|
| `HANDOFF-2026-06-17-doorway-metrics.md` | **Landed.** `25bc75b1b` added the doorway `/metrics` surface. M1–M5 are all registered in `doorway/doorway-service/src/metrics.rs`. The subscriber close frame is now captured in `projection/subscriber.rs`. The PodMonitor and alerts are in `genesis/orchestrator/manifests/infra/`. The self-healing admission and breaker accessors show LANDED in `routes/self_healing.rs`. The prometheus 0.14 pin rationale is recorded in the `Cargo.toml` comments. |
| `2026-06-18-resilience-cards-dual-doorway-sprint-RESULT.md` + `-handoff.md` | **Landed or owned.** The `/auth/me` fork closed through `928bbb5ec` (recorded in `resilience-card-self-cid-provide-loop-gate.md`). The seeder's `dhtAnchorHash` at ingest is `2ef0cba16`, with residue in `seed-provenance-anchor-gap.md`. F-COHERENCE tasks 3–4 are `c173c8390`. The F-EDGE `connected_peer_count` and the `onlinePeers {live, known}` card honesty are both in-tree. Finding 1 (cross-node reach) stays with `http-reach-cross-node-fallback-bypass.md`. The CID-canonical rule is in the `p2p-design-gate` skill. |
| `2026-06-21-shakeout-shift-handoff.md` | **Landed.** `observe_kind` is now bounded rather than cfg-gated (`3ba7a5fc5`). The buffer hoist is `3ec9c3001`. eslint and `eslint-plugin-sonarjs@3.0.7` load cleanly (probe exit 0, 2026-09-24). The PVC-deferral lesson is in memory `feedback_pvc_deferral_hides_gate_debt`. |
| `2026-06-21-visual-verification-map-for-shakeout-shift.md` | **Narration.** It was a snapshot of build `922a11a`. Coherence and bootstrap now have a2o steps. The SPA-fallthrough JSON trap moved to `a2o-console-error-allowlist.md`. |
| `HANDOFF-2026-07-27-heads-converge-truthful-resilience.md` | **Owned or landed.** The RCA is in `content-divergence-unhealable-without-canonical-heads.md`. Adopt-before-author and the collectives arm landed. The recovered-doorway lesson went to history record `2026-07-27-recovered-doorway-reveals-gap-not-regression.md`. The UNSTABLE-is-not-finished polling trap went into the CI museum record. |
| `HANDOFF-2026-07-28-resilience-cards-converge.md` | **Durable.** The two-premises DNS work and the ddclient write-war are in memory `project_two_premises_dns_beacon_owned`. Relay capacity is covered by `relay-capacity.feature`. The coturn `conf-revision` rule is in the manifest comment. The `turn.*` CNAMEs are gone (NXDOMAIN, 2026-09-24). |
| `HANDOFF-2026-07-29-saga-sprint-gaps-closed.md` + `OBJECTIVE-2026-07-29-overnight-deliver-saga.md` | **Carried by sprint results.** See `genesis/docs/superpowers/sprints/2026-07-29-close-the-chapter-day-sprint-result.md` and `…-deliver-the-saga-overnight-sprint-result.md`. The open residue is owned by `shem-conductors-signal-hairpin-suspect-dht-silent.md` (susan's manifest-fetch observation was appended there), `declare-sweep-hash-only-cannot-converge-missing-action.md`, `genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md` (kitsune2 `is_direct`) and `projection-reconcile-loc-ceiling-decomposition.md`. |

The birth-side discipline this entry asked for is now enforced at the tool boundary. The root
`.epr-meta/manifest.md` rule `handoff-routes-to-sprints` dispatches every new handoff to a sprint
result. With the directory gone, nothing remains to route here.

**The a2o navigation and console-capture track** (Track C of the 2026-06-23 record) was still open
when this entry closed. It moved to `a2o-console-error-allowlist.md`, the opposite-polarity sibling on
the same filter seam. That entry now owns it.

---

*What follows is the 2026-06-23 record, cut down. The full text, including the residual-docs table
and Track C, is in git history. The table above supersedes both.*

## Why this exists

A scout found ~1,670 lines of handoff-doc sprawl across 16 files (repo root +
`.claude/handoffs/`). There is **no culprit skill** — the `handoff` skill was retired
2026-06-11 (`df250665f`); the sprawl is an *unowned convention* that the archive README
**blessed** ("keep ≤4 at root, archive the rest") — a volume gate, never a decompose gate.
The decompose discipline already exists (`agentic-developer` Close step 5, the
No-Dumping-Grounds law) but is scoped to `.claude/shifts/` plans, not handoffs.

## Done 2026-06-23

- **De-blessed the README** (`.claude/handoffs/archive/README.md`, now removed): archive became a
  temporary holding pen pending decompose, not a permanent store.
- **Removed 10 superseded docs** whose signal was already durable (git history for tracked;
  operator-declared-stale / commit-superseded for untracked). Conductor-leak/503/tx5/attestation
  signal lives in memory: `project_storage_metrics_surface_and_leak_verdict`,
  `project_tiered_quilt_unblock_state`.
