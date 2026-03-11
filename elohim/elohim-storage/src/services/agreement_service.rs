//! Agreement service — business logic

use diesel::SqliteConnection;

use crate::db::agreements::{self, AgreementQuery, CreateAgreementInput};
use crate::db::context::AppContext;
use crate::error::StorageError;
use crate::views::AgreementView;

pub struct AgreementService;

impl AgreementService {
    pub fn create(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateAgreementInput,
    ) -> Result<AgreementView, StorageError> {
        let agreement = agreements::create_agreement(conn, ctx, input)?;
        Ok(AgreementView::from(agreement))
    }

    pub fn get_by_id(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<AgreementView>, StorageError> {
        agreements::get_agreement(conn, ctx, id).map(|opt| opt.map(AgreementView::from))
    }

    pub fn list(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        query: &AgreementQuery,
    ) -> Result<Vec<AgreementView>, StorageError> {
        agreements::list_agreements(conn, ctx, query)
            .map(|v| v.into_iter().map(AgreementView::from).collect())
    }
}
