---
title: "Trust-Legibility Atlas — interim trust-earning states declare their why"
id: trust-legibility-atlas
tier: spec
status: Draft
created: 2026-07-18
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
topic: [trust, legibility, fail-closed, resilience-card, catching-up, gapKind, diagnosability, playwright, a2o]
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: features/trust suite green with all reason-assertions wired
domain: D-resilience
cites:
  - substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:cb76e9f0ae6bacfc | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/data/timeline/backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md
  - genesis/data/timeline/backlog/provide-reconciler-actual-set-transport-id-mismatch.md
informed-by:
  - genesis/a2o/features/resilience/commitment-backed-card-lighting.feature
---

# Trust-Legibility Atlas — interim trust-earning states declare their why

## Principle

The substrate already refuses to lie: reads shed `catching-up` while anchors
diverge; counters read zero rather than count unjoinable rows; the op-gate
observes before it enforces; reach is earned before it is served. These are
**trust-earning states, not error states** — the system is being honest.

But honesty without legibility reads as brokenness. On 2026-07-17 a single
operator session crossed **nine** distinct interim states, and every one was
diagnosable only through internal probes (Loki, the commitments API,
`/admin/self-healing`, `divergentAnchor` gauges). The person looking at the
surface got a bare `{"status":"catching-up"}` or a bare zero.

**The contract this atlas enforces: never a bare zero, never a bare 503.**
Every withheld render carries three things:

1. **why** — a machine-readable reason (the `gapKind` pattern, generalized),
2. **gauge** — the live measurement behind the reason (count, staleness, drift),
3. **earn-path** — what event would transition the state (the cure, named).

`placementGaps[].gapKind` (`contracts-short` / `peers-unavailable` with
`firstSeenAt`/`lastSeenAt`) is the existing in-repo proof of the pattern.
The fail-closed layer is already a faithful messenger; this spec makes it a
*legible* one.

## Canonical fixture

All atlas scenarios run over one content set: **`evolution-of-trust`** (already
the resilience-card fixture; seeded on the household floor; its name is the
point). Household floor (matthew/jessica/james + doorway A/B) suffices for all
states except where tagged `@requires:`.

## The taxonomy (states observed live, 2026-07-17)

| # | State | What withholds | Surface today | Internal gauge (exists today) | Required legibility delta | Earn-path |
|---|-------|----------------|---------------|-------------------------------|---------------------------|-----------|
| 1 | `contracts-short` | peer_selection: zero `provide` commitments match `content:<reach>` | ✅ `gapKind` + seen-window (the model) | `achievedStewardCount` | none — already conforming | a provide commitment lands |
| 2 | `peers-unavailable` | committed providers fail `online/degraded + pool-member` | ✅ `gapKind`, but not *which/why* | peer_statuses rows + staleness | per-provider summary: `n committed / n online / n stale-key / n offline` | providers heartbeat online under live keys |
| 3 | anchor-divergence shed | storage fail-closes reads while peer anchors disagree with local chain | ❌ bare `{"status":"catching-up","retryAfter":30}` | `divergentAnchor` (observed 2091→2177, climbing) | shed body: `{reason:"anchor-divergence", gauge, since, earnPath:"identity-lineage-bridge"}` | lineage bridge reconciles old-key anchors |
| 4 | namespace-dead join | transport-id (`12D3Koo…`) in an agent_cid join column — row silently excluded | ❌ silent zero | `elohim_identity_namespace_violation_total` | card `details` carries `excludedProviders: {reason: namespace, count}` | provider re-authored under `uhCAk…` |
| 5 | rekey-fossil drift | internally-consistent stale trio (humans+session+commitment on a dead key) | ❌ masquerades as #2 | `genesis-self-heal-rekey` transitions; peer_statuses 26-day staleness | availability summary distinguishes `stale-key` from `offline` | stale-for-self heal fires (LANDED 2026-07-17) |
| 6 | reach-not-earned | anonymous read before commons reach earned | ❌ bare 403 | reach ladder position | 403 body: `{reason:"reach-not-earned", contentReach, requesterStanding, earnPath}` | reach-earning attestation |
| 7 | observe-would-deny | op-gate shadow verdict — forwarding but would deny under enforce | ❌ log-only (`op-gate OBSERVE` warn) | `would-deny` summaries + performer | admin/diagnostics surface (or response header) exposing shadow verdicts per performer | delegates-compute grant seeded |
| 8 | head-divergence | two doorways notarized over different canonical heads | ❌ false-green: both render normally | notary-authority scenario; per-deploy head probe | Tier-C self-report on `/health` + views: `dnaHash + bootstrapId + signalId + declaredHead` | canonical head election + adoption |
| 9 | diversity-degraded | placement running household-blind, silently XOR | ❌ silent | candidates `household_id NULL` count | placement report: `strategy:"xor-degraded", reason:"households-unknown", gauge` | humans projection populated (LANDED) |

## Suite shape

`genesis/a2o/features/trust/trust-legibility-atlas.feature` — one scenario per
state, spec-first (`@wip`, no step defs yet). Each scenario has the same
three-beat shape:

```
Given  <the state is induced or fixtured over evolution-of-trust>
When   <the person-facing surface is read>
Then   the response declares reason "<state>" with a live gauge
And    the declared earn-path names the transition event
```

Plus a `@browser` Playwright leg per state (the existing
`E2E_DEVICE_MODE=playwright` rail + `pnpm look` screenshots) so each state is
*demoable to a human*: the screenshot shows the surface saying why trust is
not yet earned — the suite doubles as the demo deck.

## API deltas this drives (small, enumerated)

- **Shed body reason** (state 3): storage's catching-up 503 gains
  `reason/gauge/earnPath`; doorway forwards it verbatim (single-target proxy —
  no doorway logic).
- **Availability summary** (states 2/5): peer_selection outcome carries the
  per-provider breakdown it already computes internally.
- **Excluded-rows visibility** (state 4): the card's `details` object counts
  rows dropped per exclusion reason instead of dropping silently.
- **403 reason body** (state 6): reach gate mirrors the op-gate's
  deliberately-vague-to-clients rule *inverted* — reach state is not secret;
  the requester's own earn-path is theirs to see.
- **Shadow-verdict surface** (state 7): read-only diagnostics listing
  would-deny counts per performer (feeds the enforce-flip decision).
- **Tier-C self-report** (state 8): already a named TODO (fractal-federation
  §Tier-C); this suite is its acceptance test.

Wire shapes go through the view-schema contract
(`elohim/sdk/schemas/v1/views/` → Rust struct → codegen) — no hand-rolled
response fields.

## Non-goals

- Not an alerting system (observability owns thresholds).
- Not retry/self-heal logic (those layers exist and work; this is their voice).
- Not a UI redesign — the deltas are fields on existing responses; rendering
  them is a follow-on graphos concern.

## DoD

1. All 9 scenarios exist and parse (`@wip` until wired).
2. Each API delta lands with its schema + contract test.
3. Scenarios 1–7 green on the household floor; 8 needs the A/B doorway pair;
   9 green once household data flows.
4. `@browser` legs produce one screenshot per state — the atlas as demo deck.
