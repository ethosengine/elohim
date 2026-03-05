//! Economic Events API
//!
//! Route stubs for the IEconomicEventFactory contract. Transforms staged bank
//! transactions into immutable EconomicEvent ledger entries based on
//! REA/ValueFlows ontology.
//!
//! ## Routes
//!
//! - `POST /api/v1/economic-events/from-staged`       - Create event from a single staged transaction
//! - `POST /api/v1/economic-events/from-staged/bulk`   - Batch create events from multiple staged transactions
//! - `POST /api/v1/economic-events/correction`          - Create a correction event for an existing event
//!
//! ## Design
//!
//! Events are immutable. Errors are corrected by creating new correction events,
//! never by modifying existing ones. This mirrors the event-sourcing pattern used
//! throughout Elohim's accounting layer.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// REA/ValueFlows action describing what happened to a resource.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EconomicAction {
    Produce,
    Consume,
    Transfer,
    Use,
}

/// Processing status of an economic event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus {
    Pending,
    Validated,
    Countersigned,
    Disputed,
    Corrected,
}

/// Lifecycle state snapshot attached to an economic event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventState {
    pub status: EventStatus,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

/// Quantity measure for a resource flow (e.g. `{ value: 42.50, unit: "USD" }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMeasure {
    pub value: f64,
    pub unit: String,
}

/// AI categorization suggestion for a staged transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySuggestion {
    pub category: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub source: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Type of the original bank transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransactionType {
    Debit,
    Credit,
    Transfer,
    Fee,
}

/// Review status of a staged transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
    NeedsAttention,
}

/// A staged bank transaction awaiting conversion to an immutable EconomicEvent.
///
/// Mirrors the TypeScript `StagedTransaction` interface from
/// `elohim-app/src/app/shefa/models/transaction-import.model.ts`.
///
/// Fields use `camelCase` to match the TypeScript/JSON wire format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedTransaction {
    pub id: String,
    pub batch_id: String,
    pub steward_id: String,

    // Source tracking (critical for reconciliation)
    pub plaid_transaction_id: String,
    pub plaid_account_id: String,
    pub financial_asset_id: String,

    // Transaction data from Plaid
    pub timestamp: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: ResourceMeasure,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_name: Option<String>,

    // AI categorization
    pub category: String,
    pub category_confidence: f64,
    pub category_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_categories: Option<Vec<CategorySuggestion>>,

    // Budget linkage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_category_id: Option<String>,

    // Duplicate detection
    pub is_duplicate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of_transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_confidence: Option<f64>,

    // Review workflow
    pub review_status: ReviewStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,

    // Economic event (created after approval)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economic_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_state: Option<EventState>,

    // Preserve original Plaid JSON for debugging/reconciliation
    pub plaid_raw_data: HashMap<String, serde_json::Value>,

    // Audit trail
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// An immutable economic event record based on REA/ValueFlows.
///
/// Mirrors the TypeScript `EconomicEvent` interface from
/// `elohim-app/src/app/shefa/interfaces/economic-event-factory.interface.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomicEvent {
    pub id: String,
    pub event_type: String,
    pub timestamp: String,

    // Agents
    pub provider_id: String,
    pub receiver_id: String,

    // Resource flow
    pub quantity: f64,
    pub unit: String,
    pub action: EconomicAction,

    // Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    // Audit trail
    pub metadata: HashMap<String, serde_json::Value>,
    pub state: EventState,
    pub created_at: String,
    pub created_by: String,
}

/// Partial correction fields for an existing economic event.
///
/// Mirrors the `correction` parameter of `IEconomicEventFactory.createCorrectionEvent()`.
/// All fields are optional; only provided fields override the original event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request body for `POST /api/v1/economic-events/correction`.
///
/// Creates a new correction event that supersedes `original_event_id`.
/// In immutable event sourcing, errors are corrected by appending new events,
/// never by modifying existing ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRequest {
    pub original_event_id: String,
    pub correction: CorrectionFields,
    pub reason: String,
}

/// Wrapper for batch responses so the caller knows total vs. successful counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateResponse {
    /// Successfully created events.
    pub events: Vec<EconomicEvent>,
    /// Number of staged transactions that were submitted.
    pub submitted_count: usize,
    /// Number of events successfully created.
    pub created_count: usize,
    /// Number of staged transactions that were skipped (non-approved or failed).
    pub skipped_count: usize,
}

