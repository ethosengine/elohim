# Recovery Protocol Phase 2 — Milestone M3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the end-to-end plumbing for recovery rotations: DNA coordinator correctness, libp2p mesh invitation substrate (gossipsub on the EPR-2C swarm), storage projection for witness accumulation, and a cross-cell test corpus. M3 is initiator-agnostic — it forecloses no future account/login layer decision.

**Architecture:** Three tiers, signal-driven, doorway-absent.
- **imagodei DNA** gets two new link types, a third coordinator function (`submit_intimate_witness`), bridging-field population in `create_recovery_request`, and a freeze-floor gate in `commit_key_rotation`. Signal enum gains a rich `IntimateWitnessSubmitted` variant.
- **elohim-storage** composes a new gossipsub behaviour onto the existing EPR-2C swarm; a `RecoveryInvitation` wire contract (MessagePack via rmp-serde, matching EPR-2C); a signal-bridge that publishes on `RecoveryRequestCreated`; a subscribe stub that logs; a new `recovery_witnesses` diesel table + projection off `IntimateWitnessSubmitted`; a new `RecoveryWitnessView` + JSON schema.
- **Tests** cover four sweettest scenarios + two a2o scenarios (shem cross-doorway topology).

**Tech Stack:** Rust (HDK, HDI, diesel, libp2p 0.54, rmp-serde, ts-rs), Holochain sweettest harness, Cucumber/Gherkin (a2o), JSON Schema Draft 2020-12.

**Design reference:** `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage-design.md`. When design and plan disagree on an implementation detail (e.g., the design says "CBOR via ciborium" but the plan uses "MessagePack via rmp-serde"), **the plan is authoritative** because it corrects design drift against actual repo conventions.

---

## Pre-flight — branch, worktree, baseline sanity

### Pre-flight Task: Branch setup

**Files:**
- No file changes. Git operations only.

- [ ] **Step 1: Create feature branch from current dev HEAD**

```bash
cd /projects/elohim
git fetch origin dev
git checkout dev
git pull --ff-only origin dev
git checkout -b feature/recovery-m3-coordinator
```

- [ ] **Step 2: Verify M2 baseline — imagodei integrity has the KeyRotation entry + recovery_v2.rs helpers**

Run:
```bash
grep -c "KeyRotation(KeyRotation)" elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
grep -c "check_freeze_floor_rules\|check_intimate_quorum_rules\|check_cryptographic_quorum_rules" elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs
```
Expected: first command prints `1`; second prints `3` or higher.

- [ ] **Step 3: Verify M2 baseline — storage already has `recovery_requests` table + signal handler**

Run:
```bash
grep -c "handle_recovery_v2_signal" elohim/elohim-storage/src/signals.rs
grep -c "recovery_requests (dht_anchor_hash)" elohim/elohim-storage/src/db/diesel_schema.rs
```
Expected: both print `1`.

- [ ] **Step 4: Build baseline compiles**

Run:
```bash
cd elohim/holochain/dna/imagodei && just check
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: both succeed without errors.

If either step 2/3 returns unexpected output, STOP. The M2 baseline assumed by this plan is not on the branch.

---

## Phase 1 — DNA: imagodei integrity zome (new link types + signal extension)

File Structure:
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` — add two link-type variants.
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` — extend `RecoveryV2Signal` enum.

### Task 1: Add `RecoveryRequestToHumanityWitness` and `RecoveryRequestToKeyStewardship` link types

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:1177-1181` (after the existing `// Recovery Protocol Phase 2` link block)

- [ ] **Step 1: Extend `LinkTypes` enum**

Append under the existing `// Recovery Protocol Phase 2` block (after `AgentToKeyRotation`):

```rust
    // Recovery Protocol Phase 2 — M3
    RecoveryRequestToHumanityWitness, // RecoveryRequest -> HumanityWitness (IntimateQuorum link)
    RecoveryRequestToKeyStewardship,  // RecoveryRequest -> KeyStewardship (CryptographicQuorum link; M3 registers type, no coordinator creates yet)
```

- [ ] **Step 2: Add link-validation stubs**

Locate the `validate_create_link` / `FlatOp::RegisterCreateLink` arm (search for `LinkTypes::AgentToKeyRotation` — the M3 arms go right after). Add:

```rust
                LinkTypes::RecoveryRequestToHumanityWitness => Ok(ValidateCallbackResult::Valid),
                LinkTypes::RecoveryRequestToKeyStewardship => Ok(ValidateCallbackResult::Valid),
```

Stage-1 structural acceptance: link validation returns `Valid`. Coordinator pre-commit gates are the authoritative check per `project_hdi_no_get_links_in_validators`.

- [ ] **Step 3: Build DNA**

Run:
```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: zero errors. Warnings acceptable.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
git commit -m "feat(imagodei-integrity): add RecoveryRequestToHumanityWitness + RecoveryRequestToKeyStewardship link types"
```

---

### Task 2: Extend `RecoveryV2Signal` enum with `IntimateWitnessSubmitted`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1528-1537` (existing `RecoveryV2Signal` enum)

- [ ] **Step 1: Extend the enum**

Replace the existing `RecoveryV2Signal` enum with the amended version:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RecoveryV2Signal {
    RecoveryRequestCreated {
        action_hash: ActionHash,
        request: RecoveryRequest,
    },
    IntimateWitnessSubmitted {
        action_hash: ActionHash,
        request_hash: ActionHash,
        witness: HumanityWitness,
        witness_agent_id: AgentPubKey,
    },
    KeyRotationCommitted {
        action_hash: ActionHash,
        rotation: KeyRotation,
    },
}
```

The serde tag remains `"type"` — matches the storage-side mirror in `elohim-storage/src/signals.rs`.

- [ ] **Step 2: Verify compiler is happy with existing emit sites**

The existing `emit_signal(RecoveryV2Signal::RecoveryRequestCreated { ... })` and `RecoveryV2Signal::KeyRotationCommitted { ... }` call-sites are unchanged shapes — no new fields. They must still compile.

Run:
```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): add IntimateWitnessSubmitted variant to RecoveryV2Signal"
```

---

## Phase 2 — DNA: coordinator logic (create_recovery_request bridging, freeze gate, submit_intimate_witness)

File Structure:
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` — all coordinator recovery functions live here (monolithic). No new file. Existing functions around lines 280-310 (`get_human_by_agent_key`), 1580-1617 (`create_recovery_request`), 1625-1670 (`commit_key_rotation`).

**Helper visibility note:** `check_intimate_quorum_rules`, `check_cryptographic_quorum_rules`, `check_freeze_floor_rules` and `RECOVERY_AUTHORITY_LAYERS` live in the **integrity** zome at `imagodei_integrity/src/recovery_v2.rs` and are already re-exported for coordinator use via the existing Cargo dependency (per M2 landed). If the import path is unclear, grep for existing `imagodei_integrity::` use-statements in the coordinator's `lib.rs` to copy the pattern.

### Task 3: `create_recovery_request` — populate `human_id` + `required_witness_count`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1580-1617` (existing `create_recovery_request`)

- [ ] **Step 1: Understand the existing bridging path**

