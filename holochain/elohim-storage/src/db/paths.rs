//! Path and Step CRUD operations

use rusqlite::{Connection, params, Row};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::StorageError;

/// Path row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRow {
    pub id: String,
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
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub step_count: u32,
}

impl PathRow {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            path_type: row.get("path_type")?,
            difficulty: row.get("difficulty")?,
            estimated_duration: row.get("estimated_duration")?,
            thumbnail_url: row.get("thumbnail_url")?,
            thumbnail_alt: row.get("thumbnail_alt")?,
            metadata_json: row.get("metadata_json")?,
            visibility: row.get("visibility")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            tags: vec![],
            step_count: 0,
        })
    }
}

/// Step row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRow {
    pub id: String,
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

impl StepRow {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            path_id: row.get("path_id")?,
            chapter_id: row.get("chapter_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            step_type: row.get("step_type")?,
            resource_id: row.get("resource_id")?,
            resource_type: row.get("resource_type")?,
            order_index: row.get("order_index")?,
            estimated_duration: row.get("estimated_duration")?,
            metadata_json: row.get("metadata_json")?,
        })
    }
}

/// Chapter row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRow {
    pub id: String,
    pub path_id: String,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub steps: Vec<StepRow>,
}

impl ChapterRow {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            path_id: row.get("path_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            order_index: row.get("order_index")?,
            estimated_duration: row.get("estimated_duration")?,
            steps: vec![],
        })
    }
}

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
}

fn default_path_type() -> String { "guided".to_string() }
fn default_visibility() -> String { "public".to_string() }

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
    pub path_id: String,
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

fn default_step_type() -> String { "learn".to_string() }

/// Get path by ID
pub fn get_path(conn: &Connection, id: &str) -> Result<Option<PathRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM paths WHERE id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let mut rows = stmt
        .query(params![id])
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    if let Some(row) = rows.next().map_err(|e| StorageError::Internal(format!("Row fetch failed: {}", e)))? {
        let mut path = PathRow::from_row(row)
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

        // Load tags
        path.tags = get_path_tags(conn, id)?;

        // Get step count
        path.step_count = get_step_count(conn, id)?;

        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// Get path with all chapters and steps
pub fn get_path_with_steps(conn: &Connection, id: &str) -> Result<Option<PathWithSteps>, StorageError> {
    let path = match get_path(conn, id)? {
        Some(p) => p,
        None => return Ok(None),
    };

    let chapters = get_chapters_for_path(conn, id)?;
    let ungrouped_steps = get_ungrouped_steps(conn, id)?;

    Ok(Some(PathWithSteps {
        path,
        chapters,
        ungrouped_steps,
    }))
}

/// Path with chapters and steps
#[derive(Debug, Clone, Serialize)]
pub struct PathWithSteps {
    pub path: PathRow,
    pub chapters: Vec<ChapterRow>,
    pub ungrouped_steps: Vec<StepRow>,
}

/// Get chapters for a path
fn get_chapters_for_path(conn: &Connection, path_id: &str) -> Result<Vec<ChapterRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM chapters WHERE path_id = ? ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let chapter_rows = stmt
        .query_map(params![path_id], |row| ChapterRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let mut chapters = vec![];
    for row_result in chapter_rows {
        let mut chapter = row_result
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

        // Load steps for this chapter
        chapter.steps = get_steps_for_chapter(conn, &chapter.id)?;
        chapters.push(chapter);
    }

    Ok(chapters)
}

/// Get steps for a chapter
fn get_steps_for_chapter(conn: &Connection, chapter_id: &str) -> Result<Vec<StepRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM steps WHERE chapter_id = ? ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let steps: Vec<StepRow> = stmt
        .query_map(params![chapter_id], |row| StepRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(steps)
}

/// Get steps not in any chapter
fn get_ungrouped_steps(conn: &Connection, path_id: &str) -> Result<Vec<StepRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM steps WHERE path_id = ? AND chapter_id IS NULL ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let steps: Vec<StepRow> = stmt
        .query_map(params![path_id], |row| StepRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(steps)
}

/// Get tags for a path
fn get_path_tags(conn: &Connection, path_id: &str) -> Result<Vec<String>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT tag FROM path_tags WHERE path_id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let tags: Vec<String> = stmt
        .query_map(params![path_id], |row| row.get(0))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(tags)
}

