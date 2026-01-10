# Recovery Protocol

## Overview

This document describes the human-scale identity and content recovery protocol for Elohim. Unlike traditional backup systems that recover *data*, this protocol recovers *agency* - the ability to continue being yourself after device loss.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        RECOVERY SCENARIOS                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Phone stolen        → Recover from family cluster + trusted doorways     │
│   Laptop dies         → Shards distributed, reconstruct from any 4 of 7    │
│   House fire          → Geographic distribution protects against local loss│
│   Family member dies  → Their knowledge transfers to designated heirs      │
│   Community displaced → Doorways in other regions hold the shards          │
│                                                                             │
│   The common thread: your identity persists, your content reconstructs,    │
│   and you can continue working while restoration completes.                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Design Principles

1. **Human failure modes, not just hardware**: People lose phones, have houses flood, die. Infrastructure must handle human-scale disasters.

2. **Recovery while rebuilding**: You shouldn't wait for full restoration to start working. Access content from shards while your local cache rebuilds.

3. **Social recovery over seed phrases**: Your trusted relationships are your recovery keys. No 24-word phrases to lose.

4. **Trust tiers matter most in crisis**: Anchor doorways are most valuable when everything else fails.

5. **Geographic distribution by default**: Family clusters should span regions. Intimate relationships should enable cross-region custody.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         RECOVERY LAYERS                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Layer 4: HUMAN IDENTITY (imagodei DNA)                             │  │
│   │                                                                     │  │
│   │  • HumanRelationship.emergency_access_enabled                       │  │
│   │  • Social recovery: intimate relationships can vouch for you        │  │
│   │  • RecoveryRequest → RecoveryChallenge → RecoveryAuthorization      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    ▼                                        │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Layer 3: SHARD DISTRIBUTION (node-registry DNA)                    │  │
│   │                                                                     │  │
│   │  • ShardAssignment: which custodian holds which shard               │  │
│   │  • CustodianAssignment: who is responsible for what content         │  │
│   │  • Geographic distribution policies                                 │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    ▼                                        │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Layer 2: DOORWAY FEDERATION (infrastructure DNA)                   │  │
│   │                                                                     │  │
│   │  • DoorwayRegistration with trust tiers                             │  │
│   │  • ContentServer for shard serving                                  │  │
│   │  • DID resolution for cross-doorway fetch                           │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    ▼                                        │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  Layer 1: BLOB STORAGE (elohim-storage)                             │  │
│   │                                                                     │  │
│   │  • Reed-Solomon 4+3 erasure coding                                  │  │
│   │  • ShardManifest per blob                                           │  │
│   │  • Reconstruction from any 4 shards                                 │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Entry Types (New)

### RecoveryRequest (imagodei DNA)

Initiated when a human needs to recover their identity/content from a new device.

```rust
#[hdk_entry_helper]
pub struct RecoveryRequest {
    /// DID of the human requesting recovery
    pub human_did: String,

    /// New agent pubkey (the new device)
    pub new_agent_pubkey: String,

    /// Recovery method being used
    pub method: RecoveryMethod,

    /// Doorway(s) the request was submitted through
    pub submitted_via_doorways: Vec<String>,

    /// Status: pending, challenged, authorized, denied, completed
    pub status: RecoveryStatus,

    /// Content scope: all, or specific content hashes
    pub scope: RecoveryScope,

    /// Requested at timestamp
    pub requested_at: Timestamp,

    /// Expires at (recovery requests should timeout)
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryMethod {
    /// Social recovery: N of M intimate relationships must authorize
    SocialRecovery {
        required_authorizations: u8,
        contacted_relationships: Vec<String>,
    },

    /// Single trusted relationship with emergency access
    EmergencyContact {
        relationship_id: String,
    },

    /// Doorway-mediated: trusted doorway vouches based on prior auth
    DoorwayAttestation {
        attesting_doorway_did: String,
        prior_auth_proof: String,
    },

    /// Hardware key recovery (future: WebAuthn resident keys)
    HardwareKey {
        credential_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryScope {
    /// Recover everything
    Full,

    /// Recover specific content only
    Selective {
        content_hashes: Vec<String>,
    },

    /// Recover content by reach level
    ByReach {
        max_reach: String, // e.g., "private" won't recover "intimate"
    },
}
```

### RecoveryChallenge (imagodei DNA)

Challenge issued to verify the recovery request is legitimate.

