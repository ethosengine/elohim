//! Comments CRUD operations
//!
//! Gate-checked community comments on content, scoped by reach level.

use diesel::prelude::*;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::comments;
use super::models::{current_timestamp, Comment, NewComment};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a comment (from controller after gate check)
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentInput {
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
}

/// Query parameters for listing comments - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentQuery {
    pub content_id: Option<String>,
    pub human_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new comment
pub fn create_comment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateCommentInput,
) -> Result<Comment, StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    let new = NewComment {
        id: id.clone(),
        h_app_id: ctx.h_app_id().to_string(),
        content_id: input.content_id.clone(),
        human_id: input.human_id.clone(),
        body: input.body.clone(),
        reach: input.reach.clone(),
        governance_state: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    diesel::insert_into(comments::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create comment: {}", e)))?;

    debug!("Created comment {} on content {}", id, input.content_id);

    get_comment(conn, ctx, &id)
}

/// Get a single comment by ID
pub fn get_comment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Comment, StorageError> {
    comments::table
        .filter(comments::id.eq(id))
        .filter(comments::h_app_id.eq(ctx.h_app_id()))
        .first::<Comment>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(id.to_string()),
            _ => StorageError::Internal(format!("Failed to get comment: {}", e)),
        })
}

/// List comments with optional filters
pub fn list_comments(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &CommentQuery,
) -> Result<Vec<Comment>, StorageError> {
    let mut q = comments::table
        .filter(comments::h_app_id.eq(ctx.h_app_id()))
        .order(comments::created_at.desc())
        .into_boxed();

    if let Some(content_id) = &query.content_id {
        q = q.filter(comments::content_id.eq(content_id));
    }
    if let Some(human_id) = &query.human_id {
        q = q.filter(comments::human_id.eq(human_id));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }
    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    q.load::<Comment>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list comments: {}", e)))
}

/// List comments for a specific content piece
pub fn list_comments_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<Comment>, StorageError> {
    list_comments(
        conn,
        ctx,
        &CommentQuery {
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )
}

/// Delete a comment (hard delete)
pub fn delete_comment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<(), StorageError> {
    let deleted = diesel::delete(comments::table)
        .filter(comments::id.eq(id))
        .filter(comments::h_app_id.eq(ctx.h_app_id()))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to delete comment: {}", e)))?;

    if deleted == 0 {
        return Err(StorageError::NotFound(id.to_string()));
    }

    Ok(())
}
