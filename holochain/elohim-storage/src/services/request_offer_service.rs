//! Request/Offer service - business logic for compound service request operations
//!
//! Encapsulates the 3-write compound operations (request + intent + event),
//! ownership validation, status transitions, and matching logic.
//! Uses Diesel transactions to ensure atomicity.
//!
//! ## Architecture
//!
//! Controller (api/requests_offers.rs) → **Service (this file)** → Model (db/economic_events.rs)

use chrono::Utc;
use diesel::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::db::economic_events::{
    get_economic_event, list_economic_events, record_event, update_event_state,
    CreateEconomicEventInput, EconomicEventQuery,
};
use crate::db::models::EconomicEvent;
use crate::db::{AppContext, PooledConn};
use crate::error::StorageError;

// ============================================================================
// Domain Types (camelCase serde for API boundary)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub flexible_dates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetInfo {
    pub amount: f64,
    pub unit: String,
    pub medium_of_exchange_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateInfo {
    pub amount: f64,
    pub unit: String,
    pub per: String,
    pub medium_of_exchange_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRequest {
    pub id: String,
    pub request_number: Option<String>,
    pub requester_id: String,
    pub title: String,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Vec<String>,
    pub interaction_type: Option<String>,
    pub budget: Option<BudgetInfo>,
    pub status: String,
    pub matched_offer_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestInput {
    pub requester_id: String,
    pub title: String,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Vec<String>,
    pub interaction_type: Option<String>,
    pub budget: Option<BudgetInfo>,
    pub estimated_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequestInput {
    pub requester_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Option<Vec<String>>,
    pub interaction_type: Option<String>,
    pub budget: Option<BudgetInfo>,
    pub estimated_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOffer {
    pub id: String,
    pub offer_number: Option<String>,
    pub offeror_id: String,
    pub title: String,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Vec<String>,
    pub interaction_type: Option<String>,
    pub rate: Option<RateInfo>,
    pub status: String,
    pub matched_request_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOfferInput {
    pub offeror_id: String,
    pub title: String,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Vec<String>,
    pub interaction_type: Option<String>,
    pub rate: Option<RateInfo>,
    pub hours_per_week: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOfferInput {
    pub offeror_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub contact_preference: Option<String>,
    pub date_range: Option<DateRange>,
    pub service_type_ids: Option<Vec<String>>,
    pub interaction_type: Option<String>,
    pub rate: Option<RateInfo>,
    pub hours_per_week: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMatch {
    pub request_id: String,
    pub offer_id: String,
    pub match_quality: f64,
    pub shared_service_types: Vec<String>,
    pub time_compatible: bool,
    pub exchange_compatible: bool,
    pub status: String,
}

// Composite intent type (stored as part of metadata in the event)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaIntent {
    pub id: String,
    pub action: String,
    pub provider: Option<String>,
    pub resource_conforms_to: Option<String>,
    pub effort_quantity: Option<f64>,
    pub effort_unit: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub finished: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestResponse {
    pub request: ServiceRequest,
    pub intent: ReaIntent,
    pub created_event: EconomicEventView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOfferResponse {
    pub offer: ServiceOffer,
    pub intent: ReaIntent,
    pub created_event: EconomicEventView,
}

/// View of an EconomicEvent for API responses (camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomicEventView {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub lamad_event_type: Option<String>,
    pub has_point_in_time: String,
    pub state: String,
    pub note: Option<String>,
    pub metadata: Option<Value>,
}

impl From<EconomicEvent> for EconomicEventView {
    fn from(e: EconomicEvent) -> Self {
        let metadata = e
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        Self {
            id: e.id,
            action: e.action,
            provider: e.provider,
            receiver: e.receiver,
            lamad_event_type: e.lamad_event_type,
            has_point_in_time: e.has_point_in_time,
            state: e.state,
            note: e.note,
            metadata,
        }
    }
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestQuery {
    pub requester_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferQuery {
    pub offeror_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Resource classification tags for requests/offers
// (We cannot use custom lamad_event_types since they're validated against a
//  fixed allow-list in db/models.rs — we use resource_classified_as instead)
// ============================================================================

const TAG_SERVICE_REQUEST: &str = "service-request";
const TAG_SERVICE_OFFER: &str = "service-offer";
const TAG_SERVICE_REQUEST_INTENT: &str = "service-request-intent";
const TAG_SERVICE_OFFER_INTENT: &str = "service-offer-intent";
const TAG_SERVICE_REQUEST_LIFECYCLE: &str = "service-request-lifecycle";
const TAG_SERVICE_OFFER_LIFECYCLE: &str = "service-offer-lifecycle";

// ============================================================================
// Helper: deserialize ServiceRequest/ServiceOffer from economic event metadata
// ============================================================================

fn request_from_event(event: &EconomicEvent) -> Option<ServiceRequest> {
    event
        .metadata_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
}

fn offer_from_event(event: &EconomicEvent) -> Option<ServiceOffer> {
    event
        .metadata_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
}

/// Check if this event represents a service request listing
fn is_service_request_event(event: &EconomicEvent) -> bool {
    if let Some(ref tags_json) = event.resource_classified_as_json {
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(tags_json) {
            return tags.iter().any(|t| t == TAG_SERVICE_REQUEST);
        }
    }
    false
}

/// Check if this event represents a service offer listing
fn is_service_offer_event(event: &EconomicEvent) -> bool {
    if let Some(ref tags_json) = event.resource_classified_as_json {
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(tags_json) {
            return tags.iter().any(|t| t == TAG_SERVICE_OFFER);
        }
    }
    false
}

// ============================================================================
// ID generation
// ============================================================================

fn generate_request_id() -> String {
    let ts = Utc::now().timestamp_millis();
    let hash = &Uuid::new_v4().to_string().replace('-', "")[..8];
    format!("req-{}-{}", ts, hash)
}

fn generate_offer_id() -> String {
    let ts = Utc::now().timestamp_millis();
    let hash = &Uuid::new_v4().to_string().replace('-', "")[..8];
    format!("offer-{}-{}", ts, hash)
}

fn generate_intent_id(prefix: &str) -> String {
    let ts = Utc::now().timestamp_millis();
    let hash = &Uuid::new_v4().to_string().replace('-', "")[..8];
    format!("intent-{}-{}-{}", prefix, ts, hash)
}

// ============================================================================
// Service implementation
// ============================================================================

/// Request/Offer service for compound REA operations
pub struct RequestOfferService;

impl RequestOfferService {
    // -------------------------------------------------------------------------
    // REQUESTS
    // -------------------------------------------------------------------------

    /// List service requests with optional filters
    pub fn list_requests(
        conn: &mut PooledConn,
        ctx: &AppContext,
        query: &RequestQuery,
    ) -> Result<Vec<ServiceRequest>, StorageError> {
        // Query events by action "use" (requests), optionally filtered by provider
        let event_query = EconomicEventQuery {
            action: Some("use".to_string()),
            provider: query.requester_id.clone(),
            limit: query.limit.unwrap_or(500),
            offset: query.offset.unwrap_or(0),
            ..Default::default()
        };

        let events = list_economic_events(conn, ctx, &event_query)?;
        let mut requests: Vec<ServiceRequest> = events
            .into_iter()
            .filter(is_service_request_event)
            .filter_map(|e| request_from_event(&e))
            .collect();

        if let Some(ref status_filter) = query.status {
            requests.retain(|r| &r.status == status_filter);
        }

        Ok(requests)
    }

    /// Get a single service request by ID
    pub fn get_request(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ServiceRequest>, StorageError> {
        // Look up the event directly by its derived event ID
        let event_id = format!("evt-{}", id);
        if let Some(event) = get_economic_event(conn, ctx, &event_id)? {
            if is_service_request_event(&event) {
                return Ok(request_from_event(&event));
            }
        }
        Ok(None)
    }

    /// Create a service request — compound 3-write in a single transaction
    pub fn create_request(
        conn: &mut PooledConn,
        ctx: &AppContext,
        input: CreateRequestInput,
    ) -> Result<CreateRequestResponse, StorageError> {
        if input.requester_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "requesterId must not be empty".into(),
            ));
        }
        if input.title.trim().is_empty() {
            return Err(StorageError::InvalidInput("title must not be empty".into()));
        }

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let request_id = generate_request_id();
        let intent_id = generate_intent_id("req");

        // Build the request object
        let request = ServiceRequest {
            id: request_id.clone(),
            request_number: Some(format!("REQ-{}", &request_id[4..12])),
            requester_id: input.requester_id.clone(),
            title: input.title.clone(),
            description: input.description.clone(),
            contact_preference: input.contact_preference.clone(),
            date_range: input.date_range.clone(),
            service_type_ids: input.service_type_ids.clone(),
            interaction_type: input.interaction_type.clone(),
            budget: input.budget.clone(),
            status: "active".to_string(),
            matched_offer_ids: vec![],
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        // Build the REA intent
        let resource_conforms_to = input.service_type_ids.first().cloned();
        let intent = ReaIntent {
            id: intent_id.clone(),
            action: "use".to_string(),
            provider: None,
            resource_conforms_to: resource_conforms_to.clone(),
            effort_quantity: input.estimated_hours,
            effort_unit: input.estimated_hours.map(|_| "hours".to_string()),
            has_beginning: input
                .date_range
                .as_ref()
                .and_then(|dr| dr.start_date.clone()),
            has_end: input.date_range.as_ref().and_then(|dr| dr.end_date.clone()),
            finished: false,
            note: input.description.clone(),
        };

        conn.transaction::<_, StorageError, _>(|conn| {
            // Write 1: Store the service request as a "service-request" event
            let request_json = serde_json::to_string(&request)
                .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

            let req_event_input = CreateEconomicEventInput {
                id: Some(format!("evt-{}", request_id)),
                action: "use".to_string(),
                provider: input.requester_id.clone(),
                receiver: input.requester_id.clone(),
                resource_conforms_to: resource_conforms_to.clone(),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_REQUEST.to_string()],
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: input.estimated_hours.map(|h| h as f32),
                effort_quantity_unit: input.estimated_hours.map(|_| "hours".to_string()),
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: None,
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                note: input.description.clone(),
                metadata_json: Some(request_json),
            };
            record_event(conn, ctx, req_event_input)?;

            // Write 2: Store the REA intent as a separate event
            let intent_json = serde_json::to_string(&intent)
                .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

            let intent_event_input = CreateEconomicEventInput {
                id: Some(format!("evt-{}", intent_id)),
                action: "use".to_string(),
                provider: input.requester_id.clone(),
                receiver: input.requester_id.clone(),
                resource_conforms_to: resource_conforms_to.clone(),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_REQUEST_INTENT.to_string()],
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: input.estimated_hours.map(|h| h as f32),
                effort_quantity_unit: input.estimated_hours.map(|_| "hours".to_string()),
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: Some(format!("evt-{}", request_id)),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: Some(format!("evt-{}", request_id)),
                note: Some(format!("Intent for request {}", request_id)),
                metadata_json: Some(intent_json),
            };
            record_event(conn, ctx, intent_event_input)?;

            // Write 3: Create initial economic event (the "created" lifecycle event)
            let created_event_id = Uuid::new_v4().to_string();
            let created_event_meta = serde_json::json!({
                "listingId": request_id,
                "listingType": "request",
                "action": "created"
            });

            let creation_event_input = CreateEconomicEventInput {
                id: Some(created_event_id.clone()),
                action: "produce".to_string(),
                provider: input.requester_id.clone(),
                receiver: input.requester_id.clone(),
                resource_conforms_to: Some("service-listing".to_string()),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_REQUEST_LIFECYCLE.to_string()],
                resource_quantity_value: Some(1.0),
                resource_quantity_unit: Some("listing".to_string()),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: Some(format!("evt-{}", request_id)),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                note: Some(format!("Service request created: {}", request.title)),
                metadata_json: Some(created_event_meta.to_string()),
            };
            let created_event = record_event(conn, ctx, creation_event_input)?;

            Ok(CreateRequestResponse {
                request,
                intent,
                created_event: EconomicEventView::from(created_event),
            })
        })
    }

    /// Update a service request — validates ownership and merges fields
    pub fn update_request(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        input: UpdateRequestInput,
    ) -> Result<ServiceRequest, StorageError> {
        // Find the existing request
        let existing = Self::get_request(conn, ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("Request {} not found", id)))?;

        // Ownership validation
        if existing.requester_id != input.requester_id {
            return Err(StorageError::Auth(
                "Only the requester can update this request".into(),
            ));
        }

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Merge fields
        let updated = ServiceRequest {
            id: existing.id.clone(),
            request_number: existing.request_number.clone(),
            requester_id: existing.requester_id.clone(),
            title: input.title.unwrap_or(existing.title),
            description: input.description.or(existing.description),
            contact_preference: input.contact_preference.or(existing.contact_preference),
            date_range: input.date_range.or(existing.date_range),
            service_type_ids: input.service_type_ids.unwrap_or(existing.service_type_ids),
            interaction_type: input.interaction_type.or(existing.interaction_type),
            budget: input.budget.or(existing.budget),
            status: existing.status,
            matched_offer_ids: existing.matched_offer_ids,
            created_at: existing.created_at,
            updated_at: now.clone(),
        };

        let updated_json = serde_json::to_string(&updated)
            .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

        // Update the stored event's metadata by creating a new "update" event
        // and pointing the original event to archived
        let event_id = format!("evt-{}", id);

        // Update state on original event to reflect change
        update_event_state(conn, ctx, &event_id, "updated")?;

        // Re-record the request with updated metadata (new event tracks the update)
        let resource_conforms_to = updated.service_type_ids.first().cloned();
        let update_event_input = CreateEconomicEventInput {
            id: Some(format!("evt-updated-{}", id)),
            action: "modify".to_string(),
            provider: updated.requester_id.clone(),
            receiver: updated.requester_id.clone(),
            resource_conforms_to,
            resource_inventoried_as: None,
            resource_classified_as: vec![TAG_SERVICE_REQUEST.to_string()],
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: input.estimated_hours.map(|h| h as f32),
            effort_quantity_unit: input.estimated_hours.map(|_| "hours".to_string()),
            has_point_in_time: Some(now),
            has_duration: None,
            input_of: None,
            output_of: Some(format!("evt-{}", id)),
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            note: updated.description.clone(),
            metadata_json: Some(updated_json),
        };
        record_event(conn, ctx, update_event_input)?;

        Ok(updated)
    }

    /// Archive (soft delete) a service request
    pub fn archive_request(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        requester_id: &str,
    ) -> Result<(), StorageError> {
        let existing = Self::get_request(conn, ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("Request {} not found", id)))?;

        if existing.requester_id != requester_id {
            return Err(StorageError::Auth(
                "Only the requester can archive this request".into(),
            ));
        }

        let event_id = format!("evt-{}", id);
        update_event_state(conn, ctx, &event_id, "archived")?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // OFFERS
    // -------------------------------------------------------------------------

    /// List service offers with optional filters
    pub fn list_offers(
        conn: &mut PooledConn,
        ctx: &AppContext,
        query: &OfferQuery,
    ) -> Result<Vec<ServiceOffer>, StorageError> {
        let event_query = EconomicEventQuery {
            action: Some("deliver-service".to_string()),
            provider: query.offeror_id.clone(),
            limit: query.limit.unwrap_or(500),
            offset: query.offset.unwrap_or(0),
            ..Default::default()
        };

        let events = list_economic_events(conn, ctx, &event_query)?;
        let mut offers: Vec<ServiceOffer> = events
            .into_iter()
            .filter(is_service_offer_event)
            .filter_map(|e| offer_from_event(&e))
            .collect();

        if let Some(ref status_filter) = query.status {
            offers.retain(|o| &o.status == status_filter);
        }

        Ok(offers)
    }

    /// Get a single service offer by ID
    pub fn get_offer(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ServiceOffer>, StorageError> {
        let event_id = format!("evt-{}", id);
        if let Some(event) = get_economic_event(conn, ctx, &event_id)? {
            if is_service_offer_event(&event) {
                return Ok(offer_from_event(&event));
            }
        }
        Ok(None)
    }

    /// Create a service offer — compound 3-write in a single transaction
    pub fn create_offer(
        conn: &mut PooledConn,
        ctx: &AppContext,
        input: CreateOfferInput,
    ) -> Result<CreateOfferResponse, StorageError> {
        if input.offeror_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "offerorId must not be empty".into(),
            ));
        }
        if input.title.trim().is_empty() {
            return Err(StorageError::InvalidInput("title must not be empty".into()));
        }

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let offer_id = generate_offer_id();
        let intent_id = generate_intent_id("offer");

        let offer = ServiceOffer {
            id: offer_id.clone(),
            offer_number: Some(format!("OFFER-{}", &offer_id[6..14])),
            offeror_id: input.offeror_id.clone(),
            title: input.title.clone(),
            description: input.description.clone(),
            contact_preference: input.contact_preference.clone(),
            date_range: input.date_range.clone(),
            service_type_ids: input.service_type_ids.clone(),
            interaction_type: input.interaction_type.clone(),
            rate: input.rate.clone(),
            status: "active".to_string(),
            matched_request_ids: vec![],
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let resource_conforms_to = input.service_type_ids.first().cloned();
        let intent = ReaIntent {
            id: intent_id.clone(),
            action: "deliver-service".to_string(),
            provider: Some(input.offeror_id.clone()),
            resource_conforms_to: resource_conforms_to.clone(),
            effort_quantity: input.hours_per_week,
            effort_unit: input.hours_per_week.map(|_| "hours/week".to_string()),
            has_beginning: input
                .date_range
                .as_ref()
                .and_then(|dr| dr.start_date.clone()),
            has_end: input.date_range.as_ref().and_then(|dr| dr.end_date.clone()),
            finished: false,
            note: input.description.clone(),
        };

        conn.transaction::<_, StorageError, _>(|conn| {
            // Write 1: Store the service offer
            let offer_json = serde_json::to_string(&offer)
                .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

            let offer_event_input = CreateEconomicEventInput {
                id: Some(format!("evt-{}", offer_id)),
                action: "deliver-service".to_string(),
                provider: input.offeror_id.clone(),
                receiver: input.offeror_id.clone(),
                resource_conforms_to: resource_conforms_to.clone(),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_OFFER.to_string()],
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: input.hours_per_week.map(|h| h as f32),
                effort_quantity_unit: input.hours_per_week.map(|_| "hours/week".to_string()),
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: None,
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                note: input.description.clone(),
                metadata_json: Some(offer_json),
            };
            record_event(conn, ctx, offer_event_input)?;

            // Write 2: Store the REA intent
            let intent_json = serde_json::to_string(&intent)
                .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

            let intent_event_input = CreateEconomicEventInput {
                id: Some(format!("evt-{}", intent_id)),
                action: "deliver-service".to_string(),
                provider: input.offeror_id.clone(),
                receiver: input.offeror_id.clone(),
                resource_conforms_to: resource_conforms_to.clone(),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_OFFER_INTENT.to_string()],
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: input.hours_per_week.map(|h| h as f32),
                effort_quantity_unit: input.hours_per_week.map(|_| "hours/week".to_string()),
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: Some(format!("evt-{}", offer_id)),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: Some(format!("evt-{}", offer_id)),
                note: Some(format!("Intent for offer {}", offer_id)),
                metadata_json: Some(intent_json),
            };
            record_event(conn, ctx, intent_event_input)?;

            // Write 3: Create initial lifecycle event
            let created_event_id = Uuid::new_v4().to_string();
            let created_event_meta = serde_json::json!({
                "listingId": offer_id,
                "listingType": "offer",
                "action": "created"
            });

            let creation_event_input = CreateEconomicEventInput {
                id: Some(created_event_id.clone()),
                action: "produce".to_string(),
                provider: input.offeror_id.clone(),
                receiver: input.offeror_id.clone(),
                resource_conforms_to: Some("service-listing".to_string()),
                resource_inventoried_as: None,
                resource_classified_as: vec![TAG_SERVICE_OFFER_LIFECYCLE.to_string()],
                resource_quantity_value: Some(1.0),
                resource_quantity_unit: Some("listing".to_string()),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_point_in_time: Some(now.clone()),
                has_duration: None,
                input_of: None,
                output_of: Some(format!("evt-{}", offer_id)),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                note: Some(format!("Service offer created: {}", offer.title)),
                metadata_json: Some(created_event_meta.to_string()),
            };
            let created_event = record_event(conn, ctx, creation_event_input)?;

            Ok(CreateOfferResponse {
                offer,
                intent,
                created_event: EconomicEventView::from(created_event),
            })
        })
    }

    /// Update a service offer — validates ownership and merges fields
    pub fn update_offer(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        input: UpdateOfferInput,
    ) -> Result<ServiceOffer, StorageError> {
        let existing = Self::get_offer(conn, ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("Offer {} not found", id)))?;

        if existing.offeror_id != input.offeror_id {
            return Err(StorageError::Auth(
                "Only the offeror can update this offer".into(),
            ));
        }

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let updated = ServiceOffer {
            id: existing.id.clone(),
            offer_number: existing.offer_number.clone(),
            offeror_id: existing.offeror_id.clone(),
            title: input.title.unwrap_or(existing.title),
            description: input.description.or(existing.description),
            contact_preference: input.contact_preference.or(existing.contact_preference),
            date_range: input.date_range.or(existing.date_range),
            service_type_ids: input.service_type_ids.unwrap_or(existing.service_type_ids),
            interaction_type: input.interaction_type.or(existing.interaction_type),
            rate: input.rate.or(existing.rate),
            status: existing.status,
            matched_request_ids: existing.matched_request_ids,
            created_at: existing.created_at,
            updated_at: now.clone(),
        };

        let updated_json = serde_json::to_string(&updated)
            .map_err(|e| StorageError::Internal(format!("Serialization error: {}", e)))?;

        let event_id = format!("evt-{}", id);
        update_event_state(conn, ctx, &event_id, "updated")?;

        let resource_conforms_to = updated.service_type_ids.first().cloned();
        let update_event_input = CreateEconomicEventInput {
            id: Some(format!("evt-updated-{}", id)),
            action: "modify".to_string(),
            provider: updated.offeror_id.clone(),
            receiver: updated.offeror_id.clone(),
            resource_conforms_to,
            resource_inventoried_as: None,
            resource_classified_as: vec![TAG_SERVICE_OFFER.to_string()],
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: input.hours_per_week.map(|h| h as f32),
            effort_quantity_unit: input.hours_per_week.map(|_| "hours/week".to_string()),
            has_point_in_time: Some(now),
            has_duration: None,
            input_of: None,
            output_of: Some(format!("evt-{}", id)),
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            note: updated.description.clone(),
            metadata_json: Some(updated_json),
        };
        record_event(conn, ctx, update_event_input)?;

        Ok(updated)
    }

    /// Archive (soft delete) a service offer
    pub fn archive_offer(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        offeror_id: &str,
    ) -> Result<(), StorageError> {
        let existing = Self::get_offer(conn, ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("Offer {} not found", id)))?;

        if existing.offeror_id != offeror_id {
            return Err(StorageError::Auth(
                "Only the offeror can archive this offer".into(),
            ));
        }

        let event_id = format!("evt-{}", id);
        update_event_state(conn, ctx, &event_id, "archived")?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // MATCHING
    // -------------------------------------------------------------------------

    /// Find matching offers for a given request
    pub fn match_request(
        conn: &mut PooledConn,
        ctx: &AppContext,
        request_id: &str,
    ) -> Result<Vec<ServiceMatch>, StorageError> {
        let request = Self::get_request(conn, ctx, request_id)?
            .ok_or_else(|| StorageError::NotFound(format!("Request {} not found", request_id)))?;

        // List all active offers
        let offers = Self::list_offers(
            conn,
            ctx,
            &OfferQuery {
                status: Some("active".to_string()),
                ..Default::default()
            },
        )?;

        let matches: Vec<ServiceMatch> = offers
            .into_iter()
            .filter_map(|offer| Self::compute_match_request_offer(&request, &offer))
            .collect();

        Ok(matches)
    }

    /// Find matching requests for a given offer
    pub fn match_offer(
        conn: &mut PooledConn,
        ctx: &AppContext,
        offer_id: &str,
    ) -> Result<Vec<ServiceMatch>, StorageError> {
        let offer = Self::get_offer(conn, ctx, offer_id)?
            .ok_or_else(|| StorageError::NotFound(format!("Offer {} not found", offer_id)))?;

        let requests = Self::list_requests(
            conn,
            ctx,
            &RequestQuery {
                status: Some("active".to_string()),
                ..Default::default()
            },
        )?;

        let matches: Vec<ServiceMatch> = requests
            .into_iter()
            .filter_map(|req| Self::compute_match_request_offer(&req, &offer))
            .collect();

        Ok(matches)
    }

    // -------------------------------------------------------------------------
    // Private: matching algorithm
    // -------------------------------------------------------------------------

    fn compute_match_request_offer(
        request: &ServiceRequest,
        offer: &ServiceOffer,
    ) -> Option<ServiceMatch> {
        // Service type intersection
        let shared: Vec<String> = request
            .service_type_ids
            .iter()
            .filter(|t| offer.service_type_ids.contains(t))
            .cloned()
            .collect();

        if shared.is_empty() {
            return None;
        }

        // Time compatibility check
        let time_compatible =
            check_date_range_overlap(request.date_range.as_ref(), offer.date_range.as_ref());

        // Interaction type compatibility
        let exchange_compatible = match (
            request.interaction_type.as_deref(),
            offer.interaction_type.as_deref(),
        ) {
            (Some(r), Some(o)) => {
                r == o || r == "either" || o == "either" || r == "hybrid" || o == "hybrid"
            }
            _ => true, // No preference specified = compatible
        };

        // Calculate quality score (0.0–1.0)
        let type_score = shared.len() as f64 / request.service_type_ids.len().max(1) as f64;
        let time_score = if time_compatible { 1.0 } else { 0.3 };
        let exchange_score = if exchange_compatible { 1.0 } else { 0.2 };
        let match_quality =
            (type_score * 0.5 + time_score * 0.3 + exchange_score * 0.2).clamp(0.0, 1.0);

        // Only return matches above minimal threshold
        if match_quality < 0.1 {
            return None;
        }

        Some(ServiceMatch {
            request_id: request.id.clone(),
            offer_id: offer.id.clone(),
            match_quality,
            shared_service_types: shared,
            time_compatible,
            exchange_compatible,
            status: "suggested".to_string(),
        })
    }
}

// ============================================================================
// Date range overlap helper
// ============================================================================

fn check_date_range_overlap(a: Option<&DateRange>, b: Option<&DateRange>) -> bool {
    match (a, b) {
        (None, _) | (_, None) => true, // No constraint = flexible
        (Some(ra), Some(rb)) => {
            if ra.flexible_dates || rb.flexible_dates {
                return true;
            }
            let a_start = ra.start_date.as_deref();
            let a_end = ra.end_date.as_deref();
            let b_start = rb.start_date.as_deref();
            let b_end = rb.end_date.as_deref();

            match (a_start, a_end, b_start, b_end) {
                (Some(as_), _, _, Some(be)) if as_ > be => false,
                (_, Some(ae), Some(bs), _) if ae < bs => false,
                _ => true,
            }
        }
    }
}
