# Recovery M4 Completion + Shamir Optionality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish Recovery Protocol Phase 2 M4 (fast-path revocation) on the consolidated attestation substrate; finalize Shamir off-chain transport as a fully optional cryptographic proof layer; retire the lapped feature branch.

**Architecture:** Recovery primitives (`RecoveryRequest`, `KeyRevocation`, `IdentityFreeze`) migrate from bespoke imagodei DNA entry types onto the consolidated `Content` discriminator pattern (`governance-action:recovery-request`, `governance-action:key-revocation`, `governance-action:identity-freeze`) by bridging through the existing elohim coordinator's `propose_governance_action` / `issue_attestation` paths. Storage gains a sibling `RecoveryFlowProjector` (state-machine controller for `Open → Quorum → Effective`) and a co-located `key_revocations` table writer alongside `AttestationProjector` (accumulator); a central elohim-DNA signal dispatcher does prefix-routing between them. The Shamir transport is wired into `ElohimStorageBehaviour` following the `trust_protocol` pattern with manifest-declared custodian discovery (no gossipsub capability scan), and the recovery completion path is audited so that Shamir failure can never abort an otherwise-valid social-threshold recovery.

**Tech Stack:** Rust (Holochain HDK 0.6 / HDI; libp2p 0.54 request-response + Kademlia; Diesel + SQLite); JSON Schema + ts-rs codegen; Cucumber (a2o); SweetTest (Holochain integration); Angular 19 (audit-only).

**Source spec:** `genesis/docs/plans/2026-05-15-recovery-m4-completion-shamir-optional-kickoff-prompt.md`
**Brainstorm decisions:** `genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md` (D1–D4 resolved)
**Cross-sprint binding:** D2 commits EPR companion sprint's D3 (duality, no Content envelope inlined).

---

## P2P Design Gate — Source-of-Truth Declarations (mandatory)

Per the kickoff prompt's P2P Design Gate and `.claude/skills/p2p-design-gate/SKILL.md`, every new persistent surface introduced by this sprint MUST declare its category and source of truth before SQL or schema lands. This sprint introduces **zero new DHT entry types**, **zero new HTTP routes**, and **zero new wire protocols** beyond the already-registered `ShamirShareCodec`. It DOES introduce two new SQLite projection tables. Both are Category C (operational, rebuildable):

| Persistent surface | Category | Source of truth | Identity | Rebuild path |
|---|---|---|---|---|
| `recovery_flows` table (SQLite, elohim-storage) | **C — operational** | Holochain DHT: elohim DNA `Content` entries with `content_type ∈ {governance-action:recovery-request, governance-action:identity-freeze, governance-action:key-revocation}` and their vote-children `attestation:recovery-approval` / `attestation:revocation-vote`. The `dht_anchor_hash` column on every row links back to the canonical entry. | Content CID (`id` column = the governance-action's `Content.id`, content-derived). NOT an autoincrement integer. | Replay every matching `ElohimContentSignal` through `recovery_flow_projector::handle_content_signal`. The table is deleteable and reconstructable without data loss. |
| `key_revocations` table (SQLite, elohim-storage; EPR W2D co-located here per D1) | **C — operational** | Holochain DHT: `governance-action:key-revocation` Content entries (envelope) + `attestation:revocation-vote` Content children (vote state). The W2D `derived_compromise_at` column is a controller projection of the compromise-window sweep, also rebuildable from DHT state. | Content CID. | Same replay path as `recovery_flows`. |
| `ShamirShareCodec` swarm registration (libp2p `/elohim/shamir-share/1.0.0`) | **wire-protocol** — already declared in `elohim/elohim-storage/src/p2p/shamir_transport.rs` | The protocol carries ephemeral share material; authorization remains on the DHT as `attestation:recovery-approval`. No persistent storage of share bytes; the codec is a transport, not a notary. | n/a (transport) | n/a |
| `governance-action:shamir-custody-setup` discriminator (manifest entry only) | **A — notarized** | Same as all other Content entries: elohim DNA DHT. No new entry type — extends the existing `Content` discriminator vocabulary. | Content CID. | DHT projection via existing AttestationProjector + RecoveryFlowProjector dispatch. |
| `governance-action:identity-freeze` discriminator (manifest entry only) | **A — notarized** | Same as above. | Content CID. | DHT projection. |

**Rules this declaration enforces:**
- All projection rows MUST carry `dht_anchor_hash`; reads MUST treat these tables as caches; on disagreement with the DHT, the DHT wins.
- Both tables MUST be reconstructable by replaying signals from a clean migration — verified by Task 28's acceptance check (drop the DB, restart, confirm state recomposes from signal replay).
- `id` columns are Content CIDs (content-derived). No autoincrement primary keys. No new slug identifiers introduced.
- No coordinator/HTTP write path bypasses the signal projection; the only writers are `recovery_flow_projector::handle_content_signal` and the EPR W2D compromise-window sweep (operating on `derived_compromise_at` only).

---

## File Structure

### Files created (this sprint)

| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/services/recovery_flow_projector.rs` | State-machine projector for `recovery-request:*`, `governance-action:recovery-request`, `governance-action:key-revocation`, `key-revocation:*`, `governance-action:identity-freeze`; emits `recovery_flows` rows (Open/Quorum/Effective) and the co-located `key_revocations` projection (EPR W2D). |
| `elohim/elohim-storage/src/services/elohim_content_dispatcher.rs` | Central prefix-router for `ElohimContentSignal`. Routes `attestation:*` and non-recovery `governance-action:*` to `attestation_projector::handle_content_signal`; routes recovery families to `recovery_flow_projector::handle_content_signal`. Single entry point wired by `main.rs`. |
| `elohim/elohim-storage/src/db/recovery_flows.rs` | Diesel CRUD for the `recovery_flows` projection. |
| `elohim/elohim-storage/src/db/key_revocations.rs` | Diesel CRUD for the `key_revocations` projection (EPR W2D, co-located here per D1). |
| `elohim/elohim-storage/migrations/2026-05-15-000000_recovery_flows/{up,down}.sql` | Schema for `recovery_flows` + `key_revocations` tables. |
| `elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs` | SweetTest covering bridged `create_recovery_request`, `create_self_revocation`, `create_revocation_request`, `submit_revocation_vote`, `IdentityFreeze` end-to-end against the consolidated coordinator. |
| `genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature` | `@recovery-shamir-optional` scenarios — Path A (social-only) and Path B (social + Shamir). |

### Files modified

| Path | Change |
|------|--------|
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` | Stage 1: switch `submit_intimate_witness` Gate 1 + `commit_key_rotation` revocation-floor + freeze-floor gates to cross-DNA `get` + `Content` decode. Stage 2: bridge `create_recovery_request` (lib.rs:2112), `create_self_revocation` (lib.rs:2199), `create_revocation_request` (lib.rs:2341), `IdentityFreeze` creation to the elohim coordinator; remove the `TODO(stage-G-followup)` block at ~2925; remove direct `create_entry(&EntryTypes::RecoveryRequest…)` / `KeyRevocation` / `IdentityFreeze` uses. |
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs` | Replace `create_entry(&EntryTypes::KeyRevocation(…))` at line 201 with bridge call; switch any internal entry reads to cross-DNA `Content` decode. |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Stage 4a: add `shamir_share_protocol: RequestResponse<ShamirShareCodec>` field at line ~88 (mirror `trust_protocol`); add `ShamirShareProtocol(...)` variant + `From` impl. |
| `elohim/elohim-storage/src/p2p/mod.rs` | Stage 4a: add match arm for `ElohimStorageBehaviourEvent::ShamirShareProtocol(...)` after `TrustProtocol` block (~4054); mirror request/response handling for `ShamirShareRequest`/`ShamirShareResponse`. |
| `elohim/elohim-storage/src/p2p/shamir_transport.rs` | Stage 4a: remove `TODO(G.1-swarm-wiring)` block at lines 18–25 once wired. |
| `elohim/elohim-storage/src/main.rs` | Stage 2: register the `ElohimContentSignal` subscriber alongside `subscribe_infrastructure_signals` (~line 606), wiring it to `elohim_content_dispatcher::dispatch`. |
| `elohim/elohim-storage/src/hc_client.rs` | Stage 2: add `subscribe_elohim_content_signals<F>()` mirroring `subscribe_infrastructure_signals` (~line 392). |
| `elohim/elohim-storage/src/services/mod.rs` | Add `pub mod recovery_flow_projector;` and `pub mod elohim_content_dispatcher;`. |
| `elohim/elohim-storage/src/db/mod.rs` | Add `pub mod recovery_flows;` and `pub mod key_revocations;`. |
| `elohim/sdk/domains/imagodei/manifest.json` | Stage 2: add `governance-action:identity-freeze` entry (currently only `attestation:identity-freeze` is present); Stage 4b: add `governance-action:shamir-custody-setup` discriminator. |
| `genesis/a2o/features/auth/recovery/revocation-self.feature` | Stage 5: lift `@wip` on the "Matthew can initiate full recovery after revoking his only key" scenario (line 50). |
| `genesis/a2o/features/auth/recovery/revocation-emergency-quorum.feature` | Stage 5: verify all scenarios pass; no `@wip` markers expected to remain. |

### Files explicitly NOT touched (out of scope)

These surfaces stay as-is. The DHT is source of truth for all of them; no new notarized entry types are introduced; no storage projection schemas are altered.

- `elohim/sdk/schemas/v1/dna-signals/{key-rotation,key-revocation,agent-peer-binding,revocation-attestation}.schema.json` — duality decision (D2): existing contracts stand; no schema changes. The DNA emits against these schemas as already declared; no new wire surface.
- `elohim/elohim-storage/src/services/attestation_projector.rs` — sibling-projector decision (D1): the accumulator projection of DHT-notarized `attestation:*` / `governance-action:*` Content stays untouched; recovery state lives in the sibling `RecoveryFlowProjector`.
- Browser-side Angular components needing new UI surfaces for `@recovery-shamir-optional` step defs — handed off as a follow-up; this sprint writes cucumber + Rust harness only.

---

## Task Decomposition

### Stage 1 — Cross-DNA gate-reader migration (unblocks bridging)

#### Task 1: Audit imagodei zome for legacy entry-type readers

**Files:**
- Modify (read only): `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`
- Modify (read only): `elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs`

- [ ] **Step 1: Grep every reader site**

Run:
```bash
grep -n "to_app_option::<RecoveryRequest>\|to_app_option::<KeyRevocation>\|to_app_option::<IdentityFreeze>\|::<RecoveryRequest>()\|::<KeyRevocation>()\|::<IdentityFreeze>()" elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs
```
Expected: each call site that reads one of the three legacy entry types.

- [ ] **Step 2: Record sites in `genesis/docs/plans/2026-05-15-recovery-m4-stage1-audit.md`**

Write a short audit file capturing for each site: function name, file:line, the legacy entry type, the gate's semantic role (e.g. "submit_intimate_witness Gate 1 — open-request precondition"), and whether the gate also needs the `human_id` or any envelope field that should come from the `Content.metadata` JSON after migration.

The audit also answers the **D4.1 sub-question**: which governance-action kind carries the Shamir custodian manifest. Settle on `governance-action:shamir-custody-setup` (separate entry committed at onboarding) — that keeps recovery-request bodies slim and lets the custody manifest be revised independently. Record the decision and reasoning in the audit file.

- [ ] **Step 3: Commit the audit**

```bash
git add genesis/docs/plans/2026-05-15-recovery-m4-stage1-audit.md
git commit -m "docs(recovery-m4): stage-1 audit of cross-DNA gate readers"
```

---

#### Task 2: Add `decode_content_entry` helper in imagodei

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei/src/content_decode.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (add `mod content_decode;` and `use content_decode::*;`)

- [ ] **Step 1: Write the failing unit test**

Create `elohim/holochain/dna/imagodei/zomes/imagodei/src/content_decode.rs`:
```rust
//! Cross-DNA Content entry decoder, mirroring elohim DNA's
//! `attestation_validator::decode_content_entry`. Pure deserialization helper —
//! callers handle the DHT `get`/`must_get_entry` themselves.

use hdk::prelude::*;
use serde::{Deserialize, Serialize};

/// Subset of the elohim DNA's `Content` entry that imagodei gates need to read.
/// Fields beyond these are ignored — only the discriminator + metadata matter
/// to the cross-DNA gate path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDnaContent {
    pub id: String,
    pub content_type: String,
    pub author_id: String,
    /// JSON-stringified metadata (parsed by caller into the gate-specific shape).
    pub metadata_json: String,
}

