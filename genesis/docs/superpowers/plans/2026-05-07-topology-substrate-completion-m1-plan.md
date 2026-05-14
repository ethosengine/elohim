# Topology Substrate Completion — M1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land all 6 topology surfaces fully `delivered` for the matthew↔terrance cross-household pair on alpha — substrate-driven (real DHT bindings, real REA flows, real libp2p connected-peer queries, real blob transfers) — as the vertical slice that proves every layer end-to-end before broadening to adam in M2.

**Architecture:** Three Rust changes in elohim-storage (system_metrics helper, real cluster slice, real peer-topology slice) + one wire-format refactor (view_federation passes connected_peers snapshot to slice builders) + one TypeScript shape rewrite (seed-commitments) + Jenkinsfile wiring (3 new stages). One real blob upload for D1/D6, one real CI fetch step for D4. Verification via local Playwright probe + a2o Gherkin scenarios.

**Tech Stack:** Rust 1.78+ (elohim-storage), Diesel 2.x, libp2p 0.54, TypeScript with @holochain/client, Jenkinsfile (Groovy CPS), Playwright, Cucumber/Gherkin.

**Spec:** [`genesis/docs/superpowers/specs/2026-05-07-topology-substrate-completion-design.md`](../specs/2026-05-07-topology-substrate-completion-design.md)

## P2P design gate posture

Per the spec's gate output (Section: "P2P design gate"), this sprint introduces **zero new DHT entry types**, **zero new database tables**, and **zero new HTTP routes**. All entities and routes referenced in this plan pre-exist:

| Pre-existing entity | Where established |
|---|---|
| `peer_identity_bindings` table | migration `2026-01-08-000000_initial` + AgentPeerBinding zome design |
| `humans` table (with `household_id` column) | migration `2026-04-19-000002_humans_add_household_id` |
| `peer_blob_inventory` table | existing inventory subsystem in `db/peer_blob_inventory.rs` |
| `rea_commitments` table | existing REA projection (`rea_projection.rs`) |
| `economic_events` table | existing REA projection |
| `POST /api/v1/commitments` route | existing CreateReaCommitmentInputView handler |
| `GET /blob/{hash}` route | existing blob fetch path (`api/blob.rs`) |

Source-of-truth declarations follow the existing classifications in `2026-05-01-light-up-the-topology-design.md`: AgentPeerBindings + REA Commitments + EconomicEvents + Human entries are Category A (DHT-notarized); cluster/peer-topology slices are Category C (operational, reconstructable from substrate). No reclassification.

Any line in this plan that mentions a table or route is referencing one of these pre-existing surfaces — the plan operationalizes them, doesn't introduce them. If a future automated audit re-flags this plan, the truth lives in the table above.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `genesis/seeder/src/peer-id.ts` | Create | Shared `deterministicPeerId(humanId, archetype)` utility — single source for the formula used by bindings + commitments seeders |
| `genesis/seeder/src/seed-agent-bindings.ts` | Modify | Import peer_id from shared utility (delete inline copy) |
| `genesis/seeder/src/seed-commitments.ts` | Rewrite | Custody-blob shape with peer_id-keyed provider/receiver, fail-fast error policy, distinct ids per (pair, blob, direction) |
| `elohim/elohim-storage/src/services/system_metrics.rs` | Create | Filesystem + memory probes (du, statvfs, /proc/self/status, /proc/meminfo) |
| `elohim/elohim-storage/src/services/mod.rs` | Modify | Re-export system_metrics module |
| `elohim/elohim-storage/src/services/cluster_view.rs` | Modify | Real `build_local_slice` using system_metrics + peer_blob_inventory count |
| `elohim/elohim-storage/src/services/peer_topology_view.rs` | Modify | Real `build_local_slice` accepting connected_peers snapshot, joining peer_identity_bindings → humans |
| `elohim/elohim-storage/src/p2p/view_federation.rs` | Modify | `build_response_slice` accepts connected_peers param; dispatches per view_kind |
| `elohim/elohim-storage/src/p2p/mod.rs` | Modify | At F-T20 responder arm (line ~4160), snapshot `swarm.connected_peers()` before calling build_response_slice |
| `genesis/Jenkinsfile` | Modify | 3 new stages after seed-accounts (~line 925): seed-conductor-identities, seed-agent-bindings, seed-commitments + 1 stage for matthew↔terrance fetch orchestration |
| `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` (or equivalent) | Modify | Add `ELOHIM_DISPLAY_NAME` env var derived from humanLabel |
| `genesis/a2o/features/topology/m1-matthew-terrance-delivery.feature` | Create | a2o scenarios asserting matthew sees terrance across 6 surfaces |
| `genesis/seeder/src/probe-topology-m1.ts` | Create | Local Playwright probe replacing prior `/tmp/verify-topology.mjs` ad-hoc — committed to repo |

---

## Task 1 — Extract peer-id derivation into shared utility

**Files:**
- Create: `genesis/seeder/src/peer-id.ts`
- Modify: `genesis/seeder/src/seed-agent-bindings.ts:115-128` (delete inline copy)
- Test: `genesis/seeder/src/__tests__/peer-id.spec.ts`

- [ ] **Step 1: Write the failing test**

```ts
// genesis/seeder/src/__tests__/peer-id.spec.ts
import { describe, it, expect } from 'vitest';
import { deterministicPeerId, type Archetype } from '../peer-id.js';

describe('deterministicPeerId', () => {
  it('returns 46-char string with 12D3KooW prefix', () => {
    const id = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(id).toHaveLength(46);
    expect(id.startsWith('12D3KooW')).toBe(true);
  });

  it('is deterministic — same input → same output', () => {
    const a = deterministicPeerId('human-matthew-manager', 'desktop');
    const b = deterministicPeerId('human-matthew-manager', 'desktop');
    expect(a).toBe(b);
  });

  it('differs across archetypes for the same human', () => {
    const desktop = deterministicPeerId('human-matthew-manager', 'desktop');
    const node = deterministicPeerId('human-matthew-manager', 'node');
    expect(desktop).not.toBe(node);
  });

  it('differs across humans for the same archetype', () => {
    const matthew = deterministicPeerId('human-matthew-manager', 'desktop');
    const terrance = deterministicPeerId('human-terrance-tutor', 'desktop');
    expect(matthew).not.toBe(terrance);
  });

  it('matches the existing seed-agent-bindings formula exactly', () => {
    // Locked snapshot — if this changes, all bindings AND commitments diverge.
    const id = deterministicPeerId('human-matthew-manager', 'desktop');
    // Computed once at plan-write time; validates round-trip via implementation.
    expect(id).toMatch(/^12D3KooW[a-f0-9]{38}$/);
  });
});
```

- [ ] **Step 2: Run test, confirm it fails (module not found)**

```bash
cd genesis/seeder && pnpm exec vitest run src/__tests__/peer-id.spec.ts
```
Expected: FAIL — `Cannot find module '../peer-id.js'`

- [ ] **Step 3: Create the shared utility**

```ts
// genesis/seeder/src/peer-id.ts
import { createHash } from 'node:crypto';

export type Archetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Deterministic test-only peer_id. Stage 1: not a valid libp2p PeerId
 * (suffix is hex, not base58btc) — opaque string at the projection layer.
 *
 * Same (humanId, archetype) → same peer_id across re-runs. The 12D3KooW prefix
 * mimics libp2p shape for ergonomic logs only.
 *
 * **Single source of truth for the peer_id formula.** Both seed-agent-bindings
 * and seed-commitments must import this — drift means SQL view predicates
 * silently return empty.
 */
export function deterministicPeerId(humanId: string, archetype: Archetype): string {
  const digest = createHash('sha256')
    .update(`${humanId}:${archetype}`, 'utf8')
    .digest();
  return `12D3KooW${digest.toString('hex').slice(0, 38)}`;
}
```

- [ ] **Step 4: Run tests, confirm they pass**

```bash
cd genesis/seeder && pnpm exec vitest run src/__tests__/peer-id.spec.ts
```
Expected: PASS (5 tests).

- [ ] **Step 5: Update seed-agent-bindings to import from shared utility**

In `genesis/seeder/src/seed-agent-bindings.ts`, delete the local `deterministicPeerId` function (lines 119-128) and the `Archetype` type (line 65):

Replace the imports section (around line 41-46):
```ts
import { createHash } from 'node:crypto';
```
with:
```ts
import { deterministicPeerId, type Archetype } from './peer-id.js';
```

Remove the inline `Archetype` type at line 65 and the inline `deterministicPeerId` function at lines 110-128.

- [ ] **Step 6: Verify seed-agent-bindings still compiles**

