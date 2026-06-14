---
id: "backlog-resilience-card-self-cid-provide-loop-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR resilience card zeros — APP-LAYER (in-pod conductor cell-readiness + SELF_CID unset), NOT netpol; reseed is not the lever"
slug: "resilience-card-self-cid-provide-loop-gate"
written: "2026-06-14"
author: "agentic-developer (felt-status shift, operator OPEN #1 diagnosis — corrected after operator cluster-read + runtime-triage)"
status: "backlog"
priority: "high"
ci_status: blocked
tags: [resilience, dht-anchor, self-cid, provide-loop, conductor, cell-readiness, durability-arc, app-layer]
cites:
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/config.rs
  - elohim/elohim-storage/src/services/provide_reconcile.rs
  - elohim/elohim-storage/src/services/conductor_commitment_author.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - genesis/manifests/humans/matthew-manager.yaml
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
relatedNodeIds:
  - backlog-resilience-tier-content-declared-floor
  - backlog-self-heal-doorway-startup-conductor-mint-serialization
---

# EPR resilience card zeros — the self_cid provide-loop gate + the conductor boot-race

Diagnosis of the sprinter handoff's OPEN #1 (`SPRINTER-HANDOFF-2026-06-14.md`),
done during the felt-status shift (the felt surface this lights: `feltStatus` on
ResilienceSnapshotView, commit 6a754f30f). Workstream D of the EPR Content
Durability Arc plan.

> **CORRECTION (2026-06-14):** an earlier draft framed this as "the durability
> arc's netpol gate → conductor reachable; operator runs netpol apply + reseed."
> **That was wrong** — propagated from the durability plan + handoff without
> independent verification. Operator cluster-read + the runtime-triage kill-log
> proved the cause is **app-layer, not netpol.** Corrected below.

## The chain (code-verified)

The card zeros (stewardingCollectives 0, commitmentBacked 0, no `content:<reach>`
provide rows) are gated, in order:

1. **`self_cid` is empty → the provide-loop never spawns.** `main.rs:959-966`
   spawns the P1 reconciliation WRITE half (the `replicates-content` authoring
   tick → writes the `content:<reach>` provide rows the snapshot reads) ONLY when
   `config.self_cid` is `Some` and non-empty. `self_cid` is sourced solely from
   the `SELF_CID` env (`main.rs:366-370`), which is **set in NO manifest** (the
   `elohim-storage` container env in `genesis/manifests/humans/*.yaml` has no
   `SELF_CID`). → permanently dormant; logs once: *"Slice-2b provide-loop
   authoring tick disabled: requires lamad HcClient + db pool + non-empty
   self_cid."*
2. **Even spawned, it authors via the in-pod conductor.**
   `provide_reconcile::reconcile_provides` → `conductor_commitment_author` Step 1
   notarizes via the lamad HcClient → the in-pod conductor.
3. **Content anchoring at import also needs the in-pod conductor.** genesis #1145
   seeder hit `reach circuit OPEN after 5 consecutive conductor-path failures →
   provenance-only stamping → rows stamp-failed` (cf. main.rs:616: failures happen
   "while the conductor's cells are still CellDisabled").

## The cause is APP-LAYER, NOT netpol (verified 2026-06-14)

- **elohim-storage → conductor is IN-POD over `ws://localhost:4444`**
  (`HOLOCHAIN_ADMIN_URL` in the human manifest; the `edgenode` conductor and
  `elohim-storage` are containers in the SAME pod). Not cross-pod, not
  cross-namespace → **no NetworkPolicy gates it.** The 5/5 failures are the in-pod
  conductor's cells being **CellDisabled during the boot/seed window** — app-layer
  readiness, not a blocked path.
- There is **no K8s `Service` named `conductor`**; cross-pod conductor admin is the
  per-pod `ws-proxy` socat on `:8444`. (That `:8444` path, with per-human URLs
  DNS-unresolvable for undeployed humans, is item 2 —
  `self-heal-doorway-startup-conductor-mint-serialization` — also app-layer.)
- `genesis/orchestrator/manifests/network-policies.yaml` (jenkins→conductor :8444,
  for CI *seeding* stages) is a DIFFERENT path and does not gate this card.

## CONFIRMED PRIMARY root causes — the READ joins are broken (code-review + schema verification, 2026-06-14)

The self_cid gate (below) is real but is NOT the biggest blocker. A code-review +
direct schema verification found the snapshot's READ joins are broken at the
identity level — the card stays dark **regardless of seeding, self_cid, or
conductor.** These are the priority fix (all in `household_resilience.rs`):

