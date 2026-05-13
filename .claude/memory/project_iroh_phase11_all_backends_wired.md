---
name: iroh Phase 11 — all six remaining backends wired (cutover gate #1 closed)
description: Sync had landed earlier; this push wired EPR, EPR-atom, Shard, View-fed, IdentityHandshake, Trust as transport-neutral services + iroh adapters. All 16 iroh integration test binaries pass (43 tests, 0 failures). Phase 11 prerequisite #1 closed; #2-#10 (HTTP/seeder/gossip/recovery/soaks/pkarr/rollback) remain.
type: project
originSessionId: c95a6363-756d-496f-a85f-6793fc47ce42
---
Worktree `worktree-iroh-parallel-stack` (commits stacked on `dev` after the bench expansion). Power-outage pickup, 2026-05-09: extracted six transport-neutral services and registered iroh-side adapters under their ALPNs alongside the existing sync wiring.

**Pattern set across all six new planes (mirrors `sync_backend.rs` from earlier):**

| Plane | Service | Adapter | Commit |
|---|---|---|---|
| EPR | `src/epr_service.rs` (read-side: Resolve/ResolveBatch/GetDocument/QueryDelivery; Announce kept libp2p-only pending pkarr) | `src/p2p_iroh/epr_backend.rs` | `9b5e795ef` |
| EPR-atom | `src/epr_atom_service.rs` (CBOR decode → CID dedup → ingest → reach-gate; takes `CallerIdentity` arg, iroh defaults to Anonymous) | `src/p2p_iroh/epr_atom_backend.rs` | `5180d7bc8` |
| Shard | `src/shard_service.rs` (Get/Have/Push via `BlobStore`; ListContent/GetContent via `db_pool` with reach_filter validation) | `src/p2p_iroh/shard_backend.rs` | `2ab760bda` |
| View-fed | `src/view_fed_service.rs` (wraps `build_response_slice`; `libp2p_keypair_from_ed25519_bytes` converts iroh SecretKey to libp2p Keypair so signatures byte-identical) | `src/p2p_iroh/view_fed_backend.rs` | `615190cdf` |
| Identity-handshake + Trust | `src/identity_handshake_service.rs` + `src/trust_service.rs` | `src/p2p_iroh/auth_backends.rs` | `b36f4c27a` |

**P2PNode delegation method** added per service (e.g. `epr_service()`, `epr_atom_service()`, `shard_service()`) — each is a cheap snapshot constructor that clones the relevant Arc/Option fields. The libp2p inline handlers now delegate; wire bytes unchanged.

**main.rs iroh branch** registers all 9 ALPNs: iroh-blobs (auto), iroh-gossip (auto), `/elohim/sync/2.0.0`, `/elohim/epr/2.0.0`, `/elohim/epr-atom/2.0.0`, `/elohim/shard/2.0.0`, `/elohim/view-federation/2.0.0`, `/elohim/identity-handshake/2.0.0`, `/elohim/trust/2.0.0`.

**Acceptance:** `cargo test --features p2p,p2p-iroh --test 'iroh_*' -- --test-threads=1` followed by the 5 binaries the glob missed (`iroh_auth_parity`, `iroh_auth_real_backend`, `iroh_blob_roundtrip`, `iroh_custom_alpn_echo`, `iroh_epr_atom_real_backend`) — all 16 binaries green, 43 tests, 0 failures.

**Phase 12 graduations the iroh-side adapters explicitly defer to (each has documented stopgap behavior):**

- **EPR Announce** in iroh mode returns `Announced { accepted: false, reason: "...not yet wired (n0 mitigation steps 1–4)" }` — pkarr / iroh-gossip identity-binding lands per the complementarity spec's roadmap.
- **EPR-atom caller identity** defaults to `CallerIdentity::Anonymous` in iroh mode; cross-stack peer-map's `peer_transport_manifest` projection (Phase 12) supplies the iroh-NodeId → agent_cid mapping.
- **View-fed `connected_peers`** is empty in iroh mode pending the same peer-manifest graduation.
- **Trust cache** (libp2p-PeerId-keyed) is intentionally not populated by the iroh adapter; iroh-mode reach-auth falls through to slow-path DB lookups (correct, just not ambient-cached).
- **Identity-handshake peer label** uses the binding's claimed `peer_id` until ProtocolHandler::accept threads connecting-peer NodeId to the handler.

**Phase 11 prerequisite #1 (backend wiring) is closed. Remaining cutover gates** per `2026-05-08-iroh-libp2p-complementarity.md` §"Cutover gate (revised)":

2. HTTP `/api/v1/blob/{hash}` graduation to dual-format reads.
3. Genesis seeder dual-write (BLAKE3 + SHA256).
4. Gossip topic broadcast wiring through both transports for inventory + identity-binding + recovery topics.
5. Recovery e2e harness on iroh.
6. CI parity soak (1 week, zero divergence).
7. Alpha-cluster dual-stack soak (1 week, 6 peers).
8. Latency stress (10k blob round-trips on iroh, p99 ≤ libp2p baseline).
9. **Consumer-grade soak (NEW per spec)** — iroh on phone over cellular, Chromebook over school Wi-Fi, residential CGN. Failure on any plane keeps that plane libp2p-canonical for that device class permanently.
10. Self-hostable pkarr resolver in production.
11. Rollback drill playbook.
12. Column-drop migration (`peer_blob_inventory.blob_hash`) — NOTE: stays per spec for libp2p-fallback peers; the migration is a no-op rename, not a drop.

**How to apply:** Phase 11 backend pattern is locked. The peer-transport-manifest graduation (Phase 12) is the single biggest remaining cleanup — once it lands, all five "stopgap" footnotes above resolve cleanly. Don't pre-emptively re-architect the adapters; their shape is right for the manifest.
