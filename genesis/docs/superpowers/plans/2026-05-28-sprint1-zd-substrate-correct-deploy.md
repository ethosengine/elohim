---
status: Draft
informed-by:
  - ../specs/2026-05-25-stagespablob-substrate-correct-deploy.md   # the substrate-correct-deploy design this Sprint 1 plan executes
---

# Sprint 1 — Z.D substrate-correct deploy end-to-end Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first concrete instance of the REA compute-commitment primitive. Replace the Z.1 anti-pattern (`PATCH /db/content/{slug}` from CI) with a substrate-correct flow where `stageSpaBlob` authors an `EprHead` envelope signed by a per-operator deploy-service-agent, emits a `republish-epr` `EconomicEvent` bounded by an operator-authored `delegates-compute` Commitment, and `PUT /api/v1/epr/{cid}`s where the substrate validates bounds (via Sprint 2's `bounds_validator`) before persisting. Add the §3 soft-warn ceremony for reach escalation. Cover with sweettest + a2o regression scenarios.

**Architecture:** Two zomes get new actions — Mishpat coordinator wires `delegates-compute` and `acknowledges-reach-change` Commitment actions; elohim DNA content_store coordinator wires `republish-epr` EconomicEvent action. The substrate-side `republish_epr_validator` is an instance of Sprint 2's `bounds_validator::validate` plus the schema-specific payload check; it lives in `elohim/elohim-storage/src/services/` and is invoked from the existing `PUT /api/v1/epr/{cid}` handler at `elohim/elohim-storage/src/api/epr.rs:484`. CI scripts (provision-deploy-agent + author-deploy-commitment + stage-spa-blob-zd) provision the deploy-svc-agent + author the bounding Commitment + sign-and-publish the bundle. Doorway extends with a `reach_evaluator` that gates serving on reach match against the active project-epr Commitment; mismatch fires the soft-warn ceremony.

**Tech Stack:** Rust (Mishpat + content_store zomes via HDK 0.5; elohim-storage service + handler), TypeScript (deploy-svc-agent provisioning + Jenkins-invokable script + Commitment authoring CLI), Ed25519 (`ed25519-dalek` for signing), Jenkins (CI secret loading + script invocation), Holochain HC 0.6, dag-cbor + sha2-256 CID per `elohim/epr/src/cid.rs`.

**Existing infrastructure (DISCOVERED — Sprint 1 builds on, does NOT replace):**
- `elohim/elohim-storage/src/api/epr.rs:484` — `PUT /api/v1/epr/{cid}` handler already exists; Sprint 1 extends its validation
- `elohim/holochain/dna/mishpat/zomes/mishpat/` — existing Mishpat coordinator zome; Sprint 1 adds 2 new action discriminators
- `elohim/holochain/dna/elohim/zomes/content_store/` — existing elohim coordinator with `CreateReaEconomicEventInput` and ReA EconomicEvent post-commit signal; Sprint 1 adds `republish-epr` action discriminator + schema validation
- `elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json` — shipped schema; Sprint 1 makes it load-bearing
- `elohim/sdk/schemas/v1/commitments/acknowledges-reach-change.schema.json` — shipped schema; Sprint 1 wires the action
- `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json` — shipped schema; Sprint 1 makes it load-bearing
- `elohim/sdk/schemas/v1/feedback-signals/reach-escalation-pending.schema.json` — shipped schema; Sprint 1 wires the doorway emit path
- `doorway/doorway-service/src/projection/` — existing projection subscriber; Sprint 1 adds `epr.republished` event arm

**Companion files:**
- **Spec (gospel for Sprint 1):** `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — read fully before starting. §1 is the gospel-tier primitive; §2 details Z.D as the first instance; §3 the soft-warn ceremony; §6 acceptance signals; §7 resolved design questions.
- **Sprint 2 plan:** `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md` — Sprint 1's `republish_epr_validator` delegates to Sprint 2's `bounds_validator::validate`. Sprint 1 can proceed against the trait surface (Sprint 2 Task 4) before Sprint 2 fully completes.
- **Parent roadmap:** `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` Sprint 1 entry, P2P Design Gate output (see "P2P Design Gate output" section).
- **Canon:** `genesis/docs/architecture/rea-compute-commitment-primitive.md`, `genesis/docs/architecture/stewardship-over-sovereignty.md`, `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`.

---

## P2P Design Gate output

Per `.claude/skills/p2p-design-gate/SKILL.md` and CLAUDE.md. **Headline: zero new DHT entry types in Sprint 1.** Every notarized payload reuses an existing entry type via action discriminator (`Mishpat::Commitment` + `delegates-compute` / `acknowledges-reach-change`; elohim DNA `EconomicEvent` + `republish-epr`). All schemas are shipped pre-Sprint-1.

| Artifact | Classification | DHT entry type | Source of truth |
|----------|----------------|----------------|-----------------|
| `delegates-compute` Commitment payload | **A (notarized)** | EXISTING `Mishpat::Commitment` (action discriminator only) | Holochain DHT |
| `acknowledges-reach-change` Commitment payload | **A (notarized)** | EXISTING `Mishpat::Commitment` (action discriminator only) | Holochain DHT |
| `republish-epr` EconomicEvent payload | **A (notarized)** | EXISTING elohim DNA `EconomicEvent` (action discriminator only) | Holochain DHT |
| `reach-escalation-pending` FeedbackSignal | **A (notarized)** | EXISTING `FeedbackSignal` (signal_kind extension) | Holochain DHT — already shipped |
| `republish_epr_validator.rs` | n/a (code; instance of S2 bounds_validator) | none | code |
| `reach_evaluator.rs` (doorway) | n/a (code) | none | code |
| `provision-deploy-agent.ts` | n/a (CI script) | none — produces an agent CID + Jenkins secret | code; generates entropy |
| `author-deploy-commitment.ts` | n/a (CLI; result is a DHT Commitment) | none new | Holochain DHT — produces an existing Mishpat::Commitment |
| `stage-spa-blob-zd.ts` | n/a (CI script) | none — emits an existing EconomicEvent | Holochain DHT — produces an existing EconomicEvent |
| `epr.republished` event arm in doorway subscriber | n/a (code; reacts to existing post-commit signal) | none | local doorway projection — operational |

**Anti-pattern check (gate-skill anti-patterns confirmed NOT-PRESENT):**
- UUID-pk-for-notarized-entity: not applicable — all entities use Holochain ActionHash
- REST-route-first design: not applicable — schemas shipped first, zome coordinators wire actions next, validator next, HTTP handler is LAST
- CID-as-relational-FK: not applicable — `bounded_by` walks via Holochain `get`, never SQL JOIN
- Standalone-table-for-agent-state: not applicable
- Three-address-formats-undefined: not applicable
- New-entry-type-when-one-exists: not applicable — zero new entry types
- Granular-data-on-DHT: not applicable — events are bounded fact-records, not telemetry

**Note on audit hook:** the `[P2P DESIGN AUDIT]` hook scans line-by-line; it may continue to flag schema-file references inside test code blocks or task narrative. All such references inherit the table above.

---

## File Structure

### NEW files (15 total)

```
elohim/holochain/dna/mishpat/zomes/mishpat/src/
└── commitments.rs                              (NEW — delegates-compute + acknowledges-reach-change action wiring)

elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/
└── commitments_validation.rs                   (NEW — schema-validate Commitment payloads per JSON schema)

elohim/holochain/dna/elohim/zomes/content_store/src/
└── republish_epr.rs                            (NEW — republish-epr action discriminator handling)

elohim/holochain/dna/elohim/zomes/content_store_integrity/src/
└── republish_epr_validation.rs                 (NEW — schema-validate republish-epr event payload)

elohim/elohim-storage/src/services/
└── republish_epr_validator.rs                  (NEW — instance validator delegating to Sprint 2 bounds_validator)

doorway/doorway-service/src/projection/
└── reach_evaluator.rs                          (NEW — soft-warn ceremony per §3)

genesis/orchestrator/scripts/
├── provision-deploy-agent.ts                   (NEW — Ed25519 keypair + agent CID + Jenkins credential stash)
├── author-deploy-commitment.ts                 (NEW — operator-steward CLI to author delegates-compute Commitment)
└── stage-spa-blob-zd.ts                        (NEW — replaces stage-spa-blob.sh; signs envelope + emits event)

elohim/holochain/tests/sweettest/src/
└── substrate_correct_deploy_test.rs            (NEW — two-conductor integration test)

genesis/a2o/features/doorway/
├── substrate-correct-deploy.feature            (NEW — happy-path Z.D scenario)
├── bounds-violation-rejection.feature          (NEW — substrate refuses out-of-bounds republish)
└── reach-escalation-soft-warn.feature          (NEW — §3 ceremony scenario)

