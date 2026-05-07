# iroh Parallel P2P Stack — Phase 1 (Blob Plane)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a parallel iroh-based P2P module inside `elohim-storage` that can serve and fetch content-addressed blobs end-to-end via iroh-blobs (BLAKE3, QUIC, n0 relays). Existing libp2p stack remains the default; iroh becomes selectable via feature flag + runtime config. First cutover target: blob distribution. Other protocols (sync, shard, epr, trust, identity) stay on libp2p and graduate in subsequent sprints.

**Architecture:** A new `src/p2p_iroh/` module sits alongside `src/p2p/`, gated by `p2p-iroh` Cargo feature. Both backends compile additively; runtime config (`TransportBackend::Libp2p` | `TransportBackend::Iroh`) picks one at startup. In `Iroh` mode, blob storage is `iroh_blobs::store::fs::FsStore` (BLAKE3-addressed, redb-backed) — the existing SHA256-keyed `BlobStore` is bypassed. We are pre-launch with disposable seed data, so cutover is "flip the flag, wipe `storage_dir`, reseed."

**Tech Stack:** `iroh = "1.0.0-rc.0"` (core endpoint + router + protocols), `iroh-blobs = "0.100.0"` (BLAKE3 content store + downloader), tokio, rmp-serde (unchanged for surrounding RPC), Diesel/SQLite (unchanged for inventory metadata).

**Version pinning rationale:** iroh 1.0.0-rc.0 dropped 2026-05-07 (the day this plan was written) and has had no soak time. iroh-blobs 0.100 is on a separate spinout cadence and the n0 team explicitly says "wait 1-2 more releases before production." We pin to specific patch versions, treat the iroh path as preview-quality until graduation criteria (Task 11) are met, and never set `p2p-iroh` in `default` features for this phase.

---

## P2P Design Gate: iroh parallel stack — Phase 1

This plan introduces two new persistent on-disk artifacts. Classifying each before any code is written, per `.claude/skills/p2p-design-gate/SKILL.md`.

### Entity: iroh-blobs FsStore (`<storage_dir>/blobs_iroh/`)

- **Classification**: **Operational (Category C)** — equivalent role to the existing SHA256-keyed `crate::blob_store::BlobStore`, which is also operational. The redb index + chunk files are a local cache of content-addressed bytes that any peer holding the same hash can re-supply.
- **Justification**: The bytes are already content-addressed (BLAKE3). Loss of this store on a single node means re-fetching from peers, not loss of source of truth. The collective peer inventory is the broader source of truth; any single node's store is a projection of what it currently holds.
- **Content Address Strategy**: **Content-Derived (BLAKE3 hash)** — the hash IS the identity. Note this is **not** CIDv1 (`bafkrei...`); iroh-blobs uses raw BLAKE3 32-byte hashes. The two address formats are deliberately disjoint: SHA256/CIDv1 on the libp2p path, BLAKE3 on the iroh path. They are not interchangeable; each path is canonical within its mode.
- **Address Justification**: Content-derived because the bytes are immutable. BLAKE3 (not CIDv1) because that is iroh-blobs's native addressing — wrapping it in CIDv1 would force a translation layer for no architectural gain in Phase 1.
- **Source of Truth**: **SQLite/local (operational)** — actually iroh-blobs's own redb, not SQLite. No `dht_anchor_hash` column applies because there is no SQL table; iroh-blobs manages its internal layout. Reconstruction strategy: re-fetch any missing blob from peers via BlobTicket given the hash.
- **Coordinator Zome**: N/A — this is a sidecar storage layer below the API boundary. Content authoring still goes through coordinator zomes (e.g., lamad content creation); only the byte storage layer is touched here.
- **Storage Projection**: redb at `<storage_dir>/blobs_iroh/` (iroh-blobs internal layout). Phase 1 does not project into any SQL table. Phase 2 graduation will integrate this with `peer_blob_inventory` and that integration will run its own design gate at that time.
- **HTTP Route**: None in Phase 1. Phase 2 graduates `/api/v1/blob/{hash}` to read from this store when in iroh mode (separate plan).
- **Anti-Pattern Check**: ✓ Source of truth declared (operational, reconstructable). ✓ Address format declared (BLAKE3, scoped to iroh mode). ✓ No new DHT entry type. ✓ No UUID introduced — content-derived addressing preserved.

### Entity: iroh secret key file (`<storage_dir>/iroh.key`)

- **Classification**: **Agent-Scoped (Category B)** — peer to the existing libp2p node keypair. Private to this node; identifies it on the iroh transport.
- **Justification**: The ed25519 secret is private credential material for the storage node. No other peer should ever see it. Loss = identity rotation (new EndpointId, peers must re-discover).
- **Content Address Strategy**: **Slug/UUID-equivalent** (the key bytes themselves) — but justified because this is identity material, not content. The public derivation (EndpointId) is the addressable form.
- **Source of Truth**: **Private local filesystem** — never gossipped, never replicated, file-mode 0600 on Unix.
- **Coordinator Zome**: N/A — not a Holochain entity.
- **HTTP Route**: None — never exposed.
- **Anti-Pattern Check**: ✓ Not in a shared table. ✓ File-mode tightening on creation. ✓ Distinct from libp2p identity by design (Phase 1 keeps stacks identity-disjoint; cross-stack identity unification is a later sprint, also gated).

### Design Constraints Discovered

- **Two canonical address formats coexist during Phase 1.** SHA256/CIDv1 on the libp2p path, BLAKE3 on the iroh path. Each is canonical within its mode; runtime config selects exactly one mode at startup. This is documented in the `p2p_iroh/README.md` (Task 11) and is a *temporary* state — the cutover playbook converges on BLAKE3 once libp2p is retired.
- **`peer_blob_inventory` is not modified in this plan.** The README's "what does not work yet" list flags inventory schema as a Phase 2 concern; Phase 2 will run its own design gate when it graduates the inventory layer to BLAKE3 (or to a hash-format-discriminated schema). No schema migration ships in Phase 1.
- **Genesis seeder is not modified in this plan.** It continues to write SHA256-keyed blobs to the legacy `BlobStore`. Phase 2 graduates the seeder; that graduation will run its own gate.
- **No new HTTP route surface.** All HTTP routes continue to read from the legacy `BlobStore`. Phase 1 only stands up the iroh dataplane and proves a two-node round-trip; route surface graduation is Phase 2.

