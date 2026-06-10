---
id: che-network-agency-arc-design
title: Che Network-Agency Arc — hosted-session, sovereign-peer, and delegated agency for the agentic developer
status: Draft
class: process-meta
topic: [che, agency, doorway, peer, conductor, delegation, commitment, reach, write-context, agentic-developer]
process_subdomain: agents
cites:
  - doorway-access-tier-patterns | the canonical agency-state catalog this arc maps onto — Stage A = its Tier-2 hosted user; its Tier-3 recovery proxy is the sibling story Stage C must NOT be confused with | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - che-live-peer-dev-loop-design | the eyes this arc adds hands to — L3 of the browser-feedback series; its read-mostly rail is what Stages A-C graduate beyond | sha256:f976477c2f2baba0 | path: genesis/docs/superpowers/specs/2026-06-10-che-live-peer-dev-loop-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-10-che-live-peer-dev-loop-design.md
derived_from:
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
---

# Che Network-Agency Arc — three agency states for the agentic developer

> Sequel to the Che Browser Feedback series (L1 `look` → L2 visual gate → L3 live-peer dev loop).
> That series gave the agent **eyes** (read-mostly). This arc gives the agent **hands** — network
> agency with reach and write contexts — in three stages that map onto the protocol's canonical
> agency states, so the dev tooling *teaches the protocol's own identity model along the way*.

## Vocabulary: how this arc maps to the access-tier patterns (disambiguation)

The doorway access-tier catalog (cited) numbers *access tiers*; an earlier draft of this arc
numbered its stages "tier 1/2/3", colliding with that vocabulary. **This arc names stages A/B/C**
and pins the mapping:

