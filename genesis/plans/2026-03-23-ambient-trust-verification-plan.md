# Ambient Trust Verification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the gap between per-request SQLite auth and the vision's per-connection DHT-verified ambient trust model, in three layers: A (ship now), B (verification zome + storage module), C (connection handshake + cache).

**Architecture:** Layer A mirrors existing patterns to close obvious holes. Layer B adds verification zome functions to imagodei and mishpat DNAs, plus a storage module that calls them via the existing `hc_client.rs` conductor bridge. Layer C adds a `/elohim/trust/1.0.0` request-response protocol for connection-level trust negotiation, plus a per-connection context cache that makes reach authorization ambient rather than per-request.

**Tech Stack:** Rust (elohim-storage + Holochain HDK), libp2p request-response, MessagePack, `holochain_client` crate

**Design reference:** `genesis/plans/2026-03-23-ambient-trust-verification-design.md`

---

## Layer A: Low-Hanging Fruit (Ship Now)

These use existing patterns. No design risk. Each task is independently committable.

---

### Task A1: Add attestation_requirements to EprQahalContext

**Files:**
- Modify: `elohim/elohim-storage/src/epr_codec.rs:65-73`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:1416-1419` (EPR Head building)
- Modify: `elohim/elohim-storage/src/views.rs` (EprQahalContextInputView)
- Test: existing EPR codec roundtrip tests in `epr_codec.rs`

**Step 1: Add the field to EprQahalContext**

In `elohim/elohim-storage/src/epr_codec.rs:65-73`, add `attestation_requirements`:

```rust
/// Qahal pillar — governance context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EprQahalContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Attestation requirements for body access (Layer 2 gate).
    /// Format: "type:reference" e.g. "prerequisite-mastery:calculus-101", "consent:community-guidelines", "payment:tier-1"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestation_requirements: Vec<String>,
}
```

**Step 2: Update EPR Head building to populate attestation requirements**

In `elohim/elohim-storage/src/p2p/mod.rs:1416-1419`, query content_attestations and populate:

```rust
qahal: {
    let mut attestation_requirements = Vec::new();
    if let Ok(atts) = crate::db::content_attestations::query_attestations_for_content(
        &mut conn, &content.id,
    ) {
        for att in &atts {
            if att.is_revoked == 0 {
                let req = if let Some(ref evidence) = att.evidence {
                    format!("{}:{}", att.attestation_type, evidence)
                } else {
                    att.attestation_type.clone()
                };
                attestation_requirements.push(req);
            }
        }
    }
    crate::epr_codec::EprQahalContext {
        reach: Some(content.reach.clone()),
        layer: None,
        attestation_requirements,
    }
},
```

**Step 3: Update InputView for the new field**

In `views.rs`, add the field to `EprQahalContextInputView` and the `From` impl. Use the same `#[serde(default, skip_serializing_if = "Vec::is_empty")]` pattern.

**Step 4: Run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr`
Expected: All EPR roundtrip tests pass (the new field defaults to empty vec via `#[serde(default)]`, so existing serialized data is backward compatible).

**Step 5: Run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: Clean

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/epr_codec.rs elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/views.rs
git commit -m "feat(epr): add attestation_requirements to EprQahalContext

Populates from content_attestations on EPR Head build. Format is
'type:reference' (e.g. 'prerequisite-mastery:calculus-101',
'consent:community-guidelines', 'payment:tier-1'). Backward
compatible — defaults to empty vec for existing content."
```

---

### Task A2: Wire attestation gate on P2P EPR handler

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:1720-1723` (after policy enforcement, before serving)
- Reference: `elohim/elohim-storage/src/http.rs:2218-2285` (existing HTTP attestation gate to mirror)

**Step 1: Add attestation gate after policy enforcement check**

In `elohim/elohim-storage/src/p2p/mod.rs`, after the policy enforcement block (line ~1720) and before line 1725 ("Authorized — serve the EPR Head"), insert the attestation check. Mirror the HTTP logic from `http.rs:2218-2285`:

