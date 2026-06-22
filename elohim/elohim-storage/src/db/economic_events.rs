//! Economic events CRUD operations using Diesel with app scoping
//!
//! hREA/ValueFlows compatible event tracking for the learning economy.
//! Tracks produce, consume, transfer, use, cite, and work events.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::economic_events;
use super::models::{lamad_event_types, rea_actions, EconomicEvent, NewEconomicEvent};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating an economic event
#[derive(Debug, Clone, Deserialize)]
pub struct CreateEconomicEventInput {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub contributor_presence_id: Option<String>,
    #[serde(default)]
    pub path_id: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    /// Place ID where this event occurred (spatial grounding for carrying capacity)
    #[serde(default)]
    pub at_location: Option<String>,
    /// CID of the Collab-Qahal scope under which this event was authored.
    /// When set, the HTTP handler routes the event value through share-allocation
    /// and emits Settlement events before persisting the primary.
    #[serde(default)]
    pub scope_collab_cid: Option<String>,
    /// DHT-signal tag projected from the DNA integrity field (compute-class only).
    ///
    /// Write-through projection from the notarized DHT entry. NULL for events
    /// that carry no substrate signal.
    #[serde(default)]
    pub substrate_signal: Option<String>,
}

/// Query parameters for listing economic events - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomicEventQuery {
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by provider agent
    pub provider: Option<String>,
    /// Filter by receiver agent
    pub receiver: Option<String>,
    /// Filter by lamad event type
    pub lamad_event_type: Option<String>,
    /// Filter by content ID
    pub content_id: Option<String>,
    /// Filter by contributor presence ID
    pub contributor_presence_id: Option<String>,
    /// Filter by path ID
    pub path_id: Option<String>,
    /// Filter by event state
    pub state: Option<String>,
    /// Filter events after this timestamp
    pub after: Option<String>,
    /// Filter events before this timestamp
    pub before: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkEconomicEventResult {
    pub created: u64,
    pub errors: Vec<String>,
}