```rust
#[hdk_entry_helper]
pub struct RecoveryChallenge {
    /// Links to the RecoveryRequest
    pub request_id: ActionHash,

    /// Who is issuing this challenge
    pub challenger: RecoveryChallenger,

    /// The challenge itself
    pub challenge_type: ChallengeType,

    /// Challenge data (encrypted to new_agent_pubkey)
    pub challenge_data: String,

    /// Response from the recovering party (once provided)
    pub response: Option<String>,

    /// Whether the challenge was passed
    pub passed: Option<bool>,

    /// Issued at
    pub issued_at: Timestamp,

    /// Expires at
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryChallenger {
    /// A human relationship (social recovery)
    Relationship {
        relationship_id: ActionHash,
        human_did: String,
    },

    /// A trusted doorway
    Doorway {
        doorway_id: String,
        doorway_did: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeType {
    /// Answer a personal question only the human would know
    PersonalQuestion {
        question_hash: String, // Hash of pre-registered question
    },

    /// Provide a code sent to a verified contact method
    OutOfBandCode {
        delivery_method: String, // "email", "phone", "signal"
    },

    /// Real-time video call verification
    VideoVerification {
        call_link: String,
    },

    /// Cryptographic proof from hardware key
    HardwareKeyProof {
        credential_id: String,
    },
}
```

### RecoveryAuthorization (imagodei DNA)

Grants issued by relationships/doorways to authorize shard access.

```rust
#[hdk_entry_helper]
pub struct RecoveryAuthorization {
    /// Links to the RecoveryRequest
    pub request_id: ActionHash,

    /// Who is granting this authorization
    pub grantor: RecoveryGrantor,

    /// What content this authorization covers
    pub scope: AuthorizationScope,

    /// Cryptographic capability grant
    pub capability_grant: String,

    /// Time-limited access
    pub valid_until: Timestamp,

    /// Granted at
    pub granted_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryGrantor {
    /// A human in a trusted relationship
    Human {
        human_did: String,
        relationship_id: ActionHash,
    },

    /// A doorway that previously authenticated this human
    Doorway {
        doorway_did: String,
        tier: String,
    },

    /// The human themselves (from prior self-authorization)
    SelfAuthorization {
        prior_agent_pubkey: String,
        proof: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationScope {
    /// Full access to all content
    Full,

    /// Access to specific content hashes
    Specific {
        content_hashes: Vec<String>,
    },

    /// Access to shards held by this grantor
    CustodiedContent,
}
```

### ShardAssignment (node-registry DNA)

Tracks which custodian holds which shard of which content.

```rust
#[hdk_entry_helper]
pub struct ShardAssignment {
    /// Content hash (the original blob)
    pub content_hash: String,

    /// Shard index (0-6 for 4+3 Reed-Solomon)
    pub shard_index: u8,

    /// Shard hash (for verification)
    pub shard_hash: String,

    /// Custodian DID (doorway or storage node)
    pub custodian_did: String,

    /// Custodian's agent pubkey in the network
    pub custodian_agent: String,

    /// Assignment strategy that created this
    pub strategy: ShardingStrategy,

    /// When this shard was assigned
    pub assigned_at: Timestamp,

    /// Last verification that custodian still has shard
    pub verified_at: Option<Timestamp>,

    /// Status: active, stale, failed, migrating
    pub status: ShardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardingStrategy {
    /// Geographic distribution
    Geographic {
        target_regions: Vec<String>,
    },

    /// Trust-tier distribution
    TrustTier {
        minimum_tier: String,
    },

    /// Family cluster distribution
    FamilyCluster {
        relationship_ids: Vec<ActionHash>,
    },

    /// Explicit assignment by content owner
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardStatus {
    Active,           // Custodian confirmed possession
    Stale,            // Haven't verified recently
    Failed,           // Custodian couldn't provide shard
    Migrating,        // Being moved to new custodian
    Reconstructing,   // Being reconstructed from other shards
}
```

### RecoverySession (coordinator - not DHT persisted)

Active recovery session managed by doorway coordinator.

