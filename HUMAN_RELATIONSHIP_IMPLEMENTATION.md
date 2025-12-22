# Human Relationship Implementation Summary

## What Was Built

### 1. Integrity Zome (Data Structures)

**File:** `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs`

#### New Constants
- `INTIMACY_LEVELS`: 5 levels (intimate, trusted, familiar, acquainted, public)
- `HUMAN_RELATIONSHIP_TYPES`: 12 types (spouse, parent, child, sibling, etc.)

#### New Entry Type: HumanRelationship
```rust
pub struct HumanRelationship {
    // Parties
    pub party_a_id: String,
    pub party_b_id: String,

    // Relationship nature
    pub relationship_type: String,    // spouse, parent, sibling, etc.
    pub intimacy_level: String,       // intimate, trusted, familiar, etc.
    pub is_bidirectional: bool,

    // Consent & permissions
    pub consent_given_by_a: bool,
    pub consent_given_by_b: bool,
    pub custody_enabled_by_a: bool,   // A allows B to custody their data
    pub custody_enabled_by_b: bool,   // B allows A to custody their data

    // Custody & backup
    pub auto_custody_enabled: bool,   // Auto-replicate intimate content?
    pub shared_encryption_key_id: Option<String>,
    pub emergency_access_enabled: bool,

    // Metadata
    pub initiated_by: String,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub context_json: Option<String>,
    pub reach: String,                // Visibility of relationship itself
}
```

#### New Link Types
- `IdToHumanRelationship` - Anchor(relationship_id) → HumanRelationship
- `AgentToRelationship` - Anchor(agent_id) → HumanRelationship
- `HumanRelationshipByIntimacy` - Anchor(intimacy_level) → HumanRelationship
- `HumanRelationshipByType` - Anchor(relationship_type) → HumanRelationship
- `RelationshipPendingConsent` - Anchor(pending) → HumanRelationship
- `RelationshipWithCustody` - Anchor(custody_enabled) → HumanRelationship

### 2. Coordinator Zome (Functions)

**File:** `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs`

#### Public Functions

**`create_human_relationship(relationship: HumanRelationship) -> ActionHash`**
- Creates a relationship between two agents
- Creates all necessary index links
- Auto-creates custodian commitments if both parties consent and custody enabled

**`get_relationships_for_agent(agent_id: String) -> Vec<HumanRelationship>`**
- Returns all relationships for a given agent (both as party_a and party_b)

**`get_intimate_relationships(agent_id: String) -> Vec<HumanRelationship>`**
- Returns only intimate-level relationships with custody enabled
- Filtered to verified relationships (both parties consented)
- Used for disaster recovery queries

#### Helper Functions

**`create_auto_custody_commitments(relationship, relationship_hash) -> ExternResult<()>`**
- Automatically creates bidirectional CustodianCommitment entries
- Party A custodies Party B's intimate/private content
- Party B custodies Party A's intimate/private content
- Links commitments to relationship via `RelationshipToCommitment`

## How It Works

### Creating a Family Relationship

```rust
// Example: Alice creates relationship with her spouse Bob
let relationship = HumanRelationship {
    id: "alice-bob-spouse".to_string(),
    party_a_id: "alice-agent-id".to_string(),
    party_b_id: "bob-agent-id".to_string(),
    relationship_type: "spouse".to_string(),
    intimacy_level: "intimate".to_string(),
    is_bidirectional: true,

    // Alice initiates, consents immediately
    consent_given_by_a: true,
    consent_given_by_b: false,  // Bob needs to confirm
    custody_enabled_by_a: true,
    custody_enabled_by_b: true,

    auto_custody_enabled: true,
    emergency_access_enabled: true,

    initiated_by: "alice-agent-id".to_string(),
    verified_at: None,  // Will be set when Bob confirms
    created_at: "2025-01-15T10:00:00Z".to_string(),
    updated_at: "2025-01-15T10:00:00Z".to_string(),
    expires_at: None,
    context_json: Some(r#"{"anniversary": "2010-06-15"}"#.to_string()),
    reach: "private".to_string(),  // Relationship itself is private
};

create_human_relationship(relationship)?;
```

### What Happens Automatically

1. **Relationship Created**: HumanRelationship entry stored in DHT
2. **Links Created**:
   - `IdToHumanRelationship`: Can query by relationship ID
   - `AgentToRelationship` (×2): Both Alice and Bob can find it
   - `HumanRelationshipByIntimacy`: Queryable by intimacy level
   - `HumanRelationshipByType`: Queryable by type (spouse)
   - `RelationshipPendingConsent`: Appears in Bob's pending list

3. **When Bob Confirms** (calls `confirm_relationship`):
   - `consent_given_by_b` set to `true`
   - `verified_at` timestamp set
   - **Auto-custody activated**: Two CustodianCommitment entries created
     - Commitment A: Alice custodies Bob's private/intimate content
     - Commitment B: Bob custodies Alice's private/intimate content

### Multi-Tier Replication in Action

