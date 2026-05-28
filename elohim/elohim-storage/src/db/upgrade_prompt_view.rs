//! UpgradePromptView projection — M-AGGR-1.
//!
//! Source of truth: derived projection of SessionHumanView ×
//! Manifest{kind:"onboarding"} payload for a specific agent.
//! Category C operational — reconstructable from EconomicEvent stream +
//! onboarding Manifest at any time by re-evaluating trigger conditions against
//! the current SessionHumanView.
//!
//! The active_prompts_json column stores a JSON array of UpgradePromptItem
//! objects per the schema at:
//!   elohim/sdk/schemas/v1/views/upgrade-prompt-view.schema.json
//!
//! The projection is (re)computed on two triggers:
//! 1. Signal::EconomicEventCreated (qualifying lamadEventType) — after
//!    SessionHumanView is updated, this agent's upgrade prompts are re-evaluated.
//! 2. Signal::ManifestUpdated{kind:"onboarding"} — re-evaluates for ALL agents
//!    (prompts may have been added/changed/removed in the Manifest).
//!
//! Projection writer: `project_upgrade_prompt_view`
//! Fetch: `fetch_upgrade_prompt_view`

use diesel::prelude::*;
use serde::Deserialize;

use crate::db::diesel_schema::{manifests, upgrade_prompt_view};
use crate::error::StorageError;
use elohim_views::{SessionHumanView, UpgradePromptItem, UpgradePromptView};

// ---------------------------------------------------------------------------
// Onboarding Manifest payload shape (internal — for JSON deserialization)
// ---------------------------------------------------------------------------

const MANIFEST_KIND_ONBOARDING: &str = "onboarding";

#[derive(Debug, Deserialize)]
struct OnboardingPromptDef {
    #[serde(rename = "promptId")]
    prompt_id: String,
    trigger: String,
    #[serde(rename = "triggerCondition")]
    trigger_condition: TriggerCondition,
    title: String,
    message: String,
    benefits: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TriggerCondition {
    field: String,
    operator: String,
    #[serde(default)]
    threshold: i64,
}

#[derive(Debug, Deserialize)]
struct OnboardingPayload {
    prompts: Vec<OnboardingPromptDef>,
}

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = upgrade_prompt_view)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpgradePromptViewRow {
    pub agent_id: String,
    pub h_app_id: String,
    pub active_prompts_json: String,
    pub computed_at: String,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = upgrade_prompt_view)]
pub struct UpsertUpgradePromptView<'a> {
    pub agent_id: &'a str,
    pub h_app_id: &'a str,
    pub active_prompts_json: &'a str,
    pub computed_at: &'a str,
}

// ---------------------------------------------------------------------------
// Trigger condition evaluation
// ---------------------------------------------------------------------------

fn field_value(session: &SessionHumanView, field: &str) -> i64 {
    match field {
        "nodesViewed" | "nodes_viewed" => session.nodes_viewed,
        "nodesWithAffinity" | "nodes_with_affinity" => session.nodes_with_affinity,
        "pathsStarted" | "paths_started" => session.paths_started,
        "pathsCompleted" | "paths_completed" => session.paths_completed,
        "stepsCompleted" | "steps_completed" => session.steps_completed,
        _ => 0,
    }
}

