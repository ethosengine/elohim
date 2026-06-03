---
id: inventory-verifier-research
status: design   # research + fix plan; proposed one-function relaxation now present in inventory_gossip.rs (verify via CI)
cites:
  - ../../../elohim/elohim-storage/src/p2p/inventory_gossip.rs   # is_blob_hash_shaped — the verifier this research analyzes
---

# Inventory verifier wire-format mismatch — research + plan

**Date:** 2026-05-28
**Branch:** sprint/cross-pillar-cleanup (fix lands on `dev`)
**Bug site:** `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132-134`
**Related memory:** `feedback_structural_verify_canonical_wire_shape`
**Related plan:** `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md`
**Status:** Phase 1+2 (research + plan) — awaiting operator confirm before Phase 3 implementation.

---

## TL;DR

`is_blob_hash_shaped` requires bare 64-hex; every blob-store producer in the
codebase emits prefixed `sha256-<64-hex>`. The verifier has been dropping
100% of real inventory snapshots and deltas on every peer since T13 landed
2026-05-02 (commit `9169ab99d`) — ~26 days latent. Producer side is fully
consistent (no bare-hex producer anywhere). Sibling verifier
(`IdentityBindingGossip::verify_structural`) is non-bugged because it has no
hash-shape constraint. Fix is a one-function relaxation plus fixture refresh,
no migration risk because nothing has been written to `peer_blob_inventory`
via the gossip-receive path during the latency window.

---

## R1 — Producer-side trace (write → wire)

Every blob-hash producer in `elohim-storage` emits the prefixed
`sha256-<64-hex>` shape:

| Site | Format | Notes |
|------|--------|-------|
| `blob_store.rs:113` | `format!("sha256-{}", hex::encode(result))` | Primary `BlobStore::store` write |
| `blob_store.rs:404, 413, 422, 436` | `format!("sha256-{}", hex)` | All read-side accessors normalize to prefixed |
| `blob_store.rs:777` (test) | `format!("sha256-{}", "0".repeat(64))` | Confirms on-disk filename shape |
| `blob_store.rs:613-655` (`list_hashes`) | Filesystem directory-walk → returns filename verbatim | Filename IS the prefixed hash |
| `inventory_broadcaster.rs:32-44` (`StaticInventory`) | Pass-through `Vec<String>` | No transformation |
| `http.rs:2298` (`StoreAdapter::current_hashes`) | `self.0.list_hashes().unwrap_or_default()` | Production caller — passes prefixed through |
| `inventory_broadcaster.rs:70-83` (`build_snapshot`) | `inventory.current_hashes()` direct | Production path: prefixed → snapshot.hashes |

`BlobStore::parse_content_address` (line 140) accepts either prefixed or
bare from external HTTP input (defensive on the way in) but normalizes
internally to prefixed; nothing it returns leaves the crate as bare hex.

`p2p_iroh/mod.rs:72` re-exports `iroh_blobs::Hash as BlobHash`, but that
type is independent of the libp2p inventory wire — different content path
(iroh-blobs Hash is a 32-byte BLAKE3-or-similar; not what flows through the
libp2p inventory topic). No bleed.

**Conclusion: producer side is CONSISTENT. All real production traffic on
`elohim/inventory/blob` carries prefixed `sha256-<64-hex>` strings.**

→ {confirmed}

---

## R2 — Sibling-verifier inventory

Every `verify_structural` and structural shape check in `elohim-storage`:

| Site | Verifier | Hash-shape check? | Bug? |
|------|----------|-------------------|------|
| `inventory_gossip.rs:83` (`BlobInventorySnapshot::verify_structural`) | `is_blob_hash_shaped(hash)` for every `hashes[]` entry | YES (bare 64-hex) | **THE BUG** |
| `inventory_gossip.rs:112` (`BlobInventoryDelta::verify_structural`) | `is_blob_hash_shaped(hash)` for every `added[]` + `removed[]` | YES (bare 64-hex) | **THE BUG (same predicate)** |
| `identity_binding_gossip.rs:100` (`IdentityBindingGossip::verify_structural`) | None — only non-empty checks on `agent_cid`, `valid_from`, `binding_action_hash`, `signature` | NO | Not bugged. `binding_action_hash` is a Holochain ActionHash (base64), not a sha256 — different format, different concern. |
| `p2p/mod.rs:4890` (call site for IdentityBindingGossip) | Delegates to above | N/A | Not bugged. |
| `p2p/mod.rs:5077, 5124` (call sites for inventory snapshot/delta) | Delegates to the bugged predicate; on failure logs `warn!` and drops the message — does NOT touch the DB | YES | Receive path drops everything; correct error-handling, wrong predicate. |
| `services/federator.rs:173` | Comment-only reference to identity_binding pattern | N/A | Not bugged. |
| `error.rs:103` (`InvalidContentAddress`) | Error variant used in `parse_content_address` | N/A | Used at HTTP input boundary, not on the gossip wire. |

`is_blob_hash_shaped` is the only structural predicate that constrains hash
format in elohim-storage, and the only one with the bug. No widening
required.

→ {confirmed — single-site bug, no sibling-verifier scope creep}

---

## R3 — Test-coverage inventory

The verifier has tests; they all pass; none of them exercise the production
wire shape.

| Test | Fixture shape | Exercises `verify_structural` against production shape? |
|------|---------------|---------------------------------------------------------|
| `inventory_gossip.rs:140-225` (unit) | `"a".repeat(64)` (bare hex) | NO — fixture lies about the wire format |
| `inventory_broadcaster.rs:128-204` (unit) | `"a".repeat(64)` or `"aaa"` | NO — builds snapshots, never verifies them |
| `iroh_gossip_parity.rs` (integration) | `"0".repeat(64)` (bare hex) | NO — uses a local `BlobInventoryDeltaWire` copy and never calls `verify_structural` |
| `iroh_gossip_dual_publish_inventory.rs` (integration) | `"a".repeat(64)`, `"b".repeat(64)`, `"c".repeat(64)` | NO — round-trips bytes through `from_bytes`/`to_bytes`, never verifies structurally |
| `inventory_writer_smoke.rs` (integration) | Various, including `"sha256-…"` for DB rows | NO — tests writer/apply paths, never the verify path |
| `peer_topology_phase4.rs`, `phase4_projector_topology_integration.rs` (integration) | `"sha256-cidA-pt4"` etc. (prefixed but truncated) | NO — direct DB inserts, never go through wire codec |
| `bench_gossip_perf.rs` (bench) | Various | NO — performance only |

**The test gap is total.** No test in the suite calls
`BlobInventorySnapshot { hashes: vec![format!("sha256-{}", …)] }
.verify_structural()`. The unit tests pass because the fixtures match the
broken predicate; the integration tests pass because they skip the verifier
entirely.

**Sweettest coverage:** zero — `elohim/holochain/tests/sweettest/` contains
no inventory-gossip integration. Inventory is libp2p-layer, not HC-layer, so
the sweettest harness is not the appropriate locus anyway.

**Property-based coverage:** zero — `proptest` is not a dependency of
elohim-storage. A round-trip property test
(`producer.emit() ∘ verifier.verify() == Ok(())`) would have caught this on
first run.

→ {confirmed — sibling-bug-class: test fixture drift from production wire shape}

---

## R4 — Consequences-of-fix audit

**Downstream consumers of `BlobInventorySnapshot.hashes` / `BlobInventoryDelta.{added,removed}`:**

| Consumer | Site | Format expectation |
|----------|------|--------------------|
| `peer_blob_inventory::apply_snapshot` | `p2p/mod.rs:5092-5097` | `&[String]` — TEXT column, no shape check |
| `peer_blob_inventory::apply_delta` | `p2p/mod.rs:5140-5147` | `&[String]` × 2 — TEXT columns, no shape check |
| Schema: `peer_blob_inventory.blob_hash` | Diesel TEXT | No CHECK constraint on format |

