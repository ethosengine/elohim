---
title: "Dual-WAN Utility-Plane Failover — HTTP/Bootstrap/Signal Resilience Beyond the TURN Relay Pair (deferred, vision-tier)"
id: dual-wan-utility-plane-failover
status: vision
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR superseded-by-implementation
created: 2026-07-16
maintainers: Matthew Dowell + Opus 4.8
cites:
  - genesis/data/timeline/backlog/doorway-utility-plane-and-borrowed-infra-exit-doctrine.md
  - iroh-libp2p-complementarity | Track 1 DHT-notary bootstrap (`https://doorway.elohim.host/bootstrap`) and signal (`wss://signal.doorway.elohim.host`) endpoints — the concrete HTTP/WS surfaces this doc's failover options apply to | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md
  - .claude/skills/p2p-design-gate/SKILL.md
---

# Dual-WAN Utility-Plane Failover — HTTP/Bootstrap/Signal Resilience Beyond the TURN Relay Pair

> **Status: vision-tier, deferred.** This document designs a plane that is **NOT built now**. What shipped
> 2026-07-16 is a dual-WAN sovereign TURN relay pair (coturn on ethosengine + coturn on shem, both listed in
> the conductor's ICE config) plus a general `relay-addr-beacon` that keeps each relay's dynamic residential
> WAN IP fresh in DNS. That work is the **RELAY plane** and it already gets failover for free (§1). This
> document is the design horizon for **every other plane a doorway carries on HTTP** — bootstrap, signal,
> the SPA/API surface itself — which do **not** get failover for free and need their own reframe.

## 0. Why this doc exists, and why it's separate from the TURN work

The 2026-07-16 build session shipped a dual-WAN TURN pair because ICE natively tries every listed server and
uses whichever forms a working candidate pair — redundancy was "list two servers," not "build a load
balancer." Reading that cheapness back onto the *rest* of the doorway's public surface would be a category
error: bootstrap and signal are plain HTTP(S)/WSS services with **one DNS name and no protocol-level
multi-endpoint trial**. A browser or conductor calling `https://doorway.elohim.host/bootstrap` does not try a
second address when the first times out unless something (client-side retry logic, DNS, or a load balancer)
gives it one. This doc is that "something" — designed, not built, per the exit-doctrine's standing pattern
(`doorway-utility-plane-and-borrowed-infra-exit-doctrine.md`): every borrowed-or-fragile piece of
infrastructure carries a filed exit, not a silent hope.

It is filed as **horizon (a)** of that backlog's widened scope, and it is gated by `p2p-design-gate` the
moment it moves from vision to any concrete DHT record (a doorway-set entry, a relay-set entry, a
utility-role attestation) — see §5.

## 1. The reliability reframe: the whole alpha substrate rides residential DDNS on both WANs

Neither alpha genesis peer sits behind a static, professionally-hosted endpoint. `matthew` (on-prem/home) and
`adam`/shem both terminate on **Google-Fiber residential service** with a dynamic WAN IP, kept current only by
DDNS-shaped mechanisms (the `relay-addr-beacon` built 2026-07-16 is one instance of this pattern, scoped to
TURN's `external-ip`). This is not a defect to hide — it is the literal shape of a hub-optional, no-datacenter-
required protocol (`project_hub_optional_floor`). But it means **every plane the doorway exposes over a fixed
DNS name inherits the same fragility class** as the TURN relay did before 2026-07-11: a single address, a
single ISP link, a single point where "the residential line drops" becomes "the service is down," not merely
"one candidate among several failed."

The RELAY plane escaped that fragility by accident of protocol shape (ICE tries multiple servers natively).
The **HTTP/bootstrap/signal planes did not** — they need a deliberate design, which is this document.

## 2. What's already free: the RELAY plane, contrasted

Recap so the boundary is explicit (see `sovereign-turn-relay-transport-commons.md` for the built artifact):

- Conductor ICE config lists **both** `turn:turn.elohim.host:3478` and `turn:turn-shem.elohim.host:3478`.
- ICE gathers candidates from every listed server and forms whichever pair connects. If ethosengine's WAN link
  is down, the shem candidate still forms a pair; vice versa. **No load balancer, no health check, no DNS
  trickery required** — this is WebRTC's own multi-server trial baked into the ICE state machine.
- The `relay-addr-beacon` only solves a *different* problem: keeping `turn.elohim.host` / `turn-shem.elohim.host`
  each pointed at the CORRECT current WAN IP for their OWN relay (DDNS), not at each other. Each name still
  maps to exactly one dynamic address — the failover come from ICE trying **both names**, not from either
  name itself being resilient.

That last sentence is the generalizable insight: **redundancy at the protocol layer that already fans out to
multiple servers is nearly free (list more servers). Redundancy at a plane with exactly one canonical
endpoint is not free** — it requires either the client to retry across multiple addresses (something ICE does
and HTTP does not, by default) or a server-side arbiter in front of the address space. §3 enumerates the
options for the latter case.

## 3. The HTTP/bootstrap/signal planes: three options, honestly graded

These three planes — DHT bootstrap (`https://doorway.elohim.host/bootstrap`), DHT signal
(`wss://signal.doorway.elohim.host`), and the doorway's own HTTP/SPA surface — are the ones without ICE's
free multi-trial. All three currently resolve through a single DNS name per doorway (Track 1 in
`iroh-libp2p-complementarity.md` — "operator-runnable; multi-doorway registration" already implies more than
one doorway can serve these roles, which is the seed this section elaborates).

### 3a. Multi-A + client retry (free, imperfect)

Publish multiple `A`/`AAAA` records for the same name (one per doorway WAN, e.g. ethosengine + shem), and rely
on the calling client (browser `fetch`, the conductor's bootstrap client) to retry a second resolved address
when the first connection times out.

- **Cost:** zero infrastructure. DNS multi-A is a records-only change.
- **Failure mode:** most HTTP clients do NOT automatically retry across multiple A records on connection
  failure (browsers vary; Rust `reqwest`/`hyper` do not by default) — this only works if the resolving client
  is taught the retry, i.e. it is a *client-side* fix wearing a DNS costume. DNS TTL caching also means a
  downed address can linger in resolver caches well past its actual outage.
  Honest verdict: this is the "free" option because it needs no new infrastructure, but it is imperfect
  because "free" describes the *records*, not the *reliability* — the retry logic has to be written and
  shipped in every client that needs it (conductor bootstrap client, browser SPA fetch wrapper), same
  shape as the exit-doctrine's other "borrow now, file the exit" items.

### 3b. Cloudflare Load Balancing with health checks (HTTP-only, dependency-with-filed-exit)

A managed health-checked load balancer (Cloudflare LB, or equivalent) in front of both doorway WANs, routing
HTTP(S)/WSS traffic to whichever origin is currently healthy.

- **Cost:** low engineering lift, but it is a **new borrowed dependency** — Cloudflare (or any such provider)
  becomes a duress mode of its own (outage, policy flip, geographic/censorship blocking, acquisition — the
  same category the exit-doctrine already tracks for STUN/DNS/CA). Per that doctrine's rule (**no borrow
  without a filed exit**), adopting this option means opening its own ledger row alongside the TURN
  mechanism-exit row this doc's sibling edit adds (§5 of the backlog doc).
- **Scope limit:** HTTP/WSS only — it does not help the RELAY plane (already solved, §2) and it does nothing
  for any future non-HTTP plane.
- **Honest verdict:** the correct near-term answer for the bootstrap/signal/SPA HTTP surfaces IF the operator
  accepts a filed, named dependency on a commercial LB provider as an interim step — never as the permanent
  answer, because it re-creates a captured chokepoint at exactly the layer the protocol exists to route
  around.

### 3c. BGP anycast (far horizon, council-held commons)

A single announced address block (an owned ASN + IP block) advertised from multiple physical locations via
BGP, so the network layer itself routes clients to the nearest/healthiest announcing site — true anycast, no
DNS trickery, no third-party LB.

- **Cost:** highest — acquiring and operating an ASN and address block is real-world infrastructure
  ownership, not a config change. This is explicitly named in the parent backlog as the **far horizon** of
  "logical anycast," held as a **council-held commons resource**: the earned-reach governance machinery
  applied to a real-world network artifact, not a doorway's individual property.
- **Honest verdict:** the eventual right shape for a fully-distributed, non-captured utility plane, but it is
  a multi-year, multi-operator commitment — not something either alpha genesis peer can stand up alone. Filed
  as vision, not scheduled.

## 4. The honest caution: two same-ISP lines are correlated, not independent

A design trap worth naming explicitly, because it is easy to build and easy to believe is more resilient than
it is: **standing up a self-hosted load balancer on ONE of the two WANs (say, ethosengine) just relocates the
single point of failure** — now the LB's own link is the new chokepoint, and it sits in front of BOTH origins,
so its outage takes down access to a still-healthy second doorway. A load balancer is only as resilient as its
own placement.

Deeper caution: **both alpha genesis peers currently ride Google Fiber.** Two lines from the same ISP are
correlated failure, not independent failure — a regional Google Fiber outage, a peering incident, or a
policy/ToS action against Google Fiber accounts takes out BOTH WANs simultaneously, and no amount of clever
failover between them helps, because the fault is upstream of both. This is the same "duress modes" discipline
the exit-doctrine already applies to STUN/TURN/DNS: model the CORRELATED failure, not just the naive
per-endpoint one.

**The real resilience is the N-node federated commons** — independent households, independent ISPs,
independent geographies — of which this 2-WAN pair (ethosengine + shem) is only the **seed testbed**, not the
end state. Every option in §3 should be read as "how do two-to-N doorways on independent links present as one
resilient utility plane," never as "how do we make one physical link behave reliably" — that ceiling is fixed
by physics, not engineering cleverness.

## 5. Where this sits, and the gate it must pass to become real

This document is **horizon (a)** of `doorway-utility-plane-and-borrowed-infra-exit-doctrine.md`'s widened
scope — the HTTP/bootstrap/signal legs of the utility plane, deliberately separated from the RELAY leg that
shipped 2026-07-16 (that shipment is recorded as the sibling backlog `sovereign-turn-relay-transport-commons.md`
and as the new TURN-mechanism-exit row this doc's companion edit adds to the exit-doctrine's opening ledger).

Nothing in this document is a build plan. If and when any option in §3 moves toward implementation and
introduces a **new DHT entry type, doorway-set record, relay-set record, or utility-role attestation**, it
MUST first pass `p2p-design-gate` (`.claude/skills/p2p-design-gate/SKILL.md`) — is the entity notarized (A),
derived-via-link (A2), agent-scoped (B/B2), or operational (C); does a DHT entry type already exist; is
identity content-derived, agent-composite, or slug; what coordinator function creates it and what signal
projects it. Multi-A DNS (§3a) and a commercial LB (§3b) introduce no new DHT entries and can proceed as
ops/config changes with a filed exit-ledger row; BGP anycast (§3c) is far enough out that its DHT shape (if
any — e.g. an attestation of which doorway currently announces the block) is undesigned and explicitly
deferred to the point it is actually scheduled.

## 6. Summary table

| Plane | Failover mechanism today | Cost to add real failover | Verdict |
|---|---|---|---|
| RELAY (TURN) | ICE tries every listed server natively | **Already built** (2026-07-16, dual coturn + relay-addr-beacon) | Free — solved |
| Bootstrap/Signal/SPA (HTTP/WSS) | None — single DNS name per doorway | 3a multi-A+retry (free, imperfect) / 3b Cloudflare LB (low-cost, new filed dependency) / 3c BGP anycast (high-cost, council-held commons) | Deferred — this doc is the design horizon |
| Cross-WAN correlated risk | N/A | Diversify ISPs/households, not just links | Structural — solved only by the N-node commons, not by engineering the 2-node pair harder |