---

## File Structure

### New files

| File | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/p2p_iroh/mod.rs` | Module root; re-exports `IrohNode`, `IrohConfig`, `IrohBlobStore`, `IrohCommand`. Feature-gated as a whole. |
| `elohim/elohim-storage/src/p2p_iroh/config.rs` | `IrohConfig` struct (listen addr, secret key path, blobs dir, relay mode, ALPN list). |
| `elohim/elohim-storage/src/p2p_iroh/identity.rs` | Load-or-generate `iroh::SecretKey`, persist as raw bytes to `<storage_dir>/iroh.key`. |
| `elohim/elohim-storage/src/p2p_iroh/endpoint.rs` | `build_endpoint(&IrohConfig)` returns a configured `iroh::Endpoint` with ALPNs registered. |
| `elohim/elohim-storage/src/p2p_iroh/blob_store.rs` | `IrohBlobStore` wrapper around `iroh_blobs::store::fs::FsStore` — `add_bytes`, `get_bytes`, `has`, `BlobsProtocol` accessor. |
| `elohim/elohim-storage/src/p2p_iroh/router.rs` | `build_router(endpoint, blob_store)` — assembles `iroh::protocol::Router` with `BlobsProtocol` accepting iroh-blobs ALPN. |
| `elohim/elohim-storage/src/p2p_iroh/fetch.rs` | `fetch_blob(endpoint, ticket) -> Result<Vec<u8>, FetchError>` — single-source fetch via the downloader API. |
| `elohim/elohim-storage/src/p2p_iroh/node.rs` | `IrohNode` aggregate: owns endpoint, router, store; exposes `addr()`, `add_blob()`, `fetch_blob()`, `shutdown()`. |
| `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs` | Two-node integration test: provider adds blob → fetcher pulls it via ticket → bytes match. |
| `elohim/elohim-storage/tests/iroh_node_lifecycle.rs` | Single-node smoke test: build, listen, shutdown cleanly. |

### Modified files

| File | Change |
|------|--------|
| `elohim/elohim-storage/Cargo.toml` | Add optional `iroh` and `iroh-blobs` deps; add `p2p-iroh` feature. |
| `elohim/elohim-storage/src/lib.rs` | Mount `p2p_iroh` module behind `#[cfg(feature = "p2p-iroh")]`; re-export public API. |
| `elohim/elohim-storage/src/config.rs` | Add `TransportBackend` enum (`Libp2p` | `Iroh`); add `transport_backend` field to `Config` (default `Libp2p`); load from TOML/env (`ELOHIM_TRANSPORT_BACKEND`). |
| `elohim/elohim-storage/src/main.rs` | Branch on `config.transport_backend` at boot: spawn either `P2PNode` or `IrohNode`. |

### Out of scope (Phase 1)

- Other libp2p protocols (sync, shard, epr, trust, identity, gossip, kad, mdns, dcutr, autonat) — remain on libp2p only.
- Migration of inventory tables to BLAKE3 keys — pre-launch wipe-and-reseed, not in this plan.
- Self-hosted iroh-relay — n0 hosted relays for now via `RelayMode::Default`.
- Genesis seeder rewrite to add blobs through iroh-blobs — separate sprint.
- HTTP `/api/v1/blob/{hash}` route surface — phase 2 once iroh-mode is the default.

---

## Pre-flight: Read these before starting

1. **`elohim/elohim-storage/CLAUDE.md`** — API boundary architecture; iroh module sits below the API boundary, must not change view types.
2. **`elohim/elohim-storage/src/p2p/blob_protocol.rs`** — current libp2p blob protocol; reference only, do not modify.
3. **`elohim/elohim-storage/src/blob_store.rs`** — current SHA256-keyed BlobStore; reference for what API surface `IrohBlobStore` is replacing on the iroh path.
4. **iroh docs at task time** — iroh 1.0.0-rc.0 was published 2026-05-07. Verify API signatures against `https://docs.rs/iroh/1.0.0-rc.0/iroh/` and `https://docs.rs/iroh-blobs/0.100.0/iroh_blobs/` before each task. The code in this plan reflects the README/docs as of 2026-05-07; if a method has been renamed, follow the docs and update the plan.

**Build incantation for every cargo command in this plan:**
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo <subcommand> --features p2p-iroh
```
The `RUSTFLAGS` override is required because Holochain deps depend on a custom getrandom backend; the storage crate uses it for native targets via `src/getrandom_custom.rs`.

The `justfile` at `elohim/elohim-storage/justfile` already exports `RUSTFLAGS`; you can use `just build` / `just test` instead, but features must be passed as `cargo` flags inside the recipes. Add a `just build-iroh` and `just test-iroh` recipe in Task 1.

---

## Task 1: Add iroh dependencies and feature flag

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/justfile`

- [ ] **Step 1: Add optional iroh dependencies**

Edit `elohim/elohim-storage/Cargo.toml`. After the libp2p block (around line 171), before the `futures` dep at line 174, add:

```toml
# iroh — QUIC-based P2P transport (parallel stack, Phase 1)
# Pinned to 1.0.0-rc.0 published 2026-05-07; bump explicitly, don't track.
iroh = { version = "=1.0.0-rc.0", optional = true }
# iroh-blobs — BLAKE3 content-addressed blob store + downloader.
# n0 says "wait 1-2 more releases before production"; we treat as preview-quality.
iroh-blobs = { version = "=0.100.0", default-features = false, features = ["fs-store"], optional = true }
```

- [ ] **Step 2: Add p2p-iroh feature**

In the `[features]` section (lines 179-182), change:

```toml
[features]
default = ["p2p"]
compression = ["lz4_flex"]
p2p = ["libp2p", "futures"]
p2p-iroh = ["iroh", "iroh-blobs", "futures"]
```

`p2p-iroh` is **not** added to `default`. Both `p2p` and `p2p-iroh` can be enabled simultaneously (additive); the parity test harness in Task 10 needs both.

