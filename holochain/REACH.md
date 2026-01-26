# Reach: Data Sovereignty Through Proximity

## The Problem

Most platforms have two modes: public or private. This binary fails communities:

- **Public** means extractable - corporations harvest local data for global profit
- **Private** means isolated - communities can't share within appropriate boundaries

A neighborhood event shouldn't be visible to ad networks. Family photos shouldn't require choosing between "only me" and "everyone on Earth."

## The Solution: Reach Levels

Reach gates data by social and geographic proximity. Content flows outward from creator through concentric circles of trust:

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   private ─► invited ─► local ─► neighborhood ─► municipal     │
│      │                                              │           │
│      └──────────────────────────────────────────────┘           │
│                          │                                      │
│                          ▼                                      │
│            bioregional ─► regional ─► commons                   │
│                                                                 │
│   Most restrictive ───────────────────────► Least restrictive  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

| Level | Who Can Access | Examples |
|-------|----------------|----------|
| **private** | Only the owner | Medical records, personal journals, financial data |
| **invited** | Explicitly invited individuals | Shared documents, private group conversations |
| **local** | Household/family members | Chore schedules, family calendars, household decisions |
| **neighborhood** | Geographic community (block/street) | Block party planning, local safety alerts |
| **municipal** | City/town residents | City council meetings, municipal services |
| **bioregional** | Watershed/ecosystem | Environmental data, water management |
| **regional** | State/province | Regional news, state resources |
| **commons** | Everyone (global public) | Published knowledge, open resources |

## Why This Matters

### Information Colonization

Without reach, platforms extract local value for global profit:
- Neighborhood conversations become training data
- Local business reviews feed recommendation algorithms that favor chains
- Community knowledge gets aggregated, anonymized, and sold

Reach prevents this by default. Local content stays local unless explicitly promoted.

### Appropriate Boundaries

Different content has different natural audiences:
- A recipe shared with family shouldn't require Facebook's privacy settings
- A neighborhood watch alert shouldn't be globally searchable
- A city council recording should be accessible to residents, not the world's trolls

Reach provides these boundaries as primitives, not afterthoughts.

## How Reach Flows Through the System

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        REACH ENFORCEMENT                                 │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                         DNA (Source of Truth)                    │   │
│   │                                                                  │   │
│   │   Content entry includes reach field                            │   │
│   │   Validation rules can constrain reach changes                  │   │
│   │   Author sets initial reach, qahal can govern                   │   │
│   └──────────────────────────┬──────────────────────────────────────┘   │
│                              │                                           │
│              ┌───────────────┼───────────────┐                          │
│              ▼               ▼               ▼                          │
│   ┌──────────────────┐ ┌──────────────┐ ┌──────────────────────────┐   │
│   │     Doorway      │ │elohim-storage│ │    P2P Agent Apps        │   │
│   │                  │ │              │ │                          │   │
│   │ Gates HTTP/WS    │ │ Gates blob   │ │ Gates direct DHT         │   │
│   │ requests from    │ │ replication  │ │ access based on          │   │
│   │ web clients      │ │ to peers     │ │ agent relationships      │   │
│   └──────────────────┘ └──────────────┘ └──────────────────────────┘   │
│                                                                          │
│   Every component enforces the same reach rules.                        │
│   DNA is authoritative. Components are enforcement points.              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1. DNA: Source of Truth

The Holochain DNA defines reach as a field on content entries:

```rust
#[hdk_entry_helper]
pub struct Content {
    pub id: String,
    pub title: String,
    pub content: String,
    pub reach: String,        // "private", "local", "commons", etc.
    pub beneficiary: AgentPubKey,  // Who "owns" this content
}
```

Validation rules in the DNA can:
- Constrain which reach levels are valid
- Prevent reach from being changed after creation
- Require certain relationships for certain reach levels

### 2. Doorway: Web Gateway Enforcement

Doorway gates HTTP/WebSocket requests from browsers:

- **Anonymous requests**: Can only access `commons` content
- **Authenticated requests**: Can access content at their relationship level
- **Private content**: Only accessible to the beneficiary

See [doorway/REACH.md](./doorway/REACH.md) for implementation details.

### 3. elohim-storage: Storage & Replication Enforcement

elohim-storage gates the entire data plane:

- **Storage**: Private/invited/local content encrypted at rest
- **Replication**: Reach determines who receives shards
- **Delivery**: Reach gates who can request blobs
- **Eviction**: Protect private content, shed commons first

See [elohim-storage/REACH.md](./elohim-storage/REACH.md) for implementation details.

### 4. P2P Agent Apps: Direct Access Enforcement

Native apps connecting directly to Holochain:

- DNA validation runs on every node
- Invalid reach access attempts are rejected by the DHT itself
- No doorway needed - reach is cryptographically enforced

## Reach and Relationships

Reach interacts with the relationship system:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     REACH + RELATIONSHIPS                                │
│                                                                          │
│   Alice creates content with reach: "local"                             │
│                                                                          │
│   Who can access?                                                        │
│                                                                          │
│   ✓ Alice (creator/beneficiary)                                         │
│   ✓ Bob (Alice's household member - local relationship)                 │
│   ✓ Carol (Alice's spouse - local relationship)                         │
│   ✗ Dave (neighbor - neighborhood relationship, not local)              │
│   ✗ Eve (stranger - no relationship)                                    │
│                                                                          │
│   Relationships determine which reach levels you can access.            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Special Cases

### Custodian Override

Professional custodians (doctors, lawyers, emergency services) can access content beyond their normal reach level through explicit custody relationships:

```rust
CustodianCommitment {
    custodian: "dr-bob",
    beneficiary: "alice",
    category: "medical",
    approved_reach_levels: vec!["private"],  // Can access Alice's private medical data
}
```

### Emergency Escalation

During emergencies, reach rules can temporarily expand:
- Fire department can access building layouts (normally private)
- Medical responders can access health records (normally private)
- This is logged and auditable

### Reach Promotion

Content can be promoted to broader reach:
- A family recipe (local) can be published (commons)
- A neighborhood alert can be escalated to municipal
- Original reach is preserved for audit trail

## Implementation Status

| Component | Reach Enforcement | Status |
|-----------|-------------------|--------|
| DNA (content_store) | Entry validation | ✅ Implemented |
| Doorway | HTTP/WS gating | ✅ Implemented |
| elohim-storage | Reach field tracking | ✅ Implemented |
| elohim-storage | Delivery gating | 🔄 Partial (sovereignty only) |
| elohim-storage | Replication gating | 🔄 Partial (sovereignty only) |
| elohim-storage | Encryption at rest | ❌ Not started |
| elohim-storage | LRU by reach priority | ❌ Not started |
| elohim-storage | RS shard distribution | ❌ Not started |
| Agent apps | DHT validation | ✅ Inherent |
| Geographic verification | Location-based reach | 📋 Planned |

## Related Documentation

- [doorway/REACH.md](./doorway/REACH.md) - Doorway's role in reach enforcement
- [elohim-storage/REACH.md](./elohim-storage/REACH.md) - Storage/replication reach enforcement
- [dna/elohim/README.md](./dna/elohim/README.md) - DNA entry types including reach
- [elohim-storage/P2P-ARCHITECTURE.md](./elohim-storage/P2P-ARCHITECTURE.md) - Dual-plane architecture