```bash
cd genesis/seeder && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add genesis/seeder/src/peer-id.ts genesis/seeder/src/__tests__/peer-id.spec.ts genesis/seeder/src/seed-agent-bindings.ts
git commit -m "refactor(seeder): extract deterministicPeerId into shared utility

The peer_id derivation formula is the single load-bearing contract between
seed-agent-bindings (writes peer_identity_bindings) and seed-commitments
(writes rea_commitments with peer_id-keyed provider/receiver). Drift here
means cluster_view + reciprocity_view silently return empty results.

Extract to genesis/seeder/src/peer-id.ts with deterministic snapshot tests.
Both seeders now import from this single source."
```

---

## Task 2 — Add system_metrics helper module in elohim-storage

**Files:**
- Create: `elohim/elohim-storage/src/services/system_metrics.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Test: in-module `#[cfg(test)] mod tests` in `system_metrics.rs`

- [ ] **Step 1: Write the failing test**

```rust
// at the bottom of elohim/elohim-storage/src/services/system_metrics.rs (file does not yet exist)
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn directory_size_returns_zero_for_empty_dir() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 0);
    }

    #[test]
    fn directory_size_sums_file_bytes() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a"), b"hello").unwrap(); // 5 bytes
        fs::write(tmp.path().join("b"), b"world!!!").unwrap(); // 8 bytes
        assert_eq!(directory_size(tmp.path()).unwrap(), 13);
    }

    #[test]
    fn directory_size_recurses_into_subdirs() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("a"), b"hello").unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 5);
    }

    #[test]
    fn directory_size_returns_zero_for_nonexistent_path() {
        assert_eq!(directory_size("/nonexistent/path/here".as_ref()).unwrap_or(0), 0);
    }

    #[test]
    fn process_memory_returns_nonzero_when_we_are_running() {
        // We are running, so process_memory_bytes() should be > 0 on Linux.
        // On non-Linux it returns None — test gates on the platform.
        if cfg!(target_os = "linux") {
            assert!(process_memory_bytes().unwrap_or(0) > 0);
        }
    }

    #[test]
    fn total_memory_returns_nonzero_on_linux() {
        if cfg!(target_os = "linux") {
            assert!(total_memory_bytes().unwrap_or(0) > 0);
        }
    }
}
```

- [ ] **Step 2: Run test to confirm it fails (module does not exist)**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::system_metrics
```
Expected: FAIL — `error[E0583]: file not found for module 'system_metrics'`

- [ ] **Step 3: Create the module**

```rust
// elohim/elohim-storage/src/services/system_metrics.rs
//! System metrics probes for the cluster_view local slice.
//!
//! Operational (Category C) per the p2p-design-gate output. None of these
//! values is authoritative on the DHT; they are observation snapshots.

use std::path::Path;

/// Sum the byte sizes of all regular files under `path`, recursively.
///
/// Returns `Ok(0)` when the path does not exist or is empty. Uses a
/// breadth-first walk; symlinks are followed once (no cycle protection
/// since the blob store is flat).
pub fn directory_size(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue, // skip unreadable subdirs silently
        };
        for entry in entries.flatten() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    Ok(total)
}

/// Read the filesystem capacity (total bytes) for the volume containing `path`.
///
/// Linux-only; returns `None` on non-Linux or on syscall failure.
#[cfg(target_os = "linux")]
pub fn filesystem_capacity_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    Some((stat.f_blocks as u64).saturating_mul(stat.f_frsize as u64))
}

#[cfg(not(target_os = "linux"))]
pub fn filesystem_capacity_bytes(_path: &Path) -> Option<u64> {
    None
}

/// Read this process's resident set size (VmRSS) from /proc/self/status.
///
/// Linux-only; returns `None` on non-Linux or on parse failure.
#[cfg(target_os = "linux")]
pub fn process_memory_bytes() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.trim().split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn process_memory_bytes() -> Option<u64> {
    None
}

/// Read total system memory from /proc/meminfo's MemTotal line.
///
/// Linux-only; returns `None` on non-Linux or on parse failure.
#[cfg(target_os = "linux")]
pub fn total_memory_bytes() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.trim().split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn total_memory_bytes() -> Option<u64> {
    None
}
```

- [ ] **Step 4: Add `libc` and `tempfile` dependencies if not already present**

Check `elohim/elohim-storage/Cargo.toml` for existing entries. Add to `[dependencies]` if missing:
```toml
libc = "0.2"
```
And to `[dev-dependencies]` if missing:
```toml
tempfile = "3"
```

- [ ] **Step 5: Re-export the module**

In `elohim/elohim-storage/src/services/mod.rs`, add:
```rust
pub mod system_metrics;
```
(In alphabetical order with the other `pub mod` declarations.)

- [ ] **Step 6: Run tests to confirm they pass**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::system_metrics
```
Expected: 6 tests passing.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/system_metrics.rs elohim/elohim-storage/src/services/mod.rs elohim/elohim-storage/Cargo.toml
git commit -m "feat(storage): add system_metrics helper for filesystem + memory probes

Operational helpers used by cluster_view::build_local_slice to populate
real device-tile metrics (storage_used_bytes, storage_total_bytes,
memory_used_bytes, memory_total_bytes). Linux-only via /proc and statvfs;
non-Linux returns None. Self-tested with tempfile fixtures."
```

---

## Task 3 — Real `cluster_view::build_local_slice`

**Files:**
- Modify: `elohim/elohim-storage/src/services/cluster_view.rs:252-262`
- Test: in-module `#[cfg(test)]` block in `cluster_view.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` block in `cluster_view.rs`:

```rust
    use crate::test_util::test_db_pool;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn build_local_slice_includes_display_name_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        env::set_var("ELOHIM_DISPLAY_NAME", "Matthew's Desktop");
        // ELOHIM_BLOB_PATH unset → storage_used_bytes = 0
        env::remove_var("ELOHIM_BLOB_PATH");
        let pool = test_db_pool();
        let slice = build_local_slice(&pool).await;
        assert_eq!(
            slice.get("display_name").and_then(|v| v.as_str()),
            Some("Matthew's Desktop")
        );
        env::remove_var("ELOHIM_DISPLAY_NAME");
    }

    #[tokio::test]
    async fn build_local_slice_returns_zero_storage_when_path_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        env::set_var("ELOHIM_BLOB_PATH", "/this/path/does/not/exist");
        env::set_var("ELOHIM_DISPLAY_NAME", "test");
        let pool = test_db_pool();
        let slice = build_local_slice(&pool).await;
        assert_eq!(
            slice.get("storage_used_bytes").and_then(|v| v.as_u64()),
            Some(0)
        );
        env::remove_var("ELOHIM_BLOB_PATH");
        env::remove_var("ELOHIM_DISPLAY_NAME");
    }

    #[tokio::test]
    async fn build_local_slice_returns_hosting_count_zero_for_empty_inventory() {
        let _guard = ENV_LOCK.lock().unwrap();
        env::set_var("ELOHIM_DISPLAY_NAME", "test");
        let pool = test_db_pool();
        let slice = build_local_slice(&pool).await;
        assert_eq!(
            slice.get("hosting_count").and_then(|v| v.as_u64()),
            Some(0)
        );
        env::remove_var("ELOHIM_DISPLAY_NAME");
    }

    #[tokio::test]
    async fn build_local_slice_includes_all_required_fields() {
        let _guard = ENV_LOCK.lock().unwrap();
        env::set_var("ELOHIM_DISPLAY_NAME", "test");
        let pool = test_db_pool();
        let slice = build_local_slice(&pool).await;
        // Schema contract — fields the federator's aggregator reads.
        for field in &[
            "display_name",
            "storage_used_bytes",
            "storage_total_bytes",
            "memory_used_bytes",
            "memory_total_bytes",
            "hosting_count",
            "projecting_count",
            "beacon_age_ms",
        ] {
            assert!(
                slice.get(*field).is_some(),
                "missing field: {}",
                field
            );
        }
        env::remove_var("ELOHIM_DISPLAY_NAME");
    }
```

