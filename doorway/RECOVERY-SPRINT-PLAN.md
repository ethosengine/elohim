# Recovery Protocol Sprint Plan

**Reference**: [RECOVERY-PROTOCOL.md](./RECOVERY-PROTOCOL.md)

## Sprint Overview

| Phase | Weeks | Goal | Status |
|-------|-------|------|--------|
| 1. Shard Tracking | 1-2 | Know where shards are | Not Started |
| 2. Recovery Request Flow | 3-4 | Initiate recovery, collect authorizations | Not Started |
| 3. Reconstruction Coordinator | 5-6 | Doorway orchestrates reconstruction | Not Started |
| 4. Work-While-Recovering | 7-8 | Use content before full recovery | Not Started |
| 5. Verification & Drills | 9-10 | Prove it works before crisis | Not Started |

---

## Phase 1: Shard Tracking (Weeks 1-2)

**Goal**: Track which custodian holds which shard of which content in the DHT.

### Why First
Everything else depends on knowing where shards are. Currently Reed-Solomon sharding happens in elohim-storage but the DHT doesn't know about shard placement.

### Tasks

#### 1.1 Add ShardAssignment to node-registry DNA
- [ ] Add `ShardAssignment` entry type to `node_registry_integrity/src/lib.rs`
- [ ] Add validation rules (shard_index 0-6, valid content_hash, valid custodian_did)
- [ ] Add `ShardStatus` enum: Active, Stale, Failed, Migrating, Reconstructing
- [ ] Add `ShardingStrategy` enum: Geographic, TrustTier, FamilyCluster, Manual

#### 1.2 Add link types for shard discovery
- [ ] `ContentToShardAssignment`: Anchor(content_hash) → ShardAssignment
- [ ] `CustodianToShardAssignment`: Anchor(custodian_did) → ShardAssignment
- [ ] `ShardIndexToAssignment`: Anchor(content_hash:shard_index) → ShardAssignment

#### 1.3 Add coordinator functions
- [ ] `create_shard_assignment(input)` - Register a shard assignment
- [ ] `get_shard_assignments_for_content(content_hash)` - Find all shards for content
- [ ] `get_shard_assignments_for_custodian(custodian_did)` - Find all shards a custodian holds
- [ ] `update_shard_status(assignment_hash, new_status)` - Mark shard status
- [ ] `update_shard_verified_at(assignment_hash)` - Touch verification timestamp

#### 1.4 Wire up elohim-storage
- [ ] After Reed-Solomon encoding, call node-registry to create ShardAssignments
- [ ] Include custodian_did (our DID) and shard_index for each shard
- [ ] Emit signal for projection

#### 1.5 Add post-commit signals
- [ ] `ShardAssignmentCommitted` signal for projection to MongoDB
- [ ] Include content_hash, shard_index, custodian_did, status

### Definition of Done
- [ ] Can query "which doorways hold shards for content X"
- [ ] Can query "which content shards does doorway Y hold"
- [ ] elohim-storage automatically registers shard assignments
- [ ] Shefa dashboard can show shard distribution (via projection)

---

## Phase 2: Recovery Request Flow (Weeks 3-4)

**Goal**: Human can initiate recovery from new device and collect authorizations from trusted relationships.

### Tasks

#### 2.1 Add entry types to imagodei DNA
- [ ] `RecoveryRequest` - Initial recovery request
- [ ] `RecoveryChallenge` - Challenge issued by relationship/doorway
- [ ] `RecoveryAuthorization` - Grant issued by relationship/doorway
- [ ] `RecoveryMethod` enum: SocialRecovery, EmergencyContact, DoorwayAttestation
- [ ] `RecoveryScope` enum: Full, Selective, ByReach
- [ ] `RecoveryStatus` enum: Pending, Challenged, Authorized, Denied, Completed

#### 2.2 Add recovery link types
- [ ] `HumanToRecoveryRequest`
- [ ] `RequestToChallenge`
- [ ] `RequestToAuthorization`
- [ ] `EmergencyContactsForHuman`

#### 2.3 Add coordinator functions
- [ ] `create_recovery_request(human_did, new_agent_pubkey, method, scope)`
- [ ] `get_emergency_contacts(human_did)` - Find relationships with emergency_access_enabled
- [ ] `issue_challenge(request_id, challenge_type)`
- [ ] `respond_to_challenge(challenge_id, response)`
- [ ] `grant_authorization(request_id, scope)`
- [ ] `check_authorization_threshold(request_id)` - Have we collected enough?

