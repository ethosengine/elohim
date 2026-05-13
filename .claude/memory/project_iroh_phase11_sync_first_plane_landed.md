---
name: iroh Phase 11 — sync is first wired plane (pattern set for 5 more)
description: SyncManagerBackend adapter wires SyncManager into iroh ALPN dispatch; main.rs iroh branch builds DocStore + StreamTracker + SyncManager directly (mode-exclusive, opens same sync.sled as libp2p path); pattern reusable for EPR/EPR-atom/shard/view-fed/identity-handshake/trust
type: project
originSessionId: e7e20120-789e-4174-af38-3822fb5a869e
---
Sync is the first Phase 11 plane wired (commit `c0778907f`). The pattern for the remaining 5 backends:

1. Adapter struct in `src/p2p_iroh/<plane>_backend.rs` wraps `Arc<RealService>` and impls the plane's `<Plane>Backend` trait.
2. Mirror the libp2p-side handler line-for-line — wire bytes are the contract, transport varies.
3. main.rs's iroh branch (in `_iroh_node` builder block) constructs the real service, wraps in adapter, registers under `<PLANE>_ALPN` via `IrohNode::start_with_protocols(extras)`.
4. Add `tests/iroh_<plane>_real_backend.rs` — companion to existing `iroh_<plane>_parity` (stub backend). Real test should exercise an end-to-end write→read cycle so wire→backend→storage is provably plumbed.

**Why per-mode service construction works**: modes are mutually exclusive at runtime (libp2p OR iroh, never both), so iroh-mode constructs its own SyncManager opening the same `sync.sled` directory the libp2p path uses. No lock contention because only one mode is ever loading.

**Why this is NOT the dual-stack-permanent end state**: the complementarity spec (`2026-05-08-iroh-libp2p-complementarity.md`) classifies sync as dual-stack permanent with selection per-call via cross-stack peer-map. This commit ships the iroh-side dispatch only; selection-by-manifest is later sprint work.

**How to apply**: When wiring EPR backend next, copy `sync_backend.rs` shape — `EprManagerBackend { resolver: Arc<EprResolver> }` impl `EprBackend`, mirror the libp2p `handle_epr_request` site, register `IrohEprProtocol::new(backend)` under `EPR_ALPN` in main.rs alongside the sync registration.