| Arc stage | Access-tier doc state | Who acts | Axis |
|---|---|---|---|
| **A — Hosted-session agency** | Tier 2 *doorway-hosted user* (cell in doorway's pool) | Matthew (fixture persona), via session | custody: doorway holds the cell |
| **B — Sovereign-peer agency** | post-graduation *peer-steward* | a NEW agent on its own conductor | custody: workspace holds its own cell |
| **C — Delegated agency** | (none — different axis) | the agent **as itself**, within Matthew's grant | authority: bounded delegation |
| *(not in arc)* | Tier 3 *hosted steward via web* / Pattern Recovery | the steward themself, device offline | transport: doorway proxies, zero authority |

**Stage C ≠ access-tier Tier 3.** The recovery story (lost device → doorway proxies requests
signed with the steward's recovery-verified key → full reach, one actor) is a *transport/custody*
answer for the human principal. Stage C is an *authority* answer for a second agent: two actors,
scoped reach, provenance shows the delegate. They are **sibling instances of the same substrate
primitive** — the REA compute-commitment generalization table includes *hosting* and *recovery
quorum* alongside *delegation* — but they are different user stories. They compose: a recovery
web-session is one legitimate way Matthew could *mint* a Stage-C grant.

## Stage A — Hosted-session agency (exists today; gate on explicit authorization)

Fixture Matthew (`matthew.dowell@alpha…`, admin standing) is seeded on alpha; the a2o framework
logs in as him; `look --as Matthew` is built. A doorway session gives read+write with Matthew's
authz and reach through the HTTP API. Agent key stays in the doorway pool — nothing secret in Che.

- Rails: **explicit operator authorization per the permission layer** (a live denial on 2026-06-10
  confirmed the classifier enforces this — authenticated action on shared alpha is never inferred
  from a question). Acts mint *as Matthew* — impersonation-shaped provenance; acceptable for dev
  on alpha, never the destination pattern. Alpha also has `registrationOpen: true` — an agent may
  instead register its own account for honest-attribution writes without Matthew's reach.
- [ ] Operator grants a scoped permission rule for fixture-auth flows against alpha
- [ ] Verify `look --as Matthew` end-to-end (closes the L3 open item; proves the write context)
- [ ] Document the authorized-write etiquette in `genesis/a2o/CLAUDE.md` (what dev writes on
      shared alpha are acceptable: test-persona content, never bulk seeding/destructive flows)

## Stage B — Sovereign-peer agency (spike-ready; the workspace becomes a real peer)

Doorway is the network's bootstrap+signal server and both surfaces are publicly exposed on alpha
(verified 2026-06-10: `GET /bootstrap` → 200; `/signal/*` routes live). Che already runs a full
conductor (`hc:start`). Stage B points a Che conductor at alpha's bootstrap/signal so it joins the
real DHT as a **new agent** — gossiping, hosting, witnessing: a true participant, exercising the
hub-optional floor (any device is a full participant).

- Rails: **DNA hash parity** — install the *deployed* `.dna` artifacts, never a local rebuild
  (different hash → different DHT → silent partition; the alpha genesis-pair gotcha). Conductor
  state on the persistent PVC (key continuity across workspace restarts). WebRTC egress through
  the signal relay needs one empirical pass from the pod.
- **Custody boundary**: Stage B's peer is a NEW agent. Making the Che conductor *Matthew's device*
  (steward key-bundle handoff) imports real key custody into a cloud workspace — explicitly OUT of
  this arc; that path belongs to the recovery/device-pairing canon.
- [ ] Spike: fetch deployed DNA artifacts; conductor network config → alpha bootstrap/signal;
      prove join (peer visible in gossip / agent-info at the doorway)
- [ ] Prove agency: the Che peer authors one DHT entry as itself and a household peer reads it
- [ ] Document workspace-peer lifecycle (PVC key continuity, teardown etiquette, one-peer-per-workspace)

## Stage C — Delegated agency (the destination; dogfoods the gospel primitive)

The agent acts **as itself** under a `Mishpat::Commitment` with action `delegates-compute`
(stagespablob §1 shape): provider = Matthew, recipient = the agent's key (Stage A account or
Stage B peer), scope = event classes (first instance: *content authorship* — already on the
primitive's generalization table), bounds, reciprocity, TTL, revocation. Reach is borrowed through
the bounds; provenance stays honest ("the agent did this, under this grant"); revocation never
touches Matthew's credentials. This displaces the credential-sharing shape of Stage A.

P2P design gate (passed 2026-06-10): **zero new entry types** — reuse `Commitment` (Mishpat
11/~100) + `EconomicEvent` + (optional provenance marker) existing `Attestation`. Addressing:
**commitment CID = `entry_hash`** (`action_hash` is only `dht_anchor_hash`; an action_hash-as-CID
silently breaks every bounds-gate). Known dependency: storage does not yet subscribe
`CommitmentCommitted` (the 2a gap) — fresh grants need `ConductorCommitmentFetcher` until it lands.

- [ ] Define the first delegation instance: scope vocabulary for agent content-authorship
      (event classes + bounds fields), composed from stagespablob §1 — no new entry types
- [ ] Mint + exercise one real grant on alpha: Matthew → agent, agent authors content within
      bounds, audit trail readable; revoke and prove the bounds-gate closes
- [ ] Decide the provenance marker question: is an `Attestation` ("agent-operated account")
      required for honest attribution, or does performer-key visibility suffice?
- [ ] Retire Stage-A credentialed writes for routine agent work once C is exercised
      (A remains a test-fixture path, not an agency path)

## Sequencing & why the arc order matters

A → B → C is a deliberate gradient of *understanding*, not just capability: A teaches the
session/authz surface (web2 projection), B teaches what a peer IS (DHT join, gossip, custody),
C teaches the protocol's authority model (bounded reciprocity). Each stage's rails are the next
stage's vocabulary. The arc ends where the gospel memory points: reciprocal REA compute
agreements exercised end-to-end by the first non-human agent on the network.

## Out of scope

Steward key-bundle handoff into Che (custody canon); the recovery web-session implementation
(Pattern Recovery ships it); multi-agent fleets under one grant; any new DHT entry type.