- [ ] **Step 3: Add justfile recipes**

Edit `elohim/elohim-storage/justfile`. Find the existing `build` and `test` recipes; add below them:

```makefile
# Build with iroh parallel stack enabled (libp2p still default)
build-iroh:
    cargo build --features "p2p p2p-iroh"

# Test with both stacks compiled in
test-iroh:
    cargo test --features "p2p p2p-iroh"
```

- [ ] **Step 4: Verify the feature compiles (with no code yet)**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
just build-iroh
```
Expected: builds successfully, downloads `iroh` and `iroh-blobs` from crates.io. No new code is required because the deps are optional and unused.

If the build fails with version-resolution errors (e.g., `iroh` not found at `=1.0.0-rc.0`), check `https://crates.io/crates/iroh/versions` and pin to the latest 1.0.0-rcN that exists. If 1.0.0 stable has shipped by task time, use it instead. Update this plan's version note.

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

- [ ] **Step 1: Create the module root**

Create `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
//! Parallel iroh-based P2P stack (Phase 1 — blob plane only).
//!
//! Sibling to [`crate::p2p`]; gated by the `p2p-iroh` Cargo feature. In iroh
//! mode the blob store is `iroh_blobs::store::fs::FsStore` (BLAKE3-addressed)
//! rather than [`crate::blob_store::BlobStore`] (SHA256). The two stacks are
//! mutually exclusive at runtime — selected by [`crate::config::TransportBackend`]
//! at startup — but both compile additively so the parity test harness can
//! exercise them in one binary.
//!
//! See `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`.

// Submodules will be added in subsequent tasks.
```

- [ ] **Step 2: Mount the module in lib.rs behind a feature gate**

Edit `elohim/elohim-storage/src/lib.rs`. Find the existing `#[cfg(feature = "p2p")] pub mod p2p;` line (around lines 62-67). Add directly below it:

```rust
#[cfg(feature = "p2p-iroh")]
pub mod p2p_iroh;
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
just build-iroh
```
Expected: builds successfully. No warnings about unused module — the empty `mod.rs` only has doc comments, which is fine.

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

/// Configuration for the iroh-based P2P node.
#[derive(Debug, Clone)]
pub struct IrohConfig {
    /// Directory holding the iroh-blobs filesystem store (redb + chunk files).
    /// We default to `<storage_dir>/blobs_iroh/` to keep it disjoint from the
    /// SHA256-keyed legacy `<storage_dir>/blobs/`.
    pub blobs_dir: PathBuf,

    /// Path to the persisted iroh secret key. Generated on first run.
    pub secret_key_path: PathBuf,

    /// Whether to use n0's hosted relay infrastructure. `true` matches
    /// `RelayMode::Default`; `false` matches `RelayMode::Disabled` (only useful
    /// when both sides have routable addresses, e.g. household LAN).
    pub use_n0_relays: bool,

    /// ALPN identifiers this endpoint will accept. Phase 1 only registers the
    /// iroh-blobs ALPN; later phases extend this list as protocols graduate.
    pub alpns: Vec<Vec<u8>>,
}

impl IrohConfig {
    /// Construct a default config rooted at `storage_dir`.
    pub fn from_storage_dir(storage_dir: &std::path::Path) -> Self {
        Self {
            blobs_dir: storage_dir.join("blobs_iroh"),
            secret_key_path: storage_dir.join("iroh.key"),
            use_n0_relays: true,
            alpns: vec![iroh_blobs::ALPN.to_vec()],
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
        assert_eq!(cfg.alpns[0], iroh_blobs::ALPN);
    }

    #[test]
    fn alpns_does_not_collide_with_legacy_blob_dir() {
        let cfg = IrohConfig::from_storage_dir(Path::new("/x"));
        assert_ne!(cfg.blobs_dir.file_name().unwrap(), "blobs");
    }
}
```

- [ ] **Step 2: Mount config in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`. Add at the end:

```rust
mod config;

pub use config::IrohConfig;
```

- [ ] **Step 3: Run test to verify it passes**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- p2p_iroh::config
```
Expected: 2 tests pass.

If `iroh_blobs::ALPN` is not found, check `https://docs.rs/iroh-blobs/0.100.0/iroh_blobs/` for the correct constant path. As of plan-write the constant is at the crate root.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): add IrohConfig with disjoint blobs_iroh dir"
```

---

## Task 4: Identity — load-or-generate iroh secret key

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/identity.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/identity.rs`:

```rust
//! Persisted iroh secret key. Load if present, generate-and-write if not.
//!
//! Stored as 32 raw bytes (the ed25519 secret) at the configured path. Distinct
//! from any libp2p keypair file — the two stacks have separate identities in
//! Phase 1; cross-stack identity unification is a later sprint.

use std::path::Path;
use std::{fs, io};

use iroh::SecretKey;

/// Load a secret key from `path`, or generate a fresh one and persist it if
/// the file does not exist.
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
            // Best-effort permissions tightening on Unix; ignore failure on
            // platforms that don't support it.
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
        // Public id stable for this key
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

- [ ] **Step 2: Add `rand` to Cargo if not already present, and check it's in scope**

Check `elohim/elohim-storage/Cargo.toml` for an existing `rand` dependency. There's no existing `rand` direct dep but iroh re-exports it as a transitive — verify with:

```bash
cd /projects/elohim/elohim/elohim-storage
cargo tree --features p2p-iroh -e normal | grep '^rand '
```

If `rand 0.8.x` is present transitively, add it explicitly so the import is stable:

```toml
# In [dependencies]:
rand = { version = "0.8", optional = true }
```

And update the `p2p-iroh` feature:
```toml
p2p-iroh = ["iroh", "iroh-blobs", "futures", "rand"]
```

If iroh 1.0.0-rc.0 has migrated to `rand` 0.9, match that version instead. Verify with `cargo tree`.

- [ ] **Step 3: Mount identity in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod config;
mod identity;

pub use config::IrohConfig;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 4: Run tests**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- p2p_iroh::identity
```
Expected: 3 tests pass.

If `SecretKey::from_bytes` has a different signature in 1.0.0-rc.0 (e.g., returns `Result`), follow the docs and adjust. The README example uses `SecretKey::generate(&mut rand::rngs::OsRng)` which is the canonical pattern.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/p2p_iroh/identity.rs elohim/elohim-storage/src/p2p_iroh/mod.rs
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
/// it down via `endpoint.close().await` on graceful exit.
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

