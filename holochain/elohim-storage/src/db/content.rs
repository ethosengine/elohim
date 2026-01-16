//! Content CRUD operations

use rusqlite::{Connection, params, Row};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::StorageError;

/// Content row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    /// Inline content body (markdown, JSON, etc.)
    pub content_body: Option<String>,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i64>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub validation_status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ContentRow {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            content_type: row.get("content_type")?,
            content_format: row.get("content_format")?,
            content_body: row.get("content_body")?,
            blob_hash: row.get("blob_hash")?,
            blob_cid: row.get("blob_cid")?,
            content_size_bytes: row.get("content_size_bytes")?,
            metadata_json: row.get("metadata_json")?,
            reach: row.get("reach")?,
            validation_status: row.get("validation_status")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            tags: vec![], // Loaded separately
        })
    }
}

/// Input for creating content
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContentInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default = "default_content_format")]
    pub content_format: String,
    /// Inline content body (markdown, JSON quiz data, etc.)
    #[serde(default)]
    pub content_body: Option<String>,
    #[serde(default)]
    pub blob_hash: Option<String>,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<i64>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default = "default_reach")]
    pub reach: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_content_type() -> String { "concept".to_string() }
fn default_content_format() -> String { "markdown".to_string() }
fn default_reach() -> String { "public".to_string() }

/// Query parameters for listing content - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentQuery {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 { 100 }

/// Get content by ID
pub fn get_content(conn: &Connection, id: &str) -> Result<Option<ContentRow>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT * FROM content WHERE id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let mut rows = stmt
        .query(params![id])
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    if let Some(row) = rows.next().map_err(|e| StorageError::Internal(format!("Row fetch failed: {}", e)))? {
        let mut content = ContentRow::from_row(row)
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

        // Load tags
        content.tags = get_content_tags(conn, id)?;

        Ok(Some(content))
    } else {
        Ok(None)
    }
}

/// Get tags for a content item
fn get_content_tags(conn: &Connection, content_id: &str) -> Result<Vec<String>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT tag FROM content_tags WHERE content_id = ?")
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let tags: Vec<String> = stmt
        .query_map(params![content_id], |row| row.get(0))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(tags)
}

/// List content with optional filters
pub fn list_content(conn: &Connection, query: &ContentQuery) -> Result<Vec<ContentRow>, StorageError> {
    let mut sql = String::from("SELECT DISTINCT c.* FROM content c");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    let mut conditions = vec![];

    // Join with tags if filtering by tag
    if !query.tags.is_empty() {
        sql.push_str(" INNER JOIN content_tags ct ON c.id = ct.content_id");
        let placeholders: Vec<_> = query.tags.iter().map(|_| "?").collect();
        conditions.push(format!("ct.tag IN ({})", placeholders.join(", ")));
        for tag in &query.tags {
            params.push(Box::new(tag.clone()));
        }
    }

    if let Some(ref ct) = query.content_type {
        conditions.push("c.content_type = ?".to_string());
        params.push(Box::new(ct.clone()));
    }

    if let Some(ref cf) = query.content_format {
        conditions.push("c.content_format = ?".to_string());
        params.push(Box::new(cf.clone()));
    }

    if let Some(ref search) = query.search {
        conditions.push("(c.title LIKE ? OR c.description LIKE ?)".to_string());
        let pattern = format!("%{}%", search);
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY c.created_at DESC LIMIT ? OFFSET ?");
    params.push(Box::new(query.limit as i64));
    params.push(Box::new(query.offset as i64));

    debug!("Executing query: {}", sql);

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| ContentRow::from_row(row))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let mut results = vec![];
    for row_result in rows {
        let mut content = row_result
            .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;
        content.tags = get_content_tags(conn, &content.id)?;
        results.push(content);
    }

    Ok(results)
}

/// Create a single content item
pub fn create_content(conn: &mut Connection, input: CreateContentInput) -> Result<ContentRow, StorageError> {
    let tx = conn.transaction()
        .map_err(|e| StorageError::Internal(format!("Transaction failed: {}", e)))?;

    // Insert content row
    tx.execute(
        r#"
        INSERT INTO content (
            id, title, description, content_type, content_format,
            content_body, blob_hash, blob_cid, content_size_bytes, metadata_json,
            reach, created_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            input.id,
            input.title,
            input.description,
            input.content_type,
            input.content_format,
            input.content_body,
            input.blob_hash,
            input.blob_cid,
            input.content_size_bytes,
            input.metadata_json,
            input.reach,
            input.created_by,
        ],
    ).map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    // Insert tags
    for tag in &input.tags {
        tx.execute(
            "INSERT OR IGNORE INTO content_tags (content_id, tag) VALUES (?, ?)",
            params![input.id, tag],
        ).map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
    }

    tx.commit()
        .map_err(|e| StorageError::Internal(format!("Commit failed: {}", e)))?;

    // Return the created content
    get_content(conn, &input.id)?
        .ok_or_else(|| StorageError::Internal("Content not found after insert".to_string()))
}

/// Bulk create content items (for seeding)
pub fn bulk_create_content(conn: &mut Connection, items: Vec<CreateContentInput>) -> Result<BulkResult, StorageError> {
    let tx = conn.transaction()
        .map_err(|e| StorageError::Internal(format!("Transaction failed: {}", e)))?;

    let mut inserted = 0u64;
    let mut skipped = 0u64;
    let mut errors = vec![];

    for input in items {
        // Check if already exists
        let exists: bool = tx
            .query_row("SELECT 1 FROM content WHERE id = ?", params![input.id], |_| Ok(true))
            .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        // Insert content
        let result = tx.execute(
            r#"
            INSERT INTO content (
                id, title, description, content_type, content_format,
                content_body, blob_hash, blob_cid, content_size_bytes, metadata_json,
                reach, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                input.id,
                input.title,
                input.description,
                input.content_type,
                input.content_format,
                input.content_body,
                input.blob_hash,
                input.blob_cid,
                input.content_size_bytes,
                input.metadata_json,
                input.reach,
                input.created_by,
            ],
        );

        match result {
            Ok(_) => {
                // Insert tags
                for tag in &input.tags {
                    let _ = tx.execute(
                        "INSERT OR IGNORE INTO content_tags (content_id, tag) VALUES (?, ?)",
                        params![input.id, tag],
                    );
                }
                inserted += 1;
            }
            Err(e) => {
                errors.push(format!("{}: {}", input.id, e));
            }
        }
    }

    tx.commit()
        .map_err(|e| StorageError::Internal(format!("Commit failed: {}", e)))?;

    Ok(BulkResult {
        inserted,
        skipped,
        errors,
    })
}

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkResult {
    pub inserted: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

/// Delete content by ID
pub fn delete_content(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let changes = conn
        .execute("DELETE FROM content WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(changes > 0)
}

/// Get content by tag
pub fn get_content_by_tag(conn: &Connection, tag: &str, limit: u32) -> Result<Vec<ContentRow>, StorageError> {
    list_content(conn, &ContentQuery {
        tags: vec![tag.to_string()],
        limit,
        ..Default::default()
    })
}

/// Check if content IDs exist
pub fn check_content_exists(conn: &Connection, ids: &[String]) -> Result<Vec<String>, StorageError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    let placeholders: Vec<_> = ids.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT id FROM content WHERE id IN ({})",
        placeholders.join(", ")
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Prepare failed: {}", e)))?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

    let existing: Vec<String> = stmt
        .query_map(param_refs.as_slice(), |row| row.get(0))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Internal(format!("Row parse failed: {}", e)))?;

    Ok(existing)
}
