# iroh Parallel P2P Stack — Phase 1 (Blob Plane, iroh-standalone)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a parallel iroh-based P2P module inside `elohim-storage` that can serve and fetch content-addressed blobs end-to-end via iroh's QUIC transport (custom ALPN, our own protocol handler, BLAKE3-keyed minimal file store). Existing libp2p stack remains the default; iroh becomes selectable via feature flag + runtime config. First cutover target: blob distribution. Other protocols (sync, shard, epr, trust, identity) stay on libp2p and graduate in subsequent sprints.

**Architecture:** A new `src/p2p_iroh/` module sits alongside `src/p2p/`, gated by `p2p-iroh` Cargo feature. Both backends compile additively; runtime config (`TransportBackend::Libp2p` | `TransportBackend::Iroh`) picks one at startup. In `Iroh` mode, blobs are stored in a new BLAKE3-keyed minimal file store at `<storage_dir>/blobs_iroh/` (operational, parallel to the existing SHA256-keyed `BlobStore`). Wire transport is iroh QUIC streams under a custom ALPN (`/elohim/blob/1.0.0`) carrying length-prefixed MessagePack `BlobFetchRequest` / `BlobFetchResponse` frames — same wire format as the existing libp2p blob protocol, just on QUIC instead of yamux. We deliberately **do not** use iroh-blobs in Phase 1: as of 2026-05-07, iroh-blobs 0.100 has hard incompatibilities with this codebase's `multihash-codetable` (sha2 pre-release exact pin). We revisit iroh-blobs adoption when n0 publishes a release aligned with iroh 1.0.

**Tech Stack:** `iroh = "1.0.0-rc.0"` (core endpoint + router + protocol handler API), `blake3 = "1"` (already a dep, used for content addressing on the iroh path), tokio, rmp-serde (existing for the wire format). **No iroh-blobs.**

**Phasing:** Phase 0 is a prerequisite holochain stack bump (`holochain_client 0.9.0-dev.5 → =0.9.0-dev.23`, `holochain_types 0.7.0-dev.5 → =0.7.0-dev.22`) that unblocks iroh's serde requirement. Phase 1 is the iroh module itself.

**Version pinning rationale (post-probe, 2026-05-07):**
- iroh 1.0.0-rc.0 was published 2026-05-07. We pin exactly because it has had no soak time.
- iroh 1.0.0-rc.0 requires `serde ^1.0.228`. Our existing `holochain_serialized_bytes 0.0.56` (transitive via `holochain_types 0.7.0-dev.5`) pins `serde =1.0.219`. The serde pin is exact on the holochain side, so a `[patch]` cannot thread it.
- `holochain_serialized_bytes 0.0.57` (April 2026) pins `serde =1.0.228` — exactly what iroh wants. `holochain_types 0.7.0-dev.22` is the first dev release pulling HSB 0.0.57. Bumping to dev.22 (or paired dev.23) unblocks the conflict.
- `iroh-blobs 0.100` requires `iroh ^0.98` (it has not yet caught up to iroh 1.0). iroh 0.98 + iroh-blobs 0.100 is itself viable in clean projects, but it transitively pulls `sha2 =0.11.0-rc.5` (an exact pre-release pin via iroh-base), which is incompatible with our existing `multihash-codetable 0.2`'s `sha2 ^0.11` (stable only). No `[patch]` threads this — both are exact constraints on different versions.
- The path that resolves: **iroh 1.0.0-rc.0 standalone, no iroh-blobs, holochain bumped to dev.22+.** Verified by `cargo metadata` probe on 2026-05-07; resolution succeeds with serde 1.0.228, ed25519-dalek 3.0.0-pre.7, and sha2 0.10.9 + 0.11.0 coexisting at different majors.

---

## P2P Design Gate: iroh parallel stack — Phase 1

This plan introduces two new persistent on-disk artifacts. Classifying each before any code is written, per `.claude/skills/p2p-design-gate/SKILL.md`.

### Entity: iroh blob store directory (`<storage_dir>/blobs_iroh/`)

- **Classification**: **Operational (Category C)** — parallel to the existing SHA256-keyed `crate::blob_store::BlobStore`. A minimal BLAKE3-keyed file store; Phase 1 mode-exclusive with the libp2p path so the two stores never both serve at once.
- **Justification**: Bytes are content-addressed (BLAKE3). Loss on a single node = re-fetch from peers, not loss of source of truth. The collective peer inventory is the broader source of truth; any single node's store is an operational projection of what it currently holds.
- **Content Address Strategy**: **Content-Derived (BLAKE3 hex)** — the hash IS the identity. Distinct from the libp2p path's SHA256/CIDv1 addressing; in Phase 1 each path is canonical within its mode, runtime config selects exactly one mode.
- **Address Justification**: Content-derived because bytes are immutable. BLAKE3 (not SHA256) chosen on the iroh path to be forward-compatible with eventual iroh-blobs adoption (iroh-blobs uses BLAKE3 natively). Storing as raw 64-char hex (the BLAKE3 fingerprint), not wrapped in CIDv1 — minimal viable shape.
- **Source of Truth**: **Local filesystem (operational)**. Reconstruction strategy: any blob can be re-fetched from peers via its BLAKE3 hash. No SQLite projection in Phase 1; `peer_blob_inventory` table changes are deferred to Phase 2.
- **Coordinator Zome**: N/A — sidecar storage layer below the API boundary.
- **Storage Projection**: filesystem at `<storage_dir>/blobs_iroh/<first-2-blake3-hex>/<remaining-62>`. No SQL, no redb. Phase 1 does not project into any inventory table.
- **HTTP Route**: None in Phase 1. Phase 2 graduates HTTP routes (separate plan, separate gate).
- **Anti-Pattern Check**: ✓ Source of truth declared (operational, reconstructable). ✓ Address format declared (BLAKE3, scoped to iroh mode). ✓ No new DHT entry type. ✓ Disjoint from existing SHA256 blob store directory (no aliasing).

### Entity: iroh secret key file (`<storage_dir>/iroh.key`)

- **Classification**: **Agent-Scoped (Category B)** — peer to the existing libp2p node keypair. Private to this node; identifies it on the iroh transport.
- **Justification**: ed25519 secret material; private credential. No other peer should ever see it. Loss = identity rotation.
- **Content Address Strategy**: N/A (not content; identity material). Public derivation (EndpointId) is the addressable form.
- **Source of Truth**: **Private local filesystem** — never gossipped, file-mode 0600 on Unix.
- **Coordinator Zome**: N/A — not a Holochain entity.
- **HTTP Route**: None.
- **Anti-Pattern Check**: ✓ Not in a shared table. ✓ File-mode tightening on creation. ✓ Distinct from libp2p identity by design (Phase 1 keeps stacks identity-disjoint).

### Design Constraints Discovered

- **Two canonical address formats coexist during Phase 1.** SHA256/CIDv1 on the libp2p path, BLAKE3 on the iroh path. Each canonical within its mode; runtime config selects exactly one. Documented in `p2p_iroh/README.md` (Task 11). Temporary state — the cutover playbook converges on one format once libp2p is retired.
- **`peer_blob_inventory` is not modified.** Phase 2 graduates the inventory layer with its own design gate.
- **Genesis seeder is not modified.** Continues to write SHA256-keyed blobs to the legacy `BlobStore`. Phase 2 graduates the seeder.
- **No new HTTP route surface.** All HTTP routes continue to read from the legacy `BlobStore`.
- **No iroh-blobs surface.** Phase 1 deliberately avoids iroh-blobs because it's currently incompatible with our codebase. Phase 2 (or later) will reconsider once n0 ships a 1.0-aligned release.