Read lines 280-310 of the coordinator lib.rs (`get_human_by_agent_key`) to see the `AgentKeyToHuman` link-traversal idiom. The new resolver mirrors this shape but returns `String` (the human's id).

- [ ] **Step 2: Add private helper — `resolve_human_id_for_agent`**

Add above `create_recovery_request` (or adjacent to `get_human_by_agent_key`):

```rust
/// Resolve an `AgentPubKey` to the `human_id` String by traversing
/// `AgentKeyToHuman` link → Human entry → human.id. Returns a coordinator
/// error if no Human is bound to the given pubkey.
fn resolve_human_id_for_agent(agent_pubkey: &AgentPubKey) -> ExternResult<String> {
    let links = get_links(
        LinkQuery::try_new(agent_pubkey.clone(), LinkTypes::AgentKeyToHuman)?,
        GetStrategy::default(),
    )?;
    let first = links.first().ok_or(wasm_error!(WasmErrorInner::Guest(format!(
        "No Human bound to agent pubkey {:?}",
        agent_pubkey
    ))))?;
    let human_hash = first
        .target
        .clone()
        .into_entry_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "AgentKeyToHuman target is not an entry hash".into()
        )))?;
    let record = get(human_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Human entry missing".into())))?;
    let human: Human = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Human entry deserialize failed".into())))?;
    Ok(human.id)
}
```

Type note: if the existing `get_human_by_agent_key` uses `ActionHash` not `EntryHash`, adapt `into_entry_hash()` → `into_action_hash()` and `get(hash, ...)` accordingly. Follow the working idiom.

- [ ] **Step 3: Add private helper — `count_active_emergency_contacts`**

```rust
/// Count the human's active `HumanRelationship` entries where
/// `emergency_access_enabled = true`. Active means no revocation path applied;
/// the simple rule for M3 is "entry still exists and flag is true." Future
/// milestones may add revocation semantics.
fn count_active_emergency_contacts(human_id: &str) -> ExternResult<u32> {
    let anchor_hash = anchor_for("human_relationship_by_human", human_id)?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::HumanToHumanRelationship)?,
        GetStrategy::default(),
    )?;
    let mut count: u32 = 0;
    for link in links {
        let rel_hash = match link.target.clone().into_action_hash() {
            Some(h) => h,
            None => continue,
        };
        let Some(record) = get(rel_hash, GetOptions::default())? else { continue };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if rel.emergency_access_enabled {
            count += 1;
        }
    }
    Ok(count)
}
```

**If the link name `HumanToHumanRelationship` doesn't exist**, grep the coordinator for existing "emergency contact" iteration idioms (search for `emergency_access_enabled` — one reference at line 1861) and adapt the traversal to the actual link-type name. Do not invent a link type.

- [ ] **Step 4: Add compute helper — `compute_required_witness_count`**

```rust
/// Threshold formula per revised spec §5 / M3 design §4.2: `max(2, ceil(M/2) + 1)`.
fn compute_required_witness_count(active_emergency_contacts: u32) -> u32 {
    let m = active_emergency_contacts;
    let ceil_half_plus_one = (m + 1) / 2 + 1; // ceil(m/2) + 1 for u32
    std::cmp::max(2, ceil_half_plus_one)
}
```

- [ ] **Step 5: Unit-test the compute helper (fast feedback)**

Add a `#[cfg(test)] mod m3_unit_tests` block near the helper:

```rust
#[cfg(test)]
mod m3_witness_threshold_tests {
    use super::compute_required_witness_count;

    #[test]
    fn floor_at_two_when_no_contacts() {
        assert_eq!(compute_required_witness_count(0), 2);
    }

    #[test]
    fn floor_at_two_when_one_contact() {
        assert_eq!(compute_required_witness_count(1), 2);
    }

    #[test]
    fn three_contacts_yields_three() {
        // ceil(3/2) + 1 = 2 + 1 = 3
        assert_eq!(compute_required_witness_count(3), 3);
    }

    #[test]
    fn four_contacts_yields_three() {
        // ceil(4/2) + 1 = 2 + 1 = 3
        assert_eq!(compute_required_witness_count(4), 3);
    }

    #[test]
    fn five_contacts_yields_four() {
        // ceil(5/2) + 1 = 3 + 1 = 4
        assert_eq!(compute_required_witness_count(5), 4);
    }
}
```

- [ ] **Step 6: Run unit tests and verify PASS**

Run:
```bash
cd /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei && \
  cargo test --target wasm32-unknown-unknown --lib m3_witness_threshold_tests 2>&1 | tail -20
```

If wasm test target is not configured, fall back to `cargo test m3_witness_threshold_tests` with native target — the helper has no HDK dependencies.

Expected: 5 tests pass.

- [ ] **Step 7: Wire helpers into `create_recovery_request`**

Replace the existing body of `create_recovery_request` (lines 1580-1617) with:

```rust
#[hdk_extern]
pub fn create_recovery_request(
    input: CreateRecoveryRequestInput,
) -> ExternResult<RecoveryRequestOutput> {
    let now = sys_time()?;

    // M3: resolve human_id + compute required_witness_count
    let human_id = resolve_human_id_for_agent(&input.human_agent_pubkey)?;
    let contact_count = count_active_emergency_contacts(&human_id)?;
    let required_witness_count = compute_required_witness_count(contact_count);

    let request = RecoveryRequest {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey,
        hosting_doorway_pubkey: input.hosting_doorway_pubkey,
        proposed_authority: input.proposed_authority,
        request_nonce: input.request_nonce,
        human_id: Some(human_id.clone()),
        required_witness_count,
        created_at: now,
    };

    let action_hash = create_entry(&EntryTypes::RecoveryRequest(request.clone()))?;

    // M3 decision log #2: anchor on human_id (was: pubkey). See design §12.
    let anchor_hash = anchor_for("recovery_request", &human_id)?;
    create_link(
        anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToRecoveryRequest,
        (),
    )?;

    emit_signal(RecoveryV2Signal::RecoveryRequestCreated {
        action_hash: action_hash.clone(),
        request: request.clone(),
    })?;

    Ok(RecoveryRequestOutput {
        action_hash,
        request,
    })
}
```

**Anchor convention:** `StringAnchor("recovery_request", human_id)`. If the existing M2 code uses `agent_pubkey.to_string()`, the plan migrates to human_id. Preserve any existing pubkey-keyed anchor creation for back-compat if you find one; otherwise cut it.

- [ ] **Step 8: Build DNA + re-run unit tests**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: zero errors.

- [ ] **Step 9: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): create_recovery_request populates human_id + required_witness_count, anchors on human_id"
```

---

### Task 4: `commit_key_rotation` — freeze-floor gate

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1625-1670` (existing `commit_key_rotation`)

- [ ] **Step 1: Add helper — `collect_active_freezes_for_human`**

```rust
/// Traverse `HumanToFreeze` links for the given `human_id` and return all
/// `IdentityFreeze` entries with `is_active = true`. Used by the M3 freeze-
/// floor gate on `commit_key_rotation`.
fn collect_active_freezes_for_human(human_id: &str) -> ExternResult<Vec<IdentityFreeze>> {
    let anchor_hash = anchor_for("identity_freeze_by_human", human_id)?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::HumanToFreeze)?,
        GetStrategy::default(),
    )?;
    let mut freezes = Vec::new();
    for link in links {
        let Some(hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(hash, GetOptions::default())? else { continue };
        let Some(freeze): Option<IdentityFreeze> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if freeze.is_active {
            freezes.push(freeze);
        }
    }
    Ok(freezes)
}
```

If the actual anchor string / link name for `HumanToFreeze` differs, adapt by grepping for the M2 helper that already reads freezes.

- [ ] **Step 2: Insert pre-commit gate into `commit_key_rotation`**

Amend the existing function body. The gate runs BEFORE `create_entry(KeyRotation)`. The CryptographicQuorum exemption is explicit per design §4.3.

```rust
#[hdk_extern]
pub fn commit_key_rotation(input: CommitKeyRotationInput) -> ExternResult<KeyRotationOutput> {
    let now = sys_time()?;

    // M3: freeze-floor gate (skips for CryptographicQuorum per design §4.3).
    let is_cryptographic = matches!(
        &input.authority,
        RecoveryAuthority::CryptographicQuorum { .. }
    );
    if !is_cryptographic {
        // resolve human_id from the rotating pubkey (same path as create_recovery_request)
        let human_id = resolve_human_id_for_agent(&input.human_agent_pubkey)?;
        let active_freezes = collect_active_freezes_for_human(&human_id)?;
        let freeze_refs: Vec<&IdentityFreeze> = active_freezes.iter().collect();

        check_freeze_floor_rules(&input.authority, &human_id, &freeze_refs)
            .map_err(|reason| wasm_error!(WasmErrorInner::Guest(format!(
                "freeze-floor gate rejected rotation: {reason}"
            ))))?;
    }

    let rotation = KeyRotation {
        human_agent_pubkey: input.human_agent_pubkey.clone(),
        new_agent_pubkey: input.new_agent_pubkey.clone(),
        superseded_agent_pubkey: input.superseded_agent_pubkey,
        recovery_request_hash: input.recovery_request_hash,
        authority: input.authority,
        rotated_at: now,
    };

    let action_hash = create_entry(&EntryTypes::KeyRotation(rotation.clone()))?;

    // existing M2 link creation (HumanToCurrentAgent, AgentToKeyRotation) — unchanged
    // ...

    emit_signal(RecoveryV2Signal::KeyRotationCommitted {
        action_hash: action_hash.clone(),
        rotation: rotation.clone(),
    })?;

    Ok(KeyRotationOutput { action_hash, rotation })
}
```

**Helper contract:** `check_freeze_floor_rules` returns `Result<(), String>` per M2. If your check returns `ValidateCallbackResult` instead, adapt the mapping. Inspect the actual signature in `imagodei_integrity/src/recovery_v2.rs:300-335` first.

- [ ] **Step 3: Build DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: zero errors.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): commit_key_rotation runs freeze-floor pre-commit gate (CryptographicQuorum exempt)"
```

---

### Task 5: `submit_intimate_witness` — new coordinator function

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` — add the new function adjacent to `commit_key_rotation`.

- [ ] **Step 1: Define the input struct + output struct**

