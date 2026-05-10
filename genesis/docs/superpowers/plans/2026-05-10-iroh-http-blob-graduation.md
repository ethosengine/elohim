# Iroh HTTP Blob Graduation (Cutover Gate #2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graduate `GET /blob/{hash}` so iroh-capable callers fetch BLAKE3-addressed blobs from `IrohBlobStore` first, and libp2p-only callers (or BLAKE3-unknown blobs) fall through to the legacy `BlobStore` SHA256 path — preserving the current wire contract exactly.

**Architecture:** A new `BlobBackendRouter` resolves, per request, whether a blob has a BLAKE3 alias (via `peer_blob_inventory.blake3_hash`) AND whether the caller's transport profile (looked up via Plan 1's `lookup_by_agent_cid` + `select_transport(.., Plane::Blob)`) is iroh-capable. When both are true, the handler tries `IrohBlobStore::get_bytes` first; on `Ok`, it serves identical bytes through the existing `application/octet-stream` response shape. On Iroh miss, libp2p-only caller, or unknown BLAKE3, the existing `handle_get_blob` body executes unchanged. Per-request choice is recorded as a `tracing::debug!` event and an `AtomicU64` counter on `HttpServer` so parity-soak diagnostics see backend split. Doorway requires no code changes — `storage_proxy::forward_blob_to_storage` already forwards `/blob/{hash}` byte-for-byte.

**Tech Stack:** `iroh_blobs::Hash` (BLAKE3), existing `BlobStore` (SHA256), Diesel SQLite via `peer_blob_inventory`, `tracing::debug`, `std::sync::atomic::AtomicU64`. No new crate dependencies.

---

## P2P Design Gate (source-of-truth audit)

