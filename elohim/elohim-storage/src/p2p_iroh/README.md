# `p2p_iroh` — parallel iroh-based P2P stack

Sibling to [`crate::p2p`]. Gated by the `p2p-iroh` Cargo feature. Selected at
runtime by [`crate::config::TransportBackend::Iroh`] via the
`ELOHIM_TRANSPORT_BACKEND` env var.

The two stacks are **mutually exclusive at runtime** but compile additively
when both feature flags are set, so a single binary can host the parity-test
harness used during cutover.

## Architecture decision: cutover posture

**Phase 11 backend wiring is gated by the architecture spec at**
[`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`](../../../../genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md).

The spec lands the cutover posture as **partial replacement, dual-stack
permanent for most planes**, with a per-plane verdict table and a
decision rule for Phase 11 backend wiring. Summary:

| Plane | Verdict |
|---|---|
| Blob | iroh-canonical, libp2p-fallback (BLAKE3 chunked verified streaming is the protocol) |
| Gossip / Sync / EPR / EPR-atom / Shard / View-fed / Identity-handshake / Trust | dual-stack permanent (selected by transport-profile manifest; integrity from Track 1 DHT-notarized contracts + signed wire frames, not transport) |
| Reach-authorization | n/a — internal service, not a wire plane (feature canonical to the protocol; wire-composition design in-progress) |
| Discovery | dual-stack: pkarr-DHT (iroh side) + Kademlia (libp2p side); n0 demoted to one-of-many defaults |

