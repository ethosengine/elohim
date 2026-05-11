# Light Up the Graph — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Phase 3.5's substrate primitives (back_prop, gossip_flood, standing projector, tending) into live runtime so that a real `FeedbackSignal` arriving via `PUT /api/v1/epr` produces every downstream effect end-to-end. Lift the two T20 mocks; aunt-and-rage-bait runs without direct service substitutions.

**Architecture:** Six wiring sites, no new entities. `api/epr.rs` fan-out, `main.rs` reconciliation startup, ManifestDebitWeightPolicy at the projection seam, LibP2P swarm adapters wrapping the existing P2PCommand mpsc, reach-earning gate on the author compose path, Vouch as a new `signal_kind` variant on FeedbackSignal in the `content_store` zome.

**Tech Stack:** Rust (elohim-storage), libp2p 0.54, Holochain HDK/HDI, diesel + SQLite, tokio, dryoc 2-of-2 sealing, JSON Schema + ts-rs codegen.

**Spec:** `genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md`

**Worktree:** `/projects/elohim/.claude/worktrees/light-up-graph` on branch `feature/light-up-graph`. All commands assume cwd = worktree root.

**Build flag:** `elohim-storage` requires `RUSTFLAGS='--cfg getrandom_backend="custom"'` per CLAUDE.md gotcha.

---

## File Structure

### Source-of-truth note (no new storage entities)

This plan introduces **no new SQLite tables, no new diesel migrations, and no new DHT entry types**. All "schema" references below are either:

- **JSON Schemas for wire formats** (`p2p/feedback-signal.schema.json` extension) — source of truth is the existing `FeedbackSignal` DHT entry type in `content_store_integrity` (Notarized / Category A — declared during Phase 3.5 T4).
- **JSON Schemas for manifest content** (`bootstrap-standing-policy.json` extension) — source of truth is the existing `Manifest` DHT entry type (declared by the Phase 3.5 T7 manifest validator extension).
- **Rust mirrors of those wire formats** (`p2p/feedback_signal.rs`) — projection types, not storage entities.

