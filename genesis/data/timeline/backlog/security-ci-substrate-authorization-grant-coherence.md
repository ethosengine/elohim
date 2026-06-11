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

## Composition note (2026-06-11, network-agency arc session)

The che-network-agency arc (genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md)
landed the LADDER this concern graduates along — Jenkins is the same shape of actor as the Che
agent, one stage behind:

- **Stage A for CI (available now):** a `jenkins-ci` service account authenticates through
  `DoorwaySessionClient` (framework-free `@elohim/identity/core`, proven Node-consumable by a2o;
  auth wire shapes now schema-contract-pinned). Bearer-gating `/admin/seed/*` + the seeder sending
  the session token closes perimeter hole #2 with identity + audit (not yet bounded standing).
- **Brokered conductor surface (closes hole #1):** Holochain admin WS has no auth surface — netpol
  is its ONLY gate, so identity cannot ride it directly. Doorway already holds the conductor pool
  admin connections (typed_admin.rs): a doorway-brokered, authenticated conductor-seeding surface
  lets jenkins-ns stop touching 8444/8445 entirely, the netpol reverts to 8090/8080, and the
  cluster-only ipBlock VXLAN rule (manifest-vs-live drift hazard) retires with it. Same shape as
  Che: drive the native network through doorway's governed surfaces, never raw admin sockets.
- **Stage C (the destination this file already names):** the grant becomes the delegates-compute
  commitment — rides the arc plan Phase 4 rail-readiness checkpoint (Z.D Sprint 1 / slice2a lanes),
  do not build rails here.

## Stage A LANDED 2026-06-11 (hole #2 closed — identity + audit)

The jenkins-seed-bearer-gate plan (genesis/docs/superpowers/plans/2026-06-11-jenkins-seed-bearer-gate-plan.md)
landed perimeter hole #2's Stage A, all tasks two-stage reviewed:
- **doorway** bearer-gates `PUT /admin/seed/blob` + mutating `/admin/cache/{disable,enable,clear,warm}`
  (`require_seed_authority`, dev-mode-safe, named for the Stage-C `seed-content` operator capability) — 396779747.
- **seeder** authenticates (SEED_DOORWAY_* → JWT → bearer; dev-mode no-bearer invariant test-proven) — 1372fbe57.
- **CI** sends the bearer on the genesis pipeline's one gated call (`substrate-verify.sh upload`) via a
  standalone login helper + Jenkinsfile withCredentials; the load-bearing `adminBootstrapKey` snake_case
  bug fixed (jenkins-ci must be Admin) — b99d6a186 + b3e6f91a1.

Live-verification CLAIMED until the next operator-merged genesis build's Upload stage returns 200 with a
bearer. **Zero-touch activation (edcac9800):** the pipeline self-provisions `jenkins-ci` idempotently from
the existing `doorway-admin-bootstrap-key` credential (no new credential, no manual curl) — register-or-confirm
+ login → JWT bearer; self-heals on a MongoDB wipe. Only precondition (already satisfied): that credential
exists and equals alpha's API_KEY_ADMIN. Caveat: rotate ADMIN_KEY ⇒ delete the jenkins-ci account once.

**STILL OPEN:** hole #1 (jenkins → conductor-WS 8444/8445, netpol-gated by network position — the
brokered-conductor-surface task; closing it reverts the netpol + retires the ipBlock VXLAN manifest-drift
hazard) and bounded standing (Stage C delegates-compute, rails-gated). The 8444/8445 netpol revert is
gated on hole #1, NOT licensed by this Stage-A landing.