Add near other `CreateFooInput` definitions:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitIntimateWitnessInput {
    pub recovery_request_hash: ActionHash,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitIntimateWitnessOutput {
    pub action_hash: ActionHash,
    pub witness: HumanityWitness,
}
```

- [ ] **Step 2: Add the function**

```rust
#[hdk_extern]
pub fn submit_intimate_witness(
    input: SubmitIntimateWitnessInput,
) -> ExternResult<SubmitIntimateWitnessOutput> {
    // Gate 1: fetch the RecoveryRequest; must exist.
    let request_record = get(input.recovery_request_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "RecoveryRequest not found at given hash".into()
        )))?;
    let request: RecoveryRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "RecoveryRequest record has no entry".into()
        )))?;
    let human_id = request.human_id.clone().ok_or(wasm_error!(WasmErrorInner::Guest(
        "RecoveryRequest has no human_id (pre-M3 entry?)".into()
    )))?;

    // Gate 2: authorizer must be on an active emergency-enabled HumanRelationship
    // for this human. Prevents random agents piling on.
    let authorizer = agent_info()?.agent_initial_pubkey;
    if !is_active_emergency_contact(&human_id, &authorizer)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "authorizing agent is not on an active emergency contact of this human".into()
        )));
    }

    // Gate 3: dedupe — the authorizer cannot witness the same request twice.
    if has_existing_witness_for_request(&input.recovery_request_hash, &authorizer)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "this agent has already submitted a witness for this request".into()
        )));
    }

    // Commit the HumanityWitness.
    let now = sys_time()?;
    let witness = HumanityWitness {
        human_id: human_id.clone(),
        witness_agent_id: authorizer.to_string(),
        attestation_type: "intimate_recovery".into(),
        note: input.note,
        issued_at: now,
        revoked_at: None,
    };
    let action_hash = create_entry(&EntryTypes::HumanityWitness(witness.clone()))?;

    // Create the M3 link from the request to the witness.
    create_link(
        input.recovery_request_hash.clone(),
        action_hash.clone(),
        LinkTypes::RecoveryRequestToHumanityWitness,
        (),
    )?;

    // Emit rich signal.
    emit_signal(RecoveryV2Signal::IntimateWitnessSubmitted {
        action_hash: action_hash.clone(),
        request_hash: input.recovery_request_hash,
        witness: witness.clone(),
        witness_agent_id: authorizer,
    })?;

    Ok(SubmitIntimateWitnessOutput {
        action_hash,
        witness,
    })
}
```

**`HumanityWitness` field names:** the struct is defined at `imagodei_integrity/src/lib.rs:619`. If the actual fields differ from the plan (e.g., no `attestation_type`, or `revoked_at` is under a different name), adapt the constructor to match the real struct. Do not add fields to the DHT entry.

- [ ] **Step 3: Add `is_active_emergency_contact` helper**

```rust
/// Returns true if `authorizer_pubkey` holds an active `HumanRelationship`
/// with `emergency_access_enabled = true` where the counterparty is `human_id`.
fn is_active_emergency_contact(
    human_id: &str,
    authorizer_pubkey: &AgentPubKey,
) -> ExternResult<bool> {
    // Traverse `HumanToHumanRelationship` links for the target human.
    let anchor_hash = anchor_for("human_relationship_by_human", human_id)?;
    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::HumanToHumanRelationship)?,
        GetStrategy::default(),
    )?;
    for link in links {
        let Some(rel_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(rel_hash, GetOptions::default())? else { continue };
        let Some(rel): Option<HumanRelationship> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if !rel.emergency_access_enabled { continue; }
        // Match the counterparty to authorizer_pubkey via the Human<->Agent binding
        // used on HumanRelationship. Use whatever field actually exists on the
        // existing HumanRelationship struct (likely `counterparty_agent_id` or
        // similar — grep `struct HumanRelationship`).
        if relationship_counterparty_matches(&rel, authorizer_pubkey)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Encapsulates the field-name lookup for "which side of this relationship is
/// the authorizer?" so it can be adapted to the actual `HumanRelationship`
/// struct shape without leaking through the caller.
fn relationship_counterparty_matches(
    rel: &HumanRelationship,
    authorizer_pubkey: &AgentPubKey,
) -> ExternResult<bool> {
    // Adapt to match real struct fields. Commonly one of:
    //   rel.subject_agent_id == authorizer_pubkey.to_string()
    //   rel.counterparty_agent_id == authorizer_pubkey.to_string()
    //   rel.agent_b == *authorizer_pubkey
    // Inspect the struct at imagodei_integrity/src/lib.rs (grep "struct HumanRelationship").
    // Fallback conservative impl:
    let pk_str = authorizer_pubkey.to_string();
    // TODO in-task: replace this with the real field reference before committing.
    Ok(rel.subject_agent_id == pk_str || rel.counterparty_agent_id == pk_str)
}
```

**Action required during implementation:** read `HumanRelationship` struct definition in `imagodei_integrity/src/lib.rs` before finalizing `relationship_counterparty_matches`. The placeholder `subject_agent_id`/`counterparty_agent_id` is illustrative — replace with real field names. If the struct doesn't bind agent pubkeys directly (binds human_ids instead), resolve the authorizer's pubkey → human_id via `resolve_human_id_for_agent` and compare human_ids.

- [ ] **Step 4: Add `has_existing_witness_for_request` dedupe helper**

```rust
/// Returns true if `authorizer_pubkey` already has a `HumanityWitness` linked
/// from the given request (via `RecoveryRequestToHumanityWitness`).
fn has_existing_witness_for_request(
    request_hash: &ActionHash,
    authorizer_pubkey: &AgentPubKey,
) -> ExternResult<bool> {
    let links = get_links(
        LinkQuery::try_new(request_hash.clone(), LinkTypes::RecoveryRequestToHumanityWitness)?,
        GetStrategy::default(),
    )?;
    let pk_str = authorizer_pubkey.to_string();
    for link in links {
        let Some(w_hash) = link.target.clone().into_action_hash() else { continue };
        let Some(record) = get(w_hash, GetOptions::default())? else { continue };
        let Some(w): Option<HumanityWitness> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };
        if w.witness_agent_id == pk_str {
            return Ok(true);
        }
    }
    Ok(false)
}
```

- [ ] **Step 5: Build DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```
Expected: zero errors. If you see an error about `relationship_counterparty_matches` field access, that is the TODO from Step 3 — resolve it by reading the real `HumanRelationship` struct and adjusting fields before proceeding.

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): submit_intimate_witness coordinator function with membership + dedupe gates"
```

---

### Task 6: Pack DNA + smoke compile

**Files:** no file edits; build-only.

- [ ] **Step 1: Pack the DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just pack
```
Expected: produces the `.dna` artifact under `target/`.

- [ ] **Step 2: Verify storage-side compile still passes**

Storage has a mirror enum; if the DNA signal enum's new variant doesn't match the storage mirror yet, storage may still compile (because storage `RecoveryV2Signal` has no new variant yet). That's expected; storage task 10 below adds the mirror.

Run:
```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --release
```
Expected: zero errors.

- [ ] **Step 3: Commit any build artifacts gitignored out (no-op verification)**

```bash
git status
```
Expected: working tree clean.

---

## Phase 3 — Storage: schemas → views → migrations → signal projection

File Structure:
- Create: `elohim/sdk/schemas/v1/views/recovery-witness.schema.json` — wire-contract schema.
- Modify: `elohim/elohim-storage/src/views.rs` — add `RecoveryWitnessView` struct (ts-rs exports).
- Create: `elohim/elohim-storage/migrations/2026-04-24-000000_recovery_witnesses/up.sql` + `down.sql`.
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` — add `recovery_witnesses` table.
- Modify: `elohim/elohim-storage/src/db/models.rs` — add `NewRecoveryWitnessRow` + query struct.
- Create: `elohim/elohim-storage/src/db/recovery_witnesses.rs` — `upsert_recovery_witness`.
- Modify: `elohim/elohim-storage/src/db/mod.rs` — expose the new module.
- Modify: `elohim/elohim-storage/src/signals.rs` — extend `RecoveryV2Signal` mirror, dispatch handler.
- Modify: `elohim/elohim-storage/tests/schema_contract.rs` (and/or `schema_contract_recovery_v2.rs`) — add witness contract test.
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` — add `recovery-witness` to `INTERFACE_FILES`.

### Task 7: JSON schema — `recovery-witness.schema.json` (schema-first IoC)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/recovery-witness.schema.json`

- [ ] **Step 1: Write the schema file**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:view:recovery-witness",
  "title": "RecoveryWitnessView",
  "description": "Projection of imagodei HumanityWitness DHT entry when submitted under a RecoveryRequest (IntimateQuorum path). Source of truth: DHT.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "dhtAnchorHash",
    "recoveryRequestHash",
    "witnessAgentId",
    "humanId",
    "submittedAt"
  ],
  "properties": {
    "dhtAnchorHash": {
      "type": "string",
      "description": "ActionHash of the HumanityWitness entry (base64)."
    },
    "recoveryRequestHash": {
      "type": "string",
      "description": "ActionHash of the RecoveryRequest this witness is linked to (base64)."
    },
    "witnessAgentId": {
      "type": "string",
      "description": "AgentPubKey of the submitting authorizer (base64)."
    },
    "humanId": {
      "type": "string",
      "description": "Legacy String id of the human being witnessed for — mirrors the request's resolved human_id."
    },
    "note": {
      "type": ["string", "null"],
      "description": "Optional human-readable note from the authorizer (e.g., 'Sarah called me, recognized the dog's name')."
    },
    "submittedAt": {
      "type": "string",
      "description": "ISO-8601 timestamp of witness submission."
    }
  }
}
```

- [ ] **Step 2: Validate schema loads**

Run:
```bash
cd /projects/elohim && pnpm run schema:test
```
Expected: all assertions pass. If a new assertion is needed for witness, add it in the sibling test script.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/views/recovery-witness.schema.json
git commit -m "feat(schemas): add recovery-witness view schema (M3 wire contract)"
```

---

### Task 8: Diesel migration — `recovery_witnesses` table

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-24-000000_recovery_witnesses/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-24-000000_recovery_witnesses/down.sql`

- [ ] **Step 1: Write `up.sql`**

```sql
-- Recovery Protocol Phase 2 — M3 Witness Projection
-- Source of truth: DHT (imagodei DNA HumanityWitness entry linked from RecoveryRequest
--                       via RecoveryRequestToHumanityWitness).
-- This table is a read-optimized projection off RecoveryV2Signal::IntimateWitnessSubmitted.