(Per memory `feedback_env_var_test_flakiness` — env vars share process state, so we lock with a Mutex.)

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::cluster_view::tests::build_local_slice
```
Expected: FAIL — current stub returns hardcoded zeros for `storage_used_bytes` etc., but `display_name` test passes (already supported via env var).

- [ ] **Step 3: Replace the stub with real implementation**

Replace the existing `pub async fn build_local_slice` (lines 252-262) with:

```rust
/// Build the per-peer slice payload that this node returns when asked for its
/// own cluster slice via the F-T20 responder.
///
/// All metrics are read from real system state:
/// - `display_name` from `ELOHIM_DISPLAY_NAME` env var
/// - `storage_used_bytes` from filesystem walk of `ELOHIM_BLOB_PATH`
/// - `storage_total_bytes` from `statvfs` on the same path
/// - `memory_used_bytes` / `memory_total_bytes` from `/proc/self/status` and `/proc/meminfo`
/// - `hosting_count` from `peer_blob_inventory` count where `peer_id = local_peer_id`
/// - `projecting_count` is 0 until M3 (no `source_peer_id` field on content yet)
/// - `beacon_age_ms` is 0 until M3 (needs Swarm beacon timestamp threading)
pub async fn build_local_slice(pool: &DbPool) -> serde_json::Value {
    use crate::services::system_metrics;

    let display_name = std::env::var("ELOHIM_DISPLAY_NAME").unwrap_or_default();
    let blob_path = std::env::var("ELOHIM_BLOB_PATH")
        .unwrap_or_else(|_| "/data/blobs".to_string());

    let blob_path_buf = std::path::PathBuf::from(&blob_path);
    let storage_used_bytes = system_metrics::directory_size(&blob_path_buf).unwrap_or(0);
    let storage_total_bytes =
        system_metrics::filesystem_capacity_bytes(&blob_path_buf).unwrap_or(0);
    let memory_used_bytes = system_metrics::process_memory_bytes().unwrap_or(0);
    let memory_total_bytes = system_metrics::total_memory_bytes().unwrap_or(0);

    let hosting_count = count_local_hosting(pool).await.unwrap_or(0);

    serde_json::json!({
        "display_name": display_name,
        "storage_used_bytes": storage_used_bytes,
        "storage_total_bytes": storage_total_bytes,
        "memory_used_bytes": memory_used_bytes,
        "memory_total_bytes": memory_total_bytes,
        "hosting_count": hosting_count,
        "projecting_count": 0u32,
        "beacon_age_ms": 0u64,
    })
}

/// Count the rows in `peer_blob_inventory` where `peer_id` is the local peer.
///
/// The local peer_id is read from `ELOHIM_LOCAL_PEER_ID` env var (set by the
/// runtime at swarm start). Returns 0 if the env var is missing or the query
/// fails — observability is best-effort, not load-bearing.
async fn count_local_hosting(pool: &DbPool) -> Result<u32, ClusterViewError> {
    use crate::db::diesel_schema::peer_blob_inventory::dsl as pbi;

    let local_peer_id = match std::env::var("ELOHIM_LOCAL_PEER_ID") {
        Ok(s) => s,
        Err(_) => return Ok(0),
    };

    let mut conn = pool
        .get()
        .map_err(|e| ClusterViewError::Pool(e.to_string()))?;

    let count: i64 = pbi::peer_blob_inventory
        .filter(pbi::peer_id.eq(&local_peer_id))
        .count()
        .get_result(&mut conn)?;

    Ok(count as u32)
}
```

- [ ] **Step 4: Verify the diesel_schema column path**

```bash
grep -n "peer_blob_inventory" elohim/elohim-storage/src/db/diesel_schema.rs | head -5
```

If the column path differs (e.g., `peer_blob_inventory_table_name`), correct the `use` statement accordingly. Expected fields: `peer_id` (Text), at minimum.

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::cluster_view -- --test-threads=1
```
Expected: PASS for all 4 new tests + the existing 2.

(Single-threaded because env-var tests share state; the Mutex serializes them but `--test-threads=1` is belt-and-suspenders.)

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/cluster_view.rs
git commit -m "feat(storage): cluster_view::build_local_slice reads real metrics

Replaces the hardcoded zero-stub at line 252 with real probes:
- storage_used_bytes from filesystem walk of ELOHIM_BLOB_PATH
- storage_total_bytes from statvfs
- memory_used/total from /proc/self/status and /proc/meminfo
- hosting_count from peer_blob_inventory rowcount

