//! Economic event service - business logic for REA economic event processing
//!
//! Encapsulates the staged transaction → validated event pipeline,
//! amount validation, and event type mapping.
//!
//! ## Architecture
//!
//! Controller (api/economic_events.rs) → **Service (this file)** → Model (db/economic_events.rs)

use std::collections::HashMap;

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
use crate::views::EconomicEventView;

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
