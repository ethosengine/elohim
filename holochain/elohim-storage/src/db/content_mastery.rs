//! Content mastery CRUD operations using Diesel with app scoping
//!
//! Tracks learning progress using Bloom's taxonomy with spaced repetition
//! freshness decay for optimal review scheduling.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::content_mastery;
use super::models::{ContentMastery, NewContentMastery, mastery_levels, current_timestamp};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating/updating mastery
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMasteryInput {
    #[serde(default)]
    pub id: Option<String>,
    pub human_id: String,
    pub content_id: String,
    #[serde(default = "default_mastery_level")]
    pub mastery_level: String,
    #[serde(default)]
    pub content_version_at_mastery: Option<String>,
}

fn default_mastery_level() -> String { mastery_levels::NOT_STARTED.to_string() }

/// Query parameters for listing mastery records
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MasteryQuery {
    /// Filter by human ID
    pub human_id: Option<String>,
    /// Filter by content ID
    pub content_id: Option<String>,
    /// Filter by mastery level
    pub mastery_level: Option<String>,
    /// Filter by minimum mastery level index
    pub min_mastery_level: Option<String>,
    /// Filter for items needing refresh
    pub needs_refresh: Option<bool>,
    /// Filter by freshness below threshold
    pub freshness_below: Option<f32>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkMasteryResult {
    pub created: u64,
    pub updated: u64,
    pub errors: Vec<String>,
}

/// Mastery advancement result
#[derive(Debug, Clone, Serialize)]
pub struct MasteryAdvancement {
    pub mastery: ContentMastery,
    pub previous_level: String,
    pub new_level: String,
    pub is_advancement: bool,
}

/// Path mastery summary
#[derive(Debug, Clone, Serialize)]
pub struct PathMasterySummary {
    pub path_id: String,
    pub human_id: String,
    pub total_content: usize,
    pub mastered_content: usize,
    pub average_mastery_index: f32,
    pub average_freshness: f32,
    pub needs_refresh_count: usize,
    pub by_level: Vec<(String, usize)>,
}

// ============================================================================
// Constants
// ============================================================================

/// Freshness decay rate per day (default: lose 5% freshness per day)
const FRESHNESS_DECAY_PER_DAY: f32 = 0.05;

/// Freshness threshold below which needs_refresh is set
const REFRESH_THRESHOLD: f32 = 0.5;

// ============================================================================
// Read Operations
// ============================================================================

/// Get mastery by ID - scoped by app
pub fn get_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<ContentMastery>, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get mastery for a specific human+content pair - scoped by app
pub fn get_mastery_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_id: &str,
) -> Result<Option<ContentMastery>, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::human_id.eq(human_id))
        .filter(content_mastery::content_id.eq(content_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List mastery records with filtering - scoped by app
pub fn list_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &MasteryQuery,
) -> Result<Vec<ContentMastery>, StorageError> {
    let mut base_query = content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .into_boxed();

    // Apply filters
    if let Some(ref human_id) = query.human_id {
        base_query = base_query.filter(content_mastery::human_id.eq(human_id));
    }

    if let Some(ref content_id) = query.content_id {
        base_query = base_query.filter(content_mastery::content_id.eq(content_id));
    }

    if let Some(ref level) = query.mastery_level {
        base_query = base_query.filter(content_mastery::mastery_level.eq(level));
    }

    // Filter by minimum mastery level using index
    if let Some(ref min_level) = query.min_mastery_level {
        if let Some(min_index) = mastery_levels::index_of(min_level) {
            base_query = base_query.filter(content_mastery::mastery_level_index.ge(min_index as i32));
        }
    }

    if query.needs_refresh == Some(true) {
        base_query = base_query.filter(content_mastery::needs_refresh.eq(1));
    }

    if let Some(threshold) = query.freshness_below {
        base_query = base_query.filter(content_mastery::freshness_score.lt(threshold));
    }

    base_query
        .order(content_mastery::updated_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all mastery records for a human - scoped by app
pub fn get_mastery_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<Vec<ContentMastery>, StorageError> {
    list_mastery(conn, ctx, &MasteryQuery {
        human_id: Some(human_id.to_string()),
        limit: 10000, // High limit for full mastery profile
        ..Default::default()
    })
}

/// Get content items needing refresh for a human
pub fn get_refresh_needed(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    limit: i64,
) -> Result<Vec<ContentMastery>, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::human_id.eq(human_id))
        .filter(content_mastery::needs_refresh.eq(1))
        .order(content_mastery::freshness_score.asc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get mastery for multiple content IDs (for path progress)
pub fn get_mastery_for_contents(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_ids: &[String],
) -> Result<Vec<ContentMastery>, StorageError> {
    if content_ids.is_empty() {
        return Ok(vec![]);
    }

    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::human_id.eq(human_id))
        .filter(content_mastery::content_id.eq_any(content_ids))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create or update mastery record - scoped by app
pub fn upsert_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateMasteryInput,
) -> Result<ContentMastery, StorageError> {
    // Validate mastery level
    if !mastery_levels::is_valid(&input.mastery_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid mastery level: {}. Valid levels: {:?}",
            input.mastery_level,
            mastery_levels::ALL
        )));
    }

    let mastery_index = mastery_levels::index_of(&input.mastery_level)
        .unwrap_or(0) as i32;

    // Check if exists
    let existing = get_mastery_for_content(conn, ctx, &input.human_id, &input.content_id)?;

    if let Some(existing) = existing {
        // Update existing
        diesel::update(
            content_mastery::table
                .filter(content_mastery::app_id.eq(&ctx.app_id))
                .filter(content_mastery::id.eq(&existing.id))
        )
        .set((
            content_mastery::mastery_level.eq(&input.mastery_level),
            content_mastery::mastery_level_index.eq(mastery_index),
            content_mastery::content_version_at_mastery.eq(input.content_version_at_mastery.as_deref()),
            content_mastery::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        get_mastery(conn, ctx, &existing.id)?
            .ok_or_else(|| StorageError::Internal("Failed to retrieve updated mastery".into()))
    } else {
        // Create new
        let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let new_mastery = NewContentMastery {
            id: &id,
            app_id: &ctx.app_id,
            human_id: &input.human_id,
            content_id: &input.content_id,
            mastery_level: &input.mastery_level,
            mastery_level_index: mastery_index,
            content_version_at_mastery: input.content_version_at_mastery.as_deref(),
        };

        diesel::insert_into(content_mastery::table)
            .values(&new_mastery)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        get_mastery(conn, ctx, &id)?
            .ok_or_else(|| StorageError::Internal("Failed to retrieve created mastery".into()))
    }
}

/// Record an engagement (view, practice, quiz, etc.)
pub fn record_engagement(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_id: &str,
    engagement_type: &str,
) -> Result<ContentMastery, StorageError> {
    // Get or create mastery record
    let mastery = get_mastery_for_content(conn, ctx, human_id, content_id)?;

    let (id, current_count) = match mastery {
        Some(m) => (m.id.clone(), m.engagement_count),
        None => {
            // Create initial mastery
            let created = upsert_mastery(conn, ctx, CreateMasteryInput {
                id: None,
                human_id: human_id.to_string(),
                content_id: content_id.to_string(),
                mastery_level: mastery_levels::REMEMBER.to_string(), // First engagement = remember
                content_version_at_mastery: None,
            })?;
            (created.id, 0)
        }
    };

    // Update engagement stats and refresh freshness
    diesel::update(
        content_mastery::table
            .filter(content_mastery::app_id.eq(&ctx.app_id))
            .filter(content_mastery::id.eq(&id))
    )
    .set((
        content_mastery::engagement_count.eq(current_count + 1),
        content_mastery::last_engagement_type.eq(engagement_type),
        content_mastery::last_engagement_at.eq(current_timestamp()),
        content_mastery::freshness_score.eq(1.0f32), // Reset freshness on engagement
        content_mastery::needs_refresh.eq(0),
        content_mastery::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_mastery(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated mastery".into()))
}

/// Advance mastery level (with evidence)
pub fn advance_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_id: &str,
    new_level: &str,
    assessment_evidence: Option<&str>,
) -> Result<MasteryAdvancement, StorageError> {
    // Validate new level
    if !mastery_levels::is_valid(new_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid mastery level: {}. Valid levels: {:?}",
            new_level,
            mastery_levels::ALL
        )));
    }

    let new_index = mastery_levels::index_of(new_level).unwrap_or(0) as i32;

    // Get or create mastery
    let mastery = match get_mastery_for_content(conn, ctx, human_id, content_id)? {
        Some(m) => m,
        None => {
            upsert_mastery(conn, ctx, CreateMasteryInput {
                id: None,
                human_id: human_id.to_string(),
                content_id: content_id.to_string(),
                mastery_level: mastery_levels::NOT_STARTED.to_string(),
                content_version_at_mastery: None,
            })?
        }
    };

    let previous_level = mastery.mastery_level.clone();
    let previous_index = mastery.mastery_level_index;
    let is_advancement = new_index > previous_index;

    // Update mastery
    diesel::update(
        content_mastery::table
            .filter(content_mastery::app_id.eq(&ctx.app_id))
            .filter(content_mastery::id.eq(&mastery.id))
    )
    .set((
        content_mastery::mastery_level.eq(new_level),
        content_mastery::mastery_level_index.eq(new_index),
        content_mastery::level_achieved_at.eq(current_timestamp()),
        content_mastery::assessment_evidence_json.eq(assessment_evidence),
        content_mastery::freshness_score.eq(1.0f32),
        content_mastery::needs_refresh.eq(0),
        content_mastery::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    let updated = get_mastery(conn, ctx, &mastery.id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated mastery".into()))?;

    Ok(MasteryAdvancement {
        mastery: updated,
        previous_level,
        new_level: new_level.to_string(),
        is_advancement,
    })
}

/// Update freshness scores based on time decay
///
/// Call this periodically (e.g., daily) to decay freshness and mark items needing refresh.
pub fn apply_freshness_decay(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    days_elapsed: f32,
) -> Result<usize, StorageError> {
    let decay_factor = 1.0 - (FRESHNESS_DECAY_PER_DAY * days_elapsed);
    let decay_factor = decay_factor.max(0.0); // Don't go negative

    // Update all mastery records with decay
    // Note: SQLite doesn't support UPDATE with computed values easily,
    // so we fetch, compute, and update
    let records: Vec<ContentMastery> = content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::freshness_score.gt(0.0f32))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let mut updated_count = 0;

    for record in records {
        let new_freshness = (record.freshness_score * decay_factor).max(0.0);
        let needs_refresh = if new_freshness < REFRESH_THRESHOLD { 1 } else { 0 };

        diesel::update(
            content_mastery::table
                .filter(content_mastery::id.eq(&record.id))
        )
        .set((
            content_mastery::freshness_score.eq(new_freshness),
            content_mastery::needs_refresh.eq(needs_refresh),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        updated_count += 1;
    }

    Ok(updated_count)
}

/// Grant privileges based on mastery level
pub fn update_privileges(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    privileges_json: &str,
) -> Result<ContentMastery, StorageError> {
    diesel::update(
        content_mastery::table
            .filter(content_mastery::app_id.eq(&ctx.app_id))
            .filter(content_mastery::id.eq(id))
    )
    .set((
        content_mastery::privileges_json.eq(privileges_json),
        content_mastery::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_mastery(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated mastery".into()))
}

/// Delete a mastery record by ID - scoped by app
pub fn delete_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        content_mastery::table
            .filter(content_mastery::app_id.eq(&ctx.app_id))
            .filter(content_mastery::id.eq(id))
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Path Progress
// ============================================================================

/// Calculate path mastery summary for a human
pub fn calculate_path_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    path_id: &str,
    content_ids: &[String],
) -> Result<PathMasterySummary, StorageError> {
    let mastery_records = get_mastery_for_contents(conn, ctx, human_id, content_ids)?;

    let total_content = content_ids.len();
    let mastered_content = mastery_records.iter()
        .filter(|m| mastery_levels::is_mastered(&m.mastery_level))
        .count();

    let average_mastery_index = if mastery_records.is_empty() {
        0.0
    } else {
        mastery_records.iter()
            .map(|m| m.mastery_level_index as f32)
            .sum::<f32>() / mastery_records.len() as f32
    };

    let average_freshness = if mastery_records.is_empty() {
        1.0
    } else {
        mastery_records.iter()
            .map(|m| m.freshness_score)
            .sum::<f32>() / mastery_records.len() as f32
    };

    let needs_refresh_count = mastery_records.iter()
        .filter(|m| m.needs_refresh == 1)
        .count();

    // Count by level
    let mut by_level: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for record in &mastery_records {
        *by_level.entry(record.mastery_level.clone()).or_insert(0) += 1;
    }
    // Add not_started count for content without mastery records
    let tracked_count = mastery_records.len();
    if tracked_count < total_content {
        by_level.insert(
            mastery_levels::NOT_STARTED.to_string(),
            total_content - tracked_count,
        );
    }

    Ok(PathMasterySummary {
        path_id: path_id.to_string(),
        human_id: human_id.to_string(),
        total_content,
        mastered_content,
        average_mastery_index,
        average_freshness,
        needs_refresh_count,
        by_level: by_level.into_iter().collect(),
    })
}

// ============================================================================
// Stats
// ============================================================================

/// Get mastery record count for an app
pub fn mastery_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get mastery statistics by level
pub fn stats_by_level(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .group_by(content_mastery::mastery_level)
        .select((content_mastery::mastery_level, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

/// Get count of items needing refresh
pub fn refresh_needed_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .filter(content_mastery::needs_refresh.eq(1))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get average freshness across all mastery records
pub fn average_freshness(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<f64, StorageError> {
    content_mastery::table
        .filter(content_mastery::app_id.eq(&ctx.app_id))
        .select(diesel::dsl::avg(content_mastery::freshness_score))
        .first::<Option<f64>>(conn)
        .map(|v| v.unwrap_or(1.0))
        .map_err(|e| StorageError::Internal(format!("Avg query failed: {}", e)))
}

// ============================================================================
// Bulk Operations
// ============================================================================

/// Bulk create/update mastery records (for seeding/import) - scoped by app
pub fn bulk_upsert_mastery(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: Vec<CreateMasteryInput>,
) -> Result<BulkMasteryResult, StorageError> {
    let mut created = 0u64;
    let mut updated = 0u64;
    let mut errors = vec![];

    conn.transaction(|conn| {
        for input in inputs {
            // Check if exists
            let existing = get_mastery_for_content(conn, ctx, &input.human_id, &input.content_id)
                .map_err(|e| diesel::result::Error::RollbackTransaction)?;

            match existing {
                Some(_) => {
                    match upsert_mastery(conn, ctx, input.clone()) {
                        Ok(_) => updated += 1,
                        Err(e) => {
                            errors.push(format!("{}/{}: {}", input.human_id, input.content_id, e));
                        }
                    }
                }
                None => {
                    match upsert_mastery(conn, ctx, input.clone()) {
                        Ok(_) => created += 1,
                        Err(e) => {
                            errors.push(format!("{}/{}: {}", input.human_id, input.content_id, e));
                        }
                    }
                }
            }
        }

        Ok(BulkMasteryResult { created, updated, errors })
    })
}
