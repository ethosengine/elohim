---
id: "backlog-resilience-card-self-cid-provide-loop-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR resilience card zeros — the provide-loop is gated OFF by empty self_cid (SELF_CID env unset in all manifests); long pole is conductor-anchoring (netpol+reseed, operator)"
slug: "resilience-card-self-cid-provide-loop-gate"
written: "2026-06-14"
author: "agentic-developer (felt-status shift, operator OPEN #1 diagnosis)"
status: "backlog"
priority: "high"
ci_status: blocked
tags: [resilience, dht-anchor, self-cid, provide-loop, conductor, netpol, reseed, durability-arc, operator-gated]
cites:
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/services/provide_reconcile.rs
  - elohim/elohim-storage/src/services/conductor_commitment_author.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - genesis/orchestrator/manifests/network-policies.yaml
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
relatedNodeIds:
  - backlog-resilience-tier-content-declared-floor
  - backlog-ci-genesis-conductor-adminws-unreachable
---

# EPR resilience card zeros — the self_cid provide-loop gate (diagnosis)

Diagnosis of the sprinter handoff's OPEN #1 (`SPRINTER-HANDOFF-2026-06-14.md`),
done during the felt-status shift (the felt surface this lights:
`feltStatus` on ResilienceSnapshotView, commit 6a754f30f). This is Workstream D
of the EPR Content Durability Arc plan; recording the precise gate so the next
session/operator doesn't re-derive it.

## The chain (code-verified)

The card zeros (stewardingCollectives 0, commitmentBacked 0, no `content:<reach>`
provide rows) are gated, in order:

1. **`self_cid` is empty → the provide-loop never spawns.** `main.rs:959-966` spawns
   the P1 reconciliation WRITE half (the `replicates-commons`/`replicates-content`
   authoring tick → writes the `content:<reach>` provide rows the snapshot reads)
   ONLY when `config.self_cid` is `Some` and non-empty. `self_cid` is sourced
   solely from the `SELF_CID` env var (`main.rs:366-370`). **`SELF_CID` is set in
   NO deploy manifest** (grep of genesis/manifests + genesis/orchestrator = empty).
   → `self_cid` is always `None` → the provide-loop is **permanently dormant** →
   no provide rows → snapshot zeros. The loop logs once at startup:
   *"Slice-2b provide-loop authoring tick disabled: requires lamad HcClient + db
   pool + non-empty self_cid."*
2. **Even spawned, it authors via the conductor.** `provide_reconcile::reconcile_provides`
   → `conductor_commitment_author` Step 1 notarizes the `replicates-content`
   Commitment via the lamad HcClient → needs the conductor reachable.
3. **Content anchoring at import also needs the conductor.** genesis #1145 seeder
   hit `reach circuit OPEN after 5 consecutive conductor-path failures →
   provenance-only stamping → rows stamp-failed` (cf. main.rs:616 "matthew/jessica
   failed 5/5 on alpha 2026-06-11"). No conductor anchor → no content provenance →
   reach re-notarization fails.

## The long pole is OPERATOR/CLUSTER-gated (not a bounded local code fix)

The provide-loop / author / reconcile CODE already exists and is correct. What's
missing is **conductor reachability**, which is the durability arc's netpol gate:
`genesis/orchestrator/manifests/network-policies.yaml` is **operator-applied**
(`kubectl apply` — never from dev) and conductor seeding "has NEVER run from CI"
(durability plan §Netpol gate). The plan's Phase 0 is explicit: *observe AFTER
the operator netpol apply + next edge deploy + genesis run* — i.e. do not build
ahead of the gate.

**Operator sequence to light the card** (the chain the handoff named):
1. **netpol apply** (operator) → conductor reachable from CI/pods.
2. **reseed** with `RESET_STORAGE=true` (operator pre-authorized this in the
   prior shift, unused) → content re-anchors WITH the conductor (self_cid +
   provenance populate); the household junction (`humans.agent_pub_key +
   household_id`) fills via conductor seeding.
3. provide-loop spawns (self_cid now non-empty) → authors `replicates-content`
   → side-projection writes `content:<reach>` rows → snapshot + feltStatus light.

## Bounded code legs (candidates for the post-netpol session — NOT done here)

- **self_cid derive-at-startup** instead of env-only: derive the node's steward
  CID from its own conductor/agent identity at boot (robust to a missing env),
  so the provide-loop isn't silently disabled by an unset env. (Still needs the
  conductor, so sequence after netpol.)
- **reach-circuit recovery**: the OPEN→provenance-only state is permanent for a
  seed run; a transient conductor unavailability at seed time poisons the whole
  seed. A backoff-retry + re-stamp once the conductor returns would make seeding
  robust to conductor boot-flakiness (main.rs:616 documents 5/5 boot failures).
- **observability**: surface the dormant provide-loop (self_cid empty) as a
  `/p2p/status` / `/health` diagnostic flag instead of a single startup log line,
  so "the card is dark because the loop is off" is visible without log scraping.

**Recommendation:** operator runs netpol apply + RESET_STORAGE reseed; then a
session observes (durability arc Phase 0) and lands the bounded code legs above.
Do NOT attempt to light the card from dev without the conductor — it cannot work.