// ---------------------------------------------------------------------------
// Error helpers (local to this module)
// ---------------------------------------------------------------------------

/// Standard error envelope for economic event endpoints.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    error: String,
    code: &'static str,
}

fn error_response(status: StatusCode, message: &str, code: &'static str) -> Response<Full<Bytes>> {
    let body = serde_json::to_vec(&ApiError {
        error: message.to_string(),
        code,
    })
    .unwrap_or_default();

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Internal error","code":"INTERNAL_ERROR"}"#,
                )))
                .unwrap()
        })
}

#[allow(dead_code)] // Used once todo!() stubs are implemented
fn json_response(status: StatusCode, data: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Business logic helpers
// ---------------------------------------------------------------------------

fn determine_event_type(transaction_type: &TransactionType) -> &'static str {
    match transaction_type {
        TransactionType::Credit => "credit-transfer",
        TransactionType::Debit => "credit-transfer",
        TransactionType::Fee => "credit-retire",
        TransactionType::Transfer => "credit-transfer",
    }
}

fn determine_agents(staged: &StagedTransaction) -> (String, String) {
    match staged.transaction_type {
        TransactionType::Credit => ("external-party".to_string(), staged.steward_id.clone()),
        TransactionType::Debit => (
            staged.steward_id.clone(),
            staged
                .merchant_name
                .clone()
                .unwrap_or_else(|| "external-party".to_string()),
        ),
        TransactionType::Fee => (
            staged.steward_id.clone(),
            staged
                .merchant_name
                .clone()
                .unwrap_or_else(|| "fee-collector".to_string()),
        ),
        TransactionType::Transfer => (staged.steward_id.clone(), "external-account".to_string()),
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

    if staged.description != staged.merchant_name.as_deref().unwrap_or("") {
        parts.push(format!("({})", staged.description));
    }

    parts.push(format!("[Imported from {}]", staged.plaid_account_id));

    parts.join(" ")
}

fn build_event_metadata(staged: &StagedTransaction) -> HashMap<String, serde_json::Value> {
    let mut meta = HashMap::new();

    meta.insert(
        "plaidTransactionId".to_string(),
        serde_json::Value::String(staged.plaid_transaction_id.clone()),
    );
    meta.insert(
        "plaidAccountId".to_string(),
        serde_json::Value::String(staged.plaid_account_id.clone()),
    );
    meta.insert(
        "category".to_string(),
        serde_json::Value::String(staged.category.clone()),
    );
    meta.insert(
        "categoryConfidence".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(staged.category_confidence)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    meta.insert(
        "categorySource".to_string(),
        serde_json::Value::String(staged.category_source.clone()),
    );
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
    meta.insert(
        "importBatchId".to_string(),
        serde_json::Value::String(staged.batch_id.clone()),
    );
    meta.insert(
        "merchantName".to_string(),
        staged
            .merchant_name
            .as_deref()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    meta.insert(
        "plaidRawData".to_string(),
        serde_json::Value::Object(
            staged
                .plaid_raw_data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
    );
    meta.insert(
        "eventFactory".to_string(),
        serde_json::Value::String("doorway-economic-events".to_string()),
    );
    meta.insert(
        "stagedTransactionId".to_string(),
        serde_json::Value::String(staged.id.clone()),
    );

    meta
}

fn build_event_from_staged(staged: &StagedTransaction) -> EconomicEvent {
    let event_type = determine_event_type(&staged.transaction_type);
    let (provider_id, receiver_id) = determine_agents(staged);
    let action = determine_action(event_type);
    let note = build_event_note(staged);
    let metadata = build_event_metadata(staged);
    let now = chrono::Utc::now().to_rfc3339();

    // Generate a unique ID: event-{timestamp_ms}-{random_hex}
    let random_suffix: u64 = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        staged.id.hash(&mut h);
        now.hash(&mut h);
        h.finish()
    };
    let id = format!(
        "event-{}-{:x}",
        chrono::Utc::now().timestamp_millis(),
        random_suffix
    );

    EconomicEvent {
        id,
        event_type: event_type.to_string(),
        timestamp: staged.timestamp.clone(),
        provider_id,
        receiver_id,
        quantity: staged.amount.value,
        unit: staged.amount.unit.clone(),
        action,
        note: Some(note),
        metadata,
        state: EventState {
            status: EventStatus::Validated,
            timestamp: now.clone(),
            reason_code: Some("auto-imported".to_string()),
        },
        created_at: now,
        created_by: staged.steward_id.clone(),
    }
}

/// Persist an EconomicEvent to elohim-storage at POST /db/economic-events.
async fn persist_event(event: &EconomicEvent, storage_url: &str) -> Result<(), String> {
    let body = serde_json::to_vec(event).map_err(|e| format!("Serialize error: {e}"))?;
    let url = format!("{}/db/economic-events", storage_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Storage request failed: {e}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Storage returned {}: {}",
            response.status(),
            response
                .text()
                .await
                .unwrap_or_else(|_| "<no body>".to_string())
        ))
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// `POST /api/v1/economic-events/from-staged`
///
/// Create a single immutable EconomicEvent from an approved StagedTransaction.
///
/// # Request body
///
/// A JSON-encoded [`StagedTransaction`] whose `reviewStatus` is `"approved"`.
///
/// # Response
///
/// - `201 Created` with the [`EconomicEvent`] JSON body on success
/// - `400 Bad Request` if the staged transaction is not approved
/// - `409 Conflict` if an event already exists for this staged transaction
///
/// # Errors
///
/// The endpoint validates that:
/// 1. `reviewStatus == "approved"` -- non-approved transactions must not become events.
/// 2. `economicEventId` is `None` -- prevents duplicate event creation.
pub async fn handle_create_from_staged(
    body: Bytes,
    storage_url: Option<String>,
) -> Response<Full<Bytes>> {
    let staged: StagedTransaction = match serde_json::from_slice(&body) {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
                "INVALID_BODY",
            );
        }
    };

    // Validate: must be approved
    if staged.review_status != ReviewStatus::Approved {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Cannot create event from non-approved transaction",
            "NOT_APPROVED",
        );
    }

    // Validate: no event already created
    if staged.economic_event_id.is_some() {
        return error_response(
            StatusCode::CONFLICT,
            "Event already created for staged transaction",
            "EVENT_EXISTS",
        );
    }

    let event = build_event_from_staged(&staged);

    // Persist to storage if configured; otherwise return the constructed event
    if let Some(ref url) = storage_url {
        if let Err(e) = persist_event(&event, url).await {
            warn!(error = %e, staged_id = %staged.id, "Failed to persist economic event to storage");
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .header("Cache-Control", "no-cache")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"Storage unavailable: {e}","code":"STORAGE_ERROR"}}"#
                ))))
                .unwrap();
        }
    } else {
        info!(staged_id = %staged.id, "No storage_url configured — returning event without persisting");
    }

    match serde_json::to_vec(&event) {
        Ok(body) => {
            info!(event_id = %event.id, staged_id = %staged.id, "Created economic event from staged transaction");
            json_response(StatusCode::CREATED, body)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to serialize event: {e}"),
            "SERIALIZE_ERROR",
        ),
    }
}

