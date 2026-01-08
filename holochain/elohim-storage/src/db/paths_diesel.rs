//! Path and Step CRUD operations using Diesel with app scoping
//!
//! All operations require an AppContext for multi-tenant isolation.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::context::AppContext;
use super::diesel_schema::{chapters, path_attestations, path_tags, paths, steps};
use super::models::{
    Chapter, ChapterWithSteps, NewChapter, NewPath, NewPathAttestation, NewPathTag, NewStep, Path,
    PathAttestation, PathTag, PathWithDetails, PathWithSteps, Step,
};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a path
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePathInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_path_type")]
    pub path_type: String,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub thumbnail_alt: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<CreateChapterInput>,
    #[serde(default)]
    pub attestations: Vec<CreateAttestationInput>,
}

fn default_path_type() -> String {
    "guided".to_string()
}
fn default_visibility() -> String {
    "public".to_string()
}

/// Input for creating a chapter
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChapterInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub steps: Vec<CreateStepInput>,
}

/// Input for creating a step
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStepInput {
    pub id: String,
    #[serde(default)]
    pub chapter_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_step_type")]
    pub step_type: String,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub resource_type: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_step_type() -> String {
    "learn".to_string()
}

/// Input for creating an attestation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAttestationInput {
    pub attestation_type: String,
    pub attestation_name: String,
}

/// Result of bulk path operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkPathResult {
    pub inserted: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get path by ID - scoped by app