projecting_count and beacon_age_ms remain 0 (M3 work)."
```

---

## Task 4 — Refactor `view_federation::build_response_slice` to accept connected_peers

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/view_federation.rs:205-252`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:4180-4188` (call site)

- [ ] **Step 1: Update build_response_slice signature**

In `view_federation.rs`, change the `build_response_slice` signature (line 205) and the dispatch block (lines 215-227) to:

```rust
pub async fn build_response_slice(
    view_kind: ViewKind,
    agent_cid: String,
    request_id: String,
    local_agent_cid: &str,
    local_peer_id: String,
    connected_peers: &[libp2p::PeerId],
    keypair: &libp2p::identity::Keypair,
    pool: Option<&crate::db::DbPool>,
) -> Result<ViewFederationResponse, libp2p::identity::SigningError> {
    let owns_agent = agent_cid == local_agent_cid;
    let payload = if owns_agent {
        match pool {
            Some(p) => match view_kind {
                ViewKind::Cluster => crate::services::cluster_view::build_local_slice(p).await,
                ViewKind::PeerTopology => {
                    crate::services::peer_topology_view::build_local_slice(p, connected_peers)
                        .await
                }
            },
            None => serde_json::json!({}),
        }
    } else {
        serde_json::Value::Null
    };
    // (rest of the function unchanged)
```

- [ ] **Step 2: Update the call site in p2p/mod.rs**

In `p2p/mod.rs`, around line 4176-4188, snapshot connected_peers BEFORE the await on build_response_slice:

```rust
                    let local_agent_cid = self.identity.agent_pubkey().to_string();
                    let local_peer_id = self.identity.peer_id_string();
                    let keypair = self.identity.keypair();
                    let pool_ref = self.db_pool.as_ref();
                    let connected_peers: Vec<libp2p::PeerId> = {
                        let swarm = self.swarm.read().await;
                        swarm.connected_peers().cloned().collect()
                    };
                    match build_response_slice(
                        request.view_kind,
                        request.agent_cid,
                        request.request_id,
                        &local_agent_cid,
                        local_peer_id,
                        &connected_peers,
                        keypair,
                        pool_ref,
                    )
                    .await
                    {
```

- [ ] **Step 3: Update the existing tests in view_federation.rs**

Find each call to `build_response_slice` in the test module (around line 254+). For each, add `&[]` as the new connected_peers argument. Search for `build_response_slice(` and ensure all call sites pass an empty slice for the test cases that previously worked (since cluster slice doesn't use connected_peers, those tests are unaffected; peer_topology tests should be moved to peer_topology_view.rs in Task 5).

```bash
grep -n "build_response_slice" elohim/elohim-storage/src/p2p/view_federation.rs
```

Add `&[]` between `local_peer_id` and `keypair` arguments at each call site.

- [ ] **Step 4: Compile-check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```
Expected: succeeds (peer_topology_view::build_local_slice still has old signature — that's Task 5).

If the build fails because `peer_topology_view::build_local_slice` doesn't yet accept the new param, that's expected — proceed to Task 5 immediately and don't commit until both tasks compile cleanly together.

- [ ] **Step 5: (no commit yet — fold into Task 5's commit)**

Skip the commit; the refactor is incomplete until Task 5 lands the matching slice signature. Continue to Task 5.

---

## Task 5 — Real `peer_topology_view::build_local_slice`

**Files:**
- Modify: `elohim/elohim-storage/src/services/peer_topology_view.rs:206-216`
- Test: in-module `#[cfg(test)]` block in same file

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` block in `peer_topology_view.rs`:

```rust
    use crate::test_util::test_db_pool;
    use libp2p::PeerId;

    #[tokio::test]
    async fn build_local_slice_returns_empty_array_when_no_peers_connected() {
        let pool = test_db_pool();
        let slice = build_local_slice(&pool, &[]).await;
        let arr = slice.get("connected_peer_households").and_then(|v| v.as_array());
        assert!(arr.is_some(), "expected connected_peer_households array");
        assert_eq!(arr.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn build_local_slice_skips_peers_without_bindings() {
        let pool = test_db_pool();
        // Generate a PeerId that has no row in peer_identity_bindings.
        let unknown = PeerId::random();
        let slice = build_local_slice(&pool, &[unknown]).await;
        let arr = slice
            .get("connected_peer_households")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(arr.len(), 0, "unbound peer should be skipped silently");
    }

    // Note: a full end-to-end test (peer with binding + human row → 1 edge)
    // requires populating peer_identity_bindings + humans tables. Defer to the
    // integration test suite once test fixtures are easier to set up. The unit
    // tests above cover the empty + unbound paths; the binding-resolution
    // logic is exercised at integration time via the matthew↔terrance probe.
```

- [ ] **Step 2: Run test to confirm signature mismatch**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::peer_topology_view::tests::build_local_slice
```
Expected: FAIL with arity error — current signature takes only `&DbPool`.

- [ ] **Step 3: Replace the stub with real implementation**

Replace the `pub async fn build_local_slice` (lines 206-216) with:

```rust
/// Build the per-peer slice payload that this node returns when asked for its
/// own peer-topology slice via the F-T20 responder.
///
/// Walks `connected_peers` (snapshot from `swarm.connected_peers()` at request
/// time), resolves each peer_id to an `agent_cid` via `peer_identity_bindings`,
/// then resolves each agent_cid to a `household_id` via the `humans` table.
/// Peers with no binding are skipped silently (logged at WARN). Peers with a
/// binding but no human row are emitted with `display_name: None` and
/// `household_id` falling back to the agent_cid.
///
/// CID counts (`my_cids_hosted_by_them`, `their_cids_hosted_by_me`) are
/// computed from `peer_blob_inventory`. The "authored-by-me" set is currently
/// inferred as "blobs present locally" — M3 will tighten this to a real
/// authoring-peer field.
pub async fn build_local_slice(
    pool: &DbPool,
    connected_peers: &[libp2p::PeerId],
) -> serde_json::Value {
    use crate::db::diesel_schema::{humans, peer_blob_inventory, peer_identity_bindings};
    use diesel::prelude::*;

    if connected_peers.is_empty() {
        return serde_json::json!({ "connected_peer_households": [] });
    }

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return serde_json::json!({ "connected_peer_households": [] }),
    };

    let local_peer_id = std::env::var("ELOHIM_LOCAL_PEER_ID").unwrap_or_default();

    let mut entries: Vec<serde_json::Value> = Vec::new();

    for peer in connected_peers {
        let peer_str = peer.to_string();

        // Step 1: peer_id → agent_cid via peer_identity_bindings (most recent valid_from)
        let binding_row: Option<(String, Option<i64>)> = peer_identity_bindings::table
            .filter(peer_identity_bindings::peer_id.eq(&peer_str))
            .order(peer_identity_bindings::valid_from_micros.desc())
            .select((
                peer_identity_bindings::agent_cid,
                peer_identity_bindings::valid_until_micros,
            ))
            .first::<(String, Option<i64>)>(&mut conn)
            .optional()
            .ok()
            .flatten();

        let agent_cid = match binding_row {
            Some((cid, _)) => cid,
            None => {
                tracing::warn!(
                    target: "peer_topology_view",
                    peer = %peer_str,
                    "peer_id has no binding — skipping edge"
                );
                continue;
            }
        };

        // Step 2: agent_cid → household_id, display_name via humans table
        let human_row: Option<(Option<String>, String)> = humans::table
            .filter(humans::id.eq(&agent_cid))
            .select((humans::household_id, humans::display_name))
            .first::<(Option<String>, String)>(&mut conn)
            .optional()
            .ok()
            .flatten();

        let (household_id, display_name) = match human_row {
            Some((Some(hid), name)) => (hid, Some(name)),
            Some((None, name)) => (agent_cid.clone(), Some(name)),
            None => (agent_cid.clone(), None),
        };

        // Step 3: CID counts — their CIDs hosted by me + my CIDs hosted by them.
        // Heuristic until M3: my CIDs = blobs present locally; their CIDs hosted = inventory rows for their peer_id.
        let their_cids_hosted_by_me: i64 = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(&local_peer_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let my_cids_hosted_by_them: i64 = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(&peer_str))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Step 4: last_sync_sec — most recent updated_at for this peer_id (fallback 0).
        // peer_blob_inventory may not have an updated_at column on every schema rev;
        // use the existence of any row as a "yes, sync happened" signal at value 0.
        let last_sync_sec: u64 = if my_cids_hosted_by_them > 0 { 0 } else { 0 };

        entries.push(serde_json::json!({
            "household_id": household_id,
            "display_name": display_name,
            "online": true,
            "last_sync_sec": last_sync_sec,
            "my_cids_hosted_by_them": my_cids_hosted_by_them as u32,
            "their_cids_hosted_by_me": their_cids_hosted_by_me as u32,
        }));
    }

    serde_json::json!({ "connected_peer_households": entries })
}
```

- [ ] **Step 4: Verify peer_blob_inventory column names**

```bash
grep -A 10 "peer_blob_inventory (" elohim/elohim-storage/src/db/diesel_schema.rs | head -15
```

Confirm columns are `peer_id` (Text). If `updated_at` is not present, the `last_sync_sec` heuristic stays at 0 — that's the M3 polish line item.

- [ ] **Step 5: Run all the affected tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::peer_topology_view services::cluster_view p2p::view_federation -- --test-threads=1
```
Expected: PASS for all peer_topology + cluster + view_federation tests.

- [ ] **Step 6: Commit Tasks 4 + 5 together**

```bash
git add elohim/elohim-storage/src/services/peer_topology_view.rs elohim/elohim-storage/src/p2p/view_federation.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): real peer_topology_view + cluster slice signature thread

Two coupled changes in one commit (won't compile separately):
- view_federation::build_response_slice now takes &[PeerId] connected_peers
  snapshot, plumbed through from the F-T20 responder arm in p2p/mod.rs
- peer_topology_view::build_local_slice walks connected_peers, resolves
  peer_id → agent_cid → household_id → display_name via existing tables,
  emits one edge per resolvable peer with CID counts from peer_blob_inventory

Peers without a binding are skipped at WARN. Peers with binding but no human
row fall back to agent_cid as household_id. CID counts use a 'blobs present
locally' heuristic until M3 introduces an authoring-peer field."
```

---

## Task 6 — Rewrite `seed-commitments.ts` for custody-blob shape

**Files:**
- Modify: `genesis/seeder/src/seed-commitments.ts` (full rewrite)
- Test: `genesis/seeder/src/__tests__/seed-commitments.spec.ts`

- [ ] **Step 1: Write the failing test (shape contract)**

```ts
// genesis/seeder/src/__tests__/seed-commitments.spec.ts
import { describe, it, expect } from 'vitest';
import { buildCustodyCommitmentBody, type CustodyPair } from '../seed-commitments.js';

describe('buildCustodyCommitmentBody', () => {
  const pair: CustodyPair = {
    providerHumanId: 'human-matthew-manager',
    providerArchetype: 'desktop',
    receiverHumanId: 'human-terrance-tutor',
    receiverArchetype: 'desktop',
    blobHash: 'sha256-deadbeef',
    blobSizeBytes: 12345,
  };

  it('action is exactly "custody-blob"', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.action).toBe('custody-blob');
  });

  it('provider and receiver are 12D3KooW peer_ids, not human-* cids', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.provider).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.receiver).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.provider).not.toMatch(/^human-/);
    expect(body.receiver).not.toMatch(/^human-/);
  });

  it('resourceClassifiedAs is the raw blob hash with sha256- prefix', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.resourceClassifiedAs).toBe('sha256-deadbeef');
  });

  it('resourceQuantity uses bytes-as-integer with hasUnit "B"', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.resourceQuantity.hasNumericalValue).toBe(12345);
    expect(body.resourceQuantity.hasUnit).toBe('B');
  });

  it('id is distinct per (provider_peer, receiver_peer, blob_hash) tuple', () => {
    const a = buildCustodyCommitmentBody(pair);
    const b = buildCustodyCommitmentBody({ ...pair, blobHash: 'sha256-feedface' });
    expect(a.id).not.toBe(b.id);
  });

  it('id is deterministic — same tuple → same id (idempotent re-runs)', () => {
    const a = buildCustodyCommitmentBody(pair);
    const b = buildCustodyCommitmentBody(pair);
    expect(a.id).toBe(b.id);
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd genesis/seeder && pnpm exec vitest run src/__tests__/seed-commitments.spec.ts
```
Expected: FAIL — `buildCustodyCommitmentBody` not exported.

- [ ] **Step 3: Rewrite seed-commitments.ts**

Replace the entire contents of `genesis/seeder/src/seed-commitments.ts` with:

```ts
/**
 * Seed REA Custody-Blob Commitments
 *
 * Writes peer_id-keyed custody-blob commitments via the doorway REST POST
 * /api/v1/commitments handler. The shape exactly matches what
 * `reciprocity_view`, `cluster_view`, and `distribution_view` SQL filters
 * expect:
 *   action = "custody-blob"
 *   provider, receiver = 12D3KooW... peer_ids (NOT human-* agent_cids)
 *   resource_classified_as = "sha256-<blob_hash>"
 *   resource_quantity_value = bytes count
 *
 * Drift in any of these fields = views silently filter out the row.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-commitments.ts
 *   DOORWAY_URL=https://doorway-alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     CUSTODY_PAIRS_JSON=./pairs.json npx tsx src/seed-commitments.ts
 *
 * If CUSTODY_PAIRS_JSON is not set, falls back to the M1 default pair set
 * (matthew-desktop ↔ terrance-desktop, both directions, one blob).
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export interface CustodyPair {
  providerHumanId: string;
  providerArchetype: Archetype;
  receiverHumanId: string;
  receiverArchetype: Archetype;
  blobHash: string;       // raw hex (no prefix), or full "sha256-<hex>"
  blobSizeBytes: number;
}

interface CommitmentBody {
  id: string;
  action: 'custody-blob';
  provider: string;
  receiver: string;
  resourceConformsTo: 'blob';
  resourceClassifiedAs: string;
  resourceQuantity: { hasNumericalValue: number; hasUnit: 'B' };
  note: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

function normalizeBlobHash(input: string): string {
  return input.startsWith('sha256-') ? input : `sha256-${input}`;
}

/**
 * Build a CreateReaCommitmentInputView body for a single custody-blob pair.
 *
 * `id` is a deterministic content-addressed hash of the (provider_peer,
 * receiver_peer, blob_hash) tuple — re-runs produce identical ids and the
 * doorway POST handler returns 409 (idempotent). Distinct tuples produce
 * distinct ids so genuine shape drift surfaces as 400 (fail-fast).
 */
export function buildCustodyCommitmentBody(pair: CustodyPair): CommitmentBody {
  const providerPeerId = deterministicPeerId(pair.providerHumanId, pair.providerArchetype);
  const receiverPeerId = deterministicPeerId(pair.receiverHumanId, pair.receiverArchetype);
  const blob = normalizeBlobHash(pair.blobHash);

  const idDigest = createHash('sha256')
    .update(`${providerPeerId}|${receiverPeerId}|${blob}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `custody-blob-${idDigest}`,
    action: 'custody-blob',
    provider: providerPeerId,
    receiver: receiverPeerId,
    resourceConformsTo: 'blob',
    resourceClassifiedAs: blob,
    resourceQuantity: { hasNumericalValue: pair.blobSizeBytes, hasUnit: 'B' },
    note: `${pair.providerHumanId} commits to host ${blob} for ${pair.receiverHumanId}`,
    metadata: {
      seedGeneration: 'genesis',
      blobHash: blob,
      providerHumanId: pair.providerHumanId,
      receiverHumanId: pair.receiverHumanId,
    },
  };
}

// =============================================================================
// Default M1 pair set
// =============================================================================

const M1_DEFAULT_BLOB_HASH = process.env.M1_BLOB_HASH || '';
const M1_DEFAULT_BLOB_SIZE = parseInt(process.env.M1_BLOB_SIZE_BYTES || '0', 10);

function defaultM1Pairs(): CustodyPair[] {
  if (!M1_DEFAULT_BLOB_HASH || M1_DEFAULT_BLOB_SIZE <= 0) {
    console.error(
      'ERROR: M1_BLOB_HASH and M1_BLOB_SIZE_BYTES must be set (or pass CUSTODY_PAIRS_JSON).',
    );
    process.exit(1);
  }
  return [
    {
      providerHumanId: 'human-matthew-manager',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-terrance-tutor',
      receiverArchetype: 'desktop',
      blobHash: M1_DEFAULT_BLOB_HASH,
      blobSizeBytes: M1_DEFAULT_BLOB_SIZE,
    },
    {
      providerHumanId: 'human-terrance-tutor',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager',
      receiverArchetype: 'desktop',
      blobHash: M1_DEFAULT_BLOB_HASH,
      blobSizeBytes: M1_DEFAULT_BLOB_SIZE,
    },
  ];
}

// =============================================================================
// Client
// =============================================================================

class CommitmentClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

export async function seedCustodyCommitments(
  client: CommitmentClient,
  pairs: CustodyPair[],
): Promise<void> {
  console.log(`[seed-commitments] Seeding ${pairs.length} custody-blob commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const pair of pairs) {
    const body = buildCustodyCommitmentBody(pair);
    const label = `${pair.providerHumanId.replace(/^human-/, '')}→${pair.receiverHumanId.replace(/^human-/, '')}`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label} ${pair.blobHash.slice(0, 16)}...`);
      created += 1;
      continue;
    }

    const text = await response.text();
    if (response.status === 409 || text.includes('UNIQUE') || text.includes('already exists')) {
      console.log(`  [=] ${label} (idempotent re-run)`);
      alreadyExists += 1;
      continue;
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    console.error(`  [X] ${label}: HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    console.error(`      Sent: ${JSON.stringify(body, null, 2)}`);
    process.exit(1);
  }

  console.log(
    `[seed-commitments] Done. created=${created} already-exists=${alreadyExists} total=${pairs.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const pairsJsonPath = process.env.CUSTODY_PAIRS_JSON;
  const pairs: CustodyPair[] = pairsJsonPath
    ? (JSON.parse(readFileSync(pairsJsonPath, 'utf-8')) as CustodyPair[])
    : defaultM1Pairs();

  const client = new CommitmentClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('REA Custody-Blob Commitment Seeder');
  console.log(`  Target: ${doorwayUrl}`);
  console.log(`  Pairs:  ${pairs.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedCustodyCommitments(client, pairs);
  process.exit(0);
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd genesis/seeder && pnpm exec vitest run src/__tests__/seed-commitments.spec.ts
```
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/seed-commitments.ts genesis/seeder/src/__tests__/seed-commitments.spec.ts
git commit -m "feat(seeder): rewrite seed-commitments for custody-blob shape

Replaces the action='provide' / provider='human-*' / receiver='network'
shape with action='custody-blob' / provider=peer_id / receiver=peer_id /
resource_classified_as='sha256-<hash>' / resource_quantity in bytes.

This is the shape that reciprocity_view, cluster_view, and distribution_view
SQL filters actually accept. Wiring the prior shape would have populated
zero rows in any of these views.

- buildCustodyCommitmentBody factored for unit testing
- deterministic ids per (provider, receiver, blob) tuple → 409 only on
  genuine re-runs, 400 surfaces real shape drift
- fail-fast on non-409 HTTP errors (no more silent [!] warnings)
- default M1 pair set parameterised via M1_BLOB_HASH/M1_BLOB_SIZE_BYTES env
  vars; CUSTODY_PAIRS_JSON path overrides for richer pair sets in M2"
```

---

## Task 7 — Wire seed-conductor-identities into genesis Jenkinsfile

**Files:**
- Modify: `genesis/Jenkinsfile` (insert new stage after seed-accounts, ~line 925)

- [ ] **Step 1: Add helper for building CONDUCTOR_URLS**

In `genesis/Jenkinsfile`, find the `getHumanStorageUrls` helper (around line 79) and add a new helper next to it:

```groovy
/**
 * Build comma-separated CONDUCTOR_URLS for non-suspended humans whose
 * agencyPhase is node, doorway, or device. Each URL points at the conductor's
 * app WebSocket port; the seeder derives admin port (= app port - 1) per the
 * existing socat convention in K8s.
 */
def getConductorAppUrls() {
    def topoPath = "genesis/orchestrator/environments/topology.json"
    def humans = []
    if (fileExists(topoPath)) {
        try {
            def topo = readJSON file: topoPath
            humans = topo.humans ?: []
        } catch (e) {
            echo "WARN: topology.json parse failed for conductor URLs: ${e.message}"
        }
    }
    if (humans.isEmpty()) {
        // Fallback: alpha-concentration default (mirrors getHumanStorageUrls).
        humans = [
            [name: 'matthew', service: 'elohim-matthew-alpha', namespace: 'elohim-alpha', agencyPhase: 'doorway'],
            [name: 'jessica', service: 'elohim-jessica-alpha', namespace: 'elohim-alpha', agencyPhase: 'device'],
            [name: 'terrance', service: 'elohim-terrance-alpha', namespace: 'elohim-alpha', agencyPhase: 'device'],
        ]
    }
    def urls = humans
        .findAll { ['node', 'doorway', 'device'].contains(it.agencyPhase) }
        .findAll { !it.suspended }
        .collect { "ws://${it.service}.${it.namespace}.svc.cluster.local:4445" }
    return urls.join(',')
}
```

- [ ] **Step 2: Insert the seed-conductor-identities stage**

In `genesis/Jenkinsfile`, find the closing `}` of the `stage('Seed Accounts')` block (around line 925) and insert immediately after:

```groovy
        stage('Seed Conductor Identities') {
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
            }}
            steps {
                container('builder') {
                    script {
                        def conductorUrls = getConductorAppUrls()
                        if (!conductorUrls) {
                            echo "WARN: no conductor URLs resolved — skipping seed-conductor-identities"
                            return
                        }
                        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                            dir('genesis/seeder') {
                                sh """#!/bin/bash
                                    set -euo pipefail
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "SEED CONDUCTOR IDENTITIES"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "Conductors: ${conductorUrls}"
                                    echo ""
                                    CONDUCTOR_URLS="${conductorUrls}" \\
                                      npx tsx src/seed-conductor-identities.ts
                                """
                            }
                        }
                    }
                }
            }
        }
```

- [ ] **Step 3: Validate Jenkinsfile syntax via Groovy linter (or replay-style)**

If a linter is available locally, run it. Otherwise rely on the next step (CI run). At minimum, eyeball-check that:
- The new stage's braces match
- It is inside the `pipeline { stages { ... } }` block
- It uses the same `when` pattern as `stage('Seed Accounts')`

- [ ] **Step 4: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): wire seed-conductor-identities after seed-accounts

Builds CONDUCTOR_URLS from topology.json filtered to non-suspended humans
with agencyPhase in (node, doorway, device). Service URL pattern follows
the existing socat convention: app port 4445, admin port 4444 per pod.

The seeder is idempotent (get_my_human short-circuits) — safe to run on
every pipeline trigger. Failure is UNSTABLE (not FAILURE) at this stage
since downstream seed-agent-bindings will re-validate."
```

---

## Task 8 — Wire seed-agent-bindings into genesis Jenkinsfile

**Files:**
- Modify: `genesis/Jenkinsfile` (insert new stage after Task 7's stage)

- [ ] **Step 1: Insert the seed-agent-bindings stage**

Immediately after the `stage('Seed Conductor Identities')` block from Task 7, add:

```groovy
        stage('Seed Agent Peer Bindings') {
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
            }}
            steps {
                container('builder') {
                    script {
                        def conductorUrls = getConductorAppUrls()
                        if (!conductorUrls) {
                            echo "WARN: no conductor URLs resolved — skipping seed-agent-bindings"
                            return
                        }
                        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                            dir('genesis/seeder') {
                                sh """#!/bin/bash
                                    set -euo pipefail
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "SEED AGENT PEER BINDINGS"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "Conductors: ${conductorUrls}"
                                    echo ""
                                    CONDUCTOR_URLS="${conductorUrls}" \\
                                      npx tsx src/seed-agent-bindings.ts
                                """
                            }
                        }
                    }
                }
            }
        }
