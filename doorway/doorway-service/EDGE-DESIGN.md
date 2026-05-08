# Doorway / Hub Edge Boundary

> **See also:** [ARCHITECTURE.md](./ARCHITECTURE.md), [FEDERATION.md](./FEDERATION.md), [SCALING.md](./SCALING.md), [REACH.md](./REACH.md), and the canonical design at [`genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md`](../../genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md).

## Why this doc exists

The recent SSR delivery wiring made ingress first-class instead of a future hand-wave, and a parallel exploration of how hyperscalers (Cloudflare, K8s ingress) actually handle aggregate-scale concerns — DDoS, anycast, IPv6 reachability, eBPF/XDP, PoW gates — surfaced a question: when the protocol stops leaning on hyperscalers, where does the aggregate-scale work land?

The answer is **larger than doorway**. Doorway is the per-deployment web2 projection surface, and it stays simple by design. The aggregate-scale concerns belong at the **hub layer** — the home-node cluster that stewards a family or collective's compute, federates horizontally with peer hubs, and coordinates discernment via elohim-operators.

This doc is the doorway-crate-side pointer to that boundary. The full design lives in the spec above; this doc names what stays at doorway and why.

## What stays at doorway

| Responsibility | Notes |
|---|---|
| TLS termination | cert-manager, ACME, wildcard certs as operator preference |
| HTTP/3 and QUIC | natural transport pairing with libp2p QUIC underneath storage |
| Manifest-driven request routing | `RouteRegistry` (per `2026-04-28-doorway-blob-registry-routing.md`) |
| OAuth-RP identity presentation | doorway presents identity that lives elsewhere; never owns it |
| Reach gating per request | the existing REACH.md ladder, deterministic per request |
| SSR delivery | the precipitating moment; alpha cluster end-to-end |
| Federation projection | ATProto today, possibly ActivityPub later (per `2026-05-01-atproto-lexicon-projection-doorway-design.md`) |
| Single-target dispatch | substrate moves bytes peer-to-peer; doorway projects + caches |
| Inside-out registration | peers register interest; doorway makes content available |

Doorway runs on a single blade comfortably. A household steward can deploy and operate one without thinking about kubernetes, BGP, or eBPF.

## What explicitly does NOT stay at doorway

These rules are constitutional. Memory anchors are cited because future agents (or future-Gemini-style brainstorms) will be tempted to drag these back into doorway:

| Boundary | Memory anchor |
|---|---|
| Doorway never swarms libp2p; storage does | `project_three_layer_truth_model` |
| Doorway never fans out blob delivery to peers | `project_doorway_single_target_no_fanout` |
| Doorway presents identity, never owns it | `project_peer_native_account_canonical_surface` |
| The mesh is the hosting layer; doorway is optional projection | `project_p2p_is_hosting` |
| Routes register themselves via manifest; doorway is registry-driven | `project_doorway_manifest_driven_routes` |
| Inventory exchange is metadata-only; bytes single-target | `project_inventory_exchange_not_byte_replication` |

## What goes to hub

The `elohim-hub` directory at `elohim/elohim-hub/` (see its README for archetype framing — DwellingHub primary, CollectiveHub secondary) absorbs:

- Cross-hub threat coordination (the Cloudflare-class concern at federation scope)
- AbusePattern signal aggregation (`signal_kind` extension, future)
- Mobile device inference request processing (family edge-AI)
- Elohim_observer stream processing
- Workload state migration (PVCs / cluster rebalancing)
- Continuously-negotiated meet-and-protect compute contracts
- Multi-doorway failover (which doorway is healthy for a given human)
- Elohim-operator discernment at hub scope

These responsibilities use the **reach-earning** principle — already load-bearing at per-message authoring (memories `project_reach_earned_at_authoring`, `project_social_reach_nervous_system`) — extended to compute, distribution, defense, and AI-coordination at aggregate scale. A pattern shaped like a DDoS attack is structurally unearned-reach compute or distribution; the hub fabric doesn't engage with it because no node along the way has reason to spend cycles on it.

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

A household-scale dwelling on a residential connection cannot afford or operate a hyperscaler partnership; the protocol must remain credible at that scale (memory `project_subsume_g_f_a_via_it_just_works`).

## Forward path

The full design spec carries 10 stub-epic seeds. The ones most directly affecting doorway code:

- **SSR projection attestation** — does the rendered HTML carry a `ProjectionClaim` analogous to ATProto outbound? Sibling to `2026-05-01-computation-attestation-graduated-rigor-design.md`.
- **AbusePattern emission from doorway** — doorway observes and emits; hub aggregates patterns; federation propagates.
- **Wasm projection filters** — federation manifest declares projection logic per route; doorway runs the filters.
- **Federation-level doorway-to-doorway communication** — does this violate "doorway never swarms"? Probably not — federation flavors are HTTP/web2-shaped — but the rule needs explicit treatment when the case arises.

None of these are scheduled. They are seeds.
