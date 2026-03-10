# Notary Anchors + SDK Boundary Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Activate DHT-first writes for Agreement, Commitment, and EconomicEvent with Fulfillment links, post-commit projection to elohim-storage, and CLAUDE.md SDK boundary markers.

**Architecture:** Coordinator functions write to DHT (the notary). Post-commit signals project entries to elohim-storage (the index) with `dht_anchor_hash` for cryptographic verification. Clients call the conductor directly (steward path) or via doorway (hosted path). Storage is the fast query layer, DHT is the truth.

**Tech Stack:** Rust (HDK 0.6, HDI 0.7, Diesel ORM, Hyper HTTP), Holochain WASM zomes, SQLite

**Design doc:** `genesis/plans/2026-03-10-notary-anchors-sdk-boundary-design.md`

**Build note:** Zome code targets `wasm32-unknown-unknown` with `RUSTFLAGS='--cfg getrandom_backend="custom"'`. Use `just check` from `holochain/dna/elohim/` for fast type-checking. elohim-storage native builds use `RUSTFLAGS='--cfg getrandom_backend="custom"'` too.

---

### Task 1: Agreement Entry Type (Integrity Zome)

**Files:**
- Modify: `holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Context:** The integrity zome already has 8 REA entry types (EconomicEvent, Commitment, etc.) but Agreement is missing — it's referenced by `clause_of` on Commitment and `realization_of` on EconomicEvent but has no DHT entry. We add it.

**Step 1: Add Agreement struct**

Find the section with Commitment (around line 1350). Add Agreement BEFORE Commitment (it's the parent entity):

```rust
// =============================================================================
// Shefa: Agreement Entry
// =============================================================================

/// Agreement — bilateral contract linking paired Commitments.
///
/// Deliberately thin: the Commitments carry the terms (quantities, timing,
/// actions). The Agreement just proves "these commitments belong together."
/// If a capability could be centralized for rent extraction, it must be
/// notarized on distributed infrastructure — Agreement is the anchor that
/// makes paired give/take commitments cryptographically provable.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Agreement {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
```

**Step 2: Register in EntryTypes enum**

Find the `EntryTypes` enum (around line 4029). Add `Agreement(Agreement)` in the Shefa/Economy section, near `EconomicEvent` and `Commitment`:

```rust
    // Shefa: Economy (REA/ValueFlows)
    EconomicEvent(EconomicEvent),
    EconomicResource(EconomicResource),
    Agreement(Agreement),  // ADD THIS
```

**Step 3: Add Fulfillment link type**

Find the `LinkTypes` enum (around line 4156). Add these in the Shefa section:

```rust
    // Shefa: REA Fulfillment + Agreement links
    EventFulfillsCommitment,    // EconomicEvent → Commitment (Fulfillment)
    AgreementToCommitment,      // Agreement → Commitment (clause_of reverse)
    AgreementToEvent,           // Agreement → EconomicEvent (realization_of reverse)
    IdToAgreement,              // StringAnchor(id) → Agreement
    IdToReaCommitment,          // StringAnchor(id) → Commitment (REA, not Custodian)
    IdToEconomicEvent,          // StringAnchor(id) → EconomicEvent
    ProviderToCommitment,       // StringAnchor(provider) → Commitment
    ReceiverToCommitment,       // StringAnchor(receiver) → Commitment
    ProviderToEvent,            // StringAnchor(provider) → EconomicEvent
```

**Step 4: Verify compilation**

```bash
cd holochain/dna/elohim
just check
```

Expected: Compiles for wasm32-unknown-unknown (warnings OK, no errors).

**Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/dna/elohim/zomes/content_store_integrity/
git commit -m "feat(dna): add Agreement entry type + Fulfillment link types

Agreement is the bilateral contract anchor linking paired give/take
Commitments. Fulfillment links connect EconomicEvent → Commitment
on the DHT for graph traversal. These are the notary primitives
that prevent economic activity from being centralized.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Coordinator Functions — Agreement

**Files:**
- Modify: `holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Context:** Follow the existing create_custodian_commitment pattern (line ~8422): create entry, hash entry, create StringAnchor link for ID lookup. Agreement is thin — just id, name, note, created_at.

**Step 1: Add input/output structs**

Find an appropriate location near the REA section (after CustodianCommitment functions or near the end of the economic section). Add:

```rust
// =============================================================================
// REA Agreement — Coordinator
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgreementInput {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgreementOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub agreement: Agreement,
}
```

**Step 2: Add create_agreement function**

