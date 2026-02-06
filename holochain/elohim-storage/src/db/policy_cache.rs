//! Policy Cache - SQLite-backed policy cache for offline enforcement
//!
//! This module provides fast, local policy lookups for content filtering,
//! time limits, and feature restrictions. Policies are synced from the
//! ImagoDei zome and cached locally for offline enforcement.
//!
//! ## Tables
//!
//! - `cached_policies` - Computed policies per agent (synced from DHT)
//! - `policy_sessions` - Current session tracking
//! - `policy_daily_usage` - Daily usage tracking
//! - `policy_events` - Policy violation/block logs

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::db::{DbPool, PooledConn};
use crate::error::StorageError;

// =============================================================================
// Diesel Schema (will be added to diesel_schema.rs)
// =============================================================================

diesel::table! {
    cached_policies (agent_id) {
        agent_id -> Text,
        policy_json -> Text,
        computed_at -> Text,
        expires_at -> Text,
        signature -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    policy_sessions (id) {
        id -> Text,
        agent_id -> Text,
        started_at -> Text,
        duration_minutes -> Integer,
        last_heartbeat_at -> Text,
        ended_at -> Nullable<Text>,
    }
}

diesel::table! {
    policy_daily_usage (agent_id, date) {
        agent_id -> Text,
        date -> Text,
        total_minutes -> Integer,
        session_count -> Integer,
        updated_at -> Text,
    }
}

diesel::table! {
    policy_events (id) {
        id -> Text,
        agent_id -> Text,
        session_id -> Nullable<Text>,
        event_type -> Text,
        details -> Text,
        content_hash -> Nullable<Text>,
        feature_name -> Nullable<Text>,
        timestamp -> Text,
        retention_expires_at -> Text,
    }
}

// =============================================================================
// Models
// =============================================================================

/// Cached computed policy (mirrors TypeScript ComputedPolicy)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedPolicy {
    pub subject_id: String,
    pub computed_at: String,

    // Content rules
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<u8>,

    // Time rules
    pub session_max_minutes: Option<u32>,
    pub daily_max_minutes: Option<u32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<u32>,

    // Feature rules
    pub disabled_features: Vec<String>,
    pub disabled_routes: Vec<String>,
    pub require_approval: Vec<String>,

    // Monitoring rules
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: u32,
    pub subject_can_view: bool,
}

impl Default for CachedPolicy {
    fn default() -> Self {
        Self {
            subject_id: String::new(),
            computed_at: Utc::now().to_rfc3339(),
            blocked_categories: Vec::new(),
            blocked_hashes: Vec::new(),
            age_rating_max: None,
            reach_level_max: None,
            session_max_minutes: None,
            daily_max_minutes: None,
            time_windows_json: "[]".to_string(),
            cooldown_minutes: None,
            disabled_features: Vec::new(),
            disabled_routes: Vec::new(),
            require_approval: Vec::new(),
            log_sessions: false,
            log_categories: false,
            log_policy_events: false,
            retention_days: 30,
            subject_can_view: true,
        }
    }
}

/// Time window for allowed access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeWindow {
    pub day_of_week: Vec<u8>,  // 0=Sun, 1=Mon, etc.
    pub start_hour: u8,        // 0-23
    pub start_minute: u8,      // 0-59
    pub end_hour: u8,
    pub end_minute: u8,
}

/// Policy decision result
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PolicyDecision {
    Allow,
    Block { reason: String },
}

/// Time access decision
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TimeAccessDecision {
    Allowed {
        remaining_session: Option<u32>,
        remaining_daily: Option<u32>,
    },
    OutsideWindow,
    SessionLimit,
    DailyLimit,
}

/// Content metadata for policy check
#[derive(Debug, Clone)]
pub struct ContentMetadata {
    pub hash: String,
    pub categories: Vec<String>,
    pub age_rating: Option<String>,
    pub reach_level: Option<u8>,
}

/// Policy event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvent {
    pub event_type: PolicyEventType,
    pub details: String,
    pub content_hash: Option<String>,
    pub feature_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEventType {
    BlockedContent,
    TimeLimit,
    FeatureBlocked,
    ApprovalRequired,
}

