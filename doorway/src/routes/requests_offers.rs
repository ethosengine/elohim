//! Requests & Offers API — Peer-to-peer service coordination in Shefa
//!
//! Rust serde types and route stubs for the `IRequestsAndOffers` interface.
//!
//! ## Routes
//!
//! ### Requests
//! - `POST   /api/v1/requests`                — Create a service request
//! - `PATCH  /api/v1/requests/{id}`            — Update a service request
//! - `PUT    /api/v1/requests/{id}/archive`    — Archive a request (soft delete)
//! - `DELETE /api/v1/requests/{id}`            — Delete a request
//! - `GET    /api/v1/requests/{id}`            — Get a request by ID
//! - `GET    /api/v1/requests?requesterId=`    — List requests for a user
//! - `GET    /api/v1/requests/{id}/matches`    — Find matching offers for a request
//!
//! ### Offers
//! - `POST   /api/v1/offers`                   — Create a service offer
//! - `PATCH  /api/v1/offers/{id}`              — Update a service offer
//! - `PUT    /api/v1/offers/{id}/archive`      — Archive an offer (soft delete)
//! - `DELETE /api/v1/offers/{id}`              — Delete an offer
//! - `GET    /api/v1/offers/{id}`              — Get an offer by ID
//! - `GET    /api/v1/offers?offerorId=`        — List offers for a user
//! - `GET    /api/v1/offers/{id}/matches`      — Find matching requests for an offer
//!
//! ## Architecture
//!
//! Doorway is a **thin HTTP gateway**. These stubs will forward to
//! elohim-storage / conductor once the DNA zome is implemented.
//! Access control is enforced by the DNA layer, not here.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

// ============================================================================
// Supporting Enums
// ============================================================================

/// How someone prefers to be contacted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ContactPreference {
    Email,
    Phone,
    InApp,
    Other,
}

/// Active/archived/deleted lifecycle for listings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListingLifecycleStatus {
    Active,
    Archived,
    Deleted,
}

/// Time-of-day preference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimePreference {
    Morning,
    Afternoon,
    Evening,
    Flexible,
    Other,
}

/// Virtual or in-person interaction mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionType {
    Virtual,
    InPerson,
    Hybrid,
    Either,
}

/// Medium of exchange type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExchangeType {
    Currency,
    TimeBanking,
    MutualCredit,
    Barter,
}

/// Rate period for offers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RatePer {
    Hour,
    Project,
    Month,
    Other,
}

/// Status of a service match lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchStatus {
    Suggested,
    Contacted,
    Negotiating,
    Agreed,
    Completed,
    Rejected,
}

/// Administrative moderation status for a listing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ListingStatusType {
    Pending,
    Accepted,
    Rejected,
    SuspendedTemporarily,
    SuspendedIndefinitely,
}

/// REA action verbs (subset relevant to requests/offers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReaAction {
    Use,
    Consume,
    Cite,
    Produce,
    Raise,
    Lower,
    Transfer,
    TransferCustody,
    TransferAllRights,
    Move,
    Modify,
    Combine,
    Separate,
    Work,
    DeliverService,
    Pickup,
    Dropoff,
    Give,
    Take,
    Accept,
}

/// State of an economic event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventState {
    Pending,
    Validated,
    Countersigned,
    Disputed,
    Corrected,
}

/// Listing type discriminator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListingType {
    Request,
    Offer,
}

// ============================================================================
// Value Objects
// ============================================================================

/// A quantity with a unit of measure (ValueFlows Measure).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub has_numerical_value: f64,
    pub has_unit: String,
}

/// Date range for request/offer availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    pub flexible_dates: bool,
}

/// Budget attached to a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub amount: Measure,
    pub medium_of_exchange_id: String,
}

/// Rate/pricing attached to an offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rate {
    pub amount: Measure,
    pub per: RatePer,
    pub medium_of_exchange_id: String,
}

// ============================================================================
// REA Foundation Types
// ============================================================================

/// REA Intent — an expressed desire to give or take a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub action: ReaAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_inventoried_as: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_point_in_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_beginning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classified_as: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Economic Event — an observed economic occurrence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomicEvent {
    pub id: String,
    pub action: ReaAction,
    pub provider: String,
    pub receiver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_inventoried_as: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_resource_inventoried_as: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_classified_as: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_quantity: Option<Measure>,
    pub has_point_in_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fulfills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realization_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreed_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub satisfies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub state: EventState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Categorization & Exchange
// ============================================================================

/// A category of service that can be requested or offered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_technical: bool,
    pub creator_id: String,
    pub created_at: String,
    pub is_author_only: bool,
    pub status: ListingLifecycleStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// What can be exchanged in a request/offer (currency, time, mutual credit).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediumOfExchange {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub exchange_type: ExchangeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_spec_hrea_id: Option<String>,
    pub creator_id: String,
    pub created_at: String,
    pub is_author_only: bool,
    pub status: ListingLifecycleStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Service Request
// ============================================================================

/// A service request — someone needs a service/resource.
///
/// Extends REA Intent with preferences, timing, and budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRequest {
    pub id: String,
    pub request_number: String,
    pub requester_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub title: String,
    pub description: String,

    // Preferences & Timing
    pub contact_preference: ContactPreference,
    pub contact_value: String,
    pub date_range: DateRange,
    pub time_zone: String,
    pub time_preference: TimePreference,
    pub interaction_type: InteractionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,

    // Service & Resources
    pub service_type_ids: Vec<String>,
    pub required_skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    pub medium_of_exchange_ids: Vec<String>,

    // Status & Lifecycle
    pub status: ListingLifecycleStatus,
    pub is_public: bool,
    pub created_at: String,
    pub updated_at: String,
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    // Matching & Coordination (populated after matching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_offer_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_proposal_id: Option<String>,

    // Flattened Intent fields
    pub action: ReaAction,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Input for creating a new service request.