```rust
#[hdk_extern]
pub fn create_agreement(input: CreateAgreementInput) -> ExternResult<AgreementOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let agreement = Agreement {
        id: input.id.clone(),
        name: input.name,
        note: input.note,
        created_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Agreement(agreement.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Agreement(agreement.clone()))?;

    // StringAnchor link for ID-based lookup
    let id_anchor = StringAnchor::new("agreement_id", &agreement.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToAgreement,
        (),
    )?;

    Ok(AgreementOutput {
        action_hash,
        entry_hash,
        agreement,
    })
}
```

**Step 3: Add get_agreement function**

```rust
#[hdk_extern]
pub fn get_agreement(id: String) -> ExternResult<Option<AgreementOutput>> {
    let id_anchor = StringAnchor::new("agreement_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToAgreement)?.build(),
    )?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".into())))?;
        let record = get(action_hash.clone(), GetOptions::default())?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Agreement record not found".into())))?;
        let agreement: Agreement = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("No entry in record".into())))?;
        let entry_hash = hash_entry(&EntryTypes::Agreement(agreement.clone()))?;

        Ok(Some(AgreementOutput {
            action_hash,
            entry_hash,
            agreement,
        }))
    } else {
        Ok(None)
    }
}
```

**Step 4: Verify compilation**

```bash
cd holochain/dna/elohim
just check
```

**Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/dna/elohim/zomes/content_store/
git commit -m "feat(dna): add Agreement coordinator functions

create_agreement and get_agreement with StringAnchor ID indexing.
Agreement is the bilateral contract anchor for paired Commitments.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Coordinator Functions — Commitment + EconomicEvent

**Files:**
- Modify: `holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Context:** The integrity zome already defines Commitment and EconomicEvent entry types. We add coordinator functions that write them to the DHT with proper StringAnchor links and Fulfillment links. Follow the create_custodian_commitment pattern.

**Step 1: Add Commitment input/output structs**

```rust
// =============================================================================
// REA Commitment — Coordinator
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReaCommitmentInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Vec<String>,
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Vec<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub commitment: Commitment,
}
```

**Step 2: Add create_rea_commitment function**

```rust
#[hdk_extern]
pub fn create_rea_commitment(input: CreateReaCommitmentInput) -> ExternResult<ReaCommitmentOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let commitment = Commitment {
        id: input.id.clone(),
        action: input.action,
        provider: input.provider.clone(),
        receiver: input.receiver.clone(),
        resource_conforms_to: input.resource_conforms_to,
        resource_inventoried_as: None,
        resource_classified_as_json: serde_json::to_string(&input.resource_classified_as)
            .unwrap_or_else(|_| "[]".to_string()),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit,
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit,
        has_point_in_time: None,
        has_beginning: input.has_beginning,
        has_end: input.has_end,
        due: input.due,
        clause_of: input.clause_of.clone(),
        agreed_in: None,
        input_of: None,
        output_of: None,
        satisfies: None,
        in_scope_of_json: serde_json::to_string(&input.in_scope_of)
            .unwrap_or_else(|_| "[]".to_string()),
        finished: false,
        state: "proposed".to_string(),
        note: input.note,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Commitment(commitment.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Commitment(commitment.clone()))?;

    // ID anchor
    let id_anchor = StringAnchor::new("rea_commitment_id", &commitment.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToReaCommitment, ())?;

    // Provider anchor
    let provider_anchor = StringAnchor::new("commitment_provider", &commitment.provider);
    let provider_hash = hash_entry(&EntryTypes::StringAnchor(provider_anchor))?;
    create_link(provider_hash, action_hash.clone(), LinkTypes::ProviderToCommitment, ())?;

    // Receiver anchor
    let receiver_anchor = StringAnchor::new("commitment_receiver", &commitment.receiver);
    let receiver_hash = hash_entry(&EntryTypes::StringAnchor(receiver_anchor))?;
    create_link(receiver_hash, action_hash.clone(), LinkTypes::ReceiverToCommitment, ())?;

    // Agreement link (if clause_of is set)
    if let Some(ref agreement_id) = input.clause_of {
        let agreement_anchor = StringAnchor::new("agreement_id", agreement_id);
        let agreement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agreement_anchor))?;
        // Link from agreement anchor to this commitment
        create_link(agreement_anchor_hash, action_hash.clone(), LinkTypes::AgreementToCommitment, ())?;
    }

    Ok(ReaCommitmentOutput {
        action_hash,
        entry_hash,
        commitment,
    })
}
```

**Step 3: Add get_rea_commitment and get_commitments_by_agreement**

```rust
#[hdk_extern]
pub fn get_rea_commitment(id: String) -> ExternResult<Option<ReaCommitmentOutput>> {
    let id_anchor = StringAnchor::new("rea_commitment_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToReaCommitment)?.build(),
    )?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".into())))?;
        let record = get(action_hash.clone(), GetOptions::default())?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Commitment not found".into())))?;
        let commitment: Commitment = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialize: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("No entry".into())))?;
        let entry_hash = hash_entry(&EntryTypes::Commitment(commitment.clone()))?;

        Ok(Some(ReaCommitmentOutput { action_hash, entry_hash, commitment }))
    } else {
        Ok(None)
    }
}

