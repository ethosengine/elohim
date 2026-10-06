# Doorway / Hub Edge Boundary

> **See also:** [ARCHITECTURE.md](./ARCHITECTURE.md), [FEDERATION.md](./FEDERATION.md), [SCALING.md](./SCALING.md), [REACH.md](./REACH.md), and the live statement of the doorway/hub split: `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md`, section "Doorway / hub edge". The spec this file used to cite, `2026-05-08-doorway-hub-edge-design.md`, was retired to git history in commit `53190a234` and compacted into that section.

## Why this doc exists

The recent SSR delivery wiring made ingress first-class instead of a future hand-wave, and a parallel exploration of how hyperscalers (Cloudflare, K8s ingress) actually handle aggregate-scale concerns — DDoS, anycast, IPv6 reachability, eBPF/XDP, PoW gates — surfaced a question: when the protocol stops leaning on hyperscalers, where does the aggregate-scale work land?

The answer is **larger than doorway**. Doorway is the per-deployment web2 projection surface, and it stays simple by design. The aggregate-scale concerns belong at the **hub layer** — the home-node cluster that stewards a family or collective's compute, federates horizontally with peer hubs, and coordinates discernment via elohim-operators.

This doc is the doorway-crate-side pointer to that boundary. The design lives in the boundaries document named above; this doc names what stays at doorway and why.

## The placement test (2026-10-06)

One question decides where a web-level concern goes: **does this exist only because the other end cannot address and verify a peer by key?** A browser, a crawler and a web2 protocol cannot, so a registered name, the certificate for it, the cache that absorbs traffic arriving with no relationship, and a web2 protocol projection stay here. A DNS name is the usual sign of doorway work, not its definition: a household's own browser-only clients need a name and a certificate too, and that is this service run on the hub box beside the hub runtime. Anything web-level that works when the peer is reached by its key (serving an app's files, rendering a page, judging whether a version starts) is a hub concern, and its home is `elohim/elohim-hub/`. This document says where concerns belong; where the code differs, the code is expected to move, and the pieces that have not moved are listed in that README as debt. Bytes, heads, custody and reach stay in `elohim-storage`. The full statement, and the list of what sits on the wrong side today, is `elohim/elohim-hub/README.md` §The web seam.

## What stays at doorway

| Responsibility | Notes |
|---|---|
| TLS termination | the listener is `src/tls.rs`; issuing and renewing certificates (cert-manager, ACME, wildcard certs as operator preference) is deployment work in `genesis/orchestrator/manifests/doorway/`, where cert-manager is wired for the public doorways. Issuing a household's own certificate is doorway-role work that is not built beyond the development mesh's local certificate authority (debt, hub README) |
| HTTP/3 and QUIC | natural transport pairing with libp2p QUIC underneath storage; not implemented in this crate today |
| Manifest-driven request routing | `RouteRegistry` (`src/services/route_registry.rs`; history in `genesis/docs/content/elohim-protocol/history/2026-06-02-doorway-dispatch-registry-fallback-and-vocabulary.md`) |
| OAuth-RP identity presentation | doorway presents identity that lives elsewhere; never owns it. The network holds a person's identity. A doorway may hold a hosted person's key and sign on their behalf, and its portal says so (`genesis/a2o/features/auth/device-provisioning-paths.feature`) |
| Reach gating per request | the existing REACH.md ladder, deterministic per request |
| SSR delivery, not execution | the doorway relays and caches a render its backing peer produced. Running the renderer, and judging which head is deliverable, are hub concerns (2026-10-06). The renderer code still in this crate is debt, listed in the hub README |
| Federation projection | ATProto today, possibly ActivityPub later (per `genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md`) |
| Single-target dispatch | substrate moves bytes peer-to-peer; doorway projects + caches |
| Inside-out registration | peers register interest; doorway makes content available |

Doorway runs on a single blade comfortably. A household steward can deploy and operate one without thinking about kubernetes, BGP, or eBPF.

## What explicitly does NOT stay at doorway