**Why dual-stack permanent and not full-iroh:** consumer-grade devices
(intermittent laptops, browsers wanting direct WebRTC, UDP-restricted
networks) stay first-class substrate citizens via libp2p; iroh handles
hub-to-hub federation where its bench wins are real. The cross-stack
peer-map (Phase 10's `cross_stack_peer_map` migration) graduates from
transition-bridge to permanent structural schema. See the spec for the
anti-capture, anti-datacenter, and FANG-subsumption-via-federation
mechanisms this preserves.

**Hub is a role, not a hardware tier.** The dual-stack posture is also
what enables the hub graduation gradient — recycled laptops, gaming
desktops, composed thin-client batches, and other consumer-grade
hardware can act as hubs when that's the only option available, with
a graduation path through Tier-1-lightweight (Pi 4, NUC) and Tier-3
full DwellingHub (with local AI inference). Forfeiting consumer-grade
hub viability to one-transport simplification would forfeit the
protocol's onboarding funnel for hubs themselves.

When wiring a backend in Phase 11, the rule is:

1. Look up the plane's verdict in the spec.
2. If iroh-canonical: implement iroh primary + libp2p fallback.
3. If dual-stack permanent: implement both; selection via cross-stack peer-map.
4. If libp2p-canonical: libp2p primary + iroh ALPN registered for hub-to-hub.
5. NEVER hard-code transport choice or bypass the peer-map.

## What works (Phases 1–10 complete — cutover-ready transport)

All wire-protocol planes for the iroh transport are stood up. Backend
dispatch is supplied via trait objects; production daemon code wires
them at Phase 11 cutover.

**Foundation (Phases 1–2):**
- `IrohConfig` with disjoint paths (`iroh.key`, `blobs_iroh/`) + relay
  toggle (`use_n0_relays`) + discovery toggle (`use_n0_discovery`)
- `iroh::SecretKey` persisted at `<storage_dir>/iroh.key` (mode 0600 on Unix)
- `iroh::Endpoint` built with persisted identity, relay mode, and
  optional `discovery_n0()` (Phase 10)
- `IrohBlobStore` wrapping `iroh-blobs` filesystem store
- `IrohNode` aggregates endpoint + Router + store + gossip; mounts
  `BlobsProtocol` under [`iroh_blobs::ALPN`], iroh-gossip under
  [`iroh_gossip::ALPN`], plus arbitrary custom-ALPN extras supplied via
  `start_with_protocols`
- `add_bytes` / `get_bytes` / `has` (local) and `fetch_blob_from`
  (peer-to-peer via QUIC + verified BLAKE3 streaming)
- `ELOHIM_TRANSPORT_BACKEND=iroh` boot path lights up the iroh node and
  skips libp2p init (mutually exclusive)

**Custom-ALPN harness (Phase 3):**
- `codec.rs` length-prefixed MessagePack helpers (`read_frame` /
  `write_frame` / `read_frame_default`) + CBOR variants
  (`read_frame_cbor` / `write_frame_cbor` / `read_frame_cbor_default`)
- `parity_harness.rs` `TwoNodeFixture` (symmetric + asymmetric) for
  protocol parity tests
- Worked example: `tests/iroh_custom_alpn_echo.rs`

**Gossip plane (Phase 4):**
- `IrohGossip` wrapper with `topic_id_for(name)` deterministic
  BLAKE3(name)[..32] mapping
- `peer_blob_inventory.blake3_hash` Diesel migration (NULL during
  transition; Phase 11 drops the SHA256 `blob_hash`)
- `iroh_gossip_parity` two-node MessagePack inventory-delta round-trip

**Request/response ALPNs (Phases 5–9):**

| Phase | ALPN | Encoding | Backend trait | Wire types |
|---|---|---|---|---|
| 5 | `/elohim/sync/2.0.0` | MessagePack | `SyncBackend` | `crate::p2p::sync_protocol` |
| 6 | `/elohim/epr/2.0.0` | MessagePack | `EprBackend` | `crate::p2p::epr_protocol` |
| 6 | `/elohim/epr-atom/2.0.0` | CBOR | `EprAtomBackend` | `crate::p2p::epr_atom_protocol` |
| 7 | `/elohim/shard/2.0.0` | MessagePack | `ShardBackend` | `crate::p2p::shard_protocol` |
| 8 | `/elohim/view-federation/2.0.0` | MessagePack | `ViewFederationBackend` | `crate::views` |
| 9 | `/elohim/identity-handshake/2.0.0` | MessagePack | `IdentityHandshakeBackend` | `crate::p2p::identity_handshake` |
| 9 | `/elohim/trust/2.0.0` | MessagePack | `TrustBackend` | `crate::p2p::trust_protocol` |

Each is a `<plane>.rs` module with `Iroh<Plane>Protocol` (server) +
`Iroh<Plane>Client` (client) + `<Plane>Backend` trait. Wire types reused
unchanged from libp2p side — cutover removes one transport, never two
divergent message schemas.

**Discovery + cross-stack peer mapping (Phase 10):**
- `IrohConfig.use_n0_discovery` + `Endpoint::builder().discovery_n0()`
  replace libp2p Kademlia for peer record publication
- `cross_stack_peer_map` table (Diesel migration 2026-05-08-045024)
  bridges libp2p `PeerId` ↔ iroh `NodeId` for the same `agent_cid`
  during the hybrid window. `peer_map::{record_libp2p, record_iroh,
  iroh_for_libp2p, libp2p_for_iroh}` upsert + resolve helpers.

**Test coverage:** 26 unit + 19 integration tests (parity tests for every
plane + 4 peer-map tests). Run via `cargo test --features "p2p p2p-iroh"
--test 'iroh_*' -- --test-threads=1` (or `just test-iroh` which enforces
serialization for the integration binaries).

**Bench coverage:** 9 head-to-head perf benches (loopback, release; iroh
ALPN vs libp2p `/elohim/<plane>/1.0.0`). Each runs both REUSE
(handshake-amortized engine ceiling) and FRESH (handshake per request)
scenarios; prints two markdown tables; asserts the perf-bump on REUSE.
Run via `just bench-<plane>` or `just bench-all`. Headline p50 ratios
(iroh wins / libp2p p50) on a chatty plane sweep — full per-payload
tables are in each plane's commit body:

| Plane | REUSE p50 win | FRESH p50 win | Commit | Notes |
|---|---|---|---|---|
| Blob | 4×–290× | n/a (different bench shape) | `bd0a2f75` | iroh-blobs vs libp2p `BlobProtocol` |
| Gossip | publish→receive latency parity | n/a | `278c10c5` | iroh-gossip vs libp2p-gossipsub on inventory deltas |
| Sync | 45×–541× | 19×–50× | `7db8def2a` | small-frame `GetHeads` request/response |
| EPR | 25×–249× | 19×–34× | `996038198` | MessagePack `Resolve`/`Head` |
| EPR-atom | 25×–266× | 18×–25× | `1dadb8d01` | CBOR `Fetch`/`Atom` |
| Shard | 1.2×–128× | 1.3×–25× | `261a7ece7` | Reed-Solomon shard fetch; wins narrow on large payloads |
| View-fed | 59×–290× | 18×–58× | `3aacceec5` | edge-graph `GetEdges`/`Edges` |
| Identity-handshake | 207×–300× | 28×–52× | `ac58b6cf2` | binding presentation w/ signature payload sweep |
| Trust | 73×–306× | 31×–57× | `b9b579c45` | attestation list w/ CID-count sweep |

Pattern: iroh's small-frame multiplex wins by 100×+ on chatty protocols
in REUSE and stays 20–50× ahead in FRESH (handshake per request, the
shape `IrohSyncClient::request` ships today). Wins narrow as raw payload
climbs past ~1 MiB — at that point per-frame overhead stops dominating
and the comparison becomes throughput-bound.

A coincident structural finding from the bench expansion: every Phase
5–10 ALPN handler was originally one-stream-per-connection by design.
Production unaffected (clients always open fresh QUIC connections), but
the bench's stream-reuse pattern would silently hang. All 6 affected
handlers (sync, epr, epr-atom, shard, view-fed, identity-handshake,
trust) now loop on `accept_bi`, breaking on connection-closed error.
Existing parity tests still green; bench reuse + future Phase 11
connection pool both work.

## What is stubbed / deferred (Phase 11 cutover work)

- HTTP routes still read from the legacy SHA256-keyed
  [`crate::blob_store::BlobStore`]. Cutover graduates `/api/v1/blob/{hash}`
  to read from `IrohBlobStore` when in iroh mode.
- Genesis seeder still writes to the legacy store. Phase 11 rewrites it
  to write through `IrohBlobStore`.
- Phase 5–9 backends (`SyncBackend`, `EprBackend`, `EprAtomBackend`,
  `ShardBackend`, `ViewFederationBackend`, `IdentityHandshakeBackend`,
  `TrustBackend`) are trait objects supplied by the daemon. The iroh
  ALPNs accept connections and round-trip wire bytes correctly under
  test stubs, but no production daemon code yet supplies real backends.
  Cutover wires each one to its existing libp2p-side service.
- Phase 4 gossip topic broadcasters (inventory snapshot/delta publish,
  identity-binding, integrity-revocation, recovery-invitation,
  recovery-revocation, attention, feedback) are not yet wired into the
  daemon's existing libp2p-side broadcast call sites. Topic mapping
  (`IrohGossip::topic_id_for`) is the only piece needed; the per-topic
  publish wiring graduates per protocol at cutover.
- Identity-map (`crate::p2p::identity_map`) and reach-authorization
  (`crate::p2p::reach_authorization`) are internal services, not wire
  protocols — they consume identity context from the handshake plane.
  No separate ALPNs.
- `IrohNode` is held in `main.rs` for lifetime but not yet driven through
  the `tokio::select` shutdown loop. Drop-on-exit is fine for now;
  cutover will integrate explicit `IrohNode::shutdown` for parity with
  libp2p graceful shutdown.

## How to run

```bash
# build with both stacks compiled in
cd elohim/elohim-storage
just build-iroh

# run with iroh stack selected at runtime (libp2p stays untouched)
ELOHIM_TRANSPORT_BACKEND=iroh just run

# unit + integration tests
just test-iroh
```

## Address format (cutover constraint)

Two canonical content-address formats coexist during cutover:

| Stack | Format | Path |
|---|---|---|
| libp2p (default) | SHA256 / CIDv1 (`bafkrei...`) | `<storage_dir>/blobs/` |
| iroh | BLAKE3 (raw 64-hex) | `<storage_dir>/blobs_iroh/` |

Each is canonical within its mode; runtime config selects exactly one mode.
Convergence on BLAKE3 happens after the cutover gate (Phase 11). Until
then, do not write tools that expect to read both formats interchangeably
within a single mode.

## Pinning

`iroh = "=0.92"` + `iroh-blobs = "=0.94"` + `iroh-gossip = "=0.92"`
(all pulled with stable ed25519-dalek 2.2 + curve25519-dalek 4.1;
iroh-blobs 0.95+ moves to a broken pre-release crypto path). Coexists
with current `holochain_client 0.9.0-dev.5` — no holochain bump
required. See plan for the version-walk rationale.

## Phase 11 cutover prerequisites

Wire transport for every plane is in place. Remaining work for the
cutover gate is application-layer wiring + soak validation:

1. **Backend wiring** — replace stub backends in production code paths
   so daemon services (sync engine, EPR resolver, atom store, shard
   service, view-fed dispatcher, identity verifier, trust verifier)
   route through the iroh ALPNs when `transport_backend = Iroh`.
2. **HTTP route graduation** — `/api/v1/blob/{hash}` reads from
   `IrohBlobStore` in iroh mode; format-discriminated path or BLAKE3-
   accepting variant.
3. **Genesis seeder rewrite** — write to `IrohBlobStore` instead of
   legacy `BlobStore`.
4. **Gossip topic broadcast wiring** — per-topic publish call sites
   route through `IrohGossip::subscribe`/broadcast in iroh mode.
5. **Recovery e2e** — full social-recovery flow runs over iroh ALPNs
   end-to-end.
6. **CI parity soak** — nightly run of every parity test for a week
   with zero divergences.
7. **Alpha-cluster soak** — 6-peer cluster runs in iroh mode for one
   week without regression; inventory delta convergence within target.
8. **Latency stress test** — 10k blob round-trips between two nodes,
   p99 ≤ libp2p baseline.
9. **Rollback drill** — flip default to `Iroh`, run, flip back to
   `Libp2p` via env override, smoke pass; document playbook.
10. **Column-drop migration** — `peer_blob_inventory.blob_hash` drop
    written, tested, gated behind a post-cutover follow-up release.

## Why explicit Connection in `fetch_blob_from`

`IrohNode::fetch_blob_from` does NOT use `iroh_blobs::api::downloader::Downloader`.
The Downloader's connection pool calls `endpoint.connect(node_id, alpn)`,
which strips direct addresses (a NodeId by itself converts to an empty
NodeAddr) and falls back to discovery. With `RelayMode::Disabled` and no
discovery service configured, that fails — observed in CI/loopback during
Phase 2 development.

The fix mirrors iroh-blobs' own canonical two-node test pattern:

```rust
let conn = endpoint.connect(peer_node_addr, iroh_blobs::ALPN).await?;
store.remote().fetch(conn, hash).await?;
```

— passing the **full** `NodeAddr` (with `direct_addresses` populated by
`node_addr().initialized()`) so connection resolution doesn't need
discovery.

When/if Phases 4+ introduce iroh-discovery (DNS, pkarr-DHT, mDNS), the
Downloader path becomes viable for swarm-style fetches that don't have
addresses upfront. For Phase 2's targeted peer-to-peer transfers, explicit
connection is the right shape.

## Graduation gates

### Phases 2 → 10 (transport scaffolding) — GREEN

- [x] `just build-iroh` clean
- [x] `just test-iroh` green (26/26 unit + 19/19 integration when
      serialized via `--test-threads=1`)
- [x] All ten plane parity tests pass:
      `iroh_blob_roundtrip`, `iroh_node_lifecycle`, `iroh_custom_alpn_echo`,
      `iroh_gossip_parity`, `iroh_sync_parity`, `iroh_epr_parity`,
      `iroh_shard_parity`, `iroh_view_fed_parity`, `iroh_auth_parity`,
      `iroh_peer_map`
- [x] `cargo check` (default features, libp2p-only) clean
- [x] `peer_blob_inventory.blake3_hash` migration applies + reverses
- [x] `cross_stack_peer_map` migration applies + reverses
- [ ] CI runs both feature combos at least once on the worktree branch
      (verified locally; CI verification on next push)

### Phase 11 cutover gate (NEXT)

See plan section "Phase 11: Cutover gate" for the full criteria list and
"Phase 11 cutover prerequisites" above for what's still required: backend
wiring, HTTP route graduation, seeder rewrite, gossip publish wiring,
recovery e2e, CI parity soak, alpha-cluster soak, latency stress test,
rollback drill, column-drop migration.

## Code map

| File | Purpose |
|---|---|
| `mod.rs` | Module root + re-exports |
| `config.rs` | `IrohConfig` (paths, relay/discovery toggles) |
| `identity.rs` | Persisted `SecretKey` (file-mode 0600) |
| `endpoint.rs` | `Endpoint` builder (relay + discovery wiring) |
| `blob_store.rs` | `IrohBlobStore` over iroh-blobs `FsStore` |
| `node.rs` | `IrohNode` aggregate (Endpoint + Router + store + gossip) |
| `codec.rs` | Length-prefixed MessagePack + CBOR frame helpers |
| `parity_harness.rs` | `TwoNodeFixture` for parity tests |
| `gossip.rs` | `IrohGossip` wrapper + topic-id mapping (Phase 4) |
| `sync.rs` | Sync ALPN handler + client (Phase 5) |
| `epr.rs` | EPR + EPR-atom ALPN handlers + clients (Phase 6) |
| `shard.rs` | Shard ALPN handler + client (Phase 7) |
| `view_fed.rs` | View-federation ALPN handler + client (Phase 8) |
| `auth.rs` | Identity-handshake + trust ALPN handlers + clients (Phase 9) |
| `peer_map.rs` | Cross-stack `PeerId` ↔ `NodeId` bridge (Phase 10) |

Tests under `tests/`: `iroh_blob_roundtrip`, `iroh_node_lifecycle`,
`iroh_custom_alpn_echo`, `iroh_gossip_parity`, `iroh_sync_parity`,
`iroh_epr_parity`, `iroh_shard_parity`, `iroh_view_fed_parity`,
`iroh_auth_parity`, `iroh_peer_map`.

## See also

- Plan: `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`
- Memory: `project_iroh_parallel_stack_phase0_blocker.md`,
  `feedback_cargo_resolution_vs_compilation.md`,
  `feedback_subagent_dep_conflict_supervision.md`