#[hdk_extern]
pub fn get_commitments_by_agreement(agreement_id: String) -> ExternResult<Vec<ReaCommitmentOutput>> {
    let agreement_anchor = StringAnchor::new("agreement_id", &agreement_id);
    let agreement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agreement_anchor))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(agreement_anchor_hash, LinkTypes::AgreementToCommitment)?.build(),
    )?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".into())))?;
        if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
            if let Some(commitment) = record.entry().to_app_option::<Commitment>().ok().flatten() {
                let entry_hash = hash_entry(&EntryTypes::Commitment(commitment.clone()))?;
                results.push(ReaCommitmentOutput { action_hash, entry_hash, commitment });
            }
        }
    }
    Ok(results)
}
```

**Step 4: Add EconomicEvent input/output structs**

```rust
// =============================================================================
// REA EconomicEvent — Coordinator
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReaEconomicEventInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_classified_as: Vec<String>,
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    pub fulfills: Vec<String>,               // Commitment IDs
    pub realization_of: Option<String>,       // Agreement ID
    pub lamad_event_type: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaEconomicEventOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub event: EconomicEvent,
    pub fulfillment_links: Vec<ActionHash>,   // Created Fulfillment link hashes
}
```

**Step 5: Add create_rea_economic_event function**

```rust
#[hdk_extern]
pub fn create_rea_economic_event(input: CreateReaEconomicEventInput) -> ExternResult<ReaEconomicEventOutput> {
    let event = EconomicEvent {
        id: input.id.clone(),
        action: input.action,
        provider: input.provider.clone(),
        receiver: input.receiver.clone(),
        resource_conforms_to: None,
        resource_inventoried_as: None,
        to_resource_inventoried_as: None,
        resource_classified_as_json: serde_json::to_string(&input.resource_classified_as)
            .unwrap_or_else(|_| "[]".to_string()),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit,
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit,
        has_point_in_time: input.has_point_in_time,
        has_duration: None,
        input_of: None,
        output_of: None,
        fulfills_json: serde_json::to_string(&input.fulfills)
            .unwrap_or_else(|_| "[]".to_string()),
        realization_of: input.realization_of.clone(),
        satisfies_json: "[]".to_string(),
        in_scope_of_json: "[]".to_string(),
        note: input.note,
        state: "completed".to_string(),
        triggered_by: None,
        at_location: None,
        image: None,
        lamad_event_type: input.lamad_event_type,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: format!("{:?}", sys_time()?),
    };

    let action_hash = create_entry(&EntryTypes::EconomicEvent(event.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::EconomicEvent(event.clone()))?;

    // ID anchor
    let id_anchor = StringAnchor::new("economic_event_id", &event.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToEconomicEvent, ())?;

    // Provider anchor
    let provider_anchor = StringAnchor::new("event_provider", &event.provider);
    let provider_hash = hash_entry(&EntryTypes::StringAnchor(provider_anchor))?;
    create_link(provider_hash, action_hash.clone(), LinkTypes::ProviderToEvent, ())?;

    // Fulfillment links: Event → each Commitment
    let mut fulfillment_links = Vec::new();
    for commitment_id in &input.fulfills {
        let commitment_anchor = StringAnchor::new("rea_commitment_id", commitment_id);
        let commitment_anchor_hash = hash_entry(&EntryTypes::StringAnchor(commitment_anchor))?;

        let commitment_links = get_links(
            GetLinksInputBuilder::try_new(commitment_anchor_hash, LinkTypes::IdToReaCommitment)?.build(),
        )?;

        if let Some(commitment_link) = commitment_links.first() {
            let commitment_action_hash = ActionHash::try_from(commitment_link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid commitment hash".into())))?;

            let link_hash = create_link(
                action_hash.clone(),
                commitment_action_hash,
                LinkTypes::EventFulfillsCommitment,
                (),
            )?;
            fulfillment_links.push(link_hash);
        }
    }

    // Agreement link (if realization_of is set)
    if let Some(ref agreement_id) = input.realization_of {
        let agreement_anchor = StringAnchor::new("agreement_id", agreement_id);
        let agreement_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agreement_anchor))?;
        create_link(agreement_anchor_hash, action_hash.clone(), LinkTypes::AgreementToEvent, ())?;
    }

    Ok(ReaEconomicEventOutput {
        action_hash,
        entry_hash,
        event,
        fulfillment_links,
    })
}
```

**Step 6: Add get_rea_economic_event**

```rust
#[hdk_extern]
pub fn get_rea_economic_event(id: String) -> ExternResult<Option<ReaEconomicEventOutput>> {
    let id_anchor = StringAnchor::new("economic_event_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToEconomicEvent)?.build(),
    )?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".into())))?;
        let record = get(action_hash.clone(), GetOptions::default())?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Event not found".into())))?;
        let event: EconomicEvent = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialize: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("No entry".into())))?;
        let entry_hash = hash_entry(&EntryTypes::EconomicEvent(event.clone()))?;

        // Get fulfillment links from this event
        let fulfillment_links = get_links(
            GetLinksInputBuilder::try_new(action_hash.clone(), LinkTypes::EventFulfillsCommitment)?.build(),
        )?;
        let link_hashes: Vec<ActionHash> = fulfillment_links.iter()
            .filter_map(|l| ActionHash::try_from(l.target.clone()).ok())
            .collect();

        Ok(Some(ReaEconomicEventOutput {
            action_hash,
            entry_hash,
            event,
            fulfillment_links: link_hashes,
        }))
    } else {
        Ok(None)
    }
}
```

**Step 7: Verify compilation**

```bash
cd holochain/dna/elohim
just check
```

**Step 8: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/dna/elohim/zomes/content_store/
git commit -m "feat(dna): add REA coordinator functions for Commitment + EconomicEvent

create/get for both entry types with StringAnchor indexing by ID,
provider, receiver. Fulfillment links (Event → Commitment) created
during event creation. Agreement→Commitment and Agreement→Event
links for bilateral contract traversal.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: Post-Commit Signals for REA Entries

**Files:**
- Modify: `holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Context:** The coordinator already emits ProjectionSignal variants in the post_commit handler (line ~10290). We add three new signal variants and wire them into the post_commit match chain.

