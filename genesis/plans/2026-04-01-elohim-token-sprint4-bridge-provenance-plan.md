# Elohim Token Sprint 4: Settlement Bridge Interface & Provenance Verification

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define the chain-agnostic SettlementBridge trait, implement provenance hash generation (Merkle root of REA events backing tokens), and expose a verification API that proves tokens came from witnessed contribution.

**Architecture:** The bridge trait lives in a new `elohim/elohim-token/src/` crate (pure Rust, no storage dependency — it defines the interface any settlement chain implements). Provenance hash generation and the verification API live in elohim-storage (they need DB access to walk the event chain). The verification endpoint returns a `ProvenanceProof` containing the Merkle root, event count, and event summaries.

**Tech Stack:** Rust (new elohim-token crate for trait, elohim-storage for implementation)

---

## Task 1: SettlementBridge Trait Crate

Create `elohim/elohim-token/src/` as a minimal Rust library crate defining the bridge interface. This is the contract — no implementation, no storage dependency.

**Files:**
- Create: `elohim/elohim-token/Cargo.toml`
- Create: `elohim/elohim-token/src/lib.rs`
- Modify: `elohim/Cargo.toml` (workspace members)

### Cargo.toml
```toml
[package]
name = "elohim-token"
version = "0.1.0"
edition = "2021"
description = "Elohim Protocol default economic rail — settlement bridge interface and token primitives"

[dependencies]
serde = { version = "1", features = ["derive"] }
```

### lib.rs

The trait defines what any settlement chain implementation must provide:

```rust
use serde::{Deserialize, Serialize};

/// Hash type — opaque bytes, chain implementations choose their own hash function
pub type Hash = Vec<u8>;

/// Cryptographic proof that elohim evaluated and approved a bridge crossing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElohimSignature {
    pub agent_id: String,
    pub signature: Vec<u8>,
    pub model_version: String,
    pub inference_hash: String,
    pub timestamp: String,
}

/// Constitutional context carried across the bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalContext {
    pub governance_layer: String,
    pub social_contract_health: f32,
    pub dignity_floor: f32,
    pub soft_ceiling: f32,
    pub hard_ceiling: f32,
}

/// Receipt from a successful bridge-out operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeReceipt {
    pub bridge_tx_id: String,
    pub amount: f64,
    pub provenance_root: Hash,
    pub settlement_chain: String,
    pub timestamp: String,
}

/// Proof that tokens are backed by witnessed contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceProof {
    pub merkle_root: Hash,
    pub event_count: u32,
    pub total_amount: f64,
    pub agent_id: String,
    pub governance_layer: String,
    pub event_summaries: Vec<ProvenanceEventSummary>,
}

/// Summary of a single provenance event (no private data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEventSummary {
    pub event_id: String,
    pub action: String,
    pub amount: f32,
    pub mint_tier: String,
    pub timestamp: String,
}

/// The settlement bridge interface.
/// Chain-specific implementations provide the actual bridging logic.
/// The protocol doesn't care which chain — only that the contract is met.
pub trait SettlementBridge {
    type Error: std::fmt::Debug;

    /// Bridge tokens out from Holochain to the settlement chain.
    /// Requires provenance proof and elohim signature.
    fn bridge_out(
        &self,
        amount: f64,
        provenance_root: Hash,
        constitutional_ctx: ConstitutionalContext,
        elohim_sig: ElohimSignature,
    ) -> Result<BridgeReceipt, Self::Error>;

    /// Bridge tokens back in from settlement chain to Holochain.
    fn bridge_in(
        &self,
        receipt: BridgeReceipt,
    ) -> Result<(), Self::Error>;

    /// Verify that a provenance root is valid on the settlement chain.
    fn verify_provenance(
        &self,
        root: Hash,
    ) -> Result<bool, Self::Error>;
}

/// Mint tier — micro (deterministic per-event) or discernment (elohim wisdom)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MintTier {
    Micro,
    Discernment,
}
```

### Workspace registration
Add `"elohim/elohim-token"` to the workspace members in `elohim/Cargo.toml`.

---

## Task 2: Provenance Hash Service

Add provenance hash generation to elohim-storage. This walks the mint event chain for an agent and produces a Merkle root proving the tokens came from witnessed contribution.

**Files:**
- Create: `elohim/elohim-storage/src/services/provenance_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

### provenance_service.rs

```rust
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};
use serde::Serialize;