/// Decode a Content entry from a cross-DNA `get()` result.
///
/// Mirrors elohim DNA's `attestation_validator::decode_content_entry` at
/// `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs:623`.
pub fn decode_content_entry(entry: Entry) -> Result<CrossDnaContent, String> {
    match entry {
        Entry::App(app_bytes) => {
            let bytes: Vec<u8> = app_bytes.into_sb().bytes().to_vec();
            serde_json::from_slice::<CrossDnaContent>(&bytes)
                .map_err(|e| format!("decode_content_entry: {e}"))
        }
        _ => Err("decode_content_entry: entry is not an App entry".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rejects_non_app_entry() {
        let entry = Entry::CounterSign(Default::default(), vec![]);
        assert!(decode_content_entry(entry).is_err());
    }
}
```

- [ ] **Step 2: Run the unit test to verify it fails (module not declared)**

Run:
```bash
cd elohim/holochain/dna/imagodei && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --target wasm32-unknown-unknown -p imagodei content_decode 2>&1 | head
```
Expected: FAIL with "module not declared" / unresolved import.

- [ ] **Step 3: Wire the module into `lib.rs`**

Edit `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` near the other top-level `mod` declarations (around line 1–50 — locate the existing `mod submit_specialist_revocation;` block) and add:
```rust
mod content_decode;
```

- [ ] **Step 4: Run the unit test to verify it passes**

Run:
```bash
cd elohim/holochain/dna/imagodei && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --target wasm32-unknown-unknown -p imagodei content_decode 2>&1 | tail -5
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/content_decode.rs elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(recovery-m4): add cross-DNA Content decoder helper in imagodei zome"
```

---

#### Task 3: Migrate `submit_intimate_witness` Gate 1 to cross-DNA `Content`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (function near line 2933)

- [ ] **Step 1: Write the failing SweetTest assertion**

Edit `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs` (the existing M3 intimate-quorum harness) — add a new test that:
1. Creates the `governance-action:recovery-request` on the elohim DNA via the consolidated bridge (use the existing `propose_governance_action` SweetCall helper from `attestation_coordinator.rs`).
2. Calls `submit_intimate_witness` on the imagodei zome with the **CID** (not the imagodei action hash) of that governance-action.
3. Asserts the call succeeds and that the bridged `attestation:humanness` is committed to the elohim DNA.

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 600 cargo test recovery_m3::intimate_witness_reads_cross_dna_recovery_request 2>&1 | tail -20
```
Expected: FAIL — current Gate 1 still does `get(recovery_request_hash) → to_app_option::<RecoveryRequest>()` and panics on the Content payload.

- [ ] **Step 2: Rewrite Gate 1 to cross-DNA `Content` decode**

In `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, replace the `submit_intimate_witness` Gate 1 block:

```rust
// Gate 1: fetch the recovery-request Content entry on the elohim DNA.
let cell = CallTargetCell::OtherRole("elohim".into());
let response = call(
    cell,
    ZomeName::from("content_store"),
    FunctionName::from("get_content_by_cid"),
    None,
    input.recovery_request_cid.clone(),
)?;
let cross_dna: crate::content_decode::CrossDnaContent = match response {
    ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "submit_intimate_witness Gate 1: decode failed: {e}"
        )))
    })?,
    other => {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_intimate_witness Gate 1: cross-DNA get failed: {other:?}"
        ))));
    }
};
if cross_dna.content_type != "governance-action:recovery-request" {
    return Err(wasm_error!(WasmErrorInner::Guest(format!(
        "submit_intimate_witness Gate 1: expected recovery-request, got {}",
        cross_dna.content_type
    ))));
}
let metadata: serde_json::Value = serde_json::from_str(&cross_dna.metadata_json)
    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
        "submit_intimate_witness Gate 1: bad metadata_json: {e}"
    ))))?;
let human_id = metadata["human_id"]
    .as_str()
    .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
        "submit_intimate_witness Gate 1: recovery-request metadata missing human_id".into()
    )))?
    .to_string();
```

Also update the `SubmitIntimateWitnessInput` struct (search for its definition above) to rename `recovery_request_hash: ActionHash` → `recovery_request_cid: String`. Update all callers in this file and in the SweetTest harness.

- [ ] **Step 3: Re-run the SweetTest**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 600 cargo test recovery_m3::intimate_witness_reads_cross_dna_recovery_request 2>&1 | tail -20
```
Expected: PASS.

- [ ] **Step 4: Run the broader recovery_m3 suite — no regression**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1200 cargo test recovery_m3 2>&1 | tail -30
```
Expected: all `recovery_m3::*` tests PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs
git commit -m "feat(recovery-m4): submit_intimate_witness reads recovery-request via cross-DNA Content"
```

---

#### Task 4: Migrate `commit_key_rotation` revocation-floor + freeze-floor gates

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (function `commit_key_rotation` — locate by `grep -n "fn commit_key_rotation" lib.rs`)

- [ ] **Step 1: Write failing SweetTest**

Add to `elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs`:
- Scenario A: `commit_key_rotation` BLOCKS when an `attestation:identity-freeze` exists on the elohim DNA for the target human. Currently the gate reads `IdentityFreeze` from imagodei — assert it now reads the consolidated entry instead.
- Scenario B: `commit_key_rotation` BLOCKS when an effective `governance-action:key-revocation` exists for the rotated key on the elohim DNA. Currently the gate reads `KeyRevocation` from imagodei — assert it now reads the consolidated entry.

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 600 cargo test recovery_m4::commit_key_rotation_blocked_by_consolidated_freeze 2>&1 | tail -20
```
Expected: FAIL — current gate still reads imagodei-local `IdentityFreeze` entries.

- [ ] **Step 2: Rewrite the revocation-floor gate**

Replace the gate's `must_get_valid_record` + `to_app_option::<KeyRevocation>()` with a cross-DNA query:
```rust
// Revocation-floor gate: query elohim DNA for an effective key-revocation
// over this key. Returns the most recent effective CID (or None).
let cell = CallTargetCell::OtherRole("elohim".into());
let revoked_key_str = input.revoked_key.to_string();
let response = call(
    cell,
    ZomeName::from("content_store"),
    FunctionName::from("query_effective_revocation_for_key"),
    None,
    revoked_key_str.clone(),
)?;
let effective_revocation_cid: Option<String> = match response {
    ZomeCallResponse::Ok(payload) => payload.decode().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "revocation-floor gate decode: {e}"
        )))
    })?,
    other => return Err(wasm_error!(WasmErrorInner::Guest(format!(
        "revocation-floor gate cross-DNA call failed: {other:?}"
    )))),
};
if effective_revocation_cid.is_some() {
    return Err(wasm_error!(WasmErrorInner::Guest(
        "commit_key_rotation: key is already revoked (revocation-floor gate)".into()
    )));
}
```

Apply the same pattern to the freeze-floor gate, calling
`query_effective_identity_freeze_for_human(human_id)` on the elohim coordinator.

**Note on coordinator-side functions:** `query_effective_revocation_for_key` and `query_effective_identity_freeze_for_human` may not yet exist in the elohim content_store coordinator. If absent, add them as thin wrappers in `elohim/holochain/dna/elohim/zomes/content_store/src/` that traverse the existing anchor-link layout for `attestation:identity-freeze` and `governance-action:key-revocation` Content entries; either way the gate writes against this contract. Confirm during this task; if a coordinator-side function needs adding, write it before continuing this task's Step 3.

- [ ] **Step 3: Re-run the SweetTests**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1200 cargo test recovery_m4::commit_key_rotation 2>&1 | tail -20
```
Expected: both Scenario A and Scenario B PASS.

- [ ] **Step 4: Confirm `recovery_m3` + `attestation_coordinator` still pass**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1500 cargo test recovery_m3 attestation_coordinator 2>&1 | tail -30
```
Expected: no regression.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/dna/elohim/zomes/content_store/src/ elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs
git commit -m "feat(recovery-m4): commit_key_rotation reads revocation/freeze floors via cross-DNA Content"
```

---

#### Task 5: Migrate any remaining `IdentityFreeze`/`KeyRevocation`/`RecoveryRequest` readers

**Files:**
- Modify: any imagodei zome source flagged in the Task 1 audit but not yet migrated.

- [ ] **Step 1: Re-grep**

Run:
```bash
grep -n "to_app_option::<RecoveryRequest>\|to_app_option::<KeyRevocation>\|to_app_option::<IdentityFreeze>" elohim/holochain/dna/imagodei/zomes/imagodei/src/
```
Expected: zero hits. If any remain, migrate using the same pattern as Tasks 3–4: cross-DNA `call` + `decode_content_entry`.

- [ ] **Step 2: SweetTest each newly migrated gate**

For each remaining reader, add a scenario asserting it operates against the consolidated Content entry. Pattern after Task 4 Step 1.

- [ ] **Step 3: Run them and verify failure → fix → pass cycle per site**

Run after each migration:
```bash
cd elohim/holochain/tests/sweettest && timeout 1200 cargo test <test_name> 2>&1 | tail -20
```
Expected: FAIL before migration, PASS after.

- [ ] **Step 4: Final grep — zero legacy readers**

Run:
```bash
grep -rn "to_app_option::<RecoveryRequest>\|to_app_option::<KeyRevocation>\|to_app_option::<IdentityFreeze>" elohim/holochain/
```
Expected: zero hits.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/
git commit -m "feat(recovery-m4): migrate remaining legacy entry-type readers to cross-DNA Content"
```

---

### Stage 2 — Bridge create-side + RecoveryFlowProjector

#### Task 6: Schema migration — `recovery_flows` + `key_revocations` tables

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-15-000000_recovery_flows/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-15-000000_recovery_flows/down.sql`