```rust
pub struct RecoverySession {
    /// Session ID
    pub id: String,

    /// The recovery request being processed
    pub request_id: ActionHash,

    /// Human being recovered
    pub human_did: String,

    /// New agent receiving the recovery
    pub new_agent_pubkey: String,

    /// Authorizations collected so far
    pub authorizations: Vec<RecoveryAuthorization>,

    /// Content being reconstructed
    pub content_reconstructions: Vec<ContentReconstruction>,

    /// Overall status
    pub status: SessionStatus,

    /// Progress percentage (0-100)
    pub progress_percent: f32,

    /// Started at
    pub started_at: Timestamp,

    /// Estimated completion
    pub eta: Option<Timestamp>,
}

pub struct ContentReconstruction {
    pub content_hash: String,
    pub total_shards: u8,         // 7 for 4+3 RS
    pub required_shards: u8,      // 4 for 4+3 RS
    pub retrieved_shards: Vec<u8>, // Which shard indices we have
    pub failed_shards: Vec<u8>,   // Which we couldn't retrieve
    pub status: ReconstructionStatus,
}

pub enum ReconstructionStatus {
    Pending,
    Fetching { current_shard: u8 },
    Sufficient,      // Have enough to reconstruct
    Reconstructing,
    Complete,
    Failed { reason: String },
}
```

## Recovery Flow

### Phase 1: Request Initiation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECOVERY REQUEST INITIATION                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Human loses device                                                        │
│       │                                                                     │
│       ▼                                                                     │
│   Opens browser on new device                                               │
│       │                                                                     │
│       ▼                                                                     │
│   Navigates to trusted doorway (or any doorway)                            │
│       │                                                                     │
│       ▼                                                                     │
│   Clicks "Recover my identity"                                             │
│       │                                                                     │
│       ▼                                                                     │
│   ┌─────────────────────────────────────────────────────────┐              │
│   │  Recovery UI                                            │              │
│   │                                                         │              │
│   │  "Enter your human identifier or DID"                   │              │
│   │  [________________________] [Search]                    │              │
│   │                                                         │              │
│   │  "Or sign in with a trusted doorway you've used before" │              │
│   │  [Google] [GitHub] [Passkey]                            │              │
│   │                                                         │              │
│   └─────────────────────────────────────────────────────────┘              │
│       │                                                                     │
│       ▼                                                                     │
│   Doorway creates RecoveryRequest in DHT                                   │
│       │                                                                     │
│       ▼                                                                     │
│   Request propagates to custodians who hold this human's shards            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 2: Social Recovery Challenge

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SOCIAL RECOVERY CHALLENGE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   RecoveryRequest with SocialRecovery method                               │
│       │                                                                     │
│       ▼                                                                     │
│   DHT query: find HumanRelationships with emergency_access_enabled         │
│       │                                                                     │
│       ▼                                                                     │
│   For each intimate/trusted relationship:                                  │
│       │                                                                     │
│       ├──► Notify via Signal/Email: "Alice is trying to recover"           │
│       │                                                                     │
│       ├──► Trusted human opens their Elohim app                            │
│       │        │                                                            │
│       │        ▼                                                            │
│       │    ┌─────────────────────────────────────────────────┐             │
│       │    │  Recovery Authorization Request                 │             │
│       │    │                                                 │             │
│       │    │  Alice (alice@...) is trying to recover their  │             │
│       │    │  identity from a new device.                   │             │
│       │    │                                                 │             │
│       │    │  Device: Chrome on MacBook (San Francisco)     │             │
│       │    │  Requested: 2 minutes ago                      │             │
│       │    │                                                 │             │
│       │    │  Do you recognize this recovery attempt?       │             │
│       │    │                                                 │             │
│       │    │  [Authorize] [Deny] [Call to verify]           │             │
│       │    │                                                 │             │
│       │    └─────────────────────────────────────────────────┘             │
│       │        │                                                            │
│       │        ▼                                                            │
│       └──► Create RecoveryAuthorization in DHT                             │
│                                                                             │
│   When N of M authorizations collected:                                    │
│       │                                                                     │
│       ▼                                                                     │
│   RecoveryRequest.status = authorized                                      │
│       │                                                                     │
│       ▼                                                                     │
│   Begin shard reconstruction                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 3: Shard Reconstruction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SHARD RECONSTRUCTION                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   RecoverySession created by coordinating doorway                          │
│       │                                                                     │
│       ▼                                                                     │
│   Query ShardAssignments for authorized content                            │
│       │                                                                     │
│       ▼                                                                     │
│   For each content_hash:                                                   │
│       │                                                                     │
│       ├──► Get all ShardAssignments (expect 7 for 4+3 RS)                  │
│       │                                                                     │
│       ├──► Sort custodians by: trust tier, latency, region                 │
│       │                                                                     │
│       ├──► Fetch shards in parallel until we have 4:                       │
│       │        │                                                            │
│       │        ├──► resolve custodian DID → service endpoint               │
│       │        ├──► GET /api/v1/shards/{content_hash}/{shard_index}        │
│       │        ├──► Verify shard_hash                                      │
│       │        └──► Store in session                                       │
│       │                                                                     │
│       ├──► Once 4+ shards retrieved:                                       │
│       │        │                                                            │
│       │        ▼                                                            │
│       │    Reed-Solomon decode → reconstruct original blob                 │
│       │        │                                                            │
│       │        ▼                                                            │
│       │    Verify content_hash matches                                     │
│       │        │                                                            │
│       │        ▼                                                            │
│       │    Store blob in new agent's elohim-storage                        │
│       │                                                                     │
│       └──► Emit progress update to UI                                      │
│                                                                             │
│   When all authorized content reconstructed:                               │
│       │                                                                     │
│       ▼                                                                     │
│   RecoveryRequest.status = completed                                       │
│       │                                                                     │
│       ▼                                                                     │
│   New agent is now functional, begins redistributing shards                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 4: Work While Recovering

