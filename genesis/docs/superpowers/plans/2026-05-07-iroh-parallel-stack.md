# iroh Parallel P2P Stack — Staged Cutover from libp2p

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a parallel iroh-based P2P module inside `elohim-storage` that, by the end of the plan, can serve every existing libp2p protocol over iroh's QUIC transport and become the default. Existing libp2p stack remains the runtime default until the cutover gate clears. The migration is staged protocol-by-protocol so each plane can be exercised, parity-tested against libp2p, and graduated independently.

**Architecture:** A new `src/p2p_iroh/` module sits alongside `src/p2p/`, gated by `p2p-iroh` Cargo feature. Both backends compile additively; runtime config (`TransportBackend::Libp2p` | `TransportBackend::Iroh`) selects one at startup. Wire formats (MessagePack frames defined in `crate::p2p::wire`) are *shared* across stacks so the cutover doesn't fork message schemas — only the transport. Where iroh has a native protocol (blobs via `iroh-blobs`, gossip via `iroh-gossip`), we adopt it; where we have bespoke protocols (sync, shard, EPR, view federation, identity handshake, trust, reach), we register custom ALPNs on iroh's `Router` and reuse our existing wire types.

**Tech Stack:**
- `iroh = "=0.96"` (pulled transitively by iroh-blobs/iroh-gossip; pinned for stability)
- `iroh-blobs = "=0.98"` (BLAKE3-keyed content-addressed blob protocol — Phase 2)
- `iroh-gossip = "=0.96"` (gossip overlay — Phase 4)
- `tokio`, `rmp-serde`, `bytes` (existing)

**Pinning rationale (post-probe, 2026-05-07):**

The first iteration of this plan pinned `iroh = "1.0.0-rc.0"` standalone (no iroh-blobs, custom QUIC protocol from scratch) because iroh-blobs 0.100 has a hard transitive conflict with our `multihash-codetable` (sha2 pre-release pin via iroh-base 0.98 → ed25519-dalek 3.0.0-pre.6). That premise turns out to be over-narrow.

`cargo metadata` probes against the *actual* elohim-storage workspace constraints walked the iroh-blobs version range:

| iroh-blobs | Resolves with current stack? | Notes |
|---|---|---|
| 0.100 | ❌ sha2 rc.5 conflict via iroh-base 0.98 | Plan blocker — but only at this version |
| 0.99 | ❌ same | |
| 0.98 (Jan 2026) | ✅ **clean** — pulls iroh 0.96, sha2 0.10.9 + 0.11.0 coexist | **Selected** |
| 0.97, 0.96 | ❌ crypto-common conflict | |
| 0.94–0.90 | ✅ also clean (older) | |

iroh-blobs 0.98 with iroh 0.96 is a real released crate (~3 months soak), not a release candidate. It resolves alongside our current `holochain_client 0.9.0-dev.5` — no holochain bump required. Picking this pinned pair eliminates Phase 0 entirely and lets Phase 2 use iroh-blobs' production blob protocol instead of building a custom QUIC ALPN handler from scratch.

When n0 publishes iroh-blobs aligned with iroh 1.0 stable, we revisit. By then we'll already be running on iroh-blobs natively, so the upgrade is a version bump rather than a protocol rewrite.

**Phasing (high level):**

| Phase | Scope | Surface area | Status |
|---|---|---|---|
| ~~0~~ | ~~holochain dev.5 → dev.22 bump~~ | — | **eliminated** (no longer needed under iroh-blobs 0.98) |
| 1 | Foundation: deps, feature flag, IrohNode skeleton, config wiring | ~5 files | Detailed below |
| 2 | Blob plane via iroh-blobs (replaces `p2p::blob_protocol` + `p2p::blob_fetch`) | ~5 files | Detailed below |
| 3 | Custom-ALPN harness: shared `Router` mounting, codec helpers, parity-test scaffold | ~3 files | Sketched |
| 4 | Gossip plane via iroh-gossip (replaces inventory_gossip + identity_binding_gossip + attention_tending + feedback_signal + recovery_*) | ~6 files | Sketched |
| 5 | Sync plane (custom ALPN: `/elohim/sync/2.0.0`) | ~2 files | Sketched |
| 6 | EPR plane (`/elohim/epr/2.0.0`, `/elohim/epr-atom/2.0.0`) | ~3 files | Sketched |
| 7 | Shard plane (`/elohim/shard/2.0.0`) | ~2 files | Sketched |
| 8 | View federation (`/elohim/view-federation/2.0.0`) | ~2 files | Sketched |
| 9 | Identity / handshake / trust / reach planes | ~5 files | Sketched |
| 10 | Discovery + topology (iroh built-in DNS/pkarr/mdns replaces kad+mdns) | ~3 files | Sketched |
| 11 | Cutover gate: parity verification, default flip, libp2p deprecation | docs + Cargo | Cutover playbook |