- **R1 — steward join namespace mismatch [CONFIRMED].** `snapshot()`/`compute()`
  join `shard_locations.peer_id == humans.agent_pub_key`. `shard_locations.peer_id`
  is a **libp2p peer id**; `humans.agent_pub_key` is a **holochain agent key** —
  different namespaces. The `peer_identity_bindings` table exists precisely as the
  bridge (`peer_id ↔ agent_cid ↔ dht_anchor_hash`) but the join doesn't use it.
  → **zero stewardingCollectives for every content** (the primary dark-card cause).
- **R2 — commitment-join action mismatch [CONFIRMED].** `snapshot()` filters
  `rea_commitments.action.eq("provide")`, but `mishpat_projection.rs:111` writes
  provide commitments as `action: "replicates-commons"` / `"replicates-content"`.
  `"provide"` never matches. → **zero commitmentBackedCollectives.** (`rea_commitments`
  DOES have a `state` column — that filter is fine.)
- **R3 — provider identity [CONFIRMED same class as R1].** The commitment join is
  `rea_commitments.provider == humans.agent_pub_key`; `provider` carries the peer
  id / self_cid (durability plan: seeded provider = pod `/p2p/status .peerId`).
  Same peer-id-vs-agent-key mismatch.
- **R4 — content_reach default [BLOCKER-4].** `snapshot()` defaults `content_reach`
  to `"commons"` on lookup error → wrong `content:<reach>` scope for non-commons
  content → commitment join misses.
- **R5 — commitment_backed_replication stub [BLOCKER-2].** `compute()` always
  returns `CommitmentBackedReplication::default()`; the computed count isn't
  threaded into the view.

**THE IDENTITY CONTRACT (must be settled once, truth-layer call):** the steward +
commitment joins both hinge on `humans.agent_pub_key`. Either (A) route both joins
through `peer_identity_bindings` (peer_id → agent_cid → humans), or (B) make all
three populators (`humans.agent_pub_key`, `shard_locations.peer_id`,
`rea_commitments.provider`) AND `self_cid` carry the SAME identity (the libp2p
peer id). Whichever — the seeder peer-id contract + the storage self_cid-derive
must produce the matching value, or the card stays dark and per-unit tests still
pass (the silent-empty trap).

**PROOF GATE:** a deterministic unit test that seeds coherent substrate rows and
asserts `snapshot()` returns `measured` + non-zero stewards + non-zero
commitment-backed + named collectives. It must FAIL today and pass after the join
fixes — the local "card lights" proof, no live cluster needed.

## The self_cid / conductor causes (real, but secondary to the join fixes above)

1. **`SELF_CID` config gap → provide-loop dormant** [code-verified]. No
   startup-derive; only the unset env. **Fix:** derive `self_cid` at startup from
   the in-pod conductor/agent identity (or inject it), so the loop isn't silently
   off.
2. **Reach-circuit boot-race → provenance-only** [corroborated by the item-2
   kill-log class]. The seed/import hits the in-pod conductor before its cells
   enable; the circuit OPENs after 5 and latches provenance-only for the whole
   run. **Fix:** gate the seed on conductor cell-readiness, AND/OR make the reach
   circuit recover (backoff-retry + re-stamp once cells enable) instead of
   latching.
3. (observability) surface the dormant provide-loop (`self_cid` empty) + the
   latched reach-circuit as `/p2p/status` flags, so "the card is dark because the
   loop is off / the circuit latched" is visible without log scraping.

## Confirm BEFORE any reseed (operator's caution — correct)

A `RESET_STORAGE` reseed re-runs the SAME path: it will re-hit the cold-conductor
race and re-skip the dormant provide-loop, **lighting nothing**, unless (1)
`self_cid` is populated and (2) the conductor cells are ready when the seed runs.
So the levers are the two elohim-storage fixes above (bounded, local,
dev-implementable), NOT a netpol apply and NOT a bare reseed.

**Recommended sequence:** (1) confirm the in-pod `edgenode` conductor is healthy
(cells enabled) on a live alpha pod — `/p2p/status`, the `:4444` liveness already
in the manifest; (2) land self_cid-derive-at-startup + reach-circuit-recovery in
elohim-storage; (3) THEN a reseed populates real counts and the felt card lights.
The durability arc plan's Phase 0 ("observe before building") still holds — but
the thing to observe is **conductor cell-readiness**, not a netpol.