This plan introduces **no new DHT entry types and no new HTTP routes**. All HTTP route mentions in this plan refer to the **existing** `GET /blob/{hash}` byte-route (declared in `elohim-storage` `http.rs:639` and exposed via `build_manifest()`'s `with_blobs_at("/blob")`); the `/api/v1/blob/{hash}/distribution/details` Phase-5 view route already audited in `api/blob.rs`; and the `/status` JSON surface already declared. The graduation changes WHICH backend serves bytes for an existing route; it does not propose new endpoints.

| Concern | Category | Source of truth |
|---|---|---|
| Blob bytes (BLAKE3-keyed) | C — operational | `IrohBlobStore` filesystem (iroh-blobs `FsStore`); content-addressed, regenerable |
| Blob bytes (SHA256-keyed) | C — operational | Legacy `BlobStore` filesystem; content-addressed, regenerable |
| `peer_blob_inventory.blake3_hash` reverse-lookup | C — derived | Existing column from migration `2026-05-08-033248_peer_blob_inventory_blake3_hash`; populated by libp2p inventory-gossip and iroh inventory broadcasts; not authoritative on the DHT |
| Caller transport-profile manifest | A2 — derived via DHT-attested gossip | Plan 1's `peer_transport_manifest`, which projects DHT-notarized identity-binding gossip (per spec line 499) into a local read-cache. This plan only READS the projection. |
| Backend choice counters (`blob_iroh_served_count`) | C — operational | In-memory `AtomicU64` on `HttpServer`; transient; lost on restart by design |

No new schema is created in this plan. Task 1 adds two reverse-lookup query functions over an **already-migrated** column; no `up.sql` / `down.sql` files are produced. No new HTTP route is registered with `build_manifest()`; the dispatcher merely chooses which backend serves the existing `/blob/{hash}` route.

The only data flows added are local operational projections (router selection, counter increments, debug log lines) — none of these cross the DHT boundary, none of them produce new notarized state.

---

## Pre-flight invariants (do not modify)

- HTTP wire contract: `GET /blob/{hash}` returns `application/octet-stream`, `Content-Length`, `Cache-Control: public, max-age=31536000, immutable`, and (for sharded reassembly path) an `ETag`. Status codes: 200, 400 (bad address), 403 (policy block), 404 (miss), 503 (peer-fetch persist failure). No new headers, no new status codes.
- Hash parameter format: `parse_content_address` (in `crate::blob_store::BlobStore`) accepts CID, `sha256-{64hex}`, or raw 64-hex. The graduation extends acceptance to `blake3-{52|64hex}` ONLY in the Iroh selection path; addresses that don't parse as BLAKE3 fall through to existing SHA256 parsing untouched.
- Plan 1 surface used (treated as fixed contract):
  - `crate::p2p_iroh::peer_map::Plane::Blob`
  - `crate::p2p_iroh::peer_map::TransportChoice` (variants used: `Iroh`, `Libp2p`)
  - `crate::p2p_iroh::peer_map::lookup_by_agent_cid(conn: &mut SqliteConnection, agent_cid: &str) -> Result<Option<PeerTransportManifest>, StorageError>`
  - `crate::p2p_iroh::peer_map::select_transport(self_manifest: &PeerTransportManifest, peer_manifest: &PeerTransportManifest, plane: Plane) -> Result<TransportChoice, StorageError>`
  - `HttpServer` gains a `self_transport_manifest: Option<Arc<PeerTransportManifest>>` populated by Plan 1 at startup; this plan only reads it.
- Current `handle_get_blob` lives at `/projects/elohim/elohim/elohim-storage/src/http.rs:1639`–`1912`. Do not delete or restructure it; the graduation wraps it.

---

## Task 1: Schema query for `blake3_hash` lookup by SHA256

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/db/peer_blob_inventory.rs` (add new function near `lookup_hosts` at line 159)
- Test (new): inline `#[cfg(test)] mod tests` block at end of same file

**Rationale:** The existing `peer_blob_inventory` rows already have a `blake3_hash: Option<String>` column (added by migration `2026-05-08-033248_peer_blob_inventory_blake3_hash`). The router needs two reverse lookups: `(sha256) -> Option<blake3>` and `(blake3) -> Option<sha256>`. These are read-only operational projections — no new DHT entry, no new migration.

- [ ] **Step 1.1: Failing test** — Add to `peer_blob_inventory.rs`'s test module:

```rust
#[test]
fn lookup_blake3_for_sha256_returns_some_when_present() {
    let mut conn = test_conn(); // existing helper in this module
    // Insert a row with both hashes populated
    diesel::insert_into(peer_blob_inventory::table)
        .values((
            peer_blob_inventory::peer_id.eq("peer-A"),
            peer_blob_inventory::blob_hash.eq("sha256-aaaa"),
            peer_blob_inventory::blake3_hash.eq(Some("blake3-bbbb")),
            peer_blob_inventory::source.eq("gossip-snapshot"),
            peer_blob_inventory::sequence.eq(0i64),
            peer_blob_inventory::last_seen_at.eq("2026-05-10T00:00:00Z"),
        ))
        .execute(&mut conn)
        .unwrap();

    let got = lookup_blake3_for_sha256(&mut conn, "sha256-aaaa").unwrap();
    assert_eq!(got, Some("blake3-bbbb".to_string()));
}

#[test]
fn lookup_blake3_for_sha256_returns_none_when_absent() {
    let mut conn = test_conn();
    let got = lookup_blake3_for_sha256(&mut conn, "sha256-absent").unwrap();
    assert_eq!(got, None);
}

#[test]
fn lookup_sha256_for_blake3_returns_some_when_present() {
    let mut conn = test_conn();
    diesel::insert_into(peer_blob_inventory::table)
        .values((
            peer_blob_inventory::peer_id.eq("peer-A"),
            peer_blob_inventory::blob_hash.eq("sha256-aaaa"),
            peer_blob_inventory::blake3_hash.eq(Some("blake3-bbbb")),
            peer_blob_inventory::source.eq("gossip-snapshot"),
            peer_blob_inventory::sequence.eq(0i64),
            peer_blob_inventory::last_seen_at.eq("2026-05-10T00:00:00Z"),
        ))
        .execute(&mut conn)
        .unwrap();

    let got = lookup_sha256_for_blake3(&mut conn, "blake3-bbbb").unwrap();
    assert_eq!(got, Some("sha256-aaaa".to_string()));
}
```

Run:

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib peer_blob_inventory::tests::lookup_blake3 2>&1 | tail -20
```

Expected: 3 failures (`cannot find function lookup_blake3_for_sha256`, `cannot find function lookup_sha256_for_blake3`).

- [ ] **Step 1.2: Implement the two lookups** — Add to `peer_blob_inventory.rs` after `lookup_hosts` (line 192):

```rust
/// Reverse-lookup the BLAKE3 alias for a SHA256-addressed blob. Returns
/// `None` when no row in `peer_blob_inventory` has a populated
/// `blake3_hash` for this `blob_hash`. Used by the HTTP `/blob/{hash}`
/// router to decide whether the iroh path is reachable for a given
/// SHA256 caller-supplied address.
///
/// Picks the first non-NULL `blake3_hash` observed across peers; the
/// schema's content-addressed identity guarantees all rows agree when
/// any agree.
pub fn lookup_blake3_for_sha256(
    conn: &mut SqliteConnection,
    sha256_hash: &str,
) -> Result<Option<String>, StorageError> {
    use peer_blob_inventory::dsl;
    dsl::peer_blob_inventory
        .filter(dsl::blob_hash.eq(sha256_hash))
        .filter(dsl::blake3_hash.is_not_null())
        .select(dsl::blake3_hash)
        .first::<Option<String>>(conn)
        .optional()
        .map(|opt_opt| opt_opt.flatten())
        .map_err(|e| StorageError::Database(format!("lookup_blake3_for_sha256: {e}")))
}

/// Reverse-lookup the SHA256 alias for a BLAKE3-addressed blob. Mirror
/// of [`lookup_blake3_for_sha256`]. Used by the libp2p-fallback path
/// when the caller supplied a `blake3-` prefixed address but the chosen
/// transport is libp2p (no Iroh manifest).
pub fn lookup_sha256_for_blake3(
    conn: &mut SqliteConnection,
    blake3_hash: &str,
) -> Result<Option<String>, StorageError> {
    use peer_blob_inventory::dsl;
    dsl::peer_blob_inventory
        .filter(dsl::blake3_hash.eq(blake3_hash))
        .select(dsl::blob_hash)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_sha256_for_blake3: {e}")))
}
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib peer_blob_inventory::tests::lookup_blake3 2>&1 | tail -20
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib peer_blob_inventory::tests::lookup_sha256 2>&1 | tail -20
```

Expected: 3 passing.

- [ ] **Step 1.3: Commit** —

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/db/peer_blob_inventory.rs
git commit -m "feat(storage): add blake3<->sha256 reverse lookups in peer_blob_inventory"
```

---

## Task 2: New module `http_blob_router` with backend-selection logic

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/src/http_blob_router.rs`
- Modify `/projects/elohim/elohim/elohim-storage/src/lib.rs` (add `pub mod http_blob_router;` near other top-level module declarations)

**Rationale:** Keep selection logic out of the 9693-line `http.rs`. The router is a pure function over `(caller_agent_cid, hash, peer_blob_inventory_view, self_manifest)` returning a `BlobBackendChoice` enum that the handler in `http.rs` switches on.

- [ ] **Step 2.1: Failing test** — Create `/projects/elohim/elohim/elohim-storage/src/http_blob_router.rs` with the test scaffold first:

```rust
//! Backend selection for `GET /blob/{hash}`. Decides between
//! `IrohBlobStore` (BLAKE3) and the legacy `BlobStore` (SHA256) per
//! request, honoring the caller's transport-profile manifest and the
//! blob's known address aliases.
//!
//! Returns a [`BlobBackendChoice`] the HTTP handler switches on. Pure
//! over its inputs — no I/O, no network calls — so it is unit-testable
//! without spinning up either store.

use crate::p2p_iroh::peer_map::{PeerTransportManifest, Plane, TransportChoice};

/// Resolved backend choice and the hash to use against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobBackendChoice {
    /// Try Iroh first (with the BLAKE3 hash); fall through to libp2p
    /// (with the SHA256 hash) on miss.
    IrohThenLibp2p {
        blake3_hash: String,
        sha256_hash: String,
    },
    /// Use libp2p only (with the SHA256 hash). Either the caller is
    /// libp2p-only, or no BLAKE3 alias is known for this blob.
    Libp2pOnly { sha256_hash: String },
}