- [ ] **Step 1: Write `up.sql`**

```sql
-- Recovery flow state-machine projection.
-- Source of truth: elohim DNA Content entries with
-- content_type = 'governance-action:recovery-request'
--              | 'governance-action:identity-freeze'
-- The DHT anchor hash provides provenance back to the canonical entry.
CREATE TABLE recovery_flows (
    id                       TEXT PRIMARY KEY,
    dht_anchor_hash          BLOB NOT NULL,
    flow_kind                TEXT NOT NULL,           -- 'recovery-request' | 'identity-freeze'
    subject_human_id         TEXT NOT NULL,
    initiated_by_cid         TEXT NOT NULL,
    state                    TEXT NOT NULL,           -- 'Open' | 'Quorum' | 'Effective' | 'Closed'
    required_votes           INTEGER NOT NULL,
    current_votes            INTEGER NOT NULL DEFAULT 0,
    threshold_reached        INTEGER NOT NULL DEFAULT 0,  -- bool 0/1
    effective_at             TEXT,                    -- ISO 8601 once Effective
    closes_at                TEXT NOT NULL,           -- proposal close deadline
    metadata_json            TEXT NOT NULL,           -- full metadata for state queries
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);
CREATE INDEX recovery_flows_subject_idx ON recovery_flows(subject_human_id);
CREATE INDEX recovery_flows_state_idx ON recovery_flows(state);

-- Key revocation projection (EPR W2D — co-located per D1).
-- Source of truth: elohim DNA Content entries with
-- content_type = 'governance-action:key-revocation' and
-- the corresponding 'attestation:revocation-vote' children.
CREATE TABLE key_revocations (
    id                       TEXT PRIMARY KEY,
    dht_anchor_hash          BLOB NOT NULL,
    subject_human_id         TEXT NOT NULL,
    revoked_key              TEXT NOT NULL,
    trigger_type             TEXT NOT NULL,           -- 'voluntary' | 'steward_vote' | 'challenge' | 'specialist_attestation'
    reason                   TEXT NOT NULL,
    initiated_by_cid         TEXT NOT NULL,
    required_votes           INTEGER NOT NULL,
    current_votes            INTEGER NOT NULL DEFAULT 0,
    threshold_reached        INTEGER NOT NULL DEFAULT 0,
    effective_at             TEXT,                    -- non-null when revocation is effective
    derived_compromise_at    TEXT,                    -- EPR W2D field
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);
CREATE INDEX key_revocations_subject_idx ON key_revocations(subject_human_id);
CREATE INDEX key_revocations_revoked_key_idx ON key_revocations(revoked_key);
CREATE INDEX key_revocations_effective_idx ON key_revocations(effective_at);
```

- [ ] **Step 2: Write `down.sql`**

```sql
DROP INDEX IF EXISTS key_revocations_effective_idx;
DROP INDEX IF EXISTS key_revocations_revoked_key_idx;
DROP INDEX IF EXISTS key_revocations_subject_idx;
DROP TABLE IF EXISTS key_revocations;
DROP INDEX IF EXISTS recovery_flows_state_idx;
DROP INDEX IF EXISTS recovery_flows_subject_idx;
DROP TABLE IF EXISTS recovery_flows;
```

- [ ] **Step 3: Verify timestamp doesn't collide**

Run:
```bash
ls elohim/elohim-storage/migrations | grep "^2026-05-15"
```
Expected: only the newly created directory. If another `2026-05-15-000000_*` migration already exists, bump the timestamp by one second (see `feedback_diesel_migration_timestamp_collision` memory).

- [ ] **Step 4: Compile-check**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -20
```
Expected: no errors (Diesel picks up the new migration via `embed_migrations!`).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-15-000000_recovery_flows/
git commit -m "feat(recovery-m4): add recovery_flows and key_revocations projection tables"
```

---

#### Task 7: Diesel models + CRUD for `recovery_flows` and `key_revocations`

**Files:**
- Modify: `elohim/elohim-storage/src/db/schema.rs` (run `diesel print-schema` or edit by hand to add the two tables)
- Modify: `elohim/elohim-storage/src/db/models.rs` (add `RecoveryFlowRow`, `KeyRevocationRow`)
- Create: `elohim/elohim-storage/src/db/recovery_flows.rs`
- Create: `elohim/elohim-storage/src/db/key_revocations.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Write failing CRUD unit tests**

Create `elohim/elohim-storage/src/db/recovery_flows.rs`:
```rust
//! CRUD for the recovery_flows projection table.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::RecoveryFlowRow;
use crate::db::schema::recovery_flows;
use crate::error::StorageError;

pub fn upsert(conn: &mut SqliteConnection, row: &RecoveryFlowRow) -> Result<(), StorageError> {
    diesel::insert_into(recovery_flows::table)
        .values(row)
        .on_conflict(recovery_flows::id)
        .do_update()
        .set((
            recovery_flows::state.eq(&row.state),
            recovery_flows::current_votes.eq(row.current_votes),
            recovery_flows::threshold_reached.eq(row.threshold_reached),
            recovery_flows::effective_at.eq(&row.effective_at),
            recovery_flows::metadata_json.eq(&row.metadata_json),
            recovery_flows::updated_at.eq(&row.updated_at),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn get_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<RecoveryFlowRow>, StorageError> {
    let row = recovery_flows::table
        .filter(recovery_flows::id.eq(id))
        .first::<RecoveryFlowRow>(conn)
        .optional()?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        conn
    }

    #[test]
    fn upsert_advances_state() {
        let mut conn = setup();
        let mut row = RecoveryFlowRow {
            id: "flow-001".into(),
            dht_anchor_hash: vec![1, 2, 3],
            flow_kind: "recovery-request".into(),
            subject_human_id: "human-A".into(),
            initiated_by_cid: "cid-B".into(),
            state: "Open".into(),
            required_votes: 3,
            current_votes: 0,
            threshold_reached: 0,
            effective_at: None,
            closes_at: "2099-01-01T00:00:00Z".into(),
            metadata_json: "{}".into(),
            created_at: "2026-05-15T00:00:00Z".into(),
            updated_at: "2026-05-15T00:00:00Z".into(),
        };
        upsert(&mut conn, &row).unwrap();
        row.state = "Quorum".into();
        row.current_votes = 3;
        row.threshold_reached = 1;
        upsert(&mut conn, &row).unwrap();
        let stored = get_by_id(&mut conn, "flow-001").unwrap().unwrap();
        assert_eq!(stored.state, "Quorum");
        assert_eq!(stored.current_votes, 3);
    }
}
```

Create `elohim/elohim-storage/src/db/key_revocations.rs` with parallel structure: `upsert`, `get_by_id`, `get_by_revoked_key`, plus one unit test that asserts the `effective_at` field is preserved across upsert.

- [ ] **Step 2: Add models in `db/models.rs`**

```rust
#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset)]
#[diesel(table_name = crate::db::schema::recovery_flows)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RecoveryFlowRow {
    pub id: String,
    pub dht_anchor_hash: Vec<u8>,
    pub flow_kind: String,
    pub subject_human_id: String,
    pub initiated_by_cid: String,
    pub state: String,
    pub required_votes: i32,
    pub current_votes: i32,
    pub threshold_reached: i32,
    pub effective_at: Option<String>,
    pub closes_at: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset)]
#[diesel(table_name = crate::db::schema::key_revocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct KeyRevocationRow {
    pub id: String,
    pub dht_anchor_hash: Vec<u8>,
    pub subject_human_id: String,
    pub revoked_key: String,
    pub trigger_type: String,
    pub reason: String,
    pub initiated_by_cid: String,
    pub required_votes: i32,
    pub current_votes: i32,
    pub threshold_reached: i32,
    pub effective_at: Option<String>,
    pub derived_compromise_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 3: Update `db/schema.rs`**

Append to the macro-generated schema:
```rust
diesel::table! {
    recovery_flows (id) {
        id -> Text,
        dht_anchor_hash -> Binary,
        flow_kind -> Text,
        subject_human_id -> Text,
        initiated_by_cid -> Text,
        state -> Text,
        required_votes -> Integer,
        current_votes -> Integer,
        threshold_reached -> Integer,
        effective_at -> Nullable<Text>,
        closes_at -> Text,
        metadata_json -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
diesel::table! {
    key_revocations (id) {
        id -> Text,
        dht_anchor_hash -> Binary,
        subject_human_id -> Text,
        revoked_key -> Text,
        trigger_type -> Text,
        reason -> Text,
        initiated_by_cid -> Text,
        required_votes -> Integer,
        current_votes -> Integer,
        threshold_reached -> Integer,
        effective_at -> Nullable<Text>,
        derived_compromise_at -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

- [ ] **Step 4: Wire modules into `db/mod.rs`**

Add:
```rust
pub mod recovery_flows;
pub mod key_revocations;
```

- [ ] **Step 5: Run the unit tests — must pass**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test db::recovery_flows db::key_revocations 2>&1 | tail -20
```
Expected: PASS for both.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/db/
git commit -m "feat(recovery-m4): Diesel models and CRUD for recovery_flows + key_revocations"
```

---

#### Task 8: `RecoveryFlowProjector` skeleton + Open-state branch

**Files:**
- Create: `elohim/elohim-storage/src/services/recovery_flow_projector.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write the failing test for the Open-state branch**