pub fn get_path(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_id: &str,
) -> Result<Option<Path>, StorageError> {
    paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .filter(paths::id.eq(path_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get path with all details (chapters, steps, tags, attestations) - scoped by app
pub fn get_path_with_details(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_id: &str,
) -> Result<Option<PathWithDetails>, StorageError> {
    let path_opt: Option<Path> = paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .filter(paths::id.eq(path_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let path = match path_opt {
        Some(p) => p,
        None => return Ok(None),
    };

    // Load tags
    let tags: Vec<String> = path_tags::table
        .filter(path_tags::app_id.eq(&ctx.app_id))
        .filter(path_tags::path_id.eq(path_id))
        .select(path_tags::tag)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Tags query failed: {}", e)))?;

    // Load chapters with steps
    let path_chapters: Vec<Chapter> = chapters::table
        .filter(chapters::app_id.eq(&ctx.app_id))
        .filter(chapters::path_id.eq(path_id))
        .order(chapters::order_index.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Chapters query failed: {}", e)))?;

    let mut chapters_with_steps = Vec::with_capacity(path_chapters.len());
    for chapter in path_chapters {
        let chapter_steps: Vec<Step> = steps::table
            .filter(steps::app_id.eq(&ctx.app_id))
            .filter(steps::chapter_id.eq(&chapter.id))
            .order(steps::order_index.asc())
            .load(conn)
            .map_err(|e| StorageError::Internal(format!("Steps query failed: {}", e)))?;

        chapters_with_steps.push(ChapterWithSteps {
            chapter,
            steps: chapter_steps,
        });
    }

    // Load ungrouped steps (no chapter)
    let ungrouped: Vec<Step> = steps::table
        .filter(steps::app_id.eq(&ctx.app_id))
        .filter(steps::path_id.eq(path_id))
        .filter(steps::chapter_id.is_null())
        .order(steps::order_index.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Ungrouped steps query failed: {}", e)))?;

    // Load attestations
    let attestations: Vec<PathAttestation> = path_attestations::table
        .filter(path_attestations::app_id.eq(&ctx.app_id))
        .filter(path_attestations::path_id.eq(path_id))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Attestations query failed: {}", e)))?;

    Ok(Some(PathWithDetails {
        path,
        tags,
        chapters: chapters_with_steps,
        ungrouped_steps: ungrouped,
        attestations,
    }))
}

/// Get path with steps only (flat list) - scoped by app
pub fn get_path_with_steps(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_id: &str,
) -> Result<Option<PathWithSteps>, StorageError> {
    let path_opt: Option<Path> = paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .filter(paths::id.eq(path_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let path = match path_opt {
        Some(p) => p,
        None => return Ok(None),
    };

    let path_steps: Vec<Step> = steps::table
        .filter(steps::app_id.eq(&ctx.app_id))
        .filter(steps::path_id.eq(path_id))
        .order(steps::order_index.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Steps query failed: {}", e)))?;

    Ok(Some(PathWithSteps {
        path,
        steps: path_steps,
    }))
}

/// List all paths - scoped by app
pub fn list_paths(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Path>, StorageError> {
    paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .order(paths::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get step count for a path - scoped by app
pub fn get_step_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_id: &str,
) -> Result<i64, StorageError> {
    steps::table
        .filter(steps::app_id.eq(&ctx.app_id))
        .filter(steps::path_id.eq(path_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a path with chapters and steps - scoped by app
pub fn create_path(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreatePathInput,
) -> Result<Path, StorageError> {
    conn.transaction(|conn| {
        // Insert path
        let new_path = NewPath {
            id: &input.id,
            app_id: &ctx.app_id,
            title: &input.title,
            description: input.description.as_deref(),
            path_type: &input.path_type,
            difficulty: input.difficulty.as_deref(),
            estimated_duration: input.estimated_duration.as_deref(),
            thumbnail_url: input.thumbnail_url.as_deref(),
            thumbnail_alt: input.thumbnail_alt.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
            visibility: &input.visibility,
            created_by: input.created_by.as_deref(),
        };

        diesel::insert_into(paths::table)
            .values(&new_path)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Path insert failed: {}", e)))?;

        // Insert tags
        for tag in &input.tags {
            let new_tag = NewPathTag {
                app_id: &ctx.app_id,
                path_id: &input.id,
                tag,
            };
            diesel::insert_or_ignore_into(path_tags::table)
                .values(&new_tag)
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
        }

        // Insert attestations
        for att in &input.attestations {
            let new_att = NewPathAttestation {
                app_id: &ctx.app_id,
                path_id: &input.id,
                attestation_type: &att.attestation_type,
                attestation_name: &att.attestation_name,
            };
            diesel::insert_or_ignore_into(path_attestations::table)
                .values(&new_att)
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Attestation insert failed: {}", e)))?;
        }

        // Insert chapters and their steps
        for chapter_input in &input.chapters {
            let new_chapter = NewChapter {
                id: &chapter_input.id,
                app_id: &ctx.app_id,
                path_id: &input.id,
                title: &chapter_input.title,
                description: chapter_input.description.as_deref(),
                order_index: chapter_input.order_index,
                estimated_duration: chapter_input.estimated_duration.as_deref(),
            };

            diesel::insert_into(chapters::table)
                .values(&new_chapter)
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Chapter insert failed: {}", e)))?;

            // Insert steps for this chapter
            for step_input in &chapter_input.steps {
                let new_step = NewStep {
                    id: &step_input.id,
                    app_id: &ctx.app_id,
                    path_id: &input.id,
                    chapter_id: Some(&chapter_input.id),
                    title: &step_input.title,
                    description: step_input.description.as_deref(),
                    step_type: &step_input.step_type,
                    resource_id: step_input.resource_id.as_deref(),
                    resource_type: step_input.resource_type.as_deref(),
                    order_index: step_input.order_index,
                    estimated_duration: step_input.estimated_duration.as_deref(),
                    metadata_json: step_input.metadata_json.as_deref(),
                };

                diesel::insert_into(steps::table)
                    .values(&new_step)
                    .execute(conn)
                    .map_err(|e| StorageError::Internal(format!("Step insert failed: {}", e)))?;
            }
        }

        // Return created path
        paths::table
            .filter(paths::app_id.eq(&ctx.app_id))
            .filter(paths::id.eq(&input.id))
            .first(conn)
            .map_err(|e| StorageError::Internal(format!("Fetch failed: {}", e)))
    })
}

/// Bulk create paths (for seeding) - scoped by app
pub fn bulk_create_paths(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_inputs: Vec<CreatePathInput>,
) -> Result<BulkPathResult, StorageError> {
    let mut inserted = 0u64;
    let mut skipped = 0u64;
    let mut errors = vec![];

    for input in path_inputs {
        // Check if exists
        let exists: bool = paths::table
            .filter(paths::app_id.eq(&ctx.app_id))
            .filter(paths::id.eq(&input.id))
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map(|c| c > 0)
            .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        match create_path(conn, ctx, input.clone()) {
            Ok(_) => inserted += 1,
            Err(e) => errors.push(format!("{}: {}", input.id, e)),
        }
    }

    Ok(BulkPathResult {
        inserted,
        skipped,
        errors,
    })
}

/// Delete path and all related data - scoped by app
pub fn delete_path(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    path_id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        paths::table
            .filter(paths::app_id.eq(&ctx.app_id))
            .filter(paths::id.eq(path_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get path count for an app
pub fn path_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get step count for an app (all paths)
pub fn total_step_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    steps::table
        .filter(steps::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}