/// Inputs to the backend chooser. Constructed at the HTTP handler entry
/// point and then handed to [`choose_backend`] for a pure decision.
pub struct ChooseInputs<'a> {
    /// Caller-supplied hash, normalized: either `sha256-{hex}` or
    /// `blake3-{hex}`. Other forms (raw hex, CID) are normalized to
    /// `sha256-` upstream by `BlobStore::parse_content_address`.
    pub normalized_hash: &'a str,
    /// `peer_blob_inventory.blake3_hash` for the SHA256 form, if any.
    /// `None` when caller-hash is blake3-prefixed (irrelevant) or no
    /// row knows the alias.
    pub blake3_alias_for_sha256: Option<String>,
    /// `peer_blob_inventory.blob_hash` (the SHA256) for the BLAKE3
    /// form, if any. `None` when caller-hash is sha256-prefixed
    /// (irrelevant) or no row knows the alias.
    pub sha256_alias_for_blake3: Option<String>,
    /// This node's transport-profile manifest (Plan 1). When `None`
    /// (manifest not yet wired at startup), the router degrades to
    /// libp2p-only.
    pub self_manifest: Option<&'a PeerTransportManifest>,
    /// Caller's transport-profile manifest (Plan 1's
    /// `lookup_by_agent_cid`). `None` when caller is unauthenticated
    /// (visitor) or has no manifest published yet — degrades to
    /// libp2p-only.
    pub caller_manifest: Option<&'a PeerTransportManifest>,
}