These rules are constitutional. Each names where it is written down, because a future design pass will be tempted to drag these back into doorway:

| Boundary | Written down in |
|---|---|
| Doorway never swarms libp2p; storage does | `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md` §3.9 |
| Doorway never fans out blob delivery to peers | `doorway/CLAUDE.md`, "No Blob Fan-Out" |
| Doorway presents identity, never owns it | `genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md`; `genesis/a2o/features/auth/device-provisioning-paths.feature` |
| The mesh is the hosting layer; doorway is optional projection | `genesis/docs/content/elohim-protocol/architecture/MAP.md` |
| Routes register themselves via manifest; doorway is registry-driven | `doorway/CLAUDE.md`, "No Per-Domain Proxy Files" |
| Inventory exchange is metadata-only; bytes single-target | `.claude/memory/project_inventory_exchange_not_byte_replication.md` |

## What goes to hub

The hub layer, whose home is `elohim/elohim-hub/` (its README gives the archetype framing, the web seam and the debt against it), absorbs:

- Web-level work for peers reached by key: serving an elected app bundle's files, server rendering, the deliverability verdict, render capability (the web seam, 2026-10-06)
- Cross-hub threat coordination (the Cloudflare-class concern at federation scope)
- AbusePattern signal aggregation (`signal_kind` extension, future)
- Mobile device inference request processing (family edge-AI)
- Elohim_observer stream processing
- Workload state migration (PVCs / cluster rebalancing)
- Continuously-negotiated meet-and-protect compute contracts
- Multi-doorway failover (which doorway is healthy for a given human)
- Elohim-operator discernment at hub scope

These responsibilities use the **reach-earning** principle — already load-bearing at per-message authoring (`genesis/docs/content/elohim-protocol/architecture/2026-04-23-epr-phase-2c-libp2p-federation-design.md` §3.4.1; `genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md`) — extended to compute, distribution, defense, and AI-coordination at aggregate scale. A pattern shaped like a DDoS attack is structurally unearned-reach compute or distribution; the hub fabric doesn't engage with it because no node along the way has reason to spend cycles on it.

## Operator deployment concerns (neither doorway nor hub code)

Some Gemini-stimulus topics are operator deployment concerns — they live in the CNI / kernel / cluster setup, not in protocol code:

- eBPF/XDP / Cilium kernel-layer packet drop
- IPv6 GUA per component (operator config)
- NDP cache exhaustion hardening (kernel sysctls)
- BGP-feasible Anycast (only on BGP-friendly infra; default is GSLB-via-DNS)
- MetalLB + BIRD (k8s anycast bridge)

Doorway and hub coordinate with these concerns via signals (traffic-shape feedback, AbusePattern emission) but do not implement them.

## Hyperscaler-fronting

Hyperscaler-fronting (Cloudflare, AWS Shield, GCP Armor) is **allowed** as an operator opt-in for doorways facing heavy public web traffic. It is **not** the protocol's answer to DDoS — the protocol's answer is hub federation + reach-earning + elohim-operator coordination + socially-resilient compute contracts, which must work without a hyperscaler in front of any doorway.

A household-scale dwelling on a residential connection cannot afford or operate a hyperscaler partnership; the protocol must remain credible at that scale.

## Forward path

The retired hub-edge spec carried ten stub-epic seeds and a list of open questions (readable with `git show 53190a234^:genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md`). The seeds and questions most directly affecting doorway code:

- **SSR projection attestation** — does the rendered HTML carry a `ProjectionClaim` analogous to ATProto outbound? Sibling to `genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md`.
- **AbusePattern emission from doorway** — doorway observes and emits; hub aggregates patterns; federation propagates.
- **Wasm projection filters** — federation manifest declares projection logic per route; doorway runs the filters.
- **Federation-level doorway-to-doorway communication** — settled: doorway-to-doorway federation is HTTP-shaped and live (`doorway/CLAUDE.md`, "Federation"), and it is not swarming. The retired spec asked for that to be said explicitly.

None of these are scheduled.
