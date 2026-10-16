---
id: doorway-operator-surface-under-standing-plan
title: The doorway operator surface opens under standing the network records
status: Active
habits: [operator-dashboard-under-standing]
cites:
  - "elohim-operator-loop-wedged-peer-design | The elohim-operator loop when a peer wedges | sha256:8329c39fe67980a6 | path: genesis/docs/superpowers/specs/2026-10-09-elohim-operator-loop-wedged-peer-design.md"
  - "doorway-stewardship-chain-design | 2026-05-19-doorway-stewardship-chain-design | sha256:f90729e7a9887de8 | path: genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md"
---

# The doorway operator surface opens under standing the network records

Story: `genesis/a2o/features/doorway/operator-standing.feature`. Receipts (Codex legs, file:line):
`genesis/local-dev/doorway-operator-surface/` (census-code, census-design, design, callers, s1).

## Rule

The operator surface (doorway-app `/dashboard` and every `/admin` route behind it) is a view of the
network. It answers only a holder of an active `operate-doorway` commitment scoped to this
doorway, read on every request from the doorway's own peer. A sign-in token proves who is asking,
and nothing more. Neither a Mongo `permission_level` nor a doorway env value grants standing. A
holder is the steward of a facility, never the sovereign of the people it hosts: holders manage
what the doorway hosts, and never which key is a person's.

## What holds today (2026-10-16, source-verified)

- `/dashboard` is gated by sign-in alone (`doorway-app/src/app/core/guards/auth.guard.ts:37`).
- Eight mutating `/admin` routes had no handler-level check. Slice S0 closes them with the interim
  Admin check.
- `auth/operator.rs` resolves capability from a JWT snapshot. It has no production caller, and the
  snapshot is never populated.
- `operate-doorway` commitments are written diesel-direct: `CONDUCTOR_SOFT_ACTIONS` holds only
  `custody-blob` (`elohim-storage/src/db/rea_commitments.rs:1535`). They are rows, not network
  records.
- The seeder binds Matthew `*` on both alpha doorways. Its `provider` is a synthetic hash
  (`genesis/seeder/src/peer-id.ts:28`) with no relation to any agent key a doorway JWT carries.
- Commitment creation accepts any provider string (`content_store/src/lib.rs:16956`). Anchoring
  alone therefore does not make a grant legitimate.

## Who holds what (operator ruling, 2026-10-16)

| Doorway | Primary | Deputy |
|---|---|---|
| household doorway A | Matthew `*` | — |
| alpha-elohim-host (doorway-alpha) | Matthew `*` | — |
| apex-elohim-host (elohim.host) | Adam `*` | Matthew, enumerated |

Adam is a test persona. His standing is held safe by his status in the test net, whose steward of
record is Matthew. "Operators are not absolute sovereigns."

## Issuer rule (born narrow)

A binding counts only when its root author on the DHT is the **steward of record** of the network
the doorway serves (Matthew, for the household and the alpha test net). The steward of record is
read from the network itself, never from doorway config: the elohim DNA's bootstrap steward,
`modifiers.properties.progenitor_pubkey`, which `content_store/src/bootstrap_steward.rs` already
exposes. No `StewardOfRecordAttestation` entry exists in any DNA yet (it appears only in
`2026-05-19-doorway-stewardship-chain-design.md` and a JWT field); the progenitor is its present
stand-in, and a later attestation chain can replace it without changing this rule. It does not count when the
holder authored it, unless the holder is that steward. That one check is what "held safe by the
test-net standing" means in code. It also rules out self-grants. Widening the issuer set (a
doorway's own commons council, the people who ingress through it) is a later governance story and
needs its own habit evidence.

## P2P design gate (before any route)

1. **Category.** The binding is Notarized (A). It reuses the existing REA `Commitment` entry type
   in the elohim DNA (`content_store_integrity/src/lib.rs:3737`, `action = operate-doorway`,
   capabilities in `resource_classified_as`, `doorway:<id>` in `in_scope_of`). Standing and the
   `OperatorStandingView` are Operational (C), reconstructed per request and never stored.