/// Pure selection function. Returns the chosen backend(s) and the hash
/// to use for each.
pub fn choose_backend(inputs: ChooseInputs<'_>) -> BlobBackendChoice {
    todo!("implemented in step 2.2")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iroh_capable_manifest() -> PeerTransportManifest {
        // Plan 1 will document a constructor; for the test we use the
        // public field shape it commits to: a manifest with the Blob
        // plane mapped to `TransportChoice::Iroh`. If Plan 1 ships a
        // helper like `with_iroh_blob()`, switch to it at execution
        // time.
        PeerTransportManifest::iroh_capable_for_test()
    }

    fn libp2p_only_manifest() -> PeerTransportManifest {
        PeerTransportManifest::libp2p_only_for_test()
    }

    #[test]
    fn caller_iroh_capable_and_blake3_known_picks_iroh_then_libp2p() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    #[test]
    fn caller_libp2p_only_picks_libp2p_only() {
        let self_m = iroh_capable_manifest();
        let caller_m = libp2p_only_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_blake3_alias_picks_libp2p_only_even_when_iroh_capable() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn caller_supplied_blake3_with_iroh_caller_picks_iroh() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: Some("sha256-aaaa".to_string()),
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    #[test]
    fn caller_supplied_blake3_with_libp2p_caller_falls_back_to_sha256() {
        let self_m = iroh_capable_manifest();
        let caller_m = libp2p_only_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: Some("sha256-aaaa".to_string()),
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_caller_manifest_visitor_picks_libp2p_only() {
        let self_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: None,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_self_manifest_picks_libp2p_only() {
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: None,
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }
}
```

Add `pub mod http_blob_router;` to `/projects/elohim/elohim/elohim-storage/src/lib.rs` at the same indentation as the existing `pub mod` declarations.

Run:

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib http_blob_router::tests 2>&1 | tail -30
```

Expected: compilation fails with `not yet implemented` panic from `todo!()` in `choose_backend`, OR Plan 1's `iroh_capable_for_test` / `libp2p_only_for_test` test helpers are not yet present. If the latter, the executor halts here and reports BLOCKED on Plan 1.

- [ ] **Step 2.2: Implement `choose_backend`** — Replace the `todo!()` body in `http_blob_router.rs`:

```rust
pub fn choose_backend(inputs: ChooseInputs<'_>) -> BlobBackendChoice {
    let is_blake3_input = inputs.normalized_hash.starts_with("blake3-");

    // Resolve the SHA256 form for the libp2p path. For SHA256 inputs,
    // it is the input itself; for BLAKE3 inputs, look up the alias.
    let sha256_hash: String = if is_blake3_input {
        match &inputs.sha256_alias_for_blake3 {
            Some(s) => s.clone(),
            None => {
                // BLAKE3-only blob with no SHA256 alias known. The
                // caller MUST be iroh-capable for us to serve it; if
                // not, we'd be returning a 404 anyway. Encode that as
                // libp2p-only with the BLAKE3 form so the legacy path
                // produces the existing 404 wire shape.
                inputs.normalized_hash.to_string()
            }
        }
    } else {
        inputs.normalized_hash.to_string()
    };

    // Resolve the BLAKE3 form for the iroh path.
    let blake3_hash_opt: Option<String> = if is_blake3_input {
        Some(inputs.normalized_hash.to_string())
    } else {
        inputs.blake3_alias_for_sha256.clone()
    };

    // Run the cross-stack peer map's transport selector. Any path that
    // doesn't yield TransportChoice::Iroh, or where either manifest is
    // absent, degrades to libp2p-only.
    let chose_iroh = match (inputs.self_manifest, inputs.caller_manifest) {
        (Some(self_m), Some(caller_m)) => {
            matches!(
                crate::p2p_iroh::peer_map::select_transport(
                    self_m,
                    caller_m,
                    Plane::Blob,
                ),
                Ok(TransportChoice::Iroh)
            )
        }
        _ => false,
    };

    match (chose_iroh, blake3_hash_opt) {
        (true, Some(blake3_hash)) => BlobBackendChoice::IrohThenLibp2p {
            blake3_hash,
            sha256_hash,
        },
        _ => BlobBackendChoice::Libp2pOnly { sha256_hash },
    }
}
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib http_blob_router::tests 2>&1 | tail -20
```

Expected: 7 passing.

- [ ] **Step 2.3: Commit** —

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http_blob_router.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): add http_blob_router with iroh-vs-libp2p backend selector"
```

---

## Task 3: Wire `IrohBlobStore` and Plan 1 manifests into `HttpServer`

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/http.rs` (struct definition lines 136–204; `new` constructor lines 260–292; new builder methods)
- Modify `/projects/elohim/elohim/elohim-storage/src/main.rs` (around line 1062 where `IrohNode` is currently dropped into `_iroh_node`, and around line 1162 where the HTTP server builder chain runs)

**Rationale:** Today `IrohNode` is constructed in `main.rs` at line 1049 but bound to `_iroh_node` and never threaded to the HTTP server. The graduation needs `HttpServer` to hold an `Option<Arc<IrohBlobStore>>` plus the two Plan 1 manifest hooks.

- [ ] **Step 3.1: Failing test** — Add a unit test to `http.rs` near other `HttpServer` builder tests (search for `fn new(blob_store:` and place after the existing tests for that block):

```rust
#[cfg(test)]
mod blob_backend_wiring_tests {
    use super::*;

    #[test]
    fn http_server_defaults_iroh_blob_store_to_none() {
        let blob_store = Arc::new(BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap());
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap());
        assert!(server.iroh_blob_store.is_none());
        assert!(server.self_transport_manifest.is_none());
    }

    #[tokio::test]
    async fn with_iroh_blob_store_sets_field() {
        let blob_store = Arc::new(BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap());
        let iroh_dir = tempfile::tempdir().unwrap();
        let iroh = Arc::new(
            crate::p2p_iroh::IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
                .await
                .unwrap(),
        );
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_iroh_blob_store(iroh.clone());
        assert!(server.iroh_blob_store.is_some());
    }
}
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib blob_backend_wiring_tests 2>&1 | tail -20
```

Expected: compile errors — `iroh_blob_store` and `self_transport_manifest` fields do not exist; `with_iroh_blob_store` method missing.

- [ ] **Step 3.2: Add fields to `HttpServer`** — In `http.rs` between lines 203 and 204 (after `fetch_blob_parallelism`):

```rust
    /// Iroh-side blob backend (BLAKE3-keyed). Wired at startup via
    /// [`HttpServer::with_iroh_blob_store`] when the iroh node is
    /// active. None means iroh is disabled or not yet wired — every
    /// `/blob/{hash}` request degrades to the legacy SHA256 path.
    iroh_blob_store: Option<Arc<crate::p2p_iroh::IrohBlobStore>>,
    /// This node's transport-profile manifest (Plan 1). Used by the
    /// blob handler's backend selector. Wired at startup via
    /// [`HttpServer::with_self_transport_manifest`].
    self_transport_manifest: Option<Arc<crate::p2p_iroh::peer_map::PeerTransportManifest>>,
    /// Counter — number of `GET /blob` requests served from iroh.
    /// Read by parity-soak diagnostics; never reset at runtime.
    blob_iroh_served_count: Arc<std::sync::atomic::AtomicU64>,
    /// Counter — number of `GET /blob` requests served from libp2p.
    blob_libp2p_served_count: Arc<std::sync::atomic::AtomicU64>,
```

In the `new` constructor (line 260), add the four initializers at the bottom of the struct literal (after `fetch_blob_parallelism: 3,`):

```rust
            iroh_blob_store: None,
            self_transport_manifest: None,
            blob_iroh_served_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            blob_libp2p_served_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
```

Add two builder methods after the existing builder block (e.g. after `with_fan_out_ctx`):

```rust
    /// Wire the iroh-side blob store. When set, `GET /blob/{hash}` may
    /// serve from BLAKE3-keyed iroh storage for iroh-capable callers
    /// before falling through to the legacy SHA256 path.
    pub fn with_iroh_blob_store(
        mut self,
        store: Arc<crate::p2p_iroh::IrohBlobStore>,
    ) -> Self {
        self.iroh_blob_store = Some(store);
        self
    }

    /// Wire this node's transport-profile manifest (Plan 1). Required
    /// for the blob backend selector to ever return Iroh; absent
    /// manifest forces libp2p-only.
    pub fn with_self_transport_manifest(
        mut self,
        manifest: Arc<crate::p2p_iroh::peer_map::PeerTransportManifest>,
    ) -> Self {
        self.self_transport_manifest = Some(manifest);
        self
    }
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib blob_backend_wiring_tests 2>&1 | tail -20
```

Expected: 2 passing.

- [ ] **Step 3.3: Thread `IrohBlobStore` from `main.rs`** — In `/projects/elohim/elohim/elohim-storage/src/main.rs` near line 1062, change:

```rust
                Some(node)
```

to clone the store reference for the HTTP server:

```rust
                let iroh_blob_store_for_http = Arc::new(node.store().clone());
                (Some(node), Some(iroh_blob_store_for_http))
```

Adjust the `let _iroh_node = ...` binding to destructure the tuple, and the `else` / `#[cfg(not(feature = "p2p-iroh"))]` arms to return `(None, None)`. Type the binding as `(Option<IrohNode>, Option<Arc<elohim_storage::p2p_iroh::IrohBlobStore>>)`.

Find the `HttpServer::new(...)` builder chain (search for `HttpServer::new(blob_store.clone()`) and add this conditional builder call before `.build()` (or before the chain terminates):

```rust
    let http_server = if let Some(iroh_blobs) = iroh_blob_store_for_http.clone() {
        http_server.with_iroh_blob_store(iroh_blobs)
    } else {
        http_server
    };
```

If Plan 1 has shipped a function like `crate::p2p_iroh::peer_map::load_self_manifest_from_env()`, also chain `.with_self_transport_manifest(Arc::new(loaded))` here. If Plan 1 has not shipped that helper yet, leave the manifest hook unwired (defaults to `None`, which forces libp2p-only — the safe default until Plan 1's manifest source-of-truth lands).

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -30
```

Expected: clean build.

- [ ] **Step 3.4: Commit** —

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): wire IrohBlobStore + self transport manifest into HttpServer"
```

---

## Task 4: Replace `handle_get_blob` body with router-driven dispatch

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/http.rs` (`handle_get_blob` at lines 1639–1912)

**Rationale:** Wrap the existing handler. Before the existing manifest/blob-store lookup runs, consult the router. On `IrohThenLibp2p` choice, attempt `IrohBlobStore::get_bytes`; on success, return the iroh-served bytes through the same response shape and increment the iroh counter. On any failure, fall through to the existing handler body (which already covers manifest reassembly, race-fetch, and 404). Always increment one of the two counters and emit a `tracing::debug!` event with `backend = "iroh"` or `backend = "libp2p"`.

- [ ] **Step 4.1: Failing integration test** — Create `/projects/elohim/elohim/elohim-storage/tests/blob_backend_dispatch.rs`:

```rust
//! Integration test for cutover gate #2: HTTP `GET /blob/{hash}`
//! dual-format dispatch. Two fixture blobs:
//!   * `blob_blake3_only` — present in IrohBlobStore but absent from
//!     legacy BlobStore. Iroh-capable caller must receive 200; the
//!     iroh counter must increment by 1.
//!   * `blob_sha256_only` — present in legacy BlobStore but absent
//!     from IrohBlobStore. Iroh-capable caller must still receive 200
//!     (fall-through); the libp2p counter must increment by 1.
//!
//! Wire shape (`Content-Type`, `Content-Length`, `Cache-Control`)
//! must be identical regardless of which backend served.

use bytes::Bytes;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::p2p_iroh::IrohBlobStore;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn blake3_only_blob_served_from_iroh_for_iroh_capable_caller() {
    // Arrange: IrohBlobStore with one blob; legacy BlobStore empty.
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );
    let payload = b"blake3 only blob".to_vec();
    let blake3 = iroh.add_bytes(payload.clone()).await.unwrap();

    let legacy_dir = tempdir().unwrap();
    let legacy = Arc::new(BlobStore::new(legacy_dir.path().to_path_buf()).unwrap());

    // Build HttpServer with both backends + an iroh-capable self manifest.
    let server = HttpServer::new(legacy, "127.0.0.1:0".parse().unwrap())
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(
            elohim_storage::p2p_iroh::peer_map::PeerTransportManifest::iroh_capable_for_test(),
        ));

    // Compose the request the way handle_get_blob expects (blake3-prefixed hash,
    // X-Agent-Cid header bound to an iroh-capable caller manifest fixture
    // that has been seeded into the test DbPool by Plan 1's test helpers).
    let req = elohim_storage::test_util::blob_get_request(
        &format!("blake3-{}", blake3),
        Some("did:elohim:test-iroh-caller"),
    );

    let resp = server.handle_blob_get_for_test(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap()),
        Some("application/octet-stream")
    );
    assert_eq!(server.blob_iroh_served_count_snapshot(), 1);
    assert_eq!(server.blob_libp2p_served_count_snapshot(), 0);
}