///
/// Excludes auto-generated fields: `id`, `requestNumber`, `createdAt`, `updatedAt`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestInput {
    pub requester_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub title: String,
    pub description: String,
    pub contact_preference: ContactPreference,
    pub contact_value: String,
    pub date_range: DateRange,
    pub time_zone: String,
    pub time_preference: TimePreference,
    pub interaction_type: InteractionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    pub service_type_ids: Vec<String>,
    pub required_skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    #[serde(default)]
    pub medium_of_exchange_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub is_public: bool,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Input for updating an existing service request.
///
/// All fields optional — only provided fields are patched.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequestInput {
    pub requester_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_preference: Option<ContactPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_preference: Option<TimePreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_type: Option<InteractionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium_of_exchange_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Service Offer
// ============================================================================

/// A service offer — someone can provide a service/resource.
///
/// Extends REA Intent with availability, skills, and pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOffer {
    pub id: String,
    pub offer_number: String,
    pub offeror_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub title: String,
    pub description: String,

    // Preferences & Availability
    pub contact_preference: ContactPreference,
    pub contact_value: String,
    pub date_range: DateRange,
    pub time_zone: String,
    pub time_preference: TimePreference,
    pub interaction_type: InteractionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_per_week: Option<f64>,

    // Service & Compensation
    pub service_type_ids: Vec<String>,
    pub offerred_skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Rate>,
    pub medium_of_exchange_ids: Vec<String>,
    pub accepts_alternative_payment: bool,

    // Status & Lifecycle
    pub status: ListingLifecycleStatus,
    pub is_public: bool,
    pub created_at: String,
    pub updated_at: String,
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    // Matching & Coordination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_proposal_id: Option<String>,

    // Flattened Intent fields
    pub action: ReaAction,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_quantity: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Input for creating a new service offer.
///
/// Excludes auto-generated fields: `id`, `offerNumber`, `createdAt`, `updatedAt`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOfferInput {
    pub offeror_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub title: String,
    pub description: String,
    pub contact_preference: ContactPreference,
    pub contact_value: String,
    pub date_range: DateRange,
    pub time_zone: String,
    pub time_preference: TimePreference,
    pub interaction_type: InteractionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_per_week: Option<f64>,
    pub service_type_ids: Vec<String>,
    pub offerred_skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Rate>,
    #[serde(default)]
    pub medium_of_exchange_ids: Vec<String>,
    #[serde(default)]
    pub accepts_alternative_payment: bool,
    #[serde(default = "default_true")]
    pub is_public: bool,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Input for updating an existing service offer.
///
/// All fields optional — only provided fields are patched.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOfferInput {
    pub offeror_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_preference: Option<ContactPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_preference: Option<TimePreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_type: Option<InteractionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_per_week: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offerred_skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium_of_exchange_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepts_alternative_payment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Matching
// ============================================================================

/// A potential or actual match between a request and an offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMatch {
    pub id: String,
    pub request_id: String,
    pub offer_id: String,
    pub match_reason: String,
    pub match_quality: u32,
    pub shared_service_types: Vec<String>,
    pub time_compatible: bool,
    pub interaction_compatible: bool,
    pub exchange_compatible: bool,

    // Match Lifecycle
    pub status: MatchStatus,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,

    // REA Integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Filters
// ============================================================================

/// Query filters for listing requests.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ListingLifecycleStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
}

/// Query filters for listing offers.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ListingLifecycleStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
}

// ============================================================================
// Administrative
// ============================================================================

/// Administrative moderation status for a request or offer listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingAdminStatus {
    pub id: String,
    pub listing_type: ListingType,
    pub listing_id: String,
    pub status_type: ListingStatusType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_until: Option<String>,
    pub set_by_admin_id: String,
    pub set_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Composite Responses
// ============================================================================

/// Response from creating a service request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestResponse {
    pub request: ServiceRequest,
    pub intent: Intent,
    pub created_event: EconomicEvent,
}

/// Response from updating a service request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequestResponse {
    pub request: ServiceRequest,
    pub updated_event: EconomicEvent,
}

/// Response from creating a service offer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOfferResponse {
    pub offer: ServiceOffer,
    pub intent: Intent,
    pub created_event: EconomicEvent,
}

/// Response from updating a service offer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOfferResponse {
    pub offer: ServiceOffer,
    pub updated_event: EconomicEvent,
}

// ============================================================================
// Preferences
// ============================================================================

/// User preferences for the requests-and-offers network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub id: String,
    pub user_id: String,
    pub contact_preference: ContactPreference,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: TimePreference,
    pub interaction_type: InteractionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_hours_per_week: Option<f64>,
    pub languages: Vec<String>,
    pub skills_to_learn: Vec<String>,
    pub skills_to_share: Vec<String>,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Helpers
// ============================================================================

fn default_true() -> bool {
    true
}

fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({ "error": message });
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&body).unwrap_or_default(),
        )))
        .unwrap()
}

#[allow(dead_code)]
fn not_implemented_response(handler: &str) -> Response<Full<Bytes>> {
    error_response(
        StatusCode::NOT_IMPLEMENTED,
        &format!("{handler}: not yet implemented"),
    )
}

fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    match serde_json::to_vec(body) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::from(bytes)))
            .unwrap(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to serialize response: {e}"),
        ),
    }
}

/// Forward a requests-offers request to elohim-storage under /db/*.
///
/// Maps `/api/v1/requests-offers/{tail}` → `{storage_url}/db/{tail}`.
async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    storage_path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
    let base = storage_url.trim_end_matches('/');
    let full_url = match query {
        Some(q) if !q.is_empty() => format!("{base}{storage_path}?{q}"),
        _ => format!("{base}{storage_path}"),
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding requests-offers to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::PATCH => client.patch(&full_url),
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

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
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
                        path = %storage_path,
                        "Forwarded requests-offers response"
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

// ============================================================================
// Storage Client Helpers
// ============================================================================

const SHEFA_COORDINATION: &str = "shefa-coordination";

/// POST a JSON document to storage at `{storage_url}{path}`.
async fn storage_post(
    client: &reqwest::Client,
    storage_url: &str,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let resp = client
        .post(&url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Storage POST {path} failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Storage POST {path} returned {status}: {text}"));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse storage POST {path} response: {e}"))
}

