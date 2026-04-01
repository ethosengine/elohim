//! Token mint service — micro-mint calculation and issuance.
//!
//! Translates witnessed REA economic events into elohim-token mints.
//! Mint amounts are computed from event-type weights, steward allocation ratios,
//! and an optional protocol mint rate (defaults to 1.0).

use diesel::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::context::AppContext;
use crate::db::models::NewTokenMintEvent;
use crate::db::{token_balances, token_mint_events};
use crate::db::token_balances::CreditSource;
use crate::error::StorageError;
use crate::views::TokenMintEventView;

const DEFAULT_MINT_RATE: f32 = 1.0;

const WEIGHT_MASTERY_COMPLETION: f32 = 1.0;
const WEIGHT_CONTENT_CITATION: f32 = 0.5;
const WEIGHT_ASSESSMENT_ATTEMPT: f32 = 0.1;
const WEIGHT_CONTENT_ACCESS: f32 = 0.01;

pub struct TokenMintService;

impl TokenMintService {
    /// Calculate the micro-mint amount for a given event type and allocation ratio.
    ///
    /// `mint_rate` defaults to 1.0 when `None`.
    pub fn calculate_micro_mint(
        event_type: &str,
        allocation_ratio: f32,
        mint_rate: Option<f32>,
    ) -> f32 {
        let rate = mint_rate.unwrap_or(DEFAULT_MINT_RATE);
        let weight = Self::event_weight(event_type);
        weight * allocation_ratio * rate
    }

    fn event_weight(event_type: &str) -> f32 {
        match event_type {
            "mastery-advance" | "mastery-completion" | "path-completion" => {
                WEIGHT_MASTERY_COMPLETION
            }
            "citation" | "content-citation" => WEIGHT_CONTENT_CITATION,
            "assessment-attempt" | "quiz-attempt" => WEIGHT_ASSESSMENT_ATTEMPT,
            "content-view" | "content-access" => WEIGHT_CONTENT_ACCESS,
            _ => WEIGHT_CONTENT_ACCESS,
        }
    }

    /// Issue a micro-mint for a single recognized contribution.
    ///
    /// Writes an immutable `token_mint_events` record and credits the agent's
    /// balance in the given governance layer.
    pub fn mint_for_recognition(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        provenance_event_id: &str,
        source_epr_id: &str,
        event_type: &str,
        allocation_ratio: f32,
        governance_layer: &str,
    ) -> Result<TokenMintEventView, StorageError> {
        let amount = Self::calculate_micro_mint(event_type, allocation_ratio, None);

        if amount <= 0.0 {
            return Err(StorageError::InvalidInput(
                "mint amount must be positive".into(),
            ));
        }

        let mint_id = Self::generate_mint_id(provenance_event_id, agent_id);

        let new_mint = NewTokenMintEvent {
            id: &mint_id,
            h_app_id: &ctx.h_app_id,
            amount,
            provenance_event_id,
            mint_tier: "micro",
            source_epr_id,
            agent_id,
            constitutional_context: Some(governance_layer),
            elohim_attestation: None,
            reasoning_trace: None,
            dht_anchor_hash: None,
        };

        let mint = token_mint_events::create_mint_event(conn, ctx, new_mint)?;
        token_balances::credit_balance(
            conn,
            ctx,
            agent_id,
            governance_layer,
            amount,
            CreditSource::Mint,
        )?;
        Ok(TokenMintEventView::from(mint))
    }

    /// Generate a deterministic mint ID from provenance event and agent IDs.
    fn generate_mint_id(provenance_event_id: &str, agent_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provenance_event_id.as_bytes());
        hasher.update(agent_id.as_bytes());
        let hash = hasher.finalize();
        format!("mint-{}", hex::encode(&hash[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micro_mint_calculation() {
        let amount = TokenMintService::calculate_micro_mint("mastery-advance", 1.0, Some(1.0));
        assert!((amount - 1.0).abs() < f32::EPSILON);

        let amount = TokenMintService::calculate_micro_mint("content-view", 0.5, Some(1.0));
        assert!((amount - 0.005).abs() < f32::EPSILON);

        let amount = TokenMintService::calculate_micro_mint("citation", 0.3, Some(2.0));
        assert!((amount - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_event_weight_defaults() {
        assert!(
            (TokenMintService::event_weight("unknown-type") - 0.01).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_mint_id_deterministic() {
        let id1 = TokenMintService::generate_mint_id("event-123", "agent-abc");
        let id2 = TokenMintService::generate_mint_id("event-123", "agent-abc");
        assert_eq!(id1, id2);
        let id3 = TokenMintService::generate_mint_id("event-456", "agent-abc");
        assert_ne!(id1, id3);
    }
}
