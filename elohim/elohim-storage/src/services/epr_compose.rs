//! Author-side EPR compose helper.
//!
//! Wraps a compose attempt with the reach-earning gate. The receive-path
//! (external EPRs arriving at put_epr) does NOT consult this — the gate is
//! for outgoing reach decisions only.
//!
//! See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Flow3

use diesel::SqliteConnection;

use crate::services::epr_kind::Reach;
use crate::services::manifest_registry::ManifestRegistry;
use crate::services::reach_earning::{evaluate, ReachVerdict};

/// Error returned when compose is denied or the author key is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// The reach-earning gate returned Blocked or Pending.
    ReachDenied(ReachVerdict),
    /// The author pubkey bytes could not be interpreted.
    InvalidAuthor(String),
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComposeError::ReachDenied(v) => write!(f, "reach denied: {v:?}"),
            ComposeError::InvalidAuthor(msg) => write!(f, "invalid author key: {msg}"),
        }
    }
}

impl std::error::Error for ComposeError {}

/// Successful compose result — verdict is always `Allowed`.
#[derive(Debug, Clone)]
pub struct ComposedEpr {
    pub verdict: ReachVerdict,
}

/// Author-side compose check.
///
/// Returns `Ok(ComposedEpr)` when the gate returns `Allowed`.
/// Returns `Err(ComposeError::ReachDenied)` on `Blocked` or `Pending`.
pub fn compose_epr(
    local_agent: &[u8],
    author: &[u8],
    requested_reach: Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> Result<ComposedEpr, ComposeError> {
    let verdict = evaluate(local_agent, author, requested_reach, conn, registry);
    match verdict {
        ReachVerdict::Allowed { .. } => Ok(ComposedEpr { verdict }),
        ReachVerdict::Pending { .. } | ReachVerdict::Blocked { .. } => {
            Err(ComposeError::ReachDenied(verdict))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:epr_compose_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    #[test]
    fn allowed_verdict_returns_ok() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = ManifestRegistry::default();
        // Reach::Personal is_floor_allowed → Allowed regardless of standing
        let result = compose_epr(&[0; 32], &[1; 32], Reach::Personal, &mut conn, &r);
        assert!(result.is_ok(), "floor reach should be allowed: {result:?}");
    }

    #[test]
    fn blocked_or_pending_returns_err() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = ManifestRegistry::default();
        // Unknown author at Public → Pending (Conservative default treatment)
        let result = compose_epr(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(result.is_err(), "non-floor unknown author should be denied");
        match result.unwrap_err() {
            ComposeError::ReachDenied(ReachVerdict::Pending { .. }) => {}
            other => panic!("expected ReachDenied(Pending), got {other:?}"),
        }
    }
}