use crate::db::context::AppContext;
use crate::db::token_mint_events;
use crate::error::StorageError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceProof {
    pub merkle_root: String,
    pub event_count: u32,
    pub total_amount: f64,
    pub agent_id: String,
    pub governance_layer: String,
    pub event_summaries: Vec<ProvenanceEventSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceEventSummary {
    pub event_id: String,
    pub action: String,
    pub amount: f32,
    pub mint_tier: String,
    pub source_epr_id: String,
    pub timestamp: String,
}

pub struct ProvenanceService;

impl ProvenanceService {
    /// Generate a provenance proof for an agent's token balance.
    /// Walks all mint events, builds a Merkle tree, returns the root
    /// plus event summaries (no private data).
    pub fn generate_proof(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<ProvenanceProof, StorageError> {
        let mints = token_mint_events::get_mints_for_agent(conn, ctx, agent_id)?;

        if mints.is_empty() {
            return Ok(ProvenanceProof {
                merkle_root: "0".repeat(64),
                event_count: 0,
                total_amount: 0.0,
                agent_id: agent_id.to_string(),
                governance_layer: governance_layer.to_string(),
                event_summaries: vec![],
            });
        }

        // Build leaf hashes from mint events
        let leaves: Vec<[u8; 32]> = mints
            .iter()
            .map(|m| {
                let mut hasher = Sha256::new();
                hasher.update(m.id.as_bytes());
                hasher.update(m.amount.to_le_bytes());
                hasher.update(m.provenance_event_id.as_bytes());
                hasher.update(m.mint_tier.as_bytes());
                hasher.update(m.created_at.as_bytes());
                hasher.finalize().into()
            })
            .collect();

        let merkle_root = Self::compute_merkle_root(&leaves);

        let total_amount: f64 = mints.iter().map(|m| m.amount as f64).sum();

        let summaries: Vec<ProvenanceEventSummary> = mints
            .iter()
            .map(|m| ProvenanceEventSummary {
                event_id: m.id.clone(),
                action: "mint".to_string(),
                amount: m.amount,
                mint_tier: m.mint_tier.clone(),
                source_epr_id: m.source_epr_id.clone(),
                timestamp: m.created_at.clone(),
            })
            .collect();

        Ok(ProvenanceProof {
            merkle_root: hex::encode(merkle_root),
            event_count: mints.len() as u32,
            total_amount,
            agent_id: agent_id.to_string(),
            governance_layer: governance_layer.to_string(),
            event_summaries: summaries,
        })
    }

    /// Compute Merkle root from leaf hashes.
    /// Uses SHA256 for internal nodes: H(left || right).
    /// If odd number of leaves, duplicates the last one.
    fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }
        if leaves.len() == 1 {
            return leaves[0];
        }

        let mut current_level: Vec<[u8; 32]> = leaves.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                if chunk.len() == 2 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]); // duplicate last if odd
                }
                next_level.push(hasher.finalize().into());
            }
            current_level = next_level;
        }

        current_level[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_single_leaf() {
        let leaf = Sha256::digest(b"test");
        let root = ProvenanceService::compute_merkle_root(&[leaf.into()]);
        assert_eq!(root, <[u8; 32]>::from(leaf));
    }

    #[test]
    fn test_merkle_root_two_leaves() {
        let leaf1: [u8; 32] = Sha256::digest(b"event1").into();
        let leaf2: [u8; 32] = Sha256::digest(b"event2").into();
        let root = ProvenanceService::compute_merkle_root(&[leaf1, leaf2]);

        // Manually compute expected
        let mut hasher = Sha256::new();
        hasher.update(leaf1);
        hasher.update(leaf2);
        let expected: [u8; 32] = hasher.finalize().into();

        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let leaves: Vec<[u8; 32]> = (0..5)
            .map(|i| Sha256::digest(format!("event{}", i).as_bytes()).into())
            .collect();
        let root1 = ProvenanceService::compute_merkle_root(&leaves);
        let root2 = ProvenanceService::compute_merkle_root(&leaves);
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = ProvenanceService::compute_merkle_root(&[]);
        assert_eq!(root, [0u8; 32]);
    }
}
```

---

## Task 3: Verification API Routes

**Files:**
- Modify: `elohim/elohim-storage/src/api/token.rs`

Add routes:

### GET /api/v1/token/provenance/{agent_id}/{governance_layer}
Generate and return a ProvenanceProof for an agent. This is the key verification endpoint — "prove that my tokens came from witnessed contribution."

Split path on first `/` to extract agent_id and governance_layer (same pattern as obligation route).

Call `ProvenanceService::generate_proof()`. Return `ProvenanceProof` directly (it derives Serialize with camelCase).

### GET /api/v1/token/provenance/{agent_id}
Shorthand — defaults governance_layer to "individual".

---

## Task 4: Integration Verification

- Full release build
- All tests pass (expect ~530+)
- TypeScript codegen for any new exported types
- Verify the elohim-token crate builds independently: `cd elohim/elohim-token && cargo check`
- Grep: no "amplification" in token code, no "De Beers" anywhere

---

## Sprint 4 Deliverables

1. **SettlementBridge trait** — chain-agnostic interface in its own crate (`elohim-token`)
2. **Supporting types** — ElohimSignature, ConstitutionalContext, BridgeReceipt, ProvenanceProof
3. **Provenance hash generation** — Merkle root from mint event chain
4. **Verification API** — prove tokens came from witnessed contribution
5. **4+ unit tests** — Merkle tree correctness