---

## File Structure

### New files (Phase 1)

| File | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/p2p_iroh/mod.rs` | Module root; re-exports public API. Feature-gated as a whole. |
| `elohim/elohim-storage/src/p2p_iroh/config.rs` | `IrohConfig` struct (blobs_dir, secret_key_path, listen_addrs, relay mode, ALPN list). |
| `elohim/elohim-storage/src/p2p_iroh/identity.rs` | Load-or-generate `iroh::SecretKey`; persist as 32 raw bytes at `<storage_dir>/iroh.key`. |
| `elohim/elohim-storage/src/p2p_iroh/endpoint.rs` | `build_endpoint(&IrohConfig)` returns a configured `iroh::Endpoint` with our custom ALPN registered. |
| `elohim/elohim-storage/src/p2p_iroh/blake3_store.rs` | `Blake3Store` — minimal BLAKE3-keyed filesystem store. `add_bytes(&[u8]) -> Hash`, `get_bytes(Hash) -> Vec<u8>`, `has(Hash) -> bool`. |
| `elohim/elohim-storage/src/p2p_iroh/blob_protocol.rs` | Custom protocol primitives: `BLOB_ALPN`, `BlobFetchRequest`, `BlobFetchResponse`, length-prefixed MessagePack codec helpers (`read_request_frame`, `write_response_frame`, etc.) for QUIC streams. |
| `elohim/elohim-storage/src/p2p_iroh/blob_handler.rs` | `BlobProtocolHandler` — implements `iroh::protocol::ProtocolHandler`. Accepts inbound bidi streams, parses BlobFetchRequest, looks up via Blake3Store, writes BlobFetchResponse. |
| `elohim/elohim-storage/src/p2p_iroh/router.rs` | `build_router(endpoint, handler)` — assembles `iroh::protocol::Router` registering BLOB_ALPN → BlobProtocolHandler. |
| `elohim/elohim-storage/src/p2p_iroh/fetch.rs` | `fetch_blob(endpoint, peer_addr, hash) -> Result<Vec<u8>>` — connects to peer over QUIC, opens bidi stream, sends BlobFetchRequest, awaits BlobFetchResponse, verifies BLAKE3 hash. |
| `elohim/elohim-storage/src/p2p_iroh/node.rs` | `IrohNode` aggregate — owns endpoint, router, store. Public surface: `start`, `node_id`, `node_addr`, `add_bytes`, `fetch_blob_from`, `get_bytes`, `has`, `shutdown`. |
| `elohim/elohim-storage/src/p2p_iroh/README.md` | Status, what works, graduation gates, cutover playbook. |
| `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs` | Two-node integration test: provider adds blob → fetcher pulls by hash + provider's address → bytes match. |
| `elohim/elohim-storage/tests/iroh_node_lifecycle.rs` | Single-node smoke test: build, listen, shutdown cleanly. |

### Modified files

| File | Change |
|------|--------|
| `elohim/elohim-storage/Cargo.toml` | Phase 0: `=` pins on holochain_client/_types. Phase 1: optional `iroh` dep; `p2p-iroh` feature. |
| `elohim/elohim-storage/justfile` | Add `build-iroh`, `test-iroh` recipes. |
| `elohim/elohim-storage/src/lib.rs` | Mount `p2p_iroh` module behind `#[cfg(feature = "p2p-iroh")]`. |
| `elohim/elohim-storage/src/config.rs` | `TransportBackend` enum (`Libp2p` | `Iroh`); `transport_backend` field on `Config`; loads from TOML/env (`ELOHIM_TRANSPORT_BACKEND`). |
| `elohim/elohim-storage/src/main.rs` | Branch on `config.transport_backend` at boot: spawn either `P2PNode` or `IrohNode`. |
| **(Phase 0 only)** various `*.rs` files in `elohim-storage` | Absorb holochain dev.5 → dev.22 API drift (e.g., `kitsune2_api::url::Url` → `kitsune2_api::Url`, removed `AdminRequest::GraftRecords`, etc.). Discovered during `cargo build`. |

### Out of scope (Phase 1)

- Other libp2p protocols' migration (sync/shard/epr/trust/identity).
- HTTP route surface graduation.
- Genesis seeder rewrite.
- `peer_blob_inventory` projection schema migration to BLAKE3.
- Self-hosted iroh-relay.
- Cross-stack identity unification (separate iroh `EndpointId` and libp2p `PeerId`).
- iroh-blobs adoption (deferred until n0 ships a 1.0-aligned release).

Each runs its own design gate when graduated.

---

## Pre-flight: Read these before starting

1. **`elohim/elohim-storage/CLAUDE.md`** — API boundary architecture; iroh module sits below it, no view-type changes.
2. **`elohim/elohim-storage/src/p2p/blob_protocol.rs`** — current libp2p blob protocol; we reuse the wire format (`BlobFetchRequest`, `BlobFetchResponse`) on the iroh path. Read but do not modify.
3. **`elohim/elohim-storage/src/blob_store.rs`** — current SHA256-keyed BlobStore; reference for `Blake3Store`'s on-disk layout pattern.
4. **iroh docs at task time** — iroh 1.0.0-rc.0 was published 2026-05-07. Verify API signatures against `https://docs.rs/iroh/1.0.0-rc.0/iroh/` before each task that touches iroh API. The code in this plan reflects the README/docs as of plan-write; if a method has been renamed, follow the docs and update the plan.

**Build incantation:**
```bash
cd elohim/elohim-storage
just build-iroh    # cargo build --features "p2p p2p-iroh", with RUSTFLAGS exported by justfile
just test-iroh     # cargo test --features "p2p p2p-iroh"
```

Bare `cargo build` will fail without the `RUSTFLAGS='--cfg getrandom_backend="custom"'` override.

---

## Phase 0: Bump holochain stack to dev.22+

Phase 0 is a single discovery-driven task. The holochain bump is necessary to unblock iroh's serde requirement; it is NOT optional. The cost is bounded but not pre-knowable — we know `AdminRequest::GraftRecords` was removed and `kitsune2_api::url::Url` was renamed; other API drift may surface during `cargo build`. Phase 0 is "make the bump compile and tests pass; commit; move to Phase 1."

### Phase 0, Task A: Bump holochain stack

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: any `*.rs` files using removed/renamed holochain APIs (discovered during build)

- [ ] **Step 1: Pin holochain to dev.22+**

Edit `elohim/elohim-storage/Cargo.toml`. Find lines 48-49:

```toml
holochain_client = { version = "0.9.0-dev.5", default-features = false, features = ["lair_signing"] }
holochain_types = { version = "0.7.0-dev.5", default-features = false }
```

Change to:

```toml
# Pinned to dev.23/dev.22 to align with serde 1.0.228 (required by iroh 1.0).
# holochain_serialized_bytes 0.0.57 (transitive via holochain_types 0.7.0-dev.22)
# bumps the pin from serde =1.0.219 to =1.0.228.
holochain_client = { version = "=0.9.0-dev.23", default-features = false, features = ["lair_signing"] }
holochain_types = { version = "=0.7.0-dev.22", default-features = false }
```

- [ ] **Step 2: Try to build**

```bash
cd /projects/elohim/.claude/worktrees/iroh-parallel-stack/elohim/elohim-storage
just build
```