    #[tokio::test]
    async fn builds_endpoint_with_relays_disabled() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            alpns: vec![iroh_blobs::ALPN.to_vec()],
        };

        let ep = build_endpoint(&cfg).await.expect("endpoint builds");

        // Endpoint exposes its node id; just touching it proves we constructed
        // a real endpoint, not a placeholder.
        let _id = ep.node_id();

        ep.close().await;
    }
}
```

- [ ] **Step 2: Mount endpoint in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod config;
mod endpoint;
mod identity;

pub use config::IrohConfig;
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 3: Run test**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- p2p_iroh::endpoint
```
Expected: 1 test passes.

**Likely API drift to handle:**
- `Endpoint::builder()` may require an arg (e.g., `presets::N0`) in 1.0.0-rc.0. The research report shows both `Endpoint::builder()` and `Endpoint::builder(presets::N0)` patterns in the wild. Check `https://docs.rs/iroh/1.0.0-rc.0/iroh/struct.Endpoint.html` for the exact signature. Also check whether `node_id()` is renamed to `endpoint_id()` (the 1.0 vocabulary).
- If the canonical builder takes `presets::N0`, drop the `relay_mode` line — the preset wires it.

Update both the production code and the test once you confirm the signature.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): build_endpoint constructs iroh Endpoint with ALPNs"
```

---

## Task 6: IrohBlobStore wrapper

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/blob_store.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/blob_store.rs`:

```rust
//! Wrapper around `iroh_blobs::store::fs::FsStore` exposing the operations
//! the elohim-storage daemon needs: add bytes, get bytes, has-check.
//!
//! In iroh mode, this is **the** blob store — the SHA256-keyed
//! `crate::blob_store::BlobStore` is bypassed.

use std::path::Path;

use anyhow::Result;
use iroh_blobs::api::blobs::Blobs;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::{BlobsProtocol, Hash};

/// BLAKE3-addressed blob store backed by `iroh_blobs::store::fs::FsStore`.
#[derive(Clone)]
pub struct IrohBlobStore {
    store: FsStore,
}

impl IrohBlobStore {
    /// Open or create the on-disk store at `dir`.
    pub async fn open(dir: &Path) -> Result<Self> {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        let store = FsStore::load(dir).await?;
        Ok(Self { store })
    }

    /// Add bytes; returns the BLAKE3 hash.
    pub async fn add_bytes(&self, bytes: impl Into<bytes::Bytes>) -> Result<Hash> {
        let tag = self.store.blobs().add_bytes(bytes.into()).await?;
        Ok(tag.hash)
    }

    /// Read a blob into memory by its BLAKE3 hash.
    pub async fn get_bytes(&self, hash: Hash) -> Result<Vec<u8>> {
        let bytes = self.store.blobs().get_bytes(hash).await?;
        Ok(bytes.to_vec())
    }

    /// Returns true if the store has a complete copy of the blob.
    pub async fn has(&self, hash: Hash) -> Result<bool> {
        // iroh-blobs exposes status via the blobs api; if `status` returns
        // Complete -> true, anything else -> false. Verify the exact method
        // name in 0.100 docs.
        let status = self.store.blobs().status(hash).await?;
        Ok(matches!(status, iroh_blobs::api::blobs::BlobStatus::Complete { .. }))
    }

    /// Borrow the underlying `Blobs` handle (for direct iroh-blobs calls).
    pub fn blobs(&self) -> &Blobs {
        self.store.blobs()
    }

    /// Build the BlobsProtocol handler for registration on a Router.
    pub fn protocol_handler(&self) -> BlobsProtocol {
        BlobsProtocol::new(&self.store, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn add_then_get_round_trips() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::open(dir.path()).await.unwrap();
        let hash = store.add_bytes(b"hello iroh".to_vec()).await.unwrap();
        let back = store.get_bytes(hash).await.unwrap();
        assert_eq!(back, b"hello iroh");
    }

    #[tokio::test]
    async fn has_reports_complete_after_add() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::open(dir.path()).await.unwrap();
        let hash = store.add_bytes(b"present".to_vec()).await.unwrap();
        assert!(store.has(hash).await.unwrap());
    }

    #[tokio::test]
    async fn has_reports_absent_for_unknown_hash() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::open(dir.path()).await.unwrap();
        // BLAKE3 of nothing in particular — guaranteed absent.
        let absent = Hash::from_bytes([0u8; 32]);
        assert!(!store.has(absent).await.unwrap());
    }
}
```

- [ ] **Step 2: Add `bytes` to elohim-storage if not direct**

`bytes` is already a dep at line 33 of Cargo.toml. No change.

- [ ] **Step 3: Mount blob_store in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod blob_store;
mod config;
mod endpoint;
mod identity;

pub use blob_store::IrohBlobStore;
pub use config::IrohConfig;
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
```

- [ ] **Step 4: Run tests**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- p2p_iroh::blob_store
```
Expected: 3 tests pass.

**Likely API drift:** the iroh-blobs 0.100 API has been moving rapidly. Verify the following at `https://docs.rs/iroh-blobs/0.100.0/iroh_blobs/`:

| Used in code | What to verify |
|--------------|----------------|
| `FsStore::load(dir).await` | Method name + return type. Could be `FsStore::open` or `FsStore::new`. |
| `store.blobs()` | The accessor for the high-level `Blobs` handle. |
| `blobs.add_bytes(bytes).await -> Tag { hash, .. }` | Verify the return type's field name is `hash`. |
| `blobs.get_bytes(hash).await -> Bytes` | Verify it returns `bytes::Bytes` (not `Vec<u8>`). |
| `blobs.status(hash).await -> BlobStatus` | Verify the variant `BlobStatus::Complete { .. }`. |
| `BlobsProtocol::new(&store, None)` | Verify the second arg is `Option<Downloader>` or similar; `None` is the simplest case. |