```rust
                            // Layer 2: Attestation gate (mirrors HTTP path)
                            if let Some(ref agent_key) = agent_pubkey {
                                let attestations =
                                    crate::db::content_attestations::query_attestations_for_content(
                                        &mut conn, &id,
                                    );
                                if let Ok(atts) = attestations {
                                    let prereq_atts: Vec<_> = atts
                                        .iter()
                                        .filter(|a| a.is_revoked == 0)
                                        .collect();

                                    if !prereq_atts.is_empty() {
                                        let human = crate::db::humans::get_human_by_agent_key(
                                            &mut conn, agent_key,
                                        );
                                        if let Ok(Some(human)) = human {
                                            for att in &prereq_atts {
                                                if att.attestation_type == "prerequisite-mastery" {
                                                    let prereq_content_id = att
                                                        .evidence
                                                        .as_deref()
                                                        .unwrap_or(&att.content_id);
                                                    let mastery =
                                                        crate::db::content_mastery::get_mastery_for_content(
                                                            &mut conn,
                                                            &app_ctx,
                                                            &human.id,
                                                            prereq_content_id,
                                                        );
                                                    match mastery {
                                                        Ok(Some(m)) if m.mastery_level != "not_started" => {}
                                                        _ => {
                                                            info!(id = %id, "P2P attestation gate: prerequisite mastery required");
                                                            return EprResponse::AccessDenied {
                                                                required_reach: reach.clone(),
                                                                reason: "Prerequisite mastery required".to_string(),
                                                            };
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
```

**Step 2: Run tests and clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: All pass, clean clippy

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add attestation gate to EPR handler

Mirrors the HTTP attestation check (http.rs:2218-2285) in the P2P
EPR resolve handler. Checks prerequisite-mastery attestations and
returns AccessDenied if the requesting agent lacks required mastery.
Closes the Layer 2 gap where P2P bypassed attestation checks."
```

---

### Task A3: Wire PolicyEnforcement initialization in main.rs

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs:338-346` (P2P node setup)
- Modify: `elohim/elohim-storage/src/main.rs:481-504` (HTTP server setup)
- Reference: `elohim/elohim-storage/src/db/policy_cache.rs` (PolicyCache, PolicyEnforcement)

**Step 1: Create PolicyEnforcement and wire to both servers**

In `main.rs`, after the DB pool is created for the HTTP server (~line 483), create a `PolicyEnforcement` instance and share it:

```rust
// After: let services = Arc::new(Services::new(pool.clone()));
// Create policy enforcement (shared between HTTP and P2P)
let policy_enforcement = {
    let policy_cache = crate::db::policy_cache::PolicyCache::new(pool.clone());
    Arc::new(crate::db::policy_cache::PolicyEnforcement::new(policy_cache))
};
http_server = http_server.with_policy_enforcement(policy_enforcement.clone());

// If P2P is enabled, wire policy enforcement to P2P node
// (P2P node is created earlier, need to pass via handle or re-wire)
```

Note: The P2P node is created before the HTTP server in `main.rs` (line 338). You need to either:
- Create the policy enforcement earlier (when the P2P pool is initialized at line 342), or
- Store a reference to pass later

The simplest approach: create `PolicyEnforcement` at line 342-345 when the P2P DB pool is created, and wire it immediately:

```rust
// Wire DB pool for EPR Head resolution (if content DB is available)
if args.enable_content_db {
    if let Ok(pool) = init_pool_from_dir(&config.storage_dir) {
        p2p_node = p2p_node.with_db_pool(pool.clone());
        // Wire policy enforcement for content filtering on P2P path
        let policy_cache = crate::db::policy_cache::PolicyCache::new(pool);
        let enforcement = Arc::new(crate::db::policy_cache::PolicyEnforcement::new(policy_cache));
        p2p_node = p2p_node.with_policy_enforcement(enforcement);
        info!("  P2P EPR resolution: DB pool + policy enforcement wired");
    }
}
```

For the HTTP server, create a second `PolicyEnforcement` from its own pool (line 483):

```rust
let policy_cache = crate::db::policy_cache::PolicyCache::new(pool.clone());
let enforcement = Arc::new(crate::db::policy_cache::PolicyEnforcement::new(policy_cache));
http_server = http_server.with_policy_enforcement(enforcement);
```

**Step 2: Run tests and clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: All 483+ tests pass, clean clippy

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat: wire PolicyEnforcement into P2P and HTTP servers

Creates PolicyEnforcement instances from the DB pool and passes them
to both HttpServer and P2PNode via their with_policy_enforcement()
builders. Previously the enforcement code was wired but never
instantiated — device policy checks now actually execute."
```

---

## Layer B: Three-Pillar Verification via Conductor

These tasks add the DHT verification path. Each is independently testable. B1 and B2 are DNA changes; B3 is the storage module that calls them.

---

### Task B1: Add verify_credentials zome function to imagodei DNA

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`
- Reference: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (entry types)