/// `POST /api/v1/economic-events/from-staged/bulk`
///
/// Batch create EconomicEvents from multiple staged transactions.
///
/// # Request body
///
/// A JSON array of [`StagedTransaction`] objects. Non-approved transactions
/// are silently skipped. Individual failures do not abort the batch.
///
/// # Response
///
/// - `200 OK` with a [`BulkCreateResponse`] containing successfully created events
///   plus submission/skip counts.
pub async fn handle_create_multiple_from_staged(
    body: Bytes,
    storage_url: Option<String>,
) -> Response<Full<Bytes>> {
    let staged_list: Vec<StagedTransaction> = match serde_json::from_slice(&body) {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
                "INVALID_BODY",
            );
        }
    };

    let submitted_count = staged_list.len();
    let mut events: Vec<EconomicEvent> = Vec::new();
    let mut skipped_count: usize = 0;

    for staged in &staged_list {
        if staged.review_status != ReviewStatus::Approved {
            skipped_count += 1;
            continue;
        }

        let event = build_event_from_staged(staged);

        if let Some(ref url) = storage_url {
            match persist_event(&event, url).await {
                Ok(()) => events.push(event),
                Err(e) => {
                    warn!(error = %e, staged_id = %staged.id, "Skipping staged transaction — storage persist failed");
                    skipped_count += 1;
                }
            }
        } else {
            events.push(event);
        }
    }

    let created_count = events.len();
    let response = BulkCreateResponse {
        events,
        submitted_count,
        created_count,
        skipped_count,
    };

    info!(
        submitted = submitted_count,
        created = created_count,
        skipped = skipped_count,
        "Bulk economic event creation complete"
    );

    match serde_json::to_vec(&response) {
        Ok(body) => json_response(StatusCode::OK, body),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to serialize bulk response: {e}"),
            "SERIALIZE_ERROR",
        ),
    }
}