#[tokio::test]
async fn sha256_only_blob_served_from_libp2p_even_for_iroh_capable_caller() {
    // Arrange: legacy BlobStore with one blob; IrohBlobStore empty.
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );
    let legacy_dir = tempdir().unwrap();
    let legacy = Arc::new(BlobStore::new(legacy_dir.path().to_path_buf()).unwrap());
    let payload = b"sha256 only blob".to_vec();
    let stored = legacy.store(&payload).await.unwrap();

    let server = HttpServer::new(legacy.clone(), "127.0.0.1:0".parse().unwrap())
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(
            elohim_storage::p2p_iroh::peer_map::PeerTransportManifest::iroh_capable_for_test(),
        ));

    let req = elohim_storage::test_util::blob_get_request(
        &stored.hash, // sha256-prefixed
        Some("did:elohim:test-iroh-caller"),
    );

    let resp = server.handle_blob_get_for_test(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(server.blob_iroh_served_count_snapshot(), 0);
    assert_eq!(server.blob_libp2p_served_count_snapshot(), 1);
}
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test blob_backend_dispatch 2>&1 | tail -30
```

Expected: compile errors — `handle_blob_get_for_test`, `blob_iroh_served_count_snapshot`, `blob_libp2p_served_count_snapshot`, and `test_util::blob_get_request` do not yet exist.

- [ ] **Step 4.2: Add test-only accessors and a test entry point on `HttpServer`** — Append to `http.rs` (within the `impl HttpServer` block):

```rust
    /// Snapshot of the iroh-served counter. Test-visibility helper.
    #[cfg(test)]
    pub fn blob_iroh_served_count_snapshot(&self) -> u64 {
        self.blob_iroh_served_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Snapshot of the libp2p-served counter. Test-visibility helper.
    #[cfg(test)]
    pub fn blob_libp2p_served_count_snapshot(&self) -> u64 {
        self.blob_libp2p_served_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Test entry point: invoke the GET /blob/{hash} dispatcher with a
    /// pre-built request and return the response. Wraps `handle_get_blob`
    /// after the same agent-id extraction the real router does.
    #[cfg(test)]
    pub async fn handle_blob_get_for_test(
        &self,
        req: hyper::Request<hyper::body::Incoming>,
    ) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, crate::error::StorageError>
    {
        let path = req.uri().path().to_string();
        let hash = path.strip_prefix("/blob/").unwrap_or("").to_string();
        let agent_id = Self::extract_agent_id(&req);
        self.handle_get_blob(&hash, agent_id.as_deref()).await
    }
```

Add to `/projects/elohim/elohim/elohim-storage/src/test_util.rs` (already exists per `ls` output) a builder near other test helpers:

```rust
/// Build a `GET /blob/{hash}` request shape used by the dispatcher
/// integration tests. `agent_cid` is set as `X-Agent-Cid` when `Some`.
pub fn blob_get_request(
    hash: &str,
    agent_cid: Option<&str>,
) -> hyper::Request<hyper::body::Incoming> {
    let mut builder = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(format!("/blob/{}", hash));
    if let Some(cid) = agent_cid {
        builder = builder.header("X-Agent-Cid", cid);
    }
    // Use an empty Incoming body via the standard test pattern shipped
    // by hyper-util — match the convention already in use in this file.
    let (_sender, body) = hyper::body::Incoming::channel();
    builder.body(body).unwrap()
}
```

(If `test_util.rs` already has a `_request` builder for another route, mirror its body-construction pattern instead of `Incoming::channel`.)

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --tests 2>&1 | tail -20
```

Expected: compile errors only on the missing dispatcher logic (still). Counters and test entry compile.

- [ ] **Step 4.3: Insert router-driven dispatch at the top of `handle_get_blob`** — In `http.rs` at line 1639, immediately after the `if hash.is_empty()` early-return (around line 1648) and before `parse_content_address` (line 1652), add the iroh-first attempt:

```rust
        // Cutover gate #2 (Phase 11): try iroh-side blob backend first
        // when the caller is iroh-capable AND the blob has a known
        // BLAKE3 alias. Falls through to the legacy SHA256 path on
        // miss, libp2p-only caller, or no IrohBlobStore wired.
        if let Some(ref iroh) = self.iroh_blob_store {
            // Normalize the caller-supplied hash to the form the router expects.
            let normalized_for_router: String = if hash.starts_with("blake3-") {
                hash.to_string()
            } else {
                match crate::blob_store::BlobStore::parse_content_address(hash) {
                    Ok(h) => format!("sha256-{}", h),
                    Err(_) => String::new(), // fall through to legacy parser below
                }
            };

            if !normalized_for_router.is_empty() {
                // Look up alias rows + caller manifest. Any DB or
                // manifest-lookup failure degrades silently to legacy.
                let mut conn_opt = self.db_pool.as_ref().and_then(|p| p.get().ok());
                let blake3_alias = if !normalized_for_router.starts_with("blake3-") {
                    conn_opt.as_mut().and_then(|c| {
                        crate::db::peer_blob_inventory::lookup_blake3_for_sha256(
                            c,
                            &normalized_for_router,
                        )
                        .ok()
                        .flatten()
                    })
                } else {
                    None
                };
                let sha256_alias = if normalized_for_router.starts_with("blake3-") {
                    conn_opt.as_mut().and_then(|c| {
                        crate::db::peer_blob_inventory::lookup_sha256_for_blake3(
                            c,
                            &normalized_for_router,
                        )
                        .ok()
                        .flatten()
                    })
                } else {
                    None
                };

                let caller_manifest = match (agent_id, conn_opt.as_mut()) {
                    (Some(cid), Some(c)) => {
                        crate::p2p_iroh::peer_map::lookup_by_agent_cid(c, cid)
                            .ok()
                            .flatten()
                    }
                    _ => None,
                };

                let choice = crate::http_blob_router::choose_backend(
                    crate::http_blob_router::ChooseInputs {
                        normalized_hash: &normalized_for_router,
                        blake3_alias_for_sha256: blake3_alias,
                        sha256_alias_for_blake3: sha256_alias,
                        self_manifest: self.self_transport_manifest.as_deref(),
                        caller_manifest: caller_manifest.as_ref(),
                    },
                );

                if let crate::http_blob_router::BlobBackendChoice::IrohThenLibp2p {
                    blake3_hash, ..
                } = choice
                {
                    let blake3_hex =
                        blake3_hash.strip_prefix("blake3-").unwrap_or(&blake3_hash);
                    if let Ok(iroh_hash) = blake3_hex.parse::<iroh_blobs::Hash>() {
                        match iroh.get_bytes(iroh_hash).await {
                            Ok(data) => {
                                self.blob_iroh_served_count
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                debug!(
                                    hash = %hash,
                                    backend = "iroh",
                                    size = data.len(),
                                    "blob served from iroh backend (cutover gate #2)"
                                );
                                return Ok(Self::with_cors_headers(Response::builder())
                                    .status(StatusCode::OK)
                                    .header(
                                        header::CONTENT_TYPE,
                                        "application/octet-stream",
                                    )
                                    .header(header::CONTENT_LENGTH, data.len())
                                    .header(
                                        header::CACHE_CONTROL,
                                        "public, max-age=31536000, immutable",
                                    )
                                    .body(Full::new(Bytes::from(data.to_vec())))
                                    .unwrap());
                            }
                            Err(e) => {
                                debug!(
                                    hash = %hash,
                                    error = %e,
                                    "iroh-side miss; falling through to legacy backend"
                                );
                            }
                        }
                    }
                }
            }
        }
```

Just before each existing 200-OK return inside the legacy body (the direct-blob branch around line 1721, the race-fetch hit return around line 1855, the reassembled-from-shards return around line 1904), add:

```rust
                        self.blob_libp2p_served_count
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        debug!(hash = %hash, backend = "libp2p", "blob served from legacy backend");
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test blob_backend_dispatch 2>&1 | tail -30
```

Expected: 2 passing.

- [ ] **Step 4.4: Verify no regression in existing blob-handler tests** —

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib http:: 2>&1 | tail -30
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test bench_blob_perf 2>&1 | tail -20
```

Expected: every previously-passing test still passes.

- [ ] **Step 4.5: Commit** —

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/src/test_util.rs \
        elohim/elohim-storage/tests/blob_backend_dispatch.rs
git commit -m "feat(storage): graduate /blob/{hash} to iroh-canonical with libp2p fallback"
```

---

## Task 5: Surface counters via existing observability

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/http.rs` (locate the `handle_status` or `build_status_view` function — search for `info` / `health` matching `1216`-area function `build_status_response`)

**Rationale:** Spec requires "log which backend served at debug level" (done in Task 4) and "a simple counter (existing metrics infra in src/services/system_metrics.rs)". `system_metrics.rs` is per-node probes — it has no counter aggregation. The closest existing surface is the `/health` and `/status` JSON responses. Add `blob_backend_iroh_served` and `blob_backend_libp2p_served` integer fields to whichever response is already declared near line 1216 (`build_status_response`).

- [ ] **Step 5.1: Failing test** — In `http.rs` near other status tests, add:

```rust
#[tokio::test]
async fn status_response_includes_blob_backend_counters() {
    let blob_store = Arc::new(BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap());
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap());
    server.blob_iroh_served_count.store(7, std::sync::atomic::Ordering::Relaxed);
    server.blob_libp2p_served_count.store(13, std::sync::atomic::Ordering::Relaxed);

    let resp = server.handle_status_for_test().await.unwrap();
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["blobs"]["iroh_served"].as_u64(), Some(7));
    assert_eq!(v["blobs"]["libp2p_served"].as_u64(), Some(13));
}
```

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib status_response_includes_blob_backend_counters 2>&1 | tail -20
```

