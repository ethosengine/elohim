//! Economic event service - business logic for REA economic event processing
//!
//! Encapsulates the staged transaction → validated event pipeline,
//! amount validation, and event type mapping.
//!
//! ## Architecture
//!
//! Controller (api/economic_events.rs) → **Service (this file)** → Model (db/economic_events.rs)
//!
//! ## Share-routing integration (M1 final)
//!
//! When an EconomicEvent is authored under a Collab-Qahal scope, the emission
//! path routes the event's value through the Agreement's `ShareAllocation`
//! and emits one Settlement EconomicEvent per `RoutedAmount`.
//!
//! **Status (M1 final):**
//! - DB column `scope_collab_cid`: LANDED in M1 Task 14 deepening
//!   (`migrations/2026-05-24-000000_economic_events_add_scope_collab_cid/`).
//! - Handler-level routing (`api/economic_events.rs` async fetch + bulk-emit): LANDED M1 Task 14.
//! - Full Agreement entry decode (`fetch_agreement_by_action_bytes` decodes
//!   `ZomeCollabAgreementEntry` via rmp_serde, parses `share_allocation_json`): LANDED M1 final.
//! - Remaining deferral: the CI-scope live conductor sweettest
//!   (`qahal_http_contract::economic_event_under_collab_routes_through_share_allocation`)
//!   stays `#[ignore]` — Che environment constraint, not a code gap.
//!
//! ### Historical wiring sketch (now active in api/economic_events.rs)
//!
//! ```rust,ignore
//! // Inside the emission entry point, after recording the primary event:
//! if let Some(ref collab_cid) = input.scope_collab_cid {
//!     let qahal   = self.qahal.fetch_collab_qahal(collab_cid).await?;
//!     let agreement_cid = &qahal.anchor_agreement_cid;
//!     let agreement = self.qahal.fetch_collab_agreement(agreement_cid).await?;
//!     let active_set: std::collections::HashSet<String> = qahal
//!         .member_collectives
//!         .iter()
//!         .map(|c| c.cid.clone())
//!         .collect();
//!     let inputs = route_economic_event_under_collab(
//!         &agreement,
//!         event_value,
//!         at_block_height,
//!         &active_set,
//!         &source_event_id,
//!     )?;
//!     bulk_record_events(conn, ctx, inputs)?;
//! }
//! ```

use std::collections::HashMap;
use std::collections::HashSet;

use chrono::Utc;
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};
use tracing::info;

use crate::db::economic_events::{
    bulk_record_events, get_economic_event, get_events_for_agent, get_events_for_content,
    list_economic_events, record_event, BulkEconomicEventResult, CreateEconomicEventInput,
    EconomicEventQuery,
};
use crate::db::models::EconomicEvent;
use crate::db::AppContext;
use crate::error::StorageError;
use crate::services::share_routing::evaluate_share_routing_active_only;
use crate::views::EconomicEventView;
use elohim_views::CollabAgreementView;

// ---------------------------------------------------------------------------
// Domain types (local to service layer)
// ---------------------------------------------------------------------------

/// REA/ValueFlows action describing what happened to a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicAction {
    Produce,
    Consume,
    Transfer,
    Use,
}

impl EconomicAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            EconomicAction::Produce => "produce",
            EconomicAction::Consume => "consume",
            EconomicAction::Transfer => "transfer",
            EconomicAction::Use => "use",
        }
    }
}

/// A staged bank transaction awaiting conversion to an immutable EconomicEvent.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedTransaction {
    pub id: String,
    pub batch_id: Option<String>,
    pub steward_id: String,
    pub plaid_transaction_id: Option<String>,
    pub plaid_account_id: Option<String>,
    pub transaction_type: String, // "debit", "credit", "fee", "transfer"
    pub amount: AmountValue,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub budget_id: Option<String>,
    pub budget_category_id: Option<String>,
    pub category_confidence: Option<f64>,
    pub category_source: Option<String>,
    pub review_status: String, // "approved", "pending", "rejected"
    pub economic_event_id: Option<String>,
    pub timestamp: Option<String>,
}

/// Quantity/value from a staged transaction.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmountValue {
    pub value: String,
    pub unit: String,
}