**Step 1: Add CredentialVerification types**

In `imagodei/src/lib.rs`, add the verification types and function:

```rust
// =============================================================================
// Credential Verification (for P2P trust negotiation)
// =============================================================================

/// Status of a verified credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Valid,
    NotFound,
    Revoked,
    Expired,
}

/// Result of verifying a single credential against the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialVerification {
    pub hash: ActionHash,
    pub status: VerificationStatus,
    pub entry_type: Option<String>,
    pub agent: Option<AgentPubKey>,
}

/// Verify multiple credentials against the DHT.
/// For each ActionHash, attempts to retrieve the record and returns its status.
/// Used by elohim-storage during P2P trust negotiation.
#[hdk_extern]
pub fn verify_credentials(hashes: Vec<ActionHash>) -> ExternResult<Vec<CredentialVerification>> {
    let mut results = Vec::with_capacity(hashes.len());

    for hash in hashes {
        let record = get(hash.clone(), GetOptions::default())?;
        let verification = match record {
            Some(record) => {
                let entry_type_name = match record.action().entry_type() {
                    Some(EntryType::App(app_entry)) => {
                        Some(format!("{:?}", app_entry))
                    }
                    _ => None,
                };
                let agent = Some(record.action().author().clone());
                // Check for revocation by looking for a specific link tag
                // (revocation is entry-type-specific; for now, mark as Valid
                // if the record exists and let the caller check fields)
                CredentialVerification {
                    hash,
                    status: VerificationStatus::Valid,
                    entry_type: entry_type_name,
                    agent,
                }
            }
            None => CredentialVerification {
                hash,
                status: VerificationStatus::NotFound,
                entry_type: None,
                agent: None,
            },
        };
        results.push(verification);
    }

    Ok(results)
}
```

**Step 2: Build the DNA**

Run: `cd elohim/holochain/dna/imagodei && just check`
Expected: WASM compilation succeeds

**Step 3: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(imagodei): add verify_credentials zome function

Accepts ActionHashes, resolves each against the DHT via get(),
returns status (Valid/NotFound), entry type, and creating agent.
Used by elohim-storage during P2P trust negotiation to verify
relationship and attestation CIDs presented by connecting peers."
```

---

### Task B2: Add verify_credentials zome function to mishpat DNA

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`

**Step 1: Add the same verification function to mishpat**

Same pattern as B1. The types can be duplicated (each DNA is a separate WASM; no shared crate needed for this small struct). Add `VerificationStatus`, `CredentialVerification`, and `verify_credentials` to `mishpat/src/lib.rs` with identical logic.

**Step 2: Build the DNA**

Run: `cd elohim/holochain/dna/mishpat && just check`
Expected: WASM compilation succeeds

**Step 3: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs
git commit -m "feat(mishpat): add verify_credentials zome function

Same pattern as imagodei — accepts ActionHashes, resolves against
DHT, returns verification status. Used for verifying collective
membership CIDs during P2P trust negotiation."
```

---

### Task B3: Create trust_verification module in elohim-storage

**Files:**
- Create: `elohim/elohim-storage/src/trust_verification.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (add `pub mod trust_verification;`)

**Step 1: Write the verification module**

Create `elohim/elohim-storage/src/trust_verification.rs`:

```rust
//! Three-pillar trust verification via conductor.
//!
//! Calls verify_credentials() zome functions in imagodei and mishpat DNAs
//! to verify credential CIDs against the DHT. Storage doesn't care which
//! DNA answered — it gets a unified result across all three pillars.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::error::StorageError;
use crate::hc_client::HcClient;

// =============================================================================
// Domain Types
// =============================================================================

/// Credentials presented by a peer during trust negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCredentials {
    pub agent_pubkey: String,
    /// Qahal: collective membership CIDs
    pub membership_cids: Vec<String>,
    /// Qahal: interpersonal relationship CIDs
    pub relationship_cids: Vec<String>,
    /// Any pillar: mastery, consent, payment attestation CIDs
    pub attestation_cids: Vec<String>,
    /// Shefa: stewardship commitment CIDs
    pub stewardship_cids: Vec<String>,
}

/// Verified trust context — the result of DHT verification.
#[derive(Debug, Clone)]
pub struct VerifiedTrustContext {
    pub agent_pubkey: String,
    pub agent_verified: bool,
    /// Highest reach tier this peer qualifies for (ambient ceiling)
    pub reach_ceiling: String,
    pub verified_memberships: Vec<VerifiedMembership>,
    pub verified_relationships: Vec<VerifiedRelationship>,
    pub verified_attestations: Vec<VerifiedAttestation>,
    pub verified_stewardship: Vec<VerifiedStewardship>,
    pub verified_at: Instant,
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct VerifiedMembership {
    pub cid: String,
    pub collective_id: String,
    pub consent_state: String,
}

#[derive(Debug, Clone)]
pub struct VerifiedRelationship {
    pub cid: String,
    pub party_a_id: String,
    pub party_b_id: String,
    pub intimacy_level: String,
    pub consent_given_by_a: bool,
    pub consent_given_by_b: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedAttestation {
    pub cid: String,
    pub attestation_type: String,
    pub reference: Option<String>,
    pub is_revoked: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedStewardship {
    pub cid: String,
    pub content_id: String,
    pub allocation_ratio: f64,
}

/// Default TTL for verified trust context (1 hour)
const DEFAULT_TTL_SECS: u64 = 3600;

// =============================================================================
// Verification via Conductor
// =============================================================================

/// Verify a peer's trust credentials against the DHT via conductor zome calls.
///
/// Routes CIDs to the appropriate DNA (imagodei for relationships/attestations,
/// mishpat for collective memberships) and returns a unified context.
///
/// If the conductor is unavailable, returns Err — callers should fall back
/// to per-request SQLite checks.
pub async fn verify_trust_context(
    hc_client: &HcClient,
    credentials: &TrustCredentials,
) -> Result<VerifiedTrustContext, StorageError> {
    debug!(
        agent = %credentials.agent_pubkey,
        memberships = credentials.membership_cids.len(),
        relationships = credentials.relationship_cids.len(),
        attestations = credentials.attestation_cids.len(),
        stewardship = credentials.stewardship_cids.len(),
        "Verifying trust credentials via conductor"
    );

    // Verify memberships via mishpat DNA
    let verified_memberships = verify_membership_cids(hc_client, &credentials.membership_cids).await?;

    // Verify relationships + attestations via imagodei DNA
    let verified_relationships = verify_relationship_cids(hc_client, &credentials.relationship_cids).await?;
    let verified_attestations = verify_attestation_cids(hc_client, &credentials.attestation_cids).await?;

    // Verify stewardship via imagodei DNA (stewardship coordinator)
    let verified_stewardship = verify_stewardship_cids(hc_client, &credentials.stewardship_cids).await?;

    // Calculate ambient reach ceiling from verified credentials
    let reach_ceiling = calculate_reach_ceiling(
        &verified_memberships,
        &verified_relationships,
    );

    Ok(VerifiedTrustContext {
        agent_pubkey: credentials.agent_pubkey.clone(),
        agent_verified: true,
        reach_ceiling,
        verified_memberships,
        verified_relationships,
        verified_attestations,
        verified_stewardship,
        verified_at: Instant::now(),
        ttl: Duration::from_secs(DEFAULT_TTL_SECS),
    })
}

/// Calculate the highest ambient reach ceiling from verified credentials.
///
/// Note: familiar/trusted/intimate are content-specific (depend on which
/// stewards are allocated). The ceiling here is the POTENTIAL maximum —
/// per-request checks still validate against specific content's stewards.
fn calculate_reach_ceiling(
    memberships: &[VerifiedMembership],
    relationships: &[VerifiedRelationship],
) -> String {
    // Check for intimate: mutual intimate relationship with both consents
    if relationships.iter().any(|r| {
        r.intimacy_level == "intimate" && r.consent_given_by_a && r.consent_given_by_b
    }) {
        return "intimate".to_string();
    }

    // Check for trusted: any relationship at intimacy >= trusted
    let trusted_idx = crate::db::models::intimacy_levels::index_of("trusted").unwrap_or(2);
    if relationships.iter().any(|r| {
        crate::db::models::intimacy_levels::index_of(&r.intimacy_level)
            .map(|idx| idx >= trusted_idx)
            .unwrap_or(false)
    }) {
        return "trusted".to_string();
    }

    // Check for community: any consented membership
    if memberships.iter().any(|m| m.consent_state == "consented") {
        return "community".to_string();
    }

    // Default: public (anyone can see commons/public content)
    "public".to_string()
}

// =============================================================================
// Per-DNA Verification Helpers
// =============================================================================

async fn verify_membership_cids(
    hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedMembership>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }

    // Call mishpat DNA's verify_credentials
    // For now, return empty — implementation requires ActionHash parsing
    // which depends on the conductor's serialization format
    debug!(count = cids.len(), "Membership CID verification: stub (pending conductor integration)");
    Ok(Vec::new())
}

async fn verify_relationship_cids(
    hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedRelationship>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }

    debug!(count = cids.len(), "Relationship CID verification: stub (pending conductor integration)");
    Ok(Vec::new())
}

async fn verify_attestation_cids(
    hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedAttestation>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }

    debug!(count = cids.len(), "Attestation CID verification: stub (pending conductor integration)");
    Ok(Vec::new())
}

async fn verify_stewardship_cids(
    hc_client: &HcClient,
    cids: &[String],
) -> Result<Vec<VerifiedStewardship>, StorageError> {
    if cids.is_empty() {
        return Ok(Vec::new());
    }

    debug!(count = cids.len(), "Stewardship CID verification: stub (pending conductor integration)");
    Ok(Vec::new())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_ceiling_intimate_with_mutual_consent() {
        let memberships = vec![];
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "intimate".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: true,
        }];
        assert_eq!(calculate_reach_ceiling(&memberships, &relationships), "intimate");
    }

    #[test]
    fn reach_ceiling_trusted_relationship() {
        let memberships = vec![];
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "trusted".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: false,
        }];
        assert_eq!(calculate_reach_ceiling(&memberships, &relationships), "trusted");
    }

    #[test]
    fn reach_ceiling_community_membership() {
        let memberships = vec![VerifiedMembership {
            cid: "bafkrei-test".to_string(),
            collective_id: "church-001".to_string(),
            consent_state: "consented".to_string(),
        }];
        let relationships = vec![];
        assert_eq!(calculate_reach_ceiling(&memberships, &relationships), "community");
    }

    #[test]
    fn reach_ceiling_defaults_to_public() {
        assert_eq!(calculate_reach_ceiling(&[], &[]), "public");
    }

    #[test]
    fn reach_ceiling_intimate_without_consent_falls_to_trusted() {
        let relationships = vec![VerifiedRelationship {
            cid: "bafkrei-test".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            intimacy_level: "intimate".to_string(),
            consent_given_by_a: true,
            consent_given_by_b: false, // no mutual consent
        }];
        // intimate without mutual consent → still >= trusted
        assert_eq!(calculate_reach_ceiling(&[], &relationships), "trusted");
    }
}
```