CREATE TABLE IF NOT EXISTS recovery_witnesses (
    dht_anchor_hash       TEXT PRIMARY KEY NOT NULL,
    recovery_request_hash TEXT NOT NULL,
    witness_agent_id      TEXT NOT NULL,
    human_id              TEXT NOT NULL,
    note                  TEXT,
    submitted_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recovery_witnesses_request_hash
    ON recovery_witnesses(recovery_request_hash);

CREATE INDEX IF NOT EXISTS idx_recovery_witnesses_human_id
    ON recovery_witnesses(human_id);
```

- [ ] **Step 2: Write `down.sql`**

```sql
DROP INDEX IF EXISTS idx_recovery_witnesses_human_id;
DROP INDEX IF EXISTS idx_recovery_witnesses_request_hash;
DROP TABLE IF EXISTS recovery_witnesses;
```

- [ ] **Step 3: Run migration locally (verify it applies + reverts)**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  diesel migration run --database-url "sqlite::memory:" && \
  diesel migration revert --database-url "sqlite::memory:" && \
  diesel migration run --database-url "sqlite::memory:"
```
Expected: three successful operations. If `diesel-cli` isn't installed, `cargo install diesel_cli --no-default-features --features sqlite` first, or verify at build time when schema_contract tests run.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-24-000000_recovery_witnesses/
git commit -m "feat(storage): migration for recovery_witnesses projection table"
```

---

### Task 9: Diesel schema + models + queries

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (near line 1155, after the `key_rotations` table block)
- Modify: `elohim/elohim-storage/src/db/models.rs` (near line 2923, after `NewKeyRotationRow`)
- Create: `elohim/elohim-storage/src/db/recovery_witnesses.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Add table macro to `diesel_schema.rs`**

**Source-of-truth classification:** projection table. Canonical truth = DHT (imagodei `HumanityWitness` entry, linked from its `RecoveryRequest` via `RecoveryRequestToHumanityWitness`). This table is rebuildable from the DHT by replaying `RecoveryV2Signal::IntimateWitnessSubmitted`. Loss of this projection never loses the witness — the DHT retains it.

Append after the `key_rotations` block (around line 1167):

```rust
// Recovery Protocol Phase 2 — M3 witness projection.
// Source of truth: DHT. HumanityWitness entries (linked via
// RecoveryRequestToHumanityWitness) are the notary. This table is a
// read-optimized projection rebuildable from signal replay.
diesel::table! {
    recovery_witnesses (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        recovery_request_hash -> Text,
        witness_agent_id -> Text,
        human_id -> Text,
        note -> Nullable<Text>,
        submitted_at -> Text,
    }
}
```

Add to the `allow_tables_to_appear_in_same_query!` macro block near the file's end:
```rust
    recovery_witnesses,
```
(Insert alphabetically or adjacent to `recovery_requests`.)

- [ ] **Step 2: Add model + new-row struct to `models.rs`**

Append after `NewKeyRotationRow`:

```rust
use crate::db::diesel_schema::recovery_witnesses;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = recovery_witnesses)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RecoveryWitnessRow {
    pub dht_anchor_hash: String,
    pub recovery_request_hash: String,
    pub witness_agent_id: String,
    pub human_id: String,
    pub note: Option<String>,
    pub submitted_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = recovery_witnesses)]
pub struct NewRecoveryWitnessRow {
    pub dht_anchor_hash: String,
    pub recovery_request_hash: String,
    pub witness_agent_id: String,
    pub human_id: String,
    pub note: Option<String>,
    pub submitted_at: String,
}
```

- [ ] **Step 3: Create `db/recovery_witnesses.rs`**

```rust
//! Projection CRUD for recovery witness accumulation under an IntimateQuorum
//! recovery request. Idempotent on `dht_anchor_hash`.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::recovery_witnesses;
use crate::db::models::{NewRecoveryWitnessRow, RecoveryWitnessRow};
use crate::error::StorageError;

/// Insert-or-replace a witness row. Signal projection is idempotent.
pub fn upsert_recovery_witness(
    conn: &mut SqliteConnection,
    row: NewRecoveryWitnessRow,
) -> Result<(), StorageError> {
    use recovery_witnesses::dsl::*;
    diesel::insert_into(recovery_witnesses::table)
        .values(&row)
        .on_conflict(dht_anchor_hash)
        .do_update()
        .set((
            recovery_request_hash.eq(&row.recovery_request_hash),
            witness_agent_id.eq(&row.witness_agent_id),
            human_id.eq(&row.human_id),
            note.eq(&row.note),
            submitted_at.eq(&row.submitted_at),
        ))
        .execute(conn)
        .map_err(StorageError::from)?;
    Ok(())
}

/// Count witnesses submitted for a given recovery request (for UI "2 of 3" rendering).
pub fn count_witnesses_for_request(
    conn: &mut SqliteConnection,
    request_hash: &str,
) -> Result<i64, StorageError> {
    use recovery_witnesses::dsl::*;
    recovery_witnesses
        .filter(recovery_request_hash.eq(request_hash))
        .count()
        .get_result(conn)
        .map_err(StorageError::from)
}

/// List all witnesses for a request (ordered by submitted_at).
pub fn list_witnesses_for_request(
    conn: &mut SqliteConnection,
    request_hash: &str,
) -> Result<Vec<RecoveryWitnessRow>, StorageError> {
    use recovery_witnesses::dsl::*;
    recovery_witnesses
        .filter(recovery_request_hash.eq(request_hash))
        .order(submitted_at.asc())
        .select(RecoveryWitnessRow::as_select())
        .load(conn)
        .map_err(StorageError::from)
}
```

- [ ] **Step 4: Expose the module in `db/mod.rs`**

```rust
pub mod recovery_witnesses;
```

Place alphabetically or adjacent to `pub mod recovery_requests;`.

- [ ] **Step 5: Write a unit test for the CRUD**

Append to `recovery_witnesses.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-04-24-000000_recovery_witnesses/up.sql"
        ))
        .unwrap();
        // Also need recovery_requests table for FK reality, though we don't enforce
        // FKs at the projection layer. No-op here.
        conn
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = test_conn();
        let row = NewRecoveryWitnessRow {
            dht_anchor_hash: "W1".into(),
            recovery_request_hash: "R1".into(),
            witness_agent_id: "A1".into(),
            human_id: "H1".into(),
            note: Some("hi".into()),
            submitted_at: "2026-04-24T00:00:00Z".into(),
        };
        upsert_recovery_witness(&mut conn, row.clone()).unwrap();
        upsert_recovery_witness(&mut conn, row).unwrap();
        assert_eq!(count_witnesses_for_request(&mut conn, "R1").unwrap(), 1);
    }

    #[test]
    fn lists_in_submitted_order() {
        let mut conn = test_conn();
        for (w, t) in [("W1", "2026-04-24T00:00:02Z"), ("W2", "2026-04-24T00:00:01Z")] {
            upsert_recovery_witness(
                &mut conn,
                NewRecoveryWitnessRow {
                    dht_anchor_hash: w.into(),
                    recovery_request_hash: "R1".into(),
                    witness_agent_id: "A".into(),
                    human_id: "H".into(),
                    note: None,
                    submitted_at: t.into(),
                },
            )
            .unwrap();
        }
        let rows = list_witnesses_for_request(&mut conn, "R1").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dht_anchor_hash, "W2"); // earlier
        assert_eq!(rows[1].dht_anchor_hash, "W1"); // later
    }
}
```

- [ ] **Step 6: Run the unit tests**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recovery_witnesses
```
Expected: both tests pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/src/db/recovery_witnesses.rs \
        elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): recovery_witnesses diesel schema + CRUD + unit tests"
```

---

### Task 10: `RecoveryWitnessView` + schema contract test + ts-rs export

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (after the existing `KeyRotationView` around line 6691)
- Modify: `elohim/elohim-storage/tests/schema_contract_recovery_v2.rs` (existing M2 test file)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs:INTERFACE_FILES`

- [ ] **Step 1: Add `RecoveryWitnessView`**

Append after `KeyRotationView` in `views.rs`:

```rust
/// Source of truth: DHT (imagodei HumanityWitness entry linked via
/// RecoveryRequestToHumanityWitness from a RecoveryRequest). Projection
/// populated from `RecoveryV2Signal::IntimateWitnessSubmitted`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecoveryWitnessView {
    pub dht_anchor_hash: String,
    pub recovery_request_hash: String,
    pub witness_agent_id: String,
    pub human_id: String,
    pub note: Option<String>,
    pub submitted_at: String,
}

impl From<crate::db::models::RecoveryWitnessRow> for RecoveryWitnessView {
    fn from(r: crate::db::models::RecoveryWitnessRow) -> Self {
        Self {
            dht_anchor_hash: r.dht_anchor_hash,
            recovery_request_hash: r.recovery_request_hash,
            witness_agent_id: r.witness_agent_id,
            human_id: r.human_id,
            note: r.note,
            submitted_at: r.submitted_at,
        }
    }
}
```

- [ ] **Step 2: Regenerate ts-rs TypeScript**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings
```
Expected: `elohim/sdk/storage-client-ts/src/generated/RecoveryWitnessView.ts` appears (or similarly named). Inspect:
```bash
ls elohim/sdk/storage-client-ts/src/generated/ | grep -i witness
```

- [ ] **Step 3: Add schema-contract test**

Append to `elohim/elohim-storage/tests/schema_contract_recovery_v2.rs` (or create `schema_contract_recovery_witness.rs` if isolating is preferred). Follow the existing pattern used for `RecoveryRequestView` in that file.