// ---------------------------------------------------------------------------
// Economic event service
// ---------------------------------------------------------------------------

/// Economic event service for event transformation pipeline
pub struct EconomicEventService;

impl EconomicEventService {
    // -----------------------------------------------------------------------
    // Query operations
    // -----------------------------------------------------------------------

    pub fn list_events(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        query: &EconomicEventQuery,
    ) -> Result<Vec<EconomicEventView>, StorageError> {
        let events = list_economic_events(conn, ctx, query)?;
        Ok(events.into_iter().map(EconomicEventView::from).collect())
    }

    pub fn get_event(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<EconomicEventView>, StorageError> {
        let event = get_economic_event(conn, ctx, id)?;
        Ok(event.map(EconomicEventView::from))
    }

    pub fn events_for_agent(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<EconomicEventView>, StorageError> {
        let events = get_events_for_agent(conn, ctx, agent_id, limit)?;
        Ok(events.into_iter().map(EconomicEventView::from).collect())
    }

    pub fn events_for_content(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        content_id: &str,
    ) -> Result<Vec<EconomicEventView>, StorageError> {
        let events = get_events_for_content(conn, ctx, content_id)?;
        Ok(events.into_iter().map(EconomicEventView::from).collect())
    }

    // -----------------------------------------------------------------------
    // Create operations
    // -----------------------------------------------------------------------

    pub fn create_event(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateEconomicEventInput,
    ) -> Result<EconomicEventView, StorageError> {
        let event = record_event(conn, ctx, input)?;
        Ok(EconomicEventView::from(event))
    }

    pub fn bulk_create_events(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        inputs: Vec<CreateEconomicEventInput>,
    ) -> Result<BulkEconomicEventResult, StorageError> {
        bulk_record_events(conn, ctx, inputs)
    }

    // -----------------------------------------------------------------------
    // Staged transaction pipeline (7-step)
    // -----------------------------------------------------------------------

    /// Build and persist an economic event from an approved staged transaction.
    ///
    /// Steps:
    /// 1. Validate review_status == "approved"
    /// 2. Validate amount.value is valid f64, unit is non-empty
    /// 3. Determine event_type from transaction_type
    /// 4. Determine provider/receiver agents
    /// 5. Determine REA action from event_type
    /// 6. Build note and metadata
    /// 7. Generate ID (event-{timestamp_ms}-{hash_suffix}) and persist
    pub fn build_event_from_staged(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        staged: &StagedTransaction,
    ) -> Result<EconomicEventView, StorageError> {
        // Step 1: validate review status
        if staged.review_status != "approved" {
            return Err(StorageError::InvalidInput(
                "Cannot create event from non-approved transaction".to_string(),
            ));
        }

        // Step 2: validate amount
        let amount_value = Self::validate_amount(&staged.amount)?;

        // Step 3: determine event type
        let event_type = Self::determine_event_type(&staged.transaction_type);

        // Step 4: determine agents
        let (provider_id, receiver_id) = Self::determine_agents(staged);

        // Step 5: determine REA action
        let action = Self::determine_action(event_type);

        // Step 6: build note and metadata
        let note = Self::build_event_note(staged);
        let metadata = Self::build_event_metadata(staged);
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| StorageError::Internal(format!("Metadata serialization failed: {}", e)))?;

        // Step 7: generate ID and persist
        let now = Utc::now();
        let id = Self::generate_event_id(&staged.id, now.timestamp_millis());
        let timestamp = staged.timestamp.clone().unwrap_or_else(|| now.to_rfc3339());

        info!(event_id = %id, staged_id = %staged.id, "Building economic event from staged transaction");

        let input = CreateEconomicEventInput {
            id: Some(id),
            action: action.as_str().to_string(),
            provider: provider_id,
            receiver: receiver_id,
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as: vec![event_type.to_string()],
            resource_quantity_value: Some(amount_value as f32),
            resource_quantity_unit: Some(staged.amount.unit.clone()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: Some(timestamp),
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: staged.id.clone().into(),
            note: Some(note),
            metadata_json: Some(metadata_json),
            at_location: None,
            scope_collab_cid: None,
        };

        let event = record_event(conn, ctx, input)?;
        Ok(EconomicEventView::from(event))
    }

    /// Build events from a list of staged transactions (bulk from-staged).
    /// Skips non-approved transactions silently.
    /// Returns (created events, submitted count, skipped count).
    pub fn bulk_from_staged(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        staged_list: Vec<StagedTransaction>,
    ) -> Result<(Vec<EconomicEventView>, u64, u64), StorageError> {
        let submitted = staged_list.len() as u64;
        let mut created = Vec::new();
        let mut skipped: u64 = 0;

        for staged in &staged_list {
            if staged.review_status != "approved" {
                skipped += 1;
                continue;
            }

            match Self::build_event_from_staged(conn, ctx, staged) {
                Ok(view) => created.push(view),
                Err(e) => {
                    tracing::warn!(staged_id = %staged.id, error = %e, "Skipping staged transaction — build failed");
                    skipped += 1;
                }
            }
        }

        Ok((created, submitted, skipped))
    }

    // -----------------------------------------------------------------------
    // Business logic helpers
    // -----------------------------------------------------------------------

    fn determine_event_type(transaction_type: &str) -> &'static str {
        match transaction_type {
            "credit" => "credit-transfer",
            "debit" => "credit-transfer",
            "fee" => "credit-retire",
            "transfer" => "credit-transfer",
            _ => "credit-transfer",
        }
    }

    fn determine_agents(staged: &StagedTransaction) -> (String, String) {
        match staged.transaction_type.as_str() {
            "credit" => ("external-party".to_string(), staged.steward_id.clone()),
            "debit" => (
                staged.steward_id.clone(),
                staged
                    .merchant_name
                    .clone()
                    .unwrap_or_else(|| "external-party".to_string()),
            ),
            "fee" => (
                staged.steward_id.clone(),
                staged
                    .merchant_name
                    .clone()
                    .unwrap_or_else(|| "fee-collector".to_string()),
            ),
            "transfer" => (staged.steward_id.clone(), "external-account".to_string()),
            _ => (staged.steward_id.clone(), "external-party".to_string()),
        }
    }

    fn determine_action(event_type: &str) -> EconomicAction {
        match event_type {
            "credit-transfer" => EconomicAction::Transfer,
            "credit-retire" => EconomicAction::Consume,
            "credit-produce" => EconomicAction::Produce,
            "credit-use" => EconomicAction::Use,
            _ => EconomicAction::Transfer,
        }
    }

    fn build_event_note(staged: &StagedTransaction) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(ref merchant) = staged.merchant_name {
            parts.push(merchant.clone());
        }

        if let Some(ref desc) = staged.description {
            let merchant_str = staged.merchant_name.as_deref().unwrap_or("");
            if desc != merchant_str {
                parts.push(format!("({})", desc));
            }
        }

        if let Some(ref account_id) = staged.plaid_account_id {
            parts.push(format!("[Imported from {}]", account_id));
        }

        parts.join(" ")
    }