/// Aggregated event statistics
#[derive(Debug, Clone, Serialize)]
pub struct EventAggregation {
    pub total_events: i64,
    pub total_resource_quantity: f64,
    pub total_effort_quantity: f64,
    pub by_action: Vec<(String, i64)>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get economic event by ID - scoped by app
pub fn get_economic_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .filter(economic_events::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List economic events with filtering - scoped by app
pub fn list_economic_events(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &EconomicEventQuery,
) -> Result<Vec<EconomicEvent>, StorageError> {
    let mut base_query = economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    // Apply filters
    if let Some(ref action) = query.action {
        base_query = base_query.filter(economic_events::action.eq(action));
    }

    if let Some(ref provider) = query.provider {
        base_query = base_query.filter(economic_events::provider.eq(provider));
    }

    if let Some(ref receiver) = query.receiver {
        base_query = base_query.filter(economic_events::receiver.eq(receiver));
    }

    if let Some(ref lamad_type) = query.lamad_event_type {
        base_query = base_query.filter(economic_events::lamad_event_type.eq(lamad_type));
    }

    if let Some(ref content_id) = query.content_id {
        base_query = base_query.filter(economic_events::content_id.eq(content_id));
    }

    if let Some(ref presence_id) = query.contributor_presence_id {
        base_query = base_query.filter(economic_events::contributor_presence_id.eq(presence_id));
    }

    if let Some(ref path_id) = query.path_id {
        base_query = base_query.filter(economic_events::path_id.eq(path_id));
    }

    if let Some(ref state) = query.state {
        base_query = base_query.filter(economic_events::state.eq(state));
    }

    if let Some(ref after) = query.after {
        base_query = base_query.filter(economic_events::has_point_in_time.gt(after));
    }

    if let Some(ref before) = query.before {
        base_query = base_query.filter(economic_events::has_point_in_time.lt(before));
    }

    base_query
        .order(economic_events::has_point_in_time.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get events for an agent (as provider or receiver)
pub fn get_events_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .filter(
            economic_events::provider
                .eq(agent_id)
                .or(economic_events::receiver.eq(agent_id)),
        )
        .order(economic_events::has_point_in_time.desc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get events for content (engagement, citations, etc.)
pub fn get_events_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .filter(economic_events::content_id.eq(content_id))
        .order(economic_events::has_point_in_time.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get events for contributor presence (recognition flows)
pub fn get_events_for_presence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    presence_id: &str,
) -> Result<Vec<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .filter(economic_events::contributor_presence_id.eq(presence_id))
        .order(economic_events::has_point_in_time.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get events by lamad event type
pub fn get_events_by_lamad_type(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    lamad_type: &str,
    limit: i64,
) -> Result<Vec<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .filter(economic_events::lamad_event_type.eq(lamad_type))
        .order(economic_events::has_point_in_time.desc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Record an economic event - scoped by app
pub fn record_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateEconomicEventInput,
) -> Result<EconomicEvent, StorageError> {
    // Validate action
    if !rea_actions::is_valid(&input.action) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid action: {}. Valid actions: {:?}",
            input.action,
            rea_actions::ALL
        )));
    }

    // Validate lamad_event_type if provided
    if let Some(ref lamad_type) = input.lamad_event_type {
        if !lamad_event_types::is_valid(lamad_type) {
            return Err(StorageError::InvalidInput(format!(
                "Invalid lamad event type: {}. Valid types: {:?}",
                lamad_type,
                lamad_event_types::ALL
            )));
        }
    }

    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // Convert resource_classified_as to JSON
    let classified_json = if input.resource_classified_as.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&input.resource_classified_as)
                .map_err(|e| StorageError::Internal(format!("JSON serialization failed: {}", e)))?,
        )
    };

    // Default timestamp to now if not provided
    let point_in_time = input
        .has_point_in_time
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

    let new_event = NewEconomicEvent {
        id: &id,
        h_app_id: &ctx.h_app_id,
        action: &input.action,
        provider: &input.provider,
        receiver: &input.receiver,
        resource_conforms_to: input.resource_conforms_to.as_deref(),
        resource_inventoried_as: input.resource_inventoried_as.as_deref(),
        resource_classified_as_json: classified_json.as_deref(),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit.as_deref(),
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit.as_deref(),
        has_point_in_time: &point_in_time,
        has_duration: input.has_duration.as_deref(),
        input_of: input.input_of.as_deref(),
        output_of: input.output_of.as_deref(),
        lamad_event_type: input.lamad_event_type.as_deref(),
        content_id: input.content_id.as_deref(),
        contributor_presence_id: input.contributor_presence_id.as_deref(),
        path_id: input.path_id.as_deref(),
        triggered_by: input.triggered_by.as_deref(),
        state: "recorded",
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
        dht_anchor_hash: None,
        at_location: input.at_location.as_deref(),
        verified_at: None,
        scope_collab_cid: input.scope_collab_cid.as_deref(),
        bounded_by: None,
        substrate_signal: input.substrate_signal.as_deref(),
    };

    diesel::insert_into(economic_events::table)
        .values(&new_event)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_economic_event(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created event".into()))
}

