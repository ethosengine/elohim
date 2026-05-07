# `p2p_iroh` — parallel iroh-based P2P stack

Sibling to [`crate::p2p`]. Gated by the `p2p-iroh` Cargo feature. Selected at
runtime by [`crate::config::TransportBackend::Iroh`] via the
`ELOHIM_TRANSPORT_BACKEND` env var.

The two stacks are **mutually exclusive at runtime** but compile additively
when both feature flags are set, so a single binary can host the parity-test
harness used during cutover.

## What works (Phase 1 + 2 complete)

- `IrohConfig` with disjoint paths (`iroh.key`, `blobs_iroh/`)
- `iroh::SecretKey` persisted at `<storage_dir>/iroh.key` (mode 0600 on Unix)
- `iroh::Endpoint` built with persisted identity + relay-mode toggle
- `IrohBlobStore` wrapping `iroh-blobs` filesystem store
- `IrohNode` aggregates endpoint + Router + store, with `BlobsProtocol`
  mounted under [`iroh_blobs::ALPN`]
- `add_bytes` / `get_bytes` / `has` (local) and `fetch_blob_from`
  (peer-to-peer via QUIC + verified BLAKE3 streaming)
- `ELOHIM_TRANSPORT_BACKEND=iroh` boot path lights up the iroh node and
  skips libp2p init (mutually exclusive)
- Two-node loopback integration test (`tests/iroh_blob_roundtrip.rs`)

## What is stubbed / deferred

- HTTP routes still read from the legacy SHA256-keyed
  [`crate::blob_store::BlobStore`]. Cutover (Phase 11) graduates them.
- Genesis seeder still writes to the legacy store.
- `peer_blob_inventory` table has no BLAKE3 column yet (Phase 4).
- No gossip / sync / shard / EPR / view-fed / identity protocols on the
  iroh path yet — those are Phases 3–9 in the plan.
- `IrohNode` is held in `main.rs` for lifetime but not yet driven through
  the `tokio::select` shutdown loop. Drop-on-exit is fine for now; Phase
  3+ will integrate explicit `IrohNode::shutdown` for parity with libp2p
  graceful shutdown.

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

`iroh = "=0.92"` + `iroh-blobs = "=0.94"` (both pulled with stable
ed25519-dalek 2.2 + curve25519-dalek 4.1; iroh-blobs 0.95+ moves to a
broken pre-release crypto path). Coexists with current
`holochain_client 0.9.0-dev.5` — no holochain bump required. See plan
for the version-walk rationale.

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

Each gate must be green before the next phase picks up.

### Phase 2 → Phase 3 (custom-ALPN harness)

- [x] `just build-iroh` clean
- [x] `just test-iroh -- p2p_iroh` green (13/13 unit)
- [x] `tests/iroh_blob_roundtrip.rs` green (2/2 integration)
- [x] `tests/iroh_node_lifecycle.rs` green (2/2 integration)
- [x] `cargo check` (default features, libp2p-only) clean
- [x] `cargo clippy --features "p2p p2p-iroh" --lib -- -D warnings` clean
      for new code (pre-existing `constitution` + `ts-rs` noise unaffected)
- [ ] CI runs both feature combos at least once on the worktree branch
      (verified locally; CI verification on next push)

### Phase 11 cutover gate

See plan section "Phase 11: Cutover gate" for the full criteria list (parity
soak, alpha-cluster validation, latency parity, recovery e2e, seeder
graduation, HTTP route graduation, rollback drill, column-drop migration).

## See also

- Plan: `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`
- Memory: `project_iroh_parallel_stack_phase0_blocker.md`,
  `feedback_cargo_resolution_vs_compilation.md`,
  `feedback_subagent_dep_conflict_supervision.md`