Create `elohim/elohim-storage/src/services/recovery_flow_projector.rs`:
```rust
//! State-machine projector for recovery-flow Content signals.
//!
//! Source of truth: elohim DNA. This module observes signals and projects them
//! into two SQLite tables: `recovery_flows` (state machine: Open → Quorum →
//! Effective → Closed) and `key_revocations` (EPR W2D co-located projection).
//!
//! Routing (driven by the central elohim_content_dispatcher):
//! - `governance-action:recovery-request` → recovery_flows (Open)
//! - `governance-action:identity-freeze` → recovery_flows (Effective immediately, no quorum)
//! - `governance-action:key-revocation` → key_revocations (Open) + recovery_flows mirror
//! - `attestation:recovery-approval` with parent_governance_action_cid → vote
//!   advances the parent recovery_flows row toward Quorum
//! - `attestation:revocation-vote` with parent → vote advances key_revocations row
//!
//! Vote-children (`attestation:recovery-approval`, `attestation:revocation-vote`)
//! continue to be projected into the `attestations` accumulator by
//! `AttestationProjector` — this module only updates the FLOW state for them.

use diesel::sqlite::SqliteConnection;
use tracing::debug;

use crate::db::models::{KeyRevocationRow, RecoveryFlowRow};
use crate::db::{key_revocations, recovery_flows};
use crate::error::StorageError;
use crate::signals::ElohimContentSignal;

pub fn handle_content_signal(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    match signal.content_type.as_str() {
        "governance-action:recovery-request" => project_recovery_request_open(conn, signal),
        "governance-action:identity-freeze" => project_identity_freeze(conn, signal),
        "governance-action:key-revocation" => project_key_revocation_open(conn, signal),
        kind if kind.starts_with("attestation:recovery-approval") => {
            project_recovery_vote(conn, signal)
        }
        kind if kind.starts_with("attestation:revocation-vote") => {
            project_revocation_vote(conn, signal)
        }
        _ => {
            debug!(kind = %signal.content_type, "recovery_flow_projector: ignored");
            Ok(())
        }
    }
}

fn project_recovery_request_open(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let metadata: serde_json::Value = serde_json::from_str(&signal.metadata_json).unwrap_or_default();
    let row = RecoveryFlowRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        flow_kind: "recovery-request".into(),
        subject_human_id: metadata["human_id"].as_str().unwrap_or_default().to_string(),
        initiated_by_cid: signal.author_id.clone().unwrap_or_default(),
        state: "Open".into(),
        required_votes: metadata["threshold"]["m"].as_i64().unwrap_or(0) as i32,
        current_votes: 0,
        threshold_reached: 0,
        effective_at: None,
        closes_at: metadata["closes_at"].as_str().unwrap_or_default().to_string(),
        metadata_json: signal.metadata_json.clone(),
        created_at: signal.created_at.clone(),
        updated_at: signal.created_at.clone(),
    };
    recovery_flows::upsert(conn, &row)?;
    debug!(id = %row.id, "recovery_flow opened");
    Ok(())
}

// TODO: project_identity_freeze, project_key_revocation_open,
// project_recovery_vote, project_revocation_vote — implemented in Task 9.
fn project_identity_freeze(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("Task 9")
}
fn project_key_revocation_open(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("Task 9")
}
fn project_recovery_vote(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("Task 9")
}
fn project_revocation_vote(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("Task 9")
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        conn
    }

    fn make_signal(id: &str, content_type: &str, metadata_json: &str) -> ElohimContentSignal {
        ElohimContentSignal {
            id: id.into(),
            content_type: content_type.into(),
            entry_hash: format!("entry-{id}"),
            metadata_json: metadata_json.into(),
            author_id: Some("cid-init".into()),
            title: "T".into(),
            description: String::new(),
            created_at: "2026-05-15T00:00:00Z".into(),
        }
    }

    #[test]
    fn recovery_request_opens_a_flow() {
        let mut conn = setup();
        let signal = make_signal(
            "flow-A",
            "governance-action:recovery-request",
            r#"{"human_id":"human-X","threshold":{"m":3},"closes_at":"2099-01-01T00:00:00Z"}"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();
        let row = recovery_flows::get_by_id(&mut conn, "flow-A").unwrap().unwrap();
        assert_eq!(row.state, "Open");
        assert_eq!(row.required_votes, 3);
        assert_eq!(row.subject_human_id, "human-X");
    }
}
```

In `services/mod.rs`, add:
```rust
pub mod recovery_flow_projector;
```

- [ ] **Step 2: Run — must pass**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test services::recovery_flow_projector 2>&1 | tail -10
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/services/recovery_flow_projector.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(recovery-m4): RecoveryFlowProjector skeleton with Open-state branch"
```

---

#### Task 9: Fill out the four remaining state-machine branches

**Files:**
- Modify: `elohim/elohim-storage/src/services/recovery_flow_projector.rs`

- [ ] **Step 1: Write failing tests for each branch**

Append to `services/recovery_flow_projector.rs` tests:
- `identity_freeze_lands_effective` — `governance-action:identity-freeze` row appears in `recovery_flows` with `state = "Effective"` and `effective_at` set.
- `key_revocation_open_seeds_revocations_table` — `governance-action:key-revocation` upserts a `key_revocations` row with `state` semantics (uses `threshold_reached = 0` / `effective_at = None`).
- `recovery_vote_advances_parent_flow` — given an existing `recovery_flows` row with `required_votes = 2`, a single `attestation:recovery-approval` with `parent_governance_action_cid` increments `current_votes` and transitions to `Quorum` when threshold reached.
- `revocation_vote_advances_key_revocation` — given a `key_revocations` row with `required_votes = 2`, a single `attestation:revocation-vote` with `parent_governance_action_cid` increments `current_votes`; when threshold reached, sets `effective_at` and `threshold_reached = 1`.

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test services::recovery_flow_projector 2>&1 | tail -20
```
Expected: 4 new tests FAIL (`unimplemented!` panics).

- [ ] **Step 2: Implement the four branches**

Replace each `unimplemented!()` body. Key invariants:
- `Open → Quorum`: only flip `state` when `current_votes >= required_votes`; record `effective_at` at that moment for `identity-freeze` and `key-revocation`, but leave `recovery-request` as `Quorum` (the actual rotation lands later via `commit_key_rotation`).
- Each vote-child recompute reads the parent row, increments `current_votes`, and upserts.
- The `key_revocations.derived_compromise_at` field defaults to None and is computed by EPR W2D's compromise-window sweep — leave it None here.
- All four functions are idempotent under signal redelivery (signals can arrive twice on retry).

- [ ] **Step 3: Run tests**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test services::recovery_flow_projector 2>&1 | tail -20
```
Expected: all PASS (including the original `recovery_request_opens_a_flow`).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/recovery_flow_projector.rs
git commit -m "feat(recovery-m4): RecoveryFlowProjector state-machine branches (identity-freeze, key-revocation, votes)"
```

---

#### Task 10: Central `elohim_content_dispatcher` with prefix-routing

**Files:**
- Create: `elohim/elohim-storage/src/services/elohim_content_dispatcher.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write the failing routing test**

```rust
//! Central dispatcher for ElohimContentSignal — prefix-routes between
//! AttestationProjector (accumulator) and RecoveryFlowProjector (state machine).
//!
//! See D1 brainstorm: AttestationProjector handles all `attestation:*` (including
//! `attestation:recovery-approval` and `attestation:revocation-vote`, which are
//! vote-children that ALSO land in the accumulator). RecoveryFlowProjector
//! handles the governance-action openers and the state-machine vote-tracking.
//!
//! Some signals are dispatched to BOTH projectors:
//! - `attestation:recovery-approval` → AttestationProjector (accumulator row +
//!   tally recompute) AND RecoveryFlowProjector (parent flow vote increment).
//! - `attestation:revocation-vote` → same dual-dispatch.

use diesel::sqlite::SqliteConnection;
use tracing::warn;

use crate::error::StorageError;
use crate::services::{attestation_projector, recovery_flow_projector};
use crate::signals::ElohimContentSignal;