/// Record a content view/consumption event
pub fn record_content_view(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    viewer_id: &str,
    content_id: &str,
    duration_seconds: Option<i64>,
) -> Result<EconomicEvent, StorageError> {
    record_event(
        conn,
        ctx,
        CreateEconomicEventInput {
            id: None,
            action: rea_actions::CONSUME.to_string(),
            provider: content_id.to_string(), // Content provides value
            receiver: viewer_id.to_string(),  // Viewer receives value
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as: vec!["learning-content".to_string()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("view".to_string()),
            effort_quantity_value: duration_seconds.map(|d| d as f32),
            effort_quantity_unit: Some("seconds".to_string()),
            has_point_in_time: None,
            has_duration: duration_seconds.map(|d| format!("{}s", d)),
            input_of: None,
            output_of: None,
            lamad_event_type: Some(lamad_event_types::CONTENT_VIEW.to_string()),
            content_id: Some(content_id.to_string()),
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            note: None,
            metadata_json: None,
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        },
    )
}

/// Record a content mastery event
pub fn record_mastery_advancement(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    learner_id: &str,
    content_id: &str,
    from_level: &str,
    to_level: &str,
) -> Result<EconomicEvent, StorageError> {
    let metadata = serde_json::json!({
        "from_level": from_level,
        "to_level": to_level,
    });

    record_event(
        conn,
        ctx,
        CreateEconomicEventInput {
            id: None,
            action: rea_actions::PRODUCE.to_string(),
            provider: learner_id.to_string(), // Learner produces mastery
            receiver: learner_id.to_string(), // Learner receives credential
            resource_conforms_to: Some("mastery-credential".to_string()),
            resource_inventoried_as: None,
            resource_classified_as: vec!["mastery".to_string(), to_level.to_string()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("level".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: Some(lamad_event_types::MASTERY_ADVANCE.to_string()),
            content_id: Some(content_id.to_string()),
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            note: Some(format!("Advanced from {} to {}", from_level, to_level)),
            metadata_json: Some(metadata.to_string()),
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        },
    )
}

/// Record a citation event
pub fn record_citation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    citer_id: &str,
    cited_content_id: &str,
    contributor_presence_id: Option<&str>,
) -> Result<EconomicEvent, StorageError> {
    record_event(
        conn,
        ctx,
        CreateEconomicEventInput {
            id: None,
            action: rea_actions::CITE.to_string(),
            provider: citer_id.to_string(),
            receiver: cited_content_id.to_string(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as: vec!["citation".to_string()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("citation".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: Some(lamad_event_types::CITATION.to_string()),
            content_id: Some(cited_content_id.to_string()),
            contributor_presence_id: contributor_presence_id.map(|s| s.to_string()),
            path_id: None,
            triggered_by: None,
            note: None,
            metadata_json: None,
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        },
    )
}

/// Record a path completion event
pub fn record_path_completion(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    learner_id: &str,
    path_id: &str,
    attestation_type: Option<&str>,
) -> Result<EconomicEvent, StorageError> {
    record_event(
        conn,
        ctx,
        CreateEconomicEventInput {
            id: None,
            action: rea_actions::PRODUCE.to_string(),
            provider: learner_id.to_string(),
            receiver: learner_id.to_string(),
            resource_conforms_to: attestation_type.map(|s| s.to_string()),
            resource_inventoried_as: None,
            resource_classified_as: vec!["path-completion".to_string()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("completion".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: Some(lamad_event_types::PATH_COMPLETION.to_string()),
            content_id: None,
            contributor_presence_id: None,
            path_id: Some(path_id.to_string()),
            triggered_by: None,
            note: None,
            metadata_json: None,
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        },
    )
}

/// Record an affinity transfer (recognition flow)
pub fn record_affinity_transfer(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    from_agent: &str,
    to_presence_id: &str,
    affinity_value: f32,
    content_id: &str,
) -> Result<EconomicEvent, StorageError> {
    record_event(
        conn,
        ctx,
        CreateEconomicEventInput {
            id: None,
            action: rea_actions::TRANSFER.to_string(),
            provider: from_agent.to_string(),
            receiver: to_presence_id.to_string(),
            resource_conforms_to: Some("affinity".to_string()),
            resource_inventoried_as: None,
            resource_classified_as: vec!["recognition".to_string()],
            resource_quantity_value: Some(affinity_value),
            resource_quantity_unit: Some("affinity".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: Some(lamad_event_types::AFFINITY_TRANSFER.to_string()),
            content_id: Some(content_id.to_string()),
            contributor_presence_id: Some(to_presence_id.to_string()),
            path_id: None,
            triggered_by: None,
            note: None,
            metadata_json: None,
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        },
    )
}

/// Bulk record events (for seeding/import) - scoped by app
pub fn bulk_record_events(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: Vec<CreateEconomicEventInput>,
) -> Result<BulkEconomicEventResult, StorageError> {
    let mut created = 0u64;
    let mut errors = vec![];

    conn.transaction(|conn| {
        for input in inputs {
            match record_event(conn, ctx, input.clone()) {
                Ok(_) => created += 1,
                Err(e) => {
                    errors.push(format!("{}: {}", input.action, e));
                }
            }
        }

        Ok(BulkEconomicEventResult { created, errors })
    })
}

/// Update event state
pub fn update_event_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    new_state: &str,
) -> Result<EconomicEvent, StorageError> {
    diesel::update(
        economic_events::table
            .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
            .filter(economic_events::id.eq(id)),
    )
    .set(economic_events::state.eq(new_state))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_economic_event(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated event".into()))
}

/// Upsert an economic event with DHT anchor hash — insert or update anchor if it already exists.
/// Used by REA projection signal handler to anchor events onto the DHT.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateEconomicEventInput,
    dht_anchor_hash: Option<&str>,
) -> Result<EconomicEvent, StorageError> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let existing = get_economic_event(conn, ctx, &id)?;

    if existing.is_some() {
        // Update dht_anchor_hash on existing record
        diesel::update(
            economic_events::table
                .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
                .filter(economic_events::id.eq(&id)),
        )
        .set(economic_events::dht_anchor_hash.eq(dht_anchor_hash))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;
    } else {
        // Convert resource_classified_as to JSON
        let classified_json = if input.resource_classified_as.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&input.resource_classified_as).map_err(|e| {
                    StorageError::Internal(format!("JSON serialization failed: {}", e))
                })?,
            )
        };

        let point_in_time = input
            .has_point_in_time
            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

        let new_event = NewEconomicEvent {
            id: &id,
            h_app_id: &ctx.h_app_id,
            action: &input.action,
            provider: &input.provider,
            receiver: &input.receiver,
            resource_conforms_to: input.resource_conforms_to.as_deref(),
            resource_inventoried_as: input.resource_inventoried_as.as_deref(),
            resource_classified_as_json: classified_json.as_deref(),
            resource_quantity_value: input.resource_quantity_value,
            resource_quantity_unit: input.resource_quantity_unit.as_deref(),
            effort_quantity_value: input.effort_quantity_value,
            effort_quantity_unit: input.effort_quantity_unit.as_deref(),
            has_point_in_time: &point_in_time,
            has_duration: input.has_duration.as_deref(),
            input_of: input.input_of.as_deref(),
            output_of: input.output_of.as_deref(),
            lamad_event_type: input.lamad_event_type.as_deref(),
            content_id: input.content_id.as_deref(),
            contributor_presence_id: input.contributor_presence_id.as_deref(),
            path_id: input.path_id.as_deref(),
            triggered_by: input.triggered_by.as_deref(),
            state: "recorded",
            note: input.note.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
            dht_anchor_hash,
            at_location: input.at_location.as_deref(),
            verified_at: None,
            scope_collab_cid: input.scope_collab_cid.as_deref(),
            bounded_by: None,
            substrate_signal: input.substrate_signal.as_deref(),
        };

        diesel::insert_into(economic_events::table)
            .values(&new_event)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_economic_event(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve upserted event".into()))
}

// ============================================================================
// Stats & Aggregations
// ============================================================================

/// Get economic event count for an app
pub fn event_count(conn: &mut SqliteConnection, ctx: &AppContext) -> Result<i64, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get event statistics by action
pub fn stats_by_action(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .group_by(economic_events::action)
        .select((economic_events::action, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

/// Get event statistics by lamad event type
pub fn stats_by_lamad_type(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(Option<String>, i64)>, StorageError> {
    economic_events::table
        .filter(economic_events::h_app_id.eq(&ctx.h_app_id))
        .group_by(economic_events::lamad_event_type)
        .select((economic_events::lamad_event_type, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

/// Get aggregate event stats for a content item
pub fn aggregate_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<EventAggregation, StorageError> {
    let events = get_events_for_content(conn, ctx, content_id)?;

    let total_events = events.len() as i64;
    let total_resource_quantity: f64 = events
        .iter()
        .filter_map(|e| e.resource_quantity_value)
        .map(|v| v as f64)
        .sum();
    let total_effort_quantity: f64 = events
        .iter()
        .filter_map(|e| e.effort_quantity_value)
        .map(|v| v as f64)
        .sum();

    let mut by_action: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for event in &events {
        *by_action.entry(event.action.clone()).or_insert(0) += 1;
    }

    Ok(EventAggregation {
        total_events,
        total_resource_quantity,
        total_effort_quantity,
        by_action: by_action.into_iter().collect(),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn test_ctx() -> AppContext {
        AppContext::new("test-app")
    }

    /// Build a minimal valid CreateEconomicEventInput for "use" action.
    fn minimal_input() -> CreateEconomicEventInput {
        CreateEconomicEventInput {
            id: None,
            action: "use".to_string(),
            provider: "provider-test".to_string(),
            receiver: "receiver-test".to_string(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as: vec![],
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            note: None,
            metadata_json: None,
            at_location: None,
            scope_collab_cid: None,
            substrate_signal: None,
        }
    }

    #[test]
    fn substrate_signal_persists_through_record_event() {
        let mut conn = test_conn();
        let ctx = test_ctx();

        let input = CreateEconomicEventInput {
            substrate_signal: Some("attention".to_string()),
            ..minimal_input()
        };

        let row = record_event(&mut conn, &ctx, input).expect("record_event must succeed");

        let got: Option<String> = economic_events::table
            .find(&row.id)
            .select(economic_events::substrate_signal)
            .first(&mut conn)
            .expect("select substrate_signal");

        assert_eq!(got.as_deref(), Some("attention"));
    }

    #[test]
    fn absent_substrate_signal_persists_as_null() {
        let mut conn = test_conn();
        let ctx = test_ctx();

        let input = CreateEconomicEventInput {
            substrate_signal: None,
            ..minimal_input()
        };

        let row = record_event(&mut conn, &ctx, input).expect("record_event must succeed");

        let got: Option<String> = economic_events::table
            .find(&row.id)
            .select(economic_events::substrate_signal)
            .first(&mut conn)
            .expect("select substrate_signal");

        assert_eq!(got, None);
    }

    /// Robustness guard / DOCUMENTED NEGATIVE RESULT.
    ///
    /// Investigation of the live bare-`GET /api/v1/economic-events` HTTP 500
    /// (filtered queries returned 200 `[]`) hypothesised that one row in the
    /// `economic_events` projection failed diesel deserialization in the
    /// unfiltered `.load::<EconomicEvent>()` at line 203. This test probes the
    /// strongest candidate shapes and proves the `.load()` path is NOT the
    /// cause: diesel 2.3.5's SQLite decoder coerces every storage class on read,
    /// so no single-column value can break decode for this table's column types.
    ///
    /// Shapes probed (all decode cleanly — `list_economic_events` returns `Ok`):
    ///   - invalid-UTF-8 BLOB (`x'ff'`) in the TEXT-affinity `note` column.
    ///     diesel `SqliteValue::parse_string` reads via `sqlite3_value_text`
    ///     (which coerces) then `str::from_utf8_unchecked` (which does NOT
    ///     validate) — so a TEXT column NEVER yields a clean decode error.
    ///   - non-numeric TEXT (`'not-a-number'`) in the REAL-affinity
    ///     `resource_quantity_value`: SQLite coerces it to `0.0` on read.
    ///
    /// Not probed (not constructible): NULL in a NOT NULL column — SQLite
    /// rejects it at insert; and the model is fully `Option<T>` for every
    /// nullable column, so NULL-into-non-Option cannot occur. The deployed
    /// alpha binary's `EconomicEvent` struct is byte-identical to this one
    /// (commit `72a62343`), so the 500 is NOT a row-decode failure here.
    #[test]
    fn list_load_path_decodes_every_constructible_row_shape() {
        let mut conn = test_conn();
        let ctx = test_ctx();

        // A clean row that the bare list is expected to surface.
        let good = CreateEconomicEventInput {
            id: Some("good-row".to_string()),
            ..minimal_input()
        };
        record_event(&mut conn, &ctx, good).expect("record clean row");

        // Shape 1: invalid-UTF-8 BLOB in a TEXT-affinity column.
        diesel::sql_query(
            "INSERT INTO economic_events \
             (id, h_app_id, action, provider, receiver, \
              note, has_point_in_time, state, created_at) \
             VALUES \
             ('blob-note-row', 'test-app', 'use', 'p', 'r', \
              x'ff', '2026-06-22T00:00:00Z', 'recorded', '2026-06-22T00:00:00Z')",
        )
        .execute(&mut conn)
        .expect("raw insert of invalid-utf-8 BLOB row");

        // Shape 2: non-numeric TEXT in a REAL-affinity column.
        diesel::sql_query(
            "INSERT INTO economic_events \
             (id, h_app_id, action, provider, receiver, \
              resource_quantity_value, has_point_in_time, state, created_at) \
             VALUES \
             ('text-qty-row', 'test-app', 'use', 'p', 'r', \
              'not-a-number', '2026-06-22T00:00:00Z', 'recorded', '2026-06-22T00:00:00Z')",
        )
        .execute(&mut conn)
        .expect("raw insert of non-numeric-in-REAL row");

        let query = EconomicEventQuery {
            limit: 100,
            ..Default::default()
        };

        // The db-layer load tolerates both malformed shapes (diesel coerces).
        let result = list_economic_events(&mut conn, &ctx, &query);
        assert!(
            result.is_ok(),
            "the .load() path must decode every constructible row shape; got {:?}",
            result.err()
        );
        let events = result.unwrap();
        assert!(
            events.iter().any(|e| e.id == "good-row"),
            "clean row must survive alongside the malformed sibling rows"
        );
    }
}
