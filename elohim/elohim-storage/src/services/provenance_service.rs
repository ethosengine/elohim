//! Provenance hash service — Merkle-tree proof of witnessed contribution.
//!
//! A provenance proof is a Merkle root computed over all mint events for an agent
//! in a governance layer. It cryptographically proves that tokens came from
//! witnessed contribution, not computation or capital.
//!
//! ## Leaf Construction
//!
//! Each leaf is SHA256(id || amount_bytes || provenance_event_id || mint_tier || created_at).
//! This binds every mint record to its core fields without including mutable metadata.
//!
//! ## Tree Shape
//!
//! Standard binary Merkle tree: leaves are paired, SHA256(left || right) at each
//! level. Odd-count levels duplicate the last node so every level is even.

use diesel::SqliteConnection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::context::AppContext;
use crate::db::models::TokenMintEvent;
use crate::db::token_mint_events;
use crate::error::StorageError;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Cryptographic proof that an agent's tokens came from witnessed contribution.
///
/// `merkle_root` is the hex-encoded SHA256 Merkle root over all mint events.
/// When there are no mint events, the root is the all-zero string (64 × '0').
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceProof {
    /// Hex-encoded Merkle root (64 chars = 32 bytes).
    pub merkle_root: String,
    /// Number of mint events covered by this proof.
    pub event_count: u32,
    /// Sum of all minted amounts.
    pub total_amount: f64,
    /// The agent whose mints form this proof.
    pub agent_id: String,
    /// Governance layer context.
    pub governance_layer: String,
    /// Ordered summaries of every mint event (same ordering used to build the tree).
    pub event_summaries: Vec<ProvenanceEventSummary>,
}

/// A condensed record of a single mint event in a provenance proof.
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

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct ProvenanceService;

impl ProvenanceService {
    /// Generate a provenance proof for an agent.
    ///
    /// Fetches all mint events for the agent (app-scoped), builds leaf hashes,
    /// computes the Merkle root, and returns a `ProvenanceProof`.
    ///
    /// The `governance_layer` is stored on the proof for display purposes only;
    /// the underlying mint query is not filtered by layer (all mints contribute
    /// to the agent's global provenance).
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

        // Build leaf hashes in fetch order (newest-first as returned by get_mints_for_agent).
        let leaves: Vec<[u8; 32]> = mints.iter().map(Self::leaf_hash).collect();

        let root = Self::compute_merkle_root(&leaves);
        let merkle_root = hex::encode(root);

        let total_amount: f64 = mints.iter().map(|m| m.amount as f64).sum();
        let event_count = mints.len() as u32;

        let event_summaries = mints
            .into_iter()
            .map(|m| ProvenanceEventSummary {
                event_id: m.id,
                action: "mint".to_string(),
                amount: m.amount,
                mint_tier: m.mint_tier,
                source_epr_id: m.source_epr_id,
                timestamp: m.created_at,
            })
            .collect();

        Ok(ProvenanceProof {
            merkle_root,
            event_count,
            total_amount,
            agent_id: agent_id.to_string(),
            governance_layer: governance_layer.to_string(),
            event_summaries,
        })
    }

    /// Compute a single leaf hash for a mint event.
    ///
    /// Hash input: id || amount_bytes (f32 BE) || provenance_event_id || mint_tier || created_at
    fn leaf_hash(mint: &TokenMintEvent) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(mint.id.as_bytes());
        hasher.update(mint.amount.to_be_bytes());
        hasher.update(mint.provenance_event_id.as_bytes());
        hasher.update(mint.mint_tier.as_bytes());
        hasher.update(mint.created_at.as_bytes());
        hasher.finalize().into()
    }

    /// Compute a binary Merkle root from a slice of 32-byte leaf hashes.
    ///
    /// - Empty slice → `[0u8; 32]`
    /// - Single leaf → returned as-is
    /// - Multiple leaves → SHA256(left || right) at each level, duplicating the
    ///   last node when the level count is odd, until one root remains.
    pub fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
        match leaves.len() {
            0 => [0u8; 32],
            1 => leaves[0],
            _ => {
                let mut level: Vec<[u8; 32]> = leaves.to_vec();
                while level.len() > 1 {
                    // Duplicate last node if odd count.
                    if level.len() % 2 == 1 {
                        let last = *level.last().unwrap();
                        level.push(last);
                    }
                    let mut next = Vec::with_capacity(level.len() / 2);
                    for pair in level.chunks_exact(2) {
                        let mut hasher = Sha256::new();
                        hasher.update(pair[0]);
                        hasher.update(pair[1]);
                        next.push(hasher.finalize().into());
                    }
                    level = next;
                }
                level[0]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn leaf(seed: u8) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update([seed]);
        h.finalize().into()
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = ProvenanceService::compute_merkle_root(&[]);
        assert_eq!(root, [0u8; 32], "empty input must yield all-zero root");
    }

    #[test]
    fn test_merkle_root_single_leaf() {
        let l = leaf(1);
        let root = ProvenanceService::compute_merkle_root(&[l]);
        assert_eq!(root, l, "single leaf must be returned unchanged");
    }

    #[test]
    fn test_merkle_root_two_leaves() {
        let l1 = leaf(1);
        let l2 = leaf(2);

        let mut hasher = Sha256::new();
        hasher.update(l1);
        hasher.update(l2);
        let expected: [u8; 32] = hasher.finalize().into();

        let root = ProvenanceService::compute_merkle_root(&[l1, l2]);
        assert_eq!(root, expected, "two leaves must produce SHA256(l1 || l2)");
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let leaves: Vec<[u8; 32]> = (1u8..=5).map(leaf).collect();
        let root1 = ProvenanceService::compute_merkle_root(&leaves);
        let root2 = ProvenanceService::compute_merkle_root(&leaves);
        assert_eq!(root1, root2, "same input must always produce same root");

        // Different ordering must produce different root.
        let mut shuffled = leaves.clone();
        shuffled.swap(0, 2);
        let root3 = ProvenanceService::compute_merkle_root(&shuffled);
        assert_ne!(root1, root3, "different ordering must produce different root");
    }
}