pub fn dispatch(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    // 1) Always run AttestationProjector first — it's the accumulator and
    //    initializes tally rows for governance-actions.
    if signal.content_type.starts_with("attestation:")
        || signal.content_type.starts_with("governance-action:")
    {
        attestation_projector::handle_content_signal(conn, signal)?;
    }

    // 2) Then route to RecoveryFlowProjector if the kind matches a recovery family.
    let is_recovery_family = matches!(
        signal.content_type.as_str(),
        "governance-action:recovery-request"
            | "governance-action:key-revocation"
            | "governance-action:identity-freeze"
    ) || signal
        .content_type
        .starts_with("attestation:recovery-approval")
        || signal
            .content_type
            .starts_with("attestation:revocation-vote");

    if is_recovery_family {
        if let Err(e) = recovery_flow_projector::handle_content_signal(conn, signal) {
            warn!(
                kind = %signal.content_type,
                error = %e,
                "recovery_flow_projector dispatch failed"
            );
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{attestations, recovery_flows};
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        conn
    }

    #[test]
    fn recovery_approval_lands_in_both_projectors() {
        let mut conn = setup();
        // First seed an Open recovery flow.
        let flow_signal = crate::signals::ElohimContentSignal {
            id: "flow-1".into(),
            content_type: "governance-action:recovery-request".into(),
            entry_hash: "eh-1".into(),
            metadata_json: r#"{"human_id":"H","threshold":{"m":2},"closes_at":"2099-01-01T00:00:00Z"}"#.into(),
            author_id: Some("init".into()),
            title: "T".into(),
            description: String::new(),
            created_at: "2026-05-15T00:00:00Z".into(),
        };
        dispatch(&mut conn, &flow_signal).unwrap();

        // Now cast a vote.
        let vote_signal = crate::signals::ElohimContentSignal {
            id: "vote-1".into(),
            content_type: "attestation:recovery-approval".into(),
            entry_hash: "eh-2".into(),
            metadata_json: r#"{"subject_cid":"flow-1","subject_kind":"recovery-flow","parent_governance_action_cid":"flow-1","vote_value":"approve"}"#.into(),
            author_id: Some("voter".into()),
            title: "V".into(),
            description: String::new(),
            created_at: "2026-05-15T00:01:00Z".into(),
        };
        dispatch(&mut conn, &vote_signal).unwrap();

        // Accumulator side
        let attest_row = attestations::get_by_id(&mut conn, "vote-1").unwrap().unwrap();
        assert_eq!(attest_row.attestation_kind, "attestation:recovery-approval");
        // State-machine side
        let flow_row = recovery_flows::get_by_id(&mut conn, "flow-1").unwrap().unwrap();
        assert_eq!(flow_row.current_votes, 1);
    }
}
```

Wire into `services/mod.rs`:
```rust
pub mod elohim_content_dispatcher;
```

- [ ] **Step 2: Run — must FAIL first**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test services::elohim_content_dispatcher 2>&1 | tail -20
```
Expected: FAIL until the module compiles + tables exist; should PASS after implementation lands.

- [ ] **Step 3: Iterate to green**

If the dual-dispatch test fails, the most likely cause is that `attestation_projector` doesn't recompute the parent flow's tally (it operates on `governance_action_tally`, not `recovery_flows`) — which is **correct**: the recovery-flow state-machine side is RecoveryFlowProjector's job. Confirm both rows are present.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/elohim_content_dispatcher.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(recovery-m4): central ElohimContentSignal dispatcher with prefix routing"
```

---

#### Task 11: Wire `subscribe_elohim_content_signals` in `hc_client.rs`

**Files:**
- Modify: `elohim/elohim-storage/src/hc_client.rs` (mirror `subscribe_infrastructure_signals` at line ~392)

- [ ] **Step 1: Add the subscription method**

In `hc_client.rs`, just below `subscribe_infrastructure_signals`, add:
```rust
/// Subscribe to ElohimContentSignals emitted by the elohim DNA's content_store
/// coordinator. Each signal carries an `attestation:*` or `governance-action:*`
/// Content entry's projection-relevant fields.
///
/// Non-content signals (lamad/imagodei/mishpat) are logged at debug and ignored.
pub async fn subscribe_elohim_content_signals<F>(&self, handler: F) -> String
where
    F: Fn(crate::signals::ElohimContentSignal) + Send + Sync + 'static,
{
    use holochain_types::signal::Signal;

    self.app_ws
        .on_signal(move |signal| {
            if let Signal::App { signal, .. } = signal {
                let bytes: Vec<u8> = signal.into_inner().into();
                match rmp_serde::from_slice::<serde_json::Value>(&bytes) {
                    Ok(value) => {
                        match serde_json::from_value::<crate::signals::ElohimContentSignal>(
                            value.clone(),
                        ) {
                            Ok(content) => handler(content),
                            Err(e) => {
                                debug!(
                                    error = %e,
                                    value = %value,
                                    "Received app signal not matching ElohimContentSignal — ignoring"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, "Failed to msgpack-decode app signal");
                    }
                }
            }
        })
        .await
}
```

- [ ] **Step 2: Compile-check**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -10
```
Expected: clean compile.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/hc_client.rs
git commit -m "feat(recovery-m4): hc_client::subscribe_elohim_content_signals"
```

---

#### Task 12: Wire the subscriber + dispatcher in `main.rs`

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs` (next to the InfrastructureSignal block at ~line 606)

- [ ] **Step 1: Append subscription block**

In `main.rs`, immediately after the `InfrastructureSignal` subscriber tokio::spawn (the `if let Some(subscriber_pool) = db_pool.clone()` block ending near line 643), add a sibling block:
```rust
if let Some(subscriber_pool) = db_pool.clone() {
    let hc_sub = hc.clone();
    tokio::spawn(async move {
        let pool = subscriber_pool;
        let handle_id = hc_sub
            .subscribe_elohim_content_signals(
                move |signal: elohim_storage::signals::ElohimContentSignal| {
                    match pool.get() {
                        Ok(mut conn) => {
                            if let Err(e) =
                                elohim_storage::services::elohim_content_dispatcher::dispatch(
                                    &mut conn, &signal,
                                )
                            {
                                warn!(
                                    kind = %signal.content_type,
                                    error = %e,
                                    "ElohimContentSignal dispatch failed"
                                );
                            }
                        }
                        Err(e) => warn!(
                            error = %e,
                            "Failed to acquire DB connection for elohim content dispatch"
                        ),
                    }
                },
            )
            .await;
        info!(
            subscription_id = %handle_id,
            "ElohimContentSignal subscriber registered (dispatches to AttestationProjector + RecoveryFlowProjector)"
        );
    });
} else {
    warn!("ElohimContentSignal subscriber disabled: shared DB pool unavailable");
}
```

Verify the `elohim_content_dispatcher` and `recovery_flow_projector` modules are exported through `services::mod`. If not, fix the visibility.

- [ ] **Step 2: Compile-check**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -10
```
Expected: clean compile.

- [ ] **Step 3: Boot the node and verify the log line**

Run a brief startup against a local conductor (or use the cluster integration smoke):
```bash
cd elohim/elohim-storage && timeout 30 cargo run --bin elohim-storage 2>&1 | grep -i "ElohimContentSignal subscriber"
```
Expected: one log line saying `ElohimContentSignal subscriber registered ...` (or the disabled-warning if no DB pool).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(recovery-m4): main.rs wires ElohimContentSignal subscriber + dispatcher"
```

---

#### Task 13: Bridge `create_recovery_request` and `create_self_revocation`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (functions at lines 2112 and 2199)

- [ ] **Step 1: Write a failing SweetTest**

In `elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs` (new file):
```rust
//! End-to-end harness for Stage 2: recovery-request and self-revocation
//! flow through the bridged elohim coordinator.

use crate::common::{spawn_two_agent_conductors, install_recovery_dnas};
use holochain_types::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn create_recovery_request_lands_governance_action_on_elohim_dna() {
    let (alice, _bob) = spawn_two_agent_conductors().await;
    install_recovery_dnas(&alice).await;

    // Build the M4-shaped input: human_id, custodian_cids, threshold.
    let input = serde_json::json!({
        "human_id": "human-alice",
        "custodian_cids": ["cid-bob", "cid-carol", "cid-dave"],
        "threshold": {"m": 2},
    });
    let cid: String = alice
        .call_imagodei("create_recovery_request", input)
        .await
        .expect("create_recovery_request bridged successfully");

    // Confirm the entry landed on the elohim DNA as a governance-action.
    let entry: serde_json::Value = alice
        .call_elohim("get_content_by_cid", cid.clone())
        .await
        .expect("entry resolves on elohim DNA");
    assert_eq!(entry["content_type"], "governance-action:recovery-request");

    // Confirm no entry was committed to the imagodei DHT with the legacy type.
    let legacy_lookup = alice.list_imagodei_entries_of_kind("RecoveryRequest").await;
    assert!(legacy_lookup.is_empty(), "no legacy RecoveryRequest entries");
}

#[tokio::test(flavor = "multi_thread")]
async fn create_self_revocation_lands_governance_action_on_elohim_dna() {
    // Analogous assertions for the self-revocation path.
}
```

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1200 cargo test recovery_flows::create_recovery_request_lands 2>&1 | tail -20
```
Expected: FAIL — current `create_recovery_request` still writes `EntryTypes::RecoveryRequest`.

- [ ] **Step 2: Rewrite `create_recovery_request` (lib.rs:2112)**

Replace the `create_entry(&EntryTypes::RecoveryRequest(request.clone()))?` block with a call to the existing `call_elohim_propose_governance_action` bridge helper (already in scope at line 716):

```rust
let metadata = serde_json::json!({
    "human_id": &request.human_id,
    "custodian_cids": &request.custodian_cids,
    "trigger_type": &request.trigger_type,
    "session_pubkey": &request.session_pubkey,
});
let bridge_input = ConsolidatedProposeGovernanceActionInput {
    governance_kind: "governance-action:recovery-request".into(),
    subject_cid: request.human_id.clone(),
    title: format!("Recovery request for {}", request.human_id),
    description: Some(format!("trigger_type={}", request.trigger_type)),
    reach: "intimate".into(),
    threshold: serde_json::json!({"m": request.required_votes}),
    eligibility_predicate: Some(serde_json::json!({
        "type": "manifest-defined",
        "manifest_ref": "imagodei:custodian-eligibility-v1"
    })),
    ballot_format: "approve-reject".into(),
    closes_at: request.closes_at.clone(),
    parameters: Some(metadata),
};
let consolidated = call_elohim_propose_governance_action(bridge_input)?;
let recovery_request_cid = consolidated.cid;
```

Remove the `EntryTypes::RecoveryRequest` create_entry call and any post-commit signal that referenced the imagodei-local action hash. Update the return type to surface `recovery_request_cid: String` (where the function previously returned an `ActionHash`). Update all callers accordingly.

Apply the same pattern to `create_self_revocation` (lib.rs:2199), passing `governance_kind = "governance-action:key-revocation"` and the appropriate metadata fields.

- [ ] **Step 3: Re-run SweetTest**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1200 cargo test recovery_flows 2>&1 | tail -30
```
Expected: both Stage 2 tests PASS.

- [ ] **Step 4: Run a full regression**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1800 cargo test recovery_m3 recovery_m4 attestation_coordinator recovery_flows 2>&1 | tail -40
```
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs elohim/holochain/tests/sweettest/src/tests/mod.rs
git commit -m "feat(recovery-m4): bridge create_recovery_request and create_self_revocation to consolidated Content"
```

---

#### Task 14: Bridge `create_revocation_request`, `submit_revocation_vote`, `IdentityFreeze`, and `submit_specialist_revocation`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (functions near lines 2341 and around `IdentityFreeze` creation; grep for `EntryTypes::IdentityFreeze` to find sites)
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs` (line 201 — `create_entry(&EntryTypes::KeyRevocation(...))`)

- [ ] **Step 1: Write failing SweetTests**

In `recovery_flows.rs`, add:
- `create_revocation_request_lands_governance_action_on_elohim_dna`
- `submit_revocation_vote_emits_attestation_revocation_vote`
- `identity_freeze_creation_lands_governance_action_identity_freeze`
- `submit_specialist_revocation_lands_governance_action_key_revocation`

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1500 cargo test recovery_flows 2>&1 | tail -30
```
Expected: 4 new tests FAIL.

- [ ] **Step 2: Apply the bridge pattern from Task 13 to each function**

For each bridged function:
- `create_revocation_request` (lib.rs:2341): `call_elohim_propose_governance_action` with `governance_kind = "governance-action:key-revocation"`.
- `submit_revocation_vote`: `call_elohim_issue_attestation` with `attestation_kind = "attestation:revocation-vote"` and `parent_governance_action_cid` populated.
- `IdentityFreeze` creation: `call_elohim_propose_governance_action` with `governance_kind = "governance-action:identity-freeze"` and a single-step threshold (`m=1` for derived-on-quorum semantics — confirm against the Stage 1 audit findings).
- `submit_specialist_revocation.rs:201`: replace the inline `create_entry` with a bridge call; thread the bridge helper into the file via `use crate::call_elohim_propose_governance_action` (or extract the helpers to a shared module if they're currently buried inside `lib.rs`).

- [ ] **Step 3: Re-run SweetTests**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1500 cargo test recovery_flows 2>&1 | tail -30
```
Expected: all PASS.

- [ ] **Step 4: Verify legacy entry types have no remaining creators**

Run:
```bash
grep -rn "create_entry(&EntryTypes::RecoveryRequest\|create_entry(&EntryTypes::KeyRevocation\|create_entry(&EntryTypes::IdentityFreeze" elohim/holochain/
```
Expected: zero hits.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/dna/imagodei/zomes/imagodei/src/submit_specialist_revocation.rs elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs
git commit -m "feat(recovery-m4): bridge remaining recovery primitives (revocation-request, revocation-vote, identity-freeze, specialist-revocation)"
```

---

#### Task 15: Remove the stage-G-followup TODO block and unused entry-type definitions

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (TODO block around lines 2925–2940)
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/` (delete `RecoveryRequest`, `KeyRevocation`, `IdentityFreeze` entry definitions if no longer referenced)

- [ ] **Step 1: Delete the TODO block**

In `lib.rs`, locate the `TODO(stage-G-followup)` comment block and remove all 12 lines (the comment block above `submit_intimate_witness`). The fact that the work landed is captured in commit history; the comment is now stale.

- [ ] **Step 2: Check whether legacy entry types are still referenced anywhere**

Run:
```bash
grep -rn "EntryTypes::RecoveryRequest\|EntryTypes::KeyRevocation\|EntryTypes::IdentityFreeze\b" elohim/holochain/dna/imagodei/
```

- [ ] **Step 3: If unreferenced, remove from integrity zome**

Edit `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (or wherever `EntryTypes` enum is defined) to remove the three variants. Also remove the struct definitions (`RecoveryRequest`, `KeyRevocation`, `IdentityFreeze`) if they're now dead code.

If any of the three is still referenced by a test or compat shim, leave it and add a `#[deprecated]` attribute pointing at the new pattern; record the holdout in a follow-up issue.

- [ ] **Step 4: Build the WASM DNA — clean**

Run:
```bash
cd elohim/holochain/dna/imagodei && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --target wasm32-unknown-unknown --release 2>&1 | tail -10
```
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/
git commit -m "chore(recovery-m4): remove stage-G-followup TODO and unused legacy entry types"
```

---

#### Task 16: Add `governance-action:identity-freeze` to imagodei manifest

**Files:**
- Modify: `elohim/sdk/domains/imagodei/manifest.json`

- [ ] **Step 1: Add the entry**

In `elohim/sdk/domains/imagodei/manifest.json`, in the `"governance-actions"` block (after the existing `"governance-action:identity-challenge"` entry), append:
```json
"governance-action:identity-freeze": {
  "description": "Effective identity freeze applied after challenge-support quorum is reached. Records the freeze as a notarized governance event so freeze-floor gates and operator dashboards have a single source of truth.",
  "child_attestation_kind": "attestation:identity-freeze",
  "default_ballot_format": "approve-reject"
}
```

- [ ] **Step 2: Run manifest schema validation**

Run:
```bash
pnpm run schema:test 2>&1 | tail -15
```
Expected: all assertions PASS.

- [ ] **Step 3: Regenerate domain types**

Run:
```bash
pnpm run imagodei:codegen 2>&1 | tail -10
```
Expected: clean codegen. Inspect `app/elohim-app/src/app/imagodei/generated/manifest-types.ts` to confirm the new kind appears.

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/imagodei/manifest.json app/elohim-app/src/app/imagodei/generated/
git commit -m "feat(recovery-m4): declare governance-action:identity-freeze in imagodei manifest"
```

---

### Stage 3 — Producer-side signal emission (M4 ↔ EPR W2B convergence)

#### Task 17: Confirm DNA emits the four signals against existing schemas

**Files:**
- Modify (read only first): `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` and `submit_specialist_revocation.rs` — find post-commit `emit_signal` sites
- Modify if drift: any payload that diverges from the schemas at `elohim/sdk/schemas/v1/dna-signals/`

- [ ] **Step 1: Grep every `emit_signal` for the four signal kinds**

Run:
```bash
grep -rn "emit_signal" elohim/holochain/dna/imagodei/zomes/imagodei/src/ | grep -iE "key.?rotation|key.?revocation|agent.?peer.?binding|revocation.?attestation"
```
Expected: at least one emission point per signal kind. Record the sites in `genesis/docs/plans/2026-05-15-recovery-m4-stage3-emission-audit.md`.

- [ ] **Step 2: Diff each emission payload against the schema**

For each emission site, compare the struct fields against:
- `elohim/sdk/schemas/v1/dna-signals/key-rotation.schema.json`
- `elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json`
- `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json`
- `elohim/sdk/schemas/v1/dna-signals/revocation-attestation.schema.json`

Record any drift in the audit file. If the only drift is field-naming (snake_case vs camelCase via serde rename), that's handled by serde — leave as is.

- [ ] **Step 3: Fix any genuine drift**

If a field is missing or the type is wrong, add it. **CRITICAL (D2):** do NOT add a `contentEnvelope` field to `RevocationAttestation` — the duality decision says the slim payload stands. If the audit shows the emission is already inlining envelope fields, REMOVE them.

- [ ] **Step 4: Re-run schema validation**

Run:
```bash
pnpm run schema:test 2>&1 | tail -10
pnpm run schema:codegen:rs 2>&1 | tail -10
```
Expected: clean.

- [ ] **Step 5: Commit (only if drift was found)**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/
git commit -m "fix(recovery-m4): align DNA signal payloads with dna-signals/* schemas"
```

If no drift, commit just the audit doc:
```bash
git add genesis/docs/plans/2026-05-15-recovery-m4-stage3-emission-audit.md
git commit -m "docs(recovery-m4): stage-3 signal emission audit (no drift found)"
```

---

#### Task 18: Cross-stack integration test — M4 producer + EPR W2B consumer

**Files:**
- Create: `elohim/elohim-storage/tests/recovery_signal_loop.rs`

- [ ] **Step 1: Write the failing integration test**

Create `elohim/elohim-storage/tests/recovery_signal_loop.rs`:
```rust
//! Cross-stack: drive a KeyRevocation through the elohim DNA, capture the
//! KeyRevocation + RevocationAttestation DNA signals, project them via the
//! ElohimContentSignal dispatcher, and assert that EPR W2B's IntegrityNotify
//! handler at `epr_atom_service.rs:340–384` receives a structurally-valid
//! payload.

use elohim_storage::services::elohim_content_dispatcher;

#[tokio::test(flavor = "multi_thread")]
async fn key_revocation_round_trips_through_signal_loop() {
    // 1. Spawn a SweetConductor with elohim + imagodei DNAs installed.
    // 2. Trigger create_revocation_request → expect KeyRevocation signal.
    // 3. Trigger submit_revocation_vote × 2 → expect RevocationAttestation
    //    signals; threshold reached on second vote.
    // 4. Project all signals via ElohimContentSignal pathway.
    // 5. Assert: key_revocations row exists with threshold_reached = 1,
    //    effective_at populated, derived_compromise_at = None (consumer-side).
    // 6. Assert: EPR W2B's IntegrityNotify::KeyRevocation handler observed
    //    the producer's payload (via a test-only channel injected in
    //    epr_atom_service.rs#KeyRotation arm — see existing pattern).
}
```

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test --test recovery_signal_loop 2>&1 | tail -10
```
Expected: FAIL (test is stubbed; harness imports need to land).

- [ ] **Step 2: Implement the test against the existing SweetTest harness**

Pattern after `elohim/holochain/tests/sweettest/src/tests/attestation_coordinator.rs` — it already wires the consolidated coordinator. The new test reuses the same conductor setup but drives the create-revocation path end-to-end, captures the AppSignals via `SweetConductor::signal_stream`, and feeds them to the dispatcher.

- [ ] **Step 3: Run — must pass**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test --test recovery_signal_loop 2>&1 | tail -15
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/tests/recovery_signal_loop.rs
git commit -m "test(recovery-m4): cross-stack integration — KeyRevocation signal loop (producer ↔ EPR W2B consumer)"
```

---

### Stage 4 — Shamir as a fully optional layer

#### Task 19: Swarm-wire `ShamirShareCodec` into `ElohimStorageBehaviour`

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs`
- Modify: `elohim/elohim-storage/src/p2p/shamir_transport.rs` (remove TODO comment)

- [ ] **Step 1: Write the failing test**

Add to `elohim/elohim-storage/src/p2p/shamir_transport.rs` tests:
```rust
#[tokio::test]
async fn shamir_protocol_negotiates_between_two_peers() {
    use crate::p2p::test_helpers::spawn_two_peers;
    let (peer_a, peer_b) = spawn_two_peers().await;
    let req = ShamirShareRequest {
        recovery_governance_action_cid: "uhCkkAbcdef".into(),
        custodian_cid: "uhCqkXyz".into(),
    };
    let resp = peer_a
        .request_shamir_share(peer_b.peer_id(), req)
        .await
        .expect("shamir protocol negotiated and round-tripped");
    assert!(!resp.share_data.is_empty() || resp.signature.len() == 64);
}
```

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test p2p::shamir_transport::shamir_protocol_negotiates 2>&1 | tail -10
```
Expected: FAIL — `request_shamir_share` doesn't exist on the test helper because the protocol isn't on the swarm.

- [ ] **Step 2: Add field + import to `behaviour.rs`**

In `elohim/elohim-storage/src/p2p/behaviour.rs`:

Imports near line 25:
```rust
use super::shamir_transport::{ShamirShareCodec, ShamirShareProtocol, ShamirShareRequest, ShamirShareResponse};
```

Inside `ElohimStorageBehaviour` (after the `trust_protocol` field at line 87):
```rust
/// Request-response for Shamir share delivery (`/elohim/shamir-share/1.0.0`).
///
/// Per D4 (manifest-declared discovery): the recovery agent dials specific
/// custodian PeerIds resolved from the DHT custody manifest; no gossipsub
/// capability scan. Authorization lives on the DHT as
/// `attestation:recovery-approval`.
pub shamir_share_protocol: RequestResponse<ShamirShareCodec>,
```

`ElohimStorageBehaviourEvent` (after `TrustProtocol` variant at line 135):
```rust
/// Shamir share protocol event (`/elohim/shamir-share/1.0.0`).
ShamirShareProtocol(request_response::Event<ShamirShareRequest, ShamirShareResponse>),
```

`From` impl (after the TrustProtocol `From` at line 230):
```rust
impl From<request_response::Event<ShamirShareRequest, ShamirShareResponse>>
    for ElohimStorageBehaviourEvent
{
    fn from(event: request_response::Event<ShamirShareRequest, ShamirShareResponse>) -> Self {
        Self::ShamirShareProtocol(event)
    }
}
```

Inside `ElohimStorageBehaviour::new` (after the `trust_protocol` build block near line 377):
```rust
let shamir_share_protocol = RequestResponse::new(
    [(ShamirShareProtocol, ProtocolSupport::Full)],
    request_response::Config::default().with_request_timeout(config.request_timeout),
);
```

And add `shamir_share_protocol,` to the `Self { ... }` literal at the end of `new()`.

- [ ] **Step 3: Remove the TODO in `shamir_transport.rs`**

In `shamir_transport.rs`, delete lines 18–25 (the `TODO(G.1-swarm-wiring)` comment block).

- [ ] **Step 4: Compile**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -15
```
Expected: clean.

- [ ] **Step 5: Commit (intermediate — mod.rs match arm follows in Task 20)**

```bash
git add elohim/elohim-storage/src/p2p/behaviour.rs elohim/elohim-storage/src/p2p/shamir_transport.rs
git commit -m "feat(recovery-m4): register ShamirShareCodec with ElohimStorageBehaviour"
```

---

#### Task 20: Swarm event-loop match arm + custodian dial logic

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (after the TrustProtocol match arms near line 4054)

- [ ] **Step 1: Write the failing test (continued)**

Same test as Task 19 Step 1 — it still fails because the request-response event isn't handled.

- [ ] **Step 2: Add the match arm**

In `p2p/mod.rs`, after the `TrustProtocol` ResponseSent arm (around line 4054), insert:
```rust
// === Shamir share protocol events (/elohim/shamir-share/1.0.0) ===
behaviour::ElohimStorageBehaviourEvent::ShamirShareProtocol(
    request_response::Event::Message { peer, message },
) => match message {
    request_response::Message::Request { request, channel, .. } => {
        debug!(
            peer = %peer,
            governance_action = %request.recovery_governance_action_cid,
            "Received Shamir share request"
        );
        // Defer to the share-custodian responder service. The responder
        // verifies (a) the recovery-request is open on the DHT, (b) the
        // custodian_cid matches our identity, and (c) an attestation:
        // recovery-approval exists for the requesting agent. Only then
        // does it return the encrypted share.
        let pool = self.db_pool.clone();
        let identity = self.identity.clone();
        let mut swarm = self.swarm.write().await;
        let response = crate::services::share_custodian_responder::respond(
            &request,
            pool.as_ref(),
            &identity,
        )
        .await
        .unwrap_or_else(|reason| {
            warn!(peer = %peer, reason = %reason, "Shamir share request denied");
            // No error response in band — surface as a transport-level failure
            // so we don't leak custodian-set enumeration. Drop the channel.
            // (Caller will see OutboundFailure::Timeout.)
            shamir_transport::ShamirShareResponse {
                share_data: Vec::new(),
                share_index: 0,
                attestation_cid: String::new(),
                signature: Vec::new(),
            }
        });
        if let Err(e) = swarm
            .behaviour_mut()
            .shamir_share_protocol
            .send_response(channel, response)
        {
            warn!(peer = %peer, error = ?e, "Failed to send Shamir share response");
        }
    }
    request_response::Message::Response { response, request_id, .. } => {
        debug!(
            peer = %peer,
            request_id = ?request_id,
            attestation = %response.attestation_cid,
            share_index = response.share_index,
            "Received Shamir share response"
        );
        // The pending response is keyed by request_id in
        // self.pending_shamir_responses (sibling of pending_view_federations).
        if let Some(tx) = self.pending_shamir_responses.write().await.remove(&request_id) {
            let _ = tx.send(Ok(response));
        }
    }
},
behaviour::ElohimStorageBehaviourEvent::ShamirShareProtocol(
    request_response::Event::OutboundFailure { peer, request_id, error, .. },
) => {
    debug!(peer = %peer, request_id = ?request_id, error = ?error, "Shamir share outbound failure");
    if let Some(tx) = self.pending_shamir_responses.write().await.remove(&request_id) {
        let _ = tx.send(Err(FederationError::OutboundFailure(format!("{error:?}"))));
    }
}
behaviour::ElohimStorageBehaviourEvent::ShamirShareProtocol(
    request_response::Event::InboundFailure { peer, error, .. },
) => {
    debug!(peer = %peer, error = ?error, "Shamir share inbound failure");
}
behaviour::ElohimStorageBehaviourEvent::ShamirShareProtocol(
    request_response::Event::ResponseSent { peer, .. },
) => {
    debug!(peer = %peer, "Shamir share response sent");
}
```

Also add the `pending_shamir_responses` field to the swarm struct (next to `pending_view_federations` at line 504) and initialize it in `new()`. Add the `request_shamir_share(peer_id, request)` method on `P2PHandle` (or wherever `request_view_federation` is defined — same pattern).

- [ ] **Step 3: Stub `share_custodian_responder::respond`**

Create `elohim/elohim-storage/src/services/share_custodian_responder.rs` with the verification skeleton. For now, return a hard-coded valid response shape for tests; the real share-store integration lands in Task 21.

- [ ] **Step 4: Run the test**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test p2p::shamir_transport::shamir_protocol_negotiates 2>&1 | tail -15
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/services/share_custodian_responder.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(recovery-m4): swarm event loop handles ShamirShareProtocol with manifest-declared custodian dial"
```

---

#### Task 21: `ShareAssembler` primitive — reconstruct seed from quorum of shares

**Files:**
- Create: `elohim/elohim-storage/src/services/share_assembler.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
//! Reconstructs a Shamir-split seed from a quorum of verified
//! ShamirShareResponse payloads.

use crate::p2p::shamir_transport::ShamirShareResponse;

pub struct ShareAssembler {
    threshold: usize,
    received: Vec<(u32, Vec<u8>)>, // (share_index, share_data)
}

impl ShareAssembler {
    pub fn new(threshold: usize) -> Self {
        Self { threshold, received: Vec::new() }
    }
    pub fn add(&mut self, response: &ShamirShareResponse) {
        if !self.received.iter().any(|(i, _)| *i == response.share_index) {
            self.received.push((response.share_index, response.share_data.clone()));
        }
    }
    pub fn quorum_reached(&self) -> bool {
        self.received.len() >= self.threshold
    }
    pub fn try_reconstruct(&self) -> Option<Vec<u8>> {
        if !self.quorum_reached() {
            return None;
        }
        // Real implementation: sharks::Sharks::recover() against received shares.
        // For the M4 acceptance test we round-trip via the `sharks` crate.
        let sharks = sharks::Sharks(self.threshold as u8);
        let shares: Vec<sharks::Share> = self
            .received
            .iter()
            .filter_map(|(_, data)| sharks::Share::try_from(data.as_slice()).ok())
            .collect();
        sharks.recover(&shares).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembler_recovers_seed_at_threshold() {
        let secret = b"my-32-byte-secret-key-for-testing".to_vec();
        let sharks = sharks::Sharks(3);
        let dealer = sharks.dealer(&secret);
        let shares: Vec<_> = dealer.take(5).collect();
        let mut a = ShareAssembler::new(3);
        for (i, share) in shares.iter().enumerate().take(3) {
            a.add(&ShamirShareResponse {
                share_data: Vec::from(share),
                share_index: (i + 1) as u32,
                attestation_cid: format!("attest-{i}"),
                signature: vec![0; 64],
            });
        }
        assert!(a.quorum_reached());
        let reconstructed = a.try_reconstruct().expect("recover");
        assert_eq!(reconstructed, secret);
    }
}
```

Add `sharks = "0.5"` to `elohim/elohim-storage/Cargo.toml` if not present.

- [ ] **Step 2: Run — must pass**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test services::share_assembler 2>&1 | tail -10
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/services/share_assembler.rs elohim/elohim-storage/src/services/mod.rs elohim/elohim-storage/Cargo.toml
git commit -m "feat(recovery-m4): ShareAssembler reconstructs seed from quorum of verified shares"
```

---

#### Task 22: Custody manifest discriminator + custody-setup ceremony

**Files:**
- Modify: `elohim/sdk/domains/imagodei/manifest.json`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (new `create_shamir_custody_setup` extern)

- [ ] **Step 1: Declare the manifest entry**

In `manifest.json`, append to `"governance-actions"`:
```json
"governance-action:shamir-custody-setup": {
  "description": "Records the custody manifest for an agent's Shamir-split seed: which CIDs hold which share indices, the (m,n) threshold, and the validity horizon. Committed at recovery-setup time so the substrate can deterministically dial custodians later, without depending on live capability advertisements.",
  "child_attestation_kind": "attestation:identity-credential",
  "default_ballot_format": "approve-reject"
}
```

- [ ] **Step 2: Add `create_shamir_custody_setup` extern**

In `lib.rs`, add a new `#[hdk_extern]` function that takes `ShamirCustodySetupInput { human_id, threshold_m, threshold_n, custodian_cids, valid_until }` and bridges to the elohim coordinator with `governance_kind = "governance-action:shamir-custody-setup"`. The body mirrors `create_recovery_request` from Task 13 but writes a longer-lived setup record (not a recovery-window record).

- [ ] **Step 3: SweetTest**

In `recovery_flows.rs`, add a test that creates a custody setup, then drives a `create_recovery_request` followed by `ShareAssembler` reads via the swarm — confirm the custody manifest CIDs match the dial targets.

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1500 cargo test recovery_flows::custody_setup_drives_dial_list 2>&1 | tail -20
```
Expected: PASS.

- [ ] **Step 4: Run manifest validation + codegen**

Run:
```bash
pnpm run schema:test && pnpm run imagodei:codegen 2>&1 | tail -10
```
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/imagodei/manifest.json elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs app/elohim-app/src/app/imagodei/generated/
git commit -m "feat(recovery-m4): governance-action:shamir-custody-setup discriminator + create_shamir_custody_setup extern"
```

---

#### Task 23: Optionality enforcement — audit + assert recovery succeeds without Shamir

**Files:**
- Modify (read only first): `elohim/elohim-storage/src/services/recovery_*` and `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts`
- Modify if a hard dependency on Shamir exists: surface as a follow-up issue + add a feature flag

- [ ] **Step 1: Grep every site that mentions Shamir in the recovery completion path**

Run:
```bash
grep -rn "shamir\|Shamir\|share_assembler\|ShareAssembler" elohim/elohim-storage/src/services/ elohim/holochain/dna/imagodei/zomes/imagodei/src/
```
Record each site in `genesis/docs/plans/2026-05-15-recovery-m4-stage4c-audit.md`.

- [ ] **Step 2: Trace each site**

For each match, classify: **gating** (recovery would fail if Shamir wasn't satisfied) vs. **optional** (only runs when Shamir is configured). The acceptance bar is: **zero gating sites.**

- [ ] **Step 3: Refactor gating sites**

If any site gates the completion path on Shamir, wrap it in an `if custody_manifest.is_some()` check and document the optional-side-effect: a successful reconstruction emits an additional `attestation:shamir-reconstructed` (a new attestation subtype if not present; declare in the imagodei manifest under `attestations` with `revocable_by: ["governance"]`).

- [ ] **Step 4: Write the cross-mode integration test**

In `elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs`, add:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn recovery_completes_without_shamir_path() {
    // 1. Setup: human, 3 emergency contacts, NO governance-action:shamir-custody-setup committed.
    // 2. Drive intimate-quorum path → key rotation commits.
    // 3. Assert: no ShamirShareRequest dialled; recovery_flows row → Effective.
    // 4. Assert: no attestation:shamir-reconstructed exists.
}

#[tokio::test(flavor = "multi_thread")]
async fn recovery_completes_with_shamir_path() {
    // 1. Setup: human, 3 emergency contacts, governance-action:shamir-custody-setup
    //    declares 3 custodians at threshold (2, 3).
    // 2. Drive intimate-quorum path → key rotation commits.
    // 3. AND share assembler receives at least 2 shares → emits
    //    attestation:shamir-reconstructed.
    // 4. Assert: recovery_flows row → Effective, attestation row present.
}

#[tokio::test(flavor = "multi_thread")]
async fn recovery_completes_when_shamir_fails() {
    // 1. Same as above but custodians OFFLINE / share request times out.
    // 2. Assert: recovery_flows row STILL → Effective (Path A unblocked).
    // 3. Assert: NO attestation:shamir-reconstructed.
}
```

- [ ] **Step 5: Run all three**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 1800 cargo test recovery_flows::recovery_completes 2>&1 | tail -30
```
Expected: all 3 PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/ elohim/holochain/dna/imagodei/zomes/imagodei/src/ elohim/holochain/tests/sweettest/src/tests/recovery_flows.rs genesis/docs/plans/2026-05-15-recovery-m4-stage4c-audit.md
git commit -m "feat(recovery-m4): enforce Shamir optionality — recovery succeeds with or without share custody"
```

---

#### Task 24: Angular UX audit — never expose seed material; never force Path A/B choice

**Files:**
- Modify (read only first): `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts`, `app/elohim-app/src/app/imagodei/components/recovery-request/*`, `recovery-interview/*`

- [ ] **Step 1: Grep for seed/share/shamir/custodian leakage in the UI layer**

Run:
```bash
grep -rn "seed\|share\b\|shamir\|Shamir\|custodian\|reconstruct" app/elohim-app/src/app/imagodei/
```
Record findings in `genesis/docs/plans/2026-05-15-recovery-m4-stage4d-ui-audit.md`.

- [ ] **Step 2: Verify the grandma-standard contracts**

Confirm three properties from the UI:
1. **No seed material visible** — there is no HTML/template path that renders raw share bytes, seed bytes, or signing keys to a user-facing surface.
2. **No A/B path selector** — the user is never asked "do you want to do recovery with or without Shamir?" The substrate decides based on whether a custody manifest exists.
3. **Graceful degradation** — when custodians are offline, the UI shows progress through the intimate-quorum path without surfacing a Shamir error.

- [ ] **Step 3: Document any UI gaps**

If the audit surfaces a gap, write it as a separate follow-up backlog item in `genesis/data/timeline/backlog/`. Do NOT scope-creep into Angular components in this sprint (per kickoff "Out of scope" section).

- [ ] **Step 4: Commit**

```bash
git add genesis/docs/plans/2026-05-15-recovery-m4-stage4d-ui-audit.md genesis/data/timeline/backlog/
git commit -m "docs(recovery-m4): stage-4d Angular UX audit — grandma-standard confirmed"
```

---

### Stage 5 — a2o scenario `@wip` lift + `@recovery-shamir-optional`

#### Task 25: Lift `@wip` on `revocation-self.feature`

**Files:**
- Modify: `genesis/a2o/features/auth/recovery/revocation-self.feature` (line 50)

- [ ] **Step 1: Run the scenario first to confirm it now passes**

Run:
```bash
cd app/elohim-app && timeout 600 pnpm exec cucumber-js genesis/a2o/features/auth/recovery/revocation-self.feature 2>&1 | tail -30
```
Expected: the previously-`@wip` scenario "Matthew can initiate full recovery after revoking his only key" now passes (or, if a step def is missing, fail with a clear missing-step message — see Step 2).

- [ ] **Step 2: Resolve any missing step defs**

If steps are missing, locate the step def file under `app/elohim-app/src/app/imagodei/step-defs/` (or the project's a2o step-def layout) and add the minimal step. **Do not invent UI flows** — wire the existing recovery-coordinator service if a step needs to talk to the backend.

- [ ] **Step 3: Remove the `@wip` marker**

In `revocation-self.feature`, delete line 50 (the `@wip` line above "Scenario: Matthew can initiate full recovery after revoking his only key").

- [ ] **Step 4: Re-run, confirm green**

Run:
```bash
cd app/elohim-app && timeout 600 pnpm exec cucumber-js genesis/a2o/features/auth/recovery/revocation-self.feature 2>&1 | tail -20
```
Expected: all scenarios green.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/auth/recovery/revocation-self.feature app/elohim-app/src/app/imagodei/step-defs/
git commit -m "test(recovery-m4): lift @wip on revocation-self full-recovery-after-self-revoke scenario"
```

---

#### Task 26: New `@recovery-shamir-optional` feature

**Files:**
- Create: `genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature`

- [ ] **Step 1: Author the feature file**

```gherkin
@recovery-shamir-optional @recovery-m4
Feature: Recovery succeeds with or without Shamir share custody
  As Matthew, whose key has been compromised
  I want to recover my account through people who know me
  So that the cryptographic-proof channel is icing, not foundation

  Background:
    Given Matthew has three emergency contacts: Jessica, Adam, and Abby
    And Matthew's required intimate-quorum threshold is 2

  Scenario: Recovery succeeds without Shamir custody (Path A only)
    Given Matthew has NOT committed a governance-action:shamir-custody-setup
    When Matthew initiates a recovery request
    And Jessica and Adam each submit a recovery-approval attestation
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And no Shamir share request is dialled to any custodian
    And no attestation:shamir-reconstructed exists for Matthew

  Scenario: Recovery succeeds with Shamir custody (Path A + Path B)
    Given Matthew has committed a governance-action:shamir-custody-setup
      naming Jessica, Adam, and Abby as custodians with threshold (m=2, n=3)
    And Jessica and Adam are online
    When Matthew initiates a recovery request
    And Jessica and Adam each submit a recovery-approval attestation
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And the substrate dials Jessica and Adam over /elohim/shamir-share/1.0.0
    And the ShareAssembler reconstructs Matthew's seed
    And an attestation:shamir-reconstructed exists for Matthew

  Scenario: Recovery still succeeds when Shamir custodians are offline
    Given Matthew has committed a governance-action:shamir-custody-setup
      naming Jessica, Adam, and Abby as custodians with threshold (m=2, n=3)
    But Jessica, Adam, and Abby are all offline at recovery time
    When Matthew initiates a recovery request
    And the social-threshold attestations arrive from a separate quorum
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And no attestation:shamir-reconstructed exists for Matthew
    And Matthew is not informed of the Shamir attempt at all

  Scenario: Recovery never asks Matthew to choose Path A or Path B
    When Matthew initiates a recovery request
    Then the recovery UI does not present a Shamir-vs-social toggle
    And Matthew is never shown raw seed bytes or share bytes
```

- [ ] **Step 2: Run cucumber**

Run:
```bash
cd app/elohim-app && timeout 600 pnpm exec cucumber-js genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature 2>&1 | tail -30
```
Expected: at least some steps fail with missing-definition messages.

- [ ] **Step 3: Implement step defs**

Add step defs under `app/elohim-app/src/app/imagodei/step-defs/` (or the project's a2o step-def layout). Backed by:
- The recovery-coordinator service (already exists).
- A test-only flag/probe on `elohim-storage` that exposes "did any ShamirShareRequest dial occur during this scenario?" — implement via a test fixture inspecting the swarm event log or the `pending_shamir_responses` map.

- [ ] **Step 4: Run cucumber — all green**

Run:
```bash
cd app/elohim-app && timeout 1200 pnpm exec cucumber-js --tags @recovery-shamir-optional 2>&1 | tail -30
```
Expected: all four scenarios PASS.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature app/elohim-app/src/app/imagodei/step-defs/
git commit -m "test(recovery-m4): @recovery-shamir-optional cucumber feature (Path A, Path B, offline fallback)"
```

---

### Stage 6 — Branch retirement + final acceptance

#### Task 27: Retire `feature/recovery-m4-fast-path-revocation`

**Files:**
- (no file changes — repository state only)

- [ ] **Step 1: Confirm zero unique commits (defensive re-check)**

Run:
```bash
git log feature/recovery-m4-fast-path-revocation ^dev | head
```
Expected: empty output.

- [ ] **Step 2: Delete the local branch (`-d`, not `-D`)**

```bash
git branch -d feature/recovery-m4-fast-path-revocation
```
Expected: `Deleted branch feature/recovery-m4-fast-path-revocation (was 5fa8d621f).`

If git refuses, STOP and investigate — the brainstorm analysis was wrong; do not force.

- [ ] **Step 3: Delete the remote branch**

```bash
git push origin --delete feature/recovery-m4-fast-path-revocation
```
Expected: `- [deleted]   feature/recovery-m4-fast-path-revocation`.

- [ ] **Step 4: Verify**

Run:
```bash
git branch -a | grep recovery-m4-fast-path-revocation
```
Expected: no output.

- [ ] **Step 5: Note in commit log (no commit needed)**

Branch deletions are recorded in reflog; no commit. Update the sprint memory in the next memory ceremony.

---

#### Task 28: Sprint acceptance check

**Files:**
- (read only)

- [ ] **Step 1: Run the full elohim-holochain pipeline locally**

Run:
```bash
cd elohim/holochain/tests/sweettest && timeout 3000 cargo test attestation_coordinator recovery_m3 recovery_m4 recovery_flows 2>&1 | tail -40
```
Expected: all PASS.

- [ ] **Step 2: Run the elohim-storage test suite**

Run:
```bash
cd elohim/elohim-storage && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30
```
Expected: all PASS, including `services::recovery_flow_projector`, `services::elohim_content_dispatcher`, `services::share_assembler`, `p2p::shamir_transport`, and the new integration test `tests/recovery_signal_loop.rs`.

- [ ] **Step 3: Run schema validation + codegen freshness**

Run:
```bash
pnpm run schema:test
pnpm run schema:validate
pnpm run imagodei:codegen:verify
pnpm run schema:codegen:ts -- --verify
```
Expected: clean.

- [ ] **Step 4: Confirm zero TODO markers remain**

Run:
```bash
grep -rn "TODO(stage-G-followup)\|TODO(G.1-swarm-wiring)" elohim/
```
Expected: zero hits.

- [ ] **Step 5: Push and watch orchestrator dev**

```bash
git push origin dev
```

Watch the orchestrator dev pipeline. Acceptance bar: SUCCESS or UNSTABLE-not-regressed for 2 consecutive runs (one fresh trigger from this sprint's push). Per the kickoff prompt, do not declare the sprint complete until that bar is met.

- [ ] **Step 6: Mark the sprint memory**

In the next memory ceremony, update `project_attestation_consolidation_sprint_state.md` and add `project_recovery_m4_completion_sprint_state.md` recording: stage commits, acceptance evidence, the `governance-action:shamir-custody-setup` decision, and any UX gaps surfaced by Task 24 that need M5 attention.

---

## Self-Review

**Spec coverage:**
- Stage 1 (cross-DNA gate-reader migration) → Tasks 1–5 ✓
- Stage 2 (bridge create-side + sibling RecoveryFlowProjector + EPR W2D `key_revocations`) → Tasks 6–16 ✓
- Stage 3 (producer-side signal emission with duality for RevocationAttestation) → Tasks 17–18 ✓
- Stage 4a (swarm wiring) → Tasks 19–20 ✓
- Stage 4b (ShareAssembler + custody manifest) → Tasks 21–22 ✓
- Stage 4c (optionality enforcement) → Task 23 ✓
- Stage 4d (Angular UX audit) → Task 24 ✓
- Stage 5 (a2o @wip lift + `@recovery-shamir-optional`) → Tasks 25–26 ✓
- D3 (branch retirement) → Task 27 ✓
- Final acceptance check → Task 28 ✓

**Placeholder scan:** No "TBD" / "implement later" / "add validation" placeholders. Every step block contains executable commands or concrete code. Where coordinator-side functions are needed (`query_effective_revocation_for_key`, `query_effective_identity_freeze_for_human`, `get_content_by_cid`), Task 4 Step 2 calls out that they must be present and to add them inline if missing.

**Type consistency:** `RecoveryFlowRow` and `KeyRevocationRow` field names are identical across Task 6 (SQL), Task 7 (Diesel model + schema), Task 8 (projector), Task 9 (state-machine branches), Task 10 (dispatcher), and Task 18 (cross-stack test). `ShamirShareCodec` / `ShamirShareProtocol` / `ShamirShareRequest` / `ShamirShareResponse` names match `elohim/elohim-storage/src/p2p/shamir_transport.rs`. The bridge helpers (`call_elohim_propose_governance_action`, `call_elohim_issue_attestation`, `ConsolidatedProposeGovernanceActionInput`, `ConsolidatedIssueAttestationInput`) are named exactly as they exist in `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`.

---

## Execution notes

- Tasks 1–5 (Stage 1) have linear data flow: each task depends on the previous. Best dispatched single-agent sequentially.
- Tasks 6–9 (storage schema + projector skeleton) can run in parallel with Tasks 1–5 in a separate subagent, since they touch different files. Tasks 10–12 must follow Tasks 6–9 because the dispatcher and main.rs wiring depend on the projector existing.
- Tasks 13–16 (create-side bridging in imagodei) depend on Stage 1 completing because the gates need to be cross-DNA-aware first.
- Tasks 19–22 (Shamir swarm wiring + ShareAssembler) are independent of Stages 1–3 and can be dispatched in parallel from the start.
- Task 23 (optionality enforcement) depends on Tasks 13–16 (creates) AND Task 21 (assembler) AND Task 22 (custody manifest).
- Tasks 25–26 (a2o scenarios) depend on Stages 1–4 being complete.
- Task 27 (branch retirement) is operator-gated — execute only after Task 28 acceptance check passes.
