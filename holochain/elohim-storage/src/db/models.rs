//! Diesel model definitions for database tables
//!
//! All models include `app_id` for multi-tenant app scoping.
//! - Queryable structs: for SELECT queries (reading data)
//! - Insertable structs: for INSERT queries (writing data)

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::diesel_schema::*;

// ============================================================================
// App Registry Models
// ============================================================================

/// Registered app from the apps table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = apps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct App {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub enabled: i32,
}

/// New app for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = apps)]
pub struct NewApp<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

// ============================================================================
// Content Models
// ============================================================================

/// Content row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = content)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Content {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i64>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub validation_status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Content with tags attached (API response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentWithTags {
    #[serde(flatten)]
    pub content: Content,
    pub tags: Vec<String>,
}

/// New content for INSERT
#[derive(Debug, Clone, Insertable, Deserialize)]
#[diesel(table_name = content)]
pub struct NewContent<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub content_type: &'a str,
    pub content_format: &'a str,
    pub blob_hash: Option<&'a str>,
    pub blob_cid: Option<&'a str>,
    pub content_size_bytes: Option<i64>,
    pub metadata_json: Option<&'a str>,
    pub reach: &'a str,
    pub created_by: Option<&'a str>,
}

/// Content tag row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = content_tags)]
pub struct ContentTag {
    pub app_id: String,
    pub content_id: String,
    pub tag: String,
}

/// New content tag for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_tags)]
pub struct NewContentTag<'a> {
    pub app_id: &'a str,
    pub content_id: &'a str,
    pub tag: &'a str,
}

// ============================================================================
// Path Models
// ============================================================================

/// Path row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = paths)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Path {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub path_type: String,
    pub difficulty: Option<String>,
    pub estimated_duration: Option<String>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_alt: Option<String>,
    pub metadata_json: Option<String>,
    pub visibility: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New path for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = paths)]
pub struct NewPath<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub path_type: &'a str,
    pub difficulty: Option<&'a str>,
    pub estimated_duration: Option<&'a str>,
    pub thumbnail_url: Option<&'a str>,
    pub thumbnail_alt: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
    pub visibility: &'a str,
    pub created_by: Option<&'a str>,
}

/// Path tag row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = path_tags)]
pub struct PathTag {
    pub app_id: String,
    pub path_id: String,
    pub tag: String,
}

/// New path tag for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = path_tags)]
pub struct NewPathTag<'a> {
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub tag: &'a str,
}

// ============================================================================
// Chapter Models
// ============================================================================

/// Chapter row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = chapters)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Chapter {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
}

/// New chapter for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = chapters)]
pub struct NewChapter<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub order_index: i32,
    pub estimated_duration: Option<&'a str>,
}

// ============================================================================
// Step Models
// ============================================================================

/// Step row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = steps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Step {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub step_type: String,
    pub resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
    pub metadata_json: Option<String>,
}

/// New step for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = steps)]
pub struct NewStep<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub chapter_id: Option<&'a str>,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub step_type: &'a str,
    pub resource_id: Option<&'a str>,
    pub resource_type: Option<&'a str>,
    pub order_index: i32,
    pub estimated_duration: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Path Attestation Models
// ============================================================================

/// Path attestation row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = path_attestations)]
pub struct PathAttestation {
    pub app_id: String,
    pub path_id: String,
    pub attestation_type: String,
    pub attestation_name: String,
}

/// New path attestation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = path_attestations)]
pub struct NewPathAttestation<'a> {
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub attestation_type: &'a str,
    pub attestation_name: &'a str,
}

// ============================================================================
// Composite Types (API responses)
// ============================================================================

/// Chapter with its steps
#[derive(Debug, Clone, Serialize)]
pub struct ChapterWithSteps {
    #[serde(flatten)]
    pub chapter: Chapter,
    pub steps: Vec<Step>,
}

/// Path with all nested data (chapters, steps, tags, attestations)
#[derive(Debug, Clone, Serialize)]
pub struct PathWithDetails {
    #[serde(flatten)]
    pub path: Path,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterWithSteps>,
    pub ungrouped_steps: Vec<Step>,
    pub attestations: Vec<PathAttestation>,
}

/// Path with just steps (no chapter grouping)
#[derive(Debug, Clone, Serialize)]
pub struct PathWithSteps {
    #[serde(flatten)]
    pub path: Path,
    pub steps: Vec<Step>,
}