```rust
#[test]
fn recovery_witness_view_matches_schema() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sdk/schemas/v1/views/recovery-witness.schema.json");
    let ts_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sdk/storage-client-ts/src/generated/RecoveryWitnessView.ts");
    assert_view_matches_schema("RecoveryWitnessView", &schema_path, &ts_path);
}
```

If `assert_view_matches_schema` doesn't exist, inspect the existing `recovery_request_view_matches_schema` test in the same file and copy its harness (likely it deserializes both the JSON schema and the emitted `.ts`, compares field sets + types).

- [ ] **Step 4: Add `recovery-witness` to codegen distribution**

Modify `elohim/sdk/schemas/scripts/codegen-ts.mjs`. Find the `INTERFACE_FILES` array (around lines 35-61) and add:

```javascript
  { src: 'views/recovery-witness.ts', dest: 'recovery-witness.ts' },
```

Place adjacent to `recovery-request.ts`.

- [ ] **Step 5: Run codegen + verify distribution**

```bash
cd /projects/elohim && pnpm run schema:codegen:ts
```
Expected: script succeeds. Inspect:
```bash
ls app/elohim-app/src/app/generated/ | grep -i recovery-witness
ls genesis/seeder/src/generated/ 2>/dev/null | grep -i recovery-witness
ls app/elohim-library/projects/elohim-service/src/generated/ | grep -i recovery-witness
```
Expected: new `recovery-witness.ts` (or similar) appears in all three locations.

- [ ] **Step 6: Run the schema contract test**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract_recovery_v2
```
Expected: pass. If fail, resolve field-name/type mismatch between schema and view struct before proceeding.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/views.rs \
        elohim/sdk/storage-client-ts/src/generated/ \
        elohim/elohim-storage/tests/schema_contract_recovery_v2.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        genesis/seeder/src/generated/
git commit -m "feat(storage): RecoveryWitnessView + schema contract + TS codegen distribution"
```

Note: `genesis/seeder/src/generated/` may not exist — include it only if it does.

---

### Task 11: Storage-side mirror of `IntimateWitnessSubmitted` + signal dispatch

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs` (lines 600-737 — the `RecoveryV2Signal` mirror + `handle_recovery_v2_signal` dispatcher)

- [ ] **Step 1: Add a witness payload struct to the mirror**

Append after `KeyRotationPayload` (around line 641):

```rust
/// Storage-side mirror of the HumanityWitness fields as they arrive in the
/// IntimateWitnessSubmitted signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanityWitnessPayload {
    pub human_id: String,
    pub witness_agent_id: String,
    pub attestation_type: String,
    pub note: Option<String>,
    /// Holochain Timestamp — serializes as microseconds i64.
    pub issued_at: serde_json::Value,
    pub revoked_at: Option<serde_json::Value>,
}
```

Adapt field names to match the **actual** DNA `HumanityWitness` struct; if the DNA struct has different fields, the storage-side mirror follows the DNA wire shape exactly.

- [ ] **Step 2: Extend the `RecoveryV2Signal` enum**

Replace:
```rust
pub enum RecoveryV2Signal {
    RecoveryRequestCreated { /* ... */ },
    KeyRotationCommitted { /* ... */ },
}
```

With:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RecoveryV2Signal {
    RecoveryRequestCreated {
        action_hash: String,
        request: RecoveryRequestPayload,
    },
    IntimateWitnessSubmitted {
        action_hash: String,
        request_hash: String,
        witness: HumanityWitnessPayload,
        witness_agent_id: String,
    },
    KeyRotationCommitted {
        action_hash: String,
        rotation: KeyRotationPayload,
    },
}
```

Serde tag stays `"type"` — matches DNA side exactly.

- [ ] **Step 3: Extend `handle_recovery_v2_signal` dispatcher**

Add a new match arm inside the existing `match signal { ... }` block:

```rust
        RecoveryV2Signal::IntimateWitnessSubmitted {
            action_hash,
            request_hash,
            witness,
            witness_agent_id,
        } => {
            let submitted_at = timestamp_to_iso(&witness.issued_at);
            let row = crate::db::models::NewRecoveryWitnessRow {
                dht_anchor_hash: action_hash,
                recovery_request_hash: request_hash,
                witness_agent_id,
                human_id: witness.human_id,
                note: witness.note,
                submitted_at,
            };
            crate::db::recovery_witnesses::upsert_recovery_witness(conn, row)
        }
```

- [ ] **Step 4: Add a unit test for the new dispatch**

Append inside the existing `#[cfg(test)] mod mishpat_signal_tests` (or create `recovery_v2_signal_tests` if scoping cleaner) in `signals.rs`:

```rust
#[test]
fn dispatches_intimate_witness_submitted() {
    let mut conn = setup_test_conn();
    let signal = RecoveryV2Signal::IntimateWitnessSubmitted {
        action_hash: "W1".into(),
        request_hash: "R1".into(),
        witness: HumanityWitnessPayload {
            human_id: "H1".into(),
            witness_agent_id: "A1".into(),
            attestation_type: "intimate_recovery".into(),
            note: Some("recognized".into()),
            issued_at: serde_json::json!(1_700_000_000_000_000_i64),
            revoked_at: None,
        },
        witness_agent_id: "A1".into(),
    };
    handle_recovery_v2_signal(&mut conn, signal).expect("dispatch ok");
    let count = crate::db::recovery_witnesses::count_witnesses_for_request(&mut conn, "R1").unwrap();
    assert_eq!(count, 1);
}
```

The existing `setup_test_conn` helper needs to run the new migration; confirm it's picking up all migrations via `embed_migrations!` or similar pattern (inspect the existing helper).

- [ ] **Step 5: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib signals::
```
Expected: all signal tests pass, including the new one.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/signals.rs
git commit -m "feat(storage): project IntimateWitnessSubmitted signals into recovery_witnesses table"
```

---

## Phase 4 — Storage: libp2p gossipsub behaviour + RecoveryInvitation wire contract

File Structure:
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` — compose gossipsub into the `ElohimStorageBehaviour` NetworkBehaviour + add event mapping.
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — export a new submodule.
- Create: `elohim/elohim-storage/src/p2p/recovery_invitation.rs` — wire contract + encode/decode.
- Modify: `elohim/elohim-storage/Cargo.toml` (if needed) — ensure `libp2p-gossipsub` feature is enabled.
- Modify: `elohim/elohim-storage/src/signals.rs` or a new `src/recovery/bridge.rs` — publish on `RecoveryRequestCreated`.

### Task 12: Wire contract — `RecoveryInvitation`

**Files:**
- Create: `elohim/elohim-storage/src/p2p/recovery_invitation.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Create the wire-contract module**

```rust
//! Recovery invitation wire contract — the payload published on the
//! `recovery.invitation` gossipsub topic when a new RecoveryRequest is
//! committed to the DHT.
//!
//! Encoding: MessagePack via `rmp-serde`. Matches the EPR-2C codec
//! convention (see `epr_protocol.rs`). Payloads are small (~100 bytes);
//! no length prefix is needed — gossipsub frames the message.

use serde::{Deserialize, Serialize};

/// Broadcast announcement that a recovery request has been committed.
/// Subscribers filter (M5) for invitations relevant to humans their
/// elohim represents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryInvitation {
    /// ActionHash of the RecoveryRequest, base64-string.
    pub request_hash: String,
    /// Legacy String id of the human being recovered.
    pub human_id: String,
    /// ISO-8601 timestamp of the request commit (forwarded from the signal).
    pub created_at: String,
}

impl RecoveryInvitation {
    /// Encode to MessagePack bytes for gossipsub publish.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Decode from gossipsub-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let inv = RecoveryInvitation {
            request_hash: "R1".into(),
            human_id: "H1".into(),
            created_at: "2026-04-24T00:00:00Z".into(),
        };
        let bytes = inv.to_bytes().unwrap();
        let decoded = RecoveryInvitation::from_bytes(&bytes).unwrap();
        assert_eq!(inv, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        let inv = RecoveryInvitation {
            request_hash: "R1".into(),
            human_id: "H1".into(),
            created_at: "2026-04-24T00:00:00Z".into(),
        };
        // Heuristic — messagepack should keep this under 150 bytes.
        assert!(inv.to_bytes().unwrap().len() < 150);
    }
}
```

- [ ] **Step 2: Expose the module**

Append to `elohim/elohim-storage/src/p2p/mod.rs`:

```rust
pub mod recovery_invitation;
```

- [ ] **Step 3: Run the wire-contract tests**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recovery_invitation
```
Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/recovery_invitation.rs \
        elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): RecoveryInvitation wire contract (MessagePack, matches EPR-2C convention)"
```

---

