---
id: "backlog-sync-edge-susan-timeouts-per-edge-observability"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "One peer (susan) is the fleet's expensive sync edge — every pod times out to her and she times out to everyone — and nothing in the dataplane prices that edge"
slug: "sync-edge-susan-timeouts-per-edge-observability"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid"
status: "open"
priority: "high"
jobs: [elohim-edge]
---

## Measured (Loki, alpha, 6 h ending 2026-08-28T03:15Z)

`{namespace="elohim-alpha"} |= "Outbound sync request failed" | json` summed by `(pod, fields_peer, fields_error)`:

- **Everyone → susan** (`12D3KooWQQsHAgfyWzT6…`, `elohim-susan-alpha-0`, node shem) `Timeout`: adam 22 · eve 24 · gertrude 31 · jessica 23 · matthew 15 · james 2.
- **susan → everyone** `Timeout`: 25 (CGiw…), 24 (GPmV…), 19 (GNZr… = elohim.host doorway), 10 (BS8a…), 8 (QAaK… = doorway-alpha).
- Fleet total 274 outbound sync failures / 6 h: susan 93, gertrude 53, jessica 38, eve 32, adam 27, matthew 18, james 13.
- susan is not memory-starved: working set 753 MB of a 3 GiB limit (shem-node pods run 3 CPU / 3 GiB; adam 8/8; matthew 4/8). Cause unknown — the shem slow-link class (history: adam slow-link write-guard saturation) is the prior.

## Why this is a trust/performance finding, not only an ops one

`elohim_sync_request_outcomes_total` is `result`-only by design (a peer-id label would be unbounded), so this edge is invisible in Prometheus and identical to a healthy edge for every sync decision: `DEFAULT_FETCH_WINDOW = 32` is global, `replication_schedule.rs` backs off on failure only, and no timeout varies per peer. The canon (`trust-as-efficiency-signal`) says cost should scale with trust; today it cannot even be *observed* per relationship. The shift's landed instrument: a bounded `peer_class` label (the peer's cached `reach_ceiling` ∪ {other, unverified}) on the sync outcome counters — see the storage commit in the same push.

## Next

1. Once `peer_class` is on the fleet, the question "do trusted edges fail less?" becomes a one-line PromQL.
2. Diagnose susan: conductor saturation vs libp2p transport (relay/NAT between shem and ethosengine nodes) — `/db/p2p/conductor-diagnostics`, susan's conductor logs, iroh vs libp2p lane split for her edges.
3. Design (not tonight): per-peer request timeout / window from observed edge health + standing (pull-queue spec R-F, second half).

## First live read of `peer_class` (household mesh, 2026-08-28T04:25Z, binaries at 0fdbbd285)

All three peers: `elohim_sync_request_outcomes_total{peer_class="public",result="ok"}` 48/46/44, `{unverified}` 0. The trust handshake ran for every edge and cached `reach_ceiling = public` for **household members**. So the instrument distinguishes nothing yet — matthew↔jessica price as strangers. Before any per-peer window/timeout can be trust-keyed, the handshake's `VerifiedTrustContext.reach_ceiling` must reflect the relationship the DHT already holds (`HumanRelationship` / household `Membership`), or `peer_class_for` should derive from `verified_relationships`/`verified_memberships` rather than the ceiling alone. That is the next bounded leg of this entry.

### Why every peer prices as `public` — the ambient trust handshake is a stub (grounded 2026-08-28T04:35Z)

- Sender (`p2p/mod.rs:5151`): `TrustHandshake { agent_pubkey: self.identity.peer_id().to_string(), membership_cids: vec![], relationship_cids: vec![], attestation_cids: vec![], stewardship_cids: vec![] }` — it presents its **libp2p peer id** as the agent key (wrong namespace — see elohim-storage CLAUDE.md "Identity & Transport-Identity Coherence") and no credentials.
- Receiver (`p2p/mod.rs:6531-6541`): builds `VerifiedTrustContext { agent_verified: true, reach_ceiling: "public", verified_*: vec![] }` straight from the request — "conductor integration fills stubs" — and answers `Verified`. `trust_verification::verify_trust_context(hc_client, credentials)` (the real verifier: DHT reads per CID + `calculate_reach_ceiling`, unit-tested) has **zero callers**, and `P2PNode` holds no `HcClient` to call it with.
- Chain / between "peer_class label" → "trusted edges cost less" / missing node: **the handshake carries a verifiable agent key + relationship/membership CIDs, and the receiver verifies them against its own DHT view**. Blocked-by: the transport-id→agent_cid resolver is not built (bindings self-asserted, `AgentPeerBinding` never emitted by an edge node). Until then the trust gradient has no live input on the dataplane; `peer_class` will read `public` for everyone — which is now an honest, visible fact instead of an invisible one.