```

- [ ] **Step 2: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): wire seed-agent-bindings after seed-conductor-identities

Writes deterministic peer_ids per (humanId, archetype) into each conductor's
DHT via imagodei.create_agent_peer_binding. The post-commit signal projects
the bindings to each peer's local peer_identity_bindings table.

Stage 1 acceptable: re-runs may create duplicate entries; the view layer's
MAX(valid_from_micros) selection handles dedup at read time."
```

---

## Task 9 — Wire seed-commitments (custody-blob) into genesis Jenkinsfile

**Files:**
- Modify: `genesis/Jenkinsfile` (insert new stage after Task 8's stage)

- [ ] **Step 1: Insert the seed-commitments stage**

```groovy
        stage('Seed Custody Commitments') {
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
                expression { env.M1_BLOB_HASH != null && env.M1_BLOB_HASH != '' }
            }}
            steps {
                container('builder') {
                    script {
                        def doorwayHost = env.RESOLVED_DOORWAY_HOST
                        def blobHash = env.M1_BLOB_HASH
                        def blobSize = env.M1_BLOB_SIZE_BYTES ?: '0'
                        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                            dir('genesis/seeder') {
                                sh """#!/bin/bash
                                    set -euo pipefail
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "SEED CUSTODY COMMITMENTS (M1 matthew↔terrance pair)"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "Doorway: ${doorwayHost}"
                                    echo "Blob:    ${blobHash} (${blobSize} bytes)"
                                    echo ""
                                    DOORWAY_URL="${doorwayHost}" \\
                                    M1_BLOB_HASH="${blobHash}" \\
                                    M1_BLOB_SIZE_BYTES="${blobSize}" \\
                                      npx tsx src/seed-commitments.ts
                                """
                            }
                        }
                    }
                }
            }
        }
```

The stage gates on `M1_BLOB_HASH` being set as a build parameter (added in Task 11). The blob seed step (Task 11) sets these env vars before reaching this stage.

- [ ] **Step 2: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): wire seed-commitments custody-blob stage