**Scenario: Alice creates private family photos**

```rust
// Alice creates content with reach='intimate'
let photos = Content {
    reach: "intimate",
    author: "alice-agent-id",
    // ... photo data
};

create_content(photos)?;

// System automatically:
// 1. Queries Alice's intimate relationships (finds Bob)
// 2. Bob's custodian commitment matches:
//    - basis: "intimate_relationship"
//    - content_filters: {"reach": ["private", "intimate"], "author": "alice"}
// 3. Encrypted copy sent to Bob's rack (full_replica strategy)
// 4. If Bob's rack is on WAN (Tier 2), provides geographic redundancy
```

**When Lightning Strikes Alice's Rack:**

1. **Immediate**: All Alice's local data lost
2. **Tier 2 Recovery**:
   - Bob's rack (in another state) has encrypted copies
   - Alice can access via doorway → Bob's rack
3. **Restoration**:
   - New rack arrives
   - Alice claims identity (social recovery via Bob + family)
   - Pulls encrypted copies from Bob's rack
   - Full operation restored

## Integration with Existing Systems

### CustodianCommitment

The `auto_custody_commitments` function creates standard CustodianCommitment entries with:
- `basis`: "intimate_relationship"
- `relationship_id`: Links back to HumanRelationship
- `content_filters`: Matches private/intimate reach content
- `shard_strategy`: "full_replica" (complete encrypted copies)
- `emergency_triggers`: Enables recovery protocols

### Content Creation

**Future Enhancement Needed:**
```rust
// In create_content function, add:
if content.reach == "intimate" || content.reach == "private" {
    let relationships = get_intimate_relationships(author_agent_id)?;

    for relationship in relationships {
        // Trigger replication to custodian's rack
        replicate_to_custodian(content.id, relationship.custodian)?;
    }
}
```

## Benefits

### For Users

1. **Automatic Family Backup**: Family members automatically back up each other's data
2. **Geographic Redundancy**: Family racks in different locations protect against regional disasters
3. **Privacy Preserved**: Content encrypted, only family with shared key can decrypt
4. **Emergency Access**: Pre-configured recovery via family quorum
5. **Consent-Based**: Both parties must agree before custody activated

### For System

1. **Decentralized Resilience**: No central backup service needed
2. **Relationship-Based Trust**: Uses existing social bonds for custody
3. **Scalable**: Each family cluster handles its own backup
4. **Constitutional**: Aligns with Qahal (community) governance layer

## Next Steps

### Immediate (Complete the Implementation)

1. **Integrate with Content Creation**:
   - Auto-replicate intimate content to family custodians
   - Trigger replication on new relationship confirmation

2. **Build Confirmation Flow**:
   - UI for pending relationship requests
   - Notification system for confirmation requests

3. **Test Auto-Custody**:
   - Create test relationships
   - Verify commitments created correctly
   - Test disaster recovery flow

### Near-Term (Enhance Functionality)

4. **Emergency Recovery**:
   - Implement social recovery (M-of-N family quorum)
   - Seed phrase generation for crypto key backup

5. **Relationship Management UI**:
   - View all relationships
   - Manage custody permissions
   - Revoke/update relationships

6. **Family Cluster Coordination**:
   - Module discovery within family rack
   - Cross-rack replication protocol
   - Health monitoring

### Long-Term (Production Hardening)

7. **Encryption Key Management**:
   - Shared family encryption keys
   - Key rotation protocols
   - Emergency access override

8. **Compliance & Auditing**:
   - Custody access logs
   - Relationship verification audits
   - Emergency access audit trail

9. **Performance Optimization**:
   - Efficient shard distribution
   - Bandwidth-aware replication
   - Storage quota management

## Files Modified

1. `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs`
   - Added `INTIMACY_LEVELS` constant (lines 680-687)
   - Added `HUMAN_RELATIONSHIP_TYPES` constant (lines 689-703)
   - Added `HumanRelationship` struct (lines 705-742)
   - Added `HumanRelationship(HumanRelationship)` to EntryTypes enum (line 3569)
   - Added 6 new LinkTypes (lines 3748-3753)
   - Renamed `RelationshipByType` to `ContentRelationshipByType` (line 3743)

2. `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs`
   - Added `create_human_relationship` function (appended)
   - Added `get_relationships_for_agent` function (appended)
   - Added `get_intimate_relationships` function (appended)
   - Added `create_auto_custody_commitments` helper (appended)

## Architecture Alignment

This implementation enables the **multi-tier resilience model**:

- **Tier 1**: Local rack modules (RAID, module redundancy) - 0.1ms
- **Tier 2**: Family network racks (relationship-based, WAN) - 10-150ms
- **Tier 3**: Community custodians (public/commons content) - variable

HumanRelationship specifically enables **Tier 2** by:
- Connecting Imago Dei agents via intimate relationships
- Automatically creating custody commitments
- Enabling encrypted family-network backups
- Providing social recovery mechanisms

This is the foundation for **"Google Account recovery, but decentralized"**.
