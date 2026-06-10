---
id: "backlog-security-ci-substrate-authorization-grant-coherence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "CI compute authorization is network-position, not grant-based — one consistent delegates-compute approach across conductor-WS and doorway admin surfaces"
slug: "security-ci-substrate-authorization-grant-coherence"
written: "2026-06-10"
author: "claude (genesis-pipeline stabilization session, operator-directed)"
status: "open"
priority: "high"
ci_status: none
jobs: [elohim-genesis]
tags: [security, authorization, delegates-compute, rea, doorway, conductor, netpol, bootstrap-debt, p2p-design-gate]
cites:
  - genesis/orchestrator/manifests/network-policies.yaml
  - genesis/data/timeline/backlog/ci-genesis-conductor-adminws-unreachable.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - doorway/doorway-service/src/routes/seed.rs
---

# CI substrate authorization: one grant pattern, not two perimeter holes

## The class (operator framing, 2026-06-10)

Two surfaces let CI compute act on the alpha substrate by **network position**
alone — no identity, no standing, no revocation, no audit event:

1. **jenkins-ns → conductor admin/app WS** (pod ports 8444/8445, opened
   2026-06-10 in `network-policies.yaml` so the three conductor-seeding stages
   can run at all). Holochain's admin WS is root-on-conductor for anyone who
   can reach the socket: generate agent keys, install apps.
2. **doorway `/admin/seed/*` (+ `/admin/cache/*`) routes** — the genesis
   pipeline's blob upload (`PUT /admin/seed/blob`) carries no credential; the
   route trusts the internal network path.

The operator's verdict: *"similarly shaped problems that should have a
consistent authorization approach."* Neither gets its own bespoke fix; both
graduate together onto the one primitive the protocol already names for this —
**`Mishpat::Commitment` with the `delegates-compute` action** (bounded
reciprocity, on-chain standing, revocation, audit trail — the same primitive
that displaces X-API-Key admin grants; spec
`2026-05-25-stagespablob-substrate-correct-deploy.md` §1). This also aligns
CI with the eclipse-che developer-auth arc (`0026de6b1` collapsed a2o's
hand-rolled auth onto `@elohim/identity` `DoorwaySessionClient`) — dev
compute and CI compute authenticate the same way.

## Why the netpol open is acceptable NOW (scoped bootstrap debt)

The conductor-seeding stages **create** the agent keys and identities the
grant system needs to exist — genesis circularity, like root kubeconfig
before RBAC has users. The open is scoped (alpha namespace × jenkins
namespace × two pod ports), explicit (rule comment names this entry's
concern), and reversible. The debt is named here so it cannot silently
normalize.

## Target end-state (retirement condition)

- Doorway grows a **conductor-admin deputy surface**: the few admin ops
  seeding needs (agent key gen, app install, zome-call session mint),
  executed by doorway over its existing intra-namespace conductor
  connections, gated by verification of a `delegates-compute` commitment
  presented by the caller.
- CI holds that commitment as an identity (issued by the operator; bounded
  scope: seed ops on alpha; revocable; every exercised op emits an audit
  event into the REA projection).
- The same commitment check gates `/admin/seed/*` and `/admin/cache/*`.
- The genesis seeder authenticates via `@elohim/identity` (the
  DoorwaySessionClient path) instead of raw `AdminWebsocket` connections.
- **Then re-close 8444/8445** in `network-policies.yaml` (operator apply) —
  only doorway needs conductor reach again.

## Design gate

This is a data-entity-bearing design (commitment kinds, audit events, doorway
routes) — run the `p2p-design-gate` skill before proposing the design:
DHT entry types first, the commitment already exists as a Mishpat primitive
(check kind headroom), identity is agent-composite, coordinator + signal
before HTTP route shape.

## Doneness

- [ ] Design doc through p2p-design-gate (doorway deputy + grant issuance for CI)
- [ ] Doorway conductor-admin deputy routes, commitment-gated
- [ ] Seeder migrated off raw AdminWebsocket
- [ ] `/admin/seed/*` gated by the same check
- [ ] 8444/8445 removed from the jenkins netpol rule + operator re-apply