impl std::fmt::Display for PolicyEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyEventType::BlockedContent => write!(f, "blocked_content"),
            PolicyEventType::TimeLimit => write!(f, "time_limit"),
            PolicyEventType::FeatureBlocked => write!(f, "feature_blocked"),
            PolicyEventType::ApprovalRequired => write!(f, "approval_required"),
        }
    }
}

// =============================================================================
// Insertable/Queryable Types
// =============================================================================

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = cached_policies)]
pub struct CachedPolicyRow {
    pub agent_id: String,
    pub policy_json: String,
    pub computed_at: String,
    pub expires_at: String,
    pub signature: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = cached_policies)]
pub struct NewCachedPolicy<'a> {
    pub agent_id: &'a str,
    pub policy_json: &'a str,
    pub computed_at: &'a str,
    pub expires_at: &'a str,
    pub signature: Option<&'a str>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = policy_sessions)]
pub struct SessionRow {
    pub id: String,
    pub agent_id: String,
    pub started_at: String,
    pub duration_minutes: i32,
    pub last_heartbeat_at: String,
    pub ended_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = policy_sessions)]
pub struct NewSession<'a> {
    pub id: &'a str,
    pub agent_id: &'a str,
    pub started_at: &'a str,
    pub duration_minutes: i32,
    pub last_heartbeat_at: &'a str,
    pub ended_at: Option<&'a str>,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = policy_daily_usage)]
pub struct DailyUsageRow {
    pub agent_id: String,
    pub date: String,
    pub total_minutes: i32,
    pub session_count: i32,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = policy_daily_usage)]
pub struct NewDailyUsage<'a> {
    pub agent_id: &'a str,
    pub date: &'a str,
    pub total_minutes: i32,
    pub session_count: i32,
    pub updated_at: &'a str,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = policy_events)]
pub struct PolicyEventRow {
    pub id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub event_type: String,
    pub details: String,
    pub content_hash: Option<String>,
    pub feature_name: Option<String>,
    pub timestamp: String,
    pub retention_expires_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = policy_events)]
pub struct NewPolicyEvent<'a> {
    pub id: &'a str,
    pub agent_id: &'a str,
    pub session_id: Option<&'a str>,
    pub event_type: &'a str,
    pub details: &'a str,
    pub content_hash: Option<&'a str>,
    pub feature_name: Option<&'a str>,
    pub timestamp: &'a str,
    pub retention_expires_at: &'a str,
}

// =============================================================================
// Policy Cache Implementation
// =============================================================================

/// SQLite-backed policy cache for offline enforcement
pub struct PolicyCache {
    pool: DbPool,
    /// Hot cache for fast lookups (agent_id -> policy)
    hot_cache: dashmap::DashMap<String, CachedPolicy>,
}