genesis/docs/research/
└── 2026-05-28-sprint1-zd-implementation-notes.md  (NEW — close-out + lessons + commit log)
```

### MODIFIED files

```
elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs          (export commitments module)
elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs (export validation module)
elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs     (export republish_epr module)
elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs (export validation module)
elohim/elohim-storage/src/api/epr.rs                           (put_epr handler invokes republish_epr_validator)
elohim/elohim-storage/src/services/mod.rs                      (declare republish_epr_validator module)
elohim/sdk/domains/elohim/manifest.json                        (declare republish-epr event_kind)
doorway/doorway-service/src/projection/mod.rs                  (export reach_evaluator module)
doorway/doorway-service/src/projection/subscriber.rs           (add epr.republished event arm)
Jenkinsfile                                                    (load deploy-svc-agent secret + invoke stage-spa-blob-zd.ts)
```

### DELETED files

```
app/elohim-app/scripts/stage-spa-blob.sh                       (Z.1 anti-pattern; Z.E pre-condition #4 from spec §4)
```

---

### Task 1: Mishpat coordinator — `delegates-compute` action

**Files:**
- Create: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`

**Spec reference:** §2 (Z.D Roles + Bounds) and §1 generalization table.

- [ ] **Step 1: Write the failing zome unit test**

In `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_commitment_with_delegates_compute_action_succeeds() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: serde_json::json!({
                "action": "delegates-compute",
                "scope": "republish-epr",
                "provider": "agent:matthew-steward",
                "recipient": "agent:deploy-svc-matthew",
                "bounds": {
                    "epr_scope": ["epr:lamad-spa"],
                    "reach_ceiling": "commons",
                    "rate_per_hour": 30,
                    "rotation_ttl_days": 90
                },
                "valid_from": "2026-05-28T00:00:00Z",
                "valid_until": "2026-08-26T00:00:00Z"
            }).to_string(),
        };
        let result = validate_commitment_payload(&input);
        assert!(result.is_ok(), "well-formed delegates-compute payload must validate");
    }

    #[test]
    fn create_commitment_rejects_invalid_payload() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: serde_json::json!({"action": "delegates-compute"}).to_string(), // missing required fields
        };
        let result = validate_commitment_payload(&input);
        assert!(result.is_err(), "incomplete payload must fail validation");
    }
}
```

- [ ] **Step 2: Implement CreateCommitmentInput + create_commitment + validate_commitment_payload**

```rust
// elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
use hdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommitmentInput {
    pub action: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
}

#[hdk_extern]
pub fn create_commitment(input: CreateCommitmentInput) -> ExternResult<CommitmentOutput> {
    validate_commitment_payload(&input)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;
    let entry = mishpat_integrity::Commitment {
        action: input.action.clone(),
        payload_json: input.payload_json.clone(),
        signed_at: sys_time()?.as_seconds_and_nanos().0.to_string(),
    };
    let action_hash = create_entry(&mishpat_integrity::EntryTypes::Commitment(entry.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("just-created entry missing".into())))?;
    let entry_hash = record.entry().to_app_option::<mishpat_integrity::Commitment>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .map(|_| record.action().entry_hash().cloned().unwrap_or_default())
        .unwrap_or_default();
    Ok(CommitmentOutput { action_hash, entry_hash })
}

/// Validate the commitment payload against the action-specific schema.
/// For `delegates-compute` action, validates against delegates-compute.schema.json.
pub fn validate_commitment_payload(input: &CreateCommitmentInput) -> Result<(), String> {
    let payload: serde_json::Value = serde_json::from_str(&input.payload_json)
        .map_err(|e| format!("payload_json not parseable: {e}"))?;

    match input.action.as_str() {
        "delegates-compute" => validate_delegates_compute(&payload),
        "acknowledges-reach-change" => validate_acknowledges_reach_change(&payload),
        // Other actions (project-epr, etc.) handled elsewhere
        other => Err(format!("commitments::validate_commitment_payload unhandled action: {other}")),
    }
}

fn validate_delegates_compute(payload: &serde_json::Value) -> Result<(), String> {
    // Required top-level fields per delegates-compute.schema.json
    let required = ["action", "scope", "provider", "recipient", "bounds", "valid_from", "valid_until"];
    for field in required {
        if !payload.get(field).is_some() {
            return Err(format!("delegates-compute missing required field: {field}"));
        }
    }
    if payload["action"] != "delegates-compute" {
        return Err("action field must equal 'delegates-compute'".into());
    }
    // Required bounds fields
    let bounds = payload.get("bounds").and_then(|b| b.as_object())
        .ok_or("bounds must be object")?;
    for field in ["epr_scope", "reach_ceiling", "rate_per_hour", "rotation_ttl_days"] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // reach_ceiling above commons/community requires reach_elevation_acknowledged=true
    let ceiling = bounds["reach_ceiling"].as_str().unwrap_or("");
    if !matches!(ceiling, "commons" | "community") {
        let acked = bounds.get("reach_elevation_acknowledged").and_then(|v| v.as_bool()).unwrap_or(false);
        if !acked {
            return Err(format!("reach_ceiling='{}' requires reach_elevation_acknowledged=true", ceiling));
        }
    }
    Ok(())
}

fn validate_acknowledges_reach_change(_payload: &serde_json::Value) -> Result<(), String> {
    // Stub — Task 3 fully implements
    Ok(())
}
```

Note: this validator hand-rolls the schema check rather than using a full JSON Schema library because HDK WASM has strict size constraints. The hand-rolled validator must mirror `delegates-compute.schema.json`; if the schema changes, the validator must change too. Capture this as a regression-test pattern in Task 5.

- [ ] **Step 3: Run zome tests; expect PASS**

```bash
cd elohim/holochain/dna/mishpat && cargo test -p mishpat 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs
git commit -m "feat(mishpat): delegates-compute Commitment action discriminator + validator"
```

---

### Task 2: Mishpat integrity — `Commitment` entry type confirmation

**Files:**
- Inspect: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs` (confirm Commitment entry type exists)
- Modify if needed: same file (add Commitment entry if it doesn't exist)

**Spec reference:** §2 (uses existing Mishpat Commitment entry type per gospel-tier shape).

- [ ] **Step 1: Inspect existing entry types**

```bash
grep -n "Commitment\|EntryTypes\|hdk_entry" elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs | head -30
```

- [ ] **Step 2: If Commitment entry type exists, confirm it accepts `action` + `payload_json` + `signed_at` fields**

If yes (likely — Mishpat is at 11/~100 entry types per gospel-tier memory), skip the next step.

If NO, add the entry type:

```rust
// elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Commitment {
    pub action: String,           // discriminator
    pub payload_json: String,     // schema per action
    pub signed_at: String,        // RFC3339
}

// Inside #[hdk_entry_defs]
#[entry_def(name = "commitment", visibility = "public")]
Commitment(Commitment),
```

- [ ] **Step 3: Validate at integrity layer (defense in depth)**

In `validate` function:

```rust
fn validate_create_commitment(op_record: &OpRecord, commitment: &Commitment) -> ExternResult<ValidateCallbackResult> {
    if commitment.action.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Commitment.action must be non-empty".into()));
    }
    if commitment.payload_json.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Commitment.payload_json must be non-empty".into()));
    }
    serde_json::from_str::<serde_json::Value>(&commitment.payload_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("payload_json not parseable: {e}"))))?;
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 4: Build the DNA WASM**

```bash
cd elohim/holochain/dna/mishpat && cargo build --release --target wasm32-unknown-unknown
```

If RUSTFLAGS isn't set, this is the one place it MUST be set to the WASM backend: `RUSTFLAGS='--cfg getrandom_backend="custom"'`.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat_integrity/
git commit -m "feat(mishpat-integrity): Commitment entry type (or confirmation) + validator"
```

---

### Task 3: Mishpat coordinator — `acknowledges-reach-change` action

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`

**Spec reference:** §3 (the soft-warn ceremony — operator-steward acknowledges intentional reach change).

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn create_commitment_with_acknowledges_reach_change_action_succeeds() {
    let input = CreateCommitmentInput {
        action: "acknowledges-reach-change".to_string(),
        payload_json: serde_json::json!({
            "action": "acknowledges-reach-change",
            "target": "epr-head-cid-new",
            "new_reach": "community",
            "previous_reach": "commons",
            "acknowledged_by": "agent:matthew-steward",
            "signed_at": "2026-05-29T00:00:00Z"
        }).to_string(),
    };
    let result = validate_commitment_payload(&input);
    assert!(result.is_ok());
}