2. **Entry type.** It already exists. There is no integrity change, and the DNA hash does not move.
3. **Head-plane cost.** Four bindings at birth, fewer than 100 after a year (one head per binding
   plus its lifecycle updates). That cost is negligible against quiesce.
4. **Identity.** The binding id stays the seeder's content digest over (provider, action, scope).
   The DHT anchor is the action hash the coordinator returns.
5. **Coordinator and signal.** `content_store::create_rea_commitment` /
   `update_rea_commitment_state` (coordinator only), projected by the existing
   `ReaCommitmentCommitted` signal. The routes below are reads of that projection. They are never
   a source of truth.

## Slices

| Slice | Deliverable | Crate / tree | Verified by |
|---|---|---|---|
| S0 | 8 actuators require Admin (Bearer JWT or admin X-API-Key); 401/403 split; fixtures send X-API-Key | doorway-service, a2o steps | `just gate doorway/doorway-service` + Codex review |
| S1a | `operate-doorway` conductor-required on create and cancel; `find_active_operator_binding` requires anchor, active, provider, window, JSON scope membership, issuer = steward of record; `GET /api/v1/operator-bindings/active?agentPubKey&doorwayId` → view or a 404 that names its reason, uncached | elohim-storage | `just gate elohim-storage` |
| S1b | `OperatorStandingView` schema + codegen; `resolve_operator_standing` per request, no positive cache; route policy table (below), unknown `/admin` route denied; `GET /admin/operator-authority`; S0 Admin check retired for humans, X-API-Key kept only on the listed deploy-maintenance routes | doorway-service, sdk/schemas | `just gate doorway/doorway-service` |
| S1c | dashboard guard on standing; standing banner; refusal page that says why; controls the binding does not name are not shown | doorway-app | `just gate doorway/doorway-app` |
| S1d | seeder: bindings above, provider = the agent key the seeder's own registration/login returned (`seed-humans.ts:411,438`), authored through the steward's storage, anchored readback verified | genesis/seeder | seeder tests + household readback |
| S2 | hosting census (read-only): stuck accounts (key with no installed app on any pool conductor), registry rows for absent conductors; remedy addressed to the owner | doorway-service + app | story scenario 6 |
| S3 | cleanup actuators as capability-gated verbs that name the commitment and record the act | doorway-service | operator-runtime-surface checks |

S1a, S1b and S1d can run concurrently: they touch different crates, with one build slot each. S1c
follows the generated view from S1b.

## Route policy (S1b)

Paths below are relative to `/admin`. Any route not listed is denied until it is classified.

| Routes | Capability |
|---|---|
| GET users, hosted-users, conductors[/…], agents/*/conductor, graduation/*, pipeline | `doorway-observe` |
| GET dashboard/topology, nodes[/…], cluster, resources, custodians; ws | `topology.read` |
| GET federation/peers; routes | `federation.peers.read`; `registry.read` |
| POST/DELETE federation/peers, POST federation/peers/refresh, POST federation/deregister | `federation.peers.mutate` |
| POST conductors/assign, POST hosted-users, DELETE hosted-users/*, user lifecycle mutations | `hosted_users.lifecycle` |
| POST graduation/force/* | `graduation.record` |
| POST steward-peers/refresh, ssr-bundle/refresh | `registry.mutate` |
| GET cache/stats, HEAD seed/blob/*; POST cache/*, PUT seed/blob | `cache.read`; `cache.mutate` |
| PUT users/*/permission | `users.permissions.mutate` |
| dev/*, users/*/steward | existing fixture gates unchanged |
| GET self-healing, bootstrap-coherence, render-stats, capability, capabilities | public peer telemetry |

Matthew's deputy list on apex is every capability above except `users.permissions.mutate`.

## Honest limits

- Revocation takes effect on the first request after the doorway's peer has received it. The
  design does not claim instant revocation across the whole network.
- A doorway registration's `operator_agent` is its conductor's author
  (`infrastructure/src/lib.rs:404`), not necessarily the holder.
- Matthew's alpha account points at a dead custodial key, which makes it a stuck account. His
  standing on alpha cannot read green until his own rebind lands (`feat/graduate-onto-node-key`,
  another session).