The DB layer takes whatever string the snapshot carries. After the fix,
peers will start writing prefixed `sha256-<hex>` into
`peer_blob_inventory.blob_hash`, which is what the test fixtures in
`peer_topology_phase4.rs:80-108` already use (`"sha256-cidA-pt4"`,
`"sha256-blobX-e2e"`). The downstream projector
(`phase4_projector_topology_integration.rs:109` asserts
`replica_count == 3`) operates on whatever string is in the column and is
agnostic to format.

**Persisted-data migration risk:** none.

- The receive-path verifier has been rejecting 100% of remote snapshots/deltas
  for ~26 days. Nothing has been written to `peer_blob_inventory` via the
  gossip-receive path during that window.
- There may be rows from test fixtures (mixed shapes — see
  `peer_topology_phase4.rs:80-108`) or from earlier code paths predating
  T13. Those don't round-trip through `verify_structural` (it's a wire-level
  gate, not a DB read-time check), so the relaxed verifier doesn't interact
  with them.
- The relaxed verifier is strictly more permissive, so any existing
  in-flight bytes that the old verifier rejected continue to be rejected
  only when they're malformed (peer-id empty, signature empty, etc.).

**`/api/v1/diagnostics/inventory-parity`:** `compute_parity` (line 235)
diffs `local_store.current_hashes()` (filesystem, prefixed) against
`last_gossiped` (whatever the broadcaster stored; also prefixed since both
sides go through the same `StoreAdapter`). Format match was already correct;
the parity report has been correct all along, even when receive-side gossip
was being dropped.

→ {non-issue — relaxed verifier is purely additive}

---

## R5 — Holochain-context check

The inventory gossip topic `elohim/inventory/blob` is published over
**libp2p gossipsub** (and in parallel over iroh-gossip via DualGossipPublisher
on `p2p-iroh` builds). It is wholly outside the Holochain DHT layer.

The hash format (`sha256-<hex>`) is the elohim-storage blob CAS identifier,
not an HC ActionHash, EntryHash, or DhtOpHash. No HC 0.6 upgrade path
interacts with this verifier; no DHT migration is implied.

`IdentityBindingGossip` (the sibling verifier) DOES reference an HC
ActionHash via `binding_action_hash`, but its verifier just non-empty-checks
the string — no shape constraint that could go stale across HC versions.

→ {non-issue — bug is isolated to libp2p/iroh-gossip wire layer}

---

# Plan

## Chosen option: A (relax the verifier to the canonical wire format)

```rust
/// Sha256 wire shape check: canonical `sha256-<64 lowercase hex>` per
/// elohim-storage/CLAUDE.md ("Wire-level identifiers — `sha256-{hex}` —
/// keep their existing names"). The producer side (BlobStore::store and
/// list_hashes) emits this shape uniformly; this verifier must match.
fn is_blob_hash_shaped(s: &str) -> bool {
    s.strip_prefix("sha256-")
        .is_some_and(|hex| {
            hex.len() == 64
                && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}
```

### Why Option A over B and C

**Option B (strip prefix at producer) is rejected:**
- CLAUDE.md is explicit: "Wire-level identifiers — `/blob/{hash}`,
  `BlobStore`, `sha256-{hex}` — keep their existing names." That's gospel.
- The blob-store filesystem layout uses `sha256-<hex>` as the actual
  filename. Stripping at gossip-emit time would require either: (a) doing
  string surgery on every snapshot publish (wasted work) or (b) changing
  the on-disk filename convention (massive blast radius, breaks all
  HTTP routes, breaks `parse_content_address`, breaks `custody.rs` —
  T17/T22 already standardised on `sha256-<hex>` per `custody.rs:48-50`).