If the API has moved, follow the current docs and update the wrapper. Keep the public surface (`open`, `add_bytes`, `get_bytes`, `has`, `protocol_handler`) stable so downstream tasks don't churn.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): IrohBlobStore wrapping iroh-blobs FsStore"
```

---

## Task 7: Router with BlobsProtocol registration

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/router.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/src/p2p_iroh/router.rs`:

```rust
//! Assemble the iroh `Router` with protocol handlers. Phase 1 only registers
//! `BlobsProtocol`; later phases extend this with custom-ALPN handlers as
//! protocols graduate from libp2p.

use anyhow::Result;
use iroh::Endpoint;
use iroh::protocol::Router;

use super::blob_store::IrohBlobStore;

/// Build a `Router` that accepts the iroh-blobs ALPN, dispatching to the
/// supplied `IrohBlobStore`. The `Router` owns its accept loop; drop it to
/// stop accepting new connections.
pub fn build_router(endpoint: Endpoint, store: &IrohBlobStore) -> Router {
    Router::builder(endpoint)
        .accept(iroh_blobs::ALPN, store.protocol_handler())
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p_iroh::{build_endpoint, IrohConfig};
    use tempfile::tempdir;

    #[tokio::test]
    async fn router_spawns_with_blobs_protocol() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            alpns: vec![iroh_blobs::ALPN.to_vec()],
        };
        let endpoint = build_endpoint(&cfg).await.unwrap();
        let store = IrohBlobStore::open(&cfg.blobs_dir).await.unwrap();

        let router = build_router(endpoint, &store);

        // If we got here the router spawned successfully.
        // Shutdown gracefully.
        router.shutdown().await.expect("clean shutdown");
    }
}
```

- [ ] **Step 2: Mount router in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod blob_store;
mod config;
mod endpoint;
mod identity;
mod router;

pub use blob_store::IrohBlobStore;
pub use config::IrohConfig;
pub use endpoint::build_endpoint;
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use router::build_router;
```

- [ ] **Step 3: Run test**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- p2p_iroh::router
```
Expected: 1 test passes.

**Likely API drift:** verify `Router::builder(endpoint).accept(alpn, handler).spawn()` against `https://docs.rs/iroh/1.0.0-rc.0/iroh/protocol/struct.Router.html`. Verify `router.shutdown().await` returns `Result<(), _>`. If `spawn()` returns a `Result`, propagate it.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): Router with iroh-blobs ALPN registered"
```

---

## Task 8: Single-source blob fetch helper

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/fetch.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Write the failing test**

The test for this task is the two-node round-trip in Task 9; this task only adds the helper function. We'll smoke-test it via the round-trip integration test.

Create `elohim/elohim-storage/src/p2p_iroh/fetch.rs`:

```rust
//! Pull a blob from a remote iroh node by ticket. Phase 1 ships single-source
//! fetch via the lower-level `get` API; multi-source via the downloader is
//! Phase 2 once we wire it to the placement-gap planner.

use anyhow::{Context, Result};
use iroh::Endpoint;
use iroh_blobs::ticket::BlobTicket;

use super::blob_store::IrohBlobStore;

/// Fetch a blob by ticket into the local store. Returns the bytes (cloned out
/// of the store for the caller's convenience). Idempotent — if the blob is
/// already present, returns the stored bytes without a network round trip.
pub async fn fetch_blob_by_ticket(
    endpoint: &Endpoint,
    store: &IrohBlobStore,
    ticket: &BlobTicket,
) -> Result<Vec<u8>> {
    let hash = ticket.hash();

    if store.has(hash).await? {
        return store.get_bytes(hash).await;
    }

    // Connect to the provider over QUIC on the iroh-blobs ALPN.
    let conn = endpoint
        .connect(ticket.node_addr().clone(), iroh_blobs::ALPN)
        .await
        .context("iroh: connect to ticket node")?;

    // Use the get-protocol primitive to fetch the blob into the local store.
    // The exact entry point in iroh-blobs 0.100 lives at
    // `iroh_blobs::api::remote::Remote` or the higher-level downloader.
    // Verify against docs.rs at task time.
    let remote = store.blobs().remote();
    remote
        .fetch(conn, hash, ticket.format())
        .await
        .context("iroh: fetch blob bytes")?;

    store.get_bytes(hash).await
}
```

- [ ] **Step 2: Mount fetch in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod blob_store;
mod config;
mod endpoint;
mod fetch;
mod identity;
mod router;

pub use blob_store::IrohBlobStore;
pub use config::IrohConfig;
pub use endpoint::build_endpoint;
pub use fetch::fetch_blob_by_ticket;
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use router::build_router;
```

- [ ] **Step 3: Verify it compiles**

```bash
cd /projects/elohim/elohim/elohim-storage
just build-iroh
```
Expected: builds successfully.

**Likely API drift:** `iroh-blobs` 0.100 has multiple fetch entry points. Check in this order:
1. `Blobs::download(ticket)` — highest-level if it exists in 0.100
2. `Blobs::remote().fetch(conn, hash, format)` — what's in the code above
3. `iroh_blobs::get::request(conn, &GetRequest::single(hash, format))` — lower-level

Use whichever the current docs document as the canonical single-blob fetch. Update the `fetch_blob_by_ticket` body and re-run `just build-iroh`.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/
git commit -m "feat(storage): fetch_blob_by_ticket pulls a blob into IrohBlobStore"
```

---

## Task 9: End-to-end two-node blob round-trip

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/node.rs`
- Create: `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`

- [ ] **Step 1: Define the IrohNode aggregate**

Create `elohim/elohim-storage/src/p2p_iroh/node.rs`:

