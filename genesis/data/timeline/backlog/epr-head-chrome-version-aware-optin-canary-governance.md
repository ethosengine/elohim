---
id: "backlog-epr-head-chrome-version-aware-optin-canary-governance"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Version-aware EPR-head chrome + reach-gated opt-in to competing heads — A/B/canary experiencing as the governance input that elects the head"
slug: "epr-head-chrome-version-aware-optin-canary-governance"
written: "2026-07-11"
author: "operator vision note (2026-07-11, mid dht-unity arc)"
status: "open"
priority: "medium"
area: "governance/head-election-ux"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "memory:project_versioned_entity_head_is_declared_dependency"
  - "memory:project_earned_reach_governance_pr_ceremony_vision"
  - "memory:project_resilience_card_data_plumbing"
cites:
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - genesis/a2o/features/dataplane/notary-authority.feature
tags: [epr-head, chrome, versioning, canary, governance, reach, imagodei, rea, brainstorm-needed]
---

# Version-aware EPR-head chrome + opt-in canary heads as governance input

## The operator's thought (2026-07-11, verbatim intent)

The epr-head "chrome" component (note: currently missing the drilldown
epr-resilience card) should be **version-aware, with a link to the actual
source** of whatever EPR is being rendered. While upgrades actually occur
on the peers, there could be A/B / Alpha/Beta/Canary-type testing going on:
users could — **if commons-reach appropriate — opt into competing versions
of a head**, and experiencing different versions + providing feedback
becomes **part of the governance process of electing the EPR head, or
establishing valid consensus for forks**. The choice is probably only
exposed to those with an **imagodei-conductor presence** (an auth identity).

## Why this is load-bearing (composition, not a new subsystem)

1. **It IS the missing social-grant arbitration.** The notary-authority
   feature explicitly names today's promotion decision "a dev-time
   hard-coded stand-in for the social grant." This note supplies the
   mechanism that replaces it: opt-in version experiencing → feedback
   (REA attention/assessment events) → attestation-derived election of the
   canonical head. The A/B infra isn't adjacent to governance — it *is*
   the governance input pipeline.
2. **The version DAG already models it.** `versioned-entity HEAD is a
   declared dependency` (cid-pin = lockfile; versions are a DAG with
   fork/revert/merge; *binding picks head*): an opt-in is just a
   reach-gated per-identity binding preference overriding the commons
   default — never a second canonical. The commons head stays the notary's
   answer; a canary opt-in is a declared, revocable, per-agent binding.
3. **Trust rule unchanged (REQ-N5/REQ-F4).** Competing heads are all
   NOTARIZED versions (staging-tier or earned); an opt-in switches which
   *verified* head this identity binds — it never adopts an unverified
   value. Same doorbell/verification grammar as dht-unity T4.
4. **Chrome = the legibility surface.** Version-aware chrome shows: which
   head you're on (canonical vs opted-in canary vs fork), the source link
   (the EPR's provenance/author lineage), and the drilldown
   epr-resilience card (currently missing — and data-starved per the
   resilience-card plumbing thread). This is the trust-legibility
   projection ("the padlock") growing version fluency.
5. **imagodei gating.** The opt-in choice surfaces only for agents with an
   imagodei-conductor presence — anonymous readers always get the commons
   canonical head. Feedback events need an author to be REA events at all,
   so the gate is intrinsic, not just cosmetic.

## Design questions for the brainstorm (p2p-design-gate applies)

- Entity classification: the per-identity binding preference (B agent-scoped?
  B2 with attestation once feedback counts toward election?); the feedback
  event (REA event on the existing rails vs new entry type — check headroom).
- Where the opt-in binds: doorway session (projection-layer) vs
  storage row (peer-layer) vs conductor (DHT-layer)? Likely projection-layer
  read preference resolved against verified heads only.
- Election math: what feedback aggregation earns promotion (ties into the
  council/PR-ceremony vision and the scale-envelope aggregation-window
  question — same rollup-attestation shape).
- Fork consensus: when competing versions do NOT converge, what makes a
  fork *valid* (legitimate sub-commons divergence) vs *failed election*.

## Sequencing

After dht-unity T4 (doorbell) lands: the doorbell gives every peer prompt
verified adoption of the elected head — the prerequisite for canary opt-ins
to be safe and observable. Chrome version-awareness + source link + the
resilience-card drilldown can start independently as UI work against the
existing `/db/content/{id}/head` + version-DAG surfaces.
