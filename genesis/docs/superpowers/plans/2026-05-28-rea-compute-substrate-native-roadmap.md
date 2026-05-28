# REA Compute Substrate — Native Implementation Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement individual sprints task-by-task. Sprints 0–2 are decomposed to bite-sized checkbox tasks here; Sprints 3–8 are roadmap entries — when picked up, each spawns its own bite-sized plan via `/superpowers:writing-plans`.

**Goal:** Graduate `Mishpat::Commitment` with `action="delegates-compute"` from "shipped schemas + Z.D proof-of-concept design spec" to load-bearing substrate truth for hosting, deploy, qahal moderation, household chores, content authorship delegation, DePIN compute lending, and recovery delegation. Inventory gossip becomes a Stage-1 fast projection of Stage-N Commitment truth; X-API-Key and ad-hoc auth patterns are absorbed and deprecated.

**Architecture:** One substrate primitive (`Mishpat::Commitment` + `bounded_by` event back-reference + single substrate-side bounds validator + signal_kind FeedbackSignal accrual into standing) instantiated across all 7 rows of the gospel-tier generalization table. Each instance gets a concrete Commitment + EconomicEvent schema (mirror of `delegates-compute.schema.json` + `republish-epr.schema.json`); all events carry `bounded_by`; all validators delegate to one substrate-side bounds-validator function. Standing aggregates from signal_kind; matchmaking gates new Commitments on prior standing; elohim mediates capacity collisions.

**Tech Stack:** Rust (Mishpat zome, elohim-storage, doorway-service, steward/node), TypeScript (Angular `elohim-app`, `storage-client-ts` SDK), JSON Schema v1 (canonical wire formats with ts-rs codegen), Holochain HC 0.6, libp2p 0.54, iroh-gossip parity.

**Canon:**
- `genesis/docs/architecture/rea-compute-commitment-primitive.md` — gospel-tier primitive shape, generalization table, auditability properties
- `genesis/docs/architecture/stewardship-over-sovereignty.md` — philosophical anchor for "key custody ≠ sovereignty"
- `genesis/docs/architecture/cradle-to-grave-capability-gradient.md` — how the primitive instantiates across life-stage capacities
- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — Z.D spec (first concrete instance)

**Companion plans:**
- `genesis/docs/research/2026-05-28-inventory-verifier-research.md` — Sprint 0 research + plan note
- `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md` — preceding substrate-rea sprint (Task 10 close-out at bottom; this roadmap picks up where that left off)

---

## Roadmap at a glance

```
                                                            ── time ──▶
Phase A (Foundation — sequential)
  S0 ▣ Inventory verifier + BlobAddress hardening              [in flight, ~1 day]
  S1 ◐ Substrate primitives shipped; Z.D as first instance ABANDONED — see close-out 2026-05-28
  S2 ✓ Bounds validator + standing aggregation primitives      [landed 2026-05-28]

Phase B (Hosting graduation — depends on A; serial within phase)
  S3 ▢ serve-url-projection schema + Commitment authoring
  S4 ▢ Inventory gossip carries bounded_by + validator enforces

Phase C (Pattern replication — depends on A; parallel within phase)
  S5a ▢ Recovery delegation (attest-recovery)
  S5b ▢ Qahal moderation (moderation-action)
  S5c ▢ Household chore (chore-done)
  S5d ▢ Content authorship delegation (publish-revision)
  S5e ▢ DePIN compute lending (provide-cycles)

Phase D (Coordination surface — depends on B + ≥2 of C)
  S6 ▢ Standing-aware matchmaking gate
  S7 ▢ Elohim-mediated dispute resolution

Phase E (Operational surface — depends on D)
  S8 ▢ Revocation propagation + audit-trail UI
```

**Sequencing rules:**
- Phase A is fully sequential. S1 needs the schemas to be wire-fixed (they are), but S2's bounds validator authoring can start in parallel with S1 implementation once S0 ships.
- Phase B is two sprints that can overlap (S3 lands schema, S4 wires it in).
- Phase C sprints are mutually independent and can be run in parallel by different agent teams. Each is a self-contained "copy the Z.D pattern for a new row of the table."
- Phase D needs at least Phase B done plus two of Phase C, so standing has multiple input streams to aggregate (not just one).
- Phase E is the human-facing audit surface; lands last so the data model is stable.

**Total scope:** 11 sprints. Sprints 0–2 are 1–3 days each. Sprints 3–7 are 1–2 weeks each. Sprint 8 is 1 sprint of Angular work. Calendar: depends on parallelism applied at Phase C.

**Phase A status:** Sprint 2 landed 2026-05-28 — bounds_validator + standing extension complete; see `2026-05-28-sprint2-bounds-validator-standing-aggregator.md` close-out. **Sprint 1 substrate-only landing 2026-05-28** — Mishpat::Commitment entry type + delegates-compute + acknowledges-reach-change + republish-epr action + republish_epr_validator + put_epr substrate-correct 503 wiring shipped (T1–T7). Z.D-as-first-instance abandoned mid-flight after design conversation reframed deploy as authorship-delegation, not compute-delegation. Recommended first real instance: **mutual storage replication between family-network peers** (proves resiliency epics, lets each peer compute free-storage vs stewarded-compute aggregates, lets each content item compute resiliency + delivery metrics). See `2026-05-28-sprint1-zd-substrate-correct-deploy.md` close-out for full scope-narrowing rationale.

---

## Agent-team allocation

| Agent type | Role on this roadmap | Sprints |
|------------|----------------------|---------|
| **rust-architect** | Mishpat zome, elohim-storage services, schemas-to-Rust, validators | S0, S1, S2, S3, S4, S5a–e |
| **angular-architect** | UI for Commitment dashboards, audit surfaces, dispute UI | S7, S8 |
| **component-architect / graphos-designer** | Lit elements for Commitment cards, FeedbackSignal badges, standing meters | S8 (Library A + B) |
| **content-pipeline** | Manifest declarations for new Commitment actions per pillar | S3, S5a–e (each pillar updates its manifest) |
| **red-team** | Adversarial probes on validator, revocation propagation, spoofing | S2, S4, S6, S7 |
| **pattern-hunter** | Find existing X-API-Key / ad-hoc auth surfaces ripe for absorption | S2 (single pass, output feeds prioritization) |
| **code-reviewer** | Pre-PR review on every sprint | All |
| **ci-investigator / ci-observer** | CI health monitoring during the campaign | All |
| **librarian / historian / cartographer / storyteller** | Memory hygiene, precedent surfacing, roadmap currency, canonical-story rewrites at substrate-tier landings | Quarterly /memory-ceremony triggered by major sprint landings |
| **agentic-developer** | Overnight shifts on long-running implementation sprints (esp. S5a–e parallel fan-out) | S5a–e |
| **quality-architect / quality-sweep / quality-deep** | Test coverage campaigns per pillar after Commitment lands | S5a–e (post-impl), S8 |

**Operator (matthew) responsibilities:**
- Approve each sprint at brainstorm-end before plan authoring (existing /shift discipline)
- Author each `delegates-compute` Commitment as the operator-steward provider for production rollouts (deploy-svc-agent, doorway-svc-agent, etc.)
- Confirm reach-ceiling choices and rotation TTLs per instance
- Hold revocation custody

---

## Phases — detail

| Phase | Goal | Sprints | Blocks | Blocked by |
|-------|------|---------|--------|------------|
| **A. Foundation** | Substrate primitive lands once and is load-bearing | S0, S1, S2 | All others | Nothing (S0 mid-flight) |
| **B. Hosting graduation** | Inventory gossip becomes a projection of `serve-url-projection` Commitments | S3, S4 | E (audit surface needs hosting Commitments to display) | A |
| **C. Pattern replication** | Other 5 rows of the gospel table instantiated | S5a–S5e | D (standing aggregator needs ≥2 instance streams) | A |
| **D. Coordination surface** | Standing-aware matchmaking + elohim dispute mediation | S6, S7 | E | B + ≥2 of C |
| **E. Operational surface** | Human-facing Commitment dashboard, audit log, revocation UI | S8 | — | D |

---

# Sprint 0 — Inventory verifier fix + BlobAddress hardening

**Status:** in flight (research + plan complete; awaiting operator confirm on Option A + newtype scope, then implementation).

**Goal:** Fix `is_blob_hash_shaped` to accept canonical `sha256-<64-hex>` wire format (Stage-1 unblock for alpha cluster gossip). Add `BlobAddress` newtype that constructor-validates the prefix (Stage-2 type-system hardening that survives all future graduations). Frame the commit as Stage-1 placeholder pending graduation to `serve-url-projection` Commitment.

**Architecture:** Single-line predicate change in `inventory_gossip.rs`; new newtype in same file; serde wiring on `BlobInventorySnapshot.hashes` and `BlobInventoryDelta.{added,removed}` from `Vec<String>` to `Vec<BlobAddress>`; fixture helper that uses canonical wire shape; producer↔verifier round-trip integration test.

