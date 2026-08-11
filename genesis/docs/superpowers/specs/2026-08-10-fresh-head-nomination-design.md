---
title: "Fresh-head nomination after ghost-declaration decay"
id: fresh-head-nomination-after-ghost-declaration-decay
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: operator review accepts the C1 evidence contract and an implementation proves the registered seam contracts
created: 2026-08-10
domain: D2
topic: [dataplane, canonical-head, ghost-decay, candidacy, anti-self-election]
cites:
  - genesis/data/timeline/backlog/2026-08-10-post-decay-adjudication-cascade-trace.md
  - genesis/data/timeline/backlog/2026-08-10-fresh-head-nomination-and-declare-error-backoff.md
  - substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
---

# Fresh-head nomination after ghost-declaration decay

**Decision requested — no implementation is authorized by this note.** After a ghost-decay author creates a fresh root action, may storage nominate that exact action through the existing canonical-head DHT arbiter? Proposed answer: **yes, but only as evidence-gated candidacy and only after operator review of the C1 contract below.** It must never stamp or crown the SQL projection directly.

## P2P design gate

This is an **A2 relationship**, not a new entity: one existing `canonical_head` link from the existing Content identity to the newly authored Holochain `ActionHash`. Content identity and version identity remain content-derived; no UUID, table, HTTP route, entry type, or signal is added. The coordinator remains `content_store::declare_canonical_content_head`; SQLite remains a reconstructable projection of the DHT result. The target must first resolve from this node's own conductor as the exact action returned by the decay-author call.

The change would add at most one candidacy effect per eligible `(content_id, fresh_action_hash)` and no recurring head row. The candidate count must be measured before implementation; **more than 500 affected ids requires explicit operator sign-off.** Arbitration is per content id, so bundling candidates would change the trust boundary rather than reduce it. Every deployment profile is network-stakes: Simulacra exercises the same DHT rule, Bootstrap and Coordinated add replication pressure, and Enforced cannot weaken the evidence or anti-self-election checks. Only pacing/fanout may be stage-priced.

## Proposed eligibility contract

A fresh action may be nominated only when one reconciliation attempt proves all of these facts together:

1. the pre-existing declaration is decay-proven phantom under the existing positive ghost predicate (local absence, peer-live hint, standing evidence absence, and no election); timeout, unreachable, or fetch miss is never absence;
2. the author call succeeds and returns the exact fresh `ActionHash`, and this node's conductor resolves that action as a locally authored record for the same content id;
3. the action enters only the existing DHT candidacy/arbiter path; it receives no SQL declaration, canonical stamp, or earned tier before the arbiter selects it;
4. the effect is idempotent on `(content_id, fresh_action_hash)`, attempts once in the bounded sweep, and failures enter finite per-id backoff; and
5. success and every refusal/backoff arm use typed reasons. A future implementation must register its eligibility predicate and boundary type in `seam-registry.yaml` with contracts before code review.

Rejected alternatives: doing nothing preserves the proven phantom-candidate loop; nominating every fresh author violates C1; overwriting the SQL declaration violates C1/C2; and treating the fresh action as already canonical bypasses the DHT arbiter.

## Concern disposition

| Concern | Status | Design answer |
|---|---|---|
| C0 plane location | answered | Existing canonical-head/version A2 link; DHT is truth, SQL is projection. |
| C1 anti-self-election | review-required | Positive decay evidence licenses candidacy only; the DHT arbiter alone may crown. Evidence checks are identical in every deployment stage. |
| C2 monotonic authority | answered | No direct stamp or tier widening; existing election ordering remains authoritative. |
| C3 liveness | partial | Supplies the missing fresh candidate and uses finite backoff, but graduation needs a convergence contract and soak evidence. |
| C4 honest absence | answered | Requires the existing positive local/peer/standing-absence proof; transport failure is excluded. |
| C5 evidence-not-authority | answered | Peer hints only trigger observation; the acting node resolves its own returned `ActionHash`. |
| C6a bounded work | answered | One attempt per eligible id inside existing fanout, slice, and wall-clock budgets; no same-sweep retry ladder. |
| C6b idempotent effect | answered | Candidate claim is keyed by `(content_id, fresh_action_hash)` and replay mints nothing. |
| C7 advertise/serve symmetry | answered | No new advertisement; a target must resolve locally before it can be nominated. |
| C8 observability per decision | partial | Requires additive typed success, refusal, and backoff reasons plus ratio-readable counters; exact labels belong in the reviewed implementation contract. |
| C9 identity lineage | not-applicable | No agent identity mapping changes; the authored action keeps its native provenance. |
| C10 contract evolution | answered | No wire/schema/entry change; any additive reason label is closed-vocabulary and contract-tested. |
| C11 external backpressure | answered | Existing sweep admission and fanout remain authoritative; nomination adds one conductor call and never queues unbounded work. |
| C12 may-act | answered | No external caller gains authority; eligibility is verified at the acting node from notarized/local evidence. |
| C13 graduated authority | answered | The fresh action enters staging candidacy only; existing earned-head/graduation rules remain its successor gate. |
| C14 witnessed residual | partial | Unknown failures must emit the existing context-rich residual path; graduation requires a soak showing the remaining phantom-candidate residual. |

**Review gate:** the operator must accept the C1 eligibility contract and measured cohort size before implementation. Acceptance authorizes a separate implementation pass, not a relaxation of any evidence floor.