    fn build_event_metadata(staged: &StagedTransaction) -> HashMap<String, serde_json::Value> {
        let mut meta = HashMap::new();

        if let Some(ref tx_id) = staged.plaid_transaction_id {
            meta.insert(
                "plaidTransactionId".to_string(),
                serde_json::Value::String(tx_id.clone()),
            );
        }

        if let Some(ref account_id) = staged.plaid_account_id {
            meta.insert(
                "plaidAccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        if let Some(ref category) = staged.category {
            meta.insert(
                "category".to_string(),
                serde_json::Value::String(category.clone()),
            );
        }

        if let Some(confidence) = staged.category_confidence {
            meta.insert(
                "categoryConfidence".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(confidence).unwrap_or(serde_json::Number::from(0)),
                ),
            );
        }

        if let Some(ref source) = staged.category_source {
            meta.insert(
                "categorySource".to_string(),
                serde_json::Value::String(source.clone()),
            );
        }

        meta.insert(
            "budgetId".to_string(),
            staged
                .budget_id
                .as_deref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );

        meta.insert(
            "budgetCategoryId".to_string(),
            staged
                .budget_category_id
                .as_deref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );

        meta.insert(
            "source".to_string(),
            serde_json::Value::String("plaid-import".to_string()),
        );

        if let Some(ref batch_id) = staged.batch_id {
            meta.insert(
                "importBatchId".to_string(),
                serde_json::Value::String(batch_id.clone()),
            );
        }

        meta.insert(
            "merchantName".to_string(),
            staged
                .merchant_name
                .as_deref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );

        meta.insert(
            "eventFactory".to_string(),
            serde_json::Value::String("elohim-storage-economic-events".to_string()),
        );

        meta.insert(
            "stagedTransactionId".to_string(),
            serde_json::Value::String(staged.id.clone()),
        );

        meta
    }

