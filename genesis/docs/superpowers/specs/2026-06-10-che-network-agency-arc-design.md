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
  - elohim-sdk-architecture | elohim-sdk | sha256:7d1a9b09f3c6592d | path: genesis/docs/architecture/elohim-sdk.md
  - rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
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
- [x] Operator grants a scoped permission rule for fixture-auth flows against alpha
- [x] Verify `look --as Matthew` end-to-end (closes the L3 open item; proves the write context)
- [x] Document the authorized-write etiquette in `genesis/a2o/CLAUDE.md` (what dev writes on
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
- [x] Spike: fetch deployed DNA artifacts; conductor network config → alpha bootstrap/signal;
      prove join (peer visible in gossip / agent-info at the doorway) — **PROVEN 2026-08-28T03:27Z** from a Che pod: `NETWORK_PROFILE=join-alpha hc-start.sh --conductor` (deployed bundle from the Jenkins artifact, `hc sandbox generate … network --bootstrap https://doorway-alpha.elohim.host/bootstrap webrtc wss://signal.alpha.elohim.host`); all 5 local cell DNA hashes ∈ alpha's diagnostics spaces (no partition); conductor held a peer URL via the signal relay at boot; doorway-alpha `/db/p2p/conductor-diagnostics` listed the workspace agent on all 5 spaces within ~4 min (agentCount 20→25). One rail learned: the tx5 signal URL must be PATHLESS (`/signal` on the doorway panics the conductor at boot); the fleet's `wss://signal.alpha.elohim.host` is the value. Story: `genesis/a2o/features/deployment/sovereign-peer-join.feature` (@wip).
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

## Dual-plane verification discipline (what the perspectives are FOR)

Stage A *is* the doorway perspective, developed by inhabiting it (every act crosses the web2
projection: sessions, reach-gating, projection caches). Stage B *is* the peer-native perspective
(the network as something you ARE: gossip, source chain, validation). Stage C is the seam where
both must converge on ONE authority model — a grant is a single DHT entry, but enforcement holds
in two planes:

- **Doorway plane**: the bounds-gate checks the commitment (scope, TTL, revocation) before
  accepting a delegate's HTTP write — the projection enforcing substrate truth.
- **Peer plane**: validation on the delegate's `EconomicEvent` verifies the same commitment by
  link traversal — peers enforcing it with zero doorway present (the D8 "doorway is optional"
  rail made executable).

**Discipline: every Stage-C gap-item lands with PAIRED verification** — one doorway-plane a2o
scenario (API/browser through alpha) and one peer-native scenario (sweettest tier or the Che
peer directly). Any verdict delta between the planes is a truth-layer bug, surfaced by design.

## Client connection matrix (agency-phase gotchas — DO NOT step on these)

How a client UI connects is inherently more complex than a simple server-client webapp (operator
directive, 2026-06-10: be explicit so agency work never tramples a client configuration). The
strategy layer lives in `app/elohim-library/projects/elohim-service/src/connection/` (factory
detection: `__TAURI__` global → tauri; else doorway/direct by config):

| Client context | Strategy | Control plane | Data/blob | Gotchas the arc must respect |
|---|---|---|---|---|
| Browser, Che/local dev | `DoorwayConnectionStrategy` + same-origin (`window.location.origin`, Che strips CORS) | Holochain Admin/App **WebSockets** proxied via doorway :8888 | `/blob/{hash}` via dev proxy | WS upgrade through Che traefik needs the devfile endpoint `secure: true`; **browser holds zome-call signing keypairs** (`generateSigningKeyPair`/`setSigningCredentials`) — clients are not HTTP-only |
| Browser, production | `DoorwayConnectionStrategy` direct `https://doorway…` | same (wss) | doorway `/blob` | `trustMode` (`doorway-host` \| `peer-conductor`) is **discovered from `/auth/me`, never config** — pinning it reintroduces the two-parallel-auth-systems trap |
| Browser, L3 `live-data` profile | same-origin → dev proxy → **deployed alpha** | HTTP verified; **WS/`/p2p` through this proxy UNVERIFIED** | alpha `/blob` | the live-data proxy covers the HTTP contexts only — don't assume websocket parity until proven |
| Tauri desktop | `TauriConnectionStrategy` (session mgmt) / `DirectConnectionStrategy` | direct ws to **embedded conductor**; bootstrap/signal arrive via doorway **`/auth/native-handoff`** (fed by doorway `BOOTSTRAP_URL`/`SIGNAL_URL` env, `config.rs:26-31`) | **elohim-storage sidecar `:8090` direct — doorway NOT in the path** | Stage B touches conductor network config; it must NOT repurpose the native-handoff channel — that is the Tauri client's bootstrap path |
| Node (a2o, seeder, agent tooling) | `DirectConnectionStrategy` (Node) or plain HTTP | HTTP/ws to doorway or storage | direct | global fetch (Node 22 = undici); no localStorage exists — session storage must be injectable |

**Arc rails derived from the matrix:** Stage A's `DoorwaySessionClient` stays transport-dumb —
`baseUrl`/`fetchImpl`/`tokenStore` injected, NO strategy detection, NO trustMode logic, NO
localStorage assumption (Tauri/Node differ). Stage B changes the *conductor's* network config
only, never the client strategy layer, and leaves `/auth/native-handoff` semantics untouched.
Stage C's `CommitmentService` must work against both doorway-proxied and sidecar-direct (`:8090`)
base URLs. Any auth change must remember browser clients sign zome calls with client-held keys —
the session token is not the only credential in play.

## Developer-surface coherence (startup scripts speak the stages)

Survey (2026-06-10): `hc-start.sh` is canonical, but full-stack startup logic exists in three
places (`hc-start.sh` / a2o `local-stack.ts` / devfile `start-doorway`), DNA build in four, two
seeders are invoked differently (`seed.ts` vs `seed-sqlite.ts`), and the local conductor
(`hc-start.sh:196`, `hc sandbox generate`) has **no bootstrap/signal hook** — today's dev
conductor is always an isolated island. The stages give these scripts their missing vocabulary —
**named network profiles** instead of accreted flags:

| Profile | Meaning | Today |
|---|---|---|
| `isolated` | full local stack, island DHT (current `hc:start`) | exists (default) |
| `live-data` | local UI × alpha data via dev proxy (L3 `start:alpha`) | exists |
| `join-alpha` | local conductor joins the alpha DHT as a Stage-B peer | missing — the gap |

- [ ] Thread `CONDUCTOR_BOOTSTRAP_URL` / `CONDUCTOR_SIGNAL_URL` (or a generated conductor-config)
      through `hc-start.sh` so `join-alpha` is a profile, not a fork; document the three profiles
      in one place the agent reads (`app/elohim-app/CLAUDE.md` Starting Development)
- [ ] `join-alpha` sources the **deployed** `.dna` bundles (parity rail) — add a fetch path for
      CI-built artifacts; local builds remain `isolated`-profile-only
- [ ] De-duplicate one seam: a2o `local-stack.ts` consumes `hc-start.sh`'s surface (single
      health-check + seeder path) instead of re-implementing it