/// GET a JSON document from storage at `{storage_url}{path}`.
async fn storage_get(
    client: &reqwest::Client,
    storage_url: &str,
    path: &str,
) -> Result<Option<serde_json::Value>, String> {
    let url = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Storage GET {path} failed: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Storage GET {path} returned {status}: {text}"));
    }
    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse storage GET {path} response: {e}"))?;
    Ok(Some(val))
}

/// PUT (replace) a JSON document in storage at `{storage_url}{path}`.
async fn storage_put(
    client: &reqwest::Client,
    storage_url: &str,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let resp = client
        .put(&url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Storage PUT {path} failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Storage PUT {path} returned {status}: {text}"));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse storage PUT {path} response: {e}"))
}

// ============================================================================
// Economic Event Factory
// ============================================================================

/// Minimal payload to create an economic event in storage.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEventPayload {
    id: String,
    action: ReaAction,
    provider: String,
    receiver: String,
    has_point_in_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_conforms_to: Option<String>,
    note: String,
    state: EventState,
}

fn make_event(
    action: ReaAction,
    provider_id: &str,
    note: &str,
    resource_conforms_to: Option<String>,
    now: &str,
) -> CreateEventPayload {
    CreateEventPayload {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        action,
        provider: provider_id.to_string(),
        receiver: SHEFA_COORDINATION.to_string(),
        has_point_in_time: now.to_string(),
        resource_conforms_to,
        note: note.to_string(),
        state: EventState::Pending,
    }
}

// ============================================================================
// Request Handlers
// ============================================================================

/// POST /api/v1/requests-offers/requests
///
/// Validates input, constructs ServiceRequest + REA Intent + EconomicEvent,
/// persists all three to storage, and returns the composite response.
async fn handle_create_request(req: Request<Incoming>, storage_url: &str) -> Response<Full<Bytes>> {
    // Read body
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    let input: CreateRequestInput = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
            )
        }
    };

    // Validate
    if input.title.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Request title is required");
    }
    if input.description.trim().len() < 20 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Request description must be at least 20 characters",
        );
    }
    if input.service_type_ids.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Request must specify at least one service type",
        );
    }
    if input.medium_of_exchange_ids.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Request must specify at least one payment method",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();
    let request_id = format!("req-{}", uuid::Uuid::new_v4());
    let ts_suffix = chrono::Utc::now().timestamp_millis().to_string();
    let request_number = format!("REQ-{}", &ts_suffix[ts_suffix.len().saturating_sub(10)..]);

    let request = ServiceRequest {
        id: request_id.clone(),
        request_number: request_number.clone(),
        requester_id: input.requester_id.clone(),
        organization_id: input.organization_id,
        title: input.title.clone(),
        description: input.description.clone(),
        contact_preference: input.contact_preference,
        contact_value: input.contact_value,
        time_zone: input.time_zone,
        time_preference: input.time_preference,
        interaction_type: input.interaction_type,
        date_range: input.date_range,
        estimated_hours: input.estimated_hours,
        service_type_ids: input.service_type_ids.clone(),
        required_skills: input.required_skills,
        budget: input.budget.clone(),
        medium_of_exchange_ids: input.medium_of_exchange_ids,
        status: ListingLifecycleStatus::Active,
        is_public: false,
        links: input.links,
        created_at: now.clone(),
        updated_at: now.clone(),
        metadata: input.metadata,
        matched_offer_ids: None,
        proposal_ids: None,
        accepted_proposal_id: None,
        // Flattened Intent fields
        action: ReaAction::Take,
        finished: false,
        name: None,
        provider: None,
        receiver: Some(input.requester_id.clone()),
        resource_conforms_to: Some(input.service_type_ids.join("|")),
        resource_quantity: input.budget.as_ref().map(|b| b.amount.clone()),
        effort_quantity: None,
        available_quantity: None,
        note: Some(format!("Request for {}", input.title)),
        in_scope_of: None,
        image: None,
    };

    let intent = Intent {
        id: format!("intent-{request_id}"),
        name: None,
        action: ReaAction::Take,
        provider: None,
        receiver: Some(input.requester_id.clone()),
        resource_conforms_to: Some(request.service_type_ids.join("|")),
        resource_inventoried_as: None,
        resource_quantity: request.budget.as_ref().map(|b| b.amount.clone()),
        effort_quantity: None,
        available_quantity: None,
        has_point_in_time: None,
        has_beginning: None,
        has_end: None,
        due: None,
        note: Some(format!("Request for {}", request.title)),
        finished: false,
        in_scope_of: None,
        input_of: None,
        output_of: None,
        classified_as: Some(vec![request_id.clone(), request_number.clone()]),
        image: None,
    };

    let event_note = format!(
        "Service request created: {}. Requester: {}. Services: {}. Request ID: {}. Request Number: {}.",
        request.title,
        input.requester_id,
        request.service_type_ids.join(", "),
        request_id,
        request_number
    );
    let event = make_event(
        ReaAction::Transfer,
        &input.requester_id,
        &event_note,
        Some("service-request".to_string()),
        &now,
    );

    let client = reqwest::Client::new();

    // Persist intent
    if let Err(e) = storage_post(&client, storage_url, "/db/intents", &intent).await {
        warn!(error = %e, "Failed to persist intent");
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to persist intent: {e}"),
        );
    }

    // Persist economic event
    let event_val = match storage_post(&client, storage_url, "/db/economic-events", &event).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "Failed to persist economic event");
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to persist economic event: {e}"),
            );
        }
    };

    // Persist request
    if let Err(e) = storage_post(&client, storage_url, "/db/requests", &request).await {
        warn!(error = %e, "Failed to persist request");
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to persist request: {e}"),
        );
    }

    info!(id = %request_id, number = %request_number, "Service request created");

    let resp = CreateRequestResponse {
        request,
        intent,
        created_event: serde_json::from_value(event_val).unwrap_or_else(|_| {
            // Fallback: reconstruct event from our payload
            EconomicEvent {
                id: event.id,
                action: event.action,
                provider: event.provider,
                receiver: event.receiver,
                resource_conforms_to: event.resource_conforms_to,
                resource_inventoried_as: None,
                to_resource_inventoried_as: None,
                resource_classified_as: None,
                resource_quantity: None,
                effort_quantity: None,
                has_point_in_time: event.has_point_in_time,
                has_duration: None,
                input_of: None,
                output_of: None,
                fulfills: None,
                realization_of: None,
                agreed_in: None,
                satisfies: None,
                in_scope_of: None,
                note: Some(event.note),
                state: event.state,
                triggered_by: None,
                at_location: None,
                image: None,
                metadata: None,
            }
        }),
    };

    json_response(StatusCode::CREATED, &resp)
}