Expected: compile error — `handle_status_for_test` missing OR JSON path mismatch.

- [ ] **Step 5.2: Wire counters into `build_status_response`** — In `http.rs` near line 1236 where `body["blobs"] = serde_json::json!(stats.total_blobs);` is set, change to:

```rust
            let stats = self.blob_store.stats().await?;
            body["blobs"] = serde_json::json!({
                "total": stats.total_blobs,
                "iroh_served": self.blob_iroh_served_count.load(std::sync::atomic::Ordering::Relaxed),
                "libp2p_served": self.blob_libp2p_served_count.load(std::sync::atomic::Ordering::Relaxed),
            });
```

Add a `#[cfg(test)] pub async fn handle_status_for_test(&self) -> Result<Response<Full<Bytes>>, StorageError>` that calls the existing status builder (mirroring the pattern in Task 4.2).

Run:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib status_response_includes_blob_backend_counters 2>&1 | tail -20
```

Expected: passing.

- [ ] **Step 5.3: Commit** —

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): expose blob backend served counters in /status"
```

---

## Task 6: Doorway verification (no code change)

**Files:**
- Read-only: `/projects/elohim/doorway/doorway-service/src/routes/storage_proxy.rs` lines 205–290 (`forward_blob_to_storage`)
- Read-only: `/projects/elohim/doorway/doorway-service/src/routes/blob.rs` lines 174–500
- Read-only: `/projects/elohim/elohim/elohim-storage/src/http.rs` line 9543 (`with_blobs_at("/blob")`)