/// Get step count for a path
fn get_step_count(conn: &Connection, path_id: &str) -> Result<u32, StorageError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM steps WHERE path_id = ?", params![path_id], |row| row.get(0))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    Ok(count as u32)
}

/// List all paths
pub fn list_paths(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<PathRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM paths ORDER BY created_at DESC LIMIT ? OFFSET ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let path_rows = stmt
        .query_map(params![limit as i64, offset as i64], |row| PathRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let mut paths = vec![];
    for row_result in path_rows {
        let mut path = row_result
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

        path.tags = get_path_tags(conn, &path.id)?;
        path.step_count = get_step_count(conn, &path.id)?;
        paths.push(path);
    }

    Ok(paths)
}

/// Create a path with chapters and steps
pub fn create_path(conn: &mut Connection, input: CreatePathInput) -> Result<PathRow, StorageError> {
    let tx = conn.transaction()
        .map_err(|e| StorageError::Internal(format!("Transaction failed: {}", e)))?;

    // Insert path
    tx.execute(
        r#"
        INSERT INTO paths (
            id, title, description, path_type, difficulty, estimated_duration,
            thumbnail_url, thumbnail_alt, metadata_json, visibility, created_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            input.id,
            input.title,
            input.description,
            input.path_type,
            input.difficulty,
            input.estimated_duration,
            input.thumbnail_url,
            input.thumbnail_alt,
            input.metadata_json,
            input.visibility,
            input.created_by,
        ],
    ).map_err(|e| StorageError::Internal(format!("Path insert failed: {}", e)))?;

    // Insert tags
    for tag in &input.tags {
        tx.execute(
            "INSERT OR IGNORE INTO path_tags (path_id, tag) VALUES (?, ?)",
            params![input.id, tag],
        ).map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
    }

    // Insert chapters and their steps
    for chapter in &input.chapters {
        tx.execute(
            r#"
            INSERT INTO chapters (id, path_id, title, description, order_index, estimated_duration)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            params![
                chapter.id,
                input.id,
                chapter.title,
                chapter.description,
                chapter.order_index,
                chapter.estimated_duration,
            ],
        ).map_err(|e| StorageError::Internal(format!("Chapter insert failed: {}", e)))?;

        // Insert steps for this chapter
        for step in &chapter.steps {
            tx.execute(
                r#"
                INSERT INTO steps (
                    id, path_id, chapter_id, title, description, step_type,
                    resource_id, resource_type, order_index, estimated_duration, metadata_json
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    step.id,
                    input.id,
                    chapter.id,
                    step.title,
                    step.description,
                    step.step_type,
                    step.resource_id,
                    step.resource_type,
                    step.order_index,
                    step.estimated_duration,
                    step.metadata_json,
                ],
            ).map_err(|e| StorageError::Internal(format!("Step insert failed: {}", e)))?;
        }
    }

    tx.commit()
        .map_err(|e| StorageError::Internal(format!("Commit failed: {}", e)))?;

    get_path(conn, &input.id)?
        .ok_or_else(|| StorageError::Internal("Path not found after insert".to_string()))
}

/// Bulk create paths (for seeding)
pub fn bulk_create_paths(conn: &mut Connection, paths: Vec<CreatePathInput>) -> Result<BulkPathResult, StorageError> {
    let mut inserted = 0u64;
    let mut skipped = 0u64;
    let mut errors = vec![];

    for path_input in paths {
        // Check if already exists
        let exists: bool = conn
            .query_row("SELECT 1 FROM paths WHERE id = ?", params![path_input.id], |_| Ok(true))
            .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        match create_path(conn, path_input.clone()) {
            Ok(_) => inserted += 1,
            Err(e) => errors.push(format!("{}: {}", path_input.id, e)),
        }
    }

    Ok(BulkPathResult {
        inserted,
        skipped,
        errors,
    })
}

/// Result of bulk path operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkPathResult {
    pub inserted: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

/// Delete path and all its chapters/steps
pub fn delete_path(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let changes = conn
        .execute("DELETE FROM paths WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(changes > 0)
}

/// Get all steps for a path (flat list)
pub fn get_steps_for_path(conn: &Connection, path_id: &str) -> Result<Vec<StepRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM steps WHERE path_id = ? ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let steps: Vec<StepRow> = stmt
        .query_map(params![path_id], |row| StepRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(steps)
}

/// Get all chapters for a path
pub fn get_chapters(conn: &Connection, path_id: &str) -> Result<Vec<ChapterRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM chapters WHERE path_id = ? ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let chapter_rows = stmt
        .query_map(params![path_id], |row| ChapterRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let mut chapters = vec![];
    for row_result in chapter_rows {
        let mut chapter = row_result
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

        // Load steps for this chapter
        chapter.steps = get_steps_for_chapter_internal(conn, &chapter.id)?;
        chapters.push(chapter);
    }

    Ok(chapters)
}

/// Get steps for a specific chapter (internal helper)
fn get_steps_for_chapter_internal(conn: &Connection, chapter_id: &str) -> Result<Vec<StepRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM steps WHERE chapter_id = ? ORDER BY order_index")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let steps: Vec<StepRow> = stmt
        .query_map(params![chapter_id], |row| StepRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(steps)
}

/// Create a single step
pub fn create_step(conn: &mut Connection, input: CreateStepInput) -> Result<StepRow, StorageError> {
    conn.execute(
        r#"
        INSERT INTO steps (
            id, path_id, chapter_id, title, description, step_type,
            resource_id, resource_type, order_index, estimated_duration, metadata_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            input.id,
            input.path_id,
            input.chapter_id,
            input.title,
            input.description,
            input.step_type,
            input.resource_id,
            input.resource_type,
            input.order_index,
            input.estimated_duration,
            input.metadata_json,
        ],
    ).map_err(|e| StorageError::Internal(format!("Insert step failed: {}", e)))?;

    // Return the created step
    let mut stmt = conn
        .prepare("SELECT * FROM steps WHERE id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    stmt.query_row(params![input.id], |row| StepRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Step not found after insert: {}", e)))
}