### Task 13: Compose gossipsub behaviour into the EPR-2C swarm

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs:62-90` (NetworkBehaviour composition)
- Modify: `elohim/elohim-storage/Cargo.toml` — confirm `libp2p` has `gossipsub` feature.

- [ ] **Step 1: Enable gossipsub feature**

Inspect:
```bash
grep -A2 '^libp2p' /projects/elohim/elohim/elohim-storage/Cargo.toml | head -5
```

If the `libp2p` dependency does not already enable `"gossipsub"` (alongside `"kad"`, `"mdns"`, `"relay"`, `"request-response"`, etc.), add it to the features array. Expected shape after edit:

```toml
libp2p = { version = "0.54", features = ["gossipsub", "kad", ...] }
```

- [ ] **Step 2: Add a gossipsub field to `ElohimStorageBehaviour`**

In `behaviour.rs:62-90`, add the field:

```rust
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ElohimStorageBehaviourEvent")]
pub struct ElohimStorageBehaviour {
    pub kademlia: Kademlia<SledRecordStore>,
    pub shard_protocol: RequestResponse<ShardCodec>,
    pub sync_protocol: RequestResponse<SyncCodec>,
    pub epr_protocol: RequestResponse<EprCodec>,
    pub epr_atom_protocol: RequestResponse<EprAtomCodec>,
    pub trust_protocol: RequestResponse<TrustCodec>,
    pub mdns: mdns::tokio::Behaviour,
    pub relay_client: relay::client::Behaviour,
    // M3: recovery invitation broadcast
    pub gossipsub: libp2p::gossipsub::Behaviour,
    // ... any trailing fields
}
```

- [ ] **Step 3: Initialize gossipsub in the swarm constructor**

Find the `ElohimStorageBehaviour::new(...)` (or whatever constructor builds the composition) and add gossipsub init. A sane default:

```rust
use libp2p::gossipsub::{self, Behaviour as GossipBehaviour, ConfigBuilder, IdentTopic, MessageAuthenticity, ValidationMode};

let gossipsub_config = ConfigBuilder::default()
    .heartbeat_interval(std::time::Duration::from_secs(10))
    .validation_mode(ValidationMode::Strict)
    .build()
    .expect("valid gossipsub config");

let mut gossipsub = GossipBehaviour::new(
    MessageAuthenticity::Signed(local_key.clone()),
    gossipsub_config,
)
.expect("gossipsub behaviour init");

let topic = IdentTopic::new("recovery.invitation");
gossipsub.subscribe(&topic).expect("subscribe to recovery.invitation");
```

If the constructor takes a `local_key: &Keypair`, reuse it; otherwise inspect the existing peer-id plumbing in the same file (EPR-2C swarm already has a keypair; use the same one).

- [ ] **Step 4: Map the gossipsub event into `ElohimStorageBehaviourEvent`**

The `#[derive(NetworkBehaviour)]` macro expects a corresponding variant on the swarm event enum. Locate the `ElohimStorageBehaviourEvent` enum (grep for it if not in the same file) and add:

```rust
    Gossipsub(libp2p::gossipsub::Event),
```

(The macro may generate this automatically depending on the `derive`'s `to_swarm` attribute. If generation is automatic, no manual change needed — just adding the field in step 2 suffices.)

- [ ] **Step 5: Build elohim-storage**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: zero errors. The swarm now has a gossipsub behaviour subscribed to `recovery.invitation`.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/behaviour.rs \
        elohim/elohim-storage/Cargo.toml
git commit -m "feat(storage): compose gossipsub into EPR-2C swarm, subscribe recovery.invitation"
```

---

### Task 14: Publish `RecoveryInvitation` on `RecoveryRequestCreated`

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs` (or wherever `handle_recovery_v2_signal` can reach the swarm — typically via an mpsc channel to the p2p runtime)

**Design note:** The signal handler runs inside the projection loop with a `&mut SqliteConnection`. It must not hold the swarm directly. Convention in this codebase: the signal handler writes a "publish intent" to an mpsc-style channel consumed by the p2p runtime, which actually calls `swarm.behaviour_mut().gossipsub.publish(...)`. Inspect how `BlobSignal` handlers in `signals.rs` dispatch to the swarm — follow that pattern.

- [ ] **Step 1: Identify the existing signal → p2p bridge**

```bash
grep -n "mpsc\|tx\|publish\|swarm" /projects/elohim/elohim/elohim-storage/src/signals.rs | head -20
grep -rn "p2p_command\|PublishIntent\|GossipCommand" /projects/elohim/elohim/elohim-storage/src/ | head -10
```

If there's an existing "p2p command" channel (e.g., `P2pCommand::Publish { topic, payload }`), extend its enum. If not, this task adds the minimal primitive.

- [ ] **Step 2: Create or extend the p2p command channel**

Preferred location: `elohim/elohim-storage/src/p2p/commands.rs` (or the file already used).

Add (or extend):
```rust
/// Commands the projection/signal layer sends to the p2p runtime.
#[derive(Debug)]
pub enum P2pCommand {
    PublishRecoveryInvitation(crate::p2p::recovery_invitation::RecoveryInvitation),
    // ... existing variants
}
```

- [ ] **Step 3: Wire the sender into `handle_recovery_v2_signal`**

The signature of `handle_recovery_v2_signal` likely takes `&mut conn`. Extend it to also accept (or hold internally via lazy-init) a `tokio::sync::mpsc::UnboundedSender<P2pCommand>`. Follow the pattern used by `BlobSignal` handlers (if they already take a sender).

In the `RecoveryRequestCreated` arm, after the successful `upsert_recovery_request`, add:

```rust
            // Publish to gossipsub substrate.
            if let Some(human_id) = &request.human_id {
                let created_at_iso = timestamp_to_iso(&request.created_at);
                let invitation = crate::p2p::recovery_invitation::RecoveryInvitation {
                    request_hash: action_hash.clone(),
                    human_id: human_id.clone(),
                    created_at: created_at_iso,
                };
                // Best-effort; dropping the command is tolerable at Stage 1.
                let _ = p2p_tx.send(crate::p2p::commands::P2pCommand::PublishRecoveryInvitation(invitation));
            }
```

- [ ] **Step 4: Consume `P2pCommand::PublishRecoveryInvitation` in the p2p runtime loop**

Locate the swarm event loop (grep for `SwarmEvent::` — typically in `p2p/runtime.rs` or similar). Add a match arm that handles incoming `P2pCommand`:

```rust
            Some(P2pCommand::PublishRecoveryInvitation(inv)) = commands_rx.recv() => {
                use libp2p::gossipsub::IdentTopic;
                let topic = IdentTopic::new("recovery.invitation");
                match inv.to_bytes() {
                    Ok(bytes) => {
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, bytes) {
                            tracing::warn!(target: "elohim_storage::recovery", "gossipsub publish failed: {e}");
                        } else {
                            tracing::info!(target: "elohim_storage::recovery",
                                "published invitation request_hash={} human_id={}",
                                inv.request_hash, inv.human_id);
                        }
                    }
                    Err(e) => tracing::warn!(target: "elohim_storage::recovery",
                        "failed to encode invitation: {e}"),
                }
            }
```

- [ ] **Step 5: Build**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: zero errors.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(storage): publish RecoveryInvitation to gossipsub on RecoveryRequestCreated"
```

---

### Task 15: Subscribe stub — log received invitations

**Files:**
- Modify: the swarm event loop (same file as Task 14 step 4).

- [ ] **Step 1: Add a GossipsubEvent match arm to the swarm loop**

```rust
            SwarmEvent::Behaviour(ElohimStorageBehaviourEvent::Gossipsub(
                libp2p::gossipsub::Event::Message { propagation_source, message_id, message },
            )) => {
                if message.topic.as_str() == "recovery.invitation" {
                    match crate::p2p::recovery_invitation::RecoveryInvitation::from_bytes(&message.data) {
                        Ok(inv) => tracing::info!(target: "elohim_storage::recovery",
                            "received invitation from={} request_hash={} human_id={}",
                            propagation_source, inv.request_hash, inv.human_id),
                        Err(e) => tracing::warn!(target: "elohim_storage::recovery",
                            "invalid invitation from={}: {e}", propagation_source),
                    }
                    let _ = message_id; // reserved for M5 dedupe/rate-limiting
                }
            }
```

Adjust match-path if the NetworkBehaviour-derived enum variant is named differently.

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(storage): subscribe stub for recovery.invitation — logs received invitations"
```

---

## Phase 5 — Tests: sweettest cross-cell scenarios

File Structure:
- Create: `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs`
- Modify: `elohim/holochain/tests/sweettest/src/tests/mod.rs` — declare the new module.

**Precondition:** Phases 1-4 must land before starting this phase. Cross-cell sweettest requires the DNA to be pack-able with the M3 coordinator code.

### Task 16: Sweettest — happy-path intimate quorum

**Files:**
- Create: `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs`
- Modify: `elohim/holochain/tests/sweettest/src/tests/mod.rs`

- [ ] **Step 1: Create the test module**

```rust
//! Sweettest — Recovery Protocol Phase 2 Milestone M3.
//!
//! These tests exercise coordinator-side correctness across multiple cells
//! in a single conductor network. They are `#[ignore]` until the DNA is
//! packed upstream by the pipeline.
//!
//! Scenarios per M3 design §9.1:
//!   1. happy-path intimate quorum (3 contacts, threshold 3)
//!   2. freeze-floor blocks intimate, allows cryptographic
//!   3. anchor durability across rotation
//!   4. non-contact witness rejected

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, multi_agent_conductor_network, SweetAgents},
    fixtures::network_seed,
};

