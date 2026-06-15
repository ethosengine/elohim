# Iroh Dataplane Toggle Sprint Result — 2026-06-15

**Branch:** `feat/iroh-dataplane-finish`
**Base:** `660d05b14` (an ancestor of `origin/dev` → the sprint is a clean 2-commit delta + 1 test-fix)
**Sprint commits:** 2 feature commits (+ 1 pre-existing-breakage test fix landed alongside)
**Status:** DONE — both toggles implemented, verified green (default **and** `p2p-iroh` feature builds), libp2p byte-identical. A 4-lens adversarial pre-merge review caught 3 build-blockers my feature-only verification missed (all fixed — see below); merge owned by integrator.

> **Objective:** make iroh-vs-libp2p transport selection togglable two ways — (1) at the node level by config, and (2) per-object by a property on the object — so the two planes can be A/B'd and the measured QUIC/iroh-blobs performance win captured. Hard constraint: **do not step on libp2p** — the toggle is additive and the live libp2p path stays byte-identical.

---

## What landed

### Toggle #1 — node-level (`279ddf8a3`)
A loosely-coupled **`NodeTransport`** seam (`src/node_transport.rs`) — a pure, feature-flag-free trait (`self_cid()`, `status_peer_id()`) with two thin String-wrapping impls (`Libp2pTransport`, `IrohTransport`). `main.rs` builds the right impl per `config.transport_backend` (`ELOHIM_TRANSPORT_BACKEND` env / `transport_backend` TOML; default **Libp2p**). This closes the long-standing **self_cid gap**: in iroh mode the node now derives `self_cid` from its iroh `NodeId` (`config.self_cid = config.self_cid.or_else(...)` — never overwrites an explicit value), so the household-resilience card lights instead of reading dark, and `GET /p2p/status` reports the iroh peerId instead of a 503.