/// Delete a step by ID
pub fn delete_step(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let changes = conn
        .execute("DELETE FROM steps WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Delete step failed: {}", e)))?;

    Ok(changes > 0)
}

/// Create a chapter
pub fn create_chapter(conn: &mut Connection, path_id: &str, input: CreateChapterInput) -> Result<ChapterRow, StorageError> {
    conn.execute(
        r#"
        INSERT INTO chapters (id, path_id, title, description, order_index, estimated_duration)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        params![
            input.id,
            path_id,
            input.title,
            input.description,
            input.order_index,
            input.estimated_duration,
        ],
    ).map_err(|e| StorageError::Internal(format!("Insert chapter failed: {}", e)))?;

    // Return the created chapter
    let mut stmt = conn
        .prepare("SELECT * FROM chapters WHERE id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    stmt.query_row(params![input.id], |row| ChapterRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Chapter not found after insert: {}", e)))
}

/// Delete a chapter by ID (cascades to steps)
pub fn delete_chapter(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    // First delete steps in this chapter
    conn.execute("DELETE FROM steps WHERE chapter_id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Delete steps failed: {}", e)))?;

    // Then delete the chapter
    let changes = conn
        .execute("DELETE FROM chapters WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Delete chapter failed: {}", e)))?;

    Ok(changes > 0)
}