**Step 1: Add signal variants to ProjectionSignal enum**

Find the `ProjectionSignal` enum (around line 10083). Add:

```rust
    /// REA Agreement committed to DHT
    AgreementCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        agreement: Agreement,
        author: AgentPubKey,
    },
    /// REA Commitment committed to DHT
    ReaCommitmentCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        commitment: Commitment,
        author: AgentPubKey,
    },
    /// REA EconomicEvent committed to DHT
    ReaEconomicEventCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        event: EconomicEvent,
        author: AgentPubKey,
    },
```

**Step 2: Add to post_commit match chain**

Find the post_commit handler (around line 10290) where it matches entry types. Add these cases in the `if let Some(...) = record.entry().to_app_option::<...>()` chain:

```rust
} else if let Some(agreement) = record.entry().to_app_option::<Agreement>().ok().flatten() {
    emit_signal(ProjectionSignal::AgreementCommitted {
        action_hash: action_hash.clone(),
        entry_hash: entry_hash.clone(),
        agreement,
        author: author.clone(),
    })?;
} else if let Some(commitment) = record.entry().to_app_option::<Commitment>().ok().flatten() {
    // Only emit for REA commitments, not CustodianCommitments (which are a different type)
    emit_signal(ProjectionSignal::ReaCommitmentCommitted {
        action_hash: action_hash.clone(),
        entry_hash: entry_hash.clone(),
        commitment,
        author: author.clone(),
    })?;
} else if let Some(event) = record.entry().to_app_option::<EconomicEvent>().ok().flatten() {
    emit_signal(ProjectionSignal::ReaEconomicEventCommitted {
        action_hash: action_hash.clone(),
        entry_hash: entry_hash.clone(),
        event,
        author: author.clone(),
    })?;
```