```rust
//! Aggregate over `Endpoint`, `Router`, and `IrohBlobStore`. Mirrors the role
//! of `crate::p2p::P2PNode` for the iroh stack — the long-lived runtime object
//! the daemon owns.

use anyhow::Result;
use iroh::{Endpoint, EndpointAddr, NodeId};
use iroh::protocol::Router;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobFormat, Hash};

use super::{
    blob_store::IrohBlobStore,
    build_endpoint, build_router,
    config::IrohConfig,
    fetch::fetch_blob_by_ticket,
};

/// Long-lived iroh P2P node. Owns the endpoint, router, and blob store.
pub struct IrohNode {
    endpoint: Endpoint,
    router: Router,
    store: IrohBlobStore,
}

impl IrohNode {
    /// Build and start a node from configuration.
    pub async fn start(config: &IrohConfig) -> Result<Self> {
        let endpoint = build_endpoint(config).await?;
        let store = IrohBlobStore::open(&config.blobs_dir).await?;
        let router = build_router(endpoint.clone(), &store);
        Ok(Self { endpoint, router, store })
    }

    /// Public node identifier (ed25519 pubkey).
    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    /// Reachable address (node id + relay url + direct addrs) for ticket
    /// construction.
    pub async fn node_addr(&self) -> Result<EndpointAddr> {
        Ok(self.endpoint.node_addr().await?)
    }

    /// Add bytes locally; returns the BLAKE3 hash.
    pub async fn add_bytes(&self, bytes: impl Into<bytes::Bytes>) -> Result<Hash> {
        self.store.add_bytes(bytes).await
    }

    /// Mint a `BlobTicket` for sharing. Caller is responsible for delivering
    /// the ticket to the recipient out of band.
    pub async fn mint_ticket(&self, hash: Hash) -> Result<BlobTicket> {
        let addr = self.node_addr().await?;
        let ticket = BlobTicket::new(addr, hash, BlobFormat::Raw);
        Ok(ticket)
    }

    /// Pull a blob from a remote node by ticket. Idempotent.
    pub async fn fetch_by_ticket(&self, ticket: &BlobTicket) -> Result<Vec<u8>> {
        fetch_blob_by_ticket(&self.endpoint, &self.store, ticket).await
    }

    /// Read a locally-known blob.
    pub async fn get_bytes(&self, hash: Hash) -> Result<Vec<u8>> {
        self.store.get_bytes(hash).await
    }

    /// Graceful shutdown.
    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        self.endpoint.close().await;
        Ok(())
    }
}
```

- [ ] **Step 2: Mount IrohNode in mod.rs**

Edit `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
mod blob_store;
mod config;
mod endpoint;
mod fetch;
mod identity;
mod node;
mod router;

pub use blob_store::IrohBlobStore;
pub use config::IrohConfig;
pub use endpoint::build_endpoint;
pub use fetch::fetch_blob_by_ticket;
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use node::IrohNode;
pub use router::build_router;
```

- [ ] **Step 3: Write the round-trip integration test**

Create `elohim/elohim-storage/tests/iroh_blob_roundtrip.rs`:

```rust
//! Two-node iroh blob round-trip — the milestone for Phase 1.
//!
//! Provider node adds a blob → mints a ticket → fetcher node pulls via ticket
//! → asserts the bytes match. Both nodes run in-process with relays disabled
//! (LAN/loopback only) for deterministic CI.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
use tempfile::tempdir;

#[tokio::test]
async fn two_nodes_can_round_trip_a_blob_by_ticket() {
    // Provider
    let provider_dir = tempdir().unwrap();
    let provider_cfg = IrohConfig {
        blobs_dir: provider_dir.path().join("blobs_iroh"),
        secret_key_path: provider_dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![iroh_blobs::ALPN.to_vec()],
    };
    let provider = IrohNode::start(&provider_cfg).await.expect("provider starts");

    // Fetcher
    let fetcher_dir = tempdir().unwrap();
    let fetcher_cfg = IrohConfig {
        blobs_dir: fetcher_dir.path().join("blobs_iroh"),
        secret_key_path: fetcher_dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![iroh_blobs::ALPN.to_vec()],
    };
    let fetcher = IrohNode::start(&fetcher_cfg).await.expect("fetcher starts");

    // Provider adds a blob and mints a ticket
    let payload = b"two-node iroh round trip — works without relays".to_vec();
    let hash = provider.add_bytes(payload.clone()).await.expect("add bytes");
    let ticket = provider.mint_ticket(hash).await.expect("mint ticket");

    // Fetcher pulls via ticket
    let fetched = fetcher.fetch_by_ticket(&ticket).await.expect("fetch by ticket");
    assert_eq!(fetched, payload, "fetched bytes match provider's payload");

    // Idempotent: second fetch is a no-op (no network needed)
    let fetched_again = fetcher.fetch_by_ticket(&ticket).await.expect("idempotent fetch");
    assert_eq!(fetched_again, payload);

    // Cleanup
    fetcher.shutdown().await.expect("fetcher shutdown");
    provider.shutdown().await.expect("provider shutdown");
}
```

- [ ] **Step 4: Run the integration test**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh --test iroh_blob_roundtrip
```
Expected: 1 test passes. May take 10-30s for the QUIC handshake + transfer.

**If the test hangs:**
- iroh expects a relay or routable direct address. With `use_n0_relays: false`, both nodes must be able to reach each other via direct UDP. On Linux, loopback (`127.0.0.1`) should work. If the OS firewall blocks UDP between two processes on loopback, set `use_n0_relays: true` and accept dependence on the n0 hosted relay for the test (acceptable for CI, not for production).
- If the connect call times out, log `endpoint.bound_sockets()` (or whatever the 1.0 equivalent is) to confirm the endpoint is actually listening.
- If you see "no addresses to connect to," the ticket may have been minted before the endpoint discovered its addresses. Add a brief `tokio::time::sleep(Duration::from_millis(200)).await` before `mint_ticket` (and refactor to a more deterministic ready-signal in a follow-up).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/node.rs \
        elohim/elohim-storage/src/p2p_iroh/mod.rs \
        elohim/elohim-storage/tests/iroh_blob_roundtrip.rs
git commit -m "feat(storage): two-node iroh blob round-trip via BlobTicket"
```

---

## Task 10: TransportBackend config + main.rs branch

**Files:**
- Modify: `elohim/elohim-storage/src/config.rs`
- Modify: `elohim/elohim-storage/src/main.rs`
- Create: `elohim/elohim-storage/tests/iroh_node_lifecycle.rs`

- [ ] **Step 1: Write the failing test for the config enum**

