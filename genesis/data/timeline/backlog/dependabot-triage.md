---
id: "backlog-dependabot-triage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Triage Dependabot backlog — 191 vulnerabilities (1 critical, 113 high) on the default branch, untriaged"
slug: "dependabot-triage"
written: "2026-06-02"
author: "cartographer"
status: "backlog"
priority: "high"
area: "cargo"
recurrence: 2
source_shifts:
  - "2026-05-17"
domain: "operator"
deprecation_status: blocked
severity: security
fingerprints: [c4bc9714e080]
relatedNodeIds:
  - "memory:feedback_cargo_resolution_vs_compilation"
  - "memory:feedback_subagent_dep_conflict_supervision"
tags: [cargo, security, dependabot, vulnerabilities, operator-domain]
cites:
  - https://github.com/ethosengine/elohim/security/dependabot
shift_objective: |
  Dependabot/GitHub reports 191 vulnerabilities (1 critical, 113 high, 62 moderate, 15 low) on
  the default branch, untriaged (first surfaced 2026-05-17 at 170/109-high; re-surfaced
  2026-06-06 as a push-time banner at 191/113-high — count is growing, not draining). With no
  triage pass, the alert count is noise — there is no signal about which advisories are
  reachable in our code vs transitive-only, exploitable vs theoretical, or fixable-by-bump vs
  blocked-on-an-upstream.
  Resolve it with a triage pass that produces a reviewed disposition per advisory cluster:
  bump-now, blocked-on-upstream (with the blocking crate named), not-reachable (transitive
  dep we don't exercise), or accepted-risk (with rationale). Mind the cargo gotchas: a
  pre-release crate resolving is NOT the same as compiling (feedback_cargo_resolution_vs_compilation),
  and version bumps need crate-wide caller review (feedback_subagent_dep_conflict_supervision).
  This is operator+maintainer domain (the triage decisions and any risk-acceptance are
  operator calls). Done when the 191 alerts have a reviewed disposition and the
  critical+high-severity reachable subset has a remediation plan.
---

# Triage the Dependabot vulnerability backlog

## What is flagged (quote the banner)

> remote: GitHub found 191 vulnerabilities on ethosengine/elohim's default branch
> (1 critical, 113 high, 62 moderate, 15 low).

Captured as security ledger fingerprint `c4bc9714e080` (2026-06-06, push-time banner). The
earlier cartographer capture (2026-05-17) read 170/109-high; the count has grown, confirming
the surface is unmonitored, not draining.

## Why this matters

191 untriaged alerts (1 critical, 113 high) is alarm fatigue, not signal — the security
surface is effectively unmonitored because no one can tell which alerts matter. A triage pass
converts the count into a small actionable set.

## The failure shape

- Dependabot/GitHub raises 191 advisories on the default branch; none are dispositioned.
- No distinction between reachable-and-exploitable vs transitive-and-theoretical.
- No remediation plan for the critical/high-severity subset.

## Usage inventory (blast radius — bounded scope pass, 2026-06-06)

The advisories span the full polyglot dependency surface; this is why the count is large and
why a per-advisory disposition is operator-owned, not a background bump:

- **Cargo**: 33 `Cargo.lock` files across the workspace + vendored subrepos (`elohim/rust-ipfs`,
  `elohim/brit`, `elohim/rakia/...`), ~1,398 unique crates total. Three subrepos carry their
  own `.github/dependabot.yml` (`rust-ipfs`, `brit`, `rakia/elohim/brit`) — vendored advisory
  noise the top-level repo also inherits.
- **pnpm/npm**: root `pnpm-lock.yaml` (~28.7k lines), `sophia/pnpm-lock.yaml` (~19k lines),
  `che-devworkspaces/package-lock.json`.
- **No top-level `.github/dependabot.yml`** — only the three vendored subrepo configs exist, so
  there is no central update cadence or grouping policy at the repo root.

Per-advisory enumeration is **not reachable from the dev environment**: no `gh` CLI, no
`cargo-audit` binary, and `GET /repos/ethosengine/elohim/dependabot/alerts` returns HTTP 401
(requires a token the dev env doesn't hold). The authoritative list lives at the cited
GitHub security tab and is an operator-token read.

## Shape of the fix (operator/maintainer-owned dispositions)

Per advisory cluster, assign: **bump-now** / **blocked-on-upstream** (name the crate) /
**not-reachable** (transitive, unexercised) / **accepted-risk** (with rationale). Guard the
cargo traps:

- Pre-release crate resolves ≠ compiles (`feedback_cargo_resolution_vs_compilation`) — build
  before pinning a bump.
- Version bumps need crate-wide caller review (`feedback_subagent_dep_conflict_supervision`).

### Suggested first-pass sequencing (plan sketch for the operator sprint)

1. **The 1 critical first** — pull its GHSA from the security tab, identify the crate/package
   and whether it's reachable (direct dep vs transitive). A single critical is the one item
   worth a same-day decision.
2. **Cluster the 113 high by root crate/upgrade-unit**, not by alert — most of 191 will
   collapse into a handful of transitive trees (one outdated crate fans out into many CVEs).
   Canonicalize each *upgrade unit* as its own concern if it earns a distinct trajectory.
3. **Add a top-level `.github/dependabot.yml`** with grouped updates so the count stops growing
   silently between triage passes (this is the one bounded sub-deliverable that does NOT
   require per-advisory decisions and could land independently).
4. **Vendored subrepos** (`rust-ipfs`/`brit`/`rakia`) — decide whether their advisories are
   in-scope (we build them) or should be excluded from the top-level surface.

## Current decision (blocked — operator-initiated sprint)

**BLOCKED.** This is a security-class concern whose remediation crosses a dependency-major /
>20-file surface (33 Cargo.lock files, ~1,398 crates, two large pnpm trees) and whose core
work — per-advisory disposition and any risk-acceptance — is explicitly an operator/maintainer
decision. Per the deprecation-triage hard rule ("if the fix would touch >20 files or change a
dependency major version, STOP at blocked with a written plan sketch — that scale needs an
operator-initiated sprint, not a background agent"), the terminal automation state is
**blocked-and-canonicalized**. Ledger fingerprint `c4bc9714e080` is marked `triaged` so the
sentinel cites this decision and does not re-dispatch on the recurring push banner; the
deprecation-stasis sweep owns the re-check.

The one independently-landable, non-operator-gated sub-deliverable is item 3 above (a
top-level grouped `.github/dependabot.yml`). It is left here as a trajectory rather than
landed this run because it was not in the captured fingerprint's bounded scope and a config
that changes the org-wide update cadence is itself an operator policy choice.

## Acceptance

All 191 alerts have a reviewed disposition; the critical+high-severity reachable subset has a
remediation plan. Re-check trigger: the next push banner whose count differs from 191 (the
sentinel will re-capture as a new fingerprint), or operator pickup of the shift_objective.