**Tech stack:** Rust, serde, rmp_serde, tempfile, tokio.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132-134` (predicate)
- Modify: `elohim/elohim-storage/src/p2p/inventory_gossip.rs:29-55` (struct field types) and tests at 140-225 (fixtures)
- Modify: `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs:32-83` (`StaticInventory`, `build_snapshot`, `build_delta` use `BlobAddress`)
- Modify: `elohim/elohim-storage/src/http.rs:2298` (`StoreAdapter::current_hashes` returns `Vec<BlobAddress>`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:5077-5160` (receive arm threading; type changes only)
- Modify: `elohim/elohim-storage/src/db/peer_blob_inventory.rs` (`apply_snapshot`/`apply_delta` signatures take `&[BlobAddress]`; convert to `&str` for DB write)
- Modify: `elohim/elohim-storage/tests/iroh_gossip_parity.rs` (local wire-shape copy gains the prefix)
- Modify: `elohim/elohim-storage/tests/iroh_gossip_dual_publish_inventory.rs` (fixtures use prefixed)
- Update: `genesis/docs/research/2026-05-28-inventory-verifier-research.md` (close-out section)
- Update: `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md` (close-out append)
- Update: `.claude/memory/feedback_structural_verify_canonical_wire_shape.md` (fix landed; commit SHA; sibling list = none)

### Task 0.1: Refresh fixtures + add canonical-wire helper

- [ ] **Step 1:** Add to `inventory_gossip.rs` test module before existing fixtures:

```rust
/// Canonical wire-format hash fixture — matches `BlobStore::store`
/// output. Production producer always emits this prefix; the verifier
/// enforces it. Do NOT replace with bare hex.
#[cfg(test)]
fn sha256_wire(byte: char) -> String {
    format!("sha256-{}", std::iter::repeat(byte).take(64).collect::<String>())
}
```

- [ ] **Step 2:** Replace every `"a".repeat(64)` / `"b".repeat(64)` / `"c".repeat(64)` in `inventory_gossip.rs:140-225` with `sha256_wire('a')` / `sha256_wire('b')` / `sha256_wire('c')`.

- [ ] **Step 3:** Run existing tests — they should now FAIL on the bare-hex predicate:

```bash
cargo test -p elohim-storage --lib p2p::inventory_gossip:: 2>&1 | tail -30
```

Expected: `snapshot_verify_passes_well_formed` fails with `InvalidHashFormat("sha256-aaaa…aa")`.

- [ ] **Step 4:** Commit the test-update-only change (failing tests):

```bash
git add elohim/elohim-storage/src/p2p/inventory_gossip.rs
git commit -m "test(storage): inventory verifier fixtures use canonical wire shape (red)"
```

### Task 0.2: Fix the verifier predicate (green)

- [ ] **Step 1:** Replace lines 131-134 of `inventory_gossip.rs`:

```rust
/// Sha256 wire shape check: canonical `sha256-<64 lowercase hex>` per
/// `elohim-storage/CLAUDE.md` ("Wire-level identifiers — `sha256-{hex}` —
/// keep their existing names"). Every producer in the crate emits this
/// shape (BlobStore::store → list_hashes → StoreAdapter::current_hashes);
/// this verifier matches.
fn is_blob_hash_shaped(s: &str) -> bool {
    s.strip_prefix("sha256-")
        .is_some_and(|hex| {
            hex.len() == 64
                && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}
```

- [ ] **Step 2:** Run unit tests; expect PASS:

```bash
cargo test -p elohim-storage --lib p2p::inventory_gossip::
```

- [ ] **Step 3:** Add the negative-case regression tests inside the existing test module:

```rust
#[test]
fn snapshot_verify_rejects_bare_hex_without_prefix() {
    let mut s = sample_snapshot();
    s.hashes.push("a".repeat(64));  // missing sha256- prefix
    assert!(matches!(
        s.verify_structural(),
        Err(VerifyError::InvalidHashFormat(_))
    ));
}

#[test]
fn snapshot_verify_rejects_wrong_prefix() {
    let mut s = sample_snapshot();
    s.hashes.push(format!("sha512-{}", "a".repeat(64)));
    assert!(matches!(
        s.verify_structural(),
        Err(VerifyError::InvalidHashFormat(_))
    ));
}

#[test]
fn snapshot_verify_rejects_wrong_hex_length() {
    let mut s = sample_snapshot();
    s.hashes.push(format!("sha256-{}", "a".repeat(32)));  // 32 instead of 64
    assert!(matches!(
        s.verify_structural(),
        Err(VerifyError::InvalidHashFormat(_))
    ));
}
```

- [ ] **Step 4:** Run; expect PASS:

```bash
cargo test -p elohim-storage --lib p2p::inventory_gossip::
```

- [ ] **Step 5:** Commit:

```bash
git add elohim/elohim-storage/src/p2p/inventory_gossip.rs
git commit -m "fix(storage): inventory verifier accepts canonical sha256-<hex> wire (green)"
```

### Task 0.3: Add BlobAddress newtype (Stage-2 hardening)

- [ ] **Step 1:** Add to `inventory_gossip.rs` after the `INVENTORY_TOPIC` constant:

```rust
/// Canonical wire-format blob address. Wraps a `sha256-<64-lower-hex>`
/// string and constructor-validates the shape. Once a `BlobAddress`
/// exists, every downstream consumer can rely on the format without
/// re-checking.
///
/// ## Stage trajectory
/// This is Stage-1 substrate placeholder. The destination is Stage-4 of
/// the REA-compute-substrate roadmap, where hosting becomes a
/// `Mishpat::Commitment` with `action="serve-url-projection"` and
/// `BlobAddress` becomes the address type referenced in the Commitment's
/// scope field. The newtype survives the graduation unchanged.
///
/// See `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlobAddress(String);