#[test]
fn acknowledges_reach_change_rejects_missing_target() {
    let input = CreateCommitmentInput {
        action: "acknowledges-reach-change".to_string(),
        payload_json: serde_json::json!({
            "action": "acknowledges-reach-change",
            "new_reach": "community"
        }).to_string(),
    };
    assert!(validate_commitment_payload(&input).is_err());
}
```

- [ ] **Step 2: Implement validator**

Replace the `validate_acknowledges_reach_change` stub from Task 1 with:

```rust
fn validate_acknowledges_reach_change(payload: &serde_json::Value) -> Result<(), String> {
    let required = ["action", "target", "new_reach", "acknowledged_by", "signed_at"];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("acknowledges-reach-change missing field: {field}"));
        }
    }
    if payload["action"] != "acknowledges-reach-change" {
        return Err("action must equal 'acknowledges-reach-change'".into());
    }
    let reach_values = ["private", "self", "intimate", "trusted", "familiar", "community", "public", "commons"];
    let new_reach = payload["new_reach"].as_str().unwrap_or("");
    if !reach_values.contains(&new_reach) {
        return Err(format!("new_reach '{}' not a known reach value", new_reach));
    }
    Ok(())
}
```

- [ ] **Step 3: Author the schema (deferred — Sprint 1 may use existing acknowledges-reach-change.schema.json)**

Confirm the schema exists at `elohim/sdk/schemas/v1/commitments/acknowledges-reach-change.schema.json` (it does per pre-Sprint-1 inventory). Confirm field names match between schema and Rust validator. If they diverge, fix the Rust validator to match the schema.

- [ ] **Step 4: Test + commit**

```bash
cd elohim/holochain/dna/mishpat && cargo test -p mishpat 2>&1 | tail -15
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
git commit -m "feat(mishpat): acknowledges-reach-change Commitment action + validator"
```

---

### Task 4: elohim DNA — `republish-epr` EconomicEvent action

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/republish_epr.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Spec reference:** §2 "The deploy event itself" section (republish-epr.schema.json shape).

- [ ] **Step 1: Write failing test**

```rust
// elohim/holochain/dna/elohim/zomes/content_store/src/republish_epr.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_republish_epr_payload_well_formed() {
        let payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc-matthew",
            "bounded_by": "comm-cid-abc",
            "target": "epr-head-cid-new",
            "supersedes": "epr-head-cid-prev",
            "payload": {
                "blob_cid": "bafkreiabc...",
                "epr_kind": "Content",
                "reach": "commons",
                "bundle_path": "lamad-spa"
            },
            "signed_at": "2026-05-28T12:00:00Z"
        });
        assert!(validate_republish_epr_event(&payload).is_ok());
    }

    #[test]
    fn validate_republish_epr_rejects_missing_bounded_by() {
        let payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc-matthew",
            "target": "epr-head-cid-new",
            "payload": {"blob_cid": "x", "epr_kind": "Content", "reach": "commons"},
            "signed_at": "2026-05-28T12:00:00Z"
        });
        assert!(validate_republish_epr_event(&payload).is_err(),
            "anonymous publish forbidden per spec §7.4");
    }

    #[test]
    fn validate_republish_epr_rejects_unknown_epr_kind() {
        let payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc-matthew",
            "bounded_by": "comm-cid-abc",
            "target": "epr-head-cid-new",
            "payload": {"blob_cid": "x", "epr_kind": "NotARealKind", "reach": "commons"},
            "signed_at": "2026-05-28T12:00:00Z"
        });
        assert!(validate_republish_epr_event(&payload).is_err());
    }
}
```

- [ ] **Step 2: Implement validator**

```rust
// elohim/holochain/dna/elohim/zomes/content_store/src/republish_epr.rs
use serde::{Deserialize, Serialize};

pub fn validate_republish_epr_event(payload: &serde_json::Value) -> Result<(), String> {
    let required = ["action", "performer", "bounded_by", "target", "payload", "signed_at"];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("republish-epr missing required field: {field}"));
        }
    }
    if payload["action"] != "republish-epr" {
        return Err("action must equal 'republish-epr'".into());
    }
    let inner = payload.get("payload").and_then(|p| p.as_object())
        .ok_or("payload must be object")?;
    for field in ["blob_cid", "epr_kind", "reach"] {
        if !inner.contains_key(field) {
            return Err(format!("payload missing field: {field}"));
        }
    }
    let epr_kind = inner["epr_kind"].as_str().unwrap_or("");
    let valid_kinds = ["Content", "Agent", "Manifest", "Claim", "Observation",
                       "EconomicEvent", "Commitment", "Attestation", "Delegation", "FeedbackSignal"];
    if !valid_kinds.contains(&epr_kind) {
        return Err(format!("payload.epr_kind '{}' not in enum", epr_kind));
    }
    let reach = inner["reach"].as_str().unwrap_or("");
    let valid_reach = ["private", "self", "intimate", "trusted", "familiar", "community", "public", "commons"];
    if !valid_reach.contains(&reach) {
        return Err(format!("payload.reach '{}' not in enum", reach));
    }
    Ok(())
}
```

- [ ] **Step 3: Wire into existing EconomicEvent creation flow**

In `content_store/src/lib.rs`, find the `create_rea_economic_event` function (the existing handler). Add a dispatch on `input.action`:

```rust
match input.action.as_str() {
    "republish-epr" => {
        let payload: serde_json::Value = serde_json::from_str(&input.payload_json)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
        crate::republish_epr::validate_republish_epr_event(&payload)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;
        // proceed with existing create_entry flow
    }
    _ => { /* existing path */ }
}
```

- [ ] **Step 4: Run + commit**

```bash
cd elohim/holochain/dna/elohim && cargo test -p content_store 2>&1 | tail -20
git add elohim/holochain/dna/elohim/zomes/content_store/src/republish_epr.rs elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "feat(elohim-dna): republish-epr EconomicEvent action discriminator + validator"
```

---

### Task 5: elohim integrity validator for `republish-epr`

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Spec reference:** §2 "The deploy event itself" — defense-in-depth.

- [ ] **Step 1: Find existing EconomicEvent integrity validator**

```bash
grep -n "EconomicEvent\|validate.*economic\|fn validate" elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs | head -20
```

- [ ] **Step 2: Add action-discriminator dispatch in integrity validator**

Mirror the coordinator's validator. If the coordinator already validates schema, the integrity layer should at minimum:
1. Confirm `payload_json` is parseable JSON
2. Confirm `bounded_by` is non-empty when `action == "republish-epr"` (anonymous publish forbidden per spec §7.4)

```rust
fn validate_economic_event_payload(event: &EconomicEvent) -> ExternResult<ValidateCallbackResult> {
    let payload: serde_json::Value = serde_json::from_str(&event.payload_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("payload_json: {e}"))))?;
    if event.action == "republish-epr" {
        if payload.get("bounded_by").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false).not() {
            return Ok(ValidateCallbackResult::Invalid("republish-epr requires non-empty bounded_by".into()));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 3: Build WASM + commit**

```bash
cd elohim/holochain/dna/elohim && cargo build --release --target wasm32-unknown-unknown 2>&1 | tail -10
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "feat(elohim-integrity): defense-in-depth republish-epr bounded_by requirement"
```

---

### Task 6: `republish_epr_validator` service in elohim-storage

**Files:**
- Create: `elohim/elohim-storage/src/services/republish_epr_validator.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Spec reference:** §2 "Bound validator" + §7.3 "Storage validator (cheap, no DHT round-trip): sliding-window query."

**Depends on:** Sprint 2 Task 4 (`bounds_validator::validate` API surface). Can proceed against the trait surface even if Sprint 2 isn't fully complete; the production CommitmentFetcher impl can stay `todo!()` until Sprint 2 wires it.

- [ ] **Step 1: Write failing integration test**

```rust
// elohim/elohim-storage/src/services/republish_epr_validator.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::{MockCommitmentFetcher, CommitmentRecord};
    use crate::services::rate_history::MockRateHistory;

    fn deploy_svc_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "comm-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    #[tokio::test]
    async fn valid_republish_event_passes() {
        let event_payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc",
            "bounded_by": "comm-abc",
            "target": "epr-head-new",
            "payload": {
                "blob_cid": "bafkrei-blob-cid",
                "epr_kind": "Content",
                "reach": "commons",
                "bundle_path": "lamad-spa"
            },
            "signed_at": "2026-05-28T12:00:00Z"
        });
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("comm-abc", deploy_svc_commitment());
        let rate = MockRateHistory::new();
        let result = validate_republish_epr(&event_payload, &fetcher, &rate, "epr:lamad-spa").await;
        assert!(result.is_ok(), "well-formed republish-epr should pass");
    }

    #[tokio::test]
    async fn schema_violation_rejected_before_bounds_check() {
        let event_payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc",
            // missing bounded_by — schema violation
            "target": "epr-head-new",
            "payload": {"blob_cid": "x", "epr_kind": "Content", "reach": "commons"},
            "signed_at": "2026-05-28T12:00:00Z"
        });
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_republish_epr(&event_payload, &fetcher, &rate, "epr:lamad-spa").await;
        assert!(matches!(result, Err(ValidationError::Schema(_))));
    }

    #[tokio::test]
    async fn bounds_violation_emits_feedback_signal_via_bounds_validator() {
        // Stale rotation_ttl
        let mut c = deploy_svc_commitment();
        c.valid_from = "2026-01-01T00:00:00Z".into();
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("comm-abc", c);
        let rate = MockRateHistory::new();
        let event_payload = serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc",
            "bounded_by": "comm-abc",
            "target": "epr-head-new",
            "payload": {"blob_cid": "x", "epr_kind": "Content", "reach": "commons"},
            "signed_at": "2026-05-28T12:00:00Z"
        });
        let result = validate_republish_epr(&event_payload, &fetcher, &rate, "epr:lamad-spa").await;
        assert!(matches!(result, Err(ValidationError::Bounds(_))));
    }
}
```

- [ ] **Step 2: Implement validate_republish_epr**

```rust
// elohim/elohim-storage/src/services/republish_epr_validator.rs
use crate::services::bounds_validator::{validate, EventForValidation, BoundsViolation};
use crate::services::commitment_fetcher::CommitmentFetcher;
use crate::services::rate_history::RateHistory;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("bounds validation failed: {0:?}")]
    Bounds(BoundsViolation),
}