- Even setting CLAUDE.md aside, the verifier is a one-line predicate; the
  producer is a fan-out across the crate. Single source of truth lives at
  the predicate.

**Option C (accept both prefixed and bare) is rejected:**
- No producer in the codebase emits bare hex on this wire. Accepting bare
  would advertise "we tolerate format X" while X is never emitted, which
  invites future drift and confusion.
- R4 confirmed no persisted bare-hex data needs round-tripping.
- A predicate that accepts two forms is a predicate that has to be tested
  twice and reasoned about every time someone touches it. Strict is simpler.

**Option A wins on:**
- CLAUDE.md alignment.
- Single-line change.
- Matches every producer.
- One shape to test, document, and reason about.

## Test strategy

1. **Update existing fixtures** at `inventory_gossip.rs:140-225` to use
   real-shape wire strings via a helper:

   ```rust
   /// Canonical wire-format hash fixture — matches `BlobStore::store`
   /// output. Do NOT change to bare hex; production producer always
   /// emits this prefix, and `verify_structural` enforces it.
   fn sha256_wire(byte: char) -> String {
       format!("sha256-{}", std::iter::repeat(byte).take(64).collect::<String>())
   }
   ```

2. **Add a regression test** that explicitly fails on the pre-fix verifier:

   ```rust
   #[test]
   fn snapshot_verify_accepts_canonical_wire_prefix() {
       let snapshot = BlobInventorySnapshot {
           peer_id: "12D3KooWtest1".into(),
           hashes: vec![sha256_wire('a')],
           snapshot_at: 1_700_000_000_000_000,
           sequence: 1,
           signature: vec![0x00],
       };
       assert_eq!(snapshot.verify_structural(), Ok(()));
   }
   ```

   Plus the negative: `bare hex without prefix is rejected` and `wrong
   prefix (sha512-, blake3-) is rejected`.

3. **Add a producer↔verifier round-trip test** at the boundary between
   `inventory_broadcaster` and `inventory_gossip` (new file or appended to
   `inventory_broadcaster.rs::tests`):

   ```rust
   #[test]
   fn snapshot_built_from_real_blobstore_passes_verify() {
       let temp = tempfile::TempDir::new().unwrap();
       let store = futures::executor::block_on(BlobStore::new(temp.path())).unwrap();
       futures::executor::block_on(store.store(b"payload-a")).unwrap();
       futures::executor::block_on(store.store(b"payload-b")).unwrap();

       let hashes = store.list_hashes().unwrap();
       let inv = StaticInventory::new(hashes);
       let alloc = SequenceAllocator::new(0);
       let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1);

       assert_eq!(snapshot.verify_structural(), Ok(()),
           "snapshot built from real BlobStore output must pass structural verify");
   }
   ```

   This is the test that, going forward, makes producer/verifier drift
   impossible without a red CI signal. It uses the real `BlobStore::store`
   to derive a real prefixed hash, then runs it through the real producer
   adapter and the real verifier. Async-blocking-from-sync is awkward;
   if `futures::executor::block_on` is unavailable, mark the test
   `#[tokio::test]`.

4. **Property-based:** skip for this PR (proptest is not a dep; adding it
   widens scope). Capture as follow-up if the team wants it.

## Sibling-verifier scope

Per R2, none. `IdentityBindingGossip` has no hash-shape predicate; no other
verifier in elohim-storage constrains hash format. This PR touches only
`inventory_gossip.rs` and (for the new round-trip test) optionally
`inventory_broadcaster.rs`.

## Sweettest impact

None. Inventory gossip is libp2p-layer, not HC-layer. Adding a sweettest
would be misplaced. The new producer↔verifier round-trip test inside
`elohim-storage/src/p2p/` covers the same ground at the right layer.

## Rollout