    fn validate_amount(amount: &AmountValue) -> Result<f64, StorageError> {
        if amount.unit.is_empty() {
            return Err(StorageError::InvalidInput(
                "Amount unit must be present".to_string(),
            ));
        }

        amount.value.parse::<f64>().map_err(|_| {
            StorageError::InvalidInput(format!(
                "Invalid amount value '{}': must be a valid number",
                amount.value
            ))
        })
    }

    fn generate_event_id(staged_id: &str, timestamp_ms: i64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(staged_id.as_bytes());
        hasher.update(timestamp_ms.to_le_bytes());
        let hash = hasher.finalize();
        let hex_suffix = hex::encode(&hash[..4]);
        format!("event-{}-{}", timestamp_ms, hex_suffix)
    }

    // -----------------------------------------------------------------------
    // DB model helper
    // -----------------------------------------------------------------------

    pub fn to_view(event: EconomicEvent) -> EconomicEventView {
        EconomicEventView::from(event)
    }
}

// ---------------------------------------------------------------------------
// Share-routing integration (M1 Task 14)
// ---------------------------------------------------------------------------
//
// Pure function: given a CollabAgreementView, an event value, the block height
// at which the event was authored, and the set of currently-active Collective
// CIDs (non-withdrawn), produce a Vec<CreateEconomicEventInput> — one Settlement
// event per RoutedAmount from the evaluator.
//
// This function IS the wiring point.  It does not touch the DB itself (that is
// the caller's responsibility) so that it remains testable without a connection.
//
// ## Why it lives here and not in share_routing.rs
//
// share_routing.rs is a pure-function evaluator that does not know about DB
// types.  The mapping from RoutedAmount → CreateEconomicEventInput is an
// EconomicEventService concern (same layer that owns build_event_from_staged).
//
// ## M2 activation gate
//
// Before this function can be called from the live emission path:
//   1. DB migration: add `scope_collab_cid TEXT` to the `economic_events` table
//      and `CreateEconomicEventInput`.
//   2. `EconomicEventService` must accept a `&QahalService` borrow (or
//      `Arc<QahalService>`) so it can call `fetch_collab_qahal` + fetch the
//      Agreement's `share_allocation`.
//   3. The emission entry point (e.g. `create_event`) checks
//      `input.scope_collab_cid` and, if set, calls this function + bulk-records.
//
// For M1, this function exercises the evaluator via unit test; it does NOT
// appear in any live HTTP call path.
pub fn route_economic_event_under_collab(
    agreement: &CollabAgreementView,
    event_value: f64,
    at_block_height: u64,
    active_collectives: &HashSet<String>,
    source_event_id: &str,
) -> Result<Vec<CreateEconomicEventInput>, StorageError> {
    let routed = evaluate_share_routing_active_only(
        &agreement.share_allocation,
        event_value,
        at_block_height,
        active_collectives,
    )?;

    let now = Utc::now().to_rfc3339();
    let mut inputs = Vec::with_capacity(routed.len());

    for amount in &routed {
        let settlement_id =
            generate_settlement_id(source_event_id, &amount.collective_cid, at_block_height);

        // Extensions block: annotates which Agreement drove this settlement and
        // what the commons-pool tribute was.  Stored as metadata_json in the DB.
        let extensions = serde_json::json!({
            "elohim": {
                "allocatingAgreement": agreement.cid,
                "commonsPoolTribute": agreement.commons_pool_tribute,
                "sourceEventId": source_event_id,
                "scopeCollabCid": agreement.collab_qahal_cid,
            }
        });
        let metadata_json = serde_json::to_string(&extensions).map_err(|e| {
            StorageError::Internal(format!("settlement extensions serialization: {e}"))
        })?;

        inputs.push(CreateEconomicEventInput {
            id: Some(settlement_id),
            // ValueFlows action "transfer" — value moves from the Collab scope
            // into the beneficiary Collective's account.
            action: "transfer".to_string(),
            // Provider is the Collab-Qahal (the scope that generated the value).
            provider: agreement
                .collab_qahal_cid
                .clone()
                .unwrap_or_else(|| agreement.cid.clone()),
            // Receiver is the routed beneficiary Collective (or "commons-pool").
            receiver: amount.collective_cid.clone(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as: vec!["settlement".to_string()],
            resource_quantity_value: Some(amount.amount as f32),
            resource_quantity_unit: Some("credit".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: Some(now.clone()),
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: Some(source_event_id.to_string()),
            note: Some(format!(
                "Settlement: {:.4} routed to {} via agreement {}",
                amount.amount, amount.collective_cid, agreement.cid
            )),
            metadata_json: Some(metadata_json),
            at_location: None,
            // Settlements are always scoped to the Collab-Qahal they came from;
            // the scope is recorded in extensions.elohim.scopeCollabCid in metadata_json.
            scope_collab_cid: None,
        });
    }

    Ok(inputs)
}

/// Generate a deterministic Settlement event ID from the source event, beneficiary, and block height.
fn generate_settlement_id(
    source_event_id: &str,
    collective_cid: &str,
    at_block_height: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_event_id.as_bytes());
    hasher.update(collective_cid.as_bytes());
    hasher.update(at_block_height.to_le_bytes());
    let hash = hasher.finalize();
    let hex_suffix = hex::encode(&hash[..4]);
    format!("settlement-{}-{}", at_block_height, hex_suffix)
}

// ---------------------------------------------------------------------------
// Unit tests for share-routing integration
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;
    use elohim_views::{
        CollabAgreementStatus, CollabAgreementView, DeclaredShare, ElohimTier, ShareAllocation,
        ShareAllocationForm,
    };

    fn make_agreement(tribute: f64, shares: Vec<DeclaredShare>) -> CollabAgreementView {
        CollabAgreementView {
            cid: "agreement:test".into(),
            authored_by_agent_cid: "agent:founder".into(),
            participants: vec!["collective:a".into(), "collective:b".into()],
            scope: "test".into(),
            share_allocation: ShareAllocation {
                form: ShareAllocationForm::Declared,
                shares: Some(shares),
                affinity_window_blocks: None,
                rebalance_cadence_blocks: None,
                commons_pool_tribute: tribute,
            },
            commons_pool_tribute: tribute,
            governance_terms: None,
            initial_tier: ElohimTier::T0,
            created_at_block_height: 0,
            status: CollabAgreementStatus::Instantiated,
            attested_by: vec![],
            collab_qahal_cid: Some("collective:qahal".into()),
        }
    }

    #[test]
    fn route_economic_event_two_collectives_t0() {
        let agreement = make_agreement(
            0.05,
            vec![
                DeclaredShare {
                    collective_cid: "collective:a".into(),
                    share: 0.475,
                },
                DeclaredShare {
                    collective_cid: "collective:b".into(),
                    share: 0.475,
                },
            ],
        );
        let active: HashSet<String> = vec!["collective:a".into(), "collective:b".into()]
            .into_iter()
            .collect();

        let inputs =
            route_economic_event_under_collab(&agreement, 1000.0, 100, &active, "event-source-1")
                .expect("routing must succeed for valid T0 setup");

        // Three settlement events: A, B, commons-pool
        assert_eq!(inputs.len(), 3, "expected 3 settlement events");

        let by_receiver: HashMap<_, _> = inputs
            .iter()
            .map(|i| (i.receiver.clone(), i.resource_quantity_value.unwrap_or(0.0)))
            .collect();

        let a = *by_receiver
            .get("collective:a")
            .expect("collective:a must receive a settlement") as f64;
        let b = *by_receiver
            .get("collective:b")
            .expect("collective:b must receive a settlement") as f64;
        let commons = *by_receiver
            .get("commons-pool")
            .expect("commons-pool tribute must be emitted") as f64;

        assert!(
            (a - 475.0).abs() < 0.1,
            "collective:a should get ~475, got {a}"
        );
        assert!(
            (b - 475.0).abs() < 0.1,
            "collective:b should get ~475, got {b}"
        );
        assert!(
            (commons - 50.0).abs() < 0.1,
            "commons-pool should get ~50, got {commons}"
        );
        assert!((a + b + commons - 1000.0).abs() < 0.1, "sum must be ~1000");
    }

    #[test]
    fn route_economic_event_withdrawn_member_flows_to_commons() {
        let agreement = make_agreement(
            0.05,
            vec![
                DeclaredShare {
                    collective_cid: "collective:a".into(),
                    share: 0.475,
                },
                DeclaredShare {
                    collective_cid: "collective:b".into(),
                    share: 0.475,
                },
            ],
        );
        // collective:a has withdrawn; only B is in the active set
        let active: HashSet<String> = vec!["collective:b".into()].into_iter().collect();

        let inputs =
            route_economic_event_under_collab(&agreement, 1000.0, 200, &active, "event-source-2")
                .expect("routing must succeed even with one withdrawn member");

        let by_receiver: HashMap<_, _> = inputs
            .iter()
            .map(|i| (i.receiver.clone(), i.resource_quantity_value.unwrap_or(0.0)))
            .collect();

        // A's 47.5% flows to commons-pool (no re-normalization per spec §6.4)
        let b = *by_receiver.get("collective:b").unwrap_or(&0.0) as f64;
        let commons = *by_receiver.get("commons-pool").unwrap_or(&0.0) as f64;

        assert!(
            !by_receiver.contains_key("collective:a"),
            "withdrawn collective:a must not receive settlement"
        );
        assert!((b - 475.0).abs() < 0.1, "active collective:b gets ~475");
        // commons = base tribute (5%) + withdrawn A share (47.5%)
        assert!(
            (commons - 525.0).abs() < 0.1,
            "commons-pool absorbs withdrawn share: ~525, got {commons}"
        );
    }

    #[test]
    fn route_economic_event_settlement_ids_are_deterministic() {
        let agreement = make_agreement(
            0.05,
            vec![
                DeclaredShare {
                    collective_cid: "collective:a".into(),
                    share: 0.475,
                },
                DeclaredShare {
                    collective_cid: "collective:b".into(),
                    share: 0.475,
                },
            ],
        );
        let active: HashSet<String> = vec!["collective:a".into(), "collective:b".into()]
            .into_iter()
            .collect();

        let inputs1 =
            route_economic_event_under_collab(&agreement, 1000.0, 100, &active, "event-source-3")
                .unwrap();
        let inputs2 =
            route_economic_event_under_collab(&agreement, 1000.0, 100, &active, "event-source-3")
                .unwrap();

        let ids1: Vec<_> = inputs1.iter().map(|i| i.id.clone()).collect();
        let ids2: Vec<_> = inputs2.iter().map(|i| i.id.clone()).collect();
        assert_eq!(
            ids1, ids2,
            "settlement IDs must be deterministic for same inputs"
        );
    }

    #[test]
    fn route_economic_event_metadata_carries_agreement_cid() {
        let agreement = make_agreement(
            0.05,
            vec![
                DeclaredShare {
                    collective_cid: "collective:a".into(),
                    share: 0.475,
                },
                DeclaredShare {
                    collective_cid: "collective:b".into(),
                    share: 0.475,
                },
            ],
        );
        let active: HashSet<String> = vec!["collective:a".into(), "collective:b".into()]
            .into_iter()
            .collect();

        let inputs =
            route_economic_event_under_collab(&agreement, 1000.0, 100, &active, "event-source-4")
                .unwrap();

        for input in &inputs {
            let meta: serde_json::Value =
                serde_json::from_str(input.metadata_json.as_deref().unwrap_or("{}"))
                    .expect("metadata_json must be valid JSON");
            let alloc_agreement = meta["elohim"]["allocatingAgreement"]
                .as_str()
                .expect("extensions.elohim.allocatingAgreement must be present");
            assert_eq!(alloc_agreement, "agreement:test");
        }
    }
}