Critical UX principle: **Don't wait for full recovery to start working.**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    WORK WHILE RECOVERING                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Recovery in progress (40% complete)                                      │
│       │                                                                     │
│       ▼                                                                     │
│   User requests content not yet reconstructed locally                      │
│       │                                                                     │
│       ▼                                                                     │
│   ContentResolver: Local? NO → Projection? MAYBE → Authority? YES          │
│       │                                                                     │
│       ▼                                                                     │
│   ┌─────────────────────────────────────────────────────────┐              │
│   │  Transparent shard fetch:                               │              │
│   │                                                         │              │
│   │  1. Check if content is in active RecoverySession       │              │
│   │  2. If yes, prioritize this content in queue            │              │
│   │  3. Fetch 4 shards from custodians                      │              │
│   │  4. Reconstruct on-the-fly                              │              │
│   │  5. Return to user while caching locally                │              │
│   │                                                         │              │
│   │  User sees: slight delay, then content appears          │              │
│   └─────────────────────────────────────────────────────────┘              │
│       │                                                                     │
│       ▼                                                                     │
│   User can create NEW content immediately                                  │
│       │                                                                     │
│       ▼                                                                     │
│   New content is sharded and distributed normally                          │
│   (new agent is a full participant from moment of authorization)           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Verification and Drills

### Periodic Shard Verification

```rust
// Run daily per doorway
async fn verify_custodied_shards(state: &AppState) -> Result<VerificationReport> {
    let my_assignments = get_my_shard_assignments().await?;

    let mut report = VerificationReport::new();

    for assignment in my_assignments {
        match verify_shard_integrity(&assignment).await {
            Ok(()) => {
                report.verified += 1;
                update_shard_verified_at(&assignment).await?;
            }
            Err(e) => {
                report.failed += 1;
                report.failures.push(ShardFailure {
                    content_hash: assignment.content_hash,
                    shard_index: assignment.shard_index,
                    error: e.to_string(),
                });

                // Mark as failed, trigger re-replication
                mark_shard_failed(&assignment).await?;
                emit_signal(Signal::ShardFailed(assignment)).await?;
            }
        }
    }

    Ok(report)
}
```

### Recovery Drills

```rust
// Allow humans to test their recovery setup
async fn recovery_drill(human_did: &str) -> Result<DrillReport> {
    // 1. Simulate recovery request
    let request = create_drill_recovery_request(human_did).await?;

    // 2. Check that emergency contacts are reachable
    let contacts = get_emergency_contacts(human_did).await?;
    let reachable = check_contact_reachability(&contacts).await?;

    // 3. Verify shard distribution
    let shards = get_shard_distribution(human_did).await?;
    let distribution_health = analyze_distribution(&shards)?;

    // 4. Test reconstruction of a sample blob
    let sample_blob = pick_sample_blob(human_did).await?;
    let reconstruction_test = test_reconstruction(&sample_blob).await?;

    // 5. Clean up drill artifacts
    cleanup_drill(&request).await?;

    Ok(DrillReport {
        emergency_contacts: reachable,
        shard_distribution: distribution_health,
        reconstruction: reconstruction_test,
        overall_readiness: calculate_readiness(&reachable, &distribution_health, &reconstruction_test),
    })
}
```

### Distribution Health Checks