**Important:** Make sure the Commitment match is AFTER any CustodianCommitment match (they're different types — CustodianCommitment already has its own signal). The `to_app_option::<Commitment>()` will only match the REA Commitment entry type.

**Step 3: Verify compilation**

```bash
cd holochain/dna/elohim
just check
```

**Step 4: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/dna/elohim/zomes/content_store/
git commit -m "feat(dna): add post-commit signals for REA Agreement, Commitment, Event

AgreementCommitted, ReaCommitmentCommitted, ReaEconomicEventCommitted
signal variants. Emitted in post_commit handler so elohim-storage can
project DHT entries to SQLite with dht_anchor_hash for verification.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Storage Migrations — dht_anchor_hash + agreements table

**Files:**
- Create: `holochain/elohim-storage/migrations/2026-03-10-100000_dht_anchor_hash/up.sql`
- Create: `holochain/elohim-storage/migrations/2026-03-10-100000_dht_anchor_hash/down.sql`
- Create: `holochain/elohim-storage/migrations/2026-03-10-200000_agreements/up.sql`
- Create: `holochain/elohim-storage/migrations/2026-03-10-200000_agreements/down.sql`
- Modify: `holochain/elohim-storage/src/db/diesel_schema.rs`

**Step 1: Create dht_anchor_hash migration**

Write `up.sql`:
```sql
-- Add DHT anchor hash to economic tables for cryptographic verification.
-- Records with non-null dht_anchor_hash are notarized on the DHT.
-- Records with null are storage-only (legacy/transitional).
ALTER TABLE economic_events ADD COLUMN dht_anchor_hash TEXT;
ALTER TABLE rea_commitments ADD COLUMN dht_anchor_hash TEXT;
```

Write `down.sql`:
```sql
-- SQLite doesn't support DROP COLUMN before 3.35.0
-- These are additive-only columns, safe to leave in place
```

**Step 2: Create agreements table migration**

Write `up.sql`:
```sql
-- Agreement — bilateral contract anchor linking paired Commitments.
-- Thin by design: Commitments carry the terms, Agreement proves pairing.
CREATE TABLE agreements (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    name TEXT,
    note TEXT,
    dht_anchor_hash TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agreement_app_id ON agreements(app_id);
```

Write `down.sql`:
```sql
DROP TABLE IF EXISTS agreements;
```

**Step 3: Run migrations (or manually update diesel_schema.rs)**

```bash
cd holochain/elohim-storage
diesel migration run
```

If diesel CLI unavailable, manually add to `diesel_schema.rs`:

Add `dht_anchor_hash -> Nullable<Text>` to the `economic_events` and `rea_commitments` table definitions.

Add:
```rust
diesel::table! {
    agreements (id) {
        id -> Text,
        app_id -> Text,
        name -> Nullable<Text>,
        note -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
    }
}
```

Add `agreements` to the `allow_tables_to_appear_in_same_query!()` macro.

**Step 4: Verify compilation**

```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

**Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/elohim-storage/migrations/ holochain/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add dht_anchor_hash columns + agreements table

dht_anchor_hash on economic_events and rea_commitments for DHT
verification. agreements table for bilateral contract projection.
Records with non-null anchor hash are notarized on distributed
infrastructure.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Agreement Model, Views, API in Storage

**Files:**
- Modify: `holochain/elohim-storage/src/db/models.rs`
- Create: `holochain/elohim-storage/src/db/agreements.rs`
- Modify: `holochain/elohim-storage/src/db/mod.rs`
- Modify: `holochain/elohim-storage/src/views.rs`
- Create: `holochain/elohim-storage/src/services/agreement_service.rs`
- Modify: `holochain/elohim-storage/src/services/mod.rs`
- Create: `holochain/elohim-storage/src/api/agreements.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Context:** Follow exact patterns from rea_commitments (Task 2-4 of previous plan). Agreement is simpler — fewer fields.

**Step 1: Add Diesel model to models.rs**

```rust
// ============================================================================
// Agreement
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::db::diesel_schema::agreements)]
pub struct AgreementRow {
    pub id: String,
    pub app_id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::diesel_schema::agreements)]
pub struct NewAgreement<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub name: Option<&'a str>,
    pub note: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}
```

**Step 2: Create db/agreements.rs with CRUD**

Follow the rea_commitments.rs pattern: `create`, `get_by_id`, `list` with query struct, upsert for projection.

**Step 3: Register `pub mod agreements;` in db/mod.rs**

**Step 4: Add AgreementView + CreateAgreementInputView to views.rs**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AgreementView {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
}
```

With `From<AgreementRow>` impl.

**Step 5: Create service + API route**

Follow rea_commitment_service.rs and api/rea_commitments.rs patterns. Routes:
```
POST   /api/v1/agreements
GET    /api/v1/agreements/{id}
GET    /api/v1/agreements
```

**Step 6: Wire route in api/mod.rs dispatcher**

**Step 7: Add dht_anchor_hash to existing models**

Update `ReaCommitment` and `EconomicEvent` (or equivalent) model structs in models.rs to include `pub dht_anchor_hash: Option<String>`. Update corresponding views to expose it.

**Step 8: Generate TypeScript types**

```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

**Step 9: Verify full build**

```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 10: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/elohim-storage/src/ holochain/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): Agreement model, views, API + dht_anchor_hash on REA types

Agreement CRUD matching existing patterns. dht_anchor_hash exposed on
AgreementView, ReaCommitmentView, EconomicEventView — non-null means
the record is notarized on the DHT.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 7: Storage Signal Projection Handler

**Files:**
- Modify: `holochain/elohim-storage/src/signals.rs`
- Possibly modify: `holochain/elohim-storage/src/conductor_client.rs`

**Context:** elohim-storage already has a signal handler for blob signals. We extend it (or add a parallel handler) to listen for ProjectionSignal variants from the content_store coordinator and upsert into SQLite.

**Step 1: Read signals.rs and conductor_client.rs**

Understand the current signal subscription flow. The conductor client connects via WebSocket and receives signals. We need to handle the new REA projection signals.

**Step 2: Add REA signal types**

Define Rust types matching the ProjectionSignal variants emitted by the coordinator:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ReaProjectionSignal {
    AgreementCommitted {
        action_hash: String,
        entry_hash: String,
        agreement: AgreementSignalPayload,
    },
    ReaCommitmentCommitted {
        action_hash: String,
        entry_hash: String,
        commitment: CommitmentSignalPayload,
    },
    ReaEconomicEventCommitted {
        action_hash: String,
        entry_hash: String,
        event: EconomicEventSignalPayload,
    },
}
```

The payload structs mirror the DHT entry types but use String fields (serialized from Holochain).

**Step 3: Add projection handler**

```rust
pub async fn handle_rea_projection(
    signal: ReaProjectionSignal,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<(), StorageError> {
    let mut conn = pool.get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;

    match signal {
        ReaProjectionSignal::AgreementCommitted { action_hash, agreement, .. } => {
            // Upsert: if exists by id, update dht_anchor_hash; else insert
            agreements::upsert(&mut conn, ctx, agreement.id, action_hash, agreement)?;
        }
        ReaProjectionSignal::ReaCommitmentCommitted { action_hash, commitment, .. } => {
            rea_commitments::upsert(&mut conn, ctx, commitment.id, action_hash, commitment)?;
        }
        ReaProjectionSignal::ReaEconomicEventCommitted { action_hash, event, .. } => {
            economic_events::upsert(&mut conn, ctx, event.id, action_hash, event)?;
        }
    }
    Ok(())
}
```

**Step 4: Add upsert functions to db modules**

Each db module (agreements.rs, rea_commitments.rs, economic_events.rs) gets an `upsert` function that:
- Tries to find existing record by `id`
- If found: updates `dht_anchor_hash`
- If not found: inserts full record with `dht_anchor_hash`

**Step 5: Wire signal handler into conductor client connection loop**

Find where signals are received in the WebSocket loop and add routing for REA projection signals.

**Step 6: Verify compilation**

```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 7: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/elohim-storage/src/
git commit -m "feat(storage): REA projection signal handler for DHT→storage sync

Receives AgreementCommitted, ReaCommitmentCommitted,
ReaEconomicEventCommitted signals from conductor. Upserts into
SQLite with dht_anchor_hash. DHT is the truth, storage is the index.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 8: CLAUDE.md SDK Boundary Markers

**Files:**
- Create: `holochain/dna/CLAUDE.md`
- Create: `holochain/sdk/CLAUDE.md`
- Create: `elohim-app/src/app/shefa/CLAUDE.md`

**Context:** These files teach the SDK boundary principle — not just what goes where, but WHY. The test is: if someone could capture this capability at scale and extract rent, it must be on distributed infrastructure.

**Step 1: Write holochain/dna/CLAUDE.md**

```markdown
# Holochain DNA — Protocol Integrity Layer

This directory contains the integrity and coordinator zomes that form the
Elohim Protocol's distributed truth layer. Every entry type here is a
**notary anchor** — a cryptographic proof that something happened, committed
to a distributed hash table that no single party controls.

## Why This Layer Exists

The question that determines what goes here:

**If this capability were centralized, could a handful of people extract
rent from everyone who uses it?**

If yes, it MUST be notarized here. If someone could become "the bank" by
controlling economic events, or "the credential authority" by controlling
attestations, or "the governance board" by controlling consent records —
that capability must live on distributed infrastructure where no one can
capture it.

The DHT has real limits: ~100 entry types per DNA, writes are slow and
expensive, queries require link traversal (no SQL). So the DHT is the
**notary**, not the database. It holds the minimal proof:

- **Who** — agent public key (unforgeable)
- **What** — content-addressed hash of the full record
- **When** — DHT timestamp (non-repudiable)

The full queryable data lives in elohim-storage (SQLite), projected from
post-commit signals with a `dht_anchor_hash` column that links back to the
DHT entry. Storage is the fast index. DHT is the truth. If they disagree,
the DHT wins.

## Entry Type Categories

### Notarized Protocol Primitives (SDK boundary)
These MUST be on the DHT because centralization = rent extraction:

- **Economic**: Agreement, Commitment, EconomicEvent, EconomicResource
  (Commitment + Event = bilateral promise→delivery chain; if centralized, someone becomes the bank)
- **Identity**: Human, Agent, HumanRelationship, Attestation
  (if centralized, someone becomes the identity provider)
- **Content**: Content, LearningPath, ContentAttestation
  (if centralized, someone becomes the content landlord)
- **Infrastructure**: NodeRegistration, DoorwayRegistration
  (self-registration, not centrally assigned)

### Operational Entries (DHT-appropriate but not SDK primitives)
- Heartbeats, shard assignments, health attestations
- These support the network but aren't capabilities a human interacts with

## Pattern: Post-Commit Projection

```
create_entry(EntryTypes::Commitment(c))  →  source chain + DHT gossip
post_commit signal                        →  ProjectionSignal::ReaCommitmentCommitted
elohim-storage receives signal            →  upsert into SQLite with dht_anchor_hash
```

Clients write to the conductor. Storage listens and indexes. Never write
to storage directly for notarized types (legacy code may still do this —
migrate toward conductor-first).

## Build

```bash
just check   # Fast type-check (wasm32-unknown-unknown)
just build   # Full WASM build
just pack    # Build + pack DNA
```

RUSTFLAGS is set in the justfile. Don't override it.
```

**Step 2: Write holochain/sdk/CLAUDE.md**

```markdown
# Elohim Protocol SDK

This directory contains the TypeScript surface of the Elohim Protocol —
the types and client libraries that applications (including elohim-app)
use to interact with the protocol's distributed infrastructure.

## What Belongs in the SDK

The SDK boundary is defined by one question:

**Could this capability be captured at scale for rent extraction?**

If a capability could let someone become the bank, the credential
authority, the governance board, or the content landlord — it's a
protocol primitive and its types belong here. Applications compose
these primitives into experiences; they don't own the primitives.

### Protocol Primitives (SDK types)
- Economic types: Agreement, Commitment, EconomicEvent, Measure
- Identity types: Human, Agent, Attestation, ContributorPresence
- Content types: Content, LearningPath, ContentMastery
- Governance types: (coming — consent records, proposals)

### NOT SDK (application or bridge layer)
- Doorway projection/cache types — doorway is a web2 bridge for hosted
  humans progressing toward stewardship, not a protocol primitive
- UI state, dashboard aggregations, theme preferences
- Quiz session state, streak tracking — app-level compositions

## Type Generation Pipeline

```
Rust structs (views.rs)
  → #[derive(TS)] + #[serde(rename_all = "camelCase")]
  → cargo test export_bindings
  → storage-client-ts/src/generated/*.ts
```

Types flow from Rust to TypeScript. Never hand-write TypeScript types
that mirror Rust structs — they will drift.

## storage-client-ts

The generated types in `storage-client-ts/src/generated/` are the
canonical TypeScript representation of the protocol's API boundary.
snake_case never leaves Rust. TypeScript receives camelCase with parsed
JSON and proper booleans.
```

**Step 3: Write elohim-app/src/app/shefa/CLAUDE.md**

```markdown
# Shefa Pillar — Economic Experience Layer

Shefa is the human experience of the Elohim Protocol's economic
infrastructure. It renders stewardship, banking, resource flows, and
compute sharing in ways humans can understand and interact with.

## Shefa is UX, Not Truth

The protocol primitives (economic events, commitments, agreements,
mutual credit) live on the Holochain DHT — distributed infrastructure
that no one can capture. Shefa services in this directory are the
**experience layer** that makes those primitives legible to humans.

The distinction matters: if an economic event is only recorded in an
Angular service's state, it can be lost, forged, or silently modified.
If it's notarized on the DHT and projected to storage with a
`dht_anchor_hash`, it's cryptographically provable. Shefa reads from
storage (fast), but writes should go through the conductor (truthful).

## Service Categories

### API Services (thin HTTP clients to storage projections)
Services like `EconomicEventsApiService`, `ExchangeApiService`,
`FlowPlanningApiService` read from elohim-storage's HTTP API. These
are reading the **projection** of DHT truth — fast and queryable,
but not the source of truth.

For writes, these services should call through to the Holochain
conductor (via HolochainClientService zome calls), which writes to
the DHT and projects back to storage via post-commit signals. Direct
storage writes bypass the notary and create un-notarized records
(dht_anchor_hash = null).

### Composition Services (app-level logic)
Services like `InsuranceMutualService`, `BudgetReconciliationService`
compose multiple protocol primitives into domain-specific workflows.
These belong in the app, not the SDK — they're how this particular
app interprets the protocol, not the protocol itself.

### Transition State
Some services currently POST directly to elohim-storage. As the
conductor-first pattern is wired up, these should migrate to:
1. Write via conductor zome call
2. Post-commit signal projects to storage
3. Read from storage HTTP API (unchanged)
```

**Step 4: Commit**

```bash
cd /home/matthew/git/elohim
git add holochain/dna/CLAUDE.md holochain/sdk/CLAUDE.md elohim-app/src/app/shefa/CLAUDE.md
git commit -m "docs: add SDK boundary CLAUDE.md markers

Three files explaining the protocol primitive boundary: if a capability
could be centralized for rent extraction at scale, it must be notarized
on distributed infrastructure. DHT is the notary, storage is the index,
shefa is the UX layer. Teaches the principle so future agents can apply
good judgement to capabilities we haven't thought of yet.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 9: Update Testnet Manager for Conductor-First Writes

**Files:**
- Modify: `genesis/a2o/src/framework/testnet-manager.ts`
- Modify: `genesis/a2o/src/framework/storage-client.ts`

**Context:** The testnet manager currently POSTs directly to elohim-storage. After the DHT-first pattern is wired, it should call the conductor for writes and let post-commit signals handle projection. However, the conductor client in TypeScript needs a way to make zome calls. For the testnet manager (steward path), this means calling the conductor's app WebSocket interface.

**Step 1: Add conductor client to storage-client.ts**

Add a `ConductorClient` class alongside the existing `StorageClient`:

```typescript
export class ConductorClient {
  constructor(private wsUrl: string = 'ws://localhost:8888') {}

  async callZome<T>(input: {
    zomeName: string;
    fnName: string;
    payload: unknown;
  }): Promise<T> {
    // For now, fall back to storage HTTP API
    // TODO: Wire actual WebSocket zome calls when conductor is available
    throw new Error('Conductor zome calls not yet implemented — use StorageClient fallback');
  }
}
```

**Step 2: Update startTestnet to try conductor-first, fall back to storage**

```typescript
// In startTestnet(), replace direct storage POST with:
try {
  // Try conductor-first (DHT notarized)
  const agreementResult = await conductorClient.callZome({
    zomeName: 'content_store',
    fnName: 'create_agreement',
    payload: { id: agreementId, name: `Compute sharing: ${personaList}` },
  });
  // ... create commitments via conductor ...
} catch {
  // Fall back to storage-only (transitional, dht_anchor_hash = null)
  console.warn('  Conductor unavailable — falling back to storage-only writes');
  // ... existing storage POST code ...
}
```

**Step 3: Document the transition**

Add a comment block at the top of testnet-manager.ts explaining the dual-write transition:

```typescript
/**
 * Write Path Transition:
 *
 * CURRENT: POST directly to elohim-storage (fast, no DHT notarization)
 * TARGET:  Zome call to conductor → post-commit signal → storage projection
 *
 * The conductor-first path creates dht_anchor_hash on storage records,
 * proving the economic activity was notarized on distributed infrastructure.
 * The storage-only fallback creates records with null dht_anchor_hash.
 *
 * Both paths produce identical storage records for querying. The difference
 * is cryptographic provability.
 */
```

**Step 4: Commit**

```bash
cd /home/matthew/git/elohim
git add genesis/a2o/src/framework/
git commit -m "feat(a2o): prepare testnet manager for conductor-first writes

ConductorClient stub for zome calls. startTestnet tries conductor
first, falls back to storage-only POST if conductor unavailable.
Documents the write path transition: DHT-notarized vs storage-only.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Agreement entry type + Fulfillment link types | Integrity zome |
| 2 | Agreement coordinator functions | Coordinator zome |
| 3 | Commitment + EconomicEvent coordinator functions | Coordinator zome |
| 4 | Post-commit projection signals | Coordinator zome |
| 5 | dht_anchor_hash migrations + agreements table | Storage DB |
| 6 | Agreement model, views, API + dht_anchor_hash on REA types | Storage API |
| 7 | Signal projection handler (DHT → storage) | Storage runtime |
| 8 | CLAUDE.md SDK boundary markers | Documentation |
| 9 | Testnet manager conductor-first writes | A2O framework |
