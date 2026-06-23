---
id: "backlog-wan-nat-federation-dataplane-discovery-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "WAN-NAT discovery gap across the p2p layer (Holochain DHT conductor + iroh/libp2p blob stores) and the federation/fediverse layer (doorways, over p2p) — k8s in-cluster DNS is a dev-test fixture, never the architecture"
slug: "wan-nat-federation-dataplane-discovery-gap-2026-06-23"
written: "2026-06-23"
author: "oAuth+federation shakeout — 7-agent WAN-NAT gap workflow (wf_0c940c53)"
status: "open"
priority: "medium"
tags: [federation, p2p, wan-nat, libp2p, iroh, doorway, dataplane, jwks, transport-backend, dev-test-fixture, k8s-is-not-the-architecture]
relatedNodeIds:
  - backlog-edge-deploy-2539797f-conductor-selfheal-device-rollout
cites:
  - doorway/doorway-service/src/services/federation.rs
  - elohim/elohim-storage/src/p2p/behaviour.rs
  - elohim/elohim-storage/src/p2p_iroh/endpoint.rs
  - doorway/doorway-service/src/auth/jwt.rs
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - genesis/orchestrator/manifests/doorway/alpha.yaml
---

# WAN-NAT discovery gap — p2p substrate + federation/fediverse layer

**Vocabulary (architect, 2026-06-23):** *p2p* = the **substrate** — the Holochain DHT
(conductor) + the iroh/libp2p blob stores. *Federation / fediverse* = the **doorways**, a
layer **over** p2p. Doorway peer-discovery **rides the p2p layer** (DHT-gossiped
`DoorwayRegistration` entries) — it is NOT a separate direct doorway-to-doorway mechanism.
([[feedback_p2p_vs_federation_layer_vocabulary]])

The operator-visible symptom (`/threshold/doorways` showed only self) was the
**federation/fediverse** face of a cross-cutting requirement: **both p2p and federation
must register/see each other over WAN-NAT.** The hazard this doc guards against is "fixing"
it with a co-location mechanism (k8s in-cluster DNS) and declaring victory — k8s gaps ≠
protocol gaps ([[feedback_k8s_is_not_the_architecture]]).

## WAN-NAT readiness by layer (workflow wf_0c940c53, 2026-06-23)

### p2p layer (substrate)
| Plane | Readiness | Evidence |
|---|---|---|
| **Holochain DHT gossip** (conductor, kitsune2/tx5) | ✅ **WAN-native, exercised now** — the reference posture | public `bootstrap_url`/`signal_url`, `enable_relaying`, STUN; shared `elohim-bootstrap` table; peerCount ~13 |
| **iroh / libp2p blob stores** (byte replication) | ❌ **pinned to in-cluster / dormant** | below |

### federation / fediverse layer (doorways, over p2p)
| Plane | Readiness | Evidence |
|---|---|---|
| **Doorway peer-discovery** (the `/threshold/doorways` selector) | ✅ **LANDED + verified live 2026-06-23** — DHT-native `get_all_doorways` (coordinator-only) **rides the p2p DHT**; selector now lists both doorways (alpha + apex) | `services/federation.rs` stub replaced; `infrastructure` zome `__all__` list-all anchor (b730d3302) |
| **Cross-doorway JWT/JWKS validation** | ❌ **unimplemented** (HMAC shared secret, not Ed25519/JWKS) | below |

## What remains (this backlog item)

1. **Storage libp2p dataplane is built-but-pinned-to-in-cluster.** relay/DCUtR/AutoNAT
   behaviours are compiled (`p2p/behaviour.rs`) but there is **no relay-reservation dial**
   (only the server-side `ReservationReqAccepted` counter), no `ANNOUNCE_ADDRS`, and
   `P2P_BOOTSTRAP_NODES` is all `*.svc.cluster.local` (`manifests/doorway/alpha.yaml`,
   `matthew-manager.yaml`). Byte replication therefore cannot traverse WAN-NAT today.
2. **iroh transport is wired but dormant — and gated behind dead config.** `p2p_iroh/endpoint.rs`
   sets `RelayMode::Default` + pkarr correctly, but the alpha manifest sets
   `TRANSPORT_BACKEND: "dual-stack"` while the code reads **`ELOHIM_TRANSPORT_BACKEND`** and
   `"dual-stack"` is **not a valid `TransportBackend` variant** (`config.rs` has only
   `Libp2p`/`Iroh`) — so the backend silently stays `Libp2p` and iroh never activates. The
   iroh cutover is independently gated (needs ≥3 operator-self-hosted pkarr resolvers reachable
   from alpha — `2026-05-08-iroh-libp2p-complementarity.md` §cutover; "Defaulting is acceptable;
   only-ing is not"). Fix the env-name/value drift as a precondition, not the cutover itself.
3. **Cross-doorway JWT validation is not implemented (WAN-federation security).** `doorway/CLAUDE.md`
   claims a receiving doorway verifies issuer tokens against `/.well-known/doorway-keys` (JWKS),
   but `JwtValidator::verify_token` (`auth/jwt.rs`) uses `DecodingKey::from_secret(...)` — an HMAC
   **shared secret**, not Ed25519/JWKS. `/.well-known/doorway-keys` is **served but never
   consumed**; there is no JWKS client. Cross-doorway auth over WAN cannot work until this lands.

## Dev-test-fixture stance (the guard)

Co-locating the alpha pair in one k8s namespace lets in-cluster Service DNS *substitute* for
WAN reachability. That is acceptable **only** as an explicitly-labeled dev-test fixture for
orchestrating the runtime model — never as the architecture, and never silently. The
doorway-federation fix shipping now deliberately does **not** introduce a
`FEDERATION_PEERS → svc.cluster.local` repoint: it rides the already-WAN-healthy conductor DHT
plane instead, so it needs no in-cluster crutch. If a future co-located demo genuinely needs one,
the only admissible form is a labeled `svc.cluster.local` repoint carrying a "remove when the
WAN path lands" note.

## Authoritative design

`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`
(NOT the non-existent `2026-04-19-self-healing-p2p-dataplane-design.md` referenced elsewhere —
correct stale cites on sight).