#### 2.4 Add recovery UI
- [ ] "Recover my identity" button on doorway-picker
- [ ] Recovery request form (enter DID or sign in with trusted doorway)
- [ ] Challenge response UI (answer question, enter code, video call link)
- [ ] Authorization request notification for trusted relationships

### Definition of Done
- [ ] Human can initiate recovery from browser on new device
- [ ] Trusted relationships receive notification and can authorize
- [ ] N of M authorization threshold is enforced
- [ ] Recovery status visible in UI

---

## Phase 3: Reconstruction Coordinator (Weeks 5-6)

**Goal**: Doorway orchestrates fetching shards from custodians and reconstructs content.

### Tasks

#### 3.1 Implement RecoverySession in doorway
- [ ] `RecoverySession` struct (not DHT, coordinator state)
- [ ] Session lifecycle: Created → Fetching → Reconstructing → Complete/Failed
- [ ] Progress tracking per content item

#### 3.2 Wire up NATS for recovery signals
- [ ] Subscribe to `SIGNAL.imagodei.recovery_authorized`
- [ ] Publish progress to `RECOVERY.progress.{session_id}`

#### 3.3 Implement shard fetching
- [ ] Query ShardAssignments for authorized content
- [ ] Resolve custodian DIDs to service endpoints
- [ ] Parallel fetch with configurable limit (default 4)
- [ ] Retry logic with exponential backoff
- [ ] Timeout handling (default 30s per shard)

#### 3.4 Integrate Reed-Solomon reconstruction
- [ ] Use elohim-storage RS decoder
- [ ] Verify reconstructed content matches content_hash
- [ ] Store reconstructed blob in new agent's elohim-storage

#### 3.5 Add API endpoints
- [ ] `GET /api/v1/recovery/{session_id}/status` - Current progress
- [ ] `GET /api/v1/recovery/{session_id}/content` - List content being recovered
- [ ] WebSocket endpoint for real-time progress

### Definition of Done
- [ ] Authorized recovery triggers automatic shard fetching
- [ ] Content reconstructed from 4+ shards
- [ ] Progress visible in real-time
- [ ] Reconstructed content stored locally on new device

---

## Phase 4: Work-While-Recovering (Weeks 7-8)

**Goal**: User can access content before full recovery completes.

### Tasks

#### 4.1 Modify ContentResolver
- [ ] Check if content is in active RecoverySession
- [ ] If yes, prioritize in reconstruction queue
- [ ] Return content once reconstructed (don't wait for full recovery)

#### 4.2 Add priority queue
- [ ] User-requested content gets High priority
- [ ] Background recovery gets Normal priority
- [ ] Batch operations get Bulk priority

#### 4.3 Implement on-demand reconstruction
- [ ] Fetch 4 shards for specific content on request
- [ ] Reconstruct inline
- [ ] Cache locally while returning to user

#### 4.4 Add UI indicators
- [ ] Show recovery progress per content item
- [ ] Indicate "fetching from network" state
- [ ] Show "locally cached" vs "available from shards" state

### Definition of Done
- [ ] User can request specific content during recovery
- [ ] Requested content is prioritized and delivered
- [ ] User can create new content immediately (not blocked by recovery)
- [ ] UI shows clear status of what's local vs recovering

---

## Phase 5: Verification & Drills (Weeks 9-10)

**Goal**: Prove the system works before a real crisis.

### Tasks

#### 5.1 Periodic shard verification
- [ ] Daily job to verify custodied shards
- [ ] Update `verified_at` timestamp on successful check
- [ ] Mark as `Stale` if verification fails
- [ ] Trigger re-replication for stale shards

#### 5.2 Distribution health analysis
- [ ] Calculate healthy/degraded/critical counts
- [ ] Geo distribution score
- [ ] Trust tier distribution score
- [ ] Generate recommendations

#### 5.3 Recovery drills
- [ ] `recovery_drill(human_did)` function
- [ ] Check emergency contact reachability
- [ ] Test sample blob reconstruction
- [ ] Return readiness score

#### 5.4 Alerts and dashboard
- [ ] Alert if content drops below minimum shard count
- [ ] Shefa dashboard widget for network recovery health
- [ ] Per-human recovery readiness indicator
- [ ] Drill reminder notifications

### Definition of Done
- [ ] Doorways verify their shards daily
- [ ] Degraded distributions trigger alerts
- [ ] Humans can run recovery drills from profile
- [ ] Shefa dashboard shows network-wide recovery health

---

## Quick Start Tomorrow

1. Open `holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs`
2. Add the `ShardAssignment` entry type from RECOVERY-PROTOCOL.md
3. Run `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check` to verify it compiles
4. Continue with Phase 1 tasks

Good night.