const DNA: &str = "imagodei";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_happy_path_intimate_quorum() -> Result<()> {
    // 4 agents: A (claimant), B/C/D (emergency contacts)
    let (mut net, agents) = multi_agent_conductor_network(4).await?;
    let (a, b, c, d) = (agents[0].clone(), agents[1].clone(), agents[2].clone(), agents[3].clone());

    // 1. install DNA + app on each agent
    let seed = network_seed(DNA);
    let dna_a = load_dna(DNA, &seed, Some(a.clone())).await?;
    let dna_b = load_dna(DNA, &seed, Some(b.clone())).await?;
    let dna_c = load_dna(DNA, &seed, Some(c.clone())).await?;
    let dna_d = load_dna(DNA, &seed, Some(d.clone())).await?;
    // (install via net.setup_app_for_agent(...) per each; adapt to existing harness)

    // 2. A creates Human entry; B, C, D also create Humans and HumanRelationships
    //    with A where emergency_access_enabled = true.
    //    (Use the existing coordinator create_human_relationship function — grep
    //     for its signature to confirm the input shape.)

    // 3. a call create_recovery_request from a doorway-agent cell (or simulated
    //    A-side cell for M3) → observe:
    //       response.request.human_id == Some(a_human_id)
    //       response.request.required_witness_count == 3   // ceil(3/2)+1 = 3

    // 4. B calls submit_intimate_witness → passes
    // 5. C calls submit_intimate_witness → passes
    // 6. Attempt commit_key_rotation with authority = IntimateQuorum {
    //       witness_hashes: [witness_b, witness_c]
    //    } → REJECTS (threshold 3, witnesses 2).

    // 7. D calls submit_intimate_witness → passes
    // 8. commit_key_rotation with witness_hashes: [b, c, d] → SUCCEEDS

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_freeze_floor_blocks_intimate_allows_cryptographic() -> Result<()> {
    // Set up A with 3 emergency contacts as above (reuse a helper).
    // Before any rotation attempt, A's cell (or defender proxy) commits an
    // IdentityFreeze { human_id, frozen_at_layer: Some("intimate"), is_active: true }.
    //
    // Attempt commit_key_rotation with IntimateQuorum { valid witness_hashes } → REJECTS
    //   (freeze-floor gate fires).
    // Attempt commit_key_rotation with CryptographicQuorum { stewardship_hash } → PASSES
    //   (gate is exempt per design §4.3).
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_anchor_durability_across_rotation() -> Result<()> {
    // A runs through the happy path.
    // After rotation, re-resolve HumanToRecoveryRequest anchor for A's human_id →
    // assert the original RecoveryRequest is still discoverable. (Guard against
    // a future regression that anchors on pubkey.)
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_non_contact_witness_rejected() -> Result<()> {
    // A + 3 contacts set up as above. A 5th agent E (not on any relationship)
    // invokes submit_intimate_witness → the coordinator's membership gate rejects.
    Ok(())
}
```

The scenario bodies are **stubs** — writing them out in full requires familiarity with the local `multi_agent_conductor_network` harness (which may or may not exist under that name). Inspect the existing `imagodei.rs` + any helpers under `src/common/conductors.rs` to ground the implementations.

- [ ] **Step 2: Register module**

Append to `elohim/holochain/tests/sweettest/src/tests/mod.rs`:

```rust
pub mod recovery_m3;
```

Or if that file does not exist, inspect how siblings like `imagodei.rs` are auto-included (the sweettest crate may use `#[path]` attrs from `lib.rs`). Follow the existing pattern.

- [ ] **Step 3: Build sweettest**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && cargo build
```
Expected: zero errors. Tests are `#[ignore]` so unpacked-DNA environment still compiles.

- [ ] **Step 4: Commit the skeleton**

```bash
git add elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs \
        elohim/holochain/tests/sweettest/src/tests/mod.rs
git commit -m "test(sweettest): scaffolding for M3 recovery scenarios (ignored pending pack)"
```

- [ ] **Step 5: Fill in the first scenario body — `m3_happy_path_intimate_quorum`**

Replace the TODO-style comments with full conductor calls. Inspect an existing multi-cell imagodei test (check `infrastructure.rs` or `lamad.rs` for multi-agent patterns) and copy the setup idiom.

Key call-sites per the design §7 data flow:
- `create_human` / `create_agent` for each of A, B, C, D (or use a fixture helper).
- `create_human_relationship { subject: A, counterparty: B, emergency_access_enabled: true }` × 3.
- `create_recovery_request { human_agent_pubkey: a_pubkey, ... }` via A's cell.
- `submit_intimate_witness { recovery_request_hash: ..., note: None }` via B, C, D.
- `commit_key_rotation { authority: IntimateQuorum { witness_hashes: vec![...] }, ... }` via A (or doorway proxy).

Assert with `assert!`/`assert_eq!` on returned `RecoveryRequestOutput.request.required_witness_count == 3` and the expected `commit_key_rotation` accept/reject outcomes.

- [ ] **Step 6: Fill in the remaining three scenarios**

Complete `m3_freeze_floor_blocks_intimate_allows_cryptographic`, `m3_anchor_durability_across_rotation`, `m3_non_contact_witness_rejected`. Each reuses the same 3-contact setup via a `setup_a_with_three_contacts(&mut net) -> Result<...>` helper defined in the test module.

- [ ] **Step 7: Run sweettest (integration)**

Only runs in an environment that packs the DNA (Jenkins or local nix shell). Locally:

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just pack
cd /projects/elohim/elohim/holochain/tests/sweettest && \
  cargo test --release -- --ignored recovery_m3
```

If pack requires nix: defer actual test execution to CI. Skeleton that compiles is the gate for merging locally.

- [ ] **Step 8: Commit**

```bash
git add elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs
git commit -m "test(sweettest): fill in M3 intimate-quorum + freeze-floor + anchor-durability + non-contact scenarios"
```

---

## Phase 6 — Tests: a2o cross-doorway features

File Structure:
- Create: `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature`
- Create: `genesis/a2o/features/auth/recovery/freeze-floor-blocks-intimate-rotation.feature`

### Task 17: a2o feature — intimate quorum happy path

**Files:**
- Create: `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature`

- [ ] **Step 1: Create the feature directory + file**

```gherkin
@stage1-structural @recovery-m3
Feature: Intimate Quorum Happy Path — A Lost Pubkey Is Restored Through Emergency Contacts

  The claimant Abby has lost access to her agent key. She has three emergency
  contacts (Ben, Cara, Dan) set up ambiently via HumanRelationship entries
  with emergency_access_enabled = true. Under the revised Phase 2 design, no
  ceremonial seed setup exists — the relationships ARE the recovery fabric.

  At least one emergency contact's household hosts them on a *different*
  doorway-steward from Abby's. This exercises cross-doorway invitation
  fan-out on the libp2p mesh: the gossipsub publish from Abby's colocated
  elohim-storage must reach Ben's / Cara's / Dan's elohim-storage pods too.

  Stage-1 structural acceptance: humans tap confirm. Elohim-specialist
  attestation lands in M5.

  Background:
    Given the shem topology is running with at least two doorway-stewards
    And Abby is registered via "doorway-alpha"
    And Ben is registered via "doorway-beta"
    And Cara and Dan are registered via "doorway-alpha"
    And each of Ben, Cara, Dan has a HumanRelationship to Abby with emergency_access_enabled = true

  Scenario: A recovery request reaches Abby's required_witness_count of 3
    When Abby invokes create_recovery_request from a fresh agent key
    Then the returned request has human_id == Abby's human_id
    And the returned request has required_witness_count == 3
    And a RecoveryInvitation is published on the "recovery.invitation" topic
    And the elohim-storage pod colocated with doorway-beta logs "received invitation"

  Scenario: Three witnesses satisfy the threshold and rotation succeeds
    Given an open recovery request for Abby with required_witness_count == 3
    When Ben, Cara, and Dan each submit_intimate_witness on the request
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum { witness_hashes: [ben, cara, dan] }
    Then the rotation succeeds
    And the storage projection records the KeyRotation

  Scenario: Two witnesses fall short of the threshold and rotation is rejected
    Given an open recovery request for Abby with required_witness_count == 3
    And Ben and Cara have submitted intimate witnesses
    When Abby invokes commit_key_rotation with IntimateQuorum { witness_hashes: [ben, cara] }
    Then the rotation is rejected by the M2 validator
    And the recovery_witnesses projection retains count 2

  Scenario: A non-contact cannot submit a witness
    Given an open recovery request for Abby
    And agent Evan has no HumanRelationship to Abby
    When Evan invokes submit_intimate_witness on the request
    Then the coordinator pre-commit gate rejects with an emergency-contact-membership error
    And no HumanityWitness entry is committed for Evan on this request
```

- [ ] **Step 2: Validate the feature file parses**

```bash
cd /projects/elohim && \
  node -e "const { readFileSync } = require('fs'); const path = 'genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature'; console.log(readFileSync(path,'utf8').slice(0,200))"
```
Expected: first 200 chars of the feature file print. No parse — a2o runner will parse at execution time.

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature
git commit -m "test(a2o): intimate-quorum-happy-path feature (cross-doorway shem topology)"
```

---

### Task 18: a2o feature — freeze-floor blocks intimate rotation

**Files:**
- Create: `genesis/a2o/features/auth/recovery/freeze-floor-blocks-intimate-rotation.feature`

- [ ] **Step 1: Create the feature file**

```gherkin
@stage1-structural @recovery-m3
Feature: Freeze-Floor Gate Blocks Intimate-Layer Rotation

  When an elohim defender (or, pre-M5, a manually committed IdentityFreeze)
  has flagged the intimate recovery layer as frozen for a human, the
  coordinator pre-commit gate on commit_key_rotation must reject an
  IntimateQuorum rotation even when enough witnesses are present. A
  CryptographicQuorum rotation on the same human must still pass — the
  cryptographic layer is orthogonal per design §1.1.

  Stage-1 structural acceptance: the gate runs structurally even though
  M5 is where defenders author freezes in volume. We exercise the gate by
  committing a freeze directly in the test fixture.

  Background:
    Given the shem topology is running
    And Abby has 3 emergency contacts (Ben, Cara, Dan) with emergency_access_enabled = true
    And an open RecoveryRequest exists for Abby with required_witness_count == 3
    And Ben, Cara, Dan have submitted intimate witnesses (count 3, threshold met)

  Scenario: An active intimate freeze blocks the IntimateQuorum rotation
    Given an IdentityFreeze { human_id: abby, frozen_at_layer: "intimate", is_active: true } has been committed
    When Abby invokes commit_key_rotation with IntimateQuorum { witness_hashes: [ben, cara, dan] }
    Then the coordinator pre-commit gate rejects with a freeze-floor error
    And no KeyRotation entry is committed

  Scenario: A CryptographicQuorum rotation is exempt from the freeze-floor gate
    Given an IdentityFreeze { human_id: abby, frozen_at_layer: "intimate", is_active: true } has been committed
    And Abby has a valid KeyStewardship with a threshold-reached quorum signature
    When Abby invokes commit_key_rotation with CryptographicQuorum { stewardship_hash: abby_stewardship }
    Then the rotation succeeds
    And the storage projection records the KeyRotation
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/auth/recovery/freeze-floor-blocks-intimate-rotation.feature
git commit -m "test(a2o): freeze-floor-blocks-intimate-rotation feature"
```

---

## Phase 7 — Self-review, build-all, push

### Task 19: Full build-and-test gate before pushing

- [ ] **Step 1: Run every quality gate the pre-push hook runs**

```bash
cd /projects/elohim
cd elohim/holochain/dna/imagodei && just check && just pack && cd -
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --release && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract_recovery_v2 && cd -
cd elohim/holochain/tests/sweettest && cargo build && cd -
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts
```

Every command must return success. If one fails, **do not push**; fix the root cause and re-run.

- [ ] **Step 2: Review the diff end-to-end**

```bash
git log --oneline dev..HEAD
git diff --stat dev..HEAD
```

Confirm:
- ~20 commits (one per task step completion where logical units landed).
- Files modified are in-scope (no `git log --follow` surprises outside the DNA/storage/test boundaries).
- No `--amend` or force-pushes happened (commits form a linear chain from dev).

- [ ] **Step 3: Push (with husky)**

```bash
git push -u origin feature/recovery-m3-coordinator
```

**Do not use `HUSKY=0`.** If the hook fails, resolve the underlying quality issue.

- [ ] **Step 4: Open a PR (optional — depends on user preference)**

```bash
gh pr create --base dev --title "feat(recovery): Phase 2 Milestone M3 — coordinator, gossipsub substrate, witness projection" \
  --body "$(cat <<'EOF'
## Summary

- DNA: `create_recovery_request` populates `human_id` + `required_witness_count`; `commit_key_rotation` runs freeze-floor pre-commit gate (CryptographicQuorum exempt); new `submit_intimate_witness` function with membership + dedupe gates; two new link types + `IntimateWitnessSubmitted` signal variant.
- Storage: gossipsub behaviour composed onto EPR-2C swarm; `RecoveryInvitation` wire contract (MessagePack via rmp-serde); publish on `RecoveryRequestCreated` + subscribe stub; `recovery_witnesses` table + projection off `IntimateWitnessSubmitted`.
- Tests: 4 sweettest scenarios (happy path, freeze-floor, anchor durability, non-contact rejected) + 2 a2o features (intimate-quorum-happy-path + freeze-floor-blocks-intimate-rotation) in shem cross-doorway topology.

## Design reference

`genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m3-coordinator-and-storage-design.md`

## Deferred (out-of-scope per design §11)

Account/login layer (OAuth-pattern graduation; see memory `project_peer_native_account_canonical_surface`), hosted-cell bootstrap, browser session handoff, elohim-defender freeze authoring, holder-side elohim evaluation, per-request topic sharding, hashcash — all M5+.

## Test plan

- [ ] DNA packs cleanly
- [ ] `cargo test --lib` passes in elohim-storage
- [ ] Schema contract test passes for `RecoveryWitnessView`
- [ ] Sweettest `recovery_m3` suite compiles (ignored pending pipeline pack)
- [ ] a2o features parse

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Execution Strategy — subagent dispatch

Per M3 design §10.2 the work divides across three focused dispatches. The plan's task numbering collapses to this matrix:

| Wave | Subagent | Scope | Tasks in this plan |
|------|----------|-------|-------------------:|
| 1 | rust-architect #1 (DNA) | `elohim/holochain/dna/imagodei/` + `zomes/imagodei/` + `zomes/imagodei_integrity/` | Pre-flight, Tasks 1-6 |
| 1 | rust-architect #2 (Storage) | `elohim/elohim-storage/` + `elohim/sdk/schemas/v1/views/recovery-witness.*` + `elohim/sdk/schemas/scripts/codegen-ts.mjs` | Tasks 7-15 |
| 2 | rust-architect #3 (Tests) | `elohim/holochain/tests/sweettest/` + `genesis/a2o/features/auth/recovery/` | Tasks 16-18 |
| Finishing | orchestrator | full branch | Task 19 |

Wave 1 runs in parallel isolated worktrees (zero file overlap between DNA and Storage). Merge both back before Wave 2 starts — Wave 2 compiles against both.

**Scope guardrails** (required in every dispatch prompt per memory `feedback_subagent_scope_guardrails`):
- "Do not `git revert` or `git reset` any pre-existing commit. If you encounter a scope conflict, BLOCK and report — do not silently clean up."
- "Files outside your listed scope are forbidden to modify."
- "Do not stage files you did not author in this dispatch."

After each dispatch returns, the orchestrator scans the SHA range:

```bash
git log --name-only <dispatch-base>..HEAD | grep -v "^commit\|^Author\|^Date\|^ *$\|^    "
```

Any file outside the dispatch's declared scope = block the merge, investigate before approving.

---

## Self-Review

**1. Spec coverage — M3 design §4 (DNA), §5 (Mesh), §6 (Storage), §7 (Data flow), §8 (Security), §9 (Testing):**

| Spec requirement | Covered by | Notes |
|------------------|-----------:|-------|
| §4.1 link types | Task 1 | |
| §4.2 `create_recovery_request` bridging + human_id anchor | Task 3 | Decision log #2 implemented |
| §4.3 freeze-floor gate on `commit_key_rotation` with CryptographicQuorum exemption | Task 4 | |
| §4.4 `submit_intimate_witness` (fetch + membership + dedupe gates, emit rich signal) | Task 5 | |
| §4.5 `RecoveryV2Signal::IntimateWitnessSubmitted` | Task 2 (DNA) + Task 11 (storage mirror) | Serde tag `"type"` matched on both sides |
| §5.1 gossipsub behaviour on EPR-2C swarm | Task 13 | Single broadcast topic `recovery.invitation` |
| §5.1 wire contract + codec | Task 12 | MessagePack via rmp-serde — corrects design drift (design says ciborium/CBOR; actual EPR-2C convention is rmp-serde/MessagePack) |
| §5.1 signal bridge publish | Task 14 | |
| §5.1 subscribe stub | Task 15 | |
| §6.1 schemas | Task 7 | |
| §6.3 migrations | Task 8 | |
| §6.3 diesel projection handlers | Tasks 9 + 11 | |
| §6.4 TS codegen | Task 10 step 4-5 | |
| §9.1 four sweettest scenarios | Task 16 | |
| §9.2 two a2o features (shem cross-doorway) | Tasks 17 + 18 | |

No spec requirement is uncovered.

**2. Placeholder scan:**

Remaining soft spots (not placeholders — **informed parameter deferrals**):
- Task 3 step 3 — `HumanToHumanRelationship` link-type name may differ; plan directs reader to verify via grep.
- Task 5 step 3 — `relationship_counterparty_matches` pseudo-fields (`subject_agent_id`/`counterparty_agent_id`) are placeholders and the plan explicitly flags they must be replaced by reading the real struct before commit.
- Task 11 step 1 — `HumanityWitnessPayload` fields must match DNA struct exactly; plan flags this.

These are honest acknowledgments that the plan cannot fully ground every struct field without re-reading the DNA — the subagent is expected to do that reading and adapt. Not plan failures.

**3. Type consistency:**

- `RecoveryV2Signal` variants: DNA side (Task 2) and storage side (Task 11) use identical variant names and serde tag. ✓
- `RecoveryInvitation` struct (Task 12) carries `request_hash: String`, `human_id: String`, `created_at: String` — same shape referenced in Task 14's publish construction. ✓
- `NewRecoveryWitnessRow` fields (Task 9) match `RecoveryWitnessView` fields (Task 10) one-to-one via the `From` impl. ✓
- Schema `recovery-witness.schema.json` (Task 7) fields match `RecoveryWitnessView` struct (Task 10). Contract test enforces this (Task 10 step 3). ✓

---

**Next step:** Execution via `superpowers:subagent-driven-development` with the three-wave dispatch matrix above, OR inline via `superpowers:executing-plans`. User decides.