impl BlobAddress {
    /// Construct from a canonical wire string. Returns Err if the shape
    /// is not `sha256-<64 lowercase hex>`.
    pub fn new(s: impl Into<String>) -> Result<Self, VerifyError> {
        let s = s.into();
        if is_blob_hash_shaped(&s) {
            Ok(Self(s))
        } else {
            Err(VerifyError::InvalidHashFormat(s))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BlobAddress {
    type Error = VerifyError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<BlobAddress> for String {
    fn from(b: BlobAddress) -> Self {
        b.0
    }
}

impl std::fmt::Display for BlobAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

- [ ] **Step 2:** Add unit tests for the newtype:

```rust
#[test]
fn blob_address_accepts_canonical_wire() {
    let addr = BlobAddress::new(sha256_wire('a')).unwrap();
    assert_eq!(addr.as_str(), sha256_wire('a'));
}

#[test]
fn blob_address_rejects_bare_hex() {
    assert!(matches!(
        BlobAddress::new("a".repeat(64)),
        Err(VerifyError::InvalidHashFormat(_))
    ));
}

#[test]
fn blob_address_round_trips_via_serde() {
    let addr = BlobAddress::new(sha256_wire('b')).unwrap();
    let json = serde_json::to_string(&addr).unwrap();
    let decoded: BlobAddress = serde_json::from_str(&json).unwrap();
    assert_eq!(addr, decoded);
}

#[test]
fn blob_address_deserialize_rejects_invalid_shape() {
    let json = "\"not-a-hash\"";
    let result: Result<BlobAddress, _> = serde_json::from_str(json);
    assert!(result.is_err());
}
```

- [ ] **Step 3:** Run; expect PASS:

```bash
cargo test -p elohim-storage --lib p2p::inventory_gossip::blob_address
```

- [ ] **Step 4:** Commit:

```bash
git add elohim/elohim-storage/src/p2p/inventory_gossip.rs
git commit -m "feat(storage): BlobAddress newtype constructor-validates sha256-<hex> wire"
```

### Task 0.4: Thread BlobAddress through producer + consumer types

- [ ] **Step 1:** Change `BlobInventorySnapshot.hashes: Vec<String>` → `Vec<BlobAddress>` and `BlobInventoryDelta.{added,removed}: Vec<String>` → `Vec<BlobAddress>` in `inventory_gossip.rs:29-55`. Drop the now-redundant `is_blob_hash_shaped` loop from both `verify_structural` impls (the newtype guarantees it; keep the non-empty/peer-id/signature checks). Adjust the `VerifyError::InvalidHashFormat` test usage — that error variant now fires at deserialization time, not verify_structural.

- [ ] **Step 2:** Update `LocalInventory` trait in `inventory_broadcaster.rs:21-23`:

```rust
pub trait LocalInventory: Send + Sync {
    fn current_hashes(&self) -> Vec<BlobAddress>;
}
```

- [ ] **Step 3:** Update `StaticInventory::new`, `StoreAdapter::current_hashes` (in `http.rs:2296-2301`), and the mock impls in `inventory_broadcaster.rs::tests` to return `Vec<BlobAddress>`. `StoreAdapter` constructs via:

```rust
fn current_hashes(&self) -> Vec<BlobAddress> {
    self.0
        .list_hashes()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| BlobAddress::new(s).ok())
        .collect()
}
```

(Drops any unexpectedly-shaped on-disk filename rather than panicking; logs at warn level on first drop per session — add a `tracing::warn!` if there's drift, single-shot via `std::sync::Once` to avoid log spam.)

- [ ] **Step 4:** Update receive-arm in `p2p/mod.rs:5092-5160` — `apply_snapshot` and `apply_delta` now take `&[BlobAddress]`; convert to `&[&str]` for the DB layer:

```rust
let hashes_str: Vec<&str> = snapshot.hashes.iter().map(|b| b.as_str()).collect();
crate::db::peer_blob_inventory::apply_snapshot(
    &mut conn,
    &snapshot.peer_id,
    &hashes_str,
    snapshot.sequence as i64,
    &when,
)
```

(Or change the DB layer signature to take `&[BlobAddress]` and do `.as_str()` internally — preferred for one source of truth. Decide at implementation time.)

- [ ] **Step 5:** Update integration test fixtures — `iroh_gossip_parity.rs:36-43` (local wire-shape copy: change `Vec<String>` → `Vec<BlobAddress>` and import from production module; the "local copy because production is libp2p-gated" rationale at line 31-34 no longer applies once Stage-2 is in — drop the copy entirely and import directly), and `iroh_gossip_dual_publish_inventory.rs:67-72, 142-147` (use `BlobAddress::new(sha256_wire('a')).unwrap()`).

- [ ] **Step 6:** Run full crate tests:

```bash
cargo test -p elohim-storage 2>&1 | tail -50
```

Expected: PASS. If failures cascade through inventory consumers, fix each at the type-conversion boundary.

- [ ] **Step 7:** Commit:

```bash
git add elohim/elohim-storage/
git commit -m "feat(storage): thread BlobAddress through inventory producer/consumer types"
```

### Task 0.5: Add producer↔verifier round-trip integration test

- [ ] **Step 1:** Append to `inventory_broadcaster.rs::tests` module:

```rust
#[tokio::test]
async fn snapshot_built_from_real_blobstore_decodes_with_valid_addresses() {
    use crate::blob_store::BlobStore;
    use crate::p2p::inventory_gossip::BlobInventorySnapshot;
    let temp = tempfile::TempDir::new().unwrap();
    let store = BlobStore::new(temp.path()).await.unwrap();
    store.store(b"payload-a").await.unwrap();
    store.store(b"payload-b").await.unwrap();

    let hashes: Vec<crate::p2p::inventory_gossip::BlobAddress> = store
        .list_hashes()
        .unwrap()
        .into_iter()
        .map(|s| crate::p2p::inventory_gossip::BlobAddress::new(s)
            .expect("BlobStore::list_hashes returns canonical wire format"))
        .collect();
    assert_eq!(hashes.len(), 2, "two blobs stored, two hashes listed");

    let inv = StaticInventory::new(hashes);
    let alloc = SequenceAllocator::new(0);
    let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1);

    // Round-trip through MessagePack — the wire path real gossip takes.
    let bytes = snapshot.to_bytes().unwrap();
    let decoded = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.hashes.len(), 2);
    assert_eq!(decoded.verify_structural(), Ok(()),
        "real-BlobStore snapshot must pass structural verify");
}
```

- [ ] **Step 2:** Run; expect PASS:

```bash
cargo test -p elohim-storage --lib p2p::inventory_broadcaster::tests::snapshot_built_from_real_blobstore
```

- [ ] **Step 3:** Run clippy + fmt:

```bash
cargo clippy -p elohim-storage -- -D warnings
cargo fmt --check
```

Expected: clean. Fix any drift.

- [ ] **Step 4:** Commit:

```bash
git add elohim/elohim-storage/src/p2p/inventory_broadcaster.rs
git commit -m "test(storage): producer-to-verifier round-trip from real BlobStore"
```

### Task 0.6: Close-out + memory updates

- [ ] **Step 1:** Append a `## Verifier-fix close-out` section to `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md`. Include: commit SHAs, link to research note, observation on alpha `/api/v1/commitments?action=project-epr` (after orchestrator cycle deploys the fix and Genesis seeder runs cleanly), trajectory frame ("This was Stage-1 of the REA-compute-substrate roadmap; next sprint is Z.D end-to-end").

- [ ] **Step 2:** Update `.claude/memory/feedback_structural_verify_canonical_wire_shape.md`:
    - Description: append "; fixed in `<commit-sha>` via Sprint 0 of REA-compute-substrate roadmap"
    - Add `**Sibling verifiers audited:**` section saying "none — `IdentityBindingGossip::verify_structural` has no hash-shape constraint, only non-empty checks"
    - Add `**Pattern memory:**` link to a new `.claude/memory/project_canonical_wire_shape_newtype_pattern.md` (created in step 3)

- [ ] **Step 3:** Create `.claude/memory/project_canonical_wire_shape_newtype_pattern.md` and add an entry to `.claude/memory/MEMORY.md`:

```markdown
---
name: canonical-wire-shape-newtype-pattern
description: "Wire-format types deserve a constructor-validated newtype, not raw String. Verifier becomes pleonastic; producer↔verifier drift becomes impossible at type level. BlobAddress in elohim-storage is the reference instance."
metadata:
  type: project
---

When a wire format has structural rules (prefix, length, character class — like `sha256-<64-lower-hex>`), wrap it in a newtype that constructor-validates the shape. The serde `try_from = "String"` attribute makes the deserializer enforce the shape on every receive. The newtype then propagates as a type-level guarantee through every downstream consumer.

**Why:** Producer↔verifier drift is a recurring bug class (see [[feedback_structural_verify_canonical_wire_shape]] — inventory verifier had this for ~26 days latent because the test fixture lied about the wire shape). A newtype makes the bug literally unrepresentable: the producer cannot construct an invalid value; the deserializer cannot accept one; the verifier check becomes a no-op (or just non-empty / cross-field rules).

**How to apply:**
1. For every wire-format String field that has shape rules, create a newtype.
2. Implement `TryFrom<String>` and use `#[serde(try_from = "String", into = "String")]`.
3. Implement `Display` (delegates to the inner string).
4. Reference the canonical shape spec (CLAUDE.md, schema doc) in the newtype's doc-comment.
5. If the newtype survives a future architectural graduation (e.g., `BlobAddress` survives the graduation from inventory-gossip Stage-1 to `serve-url-projection` Commitment Stage-N), say so in the doc-comment.

**Reference instance:** `BlobAddress` in `elohim/elohim-storage/src/p2p/inventory_gossip.rs`. Survives Stage-1 inventory-gossip → Stage-4 `serve-url-projection` Commitment graduation per `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`.

**Related:** [[feedback_schema_first_ioc]] (schemas drive Rust + TS via codegen; newtypes are the in-process complement), [[feedback_structural_verify_canonical_wire_shape]] (the bug-class this pattern prevents).
```

- [ ] **Step 4:** Commit close-out + memory updates:

```bash
git add genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md \
        .claude/memory/feedback_structural_verify_canonical_wire_shape.md \
        .claude/memory/project_canonical_wire_shape_newtype_pattern.md \
        .claude/memory/MEMORY.md
git commit -m "docs(memory): close-out inventory verifier fix; capture newtype pattern"
```

### Task 0.7: Push to dev + watch orchestrator cycle

- [ ] **Step 1:** Push (pre-push gate runs full quality stage; ~30-90 min):

```bash
git push origin dev
```

- [ ] **Step 2:** Watch orchestrator + Edge pipeline via ci-observer agent. Once storage image redeploys to alpha, the inventory gossip should start propagating. Re-probe:

```bash
# After Edge pipeline green:
curl -s https://alpha.elohim.host/api/v1/commitments?action=project-epr | jq 'length'
```

Expected: non-zero once Genesis seeder has run cleanly. (Genesis seeder cleanup is a separate blocker per the substrate-rea sprint close-out — not in Sprint 0 scope.)

- [ ] **Step 3:** Watch `kubectl logs -n elohim deploy/storage-alpha-*` (operator-run) for `Inventory snapshot applied` debug lines — confirms the verifier-pass path is now exercised.

- [ ] **Step 4:** Mark Sprint 0 complete in this roadmap (change `S0 ▣` → `S0 ✓`) and proceed to Sprint 1.

---

# Sprint 1 — Z.D substrate-correct deploy end-to-end (proving ground)

**Goal:** Ship the first concrete instance of the REA compute-commitment primitive: replace the Z.1 anti-pattern (`PATCH /db/content/{slug}` from CI) with a substrate-correct flow where `stageSpaBlob` authors an `EprHead` envelope signed by a per-operator deploy-service-agent, emits a `republish-epr` `EconomicEvent` bounded by an operator-authored `delegates-compute` Commitment, and PUTs to `/api/v1/epr/{cid}` where the substrate validates bounds before persisting. Proves the primitive end-to-end on one row of the gospel table.

**Architecture:** Per Z.D spec §2. Provisioning script issues per-operator deploy-svc-agent Ed25519 keys; operator-steward authors the `delegates-compute` Commitment naming the deploy-svc-agent as recipient with bounds (epr_scope, reach_ceiling=commons, rate_per_hour, rotation_ttl); CI loads the secret; `stageSpaBlob` signs envelope, emits event, PUTs to `/api/v1/epr/{cid}`; substrate validator walks `bounded_by`, checks bounds, accepts. The old PATCH path is deleted (not deprecated) — substrate-correct or nothing.

**Tech stack:** Rust (Mishpat zome coordinator + integrity validator, elohim-storage `/api/v1/epr/{cid}` handler, `bounds_validator.rs` from Sprint 2), TypeScript (deploy-svc-agent provisioning, `stageSpaBlob` rewrite), Ed25519 (`ed25519-dalek`), Jenkins (CI secret loading).

**Spec:** `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md`. Read §2 fully before authoring the Sprint 1 detailed plan.

**Files to create:**
- `genesis/orchestrator/scripts/provision-deploy-agent.ts` — Ed25519 keypair generation, Jenkins credential stash, agent CID derivation
- `genesis/orchestrator/scripts/author-deploy-commitment.ts` — operator-steward UI flow to author the `delegates-compute` Commitment naming the deploy-svc-agent (interactive; one per operator per scope)
- `elohim/elohim-storage/src/services/republish_epr_validator.rs` — instance of bounds validator for `republish-epr` events (delegates to Sprint 2's substrate-side validator)
- `app/elohim-app/scripts/stage-spa-blob-zd.ts` — Z.D shape (replaces existing `stageSpaBlob`)

**Files to modify:**
- `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/lib.rs` — wire `delegates-compute` action discriminator + accept/reject in coordinator
- `elohim/holochain/dna/mishpat/zomes/integrity/commitments/src/lib.rs` — schema-validate Commitment payload per `delegates-compute.schema.json` (Source of truth: Holochain DHT — existing `Mishpat::Commitment` entry type; no new entry type)
- `elohim/elohim-storage/src/api/epr.rs` — `PUT /api/v1/epr/{cid}` handler invokes `republish_epr_validator` on incoming event
- `Jenkinsfile` (root) — load deploy-svc-agent secret per operator; pass to `stageSpaBlob`
- Delete: `app/elohim-app/scripts/stage-spa-blob.sh` (Z.1 anti-pattern, `PATCH /db/content/{slug}`)

**Files to read for context:**
- Z.D spec §2 (full)
- `elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json` (Source of truth: Holochain DHT — `Mishpat::Commitment` entry type with action discriminator `delegates-compute`; existing type, no new DNA capacity consumed)
- `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json` (Source of truth: Holochain DHT — elohim DNA `EconomicEvent` entry type with action discriminator `republish-epr`; existing type)
- `elohim/sdk/schemas/v1/feedback-signals/rate-limit-exceeded.schema.json` (Source of truth: Holochain DHT — elohim DNA `FeedbackSignal` entry type with `signal_kind="rate-limit-exceeded"`; existing type via signal_kind extension pattern)
- Existing `EprHead` envelope construction in `elohim/elohim-storage/src/api/epr.rs`

**Agent team:**
- rust-architect: zome wiring + bounds-validator instance + handler integration
- code-reviewer: pre-PR review of validator + handler
- red-team: adversarial probes (forge event with mismatched `bounded_by`, replay attack, scope escalation attempt, rate-limit bypass attempt)
- ci-observer: monitor Jenkins App + Edge pipelines during deploy

**Blocked by:** Sprint 0 (verifier fix unblocks alpha for testing) + Sprint 2 bounds validator (the validator instance for `republish-epr` delegates to it). Sprint 2 can be authored in parallel with Sprint 1 implementation if the team picks up both at once.

**Blocks:** Sprint 3 (hosting graduation copies this pattern), Sprints 5a–e (other rows of the table copy this pattern).

**Done when:**
- A real CI run completes the full Z.D flow on alpha: deploy-svc-agent signed envelope, event emitted, validator accepted, `EprHead` persisted to DHT, `KadStartProviding` announced, `alpha.elohim.host` serves the new SPA bundle within propagation window.
- Adversarial test cases from red-team all fail validation as designed (forge attempts, replay, scope escalation, rate-limit bypass).
- The Z.1 anti-pattern (`PATCH /db/content/{slug}` from CI) is deleted from the codebase. No fallback.
- A2O scenario captures the substrate-correct deploy flow as a regression scenario.

**Hand-off to next sprint:** Sprint 1's `republish_epr_validator.rs` is the reference template for Sprint 3's `serve_url_projection_validator.rs` and Sprints 5a–e's per-instance validators. The pattern is: thin per-instance validator that schemas-checks the event payload + delegates the substrate-wide concerns (active-Commitment, scope-includes-event, reach-ceiling, rate, key-rotation) to Sprint 2's `bounds_validator::validate`.

### Sprint 1 plan-authoring

When picked up, run:

```
/superpowers:writing-plans Sprint 1 of REA-compute-substrate roadmap: Z.D substrate-correct deploy end-to-end. Read genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md §2 and decompose the seven implementation rows of the spec's "What needs to be built" table (lines ~195-220) into bite-sized TDD tasks. Reference Sprint 0's BlobAddress newtype pattern for shape-validated wire types; reference Sprint 2's bounds_validator API surface (author both in parallel).
```

---

# Sprint 2 — Single bounds validator + standing aggregation primitives

**Goal:** Author two substrate-wide primitives that every Commitment instance depends on: (1) a single `bounds_validator::validate(event, commitment) -> Result<(), BoundsViolation>` function that walks `bounded_by` → fetches Commitment → checks {active, scope-includes-event, reach-ceiling-respected, rate-not-exceeded, key-rotation-current, revoked?}, and (2) a `standing_aggregator::standing_for(agent_cid) -> StandingScore` function that aggregates `signal_kind` FeedbackSignals (rate-limit-exceeded, bad-custody, reach-escalation-pending) into a per-agent score that future Commitments can check.

**Architecture:** Both primitives live in `elohim-storage` (read-path projections of DHT truth). Bounds validator is pure (no I/O beyond the conductor Commitment fetch + event-history rate-count). Standing aggregator is a SQLite projection — `signal_kind` FeedbackSignals already accumulate via the existing projection pipeline; the aggregator queries the `feedback_signals` table by agent_cid and computes a windowed score. Each function exposes one HTTP route for diagnostic visibility (`POST /api/v1/diagnostics/validate-bounds`, `GET /api/v1/standing/{agent_cid}`).

**Tech stack:** Rust (elohim-storage services + handlers), Diesel (standing projection queries), schema-driven validation (use `schemars` crate against the `delegates-compute.schema.json` if not already wired).

**Files to create:**
- `elohim/elohim-storage/src/services/bounds_validator.rs`
- `elohim/elohim-storage/src/services/standing_aggregator.rs`
- `elohim/elohim-storage/src/api/diagnostics_bounds.rs` (route handler)
- `elohim/elohim-storage/src/api/standing.rs` (route handler)
- `elohim/elohim-storage/tests/bounds_validator_integration.rs` (adversarial test cases live here)
- `elohim/elohim-storage/tests/standing_aggregator_integration.rs`

**Files to modify:**
- `elohim/elohim-storage/src/services/mod.rs` — register new services
- `elohim/elohim-storage/src/http.rs` — mount new routes
- `elohim/elohim-storage/src/db/feedback_signals.rs` (or equivalent) — ensure schema includes `signal_kind`, `agent_cid`, `emitted_at`, `weight` columns; add `signals_for_agent` query (Source of truth: local operational projection of Holochain DHT `FeedbackSignal` entries; rebuildable from DHT replay via existing projection pipeline — see anchor `dht_anchor_hash` column already present)

**Files to read for context:**
- `elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json` (bounds shape; Source of truth: Holochain DHT — existing `Mishpat::Commitment` entry type)
- `elohim/sdk/schemas/v1/feedback-signals/*.schema.json` (3 signal types currently defined; Source of truth: Holochain DHT — existing `FeedbackSignal` entry type extended via `signal_kind` discriminator per `project_signal_kind_extensible_protocol_class`)
- `genesis/docs/architecture/rea-compute-commitment-primitive.md` (auditability properties — validator must satisfy)
- `.claude/memory/project_signal_kind_extensible_protocol_class.md` (extension pattern — validator must be schema-aware, not enum-locked; no new DHT entry types created across this roadmap)

**API design (decide at impl time):**

```rust
// bounds_validator.rs
pub struct BoundsViolation {
    pub kind: ViolationKind,  // CommitmentInactive, ScopeNotIncluded, ReachCeilingExceeded, RateLimitExceeded, KeyRotationStale, CommitmentRevoked
    pub commitment_cid: String,
    pub event_summary: String,
}

pub async fn validate(
    event: &EconomicEventView,
    commitment_fetcher: &dyn CommitmentFetcher,  // trait so tests can mock
    rate_history: &dyn RateHistory,              // ditto
) -> Result<(), BoundsViolation>;

// standing_aggregator.rs
pub struct StandingScore {
    pub agent_cid: String,
    pub score: f64,           // [0.0, 1.0]; 1.0 = pristine
    pub recent_breaches: Vec<FeedbackSignalSummary>,  // last N
    pub computed_at: String,
}

pub fn standing_for(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    window_days: u32,
) -> Result<StandingScore, StorageError>;
```

**Agent team:**
- rust-architect: both primitives + handlers
- red-team: adversarial test suite (forge fetcher returns active Commitment for revoked CID, race between revocation and event, rate-window edge cases, signal-injection from non-witnessed peer)
- pattern-hunter: cross-codebase pass for existing X-API-Key / `if admin:` / ad-hoc auth checks that should absorb into Commitment + bounds_validator. Output is a backlog for Sprints 5a–e to consume — "these are the surfaces to absorb when you instantiate your row of the table"

**Blocked by:** Sprint 0 (cluster stability for integration tests).

**Blocks:** Sprint 1's `republish_epr_validator` (instance), Sprint 3's `serve_url_projection_validator`, Sprints 5a–e per-instance validators, Sprint 6 matchmaking gate (consumes standing score).

**Done when:**
- `bounds_validator::validate` accepts a valid event-Commitment pair and rejects each of the 6 ViolationKind cases with red-team-authored test fixtures.
- `standing_aggregator::standing_for` returns expected scores for a fixture peer with N FeedbackSignals over a window.
- Both diagnostic HTTP routes return well-shaped JSON per a schema in `elohim/sdk/schemas/v1/views/`.
- pattern-hunter has produced `genesis/docs/research/2026-MM-DD-auth-surfaces-to-absorb.md` (the absorption backlog).

### Sprint 2 plan-authoring

When picked up, run:

```
/superpowers:writing-plans Sprint 2 of REA-compute-substrate roadmap: single bounds validator + standing aggregation primitives. Author bite-sized TDD tasks for elohim/elohim-storage/src/services/bounds_validator.rs and standing_aggregator.rs. Red-team test fixtures cover the 6 ViolationKind cases; standing aggregator covers the 3 currently-defined FeedbackSignal types per elohim/sdk/schemas/v1/feedback-signals/. Pattern-hunter pass produces the absorption backlog as a separate artifact.
```

---

# Phase B: Hosting graduation

## Sprint 3 — serve-url-projection schema + Commitment authoring

**Goal:** Author the `serve-url-projection` instance schemas (Commitment + EconomicEvent + FeedbackSignals if any new ones), wire the Mishpat zome to accept the action discriminator, and produce a CLI/Angular-app flow for a doorway operator to author a `delegates-compute` Commitment naming their doorway-svc-agent as recipient with bounds (doorway capacity, reach gates, URL-prefix scope).

**Files to create:**
- `elohim/sdk/schemas/v1/commitments/serve-url-projection.schema.json` (mirror `delegates-compute.schema.json` shape; scope is the action class)

  Wait — re-read: `delegates-compute` IS the Commitment action; `serve-url-projection` is the EconomicEvent action. So:

- `elohim/sdk/schemas/v1/economic-events/serve-url-projection.schema.json` (mirror `republish-epr.schema.json`; required `bounded_by` field; payload schema for doorway projection events)
- `elohim/elohim-storage/src/services/serve_url_projection_validator.rs` (mirror Sprint 1's `republish_epr_validator`; delegates to `bounds_validator`)
- `app/elohim-app/src/app/doorway/services/commitment-authoring.service.ts` (Angular service to walk operator through Commitment authoring)

**Files to modify:**
- `elohim/sdk/domains/doorway/manifest.json` (declare `serve-url-projection` event_kind under existing economic_events declaration)
- `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/lib.rs` (already wired in Sprint 1 for `delegates-compute` action; this sprint adds the recipient/scope validation for `serve-url-projection` scope value)
- `elohim/elohim-storage/src/api/epr.rs` or new `serve_url_projection.rs` handler

**Agent team:**
- rust-architect (schemas, validator, zome)
- angular-architect (operator UI)
- content-pipeline (manifest declaration)
- code-reviewer

**Blocked by:** Sprint 1 (pattern), Sprint 2 (bounds_validator).

**Blocks:** Sprint 4.

**Done when:** A doorway operator can author a `delegates-compute` Commitment with `scope="serve-url-projection"` via the Angular flow; the Commitment lands on DHT; CLI smoke-test confirms the doorway-svc-agent can emit a `serve-url-projection` event referencing the Commitment and have it accepted by the substrate.

## Sprint 4 — Inventory gossip carries `bounded_by` (Stage-3 of inventory trajectory)

**Goal:** Extend `BlobInventorySnapshot` and `BlobInventoryDelta` with a `bounded_by: CommitmentCid` field (required). Receiver-side `bounds_validator` walks the back-ref; rejected snapshots emit a `bad-custody` FeedbackSignal naming the broadcaster.

**Files to modify:**
- `elohim/elohim-storage/src/p2p/inventory_gossip.rs` — add `bounded_by: CommitmentCid` (new newtype, same pattern as `BlobAddress`); update `verify_structural`
- `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs` — `build_snapshot`/`build_delta` take the broadcaster's active hosting Commitment CID; if no active Commitment, suppress publish + emit `tracing::warn!`
- `elohim/elohim-storage/src/p2p/mod.rs:5077-5160` — receive arm calls `bounds_validator::validate` against an `serve-url-projection`-shaped event before `apply_snapshot`; on bounds violation, emit `bad-custody` signal
- `elohim/elohim-storage/src/http.rs` — `StoreAdapter` looks up active Commitment via the conductor

**Agent team:** rust-architect, red-team (broadcast with revoked Commitment, broadcast with mismatched scope, broadcast without Commitment).

**Blocked by:** Sprint 3.

**Blocks:** Phase C parallelization can start once this is in flight (the inventory→Commitment graduation proves the bidirectional flow).

**Done when:**
- Every cluster peer's snapshots carry `bounded_by`.
- Receivers reject snapshots without a valid active Commitment.
- A revoked Commitment's snapshots stop being applied within one snapshot cycle of revocation.
- Per-peer `peer_blob_inventory` row count matches the peer's Commitment scope (no "advertising hosting I'm not committed to").

---

# Phase C: Pattern replication (parallel, agent-team-fanned)

Each sub-sprint copies the Sprint 1 / Sprint 3 / Sprint 4 pattern for a different row of the gospel-tier table. All five can run in parallel by different agent teams (or by sequential agentic-developer shifts). All five share:
- New EconomicEvent schema in `elohim/sdk/schemas/v1/economic-events/` (Source of truth: Holochain DHT — existing elohim DNA `EconomicEvent` entry type; each new schema is an action-discriminator extension, NOT a new entry type; DNA capacity unchanged)
- New per-instance validator in `elohim/elohim-storage/src/services/` (Source of truth: code, delegates to Sprint 2's `bounds_validator` for substrate-wide concerns)
- Manifest declaration in the relevant pillar's manifest (Source of truth: pillar manifest, drives runtime registration of the action discriminator per `project_doorway_manifest_driven_routes`)
- Per-instance Commitment authoring flow (CLI or UI per the row's primary surface)

## Sprint 5a — Recovery delegation (`attest-recovery`)

**Provider:** steward (pre-incident). **Recipient:** recovery quorum (graduated trust circles per `project_graduated_recovery_authority`). **Bounds:** reach ceiling, quorum threshold, time window.

**Files:**
- `elohim/sdk/schemas/v1/economic-events/attest-recovery.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`attest-recovery`; no new entry type)
- `elohim/elohim-storage/src/services/attest_recovery_validator.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- `app/elohim-app/src/app/imagodei/services/recovery-commitment-authoring.service.ts` (graduated trust-circle UI per `project_recovery_grandma_standard`)

**Agent team:** rust-architect + angular-architect + red-team (quorum spoofing).

**Cross-cutting:** This row is the highest-stakes — graduated recovery authority is what saves households from absolute lockout. Pair with `project_recovery_grandma_standard` and `project_elohim_as_counsel`.

## Sprint 5b — Qahal moderation (`moderation-action`)

**Provider:** qahal collective. **Recipient:** moderator-agent. **Bounds:** qahal scope, action types, target reach class.

**Files:**
- `elohim/sdk/schemas/v1/economic-events/moderation-action.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`moderation-action`; no new entry type)
- `elohim/elohim-storage/src/services/moderation_action_validator.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- `app/elohim-app/src/app/qahal/services/moderation-commitment-authoring.service.ts`
- `elohim/sdk/domains/qahal/manifest.json` (declare event; manifest is SoT for action-discriminator registration)

**Agent team:** rust-architect + angular-architect + red-team (out-of-scope moderation attempt).

**Cross-cutting:** Pair with `project_qahal_graduated_capability_surface` and `project_commons_elohim_co_steward`.

## Sprint 5c — Household chore (`chore-done`)

**Provider:** household member. **Recipient:** another member. **Bounds:** scope (kitchen/yard), period (week), chore type.

**Files:**
- `elohim/sdk/schemas/v1/economic-events/chore-done.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`chore-done`; no new entry type)
- `elohim/elohim-storage/src/services/chore_done_validator.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- `app/elohim-app/src/app/shefa/services/household-chore-commitment.service.ts`
- `elohim/sdk/domains/shefa/manifest.json` (or wherever household chores live; pillar-place needs decision; manifest is SoT for action-discriminator registration)

**Agent team:** rust-architect + angular-architect.

**Cross-cutting:** This is the value-scanner integration point — chore-done events feed REA visibility into the care economy per `project_household_living_core_lived_contrast_diffusion`. Pair with the household value-scanner work.

## Sprint 5d — Content authorship delegation (`publish-revision`)

**Provider:** original author. **Recipient:** co-steward. **Bounds:** content CID lineage, branch policy, scope.

**Files:**
- `elohim/sdk/schemas/v1/economic-events/publish-revision.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`publish-revision`; no new entry type)
- `elohim/elohim-storage/src/services/publish_revision_validator.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- `app/elohim-app/src/app/lamad/services/co-steward-commitment.service.ts`
- `elohim/sdk/domains/lamad/manifest.json` (manifest is SoT for action-discriminator registration)

**Agent team:** rust-architect + angular-architect + red-team (off-lineage publish attempt, branch-policy violation).

## Sprint 5e — DePIN compute lending (`provide-cycles`)

**Provider:** node operator. **Recipient:** requesting peer. **Bounds:** watts, wall-time, task class.

**Files:**
- `elohim/sdk/schemas/v1/economic-events/provide-cycles.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`provide-cycles`; no new entry type)
- `elohim/sdk/schemas/v1/feedback-signals/compute-breach.schema.json` (NEW signal_kind, per `project_compute_commitments_bounded`; Source of truth: Holochain DHT — existing elohim `FeedbackSignal` entry type extended via `signal_kind="compute-breach"`; no new entry type, only a new discriminator value per `project_signal_kind_extensible_protocol_class`)
- `elohim/elohim-storage/src/services/provide_cycles_validator.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- CLI flow (no UI yet — DePIN compute is operator-facing) in `genesis/orchestrator/scripts/author-compute-commitment.ts`
- `elohim/sdk/domains/shefa/manifest.json` or new `depin/manifest.json` (manifest is SoT for action-discriminator + signal_kind registration)

**Agent team:** rust-architect + red-team (over-commitment attack, breach attribution to wrong agent).

**Cross-cutting:** Pair with `project_compute_commitments_bounded` (three trigger_kinds: request-driven, standing, subscription — DePIN compute spans all three).

---

# Phase D: Coordination surface

## Sprint 6 — Standing-aware matchmaking gate

**Goal:** Implement the matchmaking gate per `project_reach_gate_is_elohim_mediated_matchmaking`. Given an agent attempting to enter a new Commitment as recipient, the gate consults `standing_aggregator::standing_for(agent_cid)` and returns `{Allowed, Blocked, Pending}`. `Pending` triggers elohim-mediated sponsorship (the elohim-agent advocates for the recipient based on context).

**Files to create:**
- `elohim/elohim-storage/src/services/matchmaking_gate.rs` (Source of truth: code — pure function; no persisted entity)
- `elohim/elohim-storage/src/api/matchmaking.rs` (route handler: `POST /api/v1/matchmaking/check` — diagnostic/coordination only; does NOT create a new DHT entry type; reads from `feedback_signals` SoT=Holochain `FeedbackSignal` via S2 standing aggregator; route exists to surface the gate decision, not to introduce a new persisted entity)
- `matchmaking_decision_log` operational table (Source of truth: local SQLite operational projection; rebuildable from `feedback_signals` replay + manifest-declared weights at decision_time; per Design Constraint #4 in the P2P Design Gate output)
- `app/elohim-app/src/app/elohim/services/matchmaking-coordinator.service.ts` (Angular surface for Pending→elohim flow)

**Files to modify:**
- Every Commitment-authoring service from Sprints 3 + 5a–e (gate is invoked before Commitment authoring)

**Agent team:** rust-architect + angular-architect + red-team.

**Blocked by:** Sprint 2 (standing aggregator) + at least 2 of Phase C (need ≥2 instance streams for standing to be meaningful).

**Blocks:** Sprint 7.

**Done when:** A new Commitment with a low-standing recipient is gated to `Pending`; an elohim-agent sponsorship flow successfully escalates to `Allowed`; a chronically-bad-standing agent is gated to `Blocked`.

## Sprint 7 — Elohim-mediated dispute resolution

**Goal:** When two Commitments collide on capacity (e.g., a DePIN compute lender is over-committed across multiple `provide-cycles` Commitments), elohim mediates via the subagent-specialist pattern (`project_elohim_subagent_specialists`): defender / advocate / steward / gate-discerner. The mediation emits a structured outcome event (`mediation-outcome`).

**Files to create:**
- `elohim/sdk/schemas/v1/economic-events/mediation-outcome.schema.json` (Source of truth: Holochain DHT — existing elohim `EconomicEvent` entry type, action=`mediation-outcome`; no new entry type; payload back-refs colliding Commitment CIDs per Design Constraint #3)
- `elohim/elohim-storage/src/services/dispute_mediation.rs` (Source of truth: code; delegates to S2 `bounds_validator`)
- `app/elohim-app/src/app/elohim/services/dispute-coordinator.service.ts`

**Agent team:** rust-architect + angular-architect + red-team (mediation-spoofing, capacity-double-spend).

**Blocked by:** Sprint 6.

**Blocks:** Sprint 8.

**Done when:** A capacity-collision fixture produces a mediation outcome; the outcome is verifiable by walking the back-ref from any of the colliding Commitments.

---

# Phase E: Operational surface

## Sprint 8 — Revocation propagation + audit-trail UI

**Goal:** Angular surface ("Commitment dashboard") that shows: active Commitments where I'm provider/recipient, FeedbackSignals against me, standing score over time, revocation controls (one-click revoke with confirmation), audit log (every event back-referencing my Commitments). Library A primitives + Library B designed bindings per `app/elohim-library/CLAUDE.md`.

**Files to create:**
- `app/elohim-elements/src/elohim-commitment-card/elohim-commitment-card.ts` (Lit primitive, Library A)
- `app/elohim-elements/src/elohim-standing-meter/elohim-standing-meter.ts`
- `app/elohim-elements/src/elohim-feedback-signal-badge/elohim-feedback-signal-badge.ts`
- `app/elohim-library/projects/graphos/src/lib/commitment/` (Library B designed compositions)
- `app/elohim-app/src/app/elohim/components/commitment-dashboard/` (Angular composition)
- `genesis/a2o/features/elohim/commitment-dashboard.feature` (a2o scenarios)

**Files to modify:**
- `elohim/sdk/schemas/v1/views/commitment-summary-view.schema.json` — Source of truth: local (operational view-projection only); the View is composed at request time from upstream entities whose truth lives on Holochain DHT — `commitments` (Holochain `Mishpat::Commitment`), `economic_events` (Holochain `EconomicEvent`), `feedback_signals` (Holochain `FeedbackSignal`) — plus the computed `StandingScore`. No new persisted entity; no new DHT entry type; per the views.rs ts-rs export convention.
- `elohim/elohim-storage/src/views.rs` (CommitmentSummaryView struct + From impl)
- `elohim/elohim-storage/src/api/commitments.rs` (handler `GET /api/v1/commitments/{cid}/summary` — read route; no new entry type)

**Agent team:** component-architect (Library A primitives) + graphos-designer (Library B bindings) + angular-architect (Angular composition) + rust-architect (view + handler) + content-pipeline (a2o scenarios).

**Blocked by:** Sprint 7.

**Blocks:** nothing (terminal sprint).

**Done when:**
- A user can see their active Commitments, recent FeedbackSignals, standing score.
- One-click revoke produces a revocation event that propagates within one DHT cycle; downstream events fail validation immediately.
- a2o scenarios pass covering the golden path (revoke → downstream event rejected) and edge cases (revoke while event in flight → which side wins).

---

# P2P Design Gate output

Per `.claude/skills/p2p-design-gate/SKILL.md` and CLAUDE.md's MANDATORY directive ("Before proposing design approaches for ANY feature involving data entities..."), every new entity declared by this roadmap is classified below.

**Headline finding:** This roadmap creates **zero new DHT entry types**. Every notarized payload reuses an existing entry type via action discriminator (Mishpat `Commitment` + new actions like `delegates-compute`; elohim DNA `EconomicEvent` + new actions like `republish-epr`, `serve-url-projection`, etc.) or `signal_kind` vocabulary extension (existing `FeedbackSignal` entry type + new signal_kinds like `compute-breach`). DNA capacity stays at: Mishpat 11/~100, elohim DNA EconomicEvent/FeedbackSignal unchanged. This honors `project_signal_kind_extensible_protocol_class`.

## Entity classification table

| Entity | Sprint | Classification | DHT entry type | Address | Source of truth | Coordinator zome::fn | Storage projection | HTTP route |
|--------|--------|----------------|----------------|---------|-----------------|----------------------|--------------------|-----------:|
| `BlobAddress` newtype | S0 | C (operational — wire-format type) | none — process-only | n/a (wraps `sha256-<hex>` string) | BlobStore filesystem CAS | none | n/a (in-process) | n/a |
| `BlobInventorySnapshot` / `BlobInventoryDelta` (S0 shape) | S0 | C (operational — libp2p gossip projection) | none — gossip only | sender PeerId + sequence | per-peer aggregated from broadcasters | none | `peer_blob_inventory` (no dht_anchor_hash; rebuilt from gossip stream) | `GET /api/v1/diagnostics/inventory-parity` (existing) |
| `delegates-compute` Commitment payload | S1, all of B+C | **A (notarized — existing entry type)** | EXISTING `Mishpat::Commitment` (no new type; action discriminator) | Holochain ActionHash | Holochain DHT | `mishpat::coordinator::commitments::create_commitment` | `commitments` (dht_anchor_hash: yes) | `POST /api/v1/commitments` (existing) |
| `republish-epr` EconomicEvent payload | S1 | **A (notarized — existing entry type)** | EXISTING elohim DNA `EconomicEvent` (no new type; action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes; existing) | `PUT /api/v1/epr/{cid}` (existing, validator-augmented) |
| `rate-limit-exceeded` / `bad-custody` / `reach-escalation-pending` FeedbackSignals | S1 (already shipped schemas) | **A (notarized — existing entry type via signal_kind extension)** | EXISTING `FeedbackSignal` (no new type; signal_kind discriminator per `project_signal_kind_extensible_protocol_class`) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::feedback::emit_signal` | `feedback_signals` (dht_anchor_hash: yes) | `GET /api/v1/feedback-signals?signal_kind=…` |
| `bounds_validator::validate` | S2 | n/a — pure function, not an entity | n/a | n/a | code | n/a | n/a | `POST /api/v1/diagnostics/validate-bounds` (diagnostic; no persistence) |
| `StandingScore` (return type) | S2 | C (operational — computed projection) | none | composite `(agent_cid, window)` at computation time | computed on demand from `feedback_signals` (A-source) | n/a | none — computed, not persisted | `GET /api/v1/standing/{agent_cid}` |
| `serve-url-projection` EconomicEvent payload | S3 | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes) | `POST /api/v1/projections/serve-url` (new) |
| Inventory snapshot `bounded_by` field (S4) | S4 | C (gossip message field — references A entity) | n/a (field; refers to Commitment ActionHash) | inherits parent message | inherits parent message | none | `peer_blob_inventory.bounded_by` column added (FK to `commitments.dht_anchor_hash` semantically; no DB-level constraint) | inherits inventory route |
| `attest-recovery` EconomicEvent payload | S5a | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); recovery-quorum projection in `recovery_attestations` (operational fanout for quorum-count queries) | `POST /api/v1/recovery/attest` |
| `moderation-action` EconomicEvent payload | S5b | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); qahal-scoped view in existing qahal projection tables | `POST /api/v1/qahal/{cid}/moderation` |
| `chore-done` EconomicEvent payload | S5c | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); household care-economy view via shefa value-scanner integration | `POST /api/v1/household/{cid}/chore-done` |
| `publish-revision` EconomicEvent payload | S5d | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); content lineage edges in existing `content_couplings` | `POST /api/v1/content/{cid}/revisions` |
| `provide-cycles` EconomicEvent payload | S5e | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); DePIN capacity ledger in operational `compute_allocations` table (rebuilt from events) | `POST /api/v1/depin/cycles` |
| `compute-breach` FeedbackSignal payload | S5e | **A (notarized — existing entry type via signal_kind extension)** | EXISTING `FeedbackSignal` (no new type; signal_kind=`"compute-breach"` per `project_compute_commitments_bounded`) | Holochain ActionHash | Holochain DHT | `elohim::coordinator::feedback::emit_signal` | `feedback_signals` (dht_anchor_hash: yes; existing table; no schema change — discriminator column already present) | inherits feedback-signals route |
| `matchmaking_gate::check` | S6 | n/a — pure function | n/a | n/a | code | n/a | n/a | `POST /api/v1/matchmaking/check` (diagnostic/coordination; result `{Allowed,Blocked,Pending}` not persisted) |
| `mediation-outcome` EconomicEvent payload | S7 | **A (notarized — existing entry type)** | EXISTING `EconomicEvent` (action discriminator) | Holochain ActionHash; back-refs colliding Commitments | Holochain DHT | `elohim::coordinator::events::create_event` | `economic_events` (dht_anchor_hash: yes); mediation index in operational `mediation_outcomes` (FK to colliding Commitment CIDs) | `POST /api/v1/disputes/{cid}/mediate` |
| `CommitmentSummaryView` wire shape | S8 | C (operational — view projection, NOT a persisted entity) | n/a (View type in `views.rs`; ts-rs export) | n/a | composed at request time from `commitments` (A), `economic_events` (A), `feedback_signals` (A), `StandingScore` (C) | n/a — read projection only | n/a (no new table; query-side composition) | `GET /api/v1/commitments/{cid}/summary` |

## Address strategy summary

- **All EconomicEvent payloads** use the existing event-entry pattern: ActionHash from Holochain; `bounded_by: <CommitmentCid>` field anchors to the providing Commitment; `target: <outcome CID>` where applicable. Content-derived where the payload is immutable (event = historical fact); the entry's CID covers the canonical payload bytes per `republish-epr.schema.json` precedent.
- **All Commitment payloads** are agent-scoped composite by signing key + scope; Holochain ActionHash is the identity. Slug/UUID strategy explicitly NOT used — Commitments are signed by provider; identity flows from signature, not nickname.
- **`BlobAddress`** (S0) is the one exception that uses what looks like a slug (`sha256-<hex>`) — but it's content-derived from sha256 of the blob; the prefix is namespace not slug. Justified.
- **Wire-shape Views** (`StandingScore`, `CommitmentSummaryView`) have no identity — they're computed projections, not entities.

## Anti-pattern check (per gate skill catalog)

| Anti-pattern | Status across this roadmap |
|--------------|----------------------------|
| UUID primary key for notarized entity | ❌ NONE — all notarized entities use Holochain ActionHash as `dht_anchor_hash` |
| REST route as design starting point | ❌ NONE — Phase A starts with `delegates-compute` schema + Mishpat coordinator, then validator, THEN HTTP route. Sprints 5a–e copy this order |
| CID stored as relational FK | ❌ NONE — `bounded_by` is a back-reference field on events, not a FK; resolved through walking the DHT, not joining a SQL table |
| Standalone table for agent state | ❌ NONE — no new agent-scoped tables. Standing is computed, not stored |
| Three address formats undefined | ❌ NONE — every entity above has one canonical address declared; `BlobAddress` newtype encodes the one canonical wire form for sha256 hashes |
| Missing source-of-truth declaration | ✅ FIXED HERE — this section is the declaration. Every storage projection migration in Phases A–E carries a header comment `-- Source of truth: DHT (entry type X, action Y)` or `-- Source of truth: local (operational; rebuildable from <source>)` |
| New entry type when one exists | ❌ NONE — zero new entry types created across the entire roadmap. Mishpat capacity unchanged at 11/~100; elohim DNA capacity unchanged |
| Granular data on DHT | ❌ NONE — `StandingScore` is computed on demand; gossip messages (S0, S4 inventory) stay libp2p; per-event payloads are bounded fact-records (who/what/when/bounded_by), not granular telemetry |

## Design constraints discovered during the gate

1. **`bounded_by` is a back-reference, not a foreign key.** Every per-instance validator (Sprints 1, 3, 5a–e) walks `event.bounded_by` → fetches Commitment via Holochain `get`, not via SQL join. The storage projection's `bounded_by` column (added to `economic_events` and similar tables in S2) is a denormalization for fast query — the Holochain side is the truth, the column is the cache. **Validator code never trusts the column for authorization.**

2. **`signal_kind` extensibility extends to compute-breach.** S5e adds `compute-breach` as a new signal_kind value; the schema lives under `elohim/sdk/schemas/v1/feedback-signals/compute-breach.schema.json`, the entry type is unchanged. This pattern (per `project_signal_kind_extensible_protocol_class`) is the right answer for all future low-trust signals. Sprints S5a–e and S7 should resist the temptation to introduce per-instance signal entry types.

3. **`mediation-outcome` (S7) back-references two or more Commitments.** Schema must allow `bounded_by` to be either a single CID OR an ordered list `bounded_by_collision: [cid_a, cid_b, ...]`. The walking validator handles both. This shape decision needs to be locked in S2 (validator) so S7's schema authoring matches; capture as a `bounds_validator::ValidationInput` enum variant.

4. **Standing aggregator is operational, not notarized.** Score is computed; never persisted as a Commitment-like entity. Anyone could re-derive a different score with different weights — that's the point. Matchmaking gate (S6) MUST cite its weights at the time of the gate-check decision so the decision is auditable even though the score is ephemeral. Capture this as a `matchmaking_decision_log` operational table (Source of truth: local, rebuildable from `feedback_signals` + manifest-declared weights at decision_time).

5. **Recovery quorum (S5a) needs a quorum-count projection that's NOT a new DHT entry.** Counting "how many attestations exist for this recovery event" is a SQL query over `economic_events` filtered by action=`attest-recovery` + target. Add `recovery_attestations` as an operational projection (Source of truth: local, rebuildable from events). The quorum-threshold check in `bounds_validator` reads this projection. The Commitment carries `quorum_threshold` as a bound; the validator counts and compares.

6. **Hosting Commitments (S3) interact with libp2p PeerId, not Holochain AgentPubKey directly.** A doorway-svc-agent's libp2p identity ≠ its Holochain agent CID. The Commitment's recipient field is the agent CID (Holochain); the inventory broadcaster's signature is the libp2p key. The `agent_peer_binding` table (existing — established by `IdentityBindingGossip`) is the join. S4 receiver-side validation must resolve broadcaster PeerId → bound AgentCid → fetch their Commitment → validate. Capture in S4 plan-authoring.

7. **No new manifests; every event/signal goes into an existing pillar manifest.** Sprints S3 (doorway), S5a (imagodei), S5b (qahal), S5c (shefa), S5d (lamad), S5e (shefa or depin) each declare their new event_kinds under that pillar's existing `manifest.json`. The manifest entry is the public extension point — code reads the manifest, doesn't hardcode the action discriminator. Per `project_doorway_manifest_driven_routes`.

---

# Definitions of done — per phase

| Phase | DoD |
|-------|-----|
| **A** | The primitive ships. `delegates-compute` Commitments are authored by operators in production; events back-reference them; the bounds validator gates production traffic on at least one instance (Z.D). Inventory gossip has a typed wire format. |
| **B** | Hosting is the second proven instance. Inventory gossip is bidirectionally bound to `serve-url-projection` Commitments — peers without active Commitments stop being trusted; peers with revoked Commitments stop being applied within one snapshot cycle. The inventory verifier from S0 is now a fallback (the type system + Commitment back-reference do the real work). |
| **C** | At least 4 of the 5 instances are in production (recovery delegation MUST be one of them — the highest-stakes row). Each instance's per-pillar UI is operational. |
| **D** | New Commitments are gated on standing; capacity collisions resolve via elohim mediation; chronically-bad agents cannot author new Commitments without elohim sponsorship. |
| **E** | A non-technical operator (grandmother bar per `project_subsume_g_f_a_via_it_just_works`) can understand who has authority to do what, see when authority has been delegated to them, revoke that authority, and see the audit log of what happened under it. |

---

# Risk register

| Risk | Sprint | Mitigation |
|------|--------|------------|
| Bounds validator becomes too slow (every event walks back-ref → conductor fetch → schema check → rate query) | S2 | Cache active Commitments per agent in elohim-storage with revocation-aware invalidation. Sprint 4's gossip-receive path is the hot path; profile it. |
| `signal_kind` extensibility outruns the standing aggregator (new signal types ship faster than aggregator weights are updated) | S2, S5e (introduces compute-breach) | Aggregator reads weights from a manifest-declared registry per `project_signal_kind_extensible_protocol_class`; manifests carry the weight, code carries the math. |
| Operator-authored Commitment authoring is too friction-laden, operators bypass via X-API-Key | S1, S3, all of C | Make the authoring CLI/UI exceptional; default reach_ceiling sensibly per pillar; one-command rotation. The elohim-agent assists per `project_elohim_as_counsel`. |
| Revocation propagation is too slow (DHT eventual-consistency window) | S4, S8 | Revocations gossip on a dedicated topic (`elohim/commitments/revocation`) for fast propagation; the DHT-resident entry is canonical truth; receivers honor whichever is more recent. |
| Parallel Phase C sprints conflict on shared substrate (e.g., two sprints both add to `mishpat::commitments::lib.rs`) | All of C | Phase C teams coordinate via librarian-managed sprint task list; bounds validator + Commitment coordinator are the merge points; PRs serialize. |
| Standing aggregator becomes a denouncement engine (chronically-bad agents have no path back to good standing) | S2, S6 | Aggregator's weights include time-decay; matchmaking gate's `Pending` state routes through elohim sponsorship per `project_elohim_as_counsel`; gospel-tier value: stewardship over punishment. |
| Z.D operator-steward key custody is fragile (operator loses key = all CI breaks) | S1 | Recovery delegation (S5a) IS the answer for this; lands second-in-Phase-C precisely for this reason. Z.D operators get briefed on recovery quorum setup as Z.D ships. |
| The whole roadmap is too long; substrate-rea pressure resurges before Phase D | All | Phase A is fully sequential and cluster-unblocking; ship that first regardless. Phases C can fan out parallel agentic-developer shifts to compress wall-clock time. The roadmap is correct order; calendar is operator's call. |

---

# Self-review

**Spec coverage check:** the user asked for "step-by-step get to finished implementation native rea implementation status, multiple sprint, agent teams, parallel stages, whatever is needed to do it right with clean architecture that gets us to the goal." This roadmap covers: ✓ step-by-step (Sprint 0 bite-sized; Sprints 1-2 task-bullet-detailed; 3-8 sprint-summary-detailed with file paths + agent teams), ✓ multiple sprints (11), ✓ agent teams (allocation table + per-sprint), ✓ parallel stages (Phase C is explicitly parallel; S2 can parallel with S1), ✓ clean architecture (one primitive, one validator, one aggregator, instances copy the pattern; type-system hardening via newtypes; schema-first wire formats; substrate-tier separation honored).

**Placeholder scan:** Sprints 3-8 use "blocks/blocked-by + files + agent team + done-when" rather than bite-sized tasks, by design — those become detailed plans when picked up. Sprint 0 has full bite-sized TDD steps. Sprints 1+2 have file paths, API surfaces, and hand-off-to-next-sprint paragraphs sufficient for a fresh subagent to author the detailed plan via `/superpowers:writing-plans`. No TBDs or "implement later" inside the detailed sprints.

**Type consistency:** `BlobAddress` (S0) is used in S4 inventory-gossip extension. `bounds_validator::validate` (S2) signature is referenced in S1, S3, S5a-e. `standing_aggregator::standing_for` (S2) is referenced in S6. `CommitmentFetcher` / `RateHistory` traits are introduced in S2 and reused. `delegates-compute` Commitment action is consistent across all instance descriptions.

---

# Execution handoff

Plan saved to `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`.

**Two execution paths:**

1. **Sprint-by-sprint subagent-driven (recommended)** — dispatch one agentic-developer shift per sprint via `/shift`; each sprint runs to its definition-of-done before the next starts (except Phase C which fans out parallel shifts). Sprints 0-2 use this roadmap's detailed task lists; Sprints 3-8 each start with `/superpowers:writing-plans` against the sprint's summary section to author a per-sprint bite-sized plan.

2. **Inline batch through Phase A only** — execute Sprints 0-2 in this session using `superpowers:executing-plans`, checkpointing for review between sprints. Defer Phases B-E to subagent shifts. This is the lowest-overhead path if you want Phase A done quickly and then re-evaluate.

**Which approach?**
