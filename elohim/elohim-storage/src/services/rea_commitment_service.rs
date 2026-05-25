//! REA Commitment service — business logic

use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::rea_commitments::{
    self, CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState,
    PROJECT_EPR_ACTION,
};
use crate::error::StorageError;
use crate::services::events::{EventBus, StorageEvent};
use crate::views::ReaCommitmentView;

pub struct ReaCommitmentService;

impl ReaCommitmentService {
    pub fn create(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
        events: Option<&EventBus>,
    ) -> Result<ReaCommitmentView, StorageError> {
        let commitment = rea_commitments::create_commitment(conn, ctx, input)?;
        if let Some(bus) = events {
            if commitment.action == PROJECT_EPR_ACTION {
                bus.emit(StorageEvent::ProjectionRegistered {
                    commitment_id: commitment.id.clone(),
                });
            }
        }
        Ok(ReaCommitmentView::from(commitment))
    }

    pub fn get_by_id(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ReaCommitmentView>, StorageError> {
        rea_commitments::get_commitment(conn, ctx, id).map(|opt| opt.map(ReaCommitmentView::from))
    }

    pub fn list(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        query: &ReaCommitmentQuery,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::list_commitments(conn, ctx, query)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    pub fn get_by_agent(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::get_commitments_for_agent(conn, ctx, agent_id, limit)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    pub fn update_state(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        update: &UpdateReaCommitmentState,
        events: Option<&EventBus>,
    ) -> Result<ReaCommitmentView, StorageError> {
        let commitment = rea_commitments::update_commitment_state(conn, ctx, id, update)?;
        if let Some(bus) = events {
            if commitment.action == PROJECT_EPR_ACTION && update.state == "cancelled" {
                bus.emit(StorageEvent::ProjectionRevoked {
                    commitment_id: commitment.id.clone(),
                });
            }
        }
        Ok(ReaCommitmentView::from(commitment))
    }
}