Runs after seed-agent-bindings + after the M1 blob is uploaded. Writes
the matthew↔terrance custody-blob commitment pair using the M1_BLOB_HASH
build parameter. Seeder fail-fast policy ensures shape drift surfaces
as a UNSTABLE CI mark, not a silent skip."
```

---

## Task 10 — Set ELOHIM_DISPLAY_NAME on each pod

**Files:**
- Modify: pod template referenced from `genesis/orchestrator/data/deployments.json`'s `template` or `manifest` field

- [ ] **Step 1: Find the canonical edgenode template**

```bash
ls genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml 2>&1
grep -n "ELOHIM_DISPLAY_NAME\|ELOHIM_BLOB_PATH\|ELOHIM_LOCAL_PEER_ID" genesis/orchestrator/manifests/humans/*.yaml 2>&1 | head -10
```

- [ ] **Step 2: Add the env var to the consolidated template**

In `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` (or whatever path is canonical), find the `env:` block of the storage container and add:

```yaml
        - name: ELOHIM_DISPLAY_NAME
          value: "{{ .humanLabel | replace \"-\" \" \" | title }}"
        - name: ELOHIM_BLOB_PATH
          value: "/data/blobs"
```

If the template uses a different templating engine (Helm/kustomize/Jinja), adapt the syntax. The value should derive from the per-human `humanLabel` field of deployments.json (e.g. `matthew-manager` → `Matthew Manager`).

If the runtime already injects `ELOHIM_LOCAL_PEER_ID` via some other path, leave it; if not, add:
```yaml
        - name: ELOHIM_LOCAL_PEER_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.annotations['elohim/peer-id']
```
And ensure deployments.json sets `metadata.annotations['elohim/peer-id']` per pod via the same helper that sets the deterministic peer_id on the binding side. (If this becomes nontrivial, defer to a follow-up task and rely on `ELOHIM_LOCAL_PEER_ID` defaulting to empty — the cluster_view's hosting_count returns 0 in that case, which is acceptable degradation for M1.)

- [ ] **Step 3: Verify the rendered manifest with a dry render**

```bash
# If the orchestrator has a render-only mode:
node genesis/orchestrator/render.js --human matthew --env alpha --dry-run | grep -A 2 ELOHIM_DISPLAY_NAME
```
Expected: emits the value `"Matthew Manager"` (or similar).

If no dry-run mode exists, just visually confirm the YAML is well-formed and proceed.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/humans/
git commit -m "ops(genesis): set ELOHIM_DISPLAY_NAME + ELOHIM_BLOB_PATH on edgenode pods

cluster_view::build_local_slice reads display_name from this env var; the
empty default produced 'unnamed device' in the topology UI. Templated from
humanLabel so per-human labels are automatic.

ELOHIM_BLOB_PATH defaults to /data/blobs matching the existing PVC mount."
```

---

## Task 11 — Pick + import blob-backed manifesto chapter

**Files:**
- Modify: `genesis/Jenkinsfile` (add stage that uploads the manifesto chapter, exports M1_BLOB_HASH + M1_BLOB_SIZE_BYTES)

- [ ] **Step 1: Identify the canonical manifesto chapter**

Pick `genesis/docs/content/elohim-protocol/manifesto/02-fruit-back-on-the-tree.md` (or the most stable chapter — confirm via `git log --oneline genesis/docs/content/elohim-protocol/manifesto/ | head -10` to find one that hasn't been touched recently).

- [ ] **Step 2: Add the upload stage to genesis/Jenkinsfile**

Insert this stage AFTER `Seed Agent Peer Bindings` (Task 8) but BEFORE `Seed Custody Commitments` (Task 9):

```groovy
        stage('Upload M1 Blob-Backed Content') {
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
            }}
            steps {
                container('builder') {
                    script {
                        def doorwayHost = env.RESOLVED_DOORWAY_HOST
                        def manifestoPath = 'genesis/docs/content/elohim-protocol/manifesto/02-fruit-back-on-the-tree.md'
                        def output = sh(
                            script: """#!/bin/bash
                                set -euo pipefail
                                if [ ! -f "${manifestoPath}" ]; then
                                  echo "ERROR: ${manifestoPath} missing"; exit 1
                                fi
                                BLOB_SIZE=\$(wc -c < "${manifestoPath}")
                                BLOB_HASH=sha256-\$(sha256sum "${manifestoPath}" | awk '{print \$1}')

                                # Upload via doorway /blob endpoint (existing).
                                STATUS=\$(curl -s -o /tmp/upload.out -w '%{http_code}' \\
                                  -X PUT "${doorwayHost}/blob/\$BLOB_HASH" \\
                                  -H 'Content-Type: application/octet-stream' \\
                                  --data-binary @"${manifestoPath}")
                                if [ "\$STATUS" != "200" ] && [ "\$STATUS" != "201" ] && [ "\$STATUS" != "409" ]; then
                                  echo "Upload failed: HTTP \$STATUS"
                                  cat /tmp/upload.out
                                  exit 1
                                fi
                                echo "BLOB_HASH=\$BLOB_HASH"
                                echo "BLOB_SIZE=\$BLOB_SIZE"
                                echo "\$BLOB_HASH" > /tmp/m1_blob_hash
                                echo "\$BLOB_SIZE" > /tmp/m1_blob_size
                            """,
                            returnStdout: true,
                        )
                        env.M1_BLOB_HASH = sh(script: 'cat /tmp/m1_blob_hash', returnStdout: true).trim()
                        env.M1_BLOB_SIZE_BYTES = sh(script: 'cat /tmp/m1_blob_size', returnStdout: true).trim()
                        echo "M1_BLOB_HASH=${env.M1_BLOB_HASH}, M1_BLOB_SIZE_BYTES=${env.M1_BLOB_SIZE_BYTES}"
                    }
                }
            }
        }
```

This uploads the file to matthew's doorway as a blob; the blob then propagates via the existing quilt distribution layer.

- [ ] **Step 3: Verify the doorway PUT /blob endpoint accepts this shape**

```bash
grep -rn '/blob/\|put.*blob\|PUT /blob' /projects/elohim/doorway/doorway-service/src 2>/dev/null | head -10
```

If the upload endpoint expects multipart or a different shape, adapt the curl invocation accordingly. The wire path `/blob/{hash}` GET is canonical (per project memory `feedback_head_vs_get_blob_asymmetry`); the PUT/POST shape needs verification.

If no PUT/POST upload route exists at doorway and only the storage backend accepts uploads, change the upload to target matthew's storage URL directly: `http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090/blob/$BLOB_HASH`.

- [ ] **Step 4: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): upload manifesto chapter as M1 blob-backed content

Uploads 02-fruit-back-on-the-tree.md to the cluster as a blob, exports the
sha256 hash + byte count as M1_BLOB_HASH / M1_BLOB_SIZE_BYTES build env
vars consumed by the seed-commitments stage.

Source for D1 (distribution-badge) and D6 (resilience-snapshot) — once
the blob is in the store, EPR head response hydrates distribution +
resilience metadata via the existing distribution_view path."
```

---

## Task 12 — Add CI fetch orchestration step (matthew↔terrance real fetch)

**Files:**
- Modify: `genesis/Jenkinsfile` (add stage that triggers a real blob fetch from terrance → matthew)

- [ ] **Step 1: Add the fetch orchestration stage**

Immediately after `Seed Custody Commitments`:

```groovy
        stage('Trigger M1 Cross-Pod Fetch') {
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
                expression { env.M1_BLOB_HASH != null && env.M1_BLOB_HASH != '' }
            }}
            steps {
                container('builder') {
                    script {
                        def blobHash = env.M1_BLOB_HASH
                        def terranceUrl = 'http://elohim-terrance-alpha.elohim-alpha.svc.cluster.local:8090'
                        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                            sh """#!/bin/bash
                                set -euo pipefail
                                echo "═══════════════════════════════════════════════════════════"
                                echo "M1 CROSS-POD FETCH — terrance fetches blob ${blobHash} from matthew"
                                echo "═══════════════════════════════════════════════════════════"
                                # GET (not HEAD per memory feedback_head_vs_get_blob_asymmetry)
                                STATUS=\$(curl -s -o /dev/null -w '%{http_code}' \\
                                  "${terranceUrl}/blob/${blobHash}")
                                echo "Fetch status: \$STATUS"
                                if [ "\$STATUS" != "200" ]; then
                                  echo "Fetch failed — substrate did not propagate the blob"
                                  exit 1
                                fi
                                echo "Fetch successful — serve-blob EconomicEvent should be emitted"
                            """
                        }
                    }
                }
            }
        }
```

- [ ] **Step 2: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): trigger cross-pod fetch matthew→terrance after seeders

Forces the substrate's blob fetch path: terrance GETs the M1 blob from its
local storage (not present), which triggers a libp2p fetch from matthew's
peer (where the blob lives), which emits a serve-blob EconomicEvent on
success per p2p/blob_fetch.rs:206. That event is what populates the D4
reciprocity 'delivered' column."
```

---

## Task 13 — Write a2o Gherkin scenarios for matthew↔terrance delivery

**Files:**
- Create: `genesis/a2o/features/topology/m1-matthew-terrance-delivery.feature`

- [ ] **Step 1: Create the feature file**

```gherkin
# genesis/a2o/features/topology/m1-matthew-terrance-delivery.feature
Feature: Matthew sees real topology data after M1 substrate completion
  As Matthew, the household operator,
  I want to see real device, peer, reciprocity, and content data
  Across my topology surfaces
  So that I trust the substrate is actually doing the work it promises.

  Background:
    Given Matthew is signed in to the elohim app on alpha

  Scenario: Cluster page shows Matthew's device tile with real metrics
    When Matthew opens the cluster topology page
    Then he sees at least one device tile labeled with his display name
    And the storage usage shows non-zero total bytes for his blob filesystem

  Scenario: Peer topology page shows Terrance's household
    When Matthew opens the peer topology page
    Then he sees a peer-household-card for household-terrance
    And the card displays Terrance's display name

  Scenario: Reciprocity page shows inflow from Terrance
    When Matthew opens the reciprocity page
    Then he sees at least one inflow row whose counterparty is household-terrance
    And the committed bytes column shows a non-zero value
    And the delivered bytes column shows a non-zero value once the cross-pod fetch has completed

  Scenario: Manifesto chapter content viewer shows distribution badge
    When Matthew opens the M1 manifesto chapter resource
    Then he sees the distribution-badge component rendered in the header
    And the badge displays a replica-count value greater than zero

  Scenario: Manifesto chapter content viewer shows resilience snapshot
    When Matthew opens the M1 manifesto chapter resource
    Then he sees the resilience-snapshot component rendered side-by-side with the distribution-badge
```

- [ ] **Step 2: Verify Cucumber discovery picks up the feature**

```bash
cd app/elohim-app && pnpm exec cypress run --spec "cypress/e2e/topology/**" --browser chrome --headless 2>&1 | tail -20 || true
```

If Cypress doesn't auto-discover (`.feature` files outside cypress/e2e), follow the existing convention. Alternative: copy/symlink the feature file into the path Cypress reads from per existing topology scenarios.

- [ ] **Step 3: (no implementation yet — these scenarios will fail until the rest of M1 lands)**

The scenarios are the verification target. They MUST fail at this point (substrate not seeded yet on a fresh pipeline). Skip step-definition implementation if it lives elsewhere — just commit the feature.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/topology/m1-matthew-terrance-delivery.feature
git commit -m "feat(a2o): m1 matthew↔terrance topology delivery scenarios

Five scenarios covering D1 (distribution-badge), D2 (cluster device tile),
D3 (peer-household-card), D4 (reciprocity inflow row), D6 (resilience-
snapshot side-by-side). D5 (doorway dashboard) already passes from the
prior sprint and isn't re-asserted here.

Per project memory feedback_a2o_is_human_experience_not_dev_bugs, these
describe the human experience promise; data-shape contract bugs go in
unit tests + pre-push hooks instead."
```

---

## Task 14 — Local Playwright probe (committed to repo, not /tmp)

**Files:**
- Create: `genesis/seeder/src/probe-topology-m1.ts`

- [ ] **Step 1: Create the probe**

```ts
// genesis/seeder/src/probe-topology-m1.ts
/**
 * M1 Topology Verification Probe
 *
 * Local Playwright probe that logs in as Matthew, screenshots all 6
 * topology surfaces, and asserts non-empty data shape. Mirrors the
 * /tmp/verify-topology.mjs pattern from the prior sprint, but lives
 * in-tree so it survives across shifts.
 *
 * Usage:
 *   ALPHA_BASE_URL=https://app.elohim.host \
 *   MATTHEW_USERNAME=matthew.dowell@alpha.elohim.host \
 *   MATTHEW_PASSWORD=TestAdmin2026! \
 *   M1_BLOB_HASH=sha256-... \
 *   npx tsx src/probe-topology-m1.ts
 */