/// PATCH /api/v1/requests-offers/requests/{id}
///
/// Verifies ownership, merges updates, creates a 'modify' event, persists.
async fn handle_update_request(
    req: Request<Incoming>,
    storage_url: &str,
    request_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    let input: UpdateRequestInput = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid update body: {e}"),
            )
        }
    };

    let client = reqwest::Client::new();

    // Fetch existing request
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/requests/{request_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Request {request_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceRequest = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing request: {e}"),
            )
        }
    };

    // Ownership check
    if existing.requester_id != input.requester_id {
        return error_response(
            StatusCode::FORBIDDEN,
            "Only requester can update this request",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Collect changed field names for event note
    let mut changed_fields: Vec<&str> = Vec::new();
    if input.title.is_some() {
        changed_fields.push("title");
    }
    if input.description.is_some() {
        changed_fields.push("description");
    }
    if input.contact_preference.is_some() {
        changed_fields.push("contactPreference");
    }
    if input.contact_value.is_some() {
        changed_fields.push("contactValue");
    }
    if input.date_range.is_some() {
        changed_fields.push("dateRange");
    }
    if input.time_zone.is_some() {
        changed_fields.push("timeZone");
    }
    if input.time_preference.is_some() {
        changed_fields.push("timePreference");
    }
    if input.interaction_type.is_some() {
        changed_fields.push("interactionType");
    }
    if input.estimated_hours.is_some() {
        changed_fields.push("estimatedHours");
    }
    if input.service_type_ids.is_some() {
        changed_fields.push("serviceTypeIds");
    }
    if input.required_skills.is_some() {
        changed_fields.push("requiredSkills");
    }
    if input.budget.is_some() {
        changed_fields.push("budget");
    }
    if input.medium_of_exchange_ids.is_some() {
        changed_fields.push("mediumOfExchangeIds");
    }
    if input.is_public.is_some() {
        changed_fields.push("isPublic");
    }
    if input.links.is_some() {
        changed_fields.push("links");
    }
    if input.metadata.is_some() {
        changed_fields.push("metadata");
    }

    // Merge — immutable fields (id, requestNumber, requesterId, createdAt) are never patched
    let service_type_ids = input
        .service_type_ids
        .unwrap_or(existing.service_type_ids.clone());
    let updated = ServiceRequest {
        id: existing.id.clone(),
        request_number: existing.request_number.clone(),
        requester_id: existing.requester_id.clone(),
        organization_id: existing.organization_id,
        title: input.title.unwrap_or(existing.title),
        description: input.description.unwrap_or(existing.description),
        contact_preference: input
            .contact_preference
            .unwrap_or(existing.contact_preference),
        contact_value: input.contact_value.unwrap_or(existing.contact_value),
        date_range: input.date_range.unwrap_or(existing.date_range),
        time_zone: input.time_zone.unwrap_or(existing.time_zone),
        time_preference: input.time_preference.unwrap_or(existing.time_preference),
        interaction_type: input.interaction_type.unwrap_or(existing.interaction_type),
        estimated_hours: input.estimated_hours.or(existing.estimated_hours),
        service_type_ids: service_type_ids.clone(),
        required_skills: input.required_skills.unwrap_or(existing.required_skills),
        budget: input.budget.or(existing.budget),
        medium_of_exchange_ids: input
            .medium_of_exchange_ids
            .unwrap_or(existing.medium_of_exchange_ids),
        status: existing.status,
        is_public: input.is_public.unwrap_or(existing.is_public),
        created_at: existing.created_at,
        updated_at: now.clone(),
        links: input.links.unwrap_or(existing.links),
        metadata: input.metadata.or(existing.metadata),
        matched_offer_ids: existing.matched_offer_ids,
        proposal_ids: existing.proposal_ids,
        accepted_proposal_id: existing.accepted_proposal_id,
        // Intent fields — update resource_conforms_to if service types changed
        action: existing.action,
        finished: existing.finished,
        name: existing.name,
        provider: existing.provider,
        receiver: existing.receiver,
        resource_conforms_to: Some(service_type_ids.join("|")),
        resource_quantity: existing.resource_quantity,
        effort_quantity: existing.effort_quantity,
        available_quantity: existing.available_quantity,
        note: existing.note,
        in_scope_of: existing.in_scope_of,
        image: existing.image,
    };

    let event_note = format!(
        "Service request updated: {}. Changes: {}. Request ID: {}.",
        updated.title,
        changed_fields.join(", "),
        existing.id
    );
    let event = make_event(
        ReaAction::Modify,
        &input.requester_id,
        &event_note,
        None,
        &now,
    );

    // Persist event
    let event_val = match storage_post(&client, storage_url, "/db/economic-events", &event).await {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
    };

    // Persist updated request
    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/requests/{}", existing.id),
        &updated,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %existing.id, "Service request updated");

    let resp = UpdateRequestResponse {
        request: updated,
        updated_event: serde_json::from_value(event_val).unwrap_or_else(|_| EconomicEvent {
            id: event.id,
            action: event.action,
            provider: event.provider,
            receiver: event.receiver,
            resource_conforms_to: event.resource_conforms_to,
            resource_inventoried_as: None,
            to_resource_inventoried_as: None,
            resource_classified_as: None,
            resource_quantity: None,
            effort_quantity: None,
            has_point_in_time: event.has_point_in_time,
            has_duration: None,
            input_of: None,
            output_of: None,
            fulfills: None,
            realization_of: None,
            agreed_in: None,
            satisfies: None,
            in_scope_of: None,
            note: Some(event.note),
            state: event.state,
            triggered_by: None,
            at_location: None,
            image: None,
            metadata: None,
        }),
    };

    json_response(StatusCode::OK, &resp)
}