Add to the bottom of `elohim/elohim-storage/src/config.rs`, inside the existing `#[cfg(test)] mod tests` block (or add one if it doesn't exist):

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

#[test]
fn transport_backend_parses_from_lowercase() {
    let toml_str = r#"
        storage_dir = "/tmp/x"
        holochain_admin_url = "ws://x"
        app_id = "x"
        role_name = "x"
        zome_name = "x"
        max_storage_bytes = 0
        enable_eviction = false
        min_replicas_for_eviction = 0
        sync_interval_secs = 0
        p2p_port = 0
        http_port = 0
        p2p_bootstrap_nodes = []
        enable_mdns = false
        peer_policy_path = "/tmp/p"
        kick_fetch_per_peer_per_minute = 0
        custody_sweep_seconds = 0
        placement_grace_seconds = 0
        placement_gap_cooldown_seconds = 0
        inventory_freshness_seconds = 0
        fetch_blob_timeout_seconds = 0
        fetch_blob_parallelism = 0
        transport_backend = "iroh"
    "#;
    // We don't need every default field; minimal test that lowercase parses.
    // If your Config requires more fields, copy them from a default-serialized
    // sample.
    let cfg: Config = toml::from_str(toml_str).expect("parses lowercase");
    assert_eq!(cfg.transport_backend, TransportBackend::Iroh);
}
```

(Adapt the test fields to match the current `Config` definition — there are many required fields per `src/config.rs`. Use `toml::to_string(&Config::default())` to generate a baseline and tweak the one field.)

- [ ] **Step 2: Add the enum and field**

Edit `elohim/elohim-storage/src/config.rs`. Near the top of the file (after imports), add:

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

In the `Config` struct definition, add (in alphabetical order with the other `transport_*` / `t*` fields, or append at the end):

```rust
    /// Which P2P backend to start. Default `Libp2p`. Override via TOML
    /// (`transport_backend = "iroh"`) or env var `ELOHIM_TRANSPORT_BACKEND`.
    #[serde(default)]
    pub transport_backend: TransportBackend,
```

If the `Config` derives `Default`, the field is covered by `#[derive(Default)]` since `TransportBackend` derives `Default`. Otherwise, ensure the manual `Default` impl sets `transport_backend: TransportBackend::Libp2p` (which is the same as `TransportBackend::default()`).

- [ ] **Step 3: Run config tests**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh -- config::tests::transport_backend
```
Expected: 3 tests pass.

- [ ] **Step 4: Branch in main.rs**

Edit `elohim/elohim-storage/src/main.rs`. Locate the existing P2P-startup block (look for `P2PConfig`, `P2PNode::new`, or similar — should be near the other `#[cfg(feature = "p2p")]` blocks around lines 62-67 and below in the runtime startup function).

Wrap the existing libp2p startup in:

```rust
match config.transport_backend {
    TransportBackend::Libp2p => {
        #[cfg(feature = "p2p")]
        {
            // ... existing libp2p P2PNode startup code, indented one level ...
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
            // Phase 1: just hold the node alive until shutdown signal. No
            // command channel wired into the rest of the daemon yet — that's
            // Phase 2 (HTTP route surface graduation).
            // For now, the daemon serves blobs and accepts inbound fetches
            // but the existing libp2p-driven flows (kick_fetch, placement,
            // inventory_broadcast) are NOT active.
        }
        #[cfg(not(feature = "p2p-iroh"))]
        {
            anyhow::bail!("config.transport_backend = iroh but the `p2p-iroh` feature is not compiled in");
        }
    }
}
```

Add the `TransportBackend` import at the top of `main.rs`:
```rust
use elohim_storage::config::TransportBackend;
```

(Adjust the actual edit to fit the existing main.rs layout. The exact location depends on where P2P startup currently lives — search for `P2PNode::new` or `P2PConfig` to find it.)

- [ ] **Step 5: Single-node smoke test**

Create `elohim/elohim-storage/tests/iroh_node_lifecycle.rs`:

```rust
//! Single-node lifecycle: start, get node id, shutdown cleanly.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
use tempfile::tempdir;

#[tokio::test]
async fn node_starts_and_stops_clean() {
    let dir = tempdir().unwrap();
    let cfg = IrohConfig {
        blobs_dir: dir.path().join("blobs_iroh"),
        secret_key_path: dir.path().join("iroh.key"),
        use_n0_relays: false,
        alpns: vec![iroh_blobs::ALPN.to_vec()],
    };
    let node = IrohNode::start(&cfg).await.expect("starts");
    let _id = node.node_id();
    node.shutdown().await.expect("clean shutdown");
}
```

- [ ] **Step 6: Run the smoke test**

```bash
cd /projects/elohim/elohim/elohim-storage
just test-iroh --test iroh_node_lifecycle
```
Expected: 1 test passes in <2s.

- [ ] **Step 7: Verify the daemon starts in iroh mode**

Build and run:
```bash
cd /projects/elohim/elohim/elohim-storage
just build-iroh
ELOHIM_TRANSPORT_BACKEND=iroh \
RUST_LOG=elohim_storage=info \
./target/debug/elohim-storage --storage-dir /tmp/iroh-smoke
```

(Adjust the run command to match the actual CLI surface — check `src/main.rs` for the argument names.)

Expected log line: `iroh node started (Phase 1 — blob plane only; libp2p protocols disabled)`. Hit Ctrl-C; expect a clean shutdown.

If the daemon depends on libp2p-driven side effects (e.g., HTTP route registration that depends on `P2PHandle`), you may see runtime errors when those subsystems fail to initialize. **Document them but do not fix them in this plan** — those are the "things still to graduate" surface area for Phase 2. Add them as a TODO comment in `main.rs` near the iroh branch.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/config.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/tests/iroh_node_lifecycle.rs
git commit -m "feat(storage): TransportBackend config + iroh boot path"
```

---

## Task 11: Graduation criteria + sprint-end documentation

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/README.md`

- [ ] **Step 1: Write the README**

Create `elohim/elohim-storage/src/p2p_iroh/README.md`:

```markdown
# p2p_iroh — Phase 1: Blob plane

Parallel iroh-based P2P module sitting alongside `crate::p2p`. Selectable at
runtime via `Config::transport_backend = TransportBackend::Iroh`.

## Status

**Preview.** iroh-blobs 0.100 is pre-1.0; n0 says "wait 1-2 more releases
before production." Keep the libp2p stack as default until graduation.

## What works

- Two-node blob round-trip via `BlobTicket` (LAN/relay)
- BLAKE3 content addressing throughout the iroh path
- Persisted iroh `SecretKey` at `<storage_dir>/iroh.key`
- Graceful startup and shutdown

## What does not work yet (Phase 2+)

Each item below is **deferred work that will run its own P2P design gate**
when the corresponding Phase 2 plan is written. Phase 1 introduces no schema
changes, no route changes, and no seeder changes. Source-of-truth for the
two artifacts Phase 1 *does* introduce — the iroh-blobs FsStore (operational
projection of content-addressed bytes) and the `iroh.key` file (agent-scoped
private credential) — is declared in the "P2P Design Gate" section at the
top of this plan.

- HTTP `/api/v1/blob/{hash}` route does NOT serve from `IrohBlobStore` — the
  route still reads from the legacy SHA256 `BlobStore`. *(Deferred to a
  separate Phase 2 plan with its own design gate covering the dual-format
  hash boundary.)*
- Genesis seeder writes to the legacy `BlobStore`, not `IrohBlobStore`.
  *(Deferred — separate Phase 2 plan; design gate will classify the seeder's
  byte-write target.)*
- The existing `peer_blob_inventory` projection records SHA256 hashes; no
  BLAKE3-mode integration exists. *(Deferred — separate Phase 2 plan that
  graduates the inventory projection will run its own design gate.)*
- Other libp2p protocols (sync, shard, epr, trust, identity, gossip, kad,
  mdns, dcutr, autonat) have no iroh equivalents — running in iroh mode
  disables them. *(Each protocol graduation is its own plan with its own
  design gate.)*

## Graduation gates (must clear all before flipping default)

1. **iroh-blobs ≥ 0.101 with n0's "production-ready" announcement.**
2. HTTP `/api/v1/blob/{hash}` reads from `IrohBlobStore` when iroh-mode is
   active; route accepts BLAKE3 hashes (or both, with prefix discriminator).
3. Genesis seeder graduated to write through `IrohBlobStore`.
4. Inventory tables migrated to BLAKE3 keys (or — if pre-launch persists
   beyond Phase 1 — formal wipe-and-reseed runbook documented).
5. Sync, shard, EPR, and identity protocols graduate to iroh ALPNs (or, at
   minimum, the daemon refuses to start in iroh mode if any required
   protocol is missing, with a clear error).
6. Round-trip + lifecycle tests run in CI on every PR.
7. At least one alpha-cluster household runs in iroh mode for a full week
   without regression.

## Cutover playbook (for the day we flip the default)

1. Confirm all gates above are green.
2. Branch: `git checkout -b feat/iroh-default-backend`.
3. Edit `Cargo.toml`: `default = ["p2p", "p2p-iroh"]`.
4. Edit `src/config.rs`: `TransportBackend::default() = Iroh`.
5. Run full test suite under both `--features p2p` and
   `--features p2p,p2p-iroh`.
6. Run alpha cluster e2e: wipe `<storage_dir>` on every node, run seeder,
   confirm content distributes via iroh.
7. Update `CLAUDE.md` and `genesis/graphos/vocabulary.md` if any vocabulary
   changes (`blob` may need a hash-format clarification).
8. Open a PR; tag the alpha-cluster operators.

## Code map

| File | Purpose |
|------|---------|
| `mod.rs` | Module root + re-exports |
| `config.rs` | `IrohConfig` |
| `identity.rs` | Persisted `SecretKey` |
| `endpoint.rs` | `Endpoint` builder |
| `blob_store.rs` | `IrohBlobStore` over iroh-blobs `FsStore` |
| `router.rs` | `Router` with iroh-blobs ALPN |
| `fetch.rs` | Single-source fetch helper |
| `node.rs` | `IrohNode` aggregate |

Tests: `tests/iroh_blob_roundtrip.rs`, `tests/iroh_node_lifecycle.rs`.
```

- [ ] **Step 2: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/README.md
git commit -m "docs(storage): p2p_iroh README with status, graduation gates, cutover playbook"
```

- [ ] **Step 3: Run the full feature build + tests one last time**

```bash
cd /projects/elohim/elohim/elohim-storage
just build-iroh
just test-iroh
```
Expected: builds clean, all tests pass (libp2p tests + new iroh tests).

- [ ] **Step 4: Run pre-push gate**

```bash
cd /projects/elohim/elohim/elohim-storage
just gate
```
Expected: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` all green.

If clippy complains about `IrohBlobStore::has` matching `Complete { .. }` (looks like dead-code sub-pattern), use `_` or named fields — adjust to silence the lint.

---

## Self-Review Notes (filled in after writing the plan)

**Spec coverage:** All goals from the conversation are addressed:
- Parallel module — Tasks 1-2
- iroh-blobs native (BLAKE3) — Tasks 6-9
- Feature flag, additive — Task 1
- Two-node round-trip — Task 9
- Runtime config toggle — Task 10
- Cutover readiness (graduation gates + playbook) — Task 11

**Out of scope (intentional):** Each deferred item runs its own design gate
when graduated. Source-of-truth for the two artifacts Phase 1 *does* add (an
operational FsStore projection and an agent-scoped key file) is declared in
the gate section at the top.

- Other libp2p protocols' migration (sync/shard/epr/trust/identity)
- HTTP route surface graduation
- Genesis seeder rewrite
- Inventory projection schema migration to BLAKE3 — the existing operational
  projection stays SHA256-keyed in Phase 1
- Self-hosted iroh-relay
- Cross-stack identity unification (separate iroh `EndpointId` and libp2p `PeerId` for now)

**Known API uncertainty:** iroh 1.0.0-rc.0 was published 2026-05-07 — same day as this plan. Several method signatures may have shifted. Each task that touches iroh API surface flags the specific calls to verify against current docs.rs and the patterns to substitute if the API has moved. The plan is structured so API drift is contained per-task, not viral.
