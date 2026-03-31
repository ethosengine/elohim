//! ImagodeiObservation — constitutional memory layer.
//!
//! Stores elohim observations about human behavior. Access-controlled
//! by constitutional layer. Feeds behavioral_trust in TrustContext.

use diesel::prelude::*;

use super::context::AppContext;
use super::diesel_schema::imagodei_observations;
use super::models::{ImagodeiObservation, NewImagodeiObservation};
use crate::error::StorageError;

// ============================================================================
// Helpers
// ============================================================================

/// Returns the set of visibility layers visible at the given access level.
/// Constitutional access is cumulative: higher layers see all lower layers.
fn visibility_layers_up_to(layer: &str) -> Vec<&str> {
    match layer {
        "individual" => vec!["individual"],
        "family" => vec!["individual", "family"],
        "community" => vec!["individual", "family", "community"],
        "constitutional" => vec!["individual", "family", "community", "constitutional"],
        _ => vec!["individual"],
    }
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get a single observation by ID.
pub fn get_observation_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<ImagodeiObservation>, StorageError> {
    imagodei_observations::table
        .filter(imagodei_observations::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List observations for a human, filtered by constitutional visibility layer.
///
/// `max_visibility` determines which layers are visible: "individual" sees only
/// individual observations, "family" sees individual+family, etc.
/// Results ordered by observed_at DESC (most recent first).
pub fn list_observations_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    max_visibility: &str,
) -> Result<Vec<ImagodeiObservation>, StorageError> {
    let layers = visibility_layers_up_to(max_visibility);

    imagodei_observations::table
        .filter(imagodei_observations::h_app_id.eq(&ctx.h_app_id))
        .filter(imagodei_observations::human_id.eq(human_id))
        .filter(imagodei_observations::visibility_layer.eq_any(layers))
        .order(imagodei_observations::observed_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List observations for a human filtered by observation type.
/// Results ordered by observed_at DESC (most recent first).
pub fn list_observations_by_type(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    observation_type: &str,
) -> Result<Vec<ImagodeiObservation>, StorageError> {
    imagodei_observations::table
        .filter(imagodei_observations::h_app_id.eq(&ctx.h_app_id))
        .filter(imagodei_observations::human_id.eq(human_id))
        .filter(imagodei_observations::observation_type.eq(observation_type))
        .order(imagodei_observations::observed_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Count total observations for a human within the app scope.
pub fn count_observations_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<i64, StorageError> {
    imagodei_observations::table
        .filter(imagodei_observations::h_app_id.eq(&ctx.h_app_id))
        .filter(imagodei_observations::human_id.eq(human_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Insert a new observation and return the created row.
pub fn create_observation(
    conn: &mut SqliteConnection,
    _ctx: &AppContext,
    observation: &NewImagodeiObservation,
) -> Result<ImagodeiObservation, StorageError> {
    diesel::insert_into(imagodei_observations::table)
        .values(observation)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_observation_by_id(conn, observation.id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created observation".into()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

        conn.batch_execute(
            r#"
            CREATE TABLE imagodei_observations (
                id TEXT PRIMARY KEY NOT NULL,
                h_app_id TEXT NOT NULL,
                human_id TEXT NOT NULL,
                observed_at TEXT NOT NULL,
                observation_type TEXT NOT NULL,
                content TEXT NOT NULL,
                structured_signals_json TEXT,
                trust_delta REAL NOT NULL DEFAULT 0.0,
                visibility_layer TEXT NOT NULL DEFAULT 'individual',
                originating_elohim TEXT NOT NULL,
                relevance_decay REAL NOT NULL DEFAULT 1.0,
                superseded_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                dht_anchor_hash TEXT
            );
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn test_ctx() -> AppContext {
        AppContext {
            h_app_id: "lamad".to_string(),
        }
    }

    fn make_observation<'a>(
        id: &'a str,
        human_id: &'a str,
        observation_type: &'a str,
        visibility_layer: &'a str,
        observed_at: &'a str,
    ) -> NewImagodeiObservation<'a> {
        NewImagodeiObservation {
            id,
            h_app_id: "lamad",
            human_id,
            observed_at,
            observation_type,
            content: "test observation content",
            structured_signals_json: None,
            trust_delta: 0.1,
            visibility_layer,
            originating_elohim: "elohim-alpha",
            relevance_decay: 1.0,
            superseded_by: None,
            dht_anchor_hash: None,
        }
    }

    #[test]
    fn test_create_and_retrieve_observation() {
        let mut conn = setup_test_db();
        let ctx = test_ctx();
        let obs = make_observation(
            "obs-1",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T10:00:00Z",
        );

        let created = create_observation(&mut conn, &ctx, &obs).expect("Failed to create");
        assert_eq!(created.id, "obs-1");
        assert_eq!(created.human_id, "human-1");
        assert_eq!(created.observation_type, "growth_signal");
        assert_eq!(created.visibility_layer, "individual");
        assert!((created.trust_delta - 0.1).abs() < f32::EPSILON);

        let fetched = get_observation_by_id(&mut conn, "obs-1")
            .expect("Query failed")
            .expect("Not found");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.content, "test observation content");
    }

    #[test]
    fn test_list_by_human_id() {
        let mut conn = setup_test_db();
        let ctx = test_ctx();

        let obs1 = make_observation(
            "obs-1",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T10:00:00Z",
        );
        let obs2 = make_observation(
            "obs-2",
            "human-1",
            "pause_override",
            "individual",
            "2026-03-14T11:00:00Z",
        );
        let obs3 = make_observation(
            "obs-3",
            "human-2",
            "growth_signal",
            "individual",
            "2026-03-14T12:00:00Z",
        );

        create_observation(&mut conn, &ctx, &obs1).unwrap();
        create_observation(&mut conn, &ctx, &obs2).unwrap();
        create_observation(&mut conn, &ctx, &obs3).unwrap();

        let results = list_observations_for_human(&mut conn, &ctx, "human-1", "constitutional")
            .expect("Query failed");
        assert_eq!(results.len(), 2);
        // Should be ordered by observed_at DESC
        assert_eq!(results[0].id, "obs-2");
        assert_eq!(results[1].id, "obs-1");
    }

    #[test]
    fn test_visibility_layer_filtering() {
        let mut conn = setup_test_db();
        let ctx = test_ctx();

        let obs_individual = make_observation(
            "obs-ind",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T10:00:00Z",
        );
        let obs_community = make_observation(
            "obs-com",
            "human-1",
            "growth_signal",
            "community",
            "2026-03-14T11:00:00Z",
        );

        create_observation(&mut conn, &ctx, &obs_individual).unwrap();
        create_observation(&mut conn, &ctx, &obs_community).unwrap();

        // Individual access sees only individual
        let individual_view = list_observations_for_human(&mut conn, &ctx, "human-1", "individual")
            .expect("Query failed");
        assert_eq!(individual_view.len(), 1);
        assert_eq!(individual_view[0].visibility_layer, "individual");

        // Community access sees individual + family + community
        let community_view = list_observations_for_human(&mut conn, &ctx, "human-1", "community")
            .expect("Query failed");
        assert_eq!(community_view.len(), 2);
    }

    #[test]
    fn test_list_by_observation_type() {
        let mut conn = setup_test_db();
        let ctx = test_ctx();

        let obs1 = make_observation(
            "obs-1",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T10:00:00Z",
        );
        let obs2 = make_observation(
            "obs-2",
            "human-1",
            "pause_override",
            "individual",
            "2026-03-14T11:00:00Z",
        );
        let obs3 = make_observation(
            "obs-3",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T12:00:00Z",
        );

        create_observation(&mut conn, &ctx, &obs1).unwrap();
        create_observation(&mut conn, &ctx, &obs2).unwrap();
        create_observation(&mut conn, &ctx, &obs3).unwrap();

        let growth = list_observations_by_type(&mut conn, &ctx, "human-1", "growth_signal")
            .expect("Query failed");
        assert_eq!(growth.len(), 2);

        let pause = list_observations_by_type(&mut conn, &ctx, "human-1", "pause_override")
            .expect("Query failed");
        assert_eq!(pause.len(), 1);
        assert_eq!(pause[0].id, "obs-2");
    }

    #[test]
    fn test_count_observations() {
        let mut conn = setup_test_db();
        let ctx = test_ctx();

        assert_eq!(
            count_observations_for_human(&mut conn, &ctx, "human-1").unwrap(),
            0
        );

        let obs1 = make_observation(
            "obs-1",
            "human-1",
            "growth_signal",
            "individual",
            "2026-03-14T10:00:00Z",
        );
        let obs2 = make_observation(
            "obs-2",
            "human-1",
            "pause_override",
            "individual",
            "2026-03-14T11:00:00Z",
        );

        create_observation(&mut conn, &ctx, &obs1).unwrap();
        create_observation(&mut conn, &ctx, &obs2).unwrap();

        assert_eq!(
            count_observations_for_human(&mut conn, &ctx, "human-1").unwrap(),
            2
        );
    }
}