/// POST /api/v1/requests-offers/requests/{id}/withdraw
///
/// Archives the request (status → "archived") and creates a 'raise' event.
async fn handle_withdraw_request(
    req: Request<Incoming>,
    storage_url: &str,
    request_id: &str,
) -> Response<Full<Bytes>> {
    // Read requester_id from body
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WithdrawBody {
        requester_id: String,
    }
    let body: WithdrawBody = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Expected {{ \"requesterId\": \"...\" }}: {e}"),
            )
        }
    };

    let client = reqwest::Client::new();
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/requests/{request_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Request {request_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceRequest = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing request: {e}"),
            )
        }
    };

    if existing.requester_id != body.requester_id {
        return error_response(
            StatusCode::FORBIDDEN,
            "Only requester can archive this request",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();
    let archived = ServiceRequest {
        status: ListingLifecycleStatus::Archived,
        updated_at: now.clone(),
        ..existing.clone()
    };

    let event_note = format!(
        "Request archived: {}. Request ID: {}.",
        existing.title, request_id
    );
    let event = make_event(
        ReaAction::Raise,
        &body.requester_id,
        &event_note,
        None,
        &now,
    );

    if let Err(e) = storage_post(&client, storage_url, "/db/economic-events", &event).await {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/requests/{request_id}"),
        &archived,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %request_id, "Service request archived");
    json_response(StatusCode::OK, &archived)
}

/// DELETE /api/v1/requests-offers/requests/{id}
///
/// Soft-deletes the request (status → "deleted") and creates a 'modify' event.
async fn handle_delete_request(
    req: Request<Incoming>,
    storage_url: &str,
    request_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeleteBody {
        requester_id: String,
    }
    let body: DeleteBody = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            // Also accept empty body — but then we can't verify ownership
            return error_response(
                StatusCode::BAD_REQUEST,
                "Expected { \"requesterId\": \"...\" }",
            );
        }
    };

    let client = reqwest::Client::new();
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/requests/{request_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Request {request_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceRequest = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing request: {e}"),
            )
        }
    };

    if existing.requester_id != body.requester_id {
        return error_response(
            StatusCode::FORBIDDEN,
            "Only requester can delete this request",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();
    let deleted = ServiceRequest {
        status: ListingLifecycleStatus::Deleted,
        updated_at: now.clone(),
        ..existing.clone()
    };

    let event_note = format!(
        "Request deleted: {}. Request ID: {}.",
        existing.title, request_id
    );
    let event = make_event(
        ReaAction::Modify,
        &body.requester_id,
        &event_note,
        None,
        &now,
    );

    if let Err(e) = storage_post(&client, storage_url, "/db/economic-events", &event).await {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/requests/{request_id}"),
        &deleted,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %request_id, "Service request soft-deleted");
    json_response(StatusCode::NO_CONTENT, &serde_json::Value::Null)
}

// ============================================================================
// Offer Handlers
// ============================================================================

/// POST /api/v1/requests-offers/offers
async fn handle_create_offer(req: Request<Incoming>, storage_url: &str) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    let input: CreateOfferInput = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("Invalid offer body: {e}"))
        }
    };

    // Validate
    if input.title.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Offer title is required");
    }
    if input.description.trim().len() < 20 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Offer description must be at least 20 characters",
        );
    }
    if input.service_type_ids.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Offer must specify at least one service type",
        );
    }
    if input.medium_of_exchange_ids.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Offer must specify at least one payment method",
        );
    }

    let now = chrono::Utc::now().to_rfc3339();
    let offer_id = format!("ofr-{}", uuid::Uuid::new_v4());
    let ts_suffix = chrono::Utc::now().timestamp_millis().to_string();
    let offer_number = format!("OFR-{}", &ts_suffix[ts_suffix.len().saturating_sub(10)..]);

    let offer = ServiceOffer {
        id: offer_id.clone(),
        offer_number: offer_number.clone(),
        offeror_id: input.offeror_id.clone(),
        organization_id: input.organization_id,
        title: input.title.clone(),
        description: input.description.clone(),
        contact_preference: input.contact_preference,
        contact_value: input.contact_value,
        date_range: input.date_range,
        time_zone: input.time_zone,
        time_preference: input.time_preference,
        interaction_type: input.interaction_type,
        hours_per_week: input.hours_per_week.or(Some(40.0)),
        service_type_ids: input.service_type_ids.clone(),
        offerred_skills: input.offerred_skills,
        rate: input.rate.clone(),
        medium_of_exchange_ids: input.medium_of_exchange_ids,
        accepts_alternative_payment: input.accepts_alternative_payment,
        status: ListingLifecycleStatus::Active,
        is_public: false,
        links: input.links,
        created_at: now.clone(),
        updated_at: now.clone(),
        metadata: input.metadata,
        matched_request_ids: None,
        proposal_ids: None,
        accepted_proposal_id: None,
        // Flattened Intent fields
        action: ReaAction::Give,
        finished: false,
        name: None,
        provider: Some(input.offeror_id.clone()),
        receiver: None,
        resource_conforms_to: Some(input.service_type_ids.join("|")),
        resource_quantity: input.rate.as_ref().map(|r| r.amount.clone()),
        effort_quantity: None,
        available_quantity: None,
        note: Some(format!("Offer for {}", input.title)),
        in_scope_of: None,
        image: None,
    };

    let intent = Intent {
        id: format!("intent-{offer_id}"),
        name: None,
        action: ReaAction::Give,
        provider: Some(input.offeror_id.clone()),
        receiver: None,
        resource_conforms_to: Some(offer.service_type_ids.join("|")),
        resource_inventoried_as: None,
        resource_quantity: offer.rate.as_ref().map(|r| r.amount.clone()),
        effort_quantity: None,
        available_quantity: None,
        has_point_in_time: None,
        has_beginning: None,
        has_end: None,
        due: None,
        note: Some(format!("Offer for {}", offer.title)),
        finished: false,
        in_scope_of: None,
        input_of: None,
        output_of: None,
        classified_as: Some(vec![offer_id.clone(), offer_number.clone()]),
        image: None,
    };

    let event_note = format!(
        "Service offer created: {}. Offeror: {}. Services: {}. Offer ID: {}. Offer Number: {}.",
        offer.title,
        input.offeror_id,
        offer.service_type_ids.join(", "),
        offer_id,
        offer_number
    );
    let event = make_event(
        ReaAction::Transfer,
        &input.offeror_id,
        &event_note,
        Some("service-offer".to_string()),
        &now,
    );

    let client = reqwest::Client::new();

    if let Err(e) = storage_post(&client, storage_url, "/db/intents", &intent).await {
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to persist intent: {e}"),
        );
    }

    let event_val = match storage_post(&client, storage_url, "/db/economic-events", &event).await {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to persist economic event: {e}"),
            )
        }
    };

    if let Err(e) = storage_post(&client, storage_url, "/db/offers", &offer).await {
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to persist offer: {e}"),
        );
    }

    info!(id = %offer_id, number = %offer_number, "Service offer created");

    let resp = CreateOfferResponse {
        offer,
        intent,
        created_event: serde_json::from_value(event_val).unwrap_or_else(|_| EconomicEvent {
            id: event.id,
            action: event.action,
            provider: event.provider,
            receiver: event.receiver,
            resource_conforms_to: event.resource_conforms_to,
            resource_inventoried_as: None,
            to_resource_inventoried_as: None,
            resource_classified_as: None,
            resource_quantity: None,
            effort_quantity: None,
            has_point_in_time: event.has_point_in_time,
            has_duration: None,
            input_of: None,
            output_of: None,
            fulfills: None,
            realization_of: None,
            agreed_in: None,
            satisfies: None,
            in_scope_of: None,
            note: Some(event.note),
            state: event.state,
            triggered_by: None,
            at_location: None,
            image: None,
            metadata: None,
        }),
    };

    json_response(StatusCode::CREATED, &resp)
}

