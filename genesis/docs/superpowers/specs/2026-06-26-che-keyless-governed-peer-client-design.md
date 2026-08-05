---
title: "Web-tool as a keyless, governed dogfooding peer-client — authority flowing from DHT-steward meaning through doorway-custody to Che"
id: che-keyless-governed-peer-client-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-impl
domain: D8
topic: [doorway, custody, attestation, delegates-compute, capability, peer-client, dogfooding, eclipse-che, web2-constrained, authority-flow, onboarding, identity, T4-projection]
refines:
  - genesis/docs/superpowers/specs/2026-06-23-runtime-orchestration-developer-mode-bridge-design.md
  - genesis/docs/superpowers/specs/2026-06-03-admin-key-lifecycle-dev-to-production.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - genesis/docs/architecture/stewardship-over-sovereignty.md
cites:
  - rea-compute-commitment-primitive | the delegates-compute primitive this composes — a fourth costume: a web-tool's right to operate a hosted conductor | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - admin-key-lifecycle-dev-to-production | stage-3 commitment-backed delegation is the compose-home — the seeder graduates from the omnipotent admin key to a scoped delegates-compute grant by the same mechanism | sha256:44dc9b49dec9d439 | path: genesis/docs/superpowers/specs/2026-06-03-admin-key-lifecycle-dev-to-production.md
  - attestation-consolidation-design | the attestation:* subtype substrate the device-client-authorization credential rides (zero new entry type); key-stewardship is the device precedent | sha256:220c0a2a68c2a805 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - stewardship-over-sovereignty | the identity-ontology floor and the authority-flow root — custody is community-grounded stewardship, never self-sovereign ownership | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - runtime-orchestration-developer-mode-bridge-design | plane-2 (developer-mode bridge) is the compose-home for the capability-scoped orchestration surface | sha256:2060a44617e22bf0 | path: genesis/docs/superpowers/specs/2026-06-23-runtime-orchestration-developer-mode-bridge-design.md
  - resiliency-card-p2p-weave-sprint-plan | Wave 1.3 — the driving loop whose dev-session blocker this design dissolves | sha256:834716e333f5b01f | path: genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - live-distribute-shards-household-observation-plan | the Wave-1.3 plan; its can't-drive-the-live-mesh-from-the-dev-session blocker is resolved by making Che a keyless peer-client | sha256:1cc01de165e2e0ef | path: genesis/docs/superpowers/plans/2026-06-26-live-distribute-shards-household-observation-plan.md
  - dht-is-a-notary-not-a-byte-store | the binding constraint — credentials/commitments are notarized proofs; projections are reconstructable Operational-C | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs
  - elohim/sdk/domains/imagodei/manifest.json
  - elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json
  - elohim/elohim-storage/src/services/bounds_validator.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/server/http.rs
  - genesis/seeder/src/seed-commitments.ts
# Mixed-env design (CLAUDE.md scope convention): NO doc-level requires_env. Slice 1 drives the
# live matthew pod via the deployed doorway (household-nodes class). The forker-generalization
# (Slice 3+) carries hosted-tenant key custody and is its own sub-project.
---

# Web-tool as a keyless, governed dogfooding peer-client

> **One line:** a web2-address-constrained tool (Eclipse Che, and the whole class) cannot host its
> own Holochain conductor (loopback binding), so it participates by driving a **hosted** conductor
> through a **doorway** — holding a **revocable credential**, never a key. The credential's authority
> **flows from** the socially-constructed human-meaning network (DHT steward authority + provenance),
> **through** the doorway (granted custody), **to** the web tool (by extension), and is **verifiable by
> web2 over the medium**. This turns the dev environment into the canonical "a human brings a new
> device onto the network" story — and makes developing the p2p-dataplane *require participating in it*.

## 0. The authority-flow thesis (why this exists)

Conventional web2 auth flows **down** from a central identity provider: a server signs a token, and the
token's authority is the server's. This design inverts that. Authority **flows up** from a rich network
of human meaning:

```
socially-constructed human meaning          ← the root: humans vouch for humans, with context + provenance
   (Holochain DHT steward authority,            (the attestation web; authorization_predicate = issuer-has-attestation)
    rich human context, verifiable provenance)
        │  grants, bounded + revocable
        ▼
   DOORWAY  (custodian of keys; power to act)  ← custody is EARNED, revocable standing — "why doorways must be a flywheel"
        │  grants, bounded + revocable
        ▼
   WEB TOOL (Che, by extension)                ← holds a revocable credential, never a key
        │
        ▼
   verifiable by WEB2 over the medium          ← a web2 observer can trace a credential back through the
                                                  doorway to the steward network and see its human provenance
```

This is **where messy real-world trust is built**: trust is grounded in human meaning and social
authority (per `stewardship-over-sovereignty`), made *legible and verifiable* to the web2 world through
the doorway medium — not asserted by a cryptographic primitive or a central IdP. Every grant in the chain
is bounded and revocable; a misbehaving link loses standing. The web tool is the most-exposed surface and
holds the *least* authority (a revocable grant), by construction.

## 1. The constraint that defines the shape

The Holochain conductor binds `127.0.0.1`. A web-based tool cannot reliably expose or reach loopback
ports (Eclipse Che cannot consistently generate the per-port redirect URL). This is a **structural
property of the entire class of web2-address-constrained tools**, not a Che bug. Therefore:

- A web tool **cannot host-and-drive its own conductor.** Running a conductor *in* Che (call it "Rung 1")
  is foreclosed by loopback.
- A web tool participates by driving a **hosted** conductor through a **doorway** (the T4 projection
  track). Call this **Rung 0 — and it is permanent**, the correct architecture for the class, not a
  stepping stone. (The far horizon — an IDE that is itself an EPR runtime — is out of scope.)

This is legitimate rather than a hack because **peers contract *with* doorways**: a node supplies
DNS / in-kind compute so the doorway serves the EPR content the node stewards, and a web tool's *use* of
the hosted conductor is itself governed by an in-kind compute contract. Both are the same primitive
(`delegates-compute`); see §4.

## 2. Keyless client + custody-as-stewardship

**The web tool is keyless.** The key never lives in the cloud web2 tool. Because the UX is through Che, the
human *cannot and should not* hold keys there. The key lives with a **custodian**:

- **doorway-host-as-custodian (canonical)** — the hosted conductor (e.g. the `matthew` pod, family
  on-prem) holds the key in its lair-keystore and acts on the human's behalf; **or**
- **a localhost stewarded-device** on the human's native laptop (a Tauri-style steward holding the key).

Che holds only a **revocable credential** (§3) that lets it *ask the custodian to act* — never a key.

**Custody is stewardship, not ownership (identity-ontology floor).** The key is the human's. The hoster is
a **custodian**, holding it under a bounded, revocable attestation and backstopped by a **recovery quorum**
(composing with the existing recovery substrate). This is precisely **why doorway hosts must be a
flywheel**: custody is *earned, revocable standing* (the standing+revocation+audit of the compute-commitment
primitive), not a property right. A custodian that stops behaving loses the grant. The most-exposed
surface (cloud Che) carries the least authority; the key sits with the least-exposed custodian under
community-grounded stewardship. **No tier of this design is `self-sovereign`** — autonomy here is
community-grounded standing, per `stewardship-over-sovereignty`.