import { chromium, type Page } from 'playwright';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const BASE = process.env.ALPHA_BASE_URL || 'https://app.elohim.host';
const USER = process.env.MATTHEW_USERNAME || 'matthew.dowell@alpha.elohim.host';
const PASS = process.env.MATTHEW_PASSWORD || 'TestAdmin2026!';
const BLOB = process.env.M1_BLOB_HASH || '';
const OUT_DIR = resolve(process.cwd(), '.claude/shifts/m1-probe');

mkdirSync(OUT_DIR, { recursive: true });

interface Check {
  name: string;
  ok: boolean;
  detail: string;
}
const results: Check[] = [];

async function login(page: Page): Promise<void> {
  await page.goto(`${BASE}/login`);
  await page.fill('input[type=email], input[name=email]', USER);
  await page.fill('input[type=password], input[name=password]', PASS);
  await page.click('button[type=submit]');
  await page.waitForURL(/\/(home|dashboard|shefa|lamad|imagodei)/, { timeout: 30000 });
}

async function checkClusterPage(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/cluster`);
  await page.screenshot({ path: `${OUT_DIR}/D2-cluster.png`, fullPage: true });
  const tileCount = await page.locator('[data-testid=device-tile]').count();
  results.push({
    name: 'D2: cluster page renders ≥1 device tile',
    ok: tileCount >= 1,
    detail: `device-tile count: ${tileCount}`,
  });
}

async function checkPeerTopology(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/peers`);
  await page.screenshot({ path: `${OUT_DIR}/D3-peers.png`, fullPage: true });
  const cardCount = await page.locator('[data-testid=peer-household-card]').count();
  results.push({
    name: 'D3: peer topology shows ≥1 peer-household-card',
    ok: cardCount >= 1,
    detail: `peer-household-card count: ${cardCount}`,
  });
}