/// PATCH /api/v1/requests-offers/offers/{id}
async fn handle_update_offer(
    req: Request<Incoming>,
    storage_url: &str,
    offer_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    let input: UpdateOfferInput = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid update body: {e}"),
            )
        }
    };

    let client = reqwest::Client::new();
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/offers/{offer_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Offer {offer_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceOffer = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing offer: {e}"),
            )
        }
    };

    if existing.offeror_id != input.offeror_id {
        return error_response(StatusCode::FORBIDDEN, "Only offeror can update this offer");
    }

    let now = chrono::Utc::now().to_rfc3339();

    let mut changed_fields: Vec<&str> = Vec::new();
    if input.title.is_some() {
        changed_fields.push("title");
    }
    if input.description.is_some() {
        changed_fields.push("description");
    }
    if input.contact_preference.is_some() {
        changed_fields.push("contactPreference");
    }
    if input.contact_value.is_some() {
        changed_fields.push("contactValue");
    }
    if input.date_range.is_some() {
        changed_fields.push("dateRange");
    }
    if input.time_zone.is_some() {
        changed_fields.push("timeZone");
    }
    if input.time_preference.is_some() {
        changed_fields.push("timePreference");
    }
    if input.interaction_type.is_some() {
        changed_fields.push("interactionType");
    }
    if input.hours_per_week.is_some() {
        changed_fields.push("hoursPerWeek");
    }
    if input.service_type_ids.is_some() {
        changed_fields.push("serviceTypeIds");
    }
    if input.offerred_skills.is_some() {
        changed_fields.push("offeredSkills");
    }
    if input.rate.is_some() {
        changed_fields.push("rate");
    }
    if input.medium_of_exchange_ids.is_some() {
        changed_fields.push("mediumOfExchangeIds");
    }
    if input.accepts_alternative_payment.is_some() {
        changed_fields.push("acceptsAlternativePayment");
    }
    if input.is_public.is_some() {
        changed_fields.push("isPublic");
    }
    if input.links.is_some() {
        changed_fields.push("links");
    }
    if input.metadata.is_some() {
        changed_fields.push("metadata");
    }

    let service_type_ids = input
        .service_type_ids
        .unwrap_or(existing.service_type_ids.clone());
    let updated = ServiceOffer {
        id: existing.id.clone(),
        offer_number: existing.offer_number.clone(),
        offeror_id: existing.offeror_id.clone(),
        organization_id: existing.organization_id,
        title: input.title.unwrap_or(existing.title),
        description: input.description.unwrap_or(existing.description),
        contact_preference: input
            .contact_preference
            .unwrap_or(existing.contact_preference),
        contact_value: input.contact_value.unwrap_or(existing.contact_value),
        date_range: input.date_range.unwrap_or(existing.date_range),
        time_zone: input.time_zone.unwrap_or(existing.time_zone),
        time_preference: input.time_preference.unwrap_or(existing.time_preference),
        interaction_type: input.interaction_type.unwrap_or(existing.interaction_type),
        hours_per_week: input.hours_per_week.or(existing.hours_per_week),
        service_type_ids: service_type_ids.clone(),
        offerred_skills: input.offerred_skills.unwrap_or(existing.offerred_skills),
        rate: input.rate.or(existing.rate),
        medium_of_exchange_ids: input
            .medium_of_exchange_ids
            .unwrap_or(existing.medium_of_exchange_ids),
        accepts_alternative_payment: input
            .accepts_alternative_payment
            .unwrap_or(existing.accepts_alternative_payment),
        status: existing.status,
        is_public: input.is_public.unwrap_or(existing.is_public),
        created_at: existing.created_at,
        updated_at: now.clone(),
        links: input.links.unwrap_or(existing.links),
        metadata: input.metadata.or(existing.metadata),
        matched_request_ids: existing.matched_request_ids,
        proposal_ids: existing.proposal_ids,
        accepted_proposal_id: existing.accepted_proposal_id,
        action: existing.action,
        finished: existing.finished,
        name: existing.name,
        provider: existing.provider,
        receiver: existing.receiver,
        resource_conforms_to: Some(service_type_ids.join("|")),
        resource_quantity: existing.resource_quantity,
        effort_quantity: existing.effort_quantity,
        available_quantity: existing.available_quantity,
        note: existing.note,
        in_scope_of: existing.in_scope_of,
        image: existing.image,
    };

    let event_note = format!(
        "Service offer updated: {}. Changes: {}. Offer ID: {}.",
        updated.title,
        changed_fields.join(", "),
        existing.id
    );
    let event = make_event(
        ReaAction::Modify,
        &input.offeror_id,
        &event_note,
        None,
        &now,
    );

    let event_val = match storage_post(&client, storage_url, "/db/economic-events", &event).await {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
    };

    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/offers/{}", existing.id),
        &updated,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %existing.id, "Service offer updated");

    let resp = UpdateOfferResponse {
        offer: updated,
        updated_event: serde_json::from_value(event_val).unwrap_or_else(|_| EconomicEvent {
            id: event.id,
            action: event.action,
            provider: event.provider,
            receiver: event.receiver,
            resource_conforms_to: event.resource_conforms_to,
            resource_inventoried_as: None,
            to_resource_inventoried_as: None,
            resource_classified_as: None,
            resource_quantity: None,
            effort_quantity: None,
            has_point_in_time: event.has_point_in_time,
            has_duration: None,
            input_of: None,
            output_of: None,
            fulfills: None,
            realization_of: None,
            agreed_in: None,
            satisfies: None,
            in_scope_of: None,
            note: Some(event.note),
            state: event.state,
            triggered_by: None,
            at_location: None,
            image: None,
            metadata: None,
        }),
    };

    json_response(StatusCode::OK, &resp)
}