/// `POST /api/v1/economic-events/correction`
///
/// Create a correction event for an existing EconomicEvent.
///
/// In immutable event sourcing, errors are corrected by creating new events
/// that reference and supersede the original. The original event is marked
/// with state `corrected` but is never deleted.
///
/// # Request body
///
/// A JSON-encoded [`CorrectionRequest`] containing the original event ID,
/// the partial fields to override, and the reason for correction.
///
/// # Response
///
/// - `201 Created` with the correction [`EconomicEvent`] on success
/// - `404 Not Found` if the original event does not exist
/// - `400 Bad Request` if the request body is malformed
pub async fn handle_create_correction(body: Bytes) -> Response<Full<Bytes>> {
    let correction_req: CorrectionRequest = match serde_json::from_slice(&body) {
        Ok(c) => c,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
                "INVALID_BODY",
            );
        }
    };

    let _ = correction_req;
    Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"error":"Not yet implemented"}"#)))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Storage proxy helper
// ---------------------------------------------------------------------------

/// Forward an `/api/v1/economic-events/*` request to elohim-storage.
///
/// Maps the doorway path to the corresponding storage path:
/// - `/api/v1/economic-events`       → `/db/economic-events`
/// - `/api/v1/economic-events/bulk`  → `/db/economic-events/bulk`
async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    doorway_path: &str,
) -> Response<Full<Bytes>> {
    // Rewrite /api/v1/economic-events... → /db/economic-events...
    let storage_path = doorway_path.replacen("/api/v1/economic-events", "/db/economic-events", 1);

    let storage_endpoint = format!("{}{storage_path}", storage_url.trim_end_matches('/'));

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding economic-events to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        size = body.len(),
                        path = %doorway_path,
                        "Forwarded economic-events response"
                    );
                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Route Dispatcher
// ---------------------------------------------------------------------------