Expect compilation errors. Capture them:
```bash
just build 2>&1 | tee /tmp/holochain-bump-errors.txt
```

- [ ] **Step 3: Fix the API drift**

Common issues likely to appear (verified from earlier probe):

| Issue | Likely fix |
|-------|------------|
| `error[E0599]: no variant named 'GraftRecords' found for enum 'AdminRequest'` | Remove or replace usage. `GraftRecords` was a debug surface; the production path likely uses `RegisterDna` + `AddDhtOps` or similar. Check `holochain_client 0.9.0-dev.23` docs; find the equivalent. |
| `error[E0308]: mismatched types ... expected 'kitsune2_api::url::Url', found 'kitsune2_api::Url'` | Update the import: `use kitsune2_api::url::Url` → `use kitsune2_api::Url`. Both names referenced the same type; the namespacing changed. |
| `error[E0432]: unresolved import 'holochain_client::AdminWebsocket'` (or similar) | The module path may have changed. `cargo doc --open` on holochain_client 0.9.0-dev.23 shows the current surface. |

For each error: read the error line, find the calling file, look up the new API in `holochain_client 0.9.0-dev.23` docs (`https://docs.rs/holochain_client/0.9.0-dev.23`), update the call site. Do NOT introduce stubs or `todo!()` — every replacement must be a real fix.

If you encounter a removed feature with no clear replacement (rare), leave a clear `// TODO(holochain-bump): GraftRecords removed in dev.X; replacement is ...` comment, only if you've confirmed via docs no equivalent exists.

- [ ] **Step 4: Iterate until clean build**

Run `just build` after each batch of fixes. Stop iterating only when the build is green.

- [ ] **Step 5: Run tests; absorb test-only API drift**

```bash
just test
```

Tests may also use removed APIs. Same fix process: read error, find call site, update. Pre-push hook also runs `just gate` (fmt + clippy + test); make sure clippy is clean too.

- [ ] **Step 6: Commit**

Commit Phase 0 as a separate, focused commit before any Phase 1 work:

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/Cargo.lock <any-rs-files-modified>
git commit -m "chore(storage): bump holochain to dev.23/dev.22 (unblocks iroh serde)

Required to land Phase 1 of the iroh parallel stack. holochain_client and
holochain_types pinned exactly to prevent unintended further drift. Absorbed
API drift: <list the things you fixed>.

See genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
"
```

- [ ] **Step 7: Verify**

```bash
just gate    # fmt + clippy + test
```
Expected: all green.

If you hit unexpected scope (the API drift turns out to be larger than 1-3 days of work), STOP and report BLOCKED. Do not continue into Phase 1 with a half-bumped holochain. The cost of an incomplete bump is much higher than the cost of pausing.

---

## Phase 1: iroh parallel module

The 11 tasks below land on top of the Phase 0 commit. Each task is bite-sized TDD: write failing test → run failing → implement → run passing → commit.

## Task 1: Add iroh dependency and feature flag

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/justfile`

- [ ] **Step 1: Add the iroh dependency**

Edit `elohim/elohim-storage/Cargo.toml`. After the libp2p block (around line 171), before the `futures` dep (around line 174), add:

```toml
# iroh — QUIC-based P2P transport (parallel stack, Phase 1, standalone).
# Pinned to 1.0.0-rc.0 published 2026-05-07; bump explicitly when 1.0 stable
# ships and we're ready to absorb API changes. iroh-blobs is intentionally
# NOT a dep here — see plan version-pinning rationale.
iroh = { version = "=1.0.0-rc.0", optional = true }
```

- [ ] **Step 2: Add p2p-iroh feature**

In `[features]` (around line 179):

```toml
[features]
default = ["p2p"]
compression = ["lz4_flex"]
p2p = ["libp2p", "futures"]
p2p-iroh = ["iroh", "futures"]
```

`p2p-iroh` is **not** in `default`. Both features are additive (the parity test harness uses both compiled in).

- [ ] **Step 3: Add justfile recipes**

Edit `elohim/elohim-storage/justfile`. After the existing `build` and `test` recipes:

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
Expected: builds successfully (downloads iroh + transitives from crates.io). First compile is slow — allow up to 10 minutes.

If resolution fails, the Phase 0 holochain bump probably didn't land. Verify HEAD includes the holochain bump commit before continuing.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/justfile
git commit -m "feat(storage): add p2p-iroh feature flag (deps only, no code yet)"
```

---

## Task 2: Scaffold p2p_iroh module

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/mod.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

- [ ] **Step 1: Create module root**

Create `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
//! Parallel iroh-based P2P stack (Phase 1 — blob plane only, iroh-standalone).
//!
//! Sibling to [`crate::p2p`]; gated by the `p2p-iroh` Cargo feature. In iroh
//! mode the blob store is a minimal BLAKE3-keyed filesystem store at
//! `<storage_dir>/blobs_iroh/`, parallel to the SHA256-keyed
//! [`crate::blob_store::BlobStore`]. The two stacks are mutually exclusive at
//! runtime — selected by [`crate::config::TransportBackend`] at startup —
//! but compile additively so the parity test harness can exercise them in
//! one binary.
//!
//! Phase 1 deliberately does NOT use iroh-blobs; see
//! `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`.

// Submodules added in subsequent tasks.
```

- [ ] **Step 2: Mount in lib.rs**

Edit `elohim/elohim-storage/src/lib.rs`. After the existing `#[cfg(feature = "p2p")] pub mod p2p;` block, add:

```rust
#[cfg(feature = "p2p-iroh")]
pub mod p2p_iroh;
```

- [ ] **Step 3: Verify build**

```bash
just build-iroh
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/mod.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): scaffold p2p_iroh module behind p2p-iroh feature"
```

---

## Task 3: IrohConfig struct

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/config.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/config.rs`:

```rust
//! Iroh-side P2P configuration.

use std::path::PathBuf;

/// ALPN identifier for our custom blob fetch protocol on QUIC.
/// Distinct from any iroh-blobs ALPN — we are not using iroh-blobs in Phase 1.
pub const BLOB_ALPN: &[u8] = b"/elohim/blob/1.0.0";

/// Configuration for the iroh-based P2P node.
#[derive(Debug, Clone)]
pub struct IrohConfig {
    /// Directory holding the BLAKE3-keyed blob files.
    /// Defaults to `<storage_dir>/blobs_iroh/` to keep disjoint from the
    /// SHA256-keyed legacy `<storage_dir>/blobs/`.
    pub blobs_dir: PathBuf,

    /// Path to the persisted iroh secret key. Generated on first run.
    pub secret_key_path: PathBuf,

    /// Whether to use n0's hosted relay infrastructure. `true` matches
    /// `RelayMode::Default`; `false` matches `RelayMode::Disabled` (only
    /// useful when both sides have routable addresses, e.g. household LAN
    /// or loopback in tests).
    pub use_n0_relays: bool,

    /// ALPN identifiers this endpoint will accept. Phase 1 only registers
    /// BLOB_ALPN; later phases extend as protocols graduate from libp2p.
    pub alpns: Vec<Vec<u8>>,
}

