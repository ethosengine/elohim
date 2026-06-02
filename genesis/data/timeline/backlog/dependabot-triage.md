---
id: "backlog-dependabot-triage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Triage Dependabot backlog — 170 vulnerabilities (109 high) on the default branch, untriaged"
slug: "dependabot-triage"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "cargo"
recurrence: 1
source_shifts:
  - "2026-05-17"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_cargo_resolution_vs_compilation"
  - "memory:feedback_subagent_dep_conflict_supervision"
tags: [cargo, security, dependabot, vulnerabilities, operator-domain]
shift_objective: |
  Dependabot reports 170 vulnerabilities (109 high severity) on the default branch, untriaged
  (surfaced 2026-05-17, multi). With no triage pass, the alert count is noise — there is no
  signal about which advisories are reachable in our code vs transitive-only, exploitable vs
  theoretical, or fixable-by-bump vs blocked-on-an-upstream.
  Resolve it with a triage pass that produces a reviewed disposition per advisory cluster:
  bump-now, blocked-on-upstream (with the blocking crate named), not-reachable (transitive
  dep we don't exercise), or accepted-risk (with rationale). Mind the cargo gotchas: a
  pre-release crate resolving is NOT the same as compiling (feedback_cargo_resolution_vs_compilation),
  and version bumps need crate-wide caller review (feedback_subagent_dep_conflict_supervision).
  This is operator+maintainer domain (the triage decisions and any risk-acceptance are
  operator calls). Done when the 170 alerts have a reviewed disposition and the high-severity
  reachable subset has a remediation plan.
---

# Triage the Dependabot vulnerability backlog

## Why this matters

170 untriaged alerts (109 high) is alarm fatigue, not signal — the security surface is
effectively unmonitored because no one can tell which alerts matter. A triage pass converts
the count into a small actionable set.

## The failure shape

- Dependabot raises 170 advisories on the default branch; none are dispositioned.
- No distinction between reachable-and-exploitable vs transitive-and-theoretical.
- No remediation plan for the high-severity subset.

## Shape of the fix (operator/maintainer-owned dispositions)

Per advisory cluster, assign: **bump-now** / **blocked-on-upstream** (name the crate) /
**not-reachable** (transitive, unexercised) / **accepted-risk** (with rationale). Guard the
cargo traps:

- Pre-release crate resolves ≠ compiles (`feedback_cargo_resolution_vs_compilation`) — build
  before pinning a bump.
- Version bumps need crate-wide caller review (`feedback_subagent_dep_conflict_supervision`).

## Acceptance

All 170 alerts have a reviewed disposition; the high-severity reachable subset has a
remediation plan.