/// POST /api/v1/requests-offers/offers/{id}/withdraw
async fn handle_withdraw_offer(
    req: Request<Incoming>,
    storage_url: &str,
    offer_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WithdrawBody {
        offeror_id: String,
    }
    let body: WithdrawBody = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Expected {{ \"offerorId\": \"...\" }}: {e}"),
            )
        }
    };

    let client = reqwest::Client::new();
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/offers/{offer_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Offer {offer_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceOffer = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing offer: {e}"),
            )
        }
    };

    if existing.offeror_id != body.offeror_id {
        return error_response(StatusCode::FORBIDDEN, "Only offeror can archive this offer");
    }

    let now = chrono::Utc::now().to_rfc3339();
    let archived = ServiceOffer {
        status: ListingLifecycleStatus::Archived,
        updated_at: now.clone(),
        ..existing.clone()
    };

    let event_note = format!(
        "Offer archived: {}. Offer ID: {}.",
        existing.title, offer_id
    );
    let event = make_event(ReaAction::Raise, &body.offeror_id, &event_note, None, &now);

    if let Err(e) = storage_post(&client, storage_url, "/db/economic-events", &event).await {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/offers/{offer_id}"),
        &archived,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %offer_id, "Service offer archived");
    json_response(StatusCode::OK, &archived)
}

/// DELETE /api/v1/requests-offers/offers/{id}
async fn handle_delete_offer(
    req: Request<Incoming>,
    storage_url: &str,
    offer_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeleteBody {
        offeror_id: String,
    }
    let body: DeleteBody = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Expected { \"offerorId\": \"...\" }",
            )
        }
    };

    let client = reqwest::Client::new();
    let existing_val =
        match storage_get(&client, storage_url, &format!("/db/offers/{offer_id}")).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Offer {offer_id} not found"),
                )
            }
            Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
        };

    let existing: ServiceOffer = match serde_json::from_value(existing_val) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse existing offer: {e}"),
            )
        }
    };

    if existing.offeror_id != body.offeror_id {
        return error_response(StatusCode::FORBIDDEN, "Only offeror can delete this offer");
    }

    let now = chrono::Utc::now().to_rfc3339();
    let deleted = ServiceOffer {
        status: ListingLifecycleStatus::Deleted,
        updated_at: now.clone(),
        ..existing.clone()
    };

    let event_note = format!("Offer deleted: {}. Offer ID: {}.", existing.title, offer_id);
    let event = make_event(ReaAction::Modify, &body.offeror_id, &event_note, None, &now);

    if let Err(e) = storage_post(&client, storage_url, "/db/economic-events", &event).await {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    if let Err(e) = storage_put(
        &client,
        storage_url,
        &format!("/db/offers/{offer_id}"),
        &deleted,
    )
    .await
    {
        return error_response(StatusCode::BAD_GATEWAY, &e);
    }

    info!(id = %offer_id, "Service offer soft-deleted");
    json_response(StatusCode::NO_CONTENT, &serde_json::Value::Null)
}

// ============================================================================
// Route Dispatcher
// ============================================================================