impl IrohConfig {
    /// Construct a default config rooted at `storage_dir`.
    pub fn from_storage_dir(storage_dir: &std::path::Path) -> Self {
        Self {
            blobs_dir: storage_dir.join("blobs_iroh"),
            secret_key_path: storage_dir.join("iroh.key"),
            use_n0_relays: true,
            alpns: vec![BLOB_ALPN.to_vec()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn from_storage_dir_uses_disjoint_blob_dir() {
        let cfg = IrohConfig::from_storage_dir(Path::new("/var/elohim"));
        assert_eq!(cfg.blobs_dir, Path::new("/var/elohim/blobs_iroh"));
        assert_eq!(cfg.secret_key_path, Path::new("/var/elohim/iroh.key"));
        assert!(cfg.use_n0_relays);
        assert_eq!(cfg.alpns.len(), 1);
        assert_eq!(cfg.alpns[0], BLOB_ALPN);
    }

    #[test]
    fn alpns_does_not_collide_with_legacy_blob_dir() {
        let cfg = IrohConfig::from_storage_dir(Path::new("/x"));
        assert_ne!(cfg.blobs_dir.file_name().unwrap(), "blobs");
    }

    #[test]
    fn blob_alpn_is_canonical() {
        assert_eq!(BLOB_ALPN, b"/elohim/blob/1.0.0");
    }
}
```

- [ ] **Step 2: Mount in mod.rs**

```rust
mod config;

pub use config::{BLOB_ALPN, IrohConfig};
```

- [ ] **Step 3: Run tests**

```bash
just test-iroh -- p2p_iroh::config
```
Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): IrohConfig with disjoint blobs_iroh dir + BLOB_ALPN"
```

---

## Task 4: Persisted iroh secret key

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/identity.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/identity.rs`:

```rust
//! Persisted iroh secret key. Load if present, generate-and-write if not.
//!
//! Stored as 32 raw bytes (the ed25519 secret) at the configured path.
//! Distinct from any libp2p keypair file; the two stacks have separate
//! identities in Phase 1.

use std::path::Path;
use std::{fs, io};

use iroh::SecretKey;

/// Load a secret key from `path`, or generate a fresh one and persist it
/// if the file does not exist.
pub fn load_or_generate(path: &Path) -> io::Result<SecretKey> {
    match fs::read(path) {
        Ok(bytes) => {
            let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "iroh key file {} has wrong length: expected 32, got {}",
                        path.display(),
                        bytes.len()
                    ),
                )
            })?;
            Ok(SecretKey::from_bytes(&arr))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let key = SecretKey::generate(&mut rand::rngs::OsRng);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, key.to_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        let key = load_or_generate(&path).unwrap();
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap().len(), 32);
        let _ = key.public();
    }

    #[test]
    fn round_trips_existing_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        let k1 = load_or_generate(&path).unwrap();
        let k2 = load_or_generate(&path).unwrap();
        assert_eq!(k1.to_bytes(), k2.to_bytes());
    }

    #[test]
    fn rejects_wrong_length_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iroh.key");
        fs::write(&path, b"too short").unwrap();
        let err = load_or_generate(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
```

- [ ] **Step 2: Add `rand` dep**

Verify whether `rand` is already a transitive: `cargo tree --features "p2p p2p-iroh" -e normal | grep '^rand '`. If not direct, add to `Cargo.toml`:

```toml
rand = { version = "0.8", optional = true }
```

And update the feature:
```toml
p2p-iroh = ["iroh", "futures", "rand"]
```

If iroh 1.0.0-rc.0 has migrated to `rand` 0.9, match that version. Verify with `cargo tree`.

- [ ] **Step 3: Mount in mod.rs**

```rust
mod config;
mod identity;

pub use config::{BLOB_ALPN, IrohConfig};
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 4: Run tests**

```bash
just test-iroh -- p2p_iroh::identity
```
Expected: 3 tests pass.

If `SecretKey::from_bytes` has a different signature in 1.0.0-rc.0 (e.g., returns `Result`), follow the docs and adjust.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): persist iroh SecretKey at <storage_dir>/iroh.key"
```

---

## Task 5: Build the iroh Endpoint

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/endpoint.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/endpoint.rs`:

```rust
//! Iroh `Endpoint` construction. ALPN registration happens here; protocol
//! handlers are registered separately on the `Router`.

use anyhow::Result;
use iroh::Endpoint;

use super::{config::IrohConfig, identity};

/// Build an iroh `Endpoint` from config. Caller is responsible for shutting
/// it down on graceful exit.
pub async fn build_endpoint(config: &IrohConfig) -> Result<Endpoint> {
    let secret = identity::load_or_generate(&config.secret_key_path)?;

    let relay_mode = if config.use_n0_relays {
        iroh::RelayMode::Default
    } else {
        iroh::RelayMode::Disabled
    };

    let endpoint = Endpoint::builder()
        .secret_key(secret)
        .alpns(config.alpns.clone())
        .relay_mode(relay_mode)
        .bind()
        .await?;

    Ok(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::p2p_iroh::config::BLOB_ALPN;

    #[tokio::test]
    async fn builds_endpoint_with_relays_disabled() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            alpns: vec![BLOB_ALPN.to_vec()],
        };

        let ep = build_endpoint(&cfg).await.expect("endpoint builds");
        let _id = ep.node_id();
        ep.close().await;
    }
}
```

- [ ] **Step 2: Mount in mod.rs**

```rust
mod config;
mod endpoint;
mod identity;

pub use config::{BLOB_ALPN, IrohConfig};
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 3: Run test**

```bash
just test-iroh -- p2p_iroh::endpoint
```
Expected: 1 test passes.

**API drift to verify:** `Endpoint::builder()` may require a preset arg in 1.0.0-rc.0 (e.g., `Endpoint::builder(presets::N0)`). Check `https://docs.rs/iroh/1.0.0-rc.0/iroh/struct.Endpoint.html`. Also verify `node_id()` vs renamed `endpoint_id()`. Update both production code and test once confirmed.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): build_endpoint constructs iroh Endpoint with custom ALPN"
```

---

## Task 6: Blake3Store — minimal BLAKE3-keyed file store

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/blake3_store.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/blake3_store.rs`:

```rust
//! Minimal BLAKE3-keyed filesystem blob store.
//!
//! On the iroh path, blobs are content-addressed with BLAKE3 (not SHA256).
//! Storage layout: `<root>/<first-2-hex>/<remaining-62-hex>`. No SQL, no
//! redb — just files. Mirrors the layout pattern from
//! [`crate::blob_store::BlobStore`] but with BLAKE3.

use std::path::{Path, PathBuf};
use std::{fs, io};

/// 32-byte BLAKE3 fingerprint, displayed as 64 lowercase hex chars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> io::Result<Self> {
        let bytes = hex::decode(s)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected 32-byte hash, got {} bytes", bytes.len()),
            )
        })?;
        Ok(Self(arr))
    }
}

/// BLAKE3-keyed filesystem store.
pub struct Blake3Store {
    root: PathBuf,
}

impl Blake3Store {
    pub fn open(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self { root: root.to_path_buf() })
    }

    /// Hash and persist `bytes`. Idempotent.
    pub fn add_bytes(&self, bytes: &[u8]) -> io::Result<Hash> {
        let hash = Hash(blake3::hash(bytes).into());
        let path = self.path_for(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write to a temp file in the same dir, then atomic-rename.
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &path)?;
        Ok(hash)
    }

    pub fn get_bytes(&self, hash: Hash) -> io::Result<Vec<u8>> {
        fs::read(self.path_for(&hash))
    }

    pub fn has(&self, hash: Hash) -> bool {
        self.path_for(&hash).exists()
    }

    fn path_for(&self, hash: &Hash) -> PathBuf {
        let hex = hash.to_hex();
        let (prefix, rest) = hex.split_at(2);
        self.root.join(prefix).join(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn add_then_get_round_trips() {
        let dir = tempdir().unwrap();
        let store = Blake3Store::open(dir.path()).unwrap();
        let h = store.add_bytes(b"hello iroh").unwrap();
        let back = store.get_bytes(h).unwrap();
        assert_eq!(back, b"hello iroh");
    }

    #[test]
    fn add_is_idempotent() {
        let dir = tempdir().unwrap();
        let store = Blake3Store::open(dir.path()).unwrap();
        let h1 = store.add_bytes(b"same").unwrap();
        let h2 = store.add_bytes(b"same").unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn has_reports_correctly() {
        let dir = tempdir().unwrap();
        let store = Blake3Store::open(dir.path()).unwrap();
        let h = store.add_bytes(b"present").unwrap();
        assert!(store.has(h));
        let absent = Hash([0u8; 32]);
        assert!(!store.has(absent));
    }

    #[test]
    fn hash_hex_round_trips() {
        let h = Hash([0xab; 32]);
        let s = h.to_hex();
        assert_eq!(s.len(), 64);
        let back = Hash::from_hex(&s).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn path_layout_is_two_char_prefix() {
        let dir = tempdir().unwrap();
        let store = Blake3Store::open(dir.path()).unwrap();
        let h = store.add_bytes(b"test").unwrap();
        let hex = h.to_hex();
        let expected = dir.path().join(&hex[..2]).join(&hex[2..]);
        assert!(expected.exists());
    }
}
```

- [ ] **Step 2: Mount in mod.rs**

```rust
mod blake3_store;
mod config;
mod endpoint;
mod identity;

pub use blake3_store::{Blake3Store, Hash};
pub use config::{BLOB_ALPN, IrohConfig};
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 3: Run tests**

```bash
just test-iroh -- p2p_iroh::blake3_store
```
Expected: 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): Blake3Store — BLAKE3-keyed filesystem blob store"
```

---

## Task 7: Blob protocol primitives + handler

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/blob_protocol.rs`
- Create: `elohim/elohim-storage/src/p2p_iroh/blob_handler.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write blob_protocol.rs**

Create `elohim/elohim-storage/src/p2p_iroh/blob_protocol.rs`:

```rust
//! Wire format for the custom blob protocol on iroh QUIC streams.
//!
//! Length-prefixed (4-byte big-endian u32) MessagePack frames carrying
//! [`BlobFetchRequest`] / [`BlobFetchResponse`]. Same wire shape as the
//! libp2p `request_response` codec in [`crate::p2p::blob_protocol`], just
//! moved off libp2p's codec trait onto raw QUIC `tokio::io` streams.

use serde::{Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAX_REQUEST_SIZE: usize = 4 * 1024;
pub const DEFAULT_MAX_RESPONSE_SIZE: usize = 16 * 1024 * 1024;
pub const HARD_MAX_RESPONSE_SIZE: usize = 64 * 1024 * 1024;

/// Request: peer asks for a blob by 64-char BLAKE3 hex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobFetchRequest {
    pub blake3_hex: String,
}

/// Response from the targeted peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobFetchResponse {
    Found(Vec<u8>),
    NotFound,
    Error(String),
}

pub async fn write_request<W: AsyncWrite + Unpin>(
    w: &mut W,
    req: &BlobFetchRequest,
) -> io::Result<()> {
    let data = rmp_serde::to_vec(req)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let len: u32 = data.len().try_into().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "request too large for u32 length prefix")
    })?;
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(&data).await?;
    w.flush().await
}

pub async fn read_request<R: AsyncRead + Unpin>(r: &mut R) -> io::Result<BlobFetchRequest> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_REQUEST_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("request too large: {} > {}", len, MAX_REQUEST_SIZE),
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub async fn write_response<W: AsyncWrite + Unpin>(
    w: &mut W,
    resp: &BlobFetchResponse,
) -> io::Result<()> {
    let data = rmp_serde::to_vec(resp)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let len: u32 = data.len().try_into().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "response too large for u32 length prefix")
    })?;
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(&data).await?;
    w.flush().await
}

pub async fn read_response<R: AsyncRead + Unpin>(
    r: &mut R,
    max_response_size: usize,
) -> io::Result<BlobFetchResponse> {
    let cap = max_response_size.min(HARD_MAX_RESPONSE_SIZE);
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > cap {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("response too large: {} > {}", len, cap),
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncWriteExt};

    #[tokio::test]
    async fn request_round_trips_through_stream() {
        let (mut a, mut b) = duplex(8192);
        let req = BlobFetchRequest { blake3_hex: "ab".repeat(32) };
        let req_clone = req.clone();
        let writer = tokio::spawn(async move {
            write_request(&mut a, &req_clone).await.unwrap();
            a.shutdown().await.unwrap();
        });
        let got = read_request(&mut b).await.unwrap();
        writer.await.unwrap();
        assert_eq!(got, req);
    }

    #[tokio::test]
    async fn response_found_round_trips() {
        let (mut a, mut b) = duplex(2 * 1024 * 1024);
        let payload = vec![0x42_u8; 1_000_000];
        let resp = BlobFetchResponse::Found(payload.clone());
        let writer = tokio::spawn(async move {
            write_response(&mut a, &resp).await.unwrap();
            a.shutdown().await.unwrap();
        });
        let got = read_response(&mut b, DEFAULT_MAX_RESPONSE_SIZE).await.unwrap();
        writer.await.unwrap();
        match got {
            BlobFetchResponse::Found(b) => assert_eq!(b, payload),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_response_rejects_oversized_frame() {
        let (mut a, mut b) = duplex(32);
        // Write a length prefix claiming 100 MiB (above hard cap).
        let len: u32 = (HARD_MAX_RESPONSE_SIZE as u32) + 1;
        a.write_all(&len.to_be_bytes()).await.unwrap();
        a.shutdown().await.unwrap();
        let err = read_response(&mut b, DEFAULT_MAX_RESPONSE_SIZE).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
```

- [ ] **Step 2: Write blob_handler.rs**

Create `elohim/elohim-storage/src/p2p_iroh/blob_handler.rs`:

```rust
//! Inbound blob protocol handler. Implements `iroh::protocol::ProtocolHandler`
//! to accept incoming connections on BLOB_ALPN, parse fetch requests, and
//! serve from the BLAKE3 store.

use std::sync::Arc;

use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};

use super::blake3_store::{Blake3Store, Hash};
use super::blob_protocol::{
    read_request, write_response, BlobFetchResponse, MAX_REQUEST_SIZE,
};

#[derive(Debug, Clone)]
pub struct BlobProtocolHandler {
    store: Arc<Blake3Store>,
}

impl BlobProtocolHandler {
    pub fn new(store: Arc<Blake3Store>) -> Self {
        Self { store }
    }
}

impl ProtocolHandler for BlobProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()), // Peer closed; clean exit.
            };

            // Limit reads on the request side to MAX_REQUEST_SIZE worth of bytes.
            let req = match read_request(&mut recv).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = write_response(
                        &mut send,
                        &BlobFetchResponse::Error(format!("bad request: {e}")),
                    )
                    .await;
                    let _ = send.finish();
                    continue;
                }
            };

            let resp = match Hash::from_hex(&req.blake3_hex) {
                Err(e) => BlobFetchResponse::Error(format!("invalid hash: {e}")),
                Ok(h) => match self.store.get_bytes(h) {
                    Ok(bytes) => BlobFetchResponse::Found(bytes),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        BlobFetchResponse::NotFound
                    }
                    Err(e) => BlobFetchResponse::Error(format!("store error: {e}")),
                },
            };

            let _ = write_response(&mut send, &resp).await;
            let _ = send.finish();

            // Suppress unused-import warning when MAX_REQUEST_SIZE is only
            // used in blob_protocol's read path; touch it here for symmetry.
            let _ = MAX_REQUEST_SIZE;
        }
    }
}
```