fn evaluate_condition(condition: &TriggerCondition, session: &SessionHumanView) -> bool {
    match condition.operator.as_str() {
        "always" => true,
        "gte" => field_value(session, &condition.field) >= condition.threshold,
        "gt" => field_value(session, &condition.field) > condition.threshold,
        "eq" => field_value(session, &condition.field) == condition.threshold,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Core projection logic
// ---------------------------------------------------------------------------

/// Load the onboarding Manifest payload from the manifests projection table.
///
/// Returns None if no onboarding Manifest has been projected yet.
/// The manifests table is keyed by `cid`; `manifest_kind` is the discriminator.
fn load_onboarding_payload(
    conn: &mut SqliteConnection,
) -> Result<Option<OnboardingPayload>, StorageError> {
    // manifests table uses manifest_kind column; keyed by cid; no h_app_id scope.
    // Use the most recently created onboarding manifest (highest created_at).
    let payload_json: Option<String> = manifests::table
        .filter(manifests::manifest_kind.eq(MANIFEST_KIND_ONBOARDING))
        .order(manifests::created_at.desc())
        .select(manifests::payload_json)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("load_onboarding_manifest: {e}")))?;

    match payload_json {
        None => Ok(None),
        Some(json) => serde_json::from_str::<OnboardingPayload>(&json)
            .map(Some)
            .map_err(|e| {
                StorageError::InvalidInput(format!("onboarding Manifest payload invalid JSON: {e}"))
            }),
    }
}

/// Recompute and upsert the UpgradePromptView row for a single agent.
///
/// Evaluates the onboarding Manifest trigger conditions against the provided
/// SessionHumanView and upserts the active prompts.
///
/// Called after SessionHumanView is updated (Signal::EconomicEventCreated) or
/// when the onboarding Manifest changes (Signal::ManifestUpdated).
pub fn project_upgrade_prompt_view(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
    session: &SessionHumanView,
) -> Result<UpgradePromptView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    // Load the onboarding manifest. If none exists, active_prompts is empty.
    let active_prompts: Vec<UpgradePromptItem> = match load_onboarding_payload(conn)? {
        None => vec![],
        Some(payload) => payload
            .prompts
            .into_iter()
            .filter(|def| evaluate_condition(&def.trigger_condition, session))
            .map(|def| UpgradePromptItem {
                prompt_id: def.prompt_id,
                trigger: def.trigger,
                title: def.title,
                message: def.message,
                benefits: def.benefits,
            })
            .collect(),
    };

    let active_prompts_json = serde_json::to_string(&active_prompts)
        .map_err(|e| StorageError::InvalidInput(format!("upgrade_prompt_view serialize: {e}")))?;

    let row = UpsertUpgradePromptView {
        agent_id,
        h_app_id,
        active_prompts_json: &active_prompts_json,
        computed_at: &now,
    };

    diesel::insert_into(upgrade_prompt_view::table)
        .values(&row)
        .on_conflict((upgrade_prompt_view::agent_id, upgrade_prompt_view::h_app_id))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("upgrade_prompt_view upsert: {e}")))?;

    Ok(UpgradePromptView {
        agent_id: agent_id.to_string(),
        h_app_id: h_app_id.to_string(),
        active_prompts,
        computed_at: now,
    })
}

/// Fetch an already-projected UpgradePromptView row by agent_id.
///
/// Returns a view with empty active_prompts if no row exists yet.
pub fn fetch_upgrade_prompt_view(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
) -> Result<UpgradePromptView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    let row = upgrade_prompt_view::table
        .filter(upgrade_prompt_view::agent_id.eq(agent_id))
        .filter(upgrade_prompt_view::h_app_id.eq(h_app_id))
        .select(UpgradePromptViewRow::as_select())
        .first::<UpgradePromptViewRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("fetch_upgrade_prompt_view: {e}")))?;

    match row {
        None => Ok(UpgradePromptView {
            agent_id: agent_id.to_string(),
            h_app_id: h_app_id.to_string(),
            active_prompts: vec![],
            computed_at: now,
        }),
        Some(r) => {
            let active_prompts: Vec<UpgradePromptItem> =
                serde_json::from_str(&r.active_prompts_json).unwrap_or_default();
            Ok(UpgradePromptView {
                agent_id: r.agent_id,
                h_app_id: r.h_app_id,
                active_prompts,
                computed_at: r.computed_at,
            })
        }
    }
}

// Note: UpgradePromptItem has #[derive(Serialize)] from elohim-views, so
// serde_json::to_string(&active_prompts) works without any additional impls.
