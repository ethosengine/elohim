---
id: "backlog-wan-nat-federation-discovery-three-plane"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "WAN-NAT mutual discovery across three planes — doorway /federation/doorways lists self-only (DHT get_all_doorways is a stub); name in-cluster DNS as a dev-test fixture, not architecture"
slug: "wan-nat-federation-discovery-three-plane"
written: "2026-06-23"
author: "cartographer"
status: "backlog"
priority: "high"
area: "doorway / federation / p2p-dataplane"
relatedNodeIds:
  - "memory:feedback_k8s_is_not_the_architecture"
  - "memory:project_alpha_substrate_probe_rails"
tags: [doorway, federation, dht, wan-nat, nat-traversal, co-location-fixture, p2p-design-gate, iroh, libp2p, kitsune2]
cites:
  - doorway/doorway-service/src/services/federation.rs
  - doorway/doorway-service/src/routes/federation.rs
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/edgenode/alpha.yaml
  - genesis/manifests/humans/matthew-manager.yaml
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md
  - genesis/docs/superpowers/plans/2026-05-10-iroh-pkarr-resolver.md
shift_objective: |
  STEP 1 (fix-now, coordinator-only, the concrete first move):
  GET /api/v1/federation/doorways lists ONLY the local doorway because
  services::federation::get_all_doorways (federation.rs:535-562) is a STUB —
  it calls get_doorway_by_id with its OWN id (federation.rs:550) and returns
  self, self-documented "A full implementation would use a 'list all' anchor
  pattern" (federation.rs:543). The HTTP FEDERATION_PEERS fallback that could
  otherwise add a peer fails in the co-located alpha pair via a
  pod->public-ingress HAIRPIN (Loki "Failed to reach peer doorway"; external
  curl to the same URL returns 200). Implement the DHT "list-all" anchor read so
  doorway discovery is genuinely DHT-native per doorway/CLAUDE.md ("DHT-native,
  not a central registry"), riding the already-healthy kitsune2/tx5 gossip plane.

  This is COORDINATOR-ONLY (R2 verdict, confirmed against the integrity enums):
  - register_doorway (infrastructure/zomes/infrastructure/src/lib.rs:259-306)
    creates 3 links, all off per-doorway-keyed anchors (IdToDoorway off
    StringAnchor("doorway_id", input.id); OperatorToDoorway; RegionToDoorway).
    There is NO link off a fixed shared anchor -> that is exactly why a self-only
    doorway sees only itself.
  - ADD one create_link in register_doorway from a fixed sentinel anchor
    StringAnchor::new("doorway_id", "__all__") -> action_hash over the EXISTING
    LinkTypes::IdToDoorway (integrity lib.rs:241). A new anchor STRING is new
    entry CONTENT, not a new entry TYPE — StringAnchor is already in EntryTypes
    (integrity lib.rs:230). No #[hdk_entry_types]/#[hdk_link_types] change -> DNA
    hash does not move.
  - ADD a get_all_doorways() coordinator fn: verbatim copy of
    get_doorways_by_operator (infrastructure lib.rs:396-423) with the base
    swapped to the fixed sentinel, latest-wins dedup-by-id (collapse re-key churn
    as get_doorway_by_id does at lib.rs:373).
  - SWAP the doorway-service call site (federation.rs:535-562) to
    zome_caller.call::<(), Vec<DoorwayOutput>>(role, zome, "get_all_doorways", &()).
  Healed via update_coordinators hot-swap, gated ALLOW_COORDINATOR_UPDATE — no
  re-key, no DHT churn, no genesis-pair partition. DO NOT take the
  ALLOW_DNA_REINSTALL path. MANDATORY: run p2p-design-gate first — DoorwayRegistration
  is a notarized Cat-A entry; the list-all is a link-anchor read (A2), NOT a new
  entity. Scaling caveat: a single fixed anchor is a hot link-base (the
  dna/CLAUDE.md "*By{Attribute}" warning) — a non-issue at alpha/genesis scale;
  flag it as a deliberate choice.

  Done (Step 1) when each doorway lists its WAN peers from the DHT anchor with
  FEDERATION_PEERS as fallback only, verified on the alpha pair WITHOUT relying
  on in-cluster DNS.

  STEP 2 (label, same pass): add the DEV-TEST FIXTURE comment (body §4) to BOTH
  doorway/alpha.yaml and alpha-b.yaml at the in-cluster repoint, so the
  co-location convenience is never mistaken for the architecture.

  LATER (separate entries / spec waves): the storage libp2p dataplane is
  in-cluster-pinned (svc.cluster.local, no relay reservation, no QUIC, no
  ANNOUNCE_ADDRS) and the iroh stack that DOES wire relay+pkarr correctly is
  dormant (backend resolves to Libp2p; the alpha "dual-stack" manifest value is a
  no-op — wrong env var name + no enum variant). Tracked by the iroh pkarr gate
  #10 plan; this entry references it, does not subsume it.
---

## The gap, in one sentence

WAN-NAT mutual registration/discovery is the architecture on every P2P plane; in-cluster k8s DNS is a co-location fixture wherever it appears — and that distinction is not yet written down, so a co-location convenience (shared in-cluster MongoDB; FEDERATION_PEERS HTTP fallback) is silently standing in for the protocol-native path.

## Three-plane WAN-NAT readiness map (decisive evidence)

| Plane | What lives here | WAN-NAT readiness TODAY | Decisive evidence |
|---|---|---|---|
| **Doorway HTTP federation** (the `/threshold/doorways` selector) | the doorways list the user sees | **Stubbed (DHT) + hairpin-failing (HTTP fallback)** | `get_all_doorways` (`services/federation.rs:535-562`) calls `get_doorway_by_id` with its OWN id (`:550`), self-documented "list all anchor pattern" missing (`:543`); `routes/federation.rs:68-141` merges this self-only DHT source with the `FEDERATION_PEERS` HTTP cache, which fails on pod->public-ingress hairpin (Loki "Failed to reach peer doorway") while external curl 200s. |
| **Conductor DHT gossip** (kitsune2/tx5, where `DoorwayRegistration` lives) | agent + entry gossip | **Works by design, exercised over WAN today** | `conductor-config.yaml` (`edgenode/alpha.yaml:50-60`): `bootstrap_url: https://doorway-alpha.elohim.host/bootstrap`, `signal_url: wss://signal.doorway-alpha.elohim.host`, `enable_relaying: true`, cloudflare+google STUN — public URLs, no in-cluster substitution. Shared `elohim-bootstrap` MongoDB (`bootstrap/k2_mongo.rs:4-5`, keyed by `(space, agent)`, GET filters by space) kills genesis-pair islanding across two different public bootstrap hostnames. peerCount ~13, conductor.connected=true. This is the **working reference** the other two should match. |
| **Storage libp2p + iroh dataplane** (content/blob replication, port 9876) | byte replication | **Built-but-pinned-to-in-cluster (libp2p) / built-but-dormant (iroh)** | libp2p relay/DCUtR/AutoNAT behaviours compiled (`p2p/behaviour.rs:114-125`) but no relay-reservation dial exists (`p2p/mod.rs` has only the server-side `ReservationReqAccepted` counter at `:5537`); no QUIC, no `ANNOUNCE_ADDRS` in any manifest; `P2P_BOOTSTRAP_NODES` is all `svc.cluster.local` (`edgenode/alpha.yaml:277`, `matthew-manager.yaml:268`); mDNS off. iroh endpoint wires `RelayMode::Default` + pkarr correctly (`p2p_iroh/endpoint.rs:37-62`) but is dormant — backend resolves to `Libp2p` (the alpha `TRANSPORT_BACKEND: "dual-stack"` at `edgenode/alpha.yaml:284-285` is read by NOTHING: code reads `ELOHIM_TRANSPORT_BACKEND` at `main.rs:394`, and "dual-stack" isn't a `TransportBackend` variant — `config.rs:16-21` has only `Libp2p`/`Iroh`). |

The cross-plane insight (and the misrouting hazard): the same WAN-NAT requirement manifests differently per plane, and the danger is "fixing" it on one plane with a co-location mechanism and declaring victory. The conductor plane shows it can be done WAN-native; the other two are wiring/config gaps, not protocol-design gaps (`feedback_k8s_is_not_the_architecture`: "k8s gaps != protocol gaps").

## §4 — The DEV-TEST FIXTURE manifest comment (paste verbatim into BOTH doorway/alpha.yaml and alpha-b.yaml at the in-cluster repoint)

```yaml
# DEV-TEST FIXTURE (NOT architecture): this in-cluster reference lets the
# CO-LOCATED alpha genesis pair (elohim-doorway-alpha / -alpha-b, same
# elohim-alpha namespace) discover each other despite the pod->public-ingress
# HAIRPIN that breaks the WAN path inside one cluster. The ARCHITECTURE is
# WAN-NAT mutual registration: doorways register/discover over the public WAN
# (DHT DoorwayRegistration list-all anchor + the kitsune2/tx5 signal plane; pkarr
# resolvers for the storage dataplane), NOT via k8s in-cluster DNS. k8s here is
# orchestration-of-the-runtime-model only.
# Principle: memory feedback_k8s_is_not_the_architecture ("k8s gaps != protocol gaps").
# Tracked: genesis/data/timeline/backlog/wan-nat-federation-discovery-three-plane.md
```

## Prior art this does NOT duplicate (and how it differs)

- The **2026-06-14 federation arc** (`FEDERATION-WEB2-LEDGER-2026-06-14.md`, the four `2026-06-14-federation-*-plan.md`) diagnosed the *same symptom* as **bootstrap-islanding** and fixed it via **shared in-cluster MongoDB** (`federation-bootstrap-plan.md:8-12`) — but never labeled that as a dev-test fixture; it treats co-location reachability as done. This entry supplies the missing label and the WAN-native read path.
- The **iroh pkarr-resolver gate #10** (`2026-05-10-iroh-pkarr-resolver.md`) already owns the storage-dataplane WAN-NAT discovery substrate; the canonical n0-seam + STUN/SBD NAT precedent lives in `2026-05-08-iroh-libp2p-complementarity.md` (`:131`, `:351`, `:439`). This entry references those, does not re-derive them.

## Recommended follow-on

A single canonical **architecture doc** stating the cross-plane invariant once and enumerating the three plane-specific instances (sibling to the complementarity doc, under `genesis/docs/content/elohim-protocol/architecture/`). Author it when promoting this entry to `refined`; the manifest comment and this entry can cite it then. Optionally split a sibling backlog entry that tracks the storage libp2p relay-reservation/QUIC/ANNOUNCE_ADDRS wiring separately from the doorway fix.