- [ ] **Step 3: Mount in mod.rs**

```rust
mod blake3_store;
mod blob_handler;
mod blob_protocol;
mod config;
mod endpoint;
mod identity;

pub use blake3_store::{Blake3Store, Hash};
pub use blob_handler::BlobProtocolHandler;
pub use blob_protocol::{BlobFetchRequest, BlobFetchResponse};
pub use config::{BLOB_ALPN, IrohConfig};
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 4: Run tests**

```bash
just test-iroh -- p2p_iroh::blob_protocol
```
Expected: 3 tests pass. No tests for `blob_handler` yet — covered by Task 9 round-trip.

**API drift to verify:** `iroh::protocol::ProtocolHandler` trait signature, `iroh::endpoint::Connection::accept_bi()` return shape. Adjust if renamed.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): blob protocol primitives + inbound ProtocolHandler"
```

---

## Task 8: Router with custom ALPN registration

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/router.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/router.rs`:

```rust
//! Assemble the iroh `Router` with our custom blob protocol handler.

use iroh::Endpoint;
use iroh::protocol::Router;

use super::blob_handler::BlobProtocolHandler;
use super::config::BLOB_ALPN;

/// Build a `Router` that accepts BLOB_ALPN, dispatching to the supplied
/// handler. The `Router` owns its accept loop; drop it (or call `shutdown`)
/// to stop accepting new connections.
pub fn build_router(endpoint: Endpoint, handler: BlobProtocolHandler) -> Router {
    Router::builder(endpoint)
        .accept(BLOB_ALPN, handler)
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p_iroh::{
        blake3_store::Blake3Store, build_endpoint, BlobProtocolHandler, IrohConfig,
    };
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn router_spawns_and_shuts_down() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            alpns: vec![BLOB_ALPN.to_vec()],
        };
        let endpoint = build_endpoint(&cfg).await.unwrap();
        let store = Arc::new(Blake3Store::open(&cfg.blobs_dir).unwrap());
        let handler = BlobProtocolHandler::new(store);

        let router = build_router(endpoint, handler);
        router.shutdown().await.expect("clean shutdown");
    }
}
```

- [ ] **Step 2: Mount in mod.rs**

```rust
mod blake3_store;
mod blob_handler;
mod blob_protocol;
mod config;
mod endpoint;
mod identity;
mod router;

pub use blake3_store::{Blake3Store, Hash};
pub use blob_handler::BlobProtocolHandler;
pub use blob_protocol::{BlobFetchRequest, BlobFetchResponse};
pub use config::{BLOB_ALPN, IrohConfig};
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use router::build_router;
```

- [ ] **Step 3: Run test**

```bash
just test-iroh -- p2p_iroh::router
```
Expected: 1 test passes.

**API drift to verify:** `Router::builder().accept(alpn, handler).spawn()` shape; `router.shutdown().await` return type.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): Router with custom BLOB_ALPN registered"
```

---

## Task 9: Outbound fetch helper + IrohNode + round-trip integration test

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/fetch.rs`
- Create: `elohim/elohim-storage/src/p2p_iroh/node.rs`
- Create: `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

This is the milestone task: the first proof that two iroh nodes can move bytes.

- [ ] **Step 1: Write fetch.rs**

Create `elohim/elohim-storage/src/p2p_iroh/fetch.rs`:

```rust
//! Outbound blob fetch over iroh QUIC.

use anyhow::{anyhow, Context, Result};
use iroh::{Endpoint, EndpointAddr};

use super::blake3_store::Hash;
use super::blob_protocol::{
    read_response, write_request, BlobFetchRequest, BlobFetchResponse, DEFAULT_MAX_RESPONSE_SIZE,
};
use super::config::BLOB_ALPN;

/// Fetch a blob from `peer_addr` by its BLAKE3 hash. Verifies the returned
/// bytes hash to the requested hash before returning.
pub async fn fetch_blob(
    endpoint: &Endpoint,
    peer_addr: EndpointAddr,
    hash: Hash,
) -> Result<Vec<u8>> {
    let conn = endpoint
        .connect(peer_addr, BLOB_ALPN)
        .await
        .context("iroh: connect to peer")?;

    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .context("iroh: open bidi stream")?;

    let req = BlobFetchRequest { blake3_hex: hash.to_hex() };
    write_request(&mut send, &req).await.context("write request")?;
    send.finish().context("finish send side")?;

    let resp = read_response(&mut recv, DEFAULT_MAX_RESPONSE_SIZE)
        .await
        .context("read response")?;

    let bytes = match resp {
        BlobFetchResponse::Found(b) => b,
        BlobFetchResponse::NotFound => {
            return Err(anyhow!("peer reported NotFound for {}", hash.to_hex()))
        }
        BlobFetchResponse::Error(msg) => {
            return Err(anyhow!("peer error: {}", msg))
        }
    };

    // Verify hash before returning — the peer is untrusted.
    let actual = Hash(blake3::hash(&bytes).into());
    if actual != hash {
        return Err(anyhow!(
            "hash mismatch: requested {} got {}",
            hash.to_hex(),
            actual.to_hex()
        ));
    }
    Ok(bytes)
}
```

- [ ] **Step 2: Write node.rs**

Create `elohim/elohim-storage/src/p2p_iroh/node.rs`:

```rust
//! Long-lived iroh P2P node aggregate.

use std::sync::Arc;

use anyhow::Result;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, NodeId};

use super::{
    blake3_store::{Blake3Store, Hash},
    blob_handler::BlobProtocolHandler,
    build_endpoint, build_router,
    config::IrohConfig,
    fetch::fetch_blob,
};

pub struct IrohNode {
    endpoint: Endpoint,
    router: Router,
    store: Arc<Blake3Store>,
}

impl IrohNode {
    pub async fn start(config: &IrohConfig) -> Result<Self> {
        let endpoint = build_endpoint(config).await?;
        let store = Arc::new(Blake3Store::open(&config.blobs_dir)?);
        let handler = BlobProtocolHandler::new(store.clone());
        let router = build_router(endpoint.clone(), handler);
        Ok(Self { endpoint, router, store })
    }

    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    pub async fn node_addr(&self) -> Result<EndpointAddr> {
        Ok(self.endpoint.node_addr().await?)
    }

    pub fn add_bytes(&self, bytes: &[u8]) -> Result<Hash> {
        Ok(self.store.add_bytes(bytes)?)
    }

    pub fn get_bytes(&self, hash: Hash) -> Result<Vec<u8>> {
        Ok(self.store.get_bytes(hash)?)
    }

    pub fn has(&self, hash: Hash) -> bool {
        self.store.has(hash)
    }

    pub async fn fetch_blob_from(
        &self,
        peer_addr: EndpointAddr,
        hash: Hash,
    ) -> Result<Vec<u8>> {
        if self.store.has(hash) {
            return Ok(self.store.get_bytes(hash)?);
        }
        let bytes = fetch_blob(&self.endpoint, peer_addr, hash).await?;
        // Persist locally so subsequent fetches are no-ops.
        self.store.add_bytes(&bytes)?;
        Ok(bytes)
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        self.endpoint.close().await;
        Ok(())
    }
}
```