**Step 2: Register the module**

In `elohim/elohim-storage/src/lib.rs`, add `pub mod trust_verification;` alongside the other module declarations.

**Step 3: Run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test trust_verification`
Expected: 5 tests pass (reach ceiling calculation)

**Step 4: Run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: Clean (may warn about unused `hc_client` parameter in stubs — suppress with `let _ = hc_client;`)

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/trust_verification.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat: add trust_verification module for three-pillar DHT verification

Defines TrustCredentials, VerifiedTrustContext, and the verification
interface. Per-DNA verification helpers are stubbed pending conductor
integration. Reach ceiling calculation is fully implemented with tests.
This is the storage side of the ambient trust model — Layer B."
```

---

## Layer C: Connection Handshake + Context Cache

These tasks build the ambient trust mechanism. C1 creates the wire protocol, C2 creates the cache, C3 wires them into the P2P event loop, C4 adds the fast-path to reach authorization.

---

### Task C1: Create trust handshake protocol

**Files:**
- Create: `elohim/elohim-storage/src/p2p/trust_protocol.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:31-35` (add `pub mod trust_protocol;`)

**Step 1: Write the protocol codec**

Create `elohim/elohim-storage/src/p2p/trust_protocol.rs`. Follow the exact pattern of `epr_protocol.rs` — 4-byte BE length prefix + MessagePack body:

```rust
//! Trust Negotiation Protocol — per-connection credential exchange
//!
//! On ConnectionEstablished, peers exchange trust credentials (CIDs of
//! memberships, relationships, attestations, stewardship). The receiving
//! peer verifies CIDs against the DHT via conductor, caches the result,
//! and returns the verified reach ceiling + TTL.
//!
//! Wire format: 4-byte BE length prefix + MessagePack body.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

pub const TRUST_PROTOCOL_ID: &str = "/elohim/trust/1.0.0";

const MAX_REQUEST_SIZE: usize = 64 * 1024;  // 64KB (credential lists)
const MAX_RESPONSE_SIZE: usize = 4 * 1024;  // 4KB (ceiling + TTL)

#[derive(Debug, Clone)]
pub struct TrustProtocol;

impl AsRef<str> for TrustProtocol {
    fn as_ref(&self) -> &str {
        TRUST_PROTOCOL_ID
    }
}

/// Trust handshake request — peer presents credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustHandshake {
    pub agent_pubkey: String,
    pub membership_cids: Vec<String>,
    pub relationship_cids: Vec<String>,
    pub attestation_cids: Vec<String>,
    pub stewardship_cids: Vec<String>,
}

/// Trust handshake response — verified reach ceiling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustResponse {
    Verified {
        reach_ceiling: String,
        ttl_seconds: u64,
    },
    Rejected {
        reason: String,
    },
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct TrustCodec;

#[async_trait]
impl request_response::Codec for TrustCodec {
    type Protocol = TrustProtocol;
    type Request = TrustHandshake;
    type Response = TrustResponse;

    // read_request, read_response, write_request, write_response
    // Follow IDENTICAL pattern to EprCodec in epr_protocol.rs:
    // 4-byte BE length prefix + rmp_serde::from_slice / to_vec
    // Use MAX_REQUEST_SIZE / MAX_RESPONSE_SIZE for limits
    // (Copy the implementation from epr_protocol.rs, changing only the type parameters and size limits)

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where T: AsyncRead + Unpin + Send {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_REQUEST_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Trust request too large: {} (max {})", len, MAX_REQUEST_SIZE)));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where T: AsyncRead + Unpin + Send {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_RESPONSE_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Trust response too large: {} (max {})", len, MAX_RESPONSE_SIZE)));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        let data = rmp_serde::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&(data.len() as u32).to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, resp: Self::Response) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        let data = rmp_serde::to_vec(&resp).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&(data.len() as u32).to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_roundtrip() {
        let handshake = TrustHandshake {
            agent_pubkey: "uhCAk_test".to_string(),
            membership_cids: vec!["bafkrei-mem1".to_string()],
            relationship_cids: vec![],
            attestation_cids: vec!["bafkrei-att1".to_string()],
            stewardship_cids: vec![],
        };
        let bytes = rmp_serde::to_vec(&handshake).unwrap();
        let decoded: TrustHandshake = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.agent_pubkey, "uhCAk_test");
        assert_eq!(decoded.membership_cids.len(), 1);
    }

    #[test]
    fn response_verified_roundtrip() {
        let resp = TrustResponse::Verified { reach_ceiling: "trusted".to_string(), ttl_seconds: 3600 };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: TrustResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            TrustResponse::Verified { reach_ceiling, ttl_seconds } => {
                assert_eq!(reach_ceiling, "trusted");
                assert_eq!(ttl_seconds, 3600);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn response_rejected_roundtrip() {
        let resp = TrustResponse::Rejected { reason: "invalid agent".to_string() };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: TrustResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            TrustResponse::Rejected { reason } => assert_eq!(reason, "invalid agent"),
            _ => panic!("Wrong variant"),
        }
    }
}
```

**Step 2: Register the module**

In `elohim/elohim-storage/src/p2p/mod.rs:31-35`, add: `pub mod trust_protocol;`

**Step 3: Run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test trust_protocol`
Expected: 3 roundtrip tests pass

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/trust_protocol.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add trust handshake protocol /elohim/trust/1.0.0

Request-response protocol for per-connection trust negotiation.
Peers exchange credential CIDs (memberships, relationships,
attestations, stewardship) and receive a verified reach ceiling
with TTL. Same wire format as EPR and shard protocols."
```

---

### Task C2: Create per-connection trust context cache

**Files:**
- Create: `elohim/elohim-storage/src/p2p/trust_cache.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add `pub mod trust_cache;`)

**Step 1: Write the cache module**

Create `elohim/elohim-storage/src/p2p/trust_cache.rs`:

```rust
//! Per-connection trust context cache.
//!
//! Stores verified trust contexts keyed by libp2p PeerId. Populated by
//! the trust handshake, queried by check_reach_authorization for fast-path
//! ambient authorization.