/// Validate a republish-epr EconomicEvent payload at the elohim-storage HTTP
/// handler boundary. This is the hot-path validator that runs BEFORE the
/// event is forwarded to the conductor for DHT commit.
///
/// Per spec §7.3: this is the cheap-no-DHT-round-trip path. The Mishpat zome's
/// integrity validator is the slow-but-DHT-resident audit truth.
pub async fn validate_republish_epr<F: CommitmentFetcher, R: RateHistory>(
    event_payload: &serde_json::Value,
    fetcher: &F,
    rate_history: &R,
    target_epr_id: &str,
) -> Result<(), ValidationError> {
    // 1. Schema check (mirrors elohim-dna content_store's validator)
    validate_payload_schema(event_payload)?;

    // 2. Project to EventForValidation
    let event = EventForValidation {
        action: event_payload["action"].as_str().unwrap_or("").to_string(),
        performer: event_payload["performer"].as_str().unwrap_or("").to_string(),
        bounded_by: event_payload["bounded_by"].as_str().unwrap_or("").to_string(),
        target_epr_id: target_epr_id.to_string(),
        reach: event_payload["payload"]["reach"].as_str().unwrap_or("").to_string(),
        signed_at: event_payload["signed_at"].as_str().unwrap_or("").to_string(),
    };

    // 3. Bounds check via Sprint 2's substrate-wide validator
    validate(&event, fetcher, rate_history)
        .await
        .map(|_checks| ())
        .map_err(ValidationError::Bounds)
}

fn validate_payload_schema(payload: &serde_json::Value) -> Result<(), ValidationError> {
    let required = ["action", "performer", "bounded_by", "target", "payload", "signed_at"];
    for field in required {
        if payload.get(field).is_none() {
            return Err(ValidationError::Schema(format!("missing field: {field}")));
        }
    }
    if payload["action"] != "republish-epr" {
        return Err(ValidationError::Schema("action must be 'republish-epr'".into()));
    }
    let inner = payload.get("payload").and_then(|p| p.as_object())
        .ok_or_else(|| ValidationError::Schema("payload must be object".into()))?;
    for field in ["blob_cid", "epr_kind", "reach"] {
        if !inner.contains_key(field) {
            return Err(ValidationError::Schema(format!("payload missing: {field}")));
        }
    }
    let bounded_by = payload["bounded_by"].as_str().unwrap_or("");
    if bounded_by.is_empty() {
        return Err(ValidationError::Schema("bounded_by must be non-empty (anonymous publish forbidden)".into()));
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::republish_epr_validator 2>&1 | tail -20
git add elohim/elohim-storage/src/services/republish_epr_validator.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): republish_epr_validator delegates to bounds_validator + schema check"
```

---

### Task 7: Wire validator into `PUT /api/v1/epr/{cid}` handler

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs` (extend `put_epr` at line 484)

**Spec reference:** §2 + acceptance signal §6.5 ("bounds-violation deploy is rejected").

- [ ] **Step 1: Locate put_epr handler entry point + extension point**

```bash
grep -n "fn put_epr\|input\.envelope\|input\.event" elohim/elohim-storage/src/api/epr.rs | head -10
```

- [ ] **Step 2: Extend `EprPublishInput` view to include an optional `event` field**

The current input is `EprPublishInput { envelope }`. The Z.D shape needs to carry an optional `event: EconomicEventView` alongside the envelope. Modify:

```rust
// elohim/elohim-storage/src/views.rs (or wherever EprPublishInput lives)
pub struct EprPublishInput {
    pub envelope: EnvelopeView,
    pub event: Option<RepublishEprEventView>,  // new — required when envelope is a republish
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RepublishEprEventView {
    pub action: String,
    pub performer: String,
    pub bounded_by: String,
    pub target: String,
    pub supersedes: Option<String>,
    pub payload: serde_json::Value,
    pub signed_at: String,
}
```

- [ ] **Step 3: Add validation call in put_epr**

After envelope validation (around line 540 — wherever envelope checks complete), before the actual EPR persistence:

```rust
// Z.D substrate-correct validation: if this is a republish, validate bounds.
if let Some(event_view) = &input.event {
    if event_view.action == "republish-epr" {
        let event_payload = serde_json::to_value(event_view)
            .map_err(|e| StorageError::InvalidInput(format!("serialize event: {e}")))?;
        let target_epr_id = derive_epr_identity_from_envelope(&input.envelope);
        let fetcher = ctx.commitment_fetcher.as_ref()
            .ok_or_else(|| StorageError::Internal("CommitmentFetcher not wired".into()))?;
        let rate = ctx.rate_history.as_ref()
            .ok_or_else(|| StorageError::Internal("RateHistory not wired".into()))?;
        crate::services::republish_epr_validator::validate_republish_epr(
            &event_payload,
            fetcher.as_ref(),
            rate.as_ref(),
            &target_epr_id,
        ).await
        .map_err(|e| match e {
            crate::services::republish_epr_validator::ValidationError::Schema(s) =>
                StorageError::InvalidInput(format!("republish-epr schema: {s}")),
            crate::services::republish_epr_validator::ValidationError::Bounds(b) =>
                StorageError::InvalidInput(format!("republish-epr bounds violation: {:?}", b.kind)),
        })?;
    }
}
```

The `derive_epr_identity_from_envelope` helper extracts the EPR identity from envelope.coupling.knowledge or falls back to envelope.cid. See `elohim/epr/src/identity.rs` for the project-epr Commitment-CID-as-identity convention.

- [ ] **Step 4: Smoke test + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib api::epr 2>&1 | tail -20
git add elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): put_epr handler invokes republish_epr_validator for Z.D events"
```

---

### Task 8: `provision-deploy-agent.ts` script

**Files:**
- Create: `genesis/orchestrator/scripts/provision-deploy-agent.ts`

**Spec reference:** §2 "Roles" + §6 acceptance signal §1, §7.1 key custody.

- [ ] **Step 1: Author the script**

```typescript
// genesis/orchestrator/scripts/provision-deploy-agent.ts
import { generateKeyPair, sign, exportKey } from 'jose';
import { writeFileSync } from 'fs';
import { execSync } from 'child_process';

interface ProvisionArgs {
  operator: string;       // e.g. "matthew"
  outDir: string;         // where to write the public-side artifacts
  jenkinsCredId: string;  // Jenkins credential ID to stash the private key
}

async function main(args: ProvisionArgs) {
  // 1. Generate Ed25519 keypair
  const { privateKey, publicKey } = await generateKeyPair('EdDSA', { crv: 'Ed25519', extractable: true });

  // 2. Derive agent CID — sha256 of public key bytes, prefixed
  const pubBytes = await exportKey('raw', publicKey);
  const hashHex = await sha256Hex(pubBytes);
  const agentCid = `agent:deploy-svc-${args.operator}-${hashHex.slice(0, 16)}`;

  // 3. Write public-side artifacts (committable to repo)
  const publicJwk = await exportJWK(publicKey);
  writeFileSync(`${args.outDir}/${agentCid}.public.json`, JSON.stringify({
    agentCid,
    operator: args.operator,
    publicKey: publicJwk,
    provisionedAt: new Date().toISOString(),
  }, null, 2));

  // 4. Print PRIVATE key JWK to stdout; operator manually stashes in Jenkins.
  // (Never write private key to filesystem — operator chooses witnessable secret store per §7.1)
  const privateJwk = await exportJWK(privateKey);
  console.log('---');
  console.log(`AgentCid: ${agentCid}`);
  console.log(`Jenkins credential ID to create: ${args.jenkinsCredId}`);
  console.log('PASTE THE FOLLOWING INTO JENKINS AS A "Secret text" CREDENTIAL:');
  console.log(JSON.stringify(privateJwk));
  console.log('---');
  console.log(`Public artifact saved: ${args.outDir}/${agentCid}.public.json`);
}

// CLI entry
if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  main(args).catch(e => { console.error(e); process.exit(1); });
}
```

- [ ] **Step 2: Test in dry-run mode**

```bash
ts-node genesis/orchestrator/scripts/provision-deploy-agent.ts --operator=matthew --outDir=/tmp/deploy-agents --jenkinsCredId=deploy-svc-matthew-doorway 2>&1
```

Expect: prints private key JWK + writes public artifact.

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/scripts/provision-deploy-agent.ts
git commit -m "feat(orchestrator): provision-deploy-agent.ts — Ed25519 keypair + agent CID + Jenkins stash"
```

