---
title: "History/gotcha: A recovered doorway reveals a gap; it does not cause a regression"
id: recovered-doorway-reveals-gap-not-regression
type: history-gotcha
status: Accepted
tier: history
created: 2026-07-27
topic: [dual-doorway, resilience-card, failover, projection, diagnosis, saga-10]
# Distilled from the 2026-07-27 heads-converge handoff (removed 2026-09-24 in the
# .claude/handoffs drain). The narration is in git history; this record keeps the lesson.
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
cites:
  - "substrate-trust-contract-runbook | the dataplane invariants and per-red decision tree this gotcha adds one reading to — a surface that drops right after its doorway recovers | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - genesis/a2o/steps/dataplane/resiliency-saga.steps.ts
  - app/elohim-app/src/environments/environment.alpha.ts
memory_anchors:
  - project_resilience_card_data_plumbing
  - project_two_premises_dns_beacon_owned
---

# A recovered doorway reveals a gap; it does not cause a regression

> **Hot-context pointer:** when a doorway comes back from *degraded* and a surface it serves
> suddenly shows *less* truth (a card "loses its household", a count drops to zero), first ask
> **whether that doorway ever held the data**. A degraded doorway's reads can be answered by its
> sibling, so it shows the sibling's truth. A recovered doorway answers from its own projection. If
> that projection was always empty, recovery exposes a gap that was always there. Nothing regressed.

## What happened (2026-07-27)

The resilience card on `elohim.host` (doorway-B, adam) stopped showing the Dowell household the
evening B returned to health. It read like a regression, but B had never held `household-dowell`:
there were zero log mentions of it in three days, and B's resilience projection had been empty since
2026-07-23. While B was degraded, its readers were served A-derived truth, and the frontend's
`doorwayFallbacks` fail over across doorways. Once healthy, B truthfully served its own empty
projection. The a2o felt-safety step ("two doorways, one truth": both doorways report the same
non-zero `stewardingCollectives`, `resiliency-saga.steps.ts`) went red in the same build, and that
was the correct verdict. The underlying cause was structural and already on record: collectives had
no cross-peer reconcile arm, so the authoring peer was the only peer that held the rows
(`backlog/content-divergence-unhealable-without-canonical-heads.md`, finding 4).

## The rule

- **Diagnose which doorway answered, and from which projection, before you diagnose a regression.** A
  failover makes a sibling's data look like your own. Recovery removes that disguise.
- **The fix is replication, not rollback.** The cure is a reconcile arm that moves the rows to every
  peer (for this case, the collectives arm that landed in the 07-27/28 sprint), never a change that
  makes the recovered doorway borrow the sibling's answer again.
- **Keep the assertion comparing both doorways.** An assertion that checks one doorway at a time
  passes during failover. Only the two-doorway comparison catches a masked gap.

## Bidirectional links

- **This record → canonical:** the
  [substrate trust contract runbook](../architecture/2026-07-12-substrate-trust-contract-runbook.md)
  holds the invariants and per-red decision tree for the dataplane; this is one more reading of a red
  that the probes, not the symptom, settle.
- **Canonical → this record:** proposed as a one-line pointer in the runbook's §3 decision tree
  ("a doorway's surface drops right after it recovers → check whether it ever held the rows"), for the
  architecture tree's owner to add.