The Vouch primitive REUSES the existing FeedbackSignal entry type as a new `signal_kind` variant. P2P design gate output for entities is in the [spec](../specs/2026-05-01-light-up-the-graph-design.md#p2p-design-gate-output). DNA capacity is preserved (Lamad ~73/100 unchanged).

### New files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/services/reach_earning.rs` | `ReachVerdict` + `evaluate()` substrate gate |
| `elohim/elohim-storage/src/services/epr_compose.rs` | Author-side `compose_epr` wrapper that consults the gate |
| `elohim/elohim-storage/src/p2p/adapters.rs` | `LibP2POutboundSink` + `LibP2PGossipPublisher` production impls |
| `elohim/elohim-storage/tests/startup_wiring.rs` | Smoke test for seed_if_empty + TTL sweep |

### Modified files
| Path | What changes |
|------|--------------|
| `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` | Add `vouch` to enum + optional `vouchKind` |
| `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json` | Add `debitWeights.vouch`, `unknownTreatment`, `reachThresholds` |
| `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json` | Schema extensions for new fields |
| `elohim/elohim-storage/src/p2p/feedback_signal.rs` | Add `Vouch` variant + `vouch_kind` field |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` | Validator extension (vouch + no-self-vouch) |
| `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs` | New `create_vouch` coordinator function |
| `elohim/elohim-storage/src/services/manifest_registry.rs` | New accessors for debit weights, thresholds, etc. |
| `elohim/elohim-storage/src/services/standing_projector.rs` | New `ManifestDebitWeightPolicy` |
| `elohim/elohim-storage/src/services/standing.rs` | Add `Standing::with_lift` |
| `elohim/elohim-storage/src/services/epr_kind.rs` (or wherever `Reach` lives) | Add `Reach::is_floor_allowed` |
| `elohim/elohim-storage/src/services/tending.rs` | Implement `sweep_expired` |
| `elohim/elohim-storage/src/p2p/mod.rs` | Add `P2PCommand::SendDirect` / `GossipPublish` if missing |
| `elohim/elohim-storage/src/api/epr.rs` | FeedbackSignal arrival fan-out + local-origin dedup + transaction |
| `elohim/elohim-storage/src/main.rs` | seed_if_empty wiring + TTL sweep task + adapter construction |
| `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs` | Lift both mocks |
| `elohim/elohim-storage/src/services/mod.rs` | Re-export new services |

---

## Task 1: Extend feedback-signal schema with vouch variant

**Files:**
- Modify: `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json`

- [x] **Step 1: Read current schema**

```bash
cat elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json
```

- [x] **Step 2: Add `vouch` to the `signalKind` enum and optional `vouchKind` field**

Edit the schema to add `"vouch"` to the `signalKind` enum and add an optional `vouchKind` property:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "...",
  "title": "FeedbackSignal",
  "type": "object",
  "required": ["targetCid", "signalKind", "standingImpact", "signedBy", "signature"],
  "properties": {
    "targetCid": { "type": "string" },
    "signalKind": {
      "type": "string",
      "enum": ["squelch", "correction", "retraction", "quarantine", "vouch"]
    },
    "vouchKind": {
      "type": "string",
      "enum": ["accept-correction", "restitution"],
      "description": "Required iff signalKind == 'vouch'. Distinguishes vouch sub-semantics."
    },
    "evidenceCid": { "type": ["string", "null"] },
    "standingImpact": { "type": "string", "enum": ["advisory", "debit-soft", "debit-firm"] },
    "signedBy": { "type": "string" },
    "signature": { "type": "string" }
  },
  "allOf": [
    {
      "if": { "properties": { "signalKind": { "const": "correction" } } },
      "then": { "required": ["evidenceCid"] }
    },
    {
      "if": { "properties": { "signalKind": { "const": "vouch" } } },
      "then": { "required": ["vouchKind"] }
    },
    {
      "if": { "not": { "properties": { "signalKind": { "const": "vouch" } } } },
      "then": { "not": { "required": ["vouchKind"] } }
    }
  ]
}
```

Preserve any existing top-level fields/comments not shown.

- [x] **Step 3: Validate the schema**

```bash
pnpm run schema:validate
```
Expected: PASS (no schema validation errors).

- [x] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json
git commit -m "feat(epr-light-up): T01 — add vouch signal_kind variant to FeedbackSignal schema"
```

---

## Task 2: Extend bootstrap-standing-policy with new fields

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json`
- Modify: `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json`

- [x] **Step 1: Read both files first**

```bash
cat elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json
cat elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json
```

- [x] **Step 2: Add `vouch` to `debitWeights`, plus `unknownTreatment` and `reachThresholds` to bootstrap-standing-policy.json**

Add these three blocks (preserve the rest of the file, including the existing `floor` and `newVoiceBaseline` sections):

```jsonc
{
  /* ...existing manifestKind, revision, floor, newVoiceBaseline... */
  "debitWeights": {
    "squelch":     { "advisory": 0, "debit-soft": 1,  "debit-firm": 3 },
    "correction":  { "advisory": 0, "debit-soft": 10, "debit-firm": 20 },
    "retraction":  { "advisory": 0, "debit-soft": -5, "debit-firm": -10 },
    "quarantine":  { "advisory": 0, "debit-soft": 12, "debit-firm": 30 },
    "vouch":       { "advisory": 0, "debit-soft": -3, "debit-firm": -8 }
  },
  "unknownTreatment": {
    "default": "conservative",
    "evidenceSources": []
  },
  "reachThresholds": {
    "personal":     "any",
    "intimate":     "any",
    "household":    "any",
    "neighborhood": "any",
    "collective":   "neutral",
    "community":    "neutral",
    "district":     "neutral",
    "public":       "high"
  }
}
```

Replace any existing `debitWeights` block in full with the version above. Add `unknownTreatment` and `reachThresholds` if not present.

- [x] **Step 3: Extend the schema (`standing-policy-floor.schema.json`)**

Add property definitions matching the new manifest fields. The schema validates the bootstrap manifest, so the schema must permit the new fields:

```jsonc
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "...",
  "title": "StandingPolicyFloor",
  "type": "object",
  "required": ["manifestKind", "revision", "floor", "debitWeights"],
  "properties": {
    "manifestKind": { "const": "standing-policy" },
    "revision": { "type": "integer", "minimum": 1 },
    "floor": { /* existing */ },
    "newVoiceBaseline": { /* existing */ },
    "debitWeights": {
      "type": "object",
      "required": ["squelch", "correction", "retraction", "quarantine", "vouch"],
      "additionalProperties": {
        "type": "object",
        "required": ["advisory", "debit-soft", "debit-firm"],
        "properties": {
          "advisory":   { "type": "integer" },
          "debit-soft": { "type": "integer" },
          "debit-firm": { "type": "integer" }
        }
      }
    },
    "unknownTreatment": {
      "type": "object",
      "required": ["default", "evidenceSources"],
      "properties": {
        "default": { "type": "string", "enum": ["conservative", "newVoiceBaseline", "neutral"] },
        "evidenceSources": { "type": "array", "items": { "type": "string" } }
      }
    },
    "reachThresholds": {
      "type": "object",
      "additionalProperties": {
        "type": "string",
        "enum": ["any", "floor", "low", "neutral", "high", "trusted"]
      }
    }
  }
}
```

- [x] **Step 4: Validate**

```bash
pnpm run schema:validate
pnpm run schema:check-dna
```
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json \
        elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json
git commit -m "feat(epr-light-up): T02 — add vouch debit weights, unknownTreatment, reachThresholds"
```

---

## Task 3: Regenerate TypeScript bindings and verify codegen clean

**Files:**
- Modify (auto): `elohim/sdk/schemas/generated-ts/**` and TS distributions

- [x] **Step 1: Run codegen**

```bash
pnpm run schema:codegen:ts
```
Expected: PASS. Generated TS files reflect the new `signalKind` enum value.

- [x] **Step 2: Inspect what changed**

```bash
git diff --stat elohim/sdk/schemas/generated-ts/
```
Expected: small number of files in `generated-ts/p2p/feedback-signal*.ts` updated.

- [x] **Step 3: Verify the generated TS has the new variant**

```bash
grep -A2 "signalKind" elohim/sdk/schemas/generated-ts/p2p/feedback-signal.ts | head -20
```
Expected: enum literal includes `'vouch'`.

- [x] **Step 4: Commit codegen artifacts**

```bash
git add elohim/sdk/schemas/generated-ts/
git commit -m "chore(epr-light-up): T03 — regenerate TS bindings for vouch signal_kind"
```

---

## Task 4: Extend Rust mirror of FeedbackSignal with vouch + vouch_kind

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/feedback_signal.rs`

- [x] **Step 1: Read the current Rust mirror**

```bash
cat elohim/elohim-storage/src/p2p/feedback_signal.rs | head -120
```

- [x] **Step 2: Add `Vouch` variant to `SignalKind` and `vouch_kind: Option<VouchKind>` to FeedbackSignal**

Edit `elohim/elohim-storage/src/p2p/feedback_signal.rs`:

Add to the `SignalKind` enum (preserving existing variants):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalKind {
    Squelch,
    Correction,
    Retraction,
    Quarantine,
    Vouch,
}
```

Add a new enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VouchKind {
    AcceptCorrection,
    Restitution,
}
```

Add to the `FeedbackSignal` struct (between existing fields):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackSignal {
    pub target_cid: String,
    pub signal_kind: SignalKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vouch_kind: Option<VouchKind>,
    pub evidence_cid: Option<String>,
    pub standing_impact: StandingImpact,
    pub signed_by: String,
    pub signature: String,
}
```

- [x] **Step 3: Add a unit test that round-trips a vouch FeedbackSignal**

In the same file's `#[cfg(test)] mod tests`:

```rust
#[test]
fn vouch_signal_round_trips() {
    let signal = FeedbackSignal {
        target_cid: "bafyreitarget".to_string(),
        signal_kind: SignalKind::Vouch,
        vouch_kind: Some(VouchKind::AcceptCorrection),
        evidence_cid: None,
        standing_impact: StandingImpact::DebitSoft,
        signed_by: "AAA=".to_string(),
        signature: "BBB=".to_string(),
    };
    let json = serde_json::to_string(&signal).unwrap();
    assert!(json.contains("\"signalKind\":\"vouch\""));
    assert!(json.contains("\"vouchKind\":\"accept-correction\""));
    let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
    assert_eq!(back, signal);
}

#[test]
fn non_vouch_signals_omit_vouch_kind() {
    let signal = FeedbackSignal {
        target_cid: "bafyreitarget".to_string(),
        signal_kind: SignalKind::Correction,
        vouch_kind: None,
        evidence_cid: Some("bafyreievidence".to_string()),
        standing_impact: StandingImpact::DebitSoft,
        signed_by: "AAA=".to_string(),
        signature: "BBB=".to_string(),
    };
    let json = serde_json::to_string(&signal).unwrap();
    assert!(!json.contains("vouchKind"), "vouchKind must be omitted when None");
}
```

- [x] **Step 4: Run the new tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib feedback_signal::tests::vouch -- --nocapture
```
Expected: PASS.

- [x] **Step 5: Run all p2p::feedback_signal tests + lints**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib p2p::feedback_signal
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
```
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/feedback_signal.rs
git commit -m "feat(epr-light-up): T04 — add Vouch variant to SignalKind + VouchKind enum"
```

---

## Task 5: Extend integrity validator with vouch validation

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs`

- [x] **Step 1: Read the current validator**

```bash
cat elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs
```

- [x] **Step 2: Update the SignalKind whitelist + add vouch validation**

Locate the validator function (commonly `pub fn validate_create_feedback_signal` or `validate_feedback_signal`) and:

1. Extend the `signal_kind` whitelist to include `"vouch"`.
2. Add `pub vouch_kind: Option<String>` to the `FeedbackSignal` integrity struct.
3. Add the `vouch_kind` validation logic.

Code shape (adapt to the actual struct + validator location):

```rust
// Integrity entry struct:
#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct FeedbackSignal {
    pub target_cid: String,
    pub signal_kind: String,
    pub vouch_kind: Option<String>,    // NEW
    pub evidence_cid: Option<String>,
    pub standing_impact: String,
    pub signed_by: Vec<u8>,
    pub signature: Vec<u8>,
}

pub fn validate_create_feedback_signal(
    action: &SignedActionHashed,
    fs: &FeedbackSignal,
) -> ExternResult<ValidateCallbackResult> {
    // Whitelist signal_kind
    const VALID_KINDS: &[&str] = &["squelch", "correction", "retraction", "quarantine", "vouch"];
    if !VALID_KINDS.contains(&fs.signal_kind.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "invalid signal_kind: {}", fs.signal_kind)));
    }

    // standing_impact whitelist (existing)
    const VALID_IMPACTS: &[&str] = &["advisory", "debit-soft", "debit-firm"];
    if !VALID_IMPACTS.contains(&fs.standing_impact.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "invalid standing_impact: {}", fs.standing_impact)));
    }

    // squelch ⇒ advisory only (existing)
    if fs.signal_kind == "squelch" && fs.standing_impact != "advisory" {
        return Ok(ValidateCallbackResult::Invalid(
            "squelch requires standing_impact=advisory".into()));
    }

    // correction ⇒ evidence_cid required (existing)
    if fs.signal_kind == "correction" && fs.evidence_cid.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "correction requires evidence_cid".into()));
    }

    // NEW: vouch validation
    if fs.signal_kind == "vouch" {
        // vouch_kind required
        let vk = match &fs.vouch_kind {
            Some(v) => v,
            None => return Ok(ValidateCallbackResult::Invalid(
                "vouch requires vouch_kind".into())),
        };
        const VALID_VOUCH_KINDS: &[&str] = &["accept-correction", "restitution"];
        if !VALID_VOUCH_KINDS.contains(&vk.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "invalid vouch_kind: {}", vk)));
        }
        // Resolve target → original FeedbackSignal → enforce no-self-vouch
        let target_action_hash = ActionHash::try_from(fs.target_cid.clone())
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
                "target_cid not an ActionHash: {}", e))))?;
        let target_record = must_get_valid_record(target_action_hash)?;
        let target_signal: FeedbackSignal = target_record.entry().to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("target decode: {}", e))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                "target_cid does not point to a FeedbackSignal".into())))?;
        if fs.signed_by == target_signal.signed_by {
            return Ok(ValidateCallbackResult::Invalid(
                "self-vouch forbidden: signer must differ from target.signed_by".into()));
        }
    } else {
        // NEW: non-vouch must NOT carry vouch_kind
        if fs.vouch_kind.is_some() {
            return Ok(ValidateCallbackResult::Invalid(
                "vouch_kind set on non-vouch signal".into()));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
```

- [x] **Step 3: Build the integrity zome**

```bash
cd elohim/holochain/dna/elohim/zomes/content_store_integrity
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
```
Expected: PASS.

- [x] **Step 4: Run integrity unit tests if any exist**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs
git commit -m "feat(epr-light-up): T05 — extend FeedbackSignal validator with vouch + no-self-vouch"
```

---

## Task 6: Add `create_vouch` coordinator function

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs`

- [x] **Step 1: Read the existing coordinator**

```bash
cat elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs
```

- [x] **Step 2: Add `CreateVouchInput` struct and `create_vouch` function**

Add to the same file (preserving existing items):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVouchInput {
    /// ActionHash (encoded as string) of the target FeedbackSignal being vouched on.
    pub target_signal_cid: String,
    /// "accept-correction" or "restitution".
    pub vouch_kind: String,
    /// "advisory", "debit-soft", or "debit-firm".
    pub standing_impact: String,
    /// Signature over the canonical envelope (caller-supplied; signer must match agent_info).
    pub signature: Vec<u8>,
}

#[hdk_extern]
pub fn create_vouch(input: CreateVouchInput) -> ExternResult<ActionHash> {
    // Resolve target — also validates it's a real action.
    let target_action_hash = ActionHash::try_from(input.target_signal_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "target_signal_cid invalid ActionHash: {}", e))))?;
    let target_record = must_get_valid_record(target_action_hash.clone())?;
    let target_signal: FeedbackSignal = target_record.entry().to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("target decode: {}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "target_signal_cid not a FeedbackSignal".into())))?;

    // Derive signer from agent_info — caller cannot spoof.
    let signer = agent_info()?.agent_initial_pubkey;
    let signer_bytes = signer.get_raw_39().to_vec();

    // Coordinator-side guard (validator backstops; this returns a clean error to the caller).
    if signer_bytes == target_signal.signed_by {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "self-vouch forbidden".into())));
    }

    let signal = FeedbackSignal {
        target_cid: input.target_signal_cid,
        signal_kind: "vouch".to_string(),
        vouch_kind: Some(input.vouch_kind),
        evidence_cid: None,
        standing_impact: input.standing_impact,
        signed_by: signer_bytes,
        signature: input.signature,
    };
    let action_hash = create_entry(&EntryTypes::FeedbackSignal(signal.clone()))?;

    // Emit projection signal for storage-side consumption.
    emit_signal(ProjectionSignal::FeedbackSignalCommitted {
        action_hash: action_hash.clone(),
        signal,
    })?;

    // Link to target so the target's record can be queried for vouches.
    create_link(
        target_action_hash,
        action_hash.clone(),
        LinkTypes::TargetToFeedbackSignal,
        LinkTag::new("vouch"),
    )?;

    Ok(action_hash)
}
```

(Adapt `EntryTypes`, `LinkTypes`, `ProjectionSignal` to the existing module's actual paths/names — see neighbouring `create_feedback_signal` function for reference.)

- [x] **Step 3: Build the coordinator zome**

```bash
cd elohim/holochain/dna/elohim/zomes/content_store
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
```
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs
git commit -m "feat(epr-light-up): T06 — add content_store::create_vouch coordinator function"
```

---

## Task 7: Sweettest scaffolding for create_vouch

**Files:**
- Locate the existing FeedbackSignal sweettest (likely `elohim/holochain/dna/elohim/tests/feedback_signal.rs` or similar). If absent, create it.

- [x] **Step 1: Find existing sweettest**

```bash
find elohim/holochain/dna -name "*.rs" -path "*tests*" -exec grep -l "feedback_signal\|create_feedback_signal" {} \;
```

- [x] **Step 2: Add a vouch sweettest case**

Append to the discovered file (or create a new `tests/vouch.rs` module if no existing test file):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn create_vouch_succeeds_when_signer_differs_from_target() {
    use holochain::sweettest::{SweetConductor, SweetDnaFile};
    let dna_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("workdir/elohim.dna");
    let dna = SweetDnaFile::from_bundle(&dna_path).await.unwrap();
    let mut conductor_alice = SweetConductor::from_standard_config().await;
    let mut conductor_bob = SweetConductor::from_standard_config().await;
    let alice_apps = conductor_alice.setup_app("elohim", &[dna.clone()]).await.unwrap();
    let bob_apps = conductor_bob.setup_app("elohim", &[dna]).await.unwrap();
    let alice_cell = &alice_apps.cells()[0];
    let bob_cell = &bob_apps.cells()[0];

    // Bob creates an original FeedbackSignal (correction).
    let bob_correction_input = CreateFeedbackSignalInput {
        target_cid: "bafyreitarget1".into(),
        signal_kind: "correction".into(),
        evidence_cid: Some("bafyreievidence".into()),
        standing_impact: "debit-soft".into(),
        signature: vec![0xFF; 64],
    };
    let bob_correction_hash: ActionHash = conductor_bob
        .call(&bob_cell.zome("content_store"), "create_feedback_signal", bob_correction_input)
        .await;

    // Alice vouches on Bob's correction. Should succeed.
    let alice_vouch_input = CreateVouchInput {
        target_signal_cid: bob_correction_hash.to_string(),
        vouch_kind: "accept-correction".into(),
        standing_impact: "debit-soft".into(),
        signature: vec![0xAA; 64],
    };
    let result: ExternResult<ActionHash> = conductor_alice
        .call_fallible(&alice_cell.zome("content_store"), "create_vouch", alice_vouch_input)
        .await;
    assert!(result.is_ok(), "alice vouching on bob's correction should succeed: {:?}", result);
}

#[tokio::test(flavor = "multi_thread")]
async fn create_vouch_rejects_self_vouch() {
    // ... same setup as above, but bob calls create_vouch on bob's own correction
    let bob_self_vouch = CreateVouchInput {
        target_signal_cid: bob_correction_hash.to_string(),
        vouch_kind: "accept-correction".into(),
        standing_impact: "debit-soft".into(),
        signature: vec![0xCC; 64],
    };
    let result: ExternResult<ActionHash> = conductor_bob
        .call_fallible(&bob_cell.zome("content_store"), "create_vouch", bob_self_vouch)
        .await;
    assert!(result.is_err(), "self-vouch must be rejected");
    let err = format!("{:?}", result.unwrap_err());
    assert!(err.contains("self-vouch") || err.contains("forbidden"),
        "error message must mention self-vouch: {}", err);
}
```

(Adapt `CreateFeedbackSignalInput` to match the existing struct in the coordinator. If the sweettest framework uses a different invocation pattern, follow neighbouring tests' conventions.)

- [x] **Step 3: Build sweettest** (Eclipse Che cannot run sweettests; build-only verification per `feedback_shift_measure_jenkins`)

```bash
cd elohim/holochain
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --tests --release
```
Expected: PASS (no compile errors).

- [x] **Step 4: Commit. Sweettest execution validation will happen on Jenkins after push.**

```bash
git add elohim/holochain/dna/elohim/tests/  # adapt path
git commit -m "test(epr-light-up): T07 — sweettest scaffolding for create_vouch (alice-bob, no-self-vouch)"
```

---

## Task 8: ManifestRegistry accessors for new fields

**Files:**
- Modify: `elohim/elohim-storage/src/services/manifest_registry.rs`

- [x] **Step 1: Read the current ManifestRegistry**

```bash
cat elohim/elohim-storage/src/services/manifest_registry.rs
```

- [x] **Step 2: Write a failing test for `debit_weights()`**

Append to the same file's `#[cfg(test)] mod tests`:

```rust
#[test]
fn debit_weights_extracts_vouch_block_from_manifest() {
    let json = r#"{
        "manifestKind": "standing-policy",
        "revision": 1,
        "floor": { "classes": [] },
        "newVoiceBaseline": { "score": "floor", "vulnerableClassLift": "low" },
        "debitWeights": {
            "squelch":    { "advisory": 0, "debit-soft": 1, "debit-firm": 3 },
            "correction": { "advisory": 0, "debit-soft": 10, "debit-firm": 20 },
            "retraction": { "advisory": 0, "debit-soft": -5, "debit-firm": -10 },
            "quarantine": { "advisory": 0, "debit-soft": 12, "debit-firm": 30 },
            "vouch":      { "advisory": 0, "debit-soft": -3, "debit-firm": -8 }
        }
    }"#;
    let registry = ManifestRegistry::from_payload_json(json).expect("parse");
    let weights = registry.debit_weights().expect("present");
    assert_eq!(weights.get(&("vouch".into(), "debit-soft".into())), Some(&-3));
    assert_eq!(weights.get(&("correction".into(), "debit-firm".into())), Some(&20));
}

#[test]
fn debit_weights_returns_none_for_empty_registry() {
    let registry = ManifestRegistry::default();
    assert!(registry.debit_weights().is_none());
}

#[test]
fn reach_threshold_returns_correct_value() {
    let json = r#"{ "manifestKind": "standing-policy", "revision": 1,
        "floor": { "classes": [] },
        "newVoiceBaseline": { "score": "floor", "vulnerableClassLift": "low" },
        "debitWeights": {
            "squelch": {"advisory":0,"debit-soft":1,"debit-firm":3},
            "correction": {"advisory":0,"debit-soft":10,"debit-firm":20},
            "retraction": {"advisory":0,"debit-soft":-5,"debit-firm":-10},
            "quarantine": {"advisory":0,"debit-soft":12,"debit-firm":30},
            "vouch": {"advisory":0,"debit-soft":-3,"debit-firm":-8}
        },
        "reachThresholds": { "public": "high", "household": "any" }
    }"#;
    let r = ManifestRegistry::from_payload_json(json).expect("parse");
    assert_eq!(r.reach_threshold("public"), Some("high".to_string()));
    assert_eq!(r.reach_threshold("household"), Some("any".to_string()));
    assert_eq!(r.reach_threshold("not-in-map"), None);
}

#[test]
fn unknown_treatment_defaults_when_missing() {
    let r = ManifestRegistry::default();
    assert_eq!(r.unknown_treatment(), UnknownTreatment::Conservative);
}
```

- [x] **Step 3: Run tests — expect FAIL (methods not defined)**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::manifest_registry::tests
```
Expected: FAIL — `debit_weights`, `reach_threshold`, `unknown_treatment`, `from_payload_json`, `UnknownTreatment` don't exist.

- [x] **Step 4: Implement the new accessors**

Add to `services/manifest_registry.rs`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownTreatment {
    Conservative,
    NewVoiceBaseline,
    Neutral,
}

impl Default for UnknownTreatment {
    fn default() -> Self {
        UnknownTreatment::Conservative
    }
}

impl ManifestRegistry {
    /// Parse a JSON payload string (as stored in the manifests table) into a registry instance.
    /// Used in tests; production loads via `load_from_db`.
    pub fn from_payload_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let mut r = ManifestRegistry::default();
        r.standing_policy_payload = Some(value);
        Ok(r)
    }

    /// Returns flat (signal_kind, impact) → weight map from the standing-policy manifest.
    /// Returns None if no standing-policy manifest is registered.
    pub fn debit_weights(&self) -> Option<HashMap<(String, String), i32>> {
        let payload = self.standing_policy_payload.as_ref()?;
        let dw = payload.get("debitWeights")?.as_object()?;
        let mut out = HashMap::new();
        for (kind, impacts) in dw {
            for (impact, weight) in impacts.as_object()? {
                if let Some(w) = weight.as_i64() {
                    out.insert((kind.clone(), impact.clone()), w as i32);
                }
            }
        }
        Some(out)
    }

    pub fn reach_threshold(&self, reach: &str) -> Option<String> {
        let payload = self.standing_policy_payload.as_ref()?;
        let thresholds = payload.get("reachThresholds")?.as_object()?;
        thresholds.get(reach)?.as_str().map(|s| s.to_string())
    }

    pub fn unknown_treatment(&self) -> UnknownTreatment {
        let Some(payload) = self.standing_policy_payload.as_ref() else {
            return UnknownTreatment::Conservative;
        };
        match payload
            .get("unknownTreatment")
            .and_then(|v| v.get("default"))
            .and_then(|v| v.as_str())
        {
            Some("newVoiceBaseline") => UnknownTreatment::NewVoiceBaseline,
            Some("neutral") => UnknownTreatment::Neutral,
            _ => UnknownTreatment::Conservative,
        }
    }

    /// Returns true if the agent appears in any quarantine list registered in the manifests.
    /// Phase 3.5 stub: always returns false; full quarantine list in a future sprint.
    pub fn is_quarantined(&self, _agent: &[u8]) -> bool {
        false
    }

    /// Returns the manifest-declared baseline lift for vulnerable-class agents.
    /// Phase 3.5 stub: always returns None; classification fetch in a future sprint.
    pub fn vulnerable_class_lift(&self, _agent: &[u8]) -> Option<crate::services::standing::StandingScore> {
        None
    }

    /// Returns the bootstrap manifest's newVoiceBaseline.score, or None if not set.
    pub fn new_voice_baseline(&self) -> Option<crate::services::standing::StandingScore> {
        let payload = self.standing_policy_payload.as_ref()?;
        let score_str = payload.get("newVoiceBaseline")?.get("score")?.as_str()?;
        match score_str {
            "floor" => Some(crate::services::standing::StandingScore::Floor),
            "low" => Some(crate::services::standing::StandingScore::Low),
            "neutral" => Some(crate::services::standing::StandingScore::Neutral),
            "high" => Some(crate::services::standing::StandingScore::High),
            "trusted" => Some(crate::services::standing::StandingScore::Trusted),
            _ => None,
        }
    }
}
```

If `ManifestRegistry` doesn't have a `standing_policy_payload: Option<serde_json::Value>` field yet, add it. Update `load_from_db` to populate it when a `manifestKind == "standing-policy"` row is found.

- [x] **Step 5: Run tests — expect PASS**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::manifest_registry::tests
```
Expected: PASS.

- [x] **Step 6: Run lints**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
```
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/manifest_registry.rs
git commit -m "feat(epr-light-up): T08 — add ManifestRegistry accessors (debit_weights, reach_threshold, unknown_treatment)"
```

---

## Task 9: ManifestDebitWeightPolicy implementation

**Files:**
- Modify: `elohim/elohim-storage/src/services/standing_projector.rs`

- [x] **Step 1: Read the current standing_projector**

```bash
grep -n "DebitWeightPolicy\|DefaultDebitWeightPolicy" elohim/elohim-storage/src/services/standing_projector.rs
```

- [x] **Step 2: Add a failing test**

In `services/standing_projector.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn manifest_policy_returns_manifest_weights_when_registered() {
    let json = r#"{
        "manifestKind": "standing-policy", "revision": 1,
        "floor": {"classes":[]},
        "newVoiceBaseline": {"score":"floor","vulnerableClassLift":"low"},
        "debitWeights": {
            "squelch": {"advisory":0,"debit-soft":1,"debit-firm":3},
            "correction": {"advisory":0,"debit-soft":10,"debit-firm":20},
            "retraction": {"advisory":0,"debit-soft":-5,"debit-firm":-10},
            "quarantine": {"advisory":0,"debit-soft":12,"debit-firm":30},
            "vouch": {"advisory":0,"debit-soft":-3,"debit-firm":-8}
        }
    }"#;
    let registry = crate::services::manifest_registry::ManifestRegistry::from_payload_json(json)
        .expect("parse");
    let policy = ManifestDebitWeightPolicy::from_registry(&registry);
    use crate::p2p::feedback_signal::{SignalKind, StandingImpact};
    assert_eq!(policy.debit_weight(SignalKind::Vouch, StandingImpact::DebitSoft), -3);
    assert_eq!(policy.debit_weight(SignalKind::Correction, StandingImpact::DebitFirm), 20);
}

#[test]
fn manifest_policy_falls_back_to_default_when_empty() {
    let registry = crate::services::manifest_registry::ManifestRegistry::default();
    let policy = ManifestDebitWeightPolicy::from_registry(&registry);
    use crate::p2p::feedback_signal::{SignalKind, StandingImpact};
    // DefaultDebitWeightPolicy returns 1 for squelch/debit-soft.
    assert_eq!(policy.debit_weight(SignalKind::Squelch, StandingImpact::DebitSoft), 1);
}
```

- [x] **Step 3: Run — expect FAIL** (`ManifestDebitWeightPolicy` undefined)

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::standing_projector::tests::manifest_policy
```
Expected: FAIL.

- [x] **Step 4: Implement `ManifestDebitWeightPolicy`**

Add to `services/standing_projector.rs`:

```rust
use std::collections::HashMap;
use crate::p2p::feedback_signal::{SignalKind, StandingImpact};
use crate::services::manifest_registry::ManifestRegistry;

/// Manifest-driven debit-weight lookup. Falls back to [`DefaultDebitWeightPolicy`]
/// when a key is absent from the registered standing-policy manifest, or when no
/// such manifest is registered at all.
pub struct ManifestDebitWeightPolicy {
    weights: HashMap<(SignalKind, StandingImpact), i32>,
    fallback: DefaultDebitWeightPolicy,
}

impl ManifestDebitWeightPolicy {
    pub fn from_registry(registry: &ManifestRegistry) -> Self {
        let mut weights = HashMap::new();
        if let Some(map) = registry.debit_weights() {
            for ((kind_str, impact_str), w) in map {
                if let (Some(kind), Some(impact)) = (
                    parse_signal_kind(&kind_str),
                    parse_standing_impact(&impact_str),
                ) {
                    weights.insert((kind, impact), w);
                }
            }
        }
        Self { weights, fallback: DefaultDebitWeightPolicy }
    }
}

fn parse_signal_kind(s: &str) -> Option<SignalKind> {
    match s {
        "squelch" => Some(SignalKind::Squelch),
        "correction" => Some(SignalKind::Correction),
        "retraction" => Some(SignalKind::Retraction),
        "quarantine" => Some(SignalKind::Quarantine),
        "vouch" => Some(SignalKind::Vouch),
        _ => None,
    }
}

fn parse_standing_impact(s: &str) -> Option<StandingImpact> {
    match s {
        "advisory" => Some(StandingImpact::Advisory),
        "debit-soft" => Some(StandingImpact::DebitSoft),
        "debit-firm" => Some(StandingImpact::DebitFirm),
        _ => None,
    }
}

impl DebitWeightPolicy for ManifestDebitWeightPolicy {
    fn debit_weight(&self, kind: SignalKind, impact: StandingImpact) -> i32 {
        self.weights
            .get(&(kind, impact))
            .copied()
            .unwrap_or_else(|| self.fallback.debit_weight(kind, impact))
    }
}
```

(Adjust `DefaultDebitWeightPolicy` and the `DebitWeightPolicy` trait imports based on the actual module shape.)

- [x] **Step 5: Run tests — expect PASS**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::standing_projector
```
Expected: PASS.

- [x] **Step 6: Lint**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
```
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/standing_projector.rs
git commit -m "feat(epr-light-up): T09 — ManifestDebitWeightPolicy with registry-then-default fallback"
```

---

## Task 10: Standing::with_lift helper

**Files:**
- Modify: `elohim/elohim-storage/src/services/standing.rs`

- [x] **Step 1: Add a failing test**

In `services/standing.rs` `mod tests`:

```rust
#[test]
fn with_lift_promotes_unknown_to_lifted_score() {
    let lifted = Standing::Unknown.with_lift(Some(StandingScore::Low));
    assert_eq!(lifted, Standing::Computed { score: StandingScore::Low });
}

#[test]
fn with_lift_takes_max_of_existing_and_lift() {
    let s = Standing::Computed { score: StandingScore::Floor };
    let lifted = s.with_lift(Some(StandingScore::Low));
    assert_eq!(lifted, Standing::Computed { score: StandingScore::Low });
}

#[test]
fn with_lift_does_not_demote() {
    let s = Standing::Computed { score: StandingScore::High };
    let lifted = s.with_lift(Some(StandingScore::Low));
    assert_eq!(lifted, Standing::Computed { score: StandingScore::High });
}

#[test]
fn with_lift_none_is_identity() {
    let s = Standing::Computed { score: StandingScore::Neutral };
    assert_eq!(s.with_lift(None), s);
}
```

- [x] **Step 2: Run — expect FAIL**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::standing::tests::with_lift
```
Expected: FAIL — `with_lift` not defined.

- [x] **Step 3: Implement `with_lift` and `Ord` over StandingScore**

Add to `services/standing.rs`:

```rust
impl StandingScore {
    fn rank(self) -> u8 {
        match self {
            StandingScore::Floor => 0,
            StandingScore::Low => 1,
            StandingScore::Neutral => 2,
            StandingScore::High => 3,
            StandingScore::Trusted => 4,
        }
    }
}

impl Standing {
    /// Apply a vulnerable-class baseline lift. Unknown becomes Computed(lift).
    /// Computed takes max(self, lift). Returns self unchanged when lift is None.
    pub fn with_lift(self, lift: Option<StandingScore>) -> Self {
        match (self, lift) {
            (s, None) => s,
            (Standing::Unknown, Some(l)) => Standing::Computed { score: l },
            (Standing::Computed { score }, Some(l)) => Standing::Computed {
                score: if score.rank() >= l.rank() { score } else { l },
            },
        }
    }
}
```

- [x] **Step 4: Run — expect PASS**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::standing
```
Expected: PASS.

- [x] **Step 5: Lint + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
git add elohim/elohim-storage/src/services/standing.rs
git commit -m "feat(epr-light-up): T10 — Standing::with_lift baseline-lift helper"
```

---

## Task 11: Reach::is_floor_allowed helper + Reach type if missing

**Files:**
- Locate or create: where `Reach` is defined (likely `services/epr_kind.rs` or `p2p/feedback_signal.rs`)

- [x] **Step 1: Find Reach**

```bash
grep -rn "pub enum Reach\b" elohim/elohim-storage/src/ | head -10
```

- [x] **Step 2: If `Reach` exists, add `is_floor_allowed`. If it does NOT exist, declare it.**

If `Reach` exists, add a method. Else, create the type at `services/epr_kind.rs` (or wherever EPR types live):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reach {
    Personal,
    Intimate,
    Household,
    Neighborhood,
    Collective,
    Community,
    District,
    Public,
}

impl Reach {
    /// Returns true when the manifest's reachThresholds map this reach to "any" — i.e.
    /// these reach values bypass standing/floor checks (cid-targeted-lookup &
    /// local-relationship-reach floor classes).
    pub fn is_floor_allowed(self) -> bool {
        matches!(
            self,
            Reach::Personal | Reach::Intimate | Reach::Household | Reach::Neighborhood
        )
    }

    pub fn as_kebab(self) -> &'static str {
        match self {
            Reach::Personal => "personal",
            Reach::Intimate => "intimate",
            Reach::Household => "household",
            Reach::Neighborhood => "neighborhood",
            Reach::Collective => "collective",
            Reach::Community => "community",
            Reach::District => "district",
            Reach::Public => "public",
        }
    }
}
```

- [x] **Step 3: Add unit tests**

```rust
#[cfg(test)]
mod reach_tests {
    use super::*;
    #[test] fn floor_reaches_bypass() {
        assert!(Reach::Personal.is_floor_allowed());
        assert!(Reach::Household.is_floor_allowed());
    }
    #[test] fn non_floor_reaches_do_not_bypass() {
        assert!(!Reach::Public.is_floor_allowed());
        assert!(!Reach::Community.is_floor_allowed());
    }
}
```

- [x] **Step 4: Run, lint, commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib reach_tests
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
git add elohim/elohim-storage/src/services/epr_kind.rs   # or actual path
git commit -m "feat(epr-light-up): T11 — Reach::is_floor_allowed helper"
```

---

## Task 12: services/reach_earning.rs — ReachVerdict + evaluate

**Files:**
- Create: `elohim/elohim-storage/src/services/reach_earning.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod reach_earning;`)

- [x] **Step 1: Create the file with type definitions and a failing test scaffold**

Write `elohim/elohim-storage/src/services/reach_earning.rs`:

```rust
//! Reach-earning gate — Phase 3.5 author-side compose substrate.
//!
//! Pure deterministic evaluator. Returns ReachVerdict; never persists. The
//! verdict shape is forward-compatible with a future elohim-mediated discernment
//! layer that consumes Pending and produces sponsor suggestions.
//!
//! See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Components::ReachVerdict

use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::services::manifest_registry::{ManifestRegistry, UnknownTreatment};
use crate::services::standing::{Standing, StandingScore};

/// One of the five constitutional floor classes (brainstorm §2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloorClass {
    CidTargetedLookup,
    NewVoiceBaseline,
    VulnerableClassElevation,
    LocalRelationshipReach,
    ConstitutionalFloorSignatures,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingEvidence {
    pub standing: Standing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    QuarantineActive,
    FloorBreach { class: FloorClass },
    StandingBelowThreshold,
    UnknownReach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingReason {
    UnknownAuthorAtNonFloorReach,
    NewVoiceWithoutSponsor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReachVerdict {
    Allowed { floor_class_match: Option<FloorClass>, evidence: StandingEvidence },
    Blocked { reason: BlockReason, evidence: StandingEvidence },
    Pending { reason: PendingReason, evidence: StandingEvidence },
}

/// Pure substrate evaluator. Does not persist; returns ephemeral verdict.
pub fn evaluate(
    local_agent: &[u8],
    author: &[u8],
    requested_reach: crate::services::epr_kind::Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> ReachVerdict {
    use crate::services::epr_kind::Reach;

    // 1. Floor class allow: cid-targeted-lookup, local-relationship-reach
    if requested_reach.is_floor_allowed() {
        return ReachVerdict::Allowed {
            floor_class_match: Some(match requested_reach {
                Reach::Personal | Reach::Intimate => FloorClass::CidTargetedLookup,
                Reach::Household | Reach::Neighborhood => FloorClass::LocalRelationshipReach,
                _ => FloorClass::CidTargetedLookup,
            }),
            evidence: StandingEvidence { standing: Standing::Unknown },
        };
    }

    // 2. Quarantine check
    if registry.is_quarantined(author) {
        return ReachVerdict::Blocked {
            reason: BlockReason::QuarantineActive,
            evidence: StandingEvidence { standing: Standing::Unknown },
        };
    }

    // 3. Vulnerable-class lift
    let lift = registry.vulnerable_class_lift(author);

    // 4. Standing evaluation
    let raw_standing = Standing::evaluate(local_agent, author, conn);
    let effective = raw_standing.with_lift(lift);
    let evidence = StandingEvidence { standing: raw_standing };

    // 5. Required threshold from manifest (with hard-coded fallback when missing)
    let required = match registry.reach_threshold(requested_reach.as_kebab()) {
        Some(t) => t,
        None => {
            // Manifest missing — use safe-by-default conservative table.
            match requested_reach {
                Reach::Public => "high".to_string(),
                _ => "neutral".to_string(),
            }
        }
    };

    // 6. Apply UnknownTreatment policy
    match (effective, required.as_str()) {
        (Standing::Unknown, _) => match registry.unknown_treatment() {
            UnknownTreatment::Conservative => ReachVerdict::Pending {
                reason: PendingReason::UnknownAuthorAtNonFloorReach,
                evidence,
            },
            UnknownTreatment::NewVoiceBaseline => {
                let baseline = registry.new_voice_baseline().unwrap_or(StandingScore::Floor);
                evaluate_with_score(baseline, &required, evidence)
            }
            UnknownTreatment::Neutral => evaluate_with_score(StandingScore::Neutral, &required, evidence),
        },
        (Standing::Computed { score }, threshold) => evaluate_with_score(score, threshold, evidence),
    }
}

fn evaluate_with_score(score: StandingScore, threshold: &str, evidence: StandingEvidence) -> ReachVerdict {
    if threshold == "any" {
        return ReachVerdict::Allowed { floor_class_match: None, evidence };
    }
    let needed = match threshold {
        "floor" => StandingScore::Floor,
        "low" => StandingScore::Low,
        "neutral" => StandingScore::Neutral,
        "high" => StandingScore::High,
        "trusted" => StandingScore::Trusted,
        _ => return ReachVerdict::Blocked { reason: BlockReason::UnknownReach, evidence },
    };
    if score_rank(score) >= score_rank(needed) {
        ReachVerdict::Allowed { floor_class_match: None, evidence }
    } else {
        ReachVerdict::Blocked { reason: BlockReason::StandingBelowThreshold, evidence }
    }
}

fn score_rank(s: StandingScore) -> u8 {
    match s {
        StandingScore::Floor => 0,
        StandingScore::Low => 1,
        StandingScore::Neutral => 2,
        StandingScore::High => 3,
        StandingScore::Trusted => 4,
    }
}
```

- [x] **Step 2: Add comprehensive unit tests**

Append to the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::epr_kind::Reach;
    use crate::services::manifest_registry::ManifestRegistry;

    fn test_pool() -> crate::db::DbPool {
        use crate::db::run_migrations;
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::sqlite::SqliteConnection;
        let url = format!("file:reach_earn_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple());
        let pool = Pool::builder().max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url)).expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn registry_with_full_policy() -> ManifestRegistry {
        let json = r#"{
            "manifestKind":"standing-policy","revision":1,
            "floor":{"classes":[]},
            "newVoiceBaseline":{"score":"floor","vulnerableClassLift":"low"},
            "debitWeights":{
                "squelch":{"advisory":0,"debit-soft":1,"debit-firm":3},
                "correction":{"advisory":0,"debit-soft":10,"debit-firm":20},
                "retraction":{"advisory":0,"debit-soft":-5,"debit-firm":-10},
                "quarantine":{"advisory":0,"debit-soft":12,"debit-firm":30},
                "vouch":{"advisory":0,"debit-soft":-3,"debit-firm":-8}
            },
            "unknownTreatment":{"default":"conservative","evidenceSources":[]},
            "reachThresholds":{
                "personal":"any","intimate":"any","household":"any","neighborhood":"any",
                "collective":"neutral","community":"neutral","district":"neutral","public":"high"
            }
        }"#;
        ManifestRegistry::from_payload_json(json).expect("parse")
    }

    #[test]
    fn floor_reach_always_allowed_unknown_author() {
        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Personal, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Allowed { .. }));
    }

    #[test]
    fn unknown_author_at_public_with_conservative_treatment_pending() {
        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Pending { reason: PendingReason::UnknownAuthorAtNonFloorReach, .. }), "{:?}", v);
    }

    #[test]
    fn unknown_author_at_public_with_neutral_treatment_blocked_below_high() {
        let json = r#"{"manifestKind":"standing-policy","revision":1,
            "floor":{"classes":[]},
            "newVoiceBaseline":{"score":"floor","vulnerableClassLift":"low"},
            "debitWeights":{"squelch":{"advisory":0,"debit-soft":1,"debit-firm":3},"correction":{"advisory":0,"debit-soft":10,"debit-firm":20},"retraction":{"advisory":0,"debit-soft":-5,"debit-firm":-10},"quarantine":{"advisory":0,"debit-soft":12,"debit-firm":30},"vouch":{"advisory":0,"debit-soft":-3,"debit-firm":-8}},
            "unknownTreatment":{"default":"neutral","evidenceSources":[]},
            "reachThresholds":{"public":"high"}
        }"#;
        let r = ManifestRegistry::from_payload_json(json).unwrap();
        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Blocked { reason: BlockReason::StandingBelowThreshold, .. }), "{:?}", v);
    }

    #[test]
    fn computed_high_at_public_allowed() {
        // Project a positive history first, then evaluate.
        use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
        use crate::services::standing_projector::{project_signal, ManifestDebitWeightPolicy};
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let policy = ManifestDebitWeightPolicy::from_registry(&r);
        let evaluator = [0u8; 32];
        let author_bytes = [0xAB; 32];
        let author_b64 = BASE64.encode(author_bytes);

        // -3 -3 = -6 sum → score per existing thresholds (likely Trusted or High depending on cumulative)
        for _ in 0..2 {
            let sig = FeedbackSignal {
                target_cid: format!("bafyreitarget{}", uuid::Uuid::new_v4()),
                signal_kind: SignalKind::Vouch,
                vouch_kind: Some(crate::p2p::feedback_signal::VouchKind::AcceptCorrection),
                evidence_cid: None,
                standing_impact: StandingImpact::DebitFirm, // -8 each
                signed_by: author_b64.clone(),
                signature: BASE64.encode([0xFF; 64]),
            };
            project_signal(&mut conn, &policy, &evaluator, &sig, "bafyreimanifest").unwrap();
        }

        let v = evaluate(&evaluator, &author_bytes, Reach::Public, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Allowed { .. }), "{:?}", v);
    }

    #[test]
    fn manifest_absent_falls_back_to_conservative_table() {
        let r = ManifestRegistry::default();
        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        // Unknown standing + Conservative treatment → Pending
        assert!(matches!(v, ReachVerdict::Pending { .. }), "{:?}", v);
    }

    #[test]
    fn floor_reach_household_returns_local_relationship_reach_class() {
        let pool = test_pool(); let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Household, &mut conn, &r);
        if let ReachVerdict::Allowed { floor_class_match: Some(class), .. } = v {
            assert_eq!(class, FloorClass::LocalRelationshipReach);
        } else { panic!("expected Allowed with LocalRelationshipReach, got {:?}", v); }
    }
}
```

- [x] **Step 3: Register the module**

Edit `elohim/elohim-storage/src/services/mod.rs` and add:

```rust
pub mod reach_earning;
```

(Position alphabetically with neighbouring `pub mod` declarations.)

- [x] **Step 4: Run all reach_earning tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::reach_earning
```
Expected: PASS (all six tests).

- [x] **Step 5: Lint**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
```
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/reach_earning.rs \
        elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(epr-light-up): T12 — services/reach_earning.rs ReachVerdict + evaluate()"
```

---

## Task 13: services/epr_compose.rs — compose_epr helper

**Files:**
- Create: `elohim/elohim-storage/src/services/epr_compose.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod epr_compose;`)

- [x] **Step 1: Create the file**

```rust
//! Author-side EPR compose helper.
//!
//! Wraps a compose attempt with the reach-earning gate. Receive-path (external
//! EPRs arriving at put_epr) does NOT consult this — the gate is for outgoing
//! reach decisions only.

use diesel::SqliteConnection;

use crate::services::manifest_registry::ManifestRegistry;
use crate::services::reach_earning::{evaluate, ReachVerdict};
use crate::services::epr_kind::Reach;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ComposeError {
    #[error("reach denied: {0:?}")]
    ReachDenied(ReachVerdict),
    #[error("invalid author key: {0}")]
    InvalidAuthor(String),
}

#[derive(Debug, Clone)]
pub struct ComposedEpr {
    pub verdict: ReachVerdict,
}

/// Author-side compose check. Returns Ok with verdict on Allowed; Err with
/// the verdict on Blocked or Pending.
pub fn compose_epr(
    local_agent: &[u8],
    author: &[u8],
    requested_reach: Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> Result<ComposedEpr, ComposeError> {
    let verdict = evaluate(local_agent, author, requested_reach, conn, registry);
    match verdict {
        ReachVerdict::Allowed { .. } => Ok(ComposedEpr { verdict }),
        ReachVerdict::Pending { .. } | ReachVerdict::Blocked { .. } => {
            Err(ComposeError::ReachDenied(verdict))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pool() -> crate::db::DbPool {
        use crate::db::run_migrations;
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::sqlite::SqliteConnection;
        let url = format!("file:epr_compose_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple());
        let pool = Pool::builder().max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url)).expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    #[test]
    fn allowed_verdict_returns_ok() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = ManifestRegistry::default();
        // Reach::Personal is_floor_allowed → Allowed regardless
        let result = compose_epr(&[0; 32], &[1; 32], Reach::Personal, &mut conn, &r);
        assert!(result.is_ok());
    }

    #[test]
    fn blocked_or_pending_returns_err() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = ManifestRegistry::default();
        // Unknown author at Public → Pending (Conservative default)
        let result = compose_epr(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(result.is_err());
        match result.unwrap_err() {
            ComposeError::ReachDenied(ReachVerdict::Pending { .. }) => {}
            other => panic!("expected ReachDenied(Pending), got {:?}", other),
        }
    }
}
```

- [x] **Step 2: Register the module + run + lint + commit**

```bash
# Edit services/mod.rs: add `pub mod epr_compose;`
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::epr_compose
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
git add elohim/elohim-storage/src/services/epr_compose.rs \
        elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(epr-light-up): T13 — services/epr_compose.rs author-side gate wrapper"
```

---

## Task 14: P2PCommand variants for direct send + gossip publish

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [x] **Step 1: Read existing P2PCommand**

```bash
sed -n '500,560p' elohim/elohim-storage/src/p2p/mod.rs
```

- [x] **Step 2: Add variants if missing**

Add to the `P2PCommand` enum in `p2p/mod.rs` (preserve existing variants):

```rust
pub enum P2PCommand {
    // ...existing variants...

    /// Phase 3.5 — Light Up the Graph: send raw bytes directly to a peer.
    /// Used by LibP2POutboundSink (back-prop) for one-hop predecessor walks.
    SendDirect {
        peer: libp2p::PeerId,
        payload: Vec<u8>,
    },

    /// Phase 3.5 — Light Up the Graph: publish raw bytes on a gossipsub topic.
    /// Used by LibP2PGossipPublisher (gossip-flood) for content-reach broadcasts.
    GossipPublish {
        topic: String,
        payload: Vec<u8>,
    },
}
```

- [x] **Step 3: Wire the swarm event loop to handle them**

Find the swarm event loop function (likely in the same file or `p2p/swarm.rs`) and add match arms in the `P2PCommand` dispatcher:

```rust
P2PCommand::SendDirect { peer, payload } => {
    // Route via existing direct-notify protocol; if no specific protocol exists,
    // use gossipsub direct-message via behaviour.notify or a request-response handler.
    // Adapt to whatever direct-notify mechanism is in place — often via a
    // request-response codec on a `/elohim/feedback-signal/1.0.0` protocol.
    if let Err(e) = behaviour.feedback_signal_protocol.send_request(&peer, payload) {
        tracing::warn!(?e, ?peer, "SendDirect failed");
    }
}
P2PCommand::GossipPublish { topic, payload } => {
    use libp2p::gossipsub::IdentTopic;
    let t = IdentTopic::new(&topic);
    if let Err(e) = behaviour.gossipsub.publish(t, payload) {
        tracing::warn!(?e, %topic, "GossipPublish failed");
    }
}
```

(If a `feedback_signal_protocol` request-response handler doesn't exist, register a new one in the existing behaviour composition. Check `p2p/behaviour.rs` for how other protocols are wired.)

- [x] **Step 4: Build to verify**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/p2p/behaviour.rs
git commit -m "feat(epr-light-up): T14 — add P2PCommand::SendDirect + GossipPublish variants"
```

---

## Task 15: p2p/adapters.rs — production swarm adapters

**Files:**
- Create: `elohim/elohim-storage/src/p2p/adapters.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (re-export)

- [x] **Step 1: Create the file**

```rust
//! Production swarm-backed adapters for back_prop and gossip_flood services.
//!
//! Bridges the existing trait abstractions (OutboundSink, GossipPublisher) to
//! the swarm task's P2PCommand mpsc channel — actor pattern, no swarm locking.

use std::str::FromStr;

use libp2p::PeerId;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::Sender;

use crate::p2p::P2PCommand;
use crate::services::back_prop::{OutboundSink, SinkError};
use crate::services::gossip_flood::{GossipPublisher, PublishError};

#[derive(Clone)]
pub struct LibP2POutboundSink {
    tx: Sender<P2PCommand>,
}

impl LibP2POutboundSink {
    pub fn new(tx: Sender<P2PCommand>) -> Self {
        Self { tx }
    }
}

impl OutboundSink for LibP2POutboundSink {
    fn send(&self, peer_id_str: &str, payload: Vec<u8>) -> Result<(), SinkError> {
        let peer = PeerId::from_str(peer_id_str)
            .map_err(|e| SinkError::Send(format!("invalid peer_id: {}", e)))?;
        self.tx.try_send(P2PCommand::SendDirect { peer, payload }).map_err(|e| match e {
            TrySendError::Full(_) => SinkError::Send("backpressure: command channel full".into()),
            TrySendError::Closed(_) => SinkError::Send("swarm gone: command channel closed".into()),
        })
    }
}

#[derive(Clone)]
pub struct LibP2PGossipPublisher {
    tx: Sender<P2PCommand>,
}

impl LibP2PGossipPublisher {
    pub fn new(tx: Sender<P2PCommand>) -> Self {
        Self { tx }
    }
}

impl GossipPublisher for LibP2PGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.tx
            .try_send(P2PCommand::GossipPublish { topic: topic.to_string(), payload })
            .map_err(|e| match e {
                TrySendError::Full(_) => PublishError::Send("backpressure: command channel full".into()),
                TrySendError::Closed(_) => PublishError::Send("swarm gone: command channel closed".into()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn sink_send_succeeds_with_open_channel() {
        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let sink = LibP2POutboundSink::new(tx);
        let valid_peer = "12D3KooWGRUAcEqzj5N1zMyxYBfZjPDKHbz9V3xS1HmKZRwjQR4q";
        sink.send(valid_peer, vec![1, 2, 3]).expect("send");
        let cmd = rx.recv().await.expect("cmd");
        assert!(matches!(cmd, P2PCommand::SendDirect { .. }));
    }

    #[tokio::test]
    async fn sink_send_returns_err_on_closed_channel() {
        let (tx, rx) = mpsc::channel::<P2PCommand>(8);
        drop(rx);
        let sink = LibP2POutboundSink::new(tx);
        let valid_peer = "12D3KooWGRUAcEqzj5N1zMyxYBfZjPDKHbz9V3xS1HmKZRwjQR4q";
        let err = sink.send(valid_peer, vec![1]).unwrap_err();
        assert!(format!("{:?}", err).contains("swarm gone"));
    }

    #[tokio::test]
    async fn sink_send_rejects_malformed_peer_id() {
        let (tx, _rx) = mpsc::channel::<P2PCommand>(8);
        let sink = LibP2POutboundSink::new(tx);
        let err = sink.send("not-a-peer-id", vec![1]).unwrap_err();
        assert!(format!("{:?}", err).contains("invalid peer_id"));
    }

    #[tokio::test]
    async fn publisher_publish_succeeds() {
        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let pub_ = LibP2PGossipPublisher::new(tx);
        pub_.publish("test/topic", vec![9, 9, 9]).expect("publish");
        let cmd = rx.recv().await.expect("cmd");
        match cmd {
            P2PCommand::GossipPublish { topic, payload } => {
                assert_eq!(topic, "test/topic");
                assert_eq!(payload, vec![9, 9, 9]);
            }
            _ => panic!("wrong variant"),
        }
    }
}
```

- [x] **Step 2: Register module**

Edit `elohim/elohim-storage/src/p2p/mod.rs` and add:

```rust
pub mod adapters;
```

If `gossip_flood::PublishError` does not exist as a public type (the existing trait might use a different error name), grep `services/gossip_flood.rs` and adapt.

- [x] **Step 3: Run**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib p2p::adapters
```
Expected: PASS (4 tests).

- [x] **Step 4: Lint + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
git add elohim/elohim-storage/src/p2p/adapters.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(epr-light-up): T15 — p2p/adapters.rs LibP2POutboundSink + LibP2PGossipPublisher"
```

---

## Task 16: services/tending.rs — sweep_expired implementation

**Files:**
- Modify: `elohim/elohim-storage/src/services/tending.rs`

- [x] **Step 1: Read current state**

```bash
cat elohim/elohim-storage/src/services/tending.rs
```

- [x] **Step 2: Write a failing test**

Append to `services/tending.rs`'s `mod tests` (or create one):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    fn test_pool() -> crate::db::DbPool {
        let url = format!("file:tending_sweep_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple());
        let pool = Pool::builder().max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url)).expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    #[test]
    fn sweep_deletes_expired_non_safety_rows() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Insert: expired fatigue (should delete), live values-forward (keep), safety (keep regardless)
        let now = now_ms();
        crate::db::attention_tending::insert_raw(&mut conn,
            "filter1", "fatigue", 3600, &[now - 3600_001 * 2], "{}").unwrap();
        crate::db::attention_tending::insert_raw(&mut conn,
            "filter2", "values-forward", 86400, &[now - 100], "{}").unwrap();
        crate::db::attention_tending::insert_raw(&mut conn,
            "filter3", "safety", 60, &[now - 1_000_000], "{}").unwrap();
        let deleted = sweep_expired(&mut conn).expect("sweep");
        assert_eq!(deleted, 1, "only the expired fatigue row should be deleted");

        let remaining = crate::db::attention_tending::list_all(&mut conn).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|r| r.classification == "safety"));
        assert!(remaining.iter().any(|r| r.classification == "values-forward"));
    }

    #[test]
    fn sweep_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = now_ms();
        crate::db::attention_tending::insert_raw(&mut conn,
            "filter1", "fatigue", 3600, &[now - 3600_001 * 2], "{}").unwrap();
        sweep_expired(&mut conn).unwrap();
        let second = sweep_expired(&mut conn).expect("second sweep");
        assert_eq!(second, 0);
    }

    #[test]
    fn sweep_never_deletes_safety_classification() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = now_ms();
        // safety row with ttl=1 second, tended way in the past — would expire if sweep ignored class
        crate::db::attention_tending::insert_raw(&mut conn,
            "filter1", "safety", 1, &[now - 1_000_000_000], "{}").unwrap();
        sweep_expired(&mut conn).unwrap();
        let remaining = crate::db::attention_tending::list_all(&mut conn).unwrap();
        assert_eq!(remaining.len(), 1, "safety must not be deleted");
    }
}
```

(Adapt `crate::db::attention_tending::insert_raw` and `list_all` to whatever helpers the existing module exposes. If they don't exist, add minimal helpers in `db/attention_tending.rs`.)

- [x] **Step 3: Run — expect FAIL** (sweep_expired not implemented or stub)

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::tending::tests
```
Expected: FAIL.

- [x] **Step 4: Implement `sweep_expired`**

In `services/tending.rs`:

```rust
use diesel::prelude::*;
use diesel::SqliteConnection;
use crate::error::StorageError;

/// Delete expired non-safety rows from `attention_tending`. Idempotent:
/// running twice returns 0 rows on the second call. Safety classification
/// never deleted (constitutional floor — brainstorm §2.8).
///
/// Expiry is computed as `(tended_at_last_ms + ttl_seconds * 1000) < now_ms`
/// where `tended_at_last_ms` is the most recent entry in the `tended_at` array.
pub fn sweep_expired(conn: &mut SqliteConnection) -> Result<usize, StorageError> {
    use crate::schema::attention_tending::dsl::*;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // Diesel does NOT support array suffix in SQLite literally, so we rely on
    // a stored `tended_at_last_ms` column or a JSON expression. Adapt to
    // existing schema. If schema stores `tended_at` as JSON, use json_extract.
    let deleted = diesel::sql_query(
        "DELETE FROM attention_tending \
         WHERE classification != 'safety' \
         AND (json_extract(tended_at, '$[#-1]') + ttl_seconds * 1000) < ?",
    )
    .bind::<diesel::sql_types::BigInt, _>(now_ms)
    .execute(conn)
    .map_err(StorageError::from)?;

    tracing::debug!(deleted, "tending::sweep_expired");
    Ok(deleted)
}
```

If the actual schema column is `tended_at_last_ms` (denormalized) rather than a JSON array, simplify the WHERE clause accordingly. Inspect via `cat elohim/elohim-storage/src/schema.rs | grep attention_tending -A 20`.

- [x] **Step 5: Run — expect PASS**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::tending::tests
```
Expected: PASS (3 tests).

- [x] **Step 6: Lint + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --lib -- -D warnings
cargo fmt --check
git add elohim/elohim-storage/src/services/tending.rs elohim/elohim-storage/src/db/attention_tending.rs
git commit -m "feat(epr-light-up): T16 — tending::sweep_expired with safety floor protection"
```

---

## Task 17: api/epr.rs — local-origin dedup helper

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [x] **Step 1: Locate the put_epr handler**

```bash
grep -n "fn put_epr\|TODO(T19)" elohim/elohim-storage/src/api/epr.rs | head -10
```

- [x] **Step 2: Add a helper near the top of the file**

```rust
/// Returns true if the FeedbackSignal was sent by us (local node) — used to
/// skip back-prop / flood / project fan-out to avoid loops when our own
/// gossip-flood arrives back to us.
fn is_local_origin(envelope_signed_by: &str, local_peer_id: &str) -> bool {
    envelope_signed_by == local_peer_id
}

#[cfg(test)]
mod dedup_tests {
    use super::*;
    #[test] fn local_origin_match() {
        assert!(is_local_origin("12D3KooWXyz", "12D3KooWXyz"));
    }
    #[test] fn external_origin_no_match() {
        assert!(!is_local_origin("12D3KooWXyz", "12D3KooWQRS"));
    }
}
```

- [x] **Step 3: Run + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::epr::dedup_tests
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr-light-up): T17 — local-origin dedup helper for FeedbackSignal fan-out"
```

---

## Task 18: api/epr.rs — wire record_predecessor on EPR ingest

<!-- AUDIT 2026-05-11: GENUINE GAP. put_epr HTTP path explicitly documents record_predecessor as NOT wired (api/epr.rs:618). epr_atom_service.rs:189 also defers it. p2p/mod.rs swarm handler does not call it either. Wave-2 item W2A in the master plan. -->

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Find the `TODO(T19)` site for record_predecessor**

```bash
grep -n "TODO(T19): wire record_predecessor" elohim/elohim-storage/src/api/epr.rs
```
Site is at `elohim/elohim-storage/src/api/epr.rs:520`.

- [ ] **Step 2: Replace the TODO with the actual call**

Read the surrounding context first (5 lines before, 20 after) to see what variables are in scope:

```bash
sed -n '510,545p' elohim/elohim-storage/src/api/epr.rs
```

Then replace the `// TODO(T19): wire record_predecessor on EPR ingest path.` line with:

```rust
// Phase 3.5 — Light Up the Graph: record predecessor for back-prop walk.
if let (Some(sender_peer), Some(seal_keys)) = (sender_peer_id_opt.as_ref(), state.sealing_keys.as_ref()) {
    let pubs = crate::services::back_prop::SealingPubKeys {
        mishpat_pk: &seal_keys.mishpat_pk,
        imagodei_pk: &seal_keys.imagodei_pk,
    };
    if let Err(e) = crate::services::back_prop::record_predecessor(
        &mut conn,
        &cid_str,
        sender_peer.as_str(),
        pubs,
    ) {
        tracing::warn!(?e, %cid_str, "record_predecessor failed (non-fatal)");
    }
}
```

(Adapt `sender_peer_id_opt`, `state.sealing_keys`, `cid_str`, and `conn` to whatever names are actually in scope. Inspect the put_epr handler signature for the real names.)

- [ ] **Step 3: Build to verify compilation**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```
Expected: PASS.

- [ ] **Step 4: Run existing tests to check nothing broke**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::epr
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr-light-up): T18 — wire record_predecessor on EPR ingest in put_epr"
```

---

## Task 19: api/epr.rs — FeedbackSignal arrival fan-out

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [x] **Step 1: Find the FeedbackSignal arrival site**

```bash
grep -n "TODO(T19)\|FeedbackSignal" elohim/elohim-storage/src/api/epr.rs | head -20
```

- [x] **Step 2: Add the fan-out block at the FeedbackSignal arrival site**

The fan-out runs after persisting the FeedbackSignal envelope. It performs:
- Local-origin dedup
- Transactional: project_signal + record_predecessor (already wired above)
- Best-effort (non-transactional): back_prop_one_hop + flood_feedback

Insert near the top of the file under existing imports:

```rust
use crate::p2p::feedback_signal::FeedbackSignal;
use crate::services::standing_projector::ManifestDebitWeightPolicy;
```

In `put_epr` (or its FeedbackSignal-specific helper), after the envelope is persisted:

```rust
// Phase 3.5 — Light Up the Graph: FeedbackSignal arrival fan-out.
if epr_kind == "feedback-signal" {
    let envelope_signer = envelope.signed_by.as_str();
    if !is_local_origin(envelope_signer, state.local_peer_id.as_str()) {
        // Decode payload to FeedbackSignal — log and skip on decode errors.
        let signal_result: Result<FeedbackSignal, _> = rmp_serde::from_slice(&payload_bytes);
        match signal_result {
            Ok(signal) => {
                // 1. Project signal to standing_view (transactional with the existing persist write).
                let policy = ManifestDebitWeightPolicy::from_registry(&state.manifest_registry);
                let evaluator = state.local_pubkey.as_slice();
                let manifest_cid = state.standing_policy_cid.as_deref().unwrap_or("bootstrap");
                if let Err(e) = crate::services::standing_projector::project_signal(
                    &mut conn,
                    &policy,
                    evaluator,
                    &signal,
                    manifest_cid,
                ) {
                    tracing::warn!(?e, "project_signal failed (non-fatal)");
                }

                // 2. Back-prop one hop upstream — best-effort.
                if let Some(sink) = state.outbound_sink.as_ref() {
                    if let Err(e) = crate::services::back_prop::back_prop_one_hop(
                        &mut conn, &signal, sink.as_ref(),
                    ) {
                        tracing::warn!(?e, "back_prop_one_hop failed (non-fatal)");
                    }
                }

                // 3. Gossip-flood on the content-reach topic — best-effort.
                if let Some(publisher) = state.gossip_publisher.as_ref() {
                    let topic = crate::p2p::topics::reach_topic_for(&signal.target_cid);
                    let payload_for_flood = rmp_serde::to_vec(&signal).unwrap_or_default();
                    if let Err(e) = crate::services::gossip_flood::flood_feedback(
                        &signal, &topic, publisher.as_ref(),
                    ) {
                        tracing::warn!(?e, "flood_feedback failed (non-fatal)");
                    }
                }
            }
            Err(e) => tracing::warn!(?e, "FeedbackSignal payload decode failed"),
        }
    } else {
        tracing::debug!(%cid_str, "FeedbackSignal local-origin — skipping fan-out");
    }
}
```

(Adapt `state.outbound_sink`, `state.gossip_publisher`, `state.manifest_registry`, `state.local_pubkey`, `state.local_peer_id`, `state.standing_policy_cid` to whatever the actual shared state struct holds. Add fields if needed — see Task 22.)

- [x] **Step 3: Verify compilation**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```
Expected: PASS (after ensuring shared state type has the new fields).

- [x] **Step 4: Run existing api::epr tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::epr
```
Expected: PASS (existing tests still green).

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr-light-up): T19 — FeedbackSignal arrival fan-out (project + back_prop + flood)"
```

---

## Task 20: main.rs — bootstrap_manifests::seed_if_empty wiring

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`

- [x] **Step 1: Find startup sequence**

```bash
grep -n "run_migrations\|TODO(T19)\|seed_if_empty\|bootstrap_manifests" elohim/elohim-storage/src/main.rs | head -10
```

- [x] **Step 2: Add the call after migrations, before HTTP/swarm spawn**

```rust
// Phase 3.5 — Light Up the Graph: seed bootstrap manifests after migrations.
{
    let mut seed_conn = pool.get().expect("pool");
    let report = elohim_storage::services::bootstrap_manifests::seed_if_empty(&mut seed_conn)
        .expect("bootstrap manifests seed must succeed at startup");
    tracing::info!(?report, "bootstrap manifests seeded");
}
```

(Use the actual crate path for the storage crate — `crate::` if main.rs is inside elohim-storage, otherwise `elohim_storage::`.)

- [x] **Step 3: Build**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(epr-light-up): T20 — wire bootstrap_manifests::seed_if_empty into startup"
```

---

## Task 21: main.rs — TTL sweep task spawn

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`

- [x] **Step 1: Add the spawn after seed_if_empty (Task 20), before HTTP server bind**

```rust
// Phase 3.5 — Light Up the Graph: tending TTL sweep task (5-min interval).
let sweep_pool = pool.clone();
let sweep_token = shutdown_token.clone();  // existing CancellationToken in startup
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if sweep_token.is_cancelled() {
            tracing::info!("tending sweep: shutdown requested, exiting");
            break;
        }
        match sweep_pool.get() {
            Ok(mut conn) => {
                match elohim_storage::services::tending::sweep_expired(&mut conn) {
                    Ok(n) => tracing::debug!(deleted = n, "tending sweep completed"),
                    Err(e) => tracing::warn!(?e, "tending sweep failed"),
                }
            }
            Err(e) => tracing::warn!(?e, "tending sweep: pool acquisition failed"),
        }
    }
});
```

(Use the actual `shutdown_token` / `CancellationToken` variable name from existing main.rs — search for `CancellationToken` or `shutdown` to find it.)

- [x] **Step 2: Build + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(epr-light-up): T21 — spawn tending TTL sweep task (5-min, shutdown-aware)"
```

---

## Task 22: main.rs — wire ManifestDebitWeightPolicy + adapters into HTTP shared state

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`
- Modify: shared state struct (likely in `src/state.rs` or `src/api/state.rs` or inline in main.rs)

- [x] **Step 1: Locate the shared state struct**

```bash
grep -rn "pub struct AppState\|pub struct SharedState\|http_state" elohim/elohim-storage/src/ | head -10
```

- [x] **Step 2: Add fields to shared state**

Add these fields (adapt names to existing struct):

```rust
pub struct AppState {
    // ...existing fields...
    pub manifest_registry: std::sync::Arc<crate::services::manifest_registry::ManifestRegistry>,
    pub debit_weight_policy: std::sync::Arc<crate::services::standing_projector::ManifestDebitWeightPolicy>,
    pub outbound_sink: Option<std::sync::Arc<dyn crate::services::back_prop::OutboundSink>>,
    pub gossip_publisher: Option<std::sync::Arc<dyn crate::services::gossip_flood::GossipPublisher>>,
    pub local_peer_id: String,
    pub local_pubkey: Vec<u8>,
    pub standing_policy_cid: Option<String>,
    pub sealing_keys: Option<std::sync::Arc<crate::services::sealed_against_self::SealingKeyBundle>>,
}
```

- [x] **Step 3: Construct the new fields in `main.rs` after seed_if_empty + before HTTP bind**

```rust
let registry = std::sync::Arc::new(
    elohim_storage::services::manifest_registry::ManifestRegistry::load_from_db(&mut pool.get().unwrap())
        .expect("manifest registry load")
);
let policy = std::sync::Arc::new(
    elohim_storage::services::standing_projector::ManifestDebitWeightPolicy::from_registry(&registry)
);
let sink: Option<std::sync::Arc<dyn elohim_storage::services::back_prop::OutboundSink>> = Some(
    std::sync::Arc::new(elohim_storage::p2p::adapters::LibP2POutboundSink::new(p2p_command_tx.clone()))
);
let publisher: Option<std::sync::Arc<dyn elohim_storage::services::gossip_flood::GossipPublisher>> = Some(
    std::sync::Arc::new(elohim_storage::p2p::adapters::LibP2PGossipPublisher::new(p2p_command_tx.clone()))
);
```

Plug `registry`, `policy`, `sink`, `publisher`, `local_peer_id`, `local_pubkey`, `standing_policy_cid` into the AppState struct passed to the HTTP router. The actual `p2p_command_tx` channel handle should already exist from the swarm task spawn — find it with `grep "P2PCommand\|command_tx" main.rs`.

- [x] **Step 4: Build + commit**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
git add elohim/elohim-storage/src/main.rs elohim/elohim-storage/src/state.rs   # adapt
git commit -m "feat(epr-light-up): T22 — wire registry/policy/adapters into HTTP shared state"
```

---

## Task 23: tests/startup_wiring.rs — smoke test

**Files:**
- Create: `elohim/elohim-storage/tests/startup_wiring.rs`

- [x] **Step 1: Create the test**

```rust
//! Startup wiring smoke test — verifies seed_if_empty + sweep_expired
//! contract behaviours that main.rs depends on.

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use elohim_storage::db::run_migrations;
use elohim_storage::services::bootstrap_manifests::seed_if_empty;
use elohim_storage::services::tending::sweep_expired;

fn pool() -> elohim_storage::db::DbPool {
    let url = format!("file:startup_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple());
    let p = Pool::builder().max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url)).expect("pool");
    run_migrations(&p).expect("migrations");
    p
}

#[test]
fn seed_if_empty_seeds_manifests_first_time() {
    let pool = pool();
    let mut conn = pool.get().unwrap();
    let report = seed_if_empty(&mut conn).expect("seed");
    assert!(report.standing_policy_seeded || report.tending_policy_seeded,
        "at least one bootstrap manifest must be seeded on first run");
}

#[test]
fn seed_if_empty_is_idempotent() {
    let pool = pool();
    let mut conn = pool.get().unwrap();
    let _ = seed_if_empty(&mut conn).expect("first");
    let report = seed_if_empty(&mut conn).expect("second");
    assert!(!report.standing_policy_seeded, "should be no-op after first seed");
    assert!(!report.tending_policy_seeded, "should be no-op after first seed");
}

#[test]
fn sweep_expired_clean_db_returns_zero() {
    let pool = pool();
    let mut conn = pool.get().unwrap();
    let deleted = sweep_expired(&mut conn).expect("sweep");
    assert_eq!(deleted, 0);
}
```

- [x] **Step 2: Run**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test startup_wiring
```
Expected: PASS (3 tests).

- [x] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/startup_wiring.rs
git commit -m "test(epr-light-up): T23 — startup_wiring smoke test (seed + sweep contracts)"
```

---

## Task 24: Audit aunt_and_rage_bait_integration.rs and identify mock sites

**Files:**
- Read: `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs`

- [x] **Step 1: Find the `MOCKED STEP` markers**

```bash
grep -n "MOCKED STEP\|MOCK\|TODO(T20)" elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs
```

- [x] **Step 2: Read the surrounding context for each mock**

For each marker found, read 30 lines of context before and after:

```bash
sed -n '<line-30>,<line+30>p' elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs
```

Document what each mock simulates:
- Mock #1 (reach gate fails Bob's compose): which function, what assertions
- Mock #2 (Vouch + restitution recovers Bob): which function, what data flow

- [x] **Step 3: No commit — this is a reading task. Take notes for tasks 25 and 26.**

---

## Task 25: Lift Mock #1 — reach-earning gate fails Bob's compose

**Files:**
- Modify: `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs`

- [x] **Step 1: Replace Mock #1 scaffold with real `compose_epr` call**

Change the mocked block (typically a comment-out + assertion stub) to:

```rust
// ============================================================================
// Phase: Bob attempts to recompose at district reach AFTER Sarah's correction
// ============================================================================
{
    let bob_evaluator_pubkey: &[u8] = &bob_keypair.public.as_bytes();
    let bob_author_pubkey: &[u8] = bob_evaluator_pubkey;
    let mut conn = bob_node.pool.get().expect("bob conn");
    let registry = bob_node.manifest_registry.clone();

    // Bob's standing is now negative because of Sarah's correction with debit-firm.
    // The reach-earning gate must Block district reach.
    use elohim_storage::services::epr_compose::{compose_epr, ComposeError};
    use elohim_storage::services::epr_kind::Reach;
    use elohim_storage::services::reach_earning::{ReachVerdict, BlockReason};

    let result = compose_epr(
        bob_evaluator_pubkey,
        bob_author_pubkey,
        Reach::District,
        &mut conn,
        &registry,
    );
    match result {
        Err(ComposeError::ReachDenied(ReachVerdict::Blocked { reason: BlockReason::StandingBelowThreshold, .. })) => {
            // Expected: Bob's compose at district reach is blocked because
            // his standing dropped below the manifest's "neutral" threshold.
        }
        other => panic!("expected Blocked(StandingBelowThreshold), got {:?}", other),
    }
}
```

(Adapt `bob_keypair`, `bob_node.pool`, `bob_node.manifest_registry`, the `Bob` evaluator's actual binding to whatever the test currently uses. The test already has a `bob_node` with components — extend it.)

- [x] **Step 2: Add `manifest_registry` to the test node helper if missing**

If the test's per-node helper struct doesn't already have a `manifest_registry`, add the field and populate it via `ManifestRegistry::load_from_db` after running migrations + seed_if_empty.

- [x] **Step 3: Run the test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test aunt_and_rage_bait_integration -- --nocapture --test-threads=1
```
Expected: PASS through Mock #1 lifted phase.

- [x] **Step 4: Commit**

```bash
git add elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs
git commit -m "test(epr-light-up): T25 — lift Mock #1 (reach-earning gate fails Bob's compose)"
```

---

## Task 26: Lift Mock #2 — Vouch primitive recovers Bob

**Files:**
- Modify: `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs`

- [x] **Step 1: Replace Mock #2 scaffold with real Vouch flow**

Replace the `// MOCKED STEP: Sarah signs Vouch...` block with:

```rust
// ============================================================================
// Phase: Bob's restitution + Sarah's Vouch — real protocol step
// ============================================================================

// 6a. Bob publishes a Correction acknowledging Sarah's correction (restitution).
let bob_restitution_signal = FeedbackSignal {
    target_cid: sarah_correction_cid.clone(),
    signal_kind: SignalKind::Correction,
    vouch_kind: None,
    evidence_cid: Some(sarah_correction_cid.clone()),
    standing_impact: StandingImpact::Advisory,
    signed_by: BASE64.encode(bob_keypair.public.as_bytes()),
    signature: BASE64.encode([0xBB; 64]),
};
let bob_restitution_payload = rmp_serde::to_vec(&bob_restitution_signal).unwrap();
let bob_restitution_cid = bob_node.put_epr("feedback-signal", &bob_restitution_payload).await.unwrap();

// 6b. Sarah Vouches Bob's restitution as AcceptCorrection.
//     This emits a FeedbackSignal { signal_kind: "vouch", vouch_kind: "accept-correction" }
//     with debit-soft impact, which the manifest weights as -3 (recovery).
let sarah_vouch_signal = FeedbackSignal {
    target_cid: bob_restitution_cid.clone(),
    signal_kind: SignalKind::Vouch,
    vouch_kind: Some(VouchKind::AcceptCorrection),
    evidence_cid: None,
    standing_impact: StandingImpact::DebitSoft,
    signed_by: BASE64.encode(sarah_keypair.public.as_bytes()),
    signature: BASE64.encode([0xCC; 64]),
};
let sarah_vouch_payload = rmp_serde::to_vec(&sarah_vouch_signal).unwrap();
let _sarah_vouch_cid = sarah_node.put_epr("feedback-signal", &sarah_vouch_payload).await.unwrap();

// Allow the gossip-flood + project_signal to propagate to Bob's node.
tokio::time::sleep(std::time::Duration::from_millis(500)).await;

// 6c. Bob's compose at district reach should now succeed: vouch debit-soft = -3
//     reduced his debit_weight_sum below the threshold.
{
    let bob_evaluator: &[u8] = bob_keypair.public.as_bytes();
    let mut conn = bob_node.pool.get().unwrap();
    let registry = bob_node.manifest_registry.clone();
    use elohim_storage::services::epr_compose::compose_epr;
    use elohim_storage::services::epr_kind::Reach;
    let result = compose_epr(
        bob_evaluator,
        bob_evaluator,
        Reach::District,
        &mut conn,
        &registry,
    );
    assert!(result.is_ok(), "after vouch, Bob's compose at district should be Allowed: {:?}", result);
}
```

(Adapt `bob_node.put_epr`, `sarah_node.put_epr`, `sarah_correction_cid`, `bob_keypair`, `sarah_keypair` to actual variable names.)

- [x] **Step 2: Remove the two `MOCKED STEP` comments and any stale assertion stubs**

```bash
grep -n "MOCKED STEP" elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs
```
Should return no results after this task.

- [x] **Step 3: Run**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test aunt_and_rage_bait_integration -- --nocapture --test-threads=1
```
Expected: PASS end-to-end. Both mocks lifted.

- [x] **Step 4: Verify the existing 2-of-2 negative sealed-decrypt assertion still passes**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test aunt_and_rage_bait_integration negative_sealed_decrypt
```
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs
git commit -m "test(epr-light-up): T26 — lift Mock #2 (Vouch primitive recovers Bob's standing)"
```

---

## Task 27: Quality gates + push

**Files:** entire crate

- [x] **Step 1: Format**

```bash
cd elohim/elohim-storage
cargo fmt
git diff --stat
git add -u && git commit -m "chore(epr-light-up): T27 — cargo fmt"   # if fmt produced changes
```

- [x] **Step 2: Clippy**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --all-targets -- -D warnings
```
Expected: PASS.

- [x] **Step 3: Build release**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: PASS.

- [x] **Step 4: Run all storage tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --release -- --test-threads=1
```
Expected: PASS.

- [x] **Step 5: Schema validation**

```bash
cd /projects/elohim/.claude/worktrees/light-up-graph
pnpm install
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts
git diff --stat elohim/sdk/schemas/generated-ts/
```
Expected: PASS. If codegen produced any drift, commit it.

- [ ] **Step 6: Push branch**

<!-- AUDIT 2026-05-11: NOT APPLICABLE. Work landed directly on dev (per feedback_dev_branch_no_pr memory pin). No feature/light-up-graph branch was created. Jenkins CI validates via the normal dev push trigger. -->

```bash
git push -u origin feature/light-up-graph
```

- [ ] **Step 7: Verify Jenkins picked up the build via orchestrator**

<!-- AUDIT 2026-05-11: Deferred to Wave-4 soak. CI validates on dev push; separate Jenkins verification step not tracked here. -->

Use Jenkins MCP to check: orchestrator picks up changes to elohim-storage + DNA, kicks off App + Edge + DNA pipelines. Sweettest validation runs on Jenkins. Watch for green build on the DNA pipeline (validates the integrity validator + create_vouch coordinator) and the App pipeline (validates aunt-and-rage-bait integration end-to-end).

- [ ] **Step 8: Final commit (if any drift)**

```bash
git add -u
git commit -m "chore(epr-light-up): T27 — quality gates clean (fmt, clippy, tests, schema, codegen)"
git push
```

---

## Self-Review Checklist (run after writing all tasks)

Verify each spec section maps to at least one task:

| Spec section | Implementing task(s) |
|--------------|----------------------|
| Six wiring sites — api/epr.rs fan-out | T17, T18, T19 |
| Six wiring sites — main.rs reconciliation | T20, T21, T22 |
| ManifestDebitWeightPolicy | T9 |
| LibP2P swarm adapters | T14, T15 |
| Reach-earning gate | T12, T13 |
| Vouch primitive (schema, validator, coordinator, sweettest) | T1, T4, T5, T6, T7 |
| Standing-policy schema extensions | T2 |
| Standing/Reach helpers | T10, T11 |
| ManifestRegistry accessors | T8 |
| Tending TTL sweep | T16, T21 |
| T20 mock-lifting | T24, T25, T26 |
| Smoke / startup tests | T23 |
| CI quality gates | T27 |

All sections covered. ✓

---

## Notes for the Implementer

- **Eclipse Che cannot run sweettests** (per memory `feedback_shift_measure_jenkins`). T7's sweettest is build-only locally; Jenkins validates execution. T27 step 7 is mandatory.
- **`--test-threads=1`** is required for integration tests that touch env vars or shared SQLite memory DBs (per memory `feedback_env_var_test_flakiness`).
- **Schema codegen Prettier oscillation** is a known cosmetic drift on union-type files (per memory `feedback_codegen_prettier_oscillation`). If `git diff` shows only Prettier-style changes after `schema:codegen:ts`, those are non-blocking.
- **No HTTP route changes** — Vouch rides existing `PUT /api/v1/epr`. The reach gate is on the author-side compose path only, never on the receive path.
- **Trust the trait abstractions.** OutboundSink, GossipPublisher, DebitWeightPolicy were designed for this sprint. Don't refactor — just inject the new production impls.
- **DHT remains canonical.** Network publishes are best-effort; a single `tracing::warn!` and continue is the right policy. Never `5xx` on swarm publish failure.