/// Handle all `/api/v1/economic-events/*` requests.
pub async fn handle_economic_events_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let sub_path = path
        .strip_prefix("/api/v1/economic-events")
        .unwrap_or(path)
        .trim_start_matches('/');

    match (&method, sub_path) {
        // POST /api/v1/economic-events → storage proxy
        (&Method::POST, "") => {
            let storage_url = match &state.args.storage_url {
                Some(url) => url.clone(),
                None => {
                    warn!("economic-events POST called but STORAGE_URL not configured");
                    return Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                        )))
                        .unwrap();
                }
            };
            forward_to_storage(req, &storage_url, path).await
        }

        // GET /api/v1/economic-events → storage proxy
        (&Method::GET, "") => {
            let storage_url = match &state.args.storage_url {
                Some(url) => url.clone(),
                None => {
                    warn!("economic-events GET called but STORAGE_URL not configured");
                    return Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                        )))
                        .unwrap();
                }
            };
            forward_to_storage(req, &storage_url, path).await
        }

        // POST /api/v1/economic-events/bulk → storage proxy
        (&Method::POST, "bulk") => {
            let storage_url = match &state.args.storage_url {
                Some(url) => url.clone(),
                None => {
                    warn!("economic-events bulk POST called but STORAGE_URL not configured");
                    return Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                        )))
                        .unwrap();
                }
            };
            forward_to_storage(req, &storage_url, path).await
        }

        // POST /api/v1/economic-events/from-staged
        (&Method::POST, "from-staged") => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Failed to read body: {e}"),
                        "BODY_READ_ERROR",
                    );
                }
            };
            handle_create_from_staged(body, state.args.storage_url.clone()).await
        }

        // POST /api/v1/economic-events/from-staged/bulk
        (&Method::POST, "from-staged/bulk") => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Failed to read body: {e}"),
                        "BODY_READ_ERROR",
                    );
                }
            };
            handle_create_multiple_from_staged(body, state.args.storage_url.clone()).await
        }

        // POST /api/v1/economic-events/correction
        (&Method::POST, "correction") => {
            let body = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Failed to read body: {e}"),
                        "BODY_READ_ERROR",
                    );
                }
            };
            handle_create_correction(body).await
        }

        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"Unknown economic-events route: {method} {path}"}}"#
            ))))
            .unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_transaction_deserializes_from_camel_case() {
        let json = r#"{
            "id": "st-001",
            "batchId": "batch-1",
            "stewardId": "agent-abc",
            "plaidTransactionId": "plaid-tx-1",
            "plaidAccountId": "plaid-acc-1",
            "financialAssetId": "fa-1",
            "timestamp": "2026-03-05T12:00:00Z",
            "type": "debit",
            "amount": { "value": 42.50, "unit": "USD" },
            "description": "Coffee shop",
            "merchantName": "Blue Bottle",
            "category": "Food & Drink",
            "categoryConfidence": 95.0,
            "categorySource": "ai",
            "isDuplicate": false,
            "reviewStatus": "approved",
            "plaidRawData": { "raw": true },
            "createdAt": "2026-03-05T12:00:00Z"
        }"#;

        let staged: StagedTransaction = serde_json::from_str(json).unwrap();
        assert_eq!(staged.id, "st-001");
        assert_eq!(staged.transaction_type, TransactionType::Debit);
        assert_eq!(staged.amount.value, 42.5);
        assert_eq!(staged.amount.unit, "USD");
        assert_eq!(staged.review_status, ReviewStatus::Approved);
        assert_eq!(staged.merchant_name, Some("Blue Bottle".to_string()));
        assert!(!staged.is_duplicate);
    }

    #[test]
    fn economic_event_serializes_to_camel_case() {
        let event = EconomicEvent {
            id: "evt-1".to_string(),
            event_type: "credit-transfer".to_string(),
            timestamp: "2026-03-05T12:00:00Z".to_string(),
            provider_id: "agent-a".to_string(),
            receiver_id: "agent-b".to_string(),
            quantity: 100.0,
            unit: "USD".to_string(),
            action: EconomicAction::Transfer,
            note: Some("Payment".to_string()),
            metadata: HashMap::new(),
            state: EventState {
                status: EventStatus::Validated,
                timestamp: "2026-03-05T12:00:00Z".to_string(),
                reason_code: None,
            },
            created_at: "2026-03-05T12:00:00Z".to_string(),
            created_by: "agent-a".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"eventType\""));
        assert!(json.contains("\"providerId\""));
        assert!(json.contains("\"receiverId\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"createdBy\""));
        // Verify snake_case never leaks
        assert!(!json.contains("\"event_type\""));
        assert!(!json.contains("\"provider_id\""));
    }

    #[test]
    fn correction_request_round_trips() {
        let req = CorrectionRequest {
            original_event_id: "evt-1".to_string(),
            correction: CorrectionFields {
                event_type: None,
                provider_id: None,
                receiver_id: Some("agent-c".to_string()),
                quantity: Some(200.0),
                unit: None,
                note: Some("Wrong receiver".to_string()),
                metadata: None,
            },
            reason: "data-error".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CorrectionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.original_event_id, "evt-1");
        assert_eq!(
            deserialized.correction.receiver_id,
            Some("agent-c".to_string())
        );
        assert_eq!(deserialized.correction.quantity, Some(200.0));
        assert_eq!(deserialized.reason, "data-error");
    }

    #[test]
    fn bulk_create_response_serializes() {
        let resp = BulkCreateResponse {
            events: vec![],
            submitted_count: 5,
            created_count: 3,
            skipped_count: 2,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"submittedCount\""));
        assert!(json.contains("\"createdCount\""));
        assert!(json.contains("\"skippedCount\""));
    }
}