impl PolicyCache {
    /// Create a new policy cache with the given connection pool
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            hot_cache: dashmap::DashMap::new(),
        }
    }

    /// Initialize the policy cache tables
    pub fn init_tables(&self) -> Result<(), StorageError> {
        let mut conn = self.conn()?;

        // Create cached_policies table
        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS cached_policies (
                agent_id TEXT PRIMARY KEY NOT NULL,
                policy_json TEXT NOT NULL,
                computed_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                signature TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create cached_policies: {}", e)))?;

        // Create policy_sessions table
        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS policy_sessions (
                id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                duration_minutes INTEGER NOT NULL DEFAULT 0,
                last_heartbeat_at TEXT NOT NULL,
                ended_at TEXT
            )
            "#,
        )
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create policy_sessions: {}", e)))?;

        // Create policy_daily_usage table
        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS policy_daily_usage (
                agent_id TEXT NOT NULL,
                date TEXT NOT NULL,
                total_minutes INTEGER NOT NULL DEFAULT 0,
                session_count INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (agent_id, date)
            )
            "#,
        )
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create policy_daily_usage: {}", e)))?;

        // Create policy_events table
        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS policy_events (
                id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                session_id TEXT,
                event_type TEXT NOT NULL,
                details TEXT NOT NULL,
                content_hash TEXT,
                feature_name TEXT,
                timestamp TEXT NOT NULL,
                retention_expires_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create policy_events: {}", e)))?;

        // Create indexes
        diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_policy_sessions_agent ON policy_sessions(agent_id)")
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to create index: {}", e)))?;

        diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_policy_events_agent ON policy_events(agent_id)")
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to create index: {}", e)))?;

        diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_policy_events_timestamp ON policy_events(timestamp)")
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to create index: {}", e)))?;

        info!("Policy cache tables initialized");
        Ok(())
    }

    /// Get a connection from the pool
    fn conn(&self) -> Result<PooledConn, StorageError> {
        self.pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    // =========================================================================
    // Policy Operations
    // =========================================================================

    /// Get computed policy for an agent (hot cache -> SQLite -> default)
    pub fn get_policy(&self, agent_id: &str) -> Result<CachedPolicy, StorageError> {
        // 1. Check hot cache
        if let Some(policy) = self.hot_cache.get(agent_id) {
            let expires = DateTime::parse_from_rfc3339(&policy.computed_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            // If not expired, return from cache
            if expires > Utc::now() {
                debug!("Policy cache hit for agent {}", agent_id);
                return Ok(policy.clone());
            }
        }

        // 2. Check SQLite
        let mut conn = self.conn()?;
        let result = cached_policies::table
            .filter(cached_policies::agent_id.eq(agent_id))
            .first::<CachedPolicyRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

        if let Some(row) = result {
            // Check expiry
            let expires = DateTime::parse_from_rfc3339(&row.expires_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            if expires > Utc::now() {
                // Parse and cache
                let policy: CachedPolicy = serde_json::from_str(&row.policy_json)
                    .map_err(|e| StorageError::Internal(format!("Failed to parse policy: {}", e)))?;

                self.hot_cache.insert(agent_id.to_string(), policy.clone());
                debug!("Policy loaded from SQLite for agent {}", agent_id);
                return Ok(policy);
            } else {
                debug!("Cached policy expired for agent {}", agent_id);
            }
        }

        // 3. Return default (no restrictions)
        debug!("No cached policy for agent {}, using default", agent_id);
        Ok(CachedPolicy::default())
    }

    /// Save/update a computed policy
    pub fn save_policy(
        &self,
        agent_id: &str,
        policy: &CachedPolicy,
        expires_at: &str,
        signature: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let policy_json = serde_json::to_string(policy)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize policy: {}", e)))?;

        // Upsert using raw SQL (Diesel's on_conflict requires sqlite feature)
        diesel::sql_query(
            r#"
            INSERT INTO cached_policies (agent_id, policy_json, computed_at, expires_at, signature, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(agent_id) DO UPDATE SET
                policy_json = excluded.policy_json,
                computed_at = excluded.computed_at,
                expires_at = excluded.expires_at,
                signature = excluded.signature,
                updated_at = excluded.updated_at
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(agent_id)
        .bind::<diesel::sql_types::Text, _>(&policy_json)
        .bind::<diesel::sql_types::Text, _>(&policy.computed_at)
        .bind::<diesel::sql_types::Text, _>(expires_at)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(signature)
        .bind::<diesel::sql_types::Text, _>(&now)
        .bind::<diesel::sql_types::Text, _>(&now)
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to save policy: {}", e)))?;

        // Update hot cache
        self.hot_cache.insert(agent_id.to_string(), policy.clone());

        info!("Saved policy for agent {}", agent_id);
        Ok(())
    }

    /// Invalidate cached policy for an agent
    pub fn invalidate(&self, agent_id: &str) -> Result<(), StorageError> {
        self.hot_cache.remove(agent_id);

        let mut conn = self.conn()?;
        diesel::delete(cached_policies::table.filter(cached_policies::agent_id.eq(agent_id)))
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to delete policy: {}", e)))?;

        debug!("Invalidated policy cache for agent {}", agent_id);
        Ok(())
    }

    // =========================================================================
    // Session Operations
    // =========================================================================

    /// Start a new session for an agent
    pub fn start_session(&self, agent_id: &str) -> Result<String, StorageError> {
        let mut conn = self.conn()?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let new_session = NewSession {
            id: &session_id,
            agent_id,
            started_at: &now,
            duration_minutes: 0,
            last_heartbeat_at: &now,
            ended_at: None,
        };

        diesel::insert_into(policy_sessions::table)
            .values(&new_session)
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to create session: {}", e)))?;

        // Update daily usage session count
        self.increment_session_count(agent_id)?;

        info!("Started session {} for agent {}", session_id, agent_id);
        Ok(session_id)
    }

    /// Update session heartbeat and duration
    pub fn heartbeat(&self, session_id: &str, duration_minutes: i32) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        diesel::update(policy_sessions::table.filter(policy_sessions::id.eq(session_id)))
            .set((
                policy_sessions::duration_minutes.eq(duration_minutes),
                policy_sessions::last_heartbeat_at.eq(&now),
            ))
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to update session: {}", e)))?;

        Ok(())
    }

    /// End a session
    pub fn end_session(&self, session_id: &str) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // Get session to update daily usage
        let session = policy_sessions::table
            .filter(policy_sessions::id.eq(session_id))
            .first::<SessionRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

        if let Some(session) = session {
            // Update daily usage with final duration
            self.add_daily_minutes(&session.agent_id, session.duration_minutes)?;

            // Mark session as ended
            diesel::update(policy_sessions::table.filter(policy_sessions::id.eq(session_id)))
                .set(policy_sessions::ended_at.eq(Some(&now)))
                .execute(&mut conn)
                .map_err(|e| StorageError::Internal(format!("Failed to end session: {}", e)))?;

            info!("Ended session {} ({}min)", session_id, session.duration_minutes);
        }

        Ok(())
    }

    /// Get active session for an agent
    pub fn get_active_session(&self, agent_id: &str) -> Result<Option<SessionRow>, StorageError> {
        let mut conn = self.conn()?;

        policy_sessions::table
            .filter(policy_sessions::agent_id.eq(agent_id))
            .filter(policy_sessions::ended_at.is_null())
            .first::<SessionRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
    }

    // =========================================================================
    // Daily Usage Operations
    // =========================================================================

    /// Get daily usage for an agent
    pub fn get_daily_usage(&self, agent_id: &str) -> Result<i32, StorageError> {
        let mut conn = self.conn()?;
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let result = policy_daily_usage::table
            .filter(policy_daily_usage::agent_id.eq(agent_id))
            .filter(policy_daily_usage::date.eq(&today))
            .first::<DailyUsageRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

        Ok(result.map(|r| r.total_minutes).unwrap_or(0))
    }

    /// Add minutes to daily usage
    fn add_daily_minutes(&self, agent_id: &str, minutes: i32) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let now = Utc::now().to_rfc3339();

        diesel::sql_query(
            r#"
            INSERT INTO policy_daily_usage (agent_id, date, total_minutes, session_count, updated_at)
            VALUES (?, ?, ?, 0, ?)
            ON CONFLICT(agent_id, date) DO UPDATE SET
                total_minutes = total_minutes + excluded.total_minutes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(agent_id)
        .bind::<diesel::sql_types::Text, _>(&today)
        .bind::<diesel::sql_types::Integer, _>(minutes)
        .bind::<diesel::sql_types::Text, _>(&now)
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update daily usage: {}", e)))?;

        Ok(())
    }

    /// Increment session count for today
    fn increment_session_count(&self, agent_id: &str) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let now = Utc::now().to_rfc3339();

        diesel::sql_query(
            r#"
            INSERT INTO policy_daily_usage (agent_id, date, total_minutes, session_count, updated_at)
            VALUES (?, ?, 0, 1, ?)
            ON CONFLICT(agent_id, date) DO UPDATE SET
                session_count = session_count + 1,
                updated_at = excluded.updated_at
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(agent_id)
        .bind::<diesel::sql_types::Text, _>(&today)
        .bind::<diesel::sql_types::Text, _>(&now)
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to increment session count: {}", e)))?;

        Ok(())
    }

    // =========================================================================
    // Policy Event Operations
    // =========================================================================

    /// Log a policy event
    pub fn log_event(
        &self,
        agent_id: &str,
        session_id: Option<&str>,
        event: &PolicyEvent,
        retention_days: u32,
    ) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let event_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let timestamp = now.to_rfc3339();
        let retention_expires = (now + chrono::Duration::days(retention_days as i64)).to_rfc3339();

        let new_event = NewPolicyEvent {
            id: &event_id,
            agent_id,
            session_id,
            event_type: &event.event_type.to_string(),
            details: &event.details,
            content_hash: event.content_hash.as_deref(),
            feature_name: event.feature_name.as_deref(),
            timestamp: &timestamp,
            retention_expires_at: &retention_expires,
        };

        diesel::insert_into(policy_events::table)
            .values(&new_event)
            .execute(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Failed to log event: {}", e)))?;

        debug!("Logged policy event: {:?}", event.event_type);
        Ok(())
    }

    /// Get policy events for an agent
    pub fn get_events(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<PolicyEventRow>, StorageError> {
        let mut conn = self.conn()?;

        policy_events::table
            .filter(policy_events::agent_id.eq(agent_id))
            .order(policy_events::timestamp.desc())
            .limit(limit)
            .load::<PolicyEventRow>(&mut conn)
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
    }

    /// Clean up expired events
    pub fn cleanup_expired_events(&self) -> Result<usize, StorageError> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let deleted = diesel::delete(
            policy_events::table.filter(policy_events::retention_expires_at.lt(&now)),
        )
        .execute(&mut conn)
        .map_err(|e| StorageError::Internal(format!("Failed to cleanup events: {}", e)))?;

        if deleted > 0 {
            info!("Cleaned up {} expired policy events", deleted);
        }

        Ok(deleted)
    }
}

// =============================================================================
// Policy Enforcement
// =============================================================================

/// Policy enforcement engine - uses PolicyCache for decisions
pub struct PolicyEnforcement {
    cache: PolicyCache,
}

impl PolicyEnforcement {
    /// Create a new policy enforcement engine
    pub fn new(cache: PolicyCache) -> Self {
        Self { cache }
    }

    /// Check if content can be served to an agent
    pub fn can_serve(&self, agent_id: &str, content: &ContentMetadata) -> Result<PolicyDecision, StorageError> {
        let policy = self.cache.get_policy(agent_id)?;

        // Check blocked hashes
        if policy.blocked_hashes.contains(&content.hash) {
            return Ok(PolicyDecision::Block {
                reason: "Content blocked".to_string(),
            });
        }

        // Check blocked categories
        for category in &content.categories {
            if policy.blocked_categories.contains(category) {
                return Ok(PolicyDecision::Block {
                    reason: format!("Category '{}' blocked", category),
                });
            }
        }

        // Check age rating
        if let (Some(max_rating), Some(content_rating)) = (&policy.age_rating_max, &content.age_rating) {
            if !is_rating_allowed(content_rating, max_rating) {
                return Ok(PolicyDecision::Block {
                    reason: format!("Age rating '{}' exceeds maximum '{}'", content_rating, max_rating),
                });
            }
        }

        // Check reach level
        if let (Some(max_reach), Some(content_reach)) = (policy.reach_level_max, content.reach_level) {
            if content_reach > max_reach {
                return Ok(PolicyDecision::Block {
                    reason: format!("Reach level {} exceeds maximum {}", content_reach, max_reach),
                });
            }
        }

        Ok(PolicyDecision::Allow)
    }

    /// Check if a feature is allowed for an agent
    pub fn can_use_feature(&self, agent_id: &str, feature: &str) -> Result<PolicyDecision, StorageError> {
        let policy = self.cache.get_policy(agent_id)?;

        if policy.disabled_features.contains(&feature.to_string()) {
            return Ok(PolicyDecision::Block {
                reason: format!("Feature '{}' is disabled", feature),
            });
        }

        Ok(PolicyDecision::Allow)
    }

    /// Check if a route is allowed for an agent
    pub fn can_access_route(&self, agent_id: &str, route: &str) -> Result<PolicyDecision, StorageError> {
        let policy = self.cache.get_policy(agent_id)?;

        for disabled_route in &policy.disabled_routes {
            // Support wildcards like /community/*
            if route_matches(route, disabled_route) {
                return Ok(PolicyDecision::Block {
                    reason: format!("Route '{}' is disabled", route),
                });
            }
        }

        Ok(PolicyDecision::Allow)
    }

    /// Check time-based access for an agent
    pub fn check_time_access(&self, agent_id: &str) -> Result<TimeAccessDecision, StorageError> {
        let policy = self.cache.get_policy(agent_id)?;

        // Check time windows
        if !policy.time_windows_json.is_empty() && policy.time_windows_json != "[]" {
            let windows: Vec<TimeWindow> = serde_json::from_str(&policy.time_windows_json)
                .unwrap_or_default();

            if !windows.is_empty() && !is_in_allowed_window(&windows) {
                return Ok(TimeAccessDecision::OutsideWindow);
            }
        }

        // Get current session
        let session = self.cache.get_active_session(agent_id)?;
        let session_minutes = session.map(|s| s.duration_minutes).unwrap_or(0);

        // Check session limit
        if let Some(max_session) = policy.session_max_minutes {
            if session_minutes >= max_session as i32 {
                return Ok(TimeAccessDecision::SessionLimit);
            }
        }

        // Check daily limit
        let daily_minutes = self.cache.get_daily_usage(agent_id)?;
        if let Some(max_daily) = policy.daily_max_minutes {
            if daily_minutes >= max_daily as i32 {
                return Ok(TimeAccessDecision::DailyLimit);
            }
        }

        // Calculate remaining time
        let remaining_session = policy.session_max_minutes.map(|max| {
            (max as i32 - session_minutes).max(0) as u32
        });
        let remaining_daily = policy.daily_max_minutes.map(|max| {
            (max as i32 - daily_minutes).max(0) as u32
        });

        Ok(TimeAccessDecision::Allowed {
            remaining_session,
            remaining_daily,
        })
    }

    /// Get the underlying cache for direct operations
    pub fn cache(&self) -> &PolicyCache {
        &self.cache
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if a content rating is allowed given a maximum rating
fn is_rating_allowed(content_rating: &str, max_rating: &str) -> bool {
    let ratings = ["G", "PG", "PG-13", "R", "NC-17"];

    let content_idx = ratings.iter().position(|r| *r == content_rating).unwrap_or(0);
    let max_idx = ratings.iter().position(|r| *r == max_rating).unwrap_or(ratings.len() - 1);

    content_idx <= max_idx
}

/// Check if a route matches a pattern (supports wildcards)
fn route_matches(route: &str, pattern: &str) -> bool {
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        route.starts_with(prefix)
    } else if pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        route.starts_with(prefix)
    } else {
        route == pattern
    }
}

/// Check if current time is within allowed time windows
fn is_in_allowed_window(windows: &[TimeWindow]) -> bool {
    let now = chrono::Local::now();
    let day_of_week = now.weekday().num_days_from_sunday() as u8;
    let hour = now.hour() as u8;
    let minute = now.minute() as u8;

    for window in windows {
        if !window.day_of_week.contains(&day_of_week) {
            continue;
        }

        let current_minutes = hour as u32 * 60 + minute as u32;
        let start_minutes = window.start_hour as u32 * 60 + window.start_minute as u32;
        let end_minutes = window.end_hour as u32 * 60 + window.end_minute as u32;

        if current_minutes >= start_minutes && current_minutes <= end_minutes {
            return true;
        }
    }

    // No windows defined means always allowed
    windows.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_allowed() {
        assert!(is_rating_allowed("G", "PG-13"));
        assert!(is_rating_allowed("PG", "PG-13"));
        assert!(is_rating_allowed("PG-13", "PG-13"));
        assert!(!is_rating_allowed("R", "PG-13"));
        assert!(!is_rating_allowed("NC-17", "PG-13"));
    }

    #[test]
    fn test_route_matches() {
        assert!(route_matches("/community/forum", "/community/*"));
        assert!(route_matches("/community", "/community/*"));
        assert!(!route_matches("/profile", "/community/*"));
        assert!(route_matches("/api/v1/users", "/api/*"));
        assert!(route_matches("/exact", "/exact"));
        assert!(!route_matches("/exact/sub", "/exact"));
    }

    #[test]
    fn test_default_policy() {
        let policy = CachedPolicy::default();
        assert!(policy.blocked_categories.is_empty());
        assert!(policy.blocked_hashes.is_empty());
        assert!(policy.disabled_features.is_empty());
        assert_eq!(policy.retention_days, 30);
        assert!(policy.subject_can_view);
    }
}