### Toggle #2 — per-object (`82d2e2538`)
A `transport_affinity` property on the blob, honored at the per-request blob-backend decision. `TransportAffinity { Auto, PreferIroh, PreferLibp2p, IrohOnly, Libp2pOnly }` (`Auto` == today's behavior), parsed from a new nullable `peer_blob_inventory.transport_affinity` column (NULL → `Auto`), consulted inside `http_blob_router::choose_backend` to override the negotiated per-request transport for that object only. Operator setter `POST /admin/blob-transport-affinity/{hash}` for A/B experiments.

### Commit log (`660d05b14..HEAD`)
```
cc8d6327f fix(storage): feature-gate per-object affinity setter; close PeerBlobInventoryRow column drift
3c0fb6b80 fix(test): add missing hints field to BlobInventorySnapshot literals
82d2e2538 feat(storage): per-object blob transport affinity (iroh-toggle Wave 2)
279ddf8a3 feat(storage): NodeTransport seam lights resilience card on iroh dataplane
```
The 2 `feat(storage)` commits are the sprint's deliverables; the 2 `fix(...)` commits resolve the build-blockers the pre-merge review caught (see §Pre-merge adversarial review below).

---

## P2P Design Gate — `ObjectTransportAffinity`

- **Classification: Category C (Operational).** Transport affinity is a delivery/routing optimization, not authoritative content — the object's CID, bytes, and meaning are transport-independent; the protocol would not be "lying" if it changed. Fully reconstructable (absence ⇒ `Auto` ⇒ fall back to negotiation + node default). Notarizing a routing hint would be the "granular operational data on the DHT" anti-pattern.
- **Content Address Strategy: Content-Derived (CID).** Keyed by the object's existing content address (`sha256-<hex>` for blobs). No new identity minted; affinity is an attribute hung on the existing CID.
- **Source of Truth: SQLite (operational).** Nullable column `transport_affinity TEXT NULL` on the existing Category-C `peer_blob_inventory` projection. No `dht_anchor_hash`. (Migration header declares source-of-truth + reconstruction = default policy.)
- **Coordinator Zome: none** (operational). **HTTP Route:** no new read route; consumed at the existing `choose_backend` call site. One operator setter (`POST /admin/blob-transport-affinity/{hash}`) for A/B, mirroring existing `/admin/*`.
- **Anti-pattern check:** none apply — keyed by existing CID, no new entry type, no DHT bloat, source-of-truth declared.

### Resolution order (per object, per request)
1. Per-object affinity, if set and ≠ `Auto` → wins (overrides negotiation).
2. Else the per-peer × per-plane negotiated verdict (`select_transport`).
3. Else node-level `transport_backend` default.

The blob plane works **in iroh mode today** without dual swarms, because its "libp2p fallback" (`IrohThenLibp2p`) is a LOCAL legacy SHA256 store read, not a swarm. That's why the per-object blob toggle is the real, A/B-able-now deliverable — on the plane with the biggest measured win.

---

## How to flip the switch

| Lever | Mechanism | Default |
|-------|-----------|---------|
| **Node transport** | `ELOHIM_TRANSPORT_BACKEND=iroh` (env) or `transport_backend = "iroh"` (TOML) | `libp2p` |
| **Per-object (blob)** | `POST /admin/blob-transport-affinity/{hash}` body `{"affinity":"prefer-iroh"}` — kebab: `auto`\|`prefer-iroh`\|`prefer-libp2p`\|`iroh-only`\|`libp2p-only` | `auto` (NULL) |

---

## Test results (post-fix verification, surgical builds, warm `/tmp/iroh-target`)

| Suite | Result |
|-------|--------|
| **`cargo build` (default features — the CI/Docker build)** | ✅ EXIT=0 (Finished dev profile in 2m43s) — was the failing case before the gating fix |
| `cargo build --features p2p-iroh` | ✅ links |
| `--test iroh_gossip_byte_parity` (`--features p2p-iroh`) | ✅ 5/5 (incl. `inventory_snapshot_byte_parity` — libp2p↔iroh byte-parity proof) |
| `--test iroh_gossip_dual_publish_inventory` (`--features p2p-iroh`) | ✅ 2/2 (incl. `inventory_snapshot_reaches_libp2p_when_iroh_absent`) |
| `--lib` (`--features p2p-iroh`) — `node_transport` · `provide_loop_status` · `http_blob_router` · `transport_affinity` · `peer_blob_inventory` | ✅ 40/40 (the new `transport_affinity` struct field is exercised by the existing inventory test suite) |
| `--test schema_contract` (`--features p2p-iroh`) | ✅ 212/212 (incl. `p2p_status_view_matches_schema`, `…_with_null_drain`) |
| `cargo clippy --lib --bins --features p2p-iroh` | ✅ no lints in elohim-storage / sprint files (pre-existing `constitution`-crate warnings only) |
| `cargo fmt --check` on every edited file (`rustfmt --check --edition 2021`) | ✅ all 5 files clean (`http.rs`, `db/models.rs`, the 3 test files) |
| `cargo fmt --check` (crate-wide) | ⚠ pre-existing drift only (`recursion.rs`, `household_resilience.rs`, `chaos_dataplane.rs`, a `lib.rs` re-export-order line) — reproduces at base `660d05b14`; **none introduced by this sprint** |

## Pre-merge adversarial review (4 lenses) — and what it caught

A parallel 4-lens review (libp2p-safety · toggle-correctness · completeness · data-model/migration) ran read-only against the branch diff before merge. It found **3 build-blockers that the `--features p2p-iroh`-only verification structurally could not catch** — because **CI and prod build the *default* feature set** (`cargo build --release`, no `p2p-iroh`), which had not been compiled. All fixed:

| # | Sev | Lens | Issue | Fix |
|---|-----|------|-------|-----|
| 1 | 🔴 HIGH | toggle-correctness | `handle_set_blob_transport_affinity` + its `/admin/blob-transport-affinity/` route arm were **not feature-gated**, yet reference `http_blob_router::TransportAffinity` (only compiled under `p2p-iroh`) → `E0433` in the **default/CI/prod build** that ships the live libp2p node | `#[cfg(feature = "p2p-iroh")]` on both the fn and the route arm; verified by a **default-feature build** |
| 2 | 🔴 HIGH | completeness | The `hints`-missing breakage spanned **5 literals across 3 test files**, not the 2 the handoff documented. The hunt found a *third* file (`iroh_gossip_cross_stack_e2e.rs:140,322`) the first pass never touched | `hints: vec![]` at all 5 sites (`byte_parity` ×1, `dual_publish_inventory` ×2, `cross_stack_e2e` ×2) |
| 3 | 🟠 BORDERLINE | data-model | `PeerBlobInventoryRow` (6 fields) drifted from the table macro (7 columns) — compiled only because the single full-row load was patched with `.select(as_select())`, but the row type couldn't read back the column the setter writes | added `pub transport_affinity: Option<String>` to the struct → coherent again |

Root cause of the `hints` class: `BlobInventorySnapshot` gained a `hints: Vec<BlobHint>` field in commit `8287af29d` (a frontend-eyes change on the base) which updated the in-module constructors but missed the integration tests. Latent because **iroh tests don't compile in CI** (the elohim-storage Dockerfile is `cargo build --release`, default features). The base carries this latent-red too, so the fix is a clean bonus to `dev`.

The review confirmed **SOUND** (with quoted evidence): the libp2p byte-identical guarantee, the full TransportAffinity truth-table (5 variants × negotiated∈{T,F}), the additive migration + Category-C conformance, no new `unwrap`/panic risk-class, and the frozen `derived-libp2p-peer-id` wire string.

**Process lesson (for story-harvest):** feature-gated verification is necessary but **not sufficient** — the *default* feature set is the load-bearing build for CI/prod, and a `#[cfg]`-gated symbol referenced from ungated code only fails there. Always build both feature sets before declaring a feature-gated change merge-ready.

### One documented fast-follow (not a blocker)
The iroh-mode `/p2p/status` returns a minimal `{"peerId": …}` body (vs the full `P2PStatusInfo` on the libp2p path). No `schema_contract` case yet asserts that minimal body validates against `p2p-status-view.schema.json` as an intentional subset. iroh-only output, non-breaking to libp2p — a fast-follow test, not a merge gate.

---

## libp2p safety

The libp2p data path is **byte-identical**:
- `NodeTransport` libp2p arm reproduces the prior `self_cid` / `status_peer_id` derivation exactly; the iroh arm is reached only when the backend is explicitly `iroh`.
- `handle_p2p_status` libp2p branch is an early return of the unchanged `handle.status()`.
- `transport_affinity` is a nullable additive column never read on the libp2p path; `choose_backend` returns the unchanged result for `Auto`/NULL.
- The migration is purely additive (`ADD COLUMN … TEXT NULL`, no backfill, no trigger).

---

## Perf grounding (already measured — no fresh run needed)

The bench harness is built and landed; numbers are **measured loopback data** (release, 2026-05-09), per `genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`. iroh wins p50 on every chatty small-frame plane; wins narrow as payload grows past ~1 MiB (per-frame overhead stops dominating once raw bytes do).

| Plane | REUSE p50 (iroh wins) | FRESH p50 (iroh wins) | Source bench |
|-------|------------------------|------------------------|--------------|
| Blob  | 4×–290× | — | `bench_blob_perf` |
| Sync  | 45×–541× | 19×–50× | `bench_sync_perf` |
| EPR   | 25×–249× | 19×–34× | `bench_epr_perf` |
| EPR-atom | 25×–266× | 18×–25× | `bench_epr_atom_perf` |
| Shard (4 MiB RS) | 1.2×–128× | 1.3×–25× | `bench_shard_perf` |
| View-fed | 59×–290× | 18×–58× | `bench_view_fed_perf` |

The per-object blob toggle lands the switch on the highest-win plane and lets the operator A/B `prefer-iroh` vs `prefer-libp2p` per object against live traffic.

---

## Deferred fast-follows (NOT this merge — separate plans)

- **Wave 4 — iroh-mode reductions:** wire manifest-aware ctors for EPR-atom caller-resolver, view-fed connected-peers, identity-handshake peer-label; seed the local-node `peer_transport_manifest` row. Raises iroh-mode fidelity; not in DoD.
- **Tier-2 — `TransportBackend::Dual` boot mode (the keystone):** boot is mode-exclusive today (`P2PNode` XOR `IrohNode`), so the built-and-tested `DualGossipPublisher` (`main.rs:1741` `if let (Some, Some)`) is structurally dead on the live path. A `Dual` mode would make both swarms co-resident and enable per-object **cross-plane** routing (the unused `IrohOnly`/`Libp2pOnly` gossip verdicts), gossip receive-side over iroh, and revive dual-publish. Big separate plan; the per-object blob toggle does **not** need it.
- Master plan `genesis/docs/superpowers/held/plans/2026-05-10-iroh-delivery-master.md` stays HELD until `alpha-cluster-6peer` returns — only its Wave-4 SOAK gates (#6/#7/#9) need the 6-peer cluster.

## Engineering constraints discovered (for story-harvest)

- **Pins are frozen:** `iroh = 0.92` / `iroh-blobs = 0.94` / `iroh-gossip = 0.92` — the highest versions on the stable `ed25519-dalek 2.2` / `curve25519-dalek 4.1` path; `0.95+` pulls a pre-release crypto path whose published source won't compile. **Never bump.**
- **CI blind spot:** the iroh test suite never compiles in CI (release-build-only Dockerfile), so iroh-test breakage from non-iroh commits stays latent-red on `dev`. The `hints` blocker is the second instance of this class.
- **Mode-exclusive boot** is the structural ceiling on "both planes live" — documented above as the Tier-2 keystone.

---

*Build env (for any follow-up): `CARGO_TARGET_DIR=/tmp/iroh-target` (pool slots under `/projects` hit a cargo fingerprint-ENOENT in this container), `RUSTFLAGS='--cfg getrandom_backend="custom"'`, plain cargo, surgical single-test builds (disk-tight), never run ts-rs codegen.*