## SDK complement (the arc as the SDK's first consumer)

The SDK canon (`elohim-sdk.md` §3–4) already names the homes this work fills; the arc's stages
are the forcing functions, and the agent's tooling becomes the SDK's **first consumer** instead
of a fourth hand-rolled duplicate:

- **Stage A → `@elohim/identity` `DoorwaySessionClient`.** Auth/session logic is hand-rolled
  today in at least three places (`app` `auth.service.ts`, a2o `doorway-client.ts` +
  `browser-device.ts`, doorway-app `auth-state.service.ts`) around one wire shape. Stage A's
  authorized write path lands as the consolidated client (login / register / logout / me /
  exchange / restore + one `AuthResponse` model); a2o and `look --as` migrate onto it first.
- **Stage C → `@elohim/rea-runtime` `CommitmentService`.** Canon §3.5 explicitly scopes Z.D
  `delegates-compute` here. Wire types and `/api/v1/commitments` routes exist; the gap is the
  service layer: create/accept/query/revoke + a `delegates-compute` builder + state-machine
  guards, all keyed on **CID = `entry_hash`**. The arc's grant-mint-exercise-revoke loop is the
  service's first real exercise.
- **Stage B** contributes the startup profile + artifact sourcing only — a peer-conductor client
  surface in `@elohim/service` is deferred to peer-topology work (out of arc).

- [ ] `DoorwaySessionClient` in `@elohim/identity`; a2o framework + `look --as` consume it
      (delete the duplicated auth walking in `doorway-client.ts`/`browser-device.ts`)
- [ ] `CommitmentService` in `@elohim/rea-runtime` with the `delegates-compute` builder and
      transition guards; Stage C's grant loop runs through it end-to-end

## Out of scope

Steward key-bundle handoff into Che (custody canon); the recovery web-session implementation
(Pattern Recovery ships it); multi-agent fleets under one grant; any new DHT entry type; a
peer-conductor/topology client in `@elohim/service` (deferred with Stage B's hosting work).