---

### Task 9: `author-deploy-commitment.ts` CLI

**Files:**
- Create: `genesis/orchestrator/scripts/author-deploy-commitment.ts`

**Spec reference:** §2 "Bounds (the Commitment payload)".

- [ ] **Step 1: Author the script**

```typescript
// genesis/orchestrator/scripts/author-deploy-commitment.ts
import { AdminWebsocket, AppWebsocket } from '@holochain/client';
import inquirer from 'inquirer';

interface CommitmentInput {
  scope: string;          // "republish-epr"
  provider: string;       // operator-steward CID
  recipient: string;      // deploy-svc-agent CID (from provision-deploy-agent output)
  epr_scope: string[];    // EPR identifiers, or ["*"] for bootstrap
  reach_ceiling: string;  // "commons" default
  rate_per_hour: number;  // 30 default
  rotation_ttl_days: number;  // 90 default
  valid_from: string;     // ISO
  valid_until: string;    // ISO
}

async function main() {
  // Interactive prompts — operator-steward fills in
  const answers = await inquirer.prompt<CommitmentInput>([
    { name: 'provider', message: 'Operator-steward agent CID:' },
    { name: 'recipient', message: 'Deploy-svc-agent CID (from provision-deploy-agent output):' },
    { name: 'epr_scope', message: 'EPR scope (comma-separated, or "*" for bootstrap):', filter: s => (s as string).split(',').map(x => x.trim()) },
    { name: 'reach_ceiling', message: 'Reach ceiling:', default: 'commons', choices: ['private', 'self', 'intimate', 'trusted', 'familiar', 'community', 'public', 'commons'] },
    { name: 'rate_per_hour', message: 'Rate per hour:', default: 30, filter: Number },
    { name: 'rotation_ttl_days', message: 'Rotation TTL days:', default: 90, filter: Number },
  ]);

  const validFrom = new Date().toISOString();
  const validUntil = new Date(Date.now() + answers.rotation_ttl_days * 86400 * 1000).toISOString();

  const payload = {
    action: 'delegates-compute',
    scope: 'republish-epr',
    provider: answers.provider,
    recipient: answers.recipient,
    bounds: {
      epr_scope: answers.epr_scope,
      reach_ceiling: answers.reach_ceiling,
      rate_per_hour: answers.rate_per_hour,
      rotation_ttl_days: answers.rotation_ttl_days,
    },
    valid_from: validFrom,
    valid_until: validUntil,
  };

  // Sign via the operator's steward identity (their Lair-managed key)
  const app = await AppWebsocket.connect({ url: new URL('ws://localhost:8888') });
  const result = await app.callZome({
    role_name: 'mishpat',
    zome_name: 'mishpat',
    fn_name: 'create_commitment',
    payload: { action: 'delegates-compute', payload_json: JSON.stringify(payload) },
  });

  console.log(`Commitment CID: ${(result as any).action_hash}`);
  console.log('Stash this CID in Jenkins as "DEPLOY_COMMITMENT_CID" credential for the operator.');
}

main().catch(e => { console.error(e); process.exit(1); });
```

- [ ] **Step 2: Commit**

```bash
git add genesis/orchestrator/scripts/author-deploy-commitment.ts
git commit -m "feat(orchestrator): author-deploy-commitment.ts — operator-steward CLI"
```

---

### Task 10: `stage-spa-blob-zd.ts` (replaces `stageSpaBlob`)

**Files:**
- Create: `genesis/orchestrator/scripts/stage-spa-blob-zd.ts`

**Spec reference:** §2 "CI flow (the stageSpaBlob migration)" — the 8-step diagram.

- [ ] **Step 1: Author the script**

Implement the 8 steps from spec §2 CI-flow diagram. Key sub-parts:

```typescript
// genesis/orchestrator/scripts/stage-spa-blob-zd.ts
import { readFileSync } from 'fs';
import { execSync } from 'child_process';
import { sign } from 'jose';
import { encode as cborEncode } from '@ipld/dag-cbor';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

interface Args {
  distDir: string;          // bundle source
  deploySvcAgentCid: string; // from provision step
  deployCommitmentCid: string; // from author-deploy-commitment step
  privateKeyJwk: string;    // from Jenkins credential
  storageApiBase: string;   // doorway base URL
  manifestPath: string;     // bundle manifest with epr identity + reach
}

async function main(args: Args) {
  // Step 1: Tar + CID-compute the dist directory
  const tarBytes = execSync(`tar c -C ${args.distDir} .`);
  const blobMultihash = await sha256.digest(tarBytes);
  const blobCid = CID.create(1, 0x71, blobMultihash);  // dag-cbor codec
  const blobCidString = `bafkrei${blobCid.toString().slice(7)}`;  // adapt to existing convention

  // Step 2: Upload bytes via content-addressed PUT
  await fetch(`${args.storageApiBase}/blob/${blobCidString}`, {
    method: 'PUT',
    body: tarBytes,
    headers: { 'Content-Type': 'application/octet-stream' },
  });

  // Step 3: Construct EprHead envelope
  const manifest = JSON.parse(readFileSync(args.manifestPath, 'utf-8'));
  const prevEnvelope = await fetch(`${args.storageApiBase}/api/v1/epr/by-identity/${manifest.eprIdentity}/head`).then(r => r.ok ? r.json() : null);
  const envelope = {
    kind: 'Content',
    reach: manifest.reach || 'commons',
    coupling: { knowledge: manifest.eprIdentity },
    payload: { cid: blobCidString },
    supersedes: prevEnvelope?.cid ?? null,
    proof: { signer: args.deploySvcAgentCid },
  };

  // Step 4: Sign envelope canonically with deploy-svc-agent key
  const envelopeBytes = cborEncode(envelope);
  const signature = await sign(envelopeBytes, args.privateKeyJwk);
  envelope.proof.signature = signature;

  // Step 5: Compute EprHead CID
  const eprHeadMultihash = await sha256.digest(envelopeBytes);
  const eprHeadCid = CID.create(1, 0x71, eprHeadMultihash).toString();

  // Step 6: Construct EconomicEvent (republish-epr)
  const event = {
    action: 'republish-epr',
    performer: args.deploySvcAgentCid,
    bounded_by: args.deployCommitmentCid,
    target: eprHeadCid,
    supersedes: prevEnvelope?.cid ?? null,
    payload: {
      blob_cid: blobCidString,
      epr_kind: 'Content',
      reach: envelope.reach,
      bundle_path: manifest.bundlePath,
    },
    signed_at: new Date().toISOString(),
  };

  // Step 7: PUT envelope + event together
  const putResponse = await fetch(`${args.storageApiBase}/api/v1/epr/${eprHeadCid}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ envelope, event }),
  });

  if (!putResponse.ok) {
    const body = await putResponse.text();
    throw new Error(`PUT /api/v1/epr failed (${putResponse.status}): ${body}`);
  }

  console.log(`Z.D deploy succeeded: EprHead=${eprHeadCid}, blob=${blobCidString}`);
}