/// Handle all `/api/v1/requests-offers/*` requests.
///
/// Mutation routes implement business logic (validate → construct → persist compound).
/// Read routes forward to elohim-storage.
pub async fn handle_requests_offers_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Requests-offers proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    let method = req.method().clone();
    let query = req.uri().query().map(|q| q.to_owned());

    // Strip the doorway prefix, keep the sub-path for storage mapping.
    // /api/v1/requests-offers/requests        → /db/requests
    // /api/v1/requests-offers/offers/{id}     → /db/offers/{id}
    // /api/v1/requests-offers/matches         → /db/matches
    // /api/v1/requests-offers/intents         → /db/intents
    let sub_path = path.strip_prefix("/api/v1/requests-offers").unwrap_or(path);
    // sub_path is now e.g. "/requests", "/offers/123/accept", ""
    let storage_path = format!("/db{sub_path}");
    let sub = sub_path.trim_start_matches('/');

    match (&method, sub) {
        // =====================================================================
        // Business-logic mutations — implemented in Rust
        // =====================================================================

        // POST /api/v1/requests-offers/requests
        (&Method::POST, "requests") => {
            handle_create_request(req, &storage_url).await
        }

        // PATCH /api/v1/requests-offers/requests/{id}
        (&Method::PATCH, p) if p.starts_with("requests/") && !p[9..].contains('/') => {
            let id = &p[9..];
            handle_update_request(req, &storage_url, id).await
        }

        // POST /api/v1/requests-offers/requests/{id}/withdraw
        (&Method::POST, p) if p.starts_with("requests/") && p.ends_with("/withdraw") => {
            let id = p.trim_start_matches("requests/").trim_end_matches("/withdraw");
            handle_withdraw_request(req, &storage_url, id).await
        }

        // DELETE /api/v1/requests-offers/requests/{id}
        (&Method::DELETE, p) if p.starts_with("requests/") && !p[9..].contains('/') => {
            let id = &p[9..];
            handle_delete_request(req, &storage_url, id).await
        }

        // POST /api/v1/requests-offers/offers
        (&Method::POST, "offers") => {
            handle_create_offer(req, &storage_url).await
        }

        // PATCH /api/v1/requests-offers/offers/{id}
        (&Method::PATCH, p) if p.starts_with("offers/") && !p[7..].contains('/') => {
            let id = &p[7..];
            handle_update_offer(req, &storage_url, id).await
        }

        // POST /api/v1/requests-offers/offers/{id}/withdraw
        (&Method::POST, p) if p.starts_with("offers/") && p.ends_with("/withdraw") => {
            let id = p.trim_start_matches("offers/").trim_end_matches("/withdraw");
            handle_withdraw_offer(req, &storage_url, id).await
        }

        // DELETE /api/v1/requests-offers/offers/{id}
        (&Method::DELETE, p) if p.starts_with("offers/") && !p[7..].contains('/') => {
            let id = &p[7..];
            handle_delete_offer(req, &storage_url, id).await
        }

        // =====================================================================
        // Read routes — forward to storage
        // =====================================================================

        // GET  /api/v1/requests-offers/requests
        (&Method::GET, "requests")
        // GET  /api/v1/requests-offers/offers
        | (&Method::GET, "offers")
        // GET  /api/v1/requests-offers/matches
        | (&Method::GET, "matches")
        // POST /api/v1/requests-offers/intents
        | (&Method::POST, "intents")
        // GET  /api/v1/requests-offers/intents
        | (&Method::GET, "intents") => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // GET  /api/v1/requests-offers/requests/{id}
        (&Method::GET, p) if p.starts_with("requests/") && !p[9..].contains('/') => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // GET  /api/v1/requests-offers/requests/{id}/matches (per-request matches)
        (&Method::GET, p) if p.starts_with("requests/") && p.ends_with("/matches") => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // GET  /api/v1/requests-offers/offers/matching/{requestId}
        (&Method::GET, p) if p.starts_with("offers/matching/") => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // GET  /api/v1/requests-offers/offers/{id}
        (&Method::GET, p) if p.starts_with("offers/") && !p[7..].contains('/') => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // POST /api/v1/requests-offers/offers/{id}/accept
        (&Method::POST, p) if p.starts_with("offers/") && p.ends_with("/accept") => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        // POST /api/v1/requests-offers/offers/{id}/reject
        (&Method::POST, p) if p.starts_with("offers/") && p.ends_with("/reject") => {
            forward_to_storage(req, &storage_url, &storage_path, query.as_deref()).await
        }

        _ => {
            error_response(
                StatusCode::NOT_FOUND,
                &format!("Unknown requests-offers route: {method} {path}"),
            )
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_preference_serde() {
        let json = serde_json::to_string(&ContactPreference::InApp).unwrap();
        assert_eq!(json, r#""in-app""#);
        let parsed: ContactPreference = serde_json::from_str(r#""in-app""#).unwrap();
        assert_eq!(parsed, ContactPreference::InApp);
    }

    #[test]
    fn test_listing_lifecycle_status_serde() {
        let json = serde_json::to_string(&ListingLifecycleStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test]
    fn test_interaction_type_serde() {
        let json = serde_json::to_string(&InteractionType::InPerson).unwrap();
        assert_eq!(json, r#""in-person""#);
    }

    #[test]
    fn test_measure_serde_camel_case() {
        let m = Measure {
            has_numerical_value: 42.0,
            has_unit: "unit-hour".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("hasNumericalValue"));
        assert!(json.contains("hasUnit"));
    }

    #[test]
    fn test_date_range_serde() {
        let dr = DateRange {
            start_date: Some("2026-03-01".to_string()),
            end_date: None,
            flexible_dates: true,
        };
        let json = serde_json::to_string(&dr).unwrap();
        assert!(json.contains("startDate"));
        assert!(json.contains("flexibleDates"));
        assert!(!json.contains("endDate")); // skipped because None
    }

    #[test]
    fn test_match_status_serde() {
        let json = serde_json::to_string(&MatchStatus::Negotiating).unwrap();
        assert_eq!(json, r#""negotiating""#);
    }

    #[test]
    fn test_rea_action_serde() {
        let json = serde_json::to_string(&ReaAction::DeliverService).unwrap();
        assert_eq!(json, r#""deliver-service""#);
    }

    #[test]
    fn test_event_state_serde() {
        let json = serde_json::to_string(&EventState::Countersigned).unwrap();
        assert_eq!(json, r#""countersigned""#);
    }

    #[test]
    fn test_create_request_input_deserialize() {
        let json = r#"{
            "requesterId": "user-1",
            "title": "Need logo design",
            "description": "Modern logo for startup",
            "contactPreference": "email",
            "contactValue": "user@example.com",
            "dateRange": { "flexibleDates": true },
            "timeZone": "UTC",
            "timePreference": "flexible",
            "interactionType": "virtual",
            "serviceTypeIds": ["svc-design"],
            "requiredSkills": ["illustrator"]
        }"#;
        let input: CreateRequestInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.requester_id, "user-1");
        assert_eq!(input.title, "Need logo design");
        assert!(input.is_public); // default_true
        assert!(input.medium_of_exchange_ids.is_empty()); // default Vec
    }

    #[test]
    fn test_exchange_type_serde() {
        let json = serde_json::to_string(&ExchangeType::TimeBanking).unwrap();
        assert_eq!(json, r#""time-banking""#);
    }

    #[test]
    fn test_listing_status_type_serde() {
        let json = serde_json::to_string(&ListingStatusType::SuspendedTemporarily).unwrap();
        assert_eq!(json, r#""suspended-temporarily""#);
    }

    #[test]
    fn test_not_implemented_response() {
        let resp = not_implemented_response("handle_create_request");
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
