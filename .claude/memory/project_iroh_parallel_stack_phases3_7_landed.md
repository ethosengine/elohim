---
name: iroh parallel stack — Phases 1–10 landed (cutover-ready transport)
description: Wire transports for every plane (blob, gossip, sync, EPR×2, shard, view-fed, auth×2) proven via iroh QUIC parity tests; cross-stack peer-map bridge in place; Phase 11 cutover gate now reachable
type: project
originSessionId: bf3ce047-b0fb-41e7-a37a-fccd45eccad5
---
Worktree `worktree-iroh-parallel-stack` (rebased onto origin/dev 2026-05-08, 22 commits ahead).

**Done (all wire transports, parity-tested):**

| Phase | What | Tests |
|---|---|---|
| 1 | Foundation (deps, config, identity, endpoint, TransportBackend selector) | unit |
| 2 | Blob plane via iroh-blobs (IrohBlobStore + Router + two-node round-trip) | 2 integration |
| 3 | Custom-ALPN harness (codec.rs MessagePack + CBOR; parity_harness.rs TwoNodeFixture; AlpnRegistration + start_with_protocols) | 2 integration (echo) |
| 4 | iroh-gossip plane (IrohGossip wrapper, BLAKE3 topic_id mapping, mounted on Router under iroh_gossip::ALPN) + peer_blob_inventory.blake3_hash Diesel migration | 1 integration |
| 5 | Sync ALPN /elohim/sync/2.0.0 (IrohSyncProtocol/Client + SyncBackend, MessagePack) | 1 integration |
| 6 | EPR ALPNs /elohim/epr/2.0.0 (MessagePack) + /elohim/epr-atom/2.0.0 (CBOR), with EprBackend / EprAtomBackend | 2 integration |
| 7 | Shard ALPN /elohim/shard/2.0.0 (IrohShardProtocol/Client + ShardBackend, MessagePack) | 2 integration |
| 8 | View-federation ALPN /elohim/view-federation/2.0.0 (256 KiB cap, ViewFederationBackend) | 1 integration |
| 9 | Identity-handshake + trust ALPNs (/elohim/identity-handshake/2.0.0 + /elohim/trust/2.0.0). identity-map + reach-authorization are internal services (no ALPN) | 2 integration |
| 10 | Discovery (use_n0_discovery flag → Endpoint::builder().discovery_n0()) + cross_stack_peer_map Diesel migration + peer_map module bridging libp2p PeerId ↔ iroh NodeId via agent_cid | 4 integration (peer_map) |

Acceptance: 26/26 p2p_iroh unit + 19/19 iroh integration when run via `cargo test --test 'iroh_*' -- --test-threads=1` (parallel races for UDP loopback ports under contention; serialized run is reliable). justfile test-iroh enforces this.

**Pin update:** `iroh-gossip = "=0.92"` added — pairs cleanly with iroh 0.92 + iroh-blobs 0.94.

**Wire pattern (universal across Phases 5–9):** Each plane = ALPN const + ProtocolHandler + Client helper + Backend trait, framed via `super::codec::{read_frame_default, write_frame}` (or `_cbor` variants). One bidi stream = one request/response. Backends are trait objects supplied by the daemon at cutover.

**Phase 11 cutover prerequisites (still required):**
1. Backend wiring — replace stub backends so daemon services route through iroh ALPNs in iroh mode (sync engine, EPR resolver, atom store, shard service, view-fed dispatcher, identity verifier, trust verifier)
2. HTTP route graduation — `/api/v1/blob/{hash}` reads from `IrohBlobStore` in iroh mode
3. Genesis seeder rewrite — write to `IrohBlobStore` instead of legacy `BlobStore`
4. Gossip topic broadcast wiring — per-topic publish call sites route through `IrohGossip` in iroh mode (inventory, identity-binding, integrity-revocation, recovery-invitation, recovery-revocation, attention, feedback)
5. Recovery e2e — full social-recovery flow runs over iroh ALPNs
6. CI parity soak — nightly run of every parity test for one week with zero divergences
7. Alpha-cluster soak — 6-peer cluster runs in iroh mode for one week
8. Latency stress — 10k blob round-trips, p99 ≤ libp2p baseline
9. Rollback drill — flip default + run + flip back via env override; document playbook
10. Column-drop migration — `peer_blob_inventory.blob_hash` drop, gated behind a post-cutover follow-up

**Diesel migrations added:**
- `2026-05-08-033248_peer_blob_inventory_blake3_hash` — adds blake3_hash column (NULL during transition)
- `2026-05-08-045024_cross_stack_peer_map` — bridge table (agent_cid PK, peer_id NULL, node_id NULL)

**How to apply:** When picking up Phase 11, the wire transport is locked. Cutover work is in the daemon's existing service code paths — find each libp2p call site and add a `match config.transport_backend` branch routing to the iroh client. Don't re-architect; the iroh side is meant to be a drop-in. Backend wiring is mechanical; recovery e2e + alpha soak + latency stress are the substantive validation gates.

**Bench coverage addendum (2026-05-09):** All 9 planes (blob, gossip, sync, EPR, EPR-atom, shard, view-fed, identity-handshake, trust) now have head-to-head perf benches with two scenarios (REUSE = engine ceiling, FRESH = handshake per request). Verdict: iroh wins decisively on every chatty plane (45×–541× p50 in REUSE, 18×–58× in FRESH), narrows to 1.2×–1.4× on bulk transfer (shard at 1+ MiB) where per-frame overhead stops dominating. `just bench-all` runs all 9 in sequence; numbers in the README's "Bench coverage" subsection point at per-plane commits. Coincident finding: every Phase 5–10 ALPN handler was originally one-stream-per-connection; production unaffected, bench reuse and any Phase 11 connection pool now both work via accept_bi loop (commit `7db8def2a`).