main(parseArgs()).catch(e => { console.error('Z.D deploy failed:', e); process.exit(1); });
```

- [ ] **Step 2: Test against local dev stack**

Spin up local hc-dev-orchestrator. Run the script against a small bundle. Verify the EprHead lands and the blob serves.

```bash
ts-node genesis/orchestrator/scripts/stage-spa-blob-zd.ts \
  --distDir=./test-bundle \
  --deploySvcAgentCid=$(cat /tmp/deploy-agents/*public.json | jq -r .agentCid) \
  --deployCommitmentCid=<commitment-cid-from-task-9> \
  --privateKeyJwk="$(cat /tmp/deploy-key.json)" \
  --storageApiBase=http://localhost:8888 \
  --manifestPath=./test-bundle/manifest.json
```

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/scripts/stage-spa-blob-zd.ts
git commit -m "feat(orchestrator): stage-spa-blob-zd.ts — substrate-correct deploy script"
```

---

### Task 11: Jenkinsfile wiring

**Files:**
- Modify: `Jenkinsfile` (root)

**Spec reference:** §2 "stageSpaBlob Jenkinsfile rewrite".

- [ ] **Step 1: Find current stageSpaBlob invocation in Jenkinsfile**

```bash
grep -n "stageSpaBlob\|stage-spa-blob" Jenkinsfile | head -10
```

- [ ] **Step 2: Replace invocation**

Replace the existing stageSpaBlob call with:

```groovy
stage('Stage SPA Blob (Z.D substrate-correct)') {
    withCredentials([
        string(credentialsId: 'deploy-svc-matthew-doorway', variable: 'DEPLOY_PRIVATE_KEY_JWK'),
        string(credentialsId: 'DEPLOY_COMMITMENT_CID', variable: 'DEPLOY_COMMITMENT_CID'),
    ]) {
        sh '''
            cd ${WORKSPACE}
            DEPLOY_SVC_AGENT_CID=$(cat genesis/orchestrator/deploy-agents/agent:deploy-svc-matthew-doorway.public.json | jq -r .agentCid)
            ts-node genesis/orchestrator/scripts/stage-spa-blob-zd.ts \
                --distDir=app/elohim-app/dist \
                --deploySvcAgentCid="$DEPLOY_SVC_AGENT_CID" \
                --deployCommitmentCid="$DEPLOY_COMMITMENT_CID" \
                --privateKeyJwk="$DEPLOY_PRIVATE_KEY_JWK" \
                --storageApiBase="https://doorway.elohim.host" \
                --manifestPath=app/elohim-app/dist/manifest.json
        '''
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add Jenkinsfile
git commit -m "feat(ci): Jenkinsfile invokes stage-spa-blob-zd.ts with operator-scoped credentials"
```

---

### Task 12: Delete Z.1 anti-pattern

**Files:**
- Delete: `app/elohim-app/scripts/stage-spa-blob.sh`
- Search for residual PATCH callers

**Spec reference:** §4 (Z.E pre-condition #4).

- [ ] **Step 1: Confirm no other PATCH callers**

```bash
grep -rn "PATCH /db/content\|PATCH.*content/{" app/ doorway/ genesis/orchestrator/ 2>/dev/null | head -10
```

If any remain, list them and decide: migrate to Z.D, or document as Z.E follow-up.

- [ ] **Step 2: Delete the old script**

```bash
git rm app/elohim-app/scripts/stage-spa-blob.sh
```

- [ ] **Step 3: Commit**

```bash
git commit -m "chore(ci): retire Z.1 anti-pattern — delete stage-spa-blob.sh"
```

---

### Task 13: Doorway subscriber — `epr.republished` event arm

**Files:**
- Modify: `doorway/doorway-service/src/projection/subscriber.rs`

**Spec reference:** §2.5 "Doorway-side response: `epr.republished` event".

- [ ] **Step 1: Find existing handle_event match**

```bash
grep -n "handle_event\|content\.created\|content\.updated\|content\.deleted\|projection\." doorway/doorway-service/src/projection/subscriber.rs | head -20
```

- [ ] **Step 2: Add the new arm**

```rust
"epr.republished" => {
    // 1. Fetch the new envelope
    let cid = data["cid"].as_str().ok_or("missing cid")?;
    let envelope = fetch_envelope(&self.storage_client, cid).await?;
    // 2. Check projection contract
    let project_cmt = self.find_project_epr_commitment(envelope.coupling.knowledge.as_deref()).await?;
    if project_cmt.is_none() {
        tracing::debug!(cid, "epr.republished: no active project-epr commitment; skip");
        return Ok(());
    }
    let project_cmt = project_cmt.unwrap();
    // 3. Reach evaluation via reach_evaluator (Task 14)
    let reach_decision = crate::projection::reach_evaluator::evaluate(&envelope, &project_cmt).await?;
    match reach_decision {
        ReachDecision::Match => {
            self.invalidate_app_file_cache(envelope.payload.cid).await?;
            self.invalidate_projected_entries(&envelope).await?;
        }
        ReachDecision::SoftWarn => {
            // §3 ceremony
            self.emit_reach_escalation_pending(cid).await?;
            self.pause_serving_projection(cid).await?;
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/projection/subscriber.rs
git commit -m "feat(doorway): subscriber handles epr.republished event"
```

---

### Task 14: `reach_evaluator.rs` — soft-warn ceremony logic

**Files:**
- Create: `doorway/doorway-service/src/projection/reach_evaluator.rs`
- Modify: `doorway/doorway-service/src/projection/mod.rs`

**Spec reference:** §3 in full.

- [ ] **Step 1: Author with unit tests**

```rust
// doorway/doorway-service/src/projection/reach_evaluator.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReachDecision {
    /// Envelope.reach matches the project-epr commitment's expected reach;
    /// proceed with normal cache refresh.
    Match,
    /// Envelope.reach differs from the commitment's expected reach;
    /// pause serving + emit reach-escalation-pending feedback signal per §3.
    SoftWarn,
}

pub async fn evaluate(envelope: &Envelope, project_cmt: &CommitmentRecord) -> Result<ReachDecision, EvalError> {
    let expected = project_cmt.payload_json
        .get("expected_reach")
        .and_then(|v| v.as_str())
        .unwrap_or("commons");
    if envelope.reach == expected {
        Ok(ReachDecision::Match)
    } else {
        Ok(ReachDecision::SoftWarn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reach_match_returns_match_decision() {
        let env = test_envelope("commons");
        let cmt = test_commitment("commons");
        let result = evaluate(&env, &cmt).await.unwrap();
        assert_eq!(result, ReachDecision::Match);
    }

    #[tokio::test]
    async fn reach_mismatch_returns_softwarn() {
        let env = test_envelope("community");
        let cmt = test_commitment("commons");
        let result = evaluate(&env, &cmt).await.unwrap();
        assert_eq!(result, ReachDecision::SoftWarn);
    }
}
```

- [ ] **Step 2: Run + commit**

```bash
cd doorway/doorway-service && cargo test reach_evaluator 2>&1 | tail -15
git add doorway/doorway-service/src/projection/reach_evaluator.rs doorway/doorway-service/src/projection/mod.rs
git commit -m "feat(doorway): reach_evaluator for §3 soft-warn ceremony"
```

---

### Task 15: Sweettest two-conductor integration

**Files:**
- Create: `elohim/holochain/tests/sweettest/src/substrate_correct_deploy_test.rs`

**Spec reference:** §6 acceptance signals #1-#5.

- [ ] **Step 1: Author the sweettest**

```rust
// elohim/holochain/tests/sweettest/src/substrate_correct_deploy_test.rs
use holochain::sweettest::*;

#[tokio::test(flavor = "multi_thread")]
async fn substrate_correct_deploy_end_to_end() {
    let (steward, deploy_svc) = two_agent_conductors().await;

    // 1. Steward authors delegates-compute Commitment
    let commitment_input = serde_json::json!({
        "action": "delegates-compute",
        "payload_json": serde_json::to_string(&serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            "provider": steward.agent_cid(),
            "recipient": deploy_svc.agent_cid(),
            "bounds": {
                "epr_scope": ["epr:test-bundle"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            },
            "valid_from": "2026-05-01T00:00:00Z",
            "valid_until": "2026-08-01T00:00:00Z"
        })).unwrap()
    });
    let commitment_cid: ActionHash = steward.call_zome("mishpat", "create_commitment", commitment_input).await;

    // 2. DHT consistency
    exchange_peer_info(&[&steward, &deploy_svc]).await;
    await_consistency(60.0, [&steward, &deploy_svc]).await.unwrap();

    // 3. deploy-svc emits republish-epr event referencing the Commitment
    let event_input = serde_json::json!({
        "action": "republish-epr",
        "payload_json": serde_json::to_string(&serde_json::json!({
            "action": "republish-epr",
            "performer": deploy_svc.agent_cid(),
            "bounded_by": commitment_cid.to_string(),
            "target": "epr-head-test",
            "payload": {"blob_cid": "test-blob", "epr_kind": "Content", "reach": "commons"},
            "signed_at": "2026-05-28T12:00:00Z"
        })).unwrap()
    });
    let result: Result<ActionHash, _> = deploy_svc.call_zome("content_store", "create_rea_economic_event", event_input).await;
    assert!(result.is_ok(), "well-formed Z.D event must succeed");

    // 4. Now try a BOUNDS VIOLATION: emit a republish-epr with reach=public (exceeds ceiling=commons)
    let bad_event_input = serde_json::json!({
        "action": "republish-epr",
        "payload_json": serde_json::to_string(&serde_json::json!({
            "action": "republish-epr",
            "performer": deploy_svc.agent_cid(),
            "bounded_by": commitment_cid.to_string(),
            "target": "epr-head-test-2",
            "payload": {"blob_cid": "test-blob-2", "epr_kind": "Content", "reach": "public"},
            "signed_at": "2026-05-28T13:00:00Z"
        })).unwrap()
    });
    let bad_result: Result<ActionHash, _> = deploy_svc.call_zome("content_store", "create_rea_economic_event", bad_event_input).await;
    assert!(bad_result.is_err(), "reach=public against ceiling=commons must be rejected by substrate");
}
```

NOTE: the call_zome paths and signatures depend on actual zome API. Adapt to existing patterns in `elohim/holochain/tests/sweettest/`.

- [ ] **Step 2: Run the sweettest**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__holochain__tests__sweettest/dev \
    cargo test --manifest-path elohim/holochain/tests/sweettest/Cargo.toml substrate_correct_deploy 2>&1 | tail -30
```

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/tests/sweettest/src/substrate_correct_deploy_test.rs
git commit -m "test(sweettest): two-conductor Z.D substrate-correct deploy + bounds-violation"
```

---

### Task 16: A2o scenarios (3 features)

**Files:**
- Create: `genesis/a2o/features/doorway/substrate-correct-deploy.feature`
- Create: `genesis/a2o/features/doorway/bounds-violation-rejection.feature`
- Create: `genesis/a2o/features/doorway/reach-escalation-soft-warn.feature`

**Spec reference:** §6 acceptance signal #7.

- [ ] **Step 1: Author substrate-correct-deploy.feature**

```gherkin
Feature: Substrate-correct deploy via Z.D
  As a doorway operator
  I want CI deploys to use substrate-correct republish-epr events
  So that every deployed bundle has on-chain authority attestation

  Background:
    Given operator "matthew" has provisioned deploy-svc-agent "deploy-svc-matthew-doorway"
    And operator "matthew" has authored a delegates-compute Commitment
      | scope         | republish-epr        |
      | recipient     | deploy-svc-matthew   |
      | reach_ceiling | commons              |
      | rate_per_hour | 30                   |

  Scenario: A standard SPA bundle deploy emits the correct event
    When CI runs stage-spa-blob-zd.ts with bundle "lamad-spa"
    Then PUT /api/v1/epr/{cid} returns 200
    And the EprHead is persisted to the DHT
    And the event references the delegates-compute Commitment via bounded_by
    And alpha.elohim.host serves the new bundle within 30 seconds

  Scenario: A deploy carrying lower-than-ceiling reach is accepted
    When CI runs stage-spa-blob-zd.ts with reach "community"
    Then the event is accepted (community is within ceiling=commons)
```

- [ ] **Step 2: Author bounds-violation-rejection.feature**

```gherkin
Feature: Substrate refuses out-of-bounds republish
  As a substrate validator
  I want to reject events that violate their bounding Commitment
  So that compromised deploy-svc-agents cannot silently escalate reach or exceed rate

  Scenario: Reach escalation above ceiling is rejected
    Given the delegates-compute Commitment has reach_ceiling="commons"
    When CI attempts a republish with reach="public"
    Then PUT /api/v1/epr/{cid} returns 400
    And the response body names violation kind "reach_ceiling_exceeded"
    And the EprHead is NOT persisted to the DHT

  Scenario: Rate-limit breach is rejected with FeedbackSignal
    Given the delegates-compute Commitment has rate_per_hour=30
    And 30 republish-epr events have been emitted in the last hour
    When CI attempts a 31st republish
    Then PUT /api/v1/epr/{cid} returns 400 with violation "rate_limit_exceeded"
    And a rate-limit-exceeded FeedbackSignal is emitted naming deploy-svc-matthew

  Scenario: Revoked Commitment immediately stops accepting events
    Given the delegates-compute Commitment is active
    When operator-steward revokes the Commitment via Mishpat
    Then the next republish-epr event from deploy-svc-matthew is rejected
    And the violation kind is "commitment_revoked"
```

- [ ] **Step 3: Author reach-escalation-soft-warn.feature**

```gherkin
Feature: Reach-change soft-warn ceremony
  As a doorway operator
  I want intentional reach changes to require explicit stewardship acknowledgement
  So that silent reach escalation is impossible

  Scenario: Reach change pauses serving until acknowledged
    Given a course module is published with reach="private"
    When CI re-deploys the same EPR with reach="community"
    Then doorway STOPS serving the projection
    And doorway returns 503 with reach-escalation marker
    And a reach-escalation-pending FeedbackSignal is emitted

  Scenario: Steward acknowledgement resumes serving
    Given a reach-escalation-pending FeedbackSignal is active
    When operator-steward publishes acknowledges-reach-change Commitment for the EPR
    Then doorway resumes serving the new reach
    And the new reach is checked at every request
```

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/doorway/substrate-correct-deploy.feature \
        genesis/a2o/features/doorway/bounds-violation-rejection.feature \
        genesis/a2o/features/doorway/reach-escalation-soft-warn.feature
git commit -m "test(a2o): Z.D substrate-correct deploy + bounds-violation + reach-escalation features"
```

---

### Task 17: Close-out + memory updates

**Files:**
- Create: `genesis/docs/research/2026-05-28-sprint1-zd-implementation-notes.md`
- Update: `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` (mark S1 complete)
- Update: `.claude/memory/project_rea_compute_commitment_primitive.md` (status: first instance landed)
- Create: `.claude/memory/project_zd_substrate_correct_deploy_pattern.md`
- Update: `.claude/memory/MEMORY.md`

- [ ] **Step 1: Author Sprint 1 implementation notes**

Capture: commit SHAs, what landed, what's deferred, lessons learned.

- [ ] **Step 2: Update parent roadmap**

Change `S1 ▢` → `S1 ✓` in the Phase A table.

- [ ] **Step 3: Update REA compute-commitment primitive memory**

Add a `**First-instance status (2026-MM-DD):**` section noting Z.D end-to-end has shipped and the pattern is proven on one row of the generalization table.

- [ ] **Step 4: Create pattern memory**

`.claude/memory/project_zd_substrate_correct_deploy_pattern.md`:

```markdown
---
name: zd-substrate-correct-deploy-pattern
description: "The Z.D shape: CI signs an EprHead envelope, emits a republish-epr EconomicEvent bounded by a delegates-compute Commitment, PUTs to /api/v1/epr/{cid}. Substrate validates bounds via bounds_validator. First concrete instance of the REA compute-commitment primitive."
metadata:
  node_type: memory
  type: project
---

The Z.D shape replaces the Z.1 anti-pattern (PATCH /db/content/{slug}) with a substrate-correct deploy flow:

1. Operator-steward authors a Mishpat::Commitment with action=delegates-compute, scope=republish-epr, bounds={epr_scope, reach_ceiling, rate_per_hour, rotation_ttl_days}.
2. CI signs an EprHead envelope with a per-operator deploy-svc-agent Ed25519 key.
3. CI emits an EconomicEvent with action=republish-epr and bounded_by=<commitment CID>.
4. PUT /api/v1/epr/{cid} validates the event against the Commitment via bounds_validator.
5. The doorway's epr.republished arm refreshes caches; reach mismatch fires the §3 soft-warn ceremony.

This is the first concrete instance of the REA compute-commitment primitive per `[[project_rea_compute_commitment_primitive]]`. The other six rows of the gospel-tier generalization table inherit this shape — see the parent roadmap Sprints 3 (serve-url-projection) and 5a-e for the remaining instances.

**Reference:** `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` (spec); `genesis/docs/superpowers/plans/2026-05-28-sprint1-zd-substrate-correct-deploy.md` (this plan).

**Related:** `[[project_rea_compute_commitment_primitive]]` (the gospel-tier shape), `[[project_bounds_validator_pattern]]` (the substrate-side validator Z.D delegates to), `[[project_signal_kind_extensible_protocol_class]]` (the rate-limit-exceeded / bad-custody / reach-escalation-pending FeedbackSignal extensions).
```

- [ ] **Step 5: Commit**

```bash
git add genesis/docs/research/2026-05-28-sprint1-zd-implementation-notes.md \
        genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md \
        .claude/memory/project_rea_compute_commitment_primitive.md \
        .claude/memory/project_zd_substrate_correct_deploy_pattern.md \
        .claude/memory/MEMORY.md
git commit -m "docs(memory): Sprint 1 close-out + Z.D pattern memory"
```

---

# Self-Review

**Spec coverage:** every section of the Z.D spec is covered by a task above. §1 (the gospel-tier primitive) is referenced as canon; the implementation work lives at the §2 instance level. §2.5 (doorway-side `epr.republished` arm) → Task 13. §3 (soft-warn ceremony) → Task 14. §6 acceptance signals: #1 (provisioning) → Task 8, #2 (Commitment authoring) → Task 9, #3 (stageSpaBlob Z.D shape) → Task 10, #4 (doorway subscriber handling) → Task 13, #5 (bounds-violation rejection) → Tasks 7+15, #6 (Pattern Z bridge unchanged) → no work needed (additive), #7 (a2o scenarios) → Task 16, #8 (B15 unchanged) → no work needed (additive).

**Placeholder scan:** no "TBD" or "TODO later" inside task steps. Two explicit deferrals: (a) `derive_epr_identity_from_envelope` helper in Task 7 step 3 — references existing `elohim/epr/src/identity.rs` convention; if that helper doesn't exist yet, the implementer creates it inline (small). (b) the Sweettest `call_zome` signatures in Task 15 may need adaptation to the actual sweettest harness pattern — captured in the Step 1 NOTE.

**Type consistency:** `CommitmentRecord` (from Sprint 2) used in Tasks 6 (republish_epr_validator) and 14 (reach_evaluator). `EventForValidation` (from Sprint 2) constructed in Task 6. `BoundsViolation` / `ValidationError::Bounds` consistent in Tasks 6, 7, 15. `ReachDecision::{Match, SoftWarn}` enum from Task 14 used in Task 13.

# Execution Handoff

**Plan saved to** `genesis/docs/superpowers/plans/2026-05-28-sprint1-zd-substrate-correct-deploy.md`.

**Recommended execution path:** Subagent-Driven with rust-architect for Tasks 1-7 (zome work + storage handler integration; these are the substrate keystone) and angular-architect / general-purpose for Tasks 8-12 (TypeScript CI scripts). Tasks 13-14 are doorway-service work — code-reviewer + red-team (adversarial probes on reach evaluation + spoofing). Task 15 sweettest needs careful operator-watch since it involves the sweettest harness which can be flaky. Task 16 a2o scenarios benefit from operator narrative judgment (per `feedback_a2o_narrative_is_opus_work` — Opus, not Haiku, for the feature-file authoring).

**Parallel opportunities:**
- Tasks 1-3 (Mishpat zome) ← parallel with → Tasks 4-5 (elohim DNA zome) — different DNAs, no shared files.
- Tasks 8-10 (CI scripts) can author in parallel; only Task 11 (Jenkinsfile) consumes all three so it serializes after.
- Task 16 (a2o features) can start as soon as the spec scenarios are agreed; doesn't block implementation.

**Cross-sprint coordination with Sprint 2:** Task 6 (republish_epr_validator) depends on Sprint 2 Task 4's `bounds_validator::validate` API. If Sprint 1 runs concurrently with Sprint 2, ensure Sprint 2 Task 4 lands before Sprint 1 Task 6 implementer starts. If sequential, Sprint 2 completes first; Sprint 1 starts with a stable trait surface.

**Done when:** spec §6 acceptance signals all satisfied. A real CI run on alpha completes the Z.D flow end-to-end. The bounds-violation a2o scenario passes (substrate refuses the out-of-bounds republish). The reach-escalation a2o scenario passes (soft-warn ceremony fires + acknowledgement resumes serving). The Z.1 PATCH /db/content path is deleted from the codebase. Z.D substrate-correct deploy is now the first proven instance of the REA compute-commitment primitive.

---

## Close-out — Sprint 1 substrate-only landing (2026-05-28)

**Sprint 1 ended scope-narrowed:** primitives shipped; Z.D-as-first-instance abandoned mid-flight after a design conversation reframed the compute-commitment use case.

### What landed

| Task | SHA | Subject |
|------|-----|---------|
| T1+T2 | eb7e191df | feat(mishpat): Commitment entry type + delegates-compute coordinator + integrity validator |
| T3 | 4bf2dcdbf | feat(mishpat): acknowledges-reach-change Commitment action + validator |
| T4 | 1e395e876 | feat(elohim-dna): republish-epr EconomicEvent action discriminator + validator |
| T5 | 383c79891 | feat(elohim-integrity): defense-in-depth republish-epr bounded_by requirement |
| T6 | 0a9d373cc | feat(storage): republish_epr_validator delegates to bounds_validator + schema check |
| T7 | b481ed1df | feat(storage): put_epr handler invokes republish_epr_validator for Z.D events |
| revert | 5259de31c | revert: drop Z.D blueprint TS scripts |

Total: 6 substrate commits + 1 revert.

### Substrate primitives that are now ready for any real first instance

- **`Mishpat::Commitment`** — new DHT entry type with action discriminator. Two actions validated: `delegates-compute` (full schema check: scope, provider, recipient, bounds.epr_scope, bounds.reach_ceiling, bounds.rate_per_hour, bounds.rotation_ttl_days, reach_elevation_acknowledged guard) and `acknowledges-reach-change` (acknowledger, target_epr_cid, new_reach enum, signed_at). Coordinator + integrity defense-in-depth.
- **elohim DNA `EconomicEvent` action discriminator `republish-epr`** — schema check at coordinator (full required-fields + enum-membership) and integrity (defense-in-depth: non-empty bounded_by, anonymous-publish-forbidden). Wired into `create_rea_economic_event` dispatch.
- **`republish_epr_validator` service** in elohim-storage — reference implementation of the per-instance validator shape (schema check → `EventForValidation` projection → delegate to `bounds_validator::validate`). Reusable as a template for Sprints 3 + 5a-e per-instance validators against the bounds-validator-pattern.
- **`put_epr` substrate-correct 503** — when an `event` is present on `EprPublishInput`, the handler attempts bounds validation. Returns 503 substrate-correctly until the conductor bridge wires `mishpat::get_commitment` through to `ConductorCommitmentFetcher`. Harmless when no caller sends `event`.

### What was scoped out and why

**Z.D (SPA bundle deploy as first instance of the compute-commitment pattern) was abandoned mid-sprint.** The design conversation surfaced that SPA bundle deploy doesn't actually fit the three real use cases the operator named for REA compute-commitments:

1. **Mutual storage replication between family-network peers** — bounded reciprocity contracts ("I host N GB for you; you host M GB for me"); bounds_validator enforces symmetry; FeedbackSignals (rate-limit-exceeded, bad-custody) carry meaningful weight.
2. **Doorway projection compute agreements** — when doorway projects EPR-app content (SSR, cache occupancy), the compute cost flows to the stewards/collectives that approved the agreement; needs a metering model on doorway first.
3. **Distributed workloads** — REA agreements for compute tasks (Jenkins-shape distributed workload but using EPR-components / storage commitments / p2p sccache instead of pods); most ambitious; needs the peer-bidding + scheduling layer designed first.

Deploy doesn't fit any of these. It's not mutual (no reciprocity from a deploy-svc bot back to the operator-steward), not ongoing projection compute (one-shot publish), not distributed workload (single-CI publish). The Z.D framing slid into the compute-commitment pattern because the spec was written that way, but it was miscasting an authorship-delegation as a compute-delegation.

**Deploy stays on the existing Z.1 path** (`stageSpaBlobs` in Jenkinsfile:223 → PUT `/admin/seed/blob` + PATCH `/db/content/{slug}`) until a substrate-correct content-publish replacement lands. That replacement is naturally seeder-based: seeder creates a `Content` node via the conductor with `contentType=spa-bundle` and the new blobHash; post-commit projection picks it up; doorway serves via the normal slug→content lookup; operator-steward's signing identity IS the authority attestation. No new Commitment shape needed for content publishing — the existing notarized identity-bound creation suffices. **This is a separate, smaller sprint.**

### Recommended first real instance: mutual storage replication

The next-bootstrap step is **compute agreements between peers in the family-network** so the resiliency epics can be proven end-to-end:

- Each peer needs to compute aggregate views: **free-storage capacity vs stewarded-compute commitments** (am I over-committed? do I have headroom?)
- Each piece of content needs to compute **resiliency and delivery metrics** (how many active replication commitments cover this CID? what's the steward-graph distance? what's the projected fetch latency given peer reach?)
- bounds_validator runs on every replication commitment author: capacity-exceeded? scope-includes-this-CID? rate-not-exceeded (replications-per-hour)? key-rotation-current?
- Standing debits via signal_weight_registry: `bad-custody` (peer revoked unilaterally), `rate-limit-exceeded` (peer over-committed and dropped), `reach-escalation-pending` if a peer tries to escalate from intimate to commons-reach replication without acknowledgement

The deferred Sprint 1 tasks (T8–T16 from this plan) become moot when the first instance is storage replication — the CI deploy scripts and Jenkinsfile wiring were Z.D-specific. Replace with a new plan: `replicates-storage` action authoring, peer-to-peer replication handshake, capacity-aggregate views, content-resiliency views.

### Scoped-out and removed

- **T8, T9, T10** — Z.D TS blueprint scripts (provision-deploy-agent, author-deploy-commitment, stage-spa-blob-zd). Landed in 4e3604a66, reverted in 5259de31c. Not preserved as documentation — the schema-shape they document lives in the JSON schemas directly; preserving them as orphaned code would mislead future readers.
- **T11, T12** — Jenkinsfile changes (Z.D blueprint stage + transitional comment on stageSpaBlobs). Never landed. The Jenkinsfile is untouched; CI deploy continues unchanged.
- **T13, T14** — doorway subscriber `epr.republished` arm + `reach_evaluator`. Never landed. The §3 soft-warn ceremony was Z.D-specific (envelope.reach vs project-epr commitment expected_reach); the storage-replication use case has its own reach semantics that need fresh design.
- **T15** — sweettest two-conductor Z.D integration. Never landed. The storage-replication use case will have its own sweettest.
- **T16** — three a2o feature files for Z.D. Never landed. The storage-replication use case will have its own a2o scenarios.

### Follow-ups (captured for future sprints)

- **Seeder-based substrate-correct content publish** to retire the `stageSpaBlobs` Z.1 path. Small sprint. Doesn't depend on Sprint 1 substrate work.
- **Storage replication first-instance sprint** using Sprint 1's primitives. Needs design first: replication-commitment shape, peer-handshake protocol, capacity-aggregate view, content-resiliency view. Bounds_validator and signal_weight_registry already cover the bounds-checking and standing pieces.
- **`ConductorCommitmentFetcher` ↔ `mishpat::get_commitment`** wiring through `HcClientRegistry` to make `put_epr`'s republish-epr path live. Only useful once a real caller sends `event` on `EprPublishInput`. Defer until storage replication or another use case actually needs it.
- **EprPublishInput's `RepublishEprEventView`** carries `payload: JsonVal` (the ts-rs-safe wrapper). When a real wire-format consumer arrives, consider whether to narrow this to typed fields.