```rust
pub struct DistributionHealth {
    /// Content hashes with healthy distribution
    pub healthy: u32,

    /// Content hashes below minimum shard count
    pub degraded: u32,

    /// Content hashes that would fail recovery
    pub critical: u32,

    /// Geographic distribution score (0-100)
    pub geo_distribution_score: f32,

    /// Trust tier distribution score (0-100)
    pub trust_distribution_score: f32,

    /// Recommendations
    pub recommendations: Vec<String>,
}

fn analyze_distribution(shards: &[ShardAssignment]) -> Result<DistributionHealth> {
    // Group by content_hash
    let by_content: HashMap<String, Vec<&ShardAssignment>> = /* ... */;

    let mut health = DistributionHealth::default();

    for (content_hash, assignments) in by_content {
        let active_count = assignments.iter()
            .filter(|a| a.status == ShardStatus::Active)
            .count();

        if active_count >= 7 {
            health.healthy += 1;
        } else if active_count >= 4 {
            health.degraded += 1;
            health.recommendations.push(
                format!("Content {} has only {} active shards, consider re-replication",
                    content_hash, active_count)
            );
        } else {
            health.critical += 1;
            health.recommendations.push(
                format!("CRITICAL: Content {} has only {} shards, recovery may fail",
                    content_hash, active_count)
            );
        }

        // Check geographic distribution
        let regions: HashSet<_> = assignments.iter()
            .filter_map(|a| a.strategy.target_region())
            .collect();

        if regions.len() < 2 {
            health.recommendations.push(
                format!("Content {} is concentrated in {} region(s), consider geographic distribution",
                    content_hash, regions.len())
            );
        }
    }

    health.geo_distribution_score = calculate_geo_score(&by_content);
    health.trust_distribution_score = calculate_trust_score(&by_content);

    Ok(health)
}
```

## Link Types (New)

### imagodei DNA

```rust
#[hdk_link_types]
pub enum LinkTypes {
    // ... existing ...

    // Recovery links
    HumanToRecoveryRequest,      // Human -> their recovery requests
    RequestToChallenge,          // RecoveryRequest -> RecoveryChallenges
    RequestToAuthorization,      // RecoveryRequest -> RecoveryAuthorizations
    RelationshipToAuthorization, // HumanRelationship -> authorizations granted

    // Emergency access discovery
    EmergencyContactsForHuman,   // Human -> relationships with emergency_access_enabled
}
```

### node-registry DNA

```rust
#[hdk_link_types]
pub enum LinkTypes {
    // ... existing ...

    // Shard tracking
    ContentToShardAssignment,    // Anchor(content_hash) -> ShardAssignment
    CustodianToShardAssignment,  // Anchor(custodian_did) -> ShardAssignment
    ShardIndexToAssignment,      // Anchor(content_hash:shard_index) -> ShardAssignment
}
```

## Signals

### Recovery Progress Signals

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum RecoverySignal {
    /// Recovery request created
    RecoveryRequested {
        request_id: ActionHash,
        human_did: String,
        method: RecoveryMethod,
    },

    /// Challenge issued to a relationship
    ChallengeIssued {
        request_id: ActionHash,
        challenger: RecoveryChallenger,
    },

    /// Authorization granted
    AuthorizationGranted {
        request_id: ActionHash,
        grantor: RecoveryGrantor,
        authorizations_collected: u8,
        authorizations_required: u8,
    },

    /// Recovery authorized, reconstruction starting
    RecoveryAuthorized {
        request_id: ActionHash,
        human_did: String,
        new_agent_pubkey: String,
    },

    /// Reconstruction progress
    ReconstructionProgress {
        request_id: ActionHash,
        content_hash: String,
        shards_retrieved: u8,
        shards_required: u8,
        status: ReconstructionStatus,
    },

    /// Recovery complete
    RecoveryComplete {
        request_id: ActionHash,
        human_did: String,
        content_recovered: u32,
        duration_seconds: u64,
    },

    /// Recovery failed
    RecoveryFailed {
        request_id: ActionHash,
        reason: String,
    },
}
```

## Implementation Phases

### Phase 1: Shard Tracking (Week 1-2)

**Goal**: Know where shards are before we need to recover them.

- [ ] Add `ShardAssignment` entry type to node-registry DNA
- [ ] Add link types for shard discovery
- [ ] Modify elohim-storage to register ShardAssignments when storing blobs
- [ ] Add `get_shard_assignments_for_content(content_hash)` function
- [ ] Add `get_shard_assignments_for_custodian(custodian_did)` function
- [ ] Wire up post-commit signals for shard tracking

### Phase 2: Recovery Request Flow (Week 3-4)

**Goal**: Human can initiate recovery and collect authorizations.

- [ ] Add `RecoveryRequest` entry type to imagodei DNA
- [ ] Add `RecoveryChallenge` entry type
- [ ] Add `RecoveryAuthorization` entry type
- [ ] Add recovery link types
- [ ] Implement `create_recovery_request()` function
- [ ] Implement `respond_to_challenge()` function
- [ ] Implement `grant_authorization()` function
- [ ] Add recovery UI to doorway-picker component

### Phase 3: Reconstruction Coordinator (Week 5-6)

**Goal**: Doorway can orchestrate shard fetching and reconstruction.

- [ ] Implement `RecoverySession` management in doorway
- [ ] Wire up NATS integration for recovery signals
- [ ] Implement shard fetching from custodians
- [ ] Integrate Reed-Solomon reconstruction
- [ ] Implement progress tracking for Shefa dashboard
- [ ] Add recovery status endpoint to doorway API

### Phase 4: Work-While-Recovering (Week 7-8)

**Goal**: User can access content before full recovery completes.

- [ ] Modify ContentResolver to check active RecoverySession
- [ ] Implement on-demand shard fetching for requested content
- [ ] Add priority queue for user-requested content
- [ ] Implement transparent caching of reconstructed content
- [ ] Add UI indicators for recovery status per content item

### Phase 5: Verification and Drills (Week 9-10)

**Goal**: Prove the system works before crisis.

- [ ] Implement periodic shard verification
- [ ] Add distribution health analysis
- [ ] Implement recovery drill functionality
- [ ] Add recovery readiness score to human profile
- [ ] Create alerts for degraded shard distributions
- [ ] Add Shefa dashboard widgets for network recovery health

## Configuration

### Doorway Recovery Settings

```toml
[recovery]
# Minimum authorizations required for social recovery
min_social_authorizations = 2