use libp2p::PeerId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::trust_verification::VerifiedTrustContext;

/// Thread-safe peer trust cache
#[derive(Clone)]
pub struct PeerTrustCache {
    inner: Arc<RwLock<HashMap<PeerId, VerifiedTrustContext>>>,
}

impl PeerTrustCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert or replace a peer's verified trust context.
    pub async fn insert(&self, peer_id: PeerId, ctx: VerifiedTrustContext) {
        self.inner.write().await.insert(peer_id, ctx);
    }

    /// Get a peer's trust context if it exists and is not expired.
    pub async fn get(&self, peer_id: &PeerId) -> Option<VerifiedTrustContext> {
        let cache = self.inner.read().await;
        let ctx = cache.get(peer_id)?;
        if ctx.verified_at.elapsed() < ctx.ttl {
            Some(ctx.clone())
        } else {
            None
        }
    }

    /// Remove a peer's trust context (on disconnect).
    pub async fn remove(&self, peer_id: &PeerId) {
        self.inner.write().await.remove(peer_id);
    }

    /// Evict all expired entries.
    pub async fn evict_expired(&self) {
        let mut cache = self.inner.write().await;
        cache.retain(|_, ctx| ctx.verified_at.elapsed() < ctx.ttl);
    }

    /// Number of cached peers (for observability).
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