**Rationale:** Spec calls for "Doorway-side: confirm storage_proxy.rs forwards /blob/* unchanged (no behavior change at doorway, just verify in the plan)". The `forward_blob_to_storage` function strips the `/blob/<hash>` prefix and forwards the request bytes-as-is to elohim-storage's `/blob/{hash}` route. The graduation only changes which backend serves bytes inside elohim-storage; the URL path, request shape, response headers, and status codes are unchanged.

- [ ] **Step 6.1: Inspection checklist** — Confirm with `grep` that doorway never inspects the response body or rewrites the URL:

```bash
grep -n "blake3\|Hash::\|iroh" /projects/elohim/doorway/doorway-service/src/routes/storage_proxy.rs /projects/elohim/doorway/doorway-service/src/routes/blob.rs
```

Expected output: zero matches. If non-zero, add a sub-step to verify that occurrence is read-only/logging only (not behavior-bearing).

- [ ] **Step 6.2: Doorway test still passes** — Run the existing doorway blob tests:

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins blob 2>&1 | tail -20
RUSTFLAGS="" cargo test --lib --bins storage_proxy 2>&1 | tail -20
```

Expected: every previously-passing test still passes (no doorway code changed in this plan).

- [ ] **Step 6.3: Document the verification** — Append a short note to `/projects/elohim/doorway/doorway-service/src/routes/storage_proxy.rs`'s top doc comment (the `//!` block at line 1):