# Maximum time for recovery request to remain valid
request_expiry_hours = 72

# How long to wait for each shard fetch
shard_fetch_timeout_seconds = 30

# Parallel shard fetch limit
max_parallel_shard_fetches = 4

# Reconstruction retry attempts
reconstruction_retries = 3

[recovery.verification]
# How often to verify custodied shards
verification_interval_hours = 24

# Maximum shard age before marked stale
max_shard_age_days = 7

# Enable recovery drills
drills_enabled = true

# Drill notification frequency
drill_reminder_days = 30
```

### Human Recovery Settings (in profile)

```rust
pub struct RecoverySettings {
    /// Minimum authorizations for social recovery
    pub min_authorizations: u8,

    /// Preferred recovery method
    pub preferred_method: RecoveryMethod,

    /// Relationships enabled for emergency access
    pub emergency_contacts: Vec<ActionHash>,

    /// Trusted doorways for attestation-based recovery
    pub trusted_doorways: Vec<String>,

    /// Geographic distribution preference for shards
    pub shard_distribution: ShardDistributionPreference,

    /// Last recovery drill timestamp
    pub last_drill: Option<Timestamp>,
}

pub enum ShardDistributionPreference {
    /// Distribute across at least N regions
    MinRegions(u8),

    /// Prefer family cluster members
    FamilyFirst,

    /// Trust tier priority
    TrustTierPriority { minimum_tier: String },

    /// Custom distribution
    Custom { preferences: Vec<CustodianPreference> },
}
```

## Security Considerations

### Attack Vectors

| Attack | Mitigation |
|--------|------------|
| Impersonation (claim to be someone else) | Social recovery requires N of M trusted relationships |
| Sybil (create fake relationships for recovery) | Relationships require bilateral consent, built over time |
| Coerced recovery (force someone to authorize) | Rate limiting, out-of-band verification, video calls |
| Doorway collusion | Require multiple doorway attestations, prefer Anchor tier |
| Shard theft (steal encrypted shards) | Shards are encrypted to content owner's keys |
| Recovery denial (refuse to authorize legitimate request) | Multiple recovery paths, configurable thresholds |

### Privacy Considerations

- Recovery requests don't reveal what content exists (only that human is recovering)
- Shard assignments don't reveal content (only hashes and custody)
- Reconstruction happens on recovering device, not on doorways
- Emergency contacts are not publicly visible (only to the human)

## References

- [DID Federation](./DID-FEDERATION.md) - Content location via DIDs
- [Edge Architecture](../EDGE-ARCHITECTURE.md) - P2P performance at agent level
- [P2P Architecture](../elohim-storage/P2P-ARCHITECTURE.md) - Shard distribution
- [Node Registry DNA](../dna/node-registry/README.md) - Custodian assignments
- [Imagodei DNA](../dna/imagodei/) - Human relationships and emergency access