impl Default for PeerTrustCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_verification::VerifiedTrustContext;

    fn make_context(ceiling: &str, ttl_secs: u64) -> VerifiedTrustContext {
        VerifiedTrustContext {
            agent_pubkey: "uhCAk_test".to_string(),
            agent_verified: true,
            reach_ceiling: ceiling.to_string(),
            verified_memberships: vec![],
            verified_relationships: vec![],
            verified_attestations: vec![],
            verified_stewardship: vec![],
            verified_at: Instant::now(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    #[tokio::test]
    async fn insert_and_get() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        cache.insert(peer, make_context("trusted", 3600)).await;
        let ctx = cache.get(&peer).await.unwrap();
        assert_eq!(ctx.reach_ceiling, "trusted");
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_peer() {
        let cache = PeerTrustCache::new();
        assert!(cache.get(&PeerId::random()).await.is_none());
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        // TTL of 0 seconds = immediately expired
        cache.insert(peer, make_context("trusted", 0)).await;
        assert!(cache.get(&peer).await.is_none());
    }

    #[tokio::test]
    async fn remove_clears_entry() {
        let cache = PeerTrustCache::new();
        let peer = PeerId::random();
        cache.insert(peer, make_context("trusted", 3600)).await;
        cache.remove(&peer).await;
        assert!(cache.get(&peer).await.is_none());
    }

    #[tokio::test]
    async fn evict_expired_cleans_stale_entries() {
        let cache = PeerTrustCache::new();
        let fresh = PeerId::random();
        let stale = PeerId::random();
        cache.insert(fresh, make_context("trusted", 3600)).await;
        cache.insert(stale, make_context("community", 0)).await;
        cache.evict_expired().await;
        assert_eq!(cache.len().await, 1);
        assert!(cache.get(&fresh).await.is_some());
        assert!(cache.get(&stale).await.is_none());
    }
}
```

**Step 2: Register the module**

Add `pub mod trust_cache;` to `p2p/mod.rs`.

**Step 3: Run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test trust_cache`
Expected: 5 tests pass

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/trust_cache.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add PeerTrustCache for ambient trust context

In-memory cache keyed by PeerId, stores VerifiedTrustContext with
TTL-based expiry. Insert on handshake, query on EPR request,
remove on disconnect. Rebuilt on restart via re-handshake."
```

---

### Task C3: Wire handshake into P2P event loop

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (struct fields, ConnectionEstablished handler, behaviour)
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` (add trust protocol to behaviour)

**Step 1: Add trust protocol to ElohimStorageBehaviour**

In `behaviour.rs`, add the trust request-response behaviour alongside the existing EPR and shard behaviours. Follow the exact same pattern used for EPR:

```rust
// In the #[derive(NetworkBehaviour)] struct:
pub trust: request_response::Behaviour<trust_protocol::TrustCodec>,
```

Initialize it in the behaviour constructor with `trust_protocol::TrustProtocol` and `trust_protocol::TrustCodec`.

**Step 2: Add cache and event handling to P2PNode**

In `p2p/mod.rs`, add `peer_trust_cache: trust_cache::PeerTrustCache` to the `P2PNode` struct (line ~148). Initialize in constructor.

Handle trust protocol events in the event loop (`handle_event`):
- On `TrustHandshake` received: verify credentials (for now, build context from credentials without conductor call — conductor integration comes when B3 stubs are filled), cache result, send `TrustResponse::Verified`.
- On `TrustResponse::Verified` received: cache the peer's reach ceiling.

**Step 3: Trigger handshake on ConnectionEstablished**

In the `ConnectionEstablished` handler (~line 708), after the existing mDNS/Kademlia logic, send a `TrustHandshake` to the new peer. For now, send empty credential lists (the connecting peer's own credentials will be populated when the system has conductor access).

**Step 4: Handle ConnectionClosed**

In the `ConnectionClosed` handler, call `self.peer_trust_cache.remove(&peer_id).await`.

**Step 5: Run tests and clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: All tests pass, clean clippy

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/p2p/behaviour.rs
git commit -m "feat(p2p): wire trust handshake into connection lifecycle

Trust protocol added to ElohimStorageBehaviour. Handshake sent on
ConnectionEstablished, cache cleared on ConnectionClosed. Verification
uses local credentials for now — conductor integration fills in when
B3 stubs are complete."
```

---

### Task C4: Fast-path reach authorization from cache

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (check_reach_authorization)

**Step 1: Add cache fast-path at top of check_reach_authorization**

The method signature needs a `peer_id: Option<&PeerId>` parameter. At the top of the method, before the `match reach` block, add the fast-path:

```rust
fn check_reach_authorization(
    &self,
    reach: &str,
    agent_pubkey: Option<&str>,
    content_id: &str,
    peer_id: Option<&libp2p::PeerId>,  // NEW
) -> Result<(), String> {
    // Fast path: check cached peer trust context
    if let Some(pid) = peer_id {
        // Use try_read to avoid blocking — fall through to slow path if contended
        if let Ok(cache) = self.peer_trust_cache.inner.try_read() {
            if let Some(ctx) = cache.get(pid) {
                if ctx.verified_at.elapsed() < ctx.ttl {
                    let reach_idx = reach_level_index(reach);
                    let ceiling_idx = reach_level_index(&ctx.reach_ceiling);
                    if reach_idx <= ceiling_idx {
                        // For community and below, ambient ceiling is sufficient
                        if reach_idx <= reach_level_index("community") {
                            return Ok(());
                        }
                        // For familiar+, still need content-specific steward check
                        // but use cached credentials instead of DB for relationships
                        // (fall through to slow path for now — optimization in future sprint)
                    }
                }
            }
        }
    }

    // Slow path: per-request DB lookup (existing behavior)
    match reach {
        "commons" | "public" => Ok(()),
        // ... existing code unchanged ...
    }
}
```

Add a helper function:

```rust
fn reach_level_index(reach: &str) -> u8 {
    match reach {
        "commons" | "public" => 0,
        "community" => 1,
        "familiar" => 2,
        "trusted" => 3,
        "intimate" => 4,
        "self" | "private" => 5,
        _ => 0,
    }
}
```

**Step 2: Update the call site**

In `handle_epr_request`, pass the peer_id when calling `check_reach_authorization`. The peer_id is available from the request-response event context — it may need to be threaded through from the event handler.

**Step 3: Run all tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings`
Expected: All tests pass, clean clippy

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add fast-path reach authorization from trust cache

check_reach_authorization now checks the PeerTrustCache first.
For community and below, a valid cached ceiling skips all DB queries.
For familiar+ tiers, falls through to per-request checks (future:
use cached credentials for relationship lookups too).

This completes the ambient trust model: handshake → verify → cache
→ fast-path. Per-request SQLite remains as fallback."
```

---

## Verification Checklist

After all tasks are complete:

1. `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings` — clean
2. `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test` — all pass (483+ existing + new)
3. `cd elohim/holochain/dna/imagodei && just check` — WASM builds
4. `cd elohim/holochain/dna/mishpat && just check` — WASM builds
5. Layer A is fully functional immediately
6. Layer B has types + ceiling calculation working; conductor call stubs ready for integration
7. Layer C has protocol + cache + fast-path working; handshake triggers on connection