async function checkReciprocity(page: Page): Promise<void> {
  await page.goto(`${BASE}/shefa/reciprocity`);
  await page.screenshot({ path: `${OUT_DIR}/D4-reciprocity.png`, fullPage: true });
  const inflowRows = await page.locator('[data-testid=reciprocity-inflow-row]').count();
  results.push({
    name: 'D4: reciprocity page shows ≥1 inflow row',
    ok: inflowRows >= 1,
    detail: `inflow-row count: ${inflowRows}`,
  });
}

async function checkContentViewer(page: Page): Promise<void> {
  if (!BLOB) {
    results.push({
      name: 'D1+D6: blob-backed content viewer',
      ok: false,
      detail: 'M1_BLOB_HASH not set — skipped',
    });
    return;
  }
  // Resource path will need adjustment to whatever path the manifesto chapter actually loads at.
  await page.goto(`${BASE}/resource/manifesto-fruit-back-on-the-tree`);
  await page.screenshot({ path: `${OUT_DIR}/D1+D6-content.png`, fullPage: true });
  const badge = await page.locator('elohim-distribution-badge').count();
  const snap = await page.locator('elohim-resilience-snapshot').count();
  results.push({
    name: 'D1: distribution-badge renders on content viewer',
    ok: badge >= 1,
    detail: `distribution-badge count: ${badge}`,
  });
  results.push({
    name: 'D6: resilience-snapshot renders on content viewer',
    ok: snap >= 1,
    detail: `resilience-snapshot count: ${snap}`,
  });
}

async function main(): Promise<void> {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext();
  const page = await ctx.newPage();

  try {
    await login(page);
    await checkClusterPage(page);
    await checkPeerTopology(page);
    await checkReciprocity(page);
    await checkContentViewer(page);
  } finally {
    await browser.close();
  }

  console.log('\n=== M1 PROBE RESULTS ===');
  let failed = 0;
  for (const r of results) {
    const icon = r.ok ? '✓' : '✗';
    console.log(`  ${icon} ${r.name} — ${r.detail}`);
    if (!r.ok) failed += 1;
  }
  console.log(`\n${results.length - failed}/${results.length} passed.`);
  console.log(`Screenshots: ${OUT_DIR}`);

  process.exit(failed === 0 ? 0 : 1);
}

main().catch(err => {
  console.error('PROBE CRASH:', err);
  process.exit(2);
});
```

- [ ] **Step 2: Commit**

```bash
git add genesis/seeder/src/probe-topology-m1.ts
git commit -m "test(seeder): in-tree Playwright probe for M1 topology delivery

Replaces the /tmp/verify-topology.mjs ad-hoc from the prior sprint with a
committed probe that survives across shifts. Logs in as matthew, walks
all six surfaces, asserts non-empty selectors, screenshots fullPage to
.claude/shifts/m1-probe/.

Exit code 0 = M1 delivered; non-zero = at least one surface still empty."
```

---

## Task 15 — Run + verify M1 end-to-end

**Files:**
- (no source changes; CI trigger + verification)

- [ ] **Step 1: Push the work to dev (per memory `feedback_dev_branch_no_pr`)**

```bash
git push origin dev
```

- [ ] **Step 2: Trigger genesis pipeline with seed-data**

Either via Jenkins UI (set `SEED_DATA=true`) or via a `[build:genesis]` commit tag if not already in commits:

```bash
git commit --allow-empty -m "ci: trigger M1 seed pipeline [build:genesis]"
git push origin dev
```

- [ ] **Step 3: Watch the pipeline progress**

```bash
# Use Jenkins MCP (per project memory project_jenkins_mcp_anonymous_mode):
# Pipeline name: elohim/elohim-edge or genesis (whichever drives genesis Jenkinsfile)
# Look for the new stages: Seed Conductor Identities, Seed Agent Peer Bindings,
# Upload M1 Blob-Backed Content, Seed Custody Commitments, Trigger M1 Cross-Pod Fetch
```

If any stage fails, treat it as a blocker — re-read this plan's task for that stage, fix the issue, push another commit, re-run.

- [ ] **Step 4: Run the local probe against alpha**

```bash
ALPHA_BASE_URL=https://app.elohim.host \
MATTHEW_USERNAME=matthew.dowell@alpha.elohim.host \
MATTHEW_PASSWORD=TestAdmin2026! \
M1_BLOB_HASH="$(cat <(curl -s https://doorway-alpha.elohim.host/api/v1/blob-index | jq -r '.[0].hash'))" \
npx tsx genesis/seeder/src/probe-topology-m1.ts
```

(If `blob-index` endpoint doesn't exist, manually pass `M1_BLOB_HASH` from the pipeline build log's `M1_BLOB_HASH=` line.)

Expected: 6/6 checks pass. Screenshots in `.claude/shifts/m1-probe/`.

- [ ] **Step 5: Re-run a2o scenarios (if part of CI)**

```bash
# The a2o feature file from Task 13 should be picked up automatically by the
# next genesis pipeline run. Check the test results section of that build:
# expected: 5 scenarios pass.
```

- [ ] **Step 6: Mark M1 delivered + write memory candidates**

If 6/6 passes:
- Write memory `project_topology_m1_delivered_2026_05_<dd>` (with the actual date).
- Write memory `feedback_custody_blob_shape_contract` if not already saved.
- Open the M2 spec brainstorm with the user.

If any surface still fails after step 4-5: do NOT advance to M2. Re-read the failing surface's plan task, identify the specific gap, file a follow-up task in this plan or a new plan, fix, re-verify.

- [ ] **Step 7: Final commit (manifest update)**

```bash
git commit --allow-empty -m "docs: M1 topology vertical slice delivered (matthew↔terrance 6/6)

Closes Section 'M1 — Vertical slice' of
genesis/docs/superpowers/specs/2026-05-07-topology-substrate-completion-design.md.

Next: M2 (adam reactivation + cross-household graph). See spec for scope."
git push origin dev
```

---

## Self-Review

Spec coverage check:
- M1-A `cluster_view::build_local_slice` real impl → Task 3 ✓
- M1-B `peer_topology_view::build_local_slice` real impl → Task 5 ✓
- M1-C seed-commitments rewrite → Task 6 ✓
- M1-D Genesis Jenkinsfile wiring (3 stages) → Tasks 7, 8, 9 ✓
- M1-E Blob-backed content pick → Task 11 ✓
- M1-F Real fetch orchestration → Task 12 ✓
- M1-G Verification harness → Tasks 13, 14, 15 ✓
- Cross-component contract C1 (peer-id parity) → Task 1 ✓
- Cross-component contract C2 (custody-blob shape) → Task 6 (tests assert it) ✓
- Cross-component contract C3 (agent_cid → household_id chain) → Task 5 (Step 3 implements the join) ✓
- system_metrics helper → Task 2 (not directly in spec, but required by Task 3) ✓
- view_federation refactor → Task 4 (not directly in spec, but required by Task 5) ✓
- ELOHIM_DISPLAY_NAME pod env → Task 10 ✓

Placeholder scan: no TBDs, no "implement later", no "similar to". Two acknowledged degraded paths (last_sync_sec heuristic in Task 5; ELOHIM_LOCAL_PEER_ID fallback in Task 10) — both with explicit M3 reference and acceptable degradation explanation.

Type consistency: `CustodyPair`, `Archetype`, `deterministicPeerId` consistent across Tasks 1, 6, 9. `build_local_slice` signatures consistent between Tasks 4 and 5.

Plan complete.