```rust
//! ## Cutover gate #2 (iroh blob graduation)
//!
//! This module is intentionally untouched by the iroh blob graduation
//! (Plan: 2026-05-10-iroh-http-blob-graduation). The blob byte-route
//! contract is unchanged; only the upstream (elohim-storage) chooses
//! between iroh and libp2p backends per request. Doorway forwards the
//! same `/blob/<hash>` URL with the same headers, and caches the
//! response identically regardless of which backend served.
```

Run:

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 6.4: Commit** —

```bash
cd /projects/elohim
git add doorway/doorway-service/src/routes/storage_proxy.rs
git commit -m "docs(doorway): note cutover gate #2 leaves blob proxy unchanged"
```

---

## Task 7: Final parity verification

**Files:**
- All modified files from Tasks 1–6 (no further edits)

- [ ] **Step 7.1: Full storage test sweep** —

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib 2>&1 | tail -20
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test blob_backend_dispatch 2>&1 | tail -20
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib --tests -- -D warnings 2>&1 | tail -20
cargo fmt --check
```

Expected: all green; clippy clean; fmt clean.

- [ ] **Step 7.2: Doorway sweep** —

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -10
cargo fmt --check
```

Expected: all green.

- [ ] **Step 7.3: Manual smoke against a running storage node** — Run elohim-storage with iroh enabled, seed a blob via `IrohBlobStore`, and probe with `curl`:

```bash
# In one shell, start storage with p2p-iroh:
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release --features p2p-iroh -- \
    --storage-dir /tmp/iroh-graduation-smoke --enable-p2p
# In another shell, after seeding a known blob (use existing import API):
curl -s -w '%{http_code}\n' -H 'X-Agent-Cid: did:elohim:smoke-iroh-caller' \
    http://localhost:8090/blob/blake3-<hex> -o /tmp/served.bin
curl -s http://localhost:8090/status | jq '.blobs'
```

Expected: 200; `iroh_served` counter > 0 in the `/status` response. Use `GET` (not `HEAD`) per memory `feedback_head_vs_get_blob_asymmetry`.

- [ ] **Step 7.4: Final commit (if any pending)** — None expected; Tasks 1–6 already commit incrementally.

---

## Self-review checklist

- Spec coverage: cutover gate #2 line 511 is fully addressed by Tasks 2–4 (router selection, iroh-first dispatch, libp2p fallback for libp2p-only callers).
- No placeholders: every step lists the exact file path, line range, and code body.
- Inter-plan references: only `Plane::Blob`, `TransportChoice::Iroh`, `select_transport`, `lookup_by_agent_cid`, and `PeerTransportManifest` from Plan 1 are used; no other Plan 1 internals are assumed. Test fixtures use `iroh_capable_for_test()` / `libp2p_only_for_test()` constructors that Plan 1 must provide; if absent at execution time, Task 2.1 reports BLOCKED.
- Both happy-path (iroh-served) AND fallback-path (libp2p-served) have integration tests in `tests/blob_backend_dispatch.rs` (Task 4.1).
- HTTP wire contract preserved: same `application/octet-stream`, same `Content-Length`, same `Cache-Control`, same status codes; bytes are byte-identical regardless of backend.
- No new crate dependencies introduced.