Each phase from 3 onward gets its own design gate when picked up — that's not a deferral excuse, it's the rule (entity classification depends on what's being moved). This plan's executable detail is Phases 1 and 2; Phases 3–11 are the **cutover roadmap** with enough shape that the parallel scaffolding decisions (shared wire types, parity-test harness, hybrid-mode behavior) made in Phase 1–2 don't paint the later phases into corners.

---

## P2P Design Gate: iroh parallel stack — Phase 1 + 2 entities

This plan introduces persistent on-disk artifacts. Classifying each before any code is written, per `.claude/skills/p2p-design-gate/SKILL.md`.

### Entity: iroh-blobs store directory (`<storage_dir>/blobs_iroh/`)

- **Classification**: **Operational (Category C)** — parallel to the existing SHA256-keyed `crate::blob_store::BlobStore`. Phase 2 mode-exclusive with the libp2p path so the two stores never both serve at once.
- **Justification**: Bytes are content-addressed (BLAKE3, via iroh-blobs). Loss on a single node = re-fetch from peers, not loss of source of truth. The collective peer inventory is the broader source of truth; any single node's store is an operational projection of what it currently holds.
- **Content Address Strategy**: **Content-Derived (BLAKE3)** — iroh-blobs handles the hash IS the identity invariant natively. Distinct from the libp2p path's SHA256/CIDv1 addressing; in Phase 2 each path is canonical within its mode, runtime config selects exactly one mode.
- **Address Justification**: Content-derived because bytes are immutable. BLAKE3 is iroh-blobs' native addressing — chunked verified streaming, GC, dedup, partial fetches all key on it. We adopt iroh-blobs' format directly rather than re-wrapping in a CID for the iroh path.
- **Source of Truth**: **Local filesystem (operational)**. Reconstruction strategy: any blob can be re-fetched from peers via its BLAKE3 hash. The iroh-blobs store internally uses redb for metadata + filesystem for blob data; `<storage_dir>/blobs_iroh/` is the iroh-blobs `Store` root.
- **Coordinator Zome**: N/A — sidecar storage layer below the API boundary.
- **Storage Projection**: iroh-blobs internal layout. `peer_blob_inventory` table changes are deferred to Phase 4 (gossip plane), where BLAKE3 hashes get a parallel inventory column.
- **HTTP Route**: None in Phase 2. HTTP route graduation is deferred until parity is proven and the cutover playbook executes (Phase 11).
- **Anti-Pattern Check**: ✓ Source of truth declared (operational, reconstructable). ✓ Address format declared (BLAKE3, scoped to iroh mode). ✓ No new DHT entry type. ✓ Disjoint from existing SHA256 blob store directory (no aliasing).

### Entity: iroh secret key file (`<storage_dir>/iroh.key`)

- **Classification**: **Agent-Scoped (Category B)** — peer to the existing libp2p node keypair. Private to this node; identifies it on the iroh transport.
- **Justification**: ed25519 secret material; private credential. No other peer should ever see it. Loss = identity rotation.
- **Content Address Strategy**: N/A (not content; identity material). Public derivation (`NodeId`) is the addressable form.
- **Source of Truth**: **Private local filesystem** — never gossipped, file-mode 0600 on Unix.
- **Coordinator Zome**: N/A — not a Holochain entity.
- **HTTP Route**: None.
- **Anti-Pattern Check**: ✓ Not in a shared table. ✓ File-mode tightening on creation. ✓ Distinct from libp2p identity by design.

### Design Constraints Discovered

- **Two canonical address formats coexist during the cutover.** SHA256/CIDv1 on the libp2p path, BLAKE3 on the iroh path. Each canonical within its mode; runtime config selects exactly one. Documented in `p2p_iroh/README.md`. Convergence on BLAKE3 happens after the cutover gate (Phase 11).
- **`peer_blob_inventory` adds a parallel column for BLAKE3** in Phase 4 (gossip plane), not Phase 2. Phase 2 keeps the existing libp2p path unchanged.
- **Genesis seeder is not modified in Phase 2.** Continues to write SHA256-keyed blobs to the legacy `BlobStore`. Seeder graduates as part of Phase 11 cutover.
- **No HTTP route changes in Phases 1–2.** All HTTP routes continue to read from the legacy `BlobStore`.
- **Wire types are shared, not forked.** `crate::p2p::wire` is exposed publicly (or re-homed to `crate::wire`) so the iroh-side codecs reuse the same `BlobFetchRequest`/`BlobFetchResponse`/`SyncRequest`/etc. Cutover removes one transport, never two divergent message schemas.
- **`iroh-blobs` makes its own protocol surface.** Phase 2 does NOT define a custom blob ALPN — iroh-blobs is the blob protocol. The custom-ALPN harness (Phase 3) is for protocols that have no iroh equivalent.

### Entities deferred to later-phase design gates

These will be classified when their phase is picked up — not pre-decided here:

- iroh-gossip topic state (Phase 4) — operational; tied to existing gossipsub topic-name mapping
- BLAKE3 column on `peer_blob_inventory` (Phase 4) — operational projection, parallel to existing SHA256 column
- Cross-stack peer ID mapping (Phase 10) — agent-scoped operational; bridge during transition

---

## File Structure

### Phase 1 + 2 new files

| File | Phase | Responsibility |
|------|-------|----------------|
| `elohim/elohim-storage/src/p2p_iroh/mod.rs` | 1 | Module root; re-exports public API. Feature-gated as a whole. |
| `elohim/elohim-storage/src/p2p_iroh/config.rs` | 1 | `IrohConfig` struct (blobs_dir, secret_key_path, listen_addrs, relay mode, future ALPN list for Phase 3+). |
| `elohim/elohim-storage/src/p2p_iroh/identity.rs` | 1 | Load-or-generate `iroh::SecretKey`; persist as 32 raw bytes at `<storage_dir>/iroh.key`. |
| `elohim/elohim-storage/src/p2p_iroh/endpoint.rs` | 1 | `build_endpoint(&IrohConfig)` returns a configured `iroh::Endpoint`. |
| `elohim/elohim-storage/src/p2p_iroh/blob_store.rs` | 2 | Thin wrapper over `iroh_blobs::store::fs::Store` rooted at `<storage_dir>/blobs_iroh/`. Public surface: `add_bytes`, `get_bytes`, `has`, `gc`. |
| `elohim/elohim-storage/src/p2p_iroh/node.rs` | 2 | `IrohNode` aggregate — owns endpoint, iroh-blobs Router/Service, store. Public surface: `start`, `node_id`, `node_addr`, `add_bytes`, `fetch_blob_from`, `get_bytes`, `has`, `shutdown`. |
| `elohim/elohim-storage/src/p2p_iroh/README.md` | 2 | Status, what works, graduation gates, cutover playbook. |
| `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs` | 2 | Two-node integration test: provider adds blob → fetcher pulls by hash + provider's `NodeAddr` → bytes match. |
| `elohim/elohim-storage/tests/iroh_node_lifecycle.rs` | 2 | Single-node smoke test: build, listen, shutdown cleanly. |

### Phase 1 + 2 modified files

| File | Change |
|------|--------|
| `elohim/elohim-storage/Cargo.toml` | Phase 1: add optional `iroh`, `iroh-blobs`, `iroh-gossip` (gossip pinned now, used in Phase 4); add `p2p-iroh` feature. |
| `elohim/elohim-storage/justfile` | Add `build-iroh`, `test-iroh` recipes. |
| `elohim/elohim-storage/src/lib.rs` | Mount `p2p_iroh` module behind `#[cfg(feature = "p2p-iroh")]`; expose `crate::p2p::wire` (or relocate wire types to `crate::wire`) so iroh-side custom-ALPN handlers can reuse them in Phase 5+. |
| `elohim/elohim-storage/src/config.rs` | `TransportBackend` enum (`Libp2p` \| `Iroh`); `transport_backend` field on `Config`; loads from TOML/env (`ELOHIM_TRANSPORT_BACKEND`). |
| `elohim/elohim-storage/src/main.rs` | Branch on `config.transport_backend` at boot: spawn either `P2PNode` or `IrohNode`. |

### Out of scope (this plan)

- Other libp2p protocols' migration — sketched in Phases 3–10 below as cutover roadmap; each phase gets its own executable plan when picked up.
- HTTP route surface graduation (Phase 11 cutover only).
- Genesis seeder rewrite (Phase 11 cutover only).
- Self-hosted iroh-relay.
- Cross-stack identity unification — Phase 10 introduces a mapping table during transition; full unification is post-cutover.

---

## Pre-flight: Read these before starting

1. **`elohim/elohim-storage/CLAUDE.md`** — API boundary architecture; iroh module sits below it, no view-type changes.
2. **`elohim/elohim-storage/src/p2p/blob_protocol.rs`** — current libp2p blob protocol; we will reuse the wire format on the iroh path in Phase 5+ (sync, shard, EPR), but Phase 2 uses iroh-blobs' native protocol instead. Read but do not modify.
3. **`elohim/elohim-storage/src/blob_store.rs`** — current SHA256-keyed BlobStore; reference for the parallel-store-disjointness pattern.
4. **iroh-blobs 0.98 docs at task time** — `https://docs.rs/iroh-blobs/0.98.0/`. Verify the `Store::add_bytes`, `Store::get_bytes`, and `iroh_blobs::net_protocol::Blobs` (or whatever the integration shape is at this version) before each task that touches iroh-blobs API. The code in this plan reflects 0.98 docs as of plan-write; if a method has been renamed at task time, follow the docs and update the plan.

**Build incantation:**
```bash
cd elohim/elohim-storage
just build-iroh    # cargo build --features "p2p p2p-iroh", with RUSTFLAGS exported by justfile
just test-iroh     # cargo test --features "p2p p2p-iroh"
```

Bare `cargo build` will fail without the `RUSTFLAGS='--cfg getrandom_backend="custom"'` override.

---

## Phase 0: ELIMINATED

The original iteration of this plan required a holochain stack bump (`holochain_client 0.9.0-dev.5 → =0.9.0-dev.23`) to satisfy iroh 1.0-rc.0's serde 1.0.228 requirement. With iroh 0.96 pulled by iroh-blobs 0.98, that requirement disappears: the existing holochain_serialized_bytes 0.0.56 (transitive at dev.5) is fine.

The holochain bump is still worth doing on its own merits at some point — but it's no longer a precondition for iroh adoption, and doing it as part of an iroh sprint coupled two unrelated risks. We track it as a separate concern.

---

## Phase 1: Foundation

Phase 1 stands up the iroh module skeleton without touching any actual P2P logic. After Phase 1, `cargo build --features "p2p p2p-iroh"` succeeds and a no-op `IrohNode` can be constructed and shut down.

### Task 1.1: Add iroh + iroh-blobs + iroh-gossip dependencies and feature flag

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/justfile`

- [ ] **Step 1: Add the iroh dependencies**

Edit `elohim/elohim-storage/Cargo.toml`. After the libp2p block, before the `futures` dep, add:

```toml
# iroh — QUIC-based P2P transport (parallel stack; staged cutover from libp2p).
# Pinned to iroh-blobs 0.98 + iroh-gossip 0.96, which both pull iroh 0.96.
# This pin coexists with current holochain dev.5 — see plan version-pinning rationale.
# The iroh dep is pulled transitively; declare it explicitly so re-exports are stable.
iroh = { version = "=0.96", optional = true }
iroh-blobs = { version = "=0.98", optional = true }
iroh-gossip = { version = "=0.96", optional = true, features = ["net"] }
```

- [ ] **Step 2: Add p2p-iroh feature**

```toml
[features]
default = ["p2p"]
compression = ["lz4_flex"]
p2p = ["libp2p", "futures"]
p2p-iroh = ["iroh", "iroh-blobs", "iroh-gossip", "futures"]
```

`p2p-iroh` is **not** in `default`. Both features are additive (the parity test harness uses both compiled in).

- [ ] **Step 3: Add justfile recipes**

```makefile
# Build with iroh parallel stack enabled (libp2p still default)
build-iroh:
    cargo build --features "p2p p2p-iroh"

# Test with both stacks compiled in
test-iroh:
    cargo test --features "p2p p2p-iroh"
```

- [ ] **Step 4: Verify build**

```bash
cd /projects/elohim/.claude/worktrees/iroh-parallel-stack/elohim/elohim-storage
just build-iroh
```
Expected: builds successfully (downloads iroh + iroh-blobs + iroh-gossip + transitives). First compile is slow — allow up to 10 minutes.

If resolution fails: STOP. Do not change versions. The probe at plan-write proved this combo resolves; if it doesn't at task time, something in the workspace shifted. Surface BLOCKED, do NOT pick a different iroh-blobs version to "make it build" (per `feedback_subagent_dep_conflict_supervision.md`).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/justfile
git commit -m "feat(storage): add p2p-iroh feature + iroh-blobs/gossip deps (no code yet)"
```

### Task 1.2: Scaffold p2p_iroh module + relocate shared wire types

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/mod.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

- [ ] **Step 1: Create module root**

```rust
//! Parallel iroh-based P2P stack — staged cutover from libp2p.
//!
//! Sibling to [`crate::p2p`]; gated by the `p2p-iroh` Cargo feature. The two
//! stacks are mutually exclusive at runtime — selected by
//! [`crate::config::TransportBackend`] at startup — but compile additively so
//! the parity test harness can exercise them in one binary.
//!
//! Phase 2 (current): blob plane via iroh-blobs.
//! Phases 3+: gossip, sync, shard, EPR, view federation, identity, discovery.
//! See genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md.

// Submodules added in subsequent tasks.
```

- [ ] **Step 2: Mount in lib.rs**

```rust
#[cfg(feature = "p2p-iroh")]
pub mod p2p_iroh;
```

- [ ] **Step 3: Expose shared wire types**

Phase 5+ custom-ALPN handlers will reuse the existing libp2p-side wire types (`BlobFetchRequest`/`BlobFetchResponse`/`SyncRequest`/etc.). To avoid creating a parallel set, expose `crate::p2p::wire` publicly (or relocate the type definitions to a top-level `crate::wire` module if they're scattered across protocol files).

For Phase 1 the minimum is to confirm the wire types are accessible from outside `crate::p2p`. If they're currently buried in `crate::p2p::blob_protocol::BlobFetchRequest` (private to `p2p`), promote them now — it's cheap to do at module-scaffold time, expensive once Phase 5 imports them.

Decision deferred to Task 1.2 inspection: if the wire types are already `pub`, leave them. If not, relocate. Document the choice in the commit message.

- [ ] **Step 4: Verify build**

```bash
just build-iroh
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/mod.rs elohim/elohim-storage/src/lib.rs <any-relocated-files>
git commit -m "feat(storage): scaffold p2p_iroh module behind p2p-iroh feature"
```

### Task 1.3: IrohConfig struct

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/config.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

```rust
//! Iroh-side P2P configuration.

use std::path::PathBuf;

/// Configuration for the iroh-based P2P node.
#[derive(Debug, Clone)]
pub struct IrohConfig {
    /// Directory holding the iroh-blobs store.
    /// Defaults to `<storage_dir>/blobs_iroh/` to keep disjoint from the
    /// SHA256-keyed legacy `<storage_dir>/blobs/`.
    pub blobs_dir: PathBuf,

    /// Path to the persisted iroh secret key. Generated on first run.
    pub secret_key_path: PathBuf,

    /// Whether to use n0's hosted relay infrastructure.
    pub use_n0_relays: bool,
}

impl IrohConfig {
    pub fn from_storage_dir(storage_dir: &std::path::Path) -> Self {
        Self {
            blobs_dir: storage_dir.join("blobs_iroh"),
            secret_key_path: storage_dir.join("iroh.key"),
            use_n0_relays: true,
        }
    }
}
```

Tests: `from_storage_dir_uses_disjoint_blob_dir`, `secret_key_path_distinct_from_libp2p`.

Commit: `feat(storage): IrohConfig with disjoint blobs_iroh dir`

### Task 1.4: Persisted iroh secret key

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/identity.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

`load_or_generate(path) -> io::Result<iroh::SecretKey>`: read 32 raw bytes if file exists, else generate via `SecretKey::generate(&mut OsRng)` and persist with mode 0600.

**API drift to verify against iroh 0.96 docs at task time:** `SecretKey::from_bytes` signature, `SecretKey::generate` signature, key serialization (`to_bytes()` returns `[u8; 32]` historically).

Tests: `generates_when_missing`, `round_trips_existing_key`, `rejects_wrong_length_file`.

Commit: `feat(storage): persist iroh SecretKey at <storage_dir>/iroh.key`

### Task 1.5: Build the iroh Endpoint

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/endpoint.rs`

```rust
pub async fn build_endpoint(config: &IrohConfig) -> Result<iroh::Endpoint> {
    let secret = identity::load_or_generate(&config.secret_key_path)?;
    let relay_mode = if config.use_n0_relays {
        iroh::RelayMode::Default
    } else {
        iroh::RelayMode::Disabled
    };
    Endpoint::builder()
        .secret_key(secret)
        .relay_mode(relay_mode)
        .bind()
        .await
}
```

**API drift to verify against iroh 0.96 docs:** `Endpoint::builder()` signature, `bind()` vs deprecated names, `RelayMode` enum variants. Phase 2 will register iroh-blobs ALPNs via the iroh-blobs `Router` integration, NOT manually via `.alpns(...)` — so the endpoint here is just a base.

Tests: `builds_endpoint_with_relays_disabled` (single-node, then close).

Commit: `feat(storage): build_endpoint constructs iroh Endpoint`

### Task 1.6: TransportBackend config + main.rs branch

**Files:**
- Modify: `elohim/elohim-storage/src/config.rs`
- Modify: `elohim/elohim-storage/src/main.rs`

Add `TransportBackend` enum (`Libp2p` default, `Iroh`). Loads from TOML key `transport_backend` and env `ELOHIM_TRANSPORT_BACKEND`. In `main.rs`, branch at startup: spawn either the existing `P2PNode` (Libp2p) or a placeholder `IrohNode` (which Phase 2 fills in).

Phase 1 acceptance: `ELOHIM_TRANSPORT_BACKEND=iroh just run` boots a no-op IrohNode that:
- Builds endpoint
- Logs `node_id` + listen addrs
- Waits on shutdown signal
- Shuts cleanly

No actual P2P work yet; that's Phase 2.

Commit: `feat(storage): TransportBackend selector + main.rs branch (no-op iroh path)`

### Phase 1 acceptance criteria

- [ ] `just build-iroh` succeeds
- [ ] `just test-iroh -- p2p_iroh` passes (config, identity, endpoint tests)
- [ ] `ELOHIM_TRANSPORT_BACKEND=iroh just run` boots and shuts down cleanly
- [ ] `just gate` (fmt + clippy + test) clean for both feature flags
- [ ] No changes to existing libp2p code paths

---

## Phase 2: Blob plane via iroh-blobs

Phase 2 mounts iroh-blobs as the blob protocol on the iroh path. **No custom ALPN. No custom protocol handler. No hand-written codec.** iroh-blobs IS the blob plane.

The libp2p path is unchanged.

### Task 2.1: BlobStore wrapper over iroh-blobs

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/blob_store.rs`

A thin wrapper around `iroh_blobs::store::fs::Store::load(path)`. Public surface:

```rust
pub struct IrohBlobStore { inner: iroh_blobs::store::fs::Store }

impl IrohBlobStore {
    pub async fn load(blobs_dir: &Path) -> Result<Self>;
    pub async fn add_bytes(&self, data: Vec<u8>) -> Result<iroh_blobs::Hash>;
    pub async fn get_bytes(&self, hash: iroh_blobs::Hash) -> Result<Bytes>;
    pub async fn has(&self, hash: iroh_blobs::Hash) -> bool;
}
```

The wrapper exists to (a) anchor a stable Phase 2 surface so Phase 11 cutover code can replace the legacy BlobStore call sites uniformly, and (b) hide iroh-blobs API drift behind one type.

**API drift to verify against iroh-blobs 0.98 docs:** `iroh_blobs::store::fs` module exists; `Store::load` / `Store::create` / `Store::persistent` (name varies by version); `Store::add_bytes` async signature; `Store::get_bytes` returns `Bytes` or stream.

Tests: `add_then_get_round_trips`, `has_returns_false_for_unknown`, `same_bytes_dedupe_to_same_hash`.

Commit: `feat(storage): IrohBlobStore wraps iroh-blobs fs Store`

### Task 2.2: Mount iroh-blobs on the Endpoint via Router

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/node.rs`

`IrohNode` aggregates endpoint + iroh-blobs Router + store. iroh-blobs registers its own ALPN(s) via `iroh_blobs::net_protocol::Blobs::builder(store).build(endpoint)` (exact API per 0.98 docs at task time — likely `Blobs::new(store, endpoint)` returning a `Blobs` that can be inserted into a `Router`).

```rust
pub struct IrohNode {
    endpoint: iroh::Endpoint,
    router: iroh::protocol::Router,
    store: IrohBlobStore,
    blobs: iroh_blobs::net_protocol::Blobs,
}

impl IrohNode {
    pub async fn start(config: IrohConfig) -> Result<Self>;
    pub fn node_id(&self) -> iroh::NodeId;
    pub async fn node_addr(&self) -> Result<iroh::NodeAddr>;
    pub async fn add_bytes(&self, data: Vec<u8>) -> Result<iroh_blobs::Hash>;
    pub async fn fetch_blob_from(&self, peer: iroh::NodeAddr, hash: iroh_blobs::Hash) -> Result<Bytes>;
    pub async fn get_bytes(&self, hash: iroh_blobs::Hash) -> Result<Bytes>;
    pub async fn has(&self, hash: iroh_blobs::Hash) -> bool;
    pub async fn shutdown(self) -> Result<()>;
}
```

`fetch_blob_from` uses iroh-blobs' download API (`blobs_client.download(hash, peer).await?` or similar — verify at task time). It does NOT use a custom `BlobFetchRequest` — that's the libp2p path.

Tests: lifecycle (`start_then_shutdown`).

Commit: `feat(storage): IrohNode mounts iroh-blobs on shared Router`

### Task 2.3: Two-node round-trip integration test

**Files:**
- Create: `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs`
- Create: `elohim/elohim-storage/tests/iroh_node_lifecycle.rs`

```rust
#[tokio::test]
async fn two_node_blob_round_trip() {
    let provider = IrohNode::start(IrohConfig {
        use_n0_relays: false, // loopback only for CI
        ..test_config()
    }).await.unwrap();
    let fetcher = IrohNode::start(IrohConfig {
        use_n0_relays: false,
        ..test_config()
    }).await.unwrap();

    let payload = b"hello iroh world".to_vec();
    let hash = provider.add_bytes(payload.clone()).await.unwrap();
    let provider_addr = provider.node_addr().await.unwrap();

    let received = fetcher.fetch_blob_from(provider_addr, hash).await.unwrap();
    assert_eq!(received, payload);

    provider.shutdown().await.unwrap();
    fetcher.shutdown().await.unwrap();
}
```

**CI risk:** UDP loopback in Jenkins containers can behave oddly. The original plan flagged this. Mitigation: if `use_n0_relays: false` doesn't work in CI, fall back to `RelayMode::Default` for the test (n0 relay is public infra; CI environments typically have outbound). Document the choice.

Commit: `test(storage): two-node iroh blob round-trip`

### Task 2.4: README + graduation gates

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/README.md`

Contents:
- Status (what works, what's stubbed)
- How to run (env var + commands)
- Graduation gates for the next phase
- Cutover playbook reference
- Address format note (BLAKE3 on iroh path; SHA256/CIDv1 on libp2p path; mode-exclusive)

Commit: `docs(storage): p2p_iroh README + graduation gates`

### Phase 2 acceptance criteria

- [ ] `cargo test --features "p2p p2p-iroh" iroh_blob_roundtrip` passes
- [ ] `cargo test --features "p2p p2p-iroh" iroh_node_lifecycle` passes
- [ ] `ELOHIM_TRANSPORT_BACKEND=iroh just run` boots, accepts blob inserts via `IrohNode::add_bytes`, can be queried for `node_addr` via debug API
- [ ] `just gate` clean
- [ ] README updated; cutover playbook references this phase

---

## Phase 3: Custom-ALPN harness (sketched)

**Goal:** Land the shared infrastructure that Phases 5–9 will depend on. After Phase 3, registering a new custom protocol (sync, shard, EPR, etc.) is a copy-paste exercise.

**Surface:**
- `src/p2p_iroh/router_extras.rs` — extension points for adding ALPNs alongside iroh-blobs on the same `Router`
- `src/p2p_iroh/codec.rs` — generic length-prefixed MessagePack codec helpers (`read_frame<T>`, `write_frame<T>`) reusable by every custom protocol
- `src/p2p_iroh/parity_harness.rs` — test infrastructure that runs the same wire-level test against both libp2p and iroh nodes; used in Phase 5+

**Design gate runs at Phase 3 pickup.** Likely entities: ALPN registry (operational, in-memory), parity-test fixtures (test-only, no classification needed).

**No Phase 3 dependencies on later phases — but later phases all depend on Phase 3 codec helpers.** Ordering is strict.

---

## Phase 4: Gossip plane via iroh-gossip (sketched)

**Goal:** Replace libp2p gossipsub with iroh-gossip as the substrate for inventory, identity-binding, attention-tending, feedback-signal, and recovery topics.

**Topics being migrated (verified by grep at plan-write):**
- `INVENTORY_TOPIC` (peer_blob_inventory broadcasts)
- `IDENTITY_BINDING_TOPIC` / `INTEGRITY_REVOCATION_TOPIC`
- `RECOVERY_INVITATION_TOPIC` / `RECOVERY_REVOCATION_TOPIC`
- attention-tending heartbeat topic
- feedback-signal topic

**iroh-gossip mapping:** Each gossipsub topic maps to an iroh-gossip `TopicId` (32 bytes; we hash the existing topic-name string for determinism).

**`peer_blob_inventory` adds BLAKE3 column** in this phase. Migration: add `blake3_hash TEXT NULL` alongside `blob_hash`. Both populated during transition. After cutover (Phase 11), drop the SHA256 column.

**Hybrid mode:** Phase 4 can run with libp2p gossip primary + iroh-gossip secondary (or vice versa) for parity validation. The parity harness from Phase 3 verifies both stacks see the same set of topic messages within bounded time.

**Design gate runs at Phase 4 pickup.** Entities: iroh-gossip topic membership state (operational), `blake3_hash` column (operational projection, parallel to existing).

---

## Phase 5: Sync plane (sketched)

Custom ALPN: `/elohim/sync/2.0.0`. Reuse `crate::p2p::wire::SyncRequest`/`SyncResponse` (or whatever they're currently called) on iroh streams.

Parity test (via Phase 3 harness): same sync request issued to both stacks returns same wire bytes.

---

## Phase 6: EPR plane (sketched)

Two ALPNs: `/elohim/epr/2.0.0` and `/elohim/epr-atom/2.0.0`. Reuse existing wire types from `crate::p2p::epr_protocol` and `crate::p2p::epr_atom_protocol`.

EPR codec is transport-agnostic by design (per `project_epr_substrate_vs_vf_graphql.md`) — this phase is mostly plumbing.

---

## Phase 7: Shard plane (sketched)

Custom ALPN: `/elohim/shard/2.0.0`. Reuse `crate::p2p::wire::ShardRequest`/etc.

Reed-Solomon coding logic stays in pure Rust (transport-agnostic). Only the request/response framing migrates.

---

## Phase 8: View federation (sketched)

Custom ALPN: `/elohim/view-federation/2.0.0`. Reuse existing wire types.

256 KiB cap on responses (matches existing libp2p path). Document any iroh stream-flow-control implications discovered at task time.

---

## Phase 9: Identity / handshake / trust / reach (sketched)

Five ALPNs for the auth and trust planes:
- `/elohim/identity-handshake/2.0.0`
- `/elohim/identity-map/2.0.0`
- `/elohim/trust/2.0.0`
- `/elohim/reach-authorization/2.0.0`
- (kad-store needs special design — see Phase 10)

These are sensitive flows. Phase 9 design gate must verify:
- Identity material remains agent-scoped (no leakage via QUIC stream metadata)
- Reach-authorization gate semantics preserved on iroh path
- Cross-stack peer ID mapping correctly bridges identity claims during transition

---

## Phase 10: Discovery + topology (sketched)

iroh has built-in discovery (DNS, pkarr, mDNS) that replaces libp2p kad+mdns. The libp2p path uses `kad_store` for peer record persistence; the iroh path doesn't need a parallel — iroh's pkarr-DHT integration handles record publication.

**Cross-stack peer ID mapping** is required during the transition: a peer running on libp2p has a `PeerId`, on iroh a `NodeId`. The mapping table (`cross_stack_peer_map`) records both for the same logical peer during hybrid operation. Post-cutover, the table is dropped.

**kad_store migration plan:** Records currently in kad get re-published via pkarr-DHT after cutover. During Phase 10 hybrid, both record stores are populated.

---

## Phase 11: Cutover gate

Cutover criteria (must ALL hold before flipping default):

- [ ] All Phases 1–10 acceptance criteria green
- [ ] Parity-test harness runs every Phase 5–9 ALPN against both stacks for a week of nightly CI; zero divergences
- [ ] Inventory delta convergence proven on a 6-peer alpha cluster (`project_alpha_topology_bootstrap_pair.md`) within target time
- [ ] Production-shape stress test: 10k blob round-trips between two nodes via iroh, latency p99 ≤ libp2p baseline
- [ ] Recovery flow (`project_socially_derived_security.md`) end-to-end on iroh path passes
- [ ] Genesis seeder rewritten to write to BLAKE3 store (Phase 11 task — not earlier)
- [ ] HTTP routes graduated to BLAKE3 addressing (Phase 11 task — not earlier)
- [ ] Rollback playbook tested: flip default back to libp2p, run smoke test, flip forward again
- [ ] `peer_blob_inventory` SHA256 column drop migration written, tested, gated behind a follow-up release

**Cutover execution:**
1. Flip `TransportBackend` default to `Iroh` in code
2. Run `just gate` + smoke + alpha-cluster test
3. Tag release; deploy
4. Soak two weeks
5. Run column-drop migration + remove libp2p deps in a follow-up release
6. Delete `src/p2p/` module

**Rollback:** Until step 5, `ELOHIM_TRANSPORT_BACKEND=libp2p` reverts to legacy. After step 5, rollback requires reverting the deletion commit + redeploy.

---

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| iroh 0.96 → iroh 1.0 API drift later | We absorb it as a version bump when iroh-blobs ships 1.0-aligned; the Phase 2 wrapper (`IrohBlobStore`) hides drift |
| iroh-blobs internal store format incompatibility across upgrades | Pin `=0.98`; bump deliberately; document migration path in commit message |
| CI UDP loopback flake | Document fallback to n0 hosted relay for CI; alpha-cluster test is the real validation |
| `peer_blob_inventory` BLAKE3 column drift across phases | Add column in Phase 4; populate from both stacks in hybrid; drop SHA256 only after cutover stable |
| Subagent picks alternate iroh-blobs version when conflict surfaces (per `feedback_subagent_dep_conflict_supervision.md`) | Dispatch prompts in this plan explicitly forbid version changes; report BLOCKED instead |
| Parity-test divergence found mid-migration | Each phase is independently roll-back-able; isolate the divergent ALPN, fix or revert that phase only |
| Cross-stack identity confusion during hybrid | Phase 10 mapping table is the bridge; never assume a `PeerId` is the same identity as a `NodeId` without consulting the map |
| Phases 3–10 each hit unforeseen design gate work | That's by design — each phase gets its own gate at pickup; budget accordingly, don't promise dates ahead of gate runs |

---

## What this plan deliberately does NOT do

- Bump holochain stack — that's now decoupled and tracked separately
- Adopt iroh 1.0-rc.0 — pinning to soaked iroh 0.96 via iroh-blobs 0.98
- Build a custom QUIC blob protocol — iroh-blobs is the protocol
- Modify HTTP routes in Phases 1–2 — cutover concern (Phase 11)
- Modify the genesis seeder in Phases 1–2 — cutover concern (Phase 11)
- Pre-classify Phase 3+ entities — each phase's design gate runs at pickup

---

## Status

**Plan rewritten:** 2026-05-07 after second-opinion probe revealed iroh-blobs 0.98 resolves cleanly with current workspace, eliminating the need for a holochain bump and a custom QUIC blob protocol.

**Previous iteration superseded by this rewrite.** Git history preserves the standalone-iroh approach if needed for reference. The current `worktree-iroh-parallel-stack` branch retains the prior plan-amend commits as historical context — no code from those commits is on disk.

**Next session:** start at Phase 1 Task 1.1.