Self-healing. The next gossip cycle after the fix deploys on a peer will
re-broadcast its snapshot and that peer's inventory will populate at every
remote receiver running fixed code. No cluster-side cleanup required.

Cross-references in adjacent memory:
- `project_generous_probes_pattern` gates floor-restoration on this
  verifier fix landing — that memory's "Related" line cites this exact
  bug. Once this PR ships, the floor-restoration gate at
  `genesis/orchestrator/data/deployments.json` becomes unblocked (still
  also gated on shem CPU contention resolution; not in this PR's scope).

## Out of scope (deferred)

- Adding `proptest` to elohim-storage as a dev-dep and writing a
  full producer↔verifier property test. Captured here for follow-up.
- Reviewing other libp2p gossip topics
  (`elohim/observations/*`, recovery/revocation, dual-publish) for the
  same fixture-vs-wire-shape blind spot. R2 already confirmed no other
  hash-shape predicates exist; this would be a broader test-hygiene
  audit, not a bug fix.
- Renaming the `BlobInventorySnapshot.hashes` field to something like
  `blob_addresses` to match the `sha256-<hex>` shape (it's "addresses"
  more than "hashes"). Cosmetic; would break the wire serialization;
  not worth it.

---

# Stop-condition check (before Phase 3)

- R1 producers consistent? YES (all prefixed). Fix-shape choice unchanged.
- R2 sibling verifier divergent failure mode? NO sibling, no divergence.
- R4 persisted-data round-trip risk? NO (no migration needed).
- All five sections close on {confirmed | non-issue} — no open questions
  that would block implementation.

Ready for operator confirm on Option A and the sibling scope of "none."

---

# Close-out — Sprint 0 landed

Sprint 0 executed via subagent-driven-development, six commits on `sprint/cross-pillar-cleanup`:

- `c5d6dd827` test(storage): inventory verifier fixtures use canonical wire shape (red)
- `d21e0bc0c` fix(storage): inventory verifier accepts canonical sha256-<hex> wire (green)
- `680a5c0f2` feat(storage): BlobAddress newtype constructor-validates sha256-<hex> wire
- `a04c07982` refactor(storage): VerifyError uses thiserror; doc polish on BlobAddress
- `702b0cb05` feat(storage): thread BlobAddress through inventory producer/consumer types
- `6f66ffeb5` test(storage): producer-to-verifier round-trip from real BlobStore

**Outcome:** the bug-class is closed at two layers — the predicate accepts the canonical wire format (immediate fix), and the `BlobAddress` newtype makes producer-↔-verifier drift literally unrepresentable at the type level (Stage-2 hardening that survives all future graduations of the REA-compute-substrate roadmap).

**R2 confirmed at implementation time:** no sibling verifiers. `IdentityBindingGossip::verify_structural` has only non-empty checks; no other hash-shape predicate exists in the crate.

**Pattern memory captured:** `.claude/memory/project_canonical_wire_shape_newtype_pattern.md` — the newtype-as-validator pattern with `BlobAddress` as the reference instance.

**Roll-up disposition:** Sprint 0 commits are code-complete on `sprint/cross-pillar-cleanup`. The cluster-verification step (re-probe alpha's gossip propagation) is deferred to whenever this branch lands on `dev` with other work — `project_generous_probes_pattern`'s floor-restoration gate becomes unblocked at that point.

**Follow-up cleanup observed during reviews (not blocking):**
- `WARN_ONCE` duplication between `http.rs` `StoreAdapter::current_hashes` and `p2p/mod.rs` broadcast path — same "non-canonical filename" warning at two independent call sites. Candidates for extraction into a shared helper.
- `sha256_wire` test helper triplicated across three test modules (inventory_broadcaster.rs × 2, inventory_gossip.rs × 1). Identical body each time.
- `compute_parity` converts `Vec<BlobAddress>` → `HashSet<String>` via `.as_str().to_string()` (allocates); `String::from(addr)` (moves) is the zero-allocation form.