**Residual (named, NOT "resolved"): the custodian is trusted with the raw key.** The op-gate (§4) governs
the *client's* delegated authority; it does **not and cannot** constrain the custodian's *own* direct key
use — a custodian holds the lair-keystore key and can sign as the human natively, never traversing a
credential-carrying request. Revocation + recovery-quorum + standing bound the custodian's *standing* and
enable *after-the-fact* accountability; they do **not prevent** a malicious custodian signing directly. For
**self-custody** (Matthew's own family-on-prem `matthew` pod, where the human *is* the custodian) there is
no trust gap — which is why Slice 1 is safe. For **hoster-custody** (§9, the forker) it is a **standing
residual** whose real fix — remote-signing / threshold signing so the key never fully resides with the
hoster — is **deferred** (§9, §14). Stated as a residual, not a solved problem.

## 3. The credential — `attestation:device-client-authorization`

A revocable authorization, issued through the captured portal flow, that lets a client act *for* a human
within a scope — the "Claude-Code-API-key analog," but DHT-native and provenance-rooted.

- **Substrate:** rides the **existing** elohim-DNA `Content` entry as a new `attestation:*` subtype
  (consolidated model — 23 subtypes live; generic `issue_attestation`/`revoke_attestation` coordinators).
  **Zero new DHT entry types, zero new coordinators.** Direct precedent: `attestation:key-stewardship`
  (already a *device* attestation, `revocable_by: [issuer, subject]`).
- **Validated-peer issuance:** `authorization_predicate: issuer-has-attestation(attestation:key-stewardship)`
  — only a peer who already holds device-stewardship standing may issue one. This *is* the social-authority
  chain of §0, enforced by the existing attestation floors.
- **Metadata** (new schema `device-client-authorization-metadata.schema.json`):
  `client_id`, `authorization_scope: string[]` (e.g. `["node:seed", "node:reconcile"]`), `valid_from`,
  `valid_until?`, `rate_limit?`, and the bound `commitment_cid` (§4).
- **Revocation:** `revoke_attestation` issues a new Content entry with `metadata_json.revocation`; the
  projection joins on `supersedes_cid` to surface current status — append-only, provenance preserved.
- **The captured portal flow already exists** (RFC 6749 authorization-code): `GET /auth/portal` (the
  `imagodei-portal` EPR renders the consent-card) → `/auth/token` → credential. Today it returns an opaque
  HS256 **JWT**; this design mints/binds the **attestation** as the credential (§10 sequences JWT→attestation
  so the governance lands before the credential hardens).
- **Web2 verifiability:** because the credential is a DHT-notarized attestation with an issuer chain, a web2
  observer reaching the doorway can verify its human provenance over the medium (§0).

**P2P-gate:** Notarized (A), Content-Derived CID (`bafyrei…`) + `uniqueness_anchor:
attestation:device-client-authorization:{subject_cid}:{client_id}`. References `agent_cid` — resolved
canonical, never transport-string. (Full gate output §7.)

## 4. The governance — `delegates-compute` as an operation-authorization gate

The grant is backed by a real, seeded **in-kind compute contract**. For Matthew this is a self-contract
(Matthew→Che); for a forker it is developer→hoster.

- **The commitment already exists and is fully wired:** `delegates-compute.schema.json`, the Mishpat
  validator, the `mishpat_commitments` projection (with `revoked_at`), the read/PATCH/POST routes, and the
  7-check `bounds_validator` (found · not-revoked · active-window · scope · reach-ceiling · rate-limit ·
  key-rotation). **cid = entry_hash** (never action_hash).
- **Two gaps to close** (the only governance work):
  1. **Seed a `delegates-compute` commitment** — today only `custody-blob` is seeded. Add a
     `seed-delegates-compute` factory (scope e.g. `"orchestrate-node"`, provider = the human's steward
     agent, recipient = the client agent, bounds = scope/rate/reach/ttl). Activate `proposed→active`.
  2. **Consult it as an *operation* gate** — today `bounds_validator` only gates `republish-epr`
     *EconomicEvents*. Add an operation-authorization service that, before executing a client-driven
     operation, queries `mishpat_commitments WHERE action='delegates-compute' AND recipient=? AND
     provider=? AND revoked_at IS NULL` and runs the existing 7-check. Reuses `bounds_validator`; adds no
     entity. (Compose-home: `admin-key-lifecycle` stage 3 — the seeder/CI itself graduates from the
     omnipotent admin key to a scoped `delegates-compute` grant by the same mechanism.)

**P2P-gate:** the commitment is Notarized (A), already exists; the op-gate is Operational (C), a
reconstructable read over the notarized commitment. No new entity.

## 5. The orchestration surface (capability-scoped doorway dispatch)

Today the doorway proxies all methods to a single storage target with **boolean** auth
(`.auth_required()`); admin-class routes are "operator-or-nobody" (ingress-gated). The gap is
**capability scoping**:

- Routes (especially admin-class: `/admin/seed/shard-manifest`, reconcile, distribute) declare
  `scoped_capabilities: ["node:seed", …]` and are surfaced into the proxied manifest.
- The credential (§3) carries an `authorization_scope`; the doorway's `classify_dispatch` checks the
  capability set **and** consults the commitment gate (§4) before dispatch.
- Compose-home: `runtime-orchestration-developer-mode-bridge-design` (plane-1 repair endpoint /
  plane-2 developer-mode bridge — designed, deferred). This design fills plane-2 with the
  credential+commitment governance.

**P2P-gate:** the capability set is metadata on the §3/§4 entities; the dispatch check is Operational (C).
No new entity.

## 6. The driving + display loop (the Wave-1.3 unblock)

```
Che (keyless)  →[captured portal flow]→ revocable credential
   →  POST /db/content (BLOB-backed) through the deployed doorway
   →  doorway op-gate: capability check + consult delegates-compute commitment
   →  CUSTODIAN conductor (matthew pod — holds the key) runs distribute_shards
   →  RS shards fan over the LIVE libp2p dataplane → receivers' shard_locations populate (status="announced")
   →  resilience projection: stewardingCollectives ≥ 2, diversity > 0, regions
   →  the DEPLOYED doorway serves the resilience card (SSR) → Matthew's browser (web2 display)
```

This is the **exact Wave-1.3 forcing function** (`live-distribute-shards-household-observation-plan`)
whose dev-session blocker this design dissolves: the dev session could not drive the live mesh from
*outside* it; here Che drives the human's *own hosted node* from *inside* the network, as the human, via
a revocable grant. `distribute_shards` is blob-gated (`http.rs:4301`) — the loop ingests **blob-backed**
content. The display path composes with the in-flight SSR-as-substrate work (the deployed doorway serves
the card; Che's own localhost UI is unreliable per §1, so display is doorway-projected by design).

## 7. P2P Design Gate output

### Entity: `attestation:device-client-authorization`
- **Classification:** Notarized (A) — a credential peers verify and that must stop authorizing on
  revocation; rides the existing `Content` entry as a new `attestation:*` subtype. **No new entry type.**
- **Content Address:** Content-Derived CID (`bafyrei…`) + `uniqueness_anchor:
  attestation:device-client-authorization:{subject_cid}:{client_id}`.
- **Source of Truth:** Holochain DHT. **Coordinator:** `content_store::issue_attestation` /
  `revoke_attestation` (exist). **Projection:** attestations table (`dht_anchor_hash` yes).
- **HTTP:** issuance via the existing portal `/auth/token` exchange → `issue_attestation`; reads via
  existing attestation routes. No new entity POST route.
- **Anti-pattern check:** ✅ no new entry type; ✅ `agent_cid` resolved canonical (never transport-string);
  ⚠️ Identity-Ontology Guard — see §8.

### Entity: `delegates-compute` op-authorization gate (new *semantics*)
- **Classification:** the commitment is Notarized (A), **exists** (`Mishpat::Commitment`, cid=entry_hash).
  The gate is Operational (C) — reconstructable read over the notarized commitment, reusing
  `bounds_validator`. **No new entity.**
- **Anti-pattern check:** ✅ keys on `entry_hash` not action_hash; ✅ no CID-as-FK.

### Entity: capability set / scoped dispatch
- **Classification:** metadata on the two entities above (Operational dispatch check, C). **No new entity.**

### Design constraints discovered
- The seeded commitment must exist and be `active` before the op-gate can authorize anything (ordering:
  seed → activate → gate).
- The credential's `commitment_cid` binds it to the commitment; revoking *either* the attestation or the
  commitment must deny the operation (two independent revocation surfaces — assert both in tests).
- `agent_cid` is the one join key across credential, commitment, and shard_locations.

## 8. Identity Ontology Guard compliance

- **Custody is stewardship, not ownership.** The hoster is a custodian of the human's key, bounded +
  revocable, recovery-quorum-backstopped (§2). No tier is named `self-sovereign`; autonomy is
  community-grounded standing.
- **Authority is community-grounded** (§0) — it flows from the DHT steward network's human meaning, not a
  cryptographic primitive or a self-assertion.
- **Cryptography accelerates, never gates.** The revocable attestation and the recovery quorum *speed*
  recovery/authorization; their absence must never prevent the human's recovery of their own identity.
- **Life-stage holds:** the credential model is delegated-agency-shaped (a client acts *for* a human within
  bounds) — the same shape that expresses graduated/mediated agency (guardian co-authors for a ward). This
  design does not build guardian/ward entities; it stays compatible with them.

## 9. The generalization (forker case — later sub-project)

For a developer who is **not** Matthew and **cannot self-host**, the reproducible pattern is: **contract
with a hoster** (e.g. the `shem` multi-tenant peer) for a hosted conductor, get a `device-client-authorization`
credential, and drive their node from their web tool — content stewarded under **their** identity.

This carries the **hosted-tenant key-custody residual** (§2): **hoster-as-steward** — the hoster custodies
the forker's key under a bounded, revocable attestation + a recovery quorum; the key is the human's, the
hoster a custodian, not an owner. This is honest *framing*, not yet a *mechanism* — a multi-tenant hoster
holds every tenant's raw key, so a single hoster compromise exposes all of them, and nothing at the
substrate prevents the hoster signing as any tenant (the op-gate governs *clients*, not the custodian). The
mechanism that actually closes it — **remote-signing / threshold signing** so the key never fully resides
with the hoster — is heavier and unbuilt; **deferred, and named as the prerequisite before the multi-tenant
forker case ships.** This sub-project also closes the "developer joins as a peer" onboarding gap (no path
exists today) and composes with the household-formation ceremony + `AgentProvisioner`.

## 10. Slices / sequencing

- **Slice 1 — B's governance spine (drives the live dataplane; the Wave-1.3 unblock).** **Build** the
  enforcement (none of it exists today — `distribute_shards` is an *ungated* `tokio::spawn` side-effect of
  `POST /db/content`, `http.rs:4288`): (a) a `seed-delegates-compute` factory seeds a **bounded** self-contract
  (NOT `epr_scope:["*"]`; explicit rate/ttl — §14); (b) a doorway **pre-dispatch op-gate** on the *normal*
  `POST /db/content` route (NOT admin routes — §14) that re-checks the commitment **per-request** (fail-closed)
  and binds `performer == recipient` from the verified credential; (c) Che — holding a credential bootstrapped
  via the **existing `/auth/login`** (the OIDC↔portal auto-binding of §11 is deferred ergonomics, *not* a
  Slice-1 blocker) — POSTs blob-backed content → the custodian conductor runs `distribute_shards` on the live
  mesh → the card reads `stewardingCollectives > 0`, **observable via `GET /api/v1/resilience/{cid}` (JSON) or
  `pnpm look`** (the SSR-rendered card is display *polish* — the resilience route has no `render_spec` today —
  composing with in-flight SSR work, NOT a Slice-1 blocker). Credential *carrier* in Slice 1 is the existing
  portal **JWT** + capability claim + `commitment_cid`; the *authorization* becomes commitment-backed **because
  the op-gate built here consults the commitment per-request** — it is not pre-existing.
- **Slice 2 — A's credential hardening.** Mint `attestation:device-client-authorization` through the
  portal flow; the op-gate verifies the **attestation** (revocable, DHT, provenance-rooted) instead of the
  JWT. *No governance change* — the credential carrier hardens from JWT → DHT attestation.
- **Slice 3+ — the generalization.** hoster-as-steward custody + recovery quorum + the reproducible
  "developer (or web2-constrained tool) joins as a governed peer" onboarding pattern (§9).

## 11. Non-goals / boundaries

- **No key in Che, ever.** Che is keyless; custody is the doorway-host or a localhost steward (§2).
- **No conductor in Che (Rung 1).** Foreclosed by loopback (§1); not a future step.
- **No IDE-as-EPR-runtime.** The far horizon is out of scope.
- **OIDC is the web2 edge only.** It proves "this workspace is driven by human X" to kick off the portal
  consent; the *credential* is DHT-native. The OIDC↔portal binding mechanism is deferred (its own thread).
- **No new DHT entry type, no new coordinator, no new sync dialect.** One new attestation *subtype*; one
  new op-gate service reusing `bounds_validator`.
- **No broad capability taxonomy in Slice 1.** Slice 1 declares only the capabilities the driving loop
  needs (`node:seed` / the blob-ingest op); the full taxonomy is later work.
- **Cluster ops stay operator-owned.** The custodian conductor is the live hosted node; no `kubectl` from
  the dev session.

## 12. Compose-homes (extend, do not fork)

| Piece | Composes into |
|---|---|
| credential substrate | `attestation-consolidation-design` (Content `attestation:*` subtypes); `attestation:key-stewardship` precedent |
| captured portal flow | existing `doorway-service/src/routes/auth_routes.rs` (`/auth/portal`, `/auth/token`) |
| the compute contract | `rea-compute-commitment-primitive`; `delegates-compute.schema.json` + Mishpat validator + `bounds_validator` |
| seeder graduation | `admin-key-lifecycle-dev-to-production` stage-3 (commitment-backed delegation) |
| orchestration surface | `runtime-orchestration-developer-mode-bridge-design` plane-2 |
| custody/sovereignty | `stewardship-over-sovereignty`; the recovery-protocol (quorum) |
| driving loop | `resiliency-card-p2p-weave-sprint-plan` Wave 1.3 + `live-distribute-shards-household-observation-plan` |
| display path | the in-flight SSR-as-substrate work (deployed doorway serves the card) |
| onboarding (Slice 3+) | household-formation ceremony + `AgentProvisioner` |

## 13. Done (definition of the design's first slice landing)

Slice 1's op-gate (doorway pre-dispatch, **per-request** commitment re-check, `performer`-bound) and a
seeded **bounded** `delegates-compute` self-contract are **built and enforced** (none of this enforcement
exists today — the spec describes the target). Matthew, in Eclipse Che, holding only a revocable credential
(no key), drives `distribute_shards` on the live mesh through that gate; the card reads
`stewardingCollectives > 0` (observable via `GET /api/v1/resilience/{cid}` JSON or `pnpm look`); **revoking
the commitment denies the next request** (in-flight fan-out completes — the grant authorizes a discrete
request, not a session). The dogfooding loop is closed: developing the p2p-dataplane now requires
participating in it.

## 14. Threat model & review-hardening (adversarial review 2026-06-26)

A 4-lens adversarial review (red-team · identity-ontology · p2p-gate/compose · completeness) hardened this
spec. Identity-ontology **PASSED** (no self-sovereign drift). The binding corrections:

**Honesty — the spec describes a TARGET, not current enforcement.** Today the doorway forwards the
`Authorization` header without verify/capability/commitment work (`storage_proxy.rs:176`), `classify_dispatch`
does pure routing with no auth (`server/http.rs:76`), `auth_required` never gates a request, and
`distribute_shards` is an ungated side-effect (`elohim-storage/src/http.rs:4288`). The op-gate, the
per-request commitment consult, and the seeded contract are all **Slice-1 build work**.

**Op-gate hardening (binding on §4/§5/§6):**
- **Per-request re-check, fail-closed.** The op-gate consults the commitment projection on *every* operation
  (not at issuance) and denies on conductor-unreachable/un-notarized (`bounds_validator` already fails closed,
  `:128`). The JWT carrier is **not** itself revocable server-side (`handle_logout` is a no-op; the
  `is_active` check is wired only to `/auth/me` and fails *open*) — so revocation lives in the per-request
  commitment consult, **never** in the JWT. The bounded projection-lag window is accepted and noted.
- **Bind `performer == recipient`.** `bounds_validator` does *not* compare performer to `commitment.recipient`
  (`bounds_validator.rs:107-326`); the op-gate MUST, deriving `agent_cid` from the **verified credential**,
  never the client-set `X-Agent-Cid` header (trusted verbatim at `account.rs:1000`). Add a `performer==recipient`
  check inside `bounds_validator` as defense-in-depth.
- **Seed bounded.** `bounds_validator` defaults `rate_per_hour`/`rotation_ttl_days` to `u64::MAX`, and the
  `delegates-compute` schema blesses `epr_scope:["*"]` for *bootstrap* — together effectively unbounded. The
  `seed-delegates-compute` factory MUST reject `["*"]` + omitted rate/ttl for the dogfooding contract
  (minimum-bounds guard).
- **Compose capability AND scope.** The credential's `authorization_scope: string[]` and the commitment's
  single `scope` are different vocabularies; the gate enforces the **conjunction**, not either-or.

**Admin-route safety (binding on §5):** **Slice 1 touches ONLY normal authenticated routes** (`POST
/db/content`). Admin-class routes (`/admin/*`) are protected *today by ingress isolation alone* (no in-handler
auth — "operator-only is an ingress property"). Capability-scoping them is **later** work and MUST **add** the
capability gate while **retaining ingress isolation as defense-in-depth** — never substitute. Capability-scoping
an admin route before the gate is built+verified is a privilege-escalation superhighway.

**`distribute_shards` placement:** the op-gate authorizes the *client request* at the doorway; once authorized
and forwarded, `distribute_shards` runs as the node's **own** authorized side-effect (its own key, having
accepted an authorized request). In-flight fan-out completes; revocation denies the **next** request. The gate
governs requests, not the node's internal spawn.

**Carrier honesty:** the Slice-1 **JWT is HS256 symmetric** (doorway-asserted) — NOT web2-provenance-verifiable
(a `/.well-known` JWKS cannot verify it without the minting secret). §0's "verifiable by web2 over the medium"
is a **Slice-2** property (the DHT attestation + its issuer chain). Also: stop accepting tokens via query-string
(`extract_token_from_url`) — they leak into logs/referers.

**Deploy posture:** the Che-facing deploy MUST NOT run `dev_mode` — it returns `Admin` for *any* credentials when
Mongo is absent (`auth_routes.rs:1573`). The dev/prod boundary is load-bearing precisely because this design
makes a dev environment a live peer-client against the *deployed* doorway.

**Implementation prerequisites (Slice 1, explicit):** the `device-client-authorization` subtype needs a manifest
entry (`imagodei/manifest.json`) + a metadata schema + `generated_attestation_kinds.rs` regen; the op-gate needs
an `EventForValidation`-shaped adapter to reuse `bounds_validator` for *operations* (it is event-shaped today).
The §5 compose-home (`runtime-orchestration-developer-mode-bridge` plane-2) is itself OPEN/undesigned — this
spec *fills* it, it does not merely extend it.

**Recovery cross-ref (identity-ontology):** when a credential or commitment is revoked, the human's recovery of
their own identity completes via the recovery-quorum's **non-cryptographic** path (the Grandma Standard — "log
in on a new device with help from your people"), never gated on a key the human might have lost
(`stewardship-over-sovereignty` §4–§6).