- [ ] **Step 3: Mount in mod.rs**

```rust
mod blake3_store;
mod blob_handler;
mod blob_protocol;
mod config;
mod endpoint;
mod fetch;
mod identity;
mod node;
mod router;

pub use blake3_store::{Blake3Store, Hash};
pub use blob_handler::BlobProtocolHandler;
pub use blob_protocol::{BlobFetchRequest, BlobFetchResponse};
pub use config::{BLOB_ALPN, IrohConfig};
pub use endpoint::build_endpoint;
pub use fetch::fetch_blob;
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use node::IrohNode;
pub use router::build_router;
```

- [ ] **Step 4: Write the round-trip integration test**

Create `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs`:

```rust
//! Two-node iroh blob round-trip — the milestone for Phase 1.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode, BLOB_ALPN};
use tempfile::tempdir;

#[tokio::test]
async fn two_nodes_can_round_trip_a_blob() {
    let provider_dir = tempdir().unwrap();
    let provider_cfg = IrohConfig {
        blobs_dir: provider_dir.path().join("blobs_iroh"),
        secret_key_path: provider_dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![BLOB_ALPN.to_vec()],
    };
    let provider = IrohNode::start(&provider_cfg).await.expect("provider starts");

    let fetcher_dir = tempdir().unwrap();
    let fetcher_cfg = IrohConfig {
        blobs_dir: fetcher_dir.path().join("blobs_iroh"),
        secret_key_path: fetcher_dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![BLOB_ALPN.to_vec()],
    };
    let fetcher = IrohNode::start(&fetcher_cfg).await.expect("fetcher starts");

    // Provider adds a blob.
    let payload = b"two-node iroh round trip works on loopback".to_vec();
    let hash = provider.add_bytes(&payload).expect("add bytes");

    // Get provider's address (with direct addrs / relay) for the fetcher.
    let provider_addr = provider.node_addr().await.expect("provider addr");

    // Fetcher pulls.
    let fetched = fetcher
        .fetch_blob_from(provider_addr.clone(), hash)
        .await
        .expect("fetch blob");
    assert_eq!(fetched, payload);

    // Idempotent: second fetch returns from local store.
    let fetched_again = fetcher
        .fetch_blob_from(provider_addr, hash)
        .await
        .expect("idempotent fetch");
    assert_eq!(fetched_again, payload);

    fetcher.shutdown().await.expect("fetcher shutdown");
    provider.shutdown().await.expect("provider shutdown");
}
```

- [ ] **Step 5: Run the integration test**

```bash
just test-iroh --test iroh_blob_roundtrip
```
Expected: 1 test passes. May take 10-30s for QUIC handshake + transfer.

**If it hangs:** with `use_n0_relays: false`, both nodes need direct UDP reachability. On loopback this should work; if blocked, set `use_n0_relays: true` and accept the n0 relay dependency for CI. Not acceptable for production but fine for the round-trip test.

If `node_addr()` returns before any addresses are discovered (race), insert `tokio::time::sleep(Duration::from_millis(200)).await` before calling it. Refactor to a deterministic ready-signal in a follow-up.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/fetch.rs \
        elohim/elohim-storage/src/p2p_iroh/node.rs \
        elohim/elohim-storage/src/p2p_iroh/mod.rs \
        elohim/elohim-storage/tests/iroh_blob_roundtrip.rs
git commit -m "feat(storage): two-node iroh blob round-trip via custom QUIC protocol"
```

---

## Task 10: TransportBackend config + main.rs branch + lifecycle smoke

**Files:**
- Modify: `elohim/elohim-storage/src/config.rs`
- Modify: `elohim/elohim-storage/src/main.rs`
- Create: `elohim/elohim-storage/tests/iroh_node_lifecycle.rs`

- [ ] **Step 1: Add the enum and field; write config tests**

In `elohim/elohim-storage/src/config.rs`, near the top (after imports), add:

```rust
/// Which P2P transport backend the daemon should use at startup.
///
/// In Phase 1 only `Libp2p` is fully featured; `Iroh` enables the parallel
/// stack covering blob distribution only. See
/// `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransportBackend {
    #[default]
    Libp2p,
    Iroh,
}
```

In the `Config` struct, append:

```rust
    /// Which P2P backend to start. Default `Libp2p`. Override via TOML
    /// (`transport_backend = "iroh"`) or env var `ELOHIM_TRANSPORT_BACKEND`.
    #[serde(default)]
    pub transport_backend: TransportBackend,
```

Update the `Default` impl (or rely on the derive if `Config` derives Default with all fields defaulting). Default value: `TransportBackend::Libp2p`.

In the existing `#[cfg(test)] mod tests` block (or add one), append:

```rust
#[test]
fn transport_backend_defaults_to_libp2p() {
    let cfg = Config::default();
    assert_eq!(cfg.transport_backend, TransportBackend::Libp2p);
}

#[test]
fn transport_backend_round_trips_through_toml() {
    let cfg = Config {
        transport_backend: TransportBackend::Iroh,
        ..Config::default()
    };
    let s = toml::to_string(&cfg).unwrap();
    let back: Config = toml::from_str(&s).unwrap();
    assert_eq!(back.transport_backend, TransportBackend::Iroh);
}
```

- [ ] **Step 2: Run config tests**

```bash
just test-iroh -- config::tests::transport_backend
```
Expected: 2 tests pass.

- [ ] **Step 3: Branch in main.rs**

Edit `elohim/elohim-storage/src/main.rs`. Find the existing P2P startup block (search for `P2PNode::new` or `P2PConfig`). Wrap with the backend match.

```rust
use elohim_storage::config::TransportBackend;

// ... in the runtime startup function ...
match config.transport_backend {
    TransportBackend::Libp2p => {
        #[cfg(feature = "p2p")]
        {
            // ... existing libp2p P2PNode startup, indented one level ...
        }
        #[cfg(not(feature = "p2p"))]
        {
            anyhow::bail!("config.transport_backend = libp2p but the `p2p` feature is not compiled in");
        }
    }
    TransportBackend::Iroh => {
        #[cfg(feature = "p2p-iroh")]
        {
            use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
            let iroh_cfg = IrohConfig::from_storage_dir(&config.storage_dir);
            let _iroh_node = IrohNode::start(&iroh_cfg).await?;
            tracing::info!(
                node_id = %_iroh_node.node_id(),
                "iroh node started (Phase 1 — blob plane only; libp2p protocols disabled)"
            );
            // Phase 1: hold the node alive until shutdown; HTTP/inventory
            // wiring graduates in Phase 2.
        }
        #[cfg(not(feature = "p2p-iroh"))]
        {
            anyhow::bail!("config.transport_backend = iroh but the `p2p-iroh` feature is not compiled in");
        }
    }
}
```

- [ ] **Step 4: Single-node smoke test**

Create `elohim/elohim-storage/tests/iroh_node_lifecycle.rs`:

```rust
#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode, BLOB_ALPN};
use tempfile::tempdir;

#[tokio::test]
async fn node_starts_and_stops_clean() {
    let dir = tempdir().unwrap();
    let cfg = IrohConfig {
        blobs_dir: dir.path().join("blobs_iroh"),
        secret_key_path: dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![BLOB_ALPN.to_vec()],
    };
    let node = IrohNode::start(&cfg).await.expect("starts");
    let _id = node.node_id();
    node.shutdown().await.expect("clean shutdown");
}
```

- [ ] **Step 5: Run the smoke test**

```bash
just test-iroh --test iroh_node_lifecycle
```
Expected: 1 test passes in <2s.

- [ ] **Step 6: Verify daemon starts in iroh mode**

```bash
just build-iroh
ELOHIM_TRANSPORT_BACKEND=iroh \
RUST_LOG=elohim_storage=info \
./target/debug/elohim-storage --storage-dir /tmp/iroh-smoke
```

Expected log line: `iroh node started (Phase 1 — blob plane only; libp2p protocols disabled)`. Ctrl-C; clean shutdown.

If the daemon errors on missing libp2p side effects (HTTP routes that need P2PHandle, etc.), document them as TODO comments near the iroh branch. Phase 2 graduates them.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/config.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/tests/iroh_node_lifecycle.rs
git commit -m "feat(storage): TransportBackend config + iroh boot path"
```

---

## Task 11: Graduation gates README + final gate run

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/README.md`

- [ ] **Step 1: Write the README**

Create `elohim/elohim-storage/src/p2p_iroh/README.md`:

```markdown
# p2p_iroh — Phase 1: Blob plane (iroh-standalone)

Parallel iroh-based P2P module sitting alongside `crate::p2p`. Selectable at
runtime via `Config::transport_backend = TransportBackend::Iroh`.

## Status

**Preview.** Uses iroh 1.0.0-rc.0, the first 1.0 release candidate (2026-05-07).
Custom QUIC blob protocol with BLAKE3 hashing in our own code; iroh-blobs is
intentionally not used because it has hard incompatibilities with our existing
`multihash-codetable` dep. Keep `Libp2p` as default until graduation.

## What works

- Two-node blob round-trip via direct EndpointAddr fetch (LAN/loopback)
- BLAKE3 content addressing on the iroh path
- BLAKE3-keyed minimal filesystem store at `<storage_dir>/blobs_iroh/`
- Persisted iroh `SecretKey` at `<storage_dir>/iroh.key`
- Graceful startup and shutdown

## What does not work yet (Phase 2+)

- HTTP `/api/v1/blob/{hash}` route does NOT serve from `Blake3Store` — the
  route still reads from the legacy SHA256 `BlobStore`.
- Genesis seeder writes to the legacy `BlobStore`, not `Blake3Store`.
- The existing `peer_blob_inventory` projection records SHA256 hashes; no
  BLAKE3-mode integration exists.
- Other libp2p protocols (sync, shard, epr, trust, identity, gossip, kad,
  mdns, dcutr, autonat) have no iroh equivalents.
- iroh-blobs is not used. When n0 publishes a release aligned with iroh 1.0
  (estimated 4-12 weeks based on cadence), revisit to swap our minimal
  Blake3Store + custom protocol for iroh-blobs's FsStore + BlobsProtocol.

## Graduation gates (must clear all before flipping default)

1. iroh 1.0 stable shipped (or our `=1.0.0-rc.X` pin proven stable for ≥1
   month with no regressions).
2. HTTP `/api/v1/blob/{hash}` reads from `Blake3Store` when iroh-mode is
   active; route accepts BLAKE3 hashes (or both, with prefix discriminator).
3. Genesis seeder graduated to write through `Blake3Store`.
4. Inventory tables migrated to BLAKE3 keys (or — pre-launch — formal
   wipe-and-reseed runbook documented).
5. Sync, shard, EPR, and identity protocols graduate to iroh ALPNs (or
   the daemon refuses to start in iroh mode if any required protocol is
   missing, with a clear error).
6. Round-trip + lifecycle tests run in CI on every PR.
7. At least one alpha-cluster household runs in iroh mode for a full week
   without regression.
8. Reconsider iroh-blobs adoption: if n0 has shipped a 1.0-aligned release
   and the multihash-codetable conflict resolves, plan the swap.

## Cutover playbook (for the day we flip the default)

1. Confirm all graduation gates green.
2. Branch: `git checkout -b feat/iroh-default-backend`.
3. Edit `Cargo.toml`: `default = ["p2p", "p2p-iroh"]`.
4. Edit `src/config.rs`: `TransportBackend::default() = Iroh`.
5. Run full test suite under both `--features p2p` and
   `--features "p2p p2p-iroh"`.
6. Alpha cluster e2e: wipe `<storage_dir>` on every node, run seeder,
   confirm content distributes via iroh.
7. Update `CLAUDE.md` and `genesis/graphos/vocabulary.md` if any vocabulary
   changes (BLAKE3 hash format clarification on `/blob/{hash}`).
8. Open a PR; tag the alpha-cluster operators.

## Code map

| File | Purpose |
|------|---------|
| `mod.rs` | Module root + re-exports |
| `config.rs` | `IrohConfig`, `BLOB_ALPN` |
| `identity.rs` | Persisted `SecretKey` |
| `endpoint.rs` | `Endpoint` builder |
| `blake3_store.rs` | BLAKE3-keyed file store |
| `blob_protocol.rs` | Wire format + framing |
| `blob_handler.rs` | Inbound `ProtocolHandler` |
| `router.rs` | `Router` with custom ALPN |
| `fetch.rs` | Outbound fetch helper |
| `node.rs` | `IrohNode` aggregate |

Tests: `tests/iroh_blob_roundtrip.rs`, `tests/iroh_node_lifecycle.rs`.
```

- [ ] **Step 2: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/README.md
git commit -m "docs(storage): p2p_iroh README — status, graduation gates, cutover playbook"
```

- [ ] **Step 3: Run the full feature build + tests**

```bash
just build-iroh
just test-iroh
```
Expected: builds clean, all tests pass (libp2p tests + new iroh tests).

- [ ] **Step 4: Run pre-push gate**

```bash
just gate
```
Expected: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` all green.

---

## Self-Review Notes

**Spec coverage:**
- Phase 0: holochain bump (single discovery-driven task).
- Phase 1: parallel module — Tasks 1-2.
- BLAKE3 content addressing on iroh path — Task 6.
- Custom blob protocol on QUIC — Tasks 7-8.
- Two-node round-trip — Task 9.
- Runtime config toggle — Task 10.
- Graduation gates + cutover playbook — Task 11.

**Out of scope (intentional):** Each runs its own design gate when graduated.
- Other libp2p protocols' migration (sync/shard/epr/trust/identity)
- HTTP route surface graduation
- Genesis seeder rewrite
- Inventory projection migration to BLAKE3
- Self-hosted iroh-relay
- Cross-stack identity unification
- iroh-blobs adoption (deferred until n0 ships a 1.0-aligned release)

**Known API uncertainty:** iroh 1.0.0-rc.0 was published 2026-05-07. Each task that touches iroh API surface flags specific calls to verify against current docs.rs. API drift is contained per-task.

**Phase 0 risk:** holochain dev.5 → dev.22 is 17 dev releases of API drift. Known: `AdminRequest::GraftRecords` removed, `kitsune2_api::url::Url` namespacing changed. Phase 0 explicitly time-boxes — if scope balloons beyond 1-3 days of work, STOP and report BLOCKED rather than continuing into Phase 1 with a half-bumped holochain.
