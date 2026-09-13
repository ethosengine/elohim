//! Shared prepare-then-CAS application for notarized REA lifecycle observations.
use super::{
    conductor_writes,
    rea_commitment_record::{self, Observation},
};
use crate::{
    db::{
        rea_commitment_lifecycle::{self, ApplyOutcome, Snapshot},
        AppContext, DbPool,
    },
    error::StorageError,
    hc_client::HcClient,
};
use diesel::SqliteConnection;
use std::sync::Arc;

pub(crate) struct Prepared {
    expected: Option<Snapshot>,
    observation: Observation,
}

/// Exact local conductor reads happen before the database write transaction.
/// A preexisting row without an authority anchor cannot establish precedence.
pub(crate) async fn prepare(
    hc: &Arc<HcClient>,
    id: &str,
    action_hash: &str,
    expected: Option<Snapshot>,
) -> Result<Option<Prepared>, StorageError> {
    prepare_with_reader(id, action_hash, expected, |hash| {
        let hc = Arc::clone(hc);
        async move { conductor_writes::call_get_record_for_action(&hc, &hash).await }
    })
    .await
}

pub(crate) async fn prepare_with_reader<F, Fut>(
    id: &str,
    action_hash: &str,
    expected: Option<Snapshot>,
    read: F,
) -> Result<Option<Prepared>, StorageError>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<
        Output = Result<Option<conductor_writes::CarriedRecordWire>, StorageError>,
    >,
{
    if expected
        .as_ref()
        .is_some_and(|s| s.anchor.as_deref().is_none_or(str::is_empty))
    {
        return Err(StorageError::InvalidInput(
            "REA lifecycle stored authority missing".into(),
        ));
    }
    let observation = rea_commitment_record::observe(
        id,
        action_hash,
        expected.as_ref().and_then(|s| s.anchor.as_deref()),
        read,
    )
    .await?;
    Ok(observation.map(|observation| Prepared {
        expected,
        observation,
    }))
}

pub(crate) fn apply_prepared(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    prepared: Prepared,
) -> Result<ApplyOutcome, StorageError> {
    let c = &prepared.observation.commitment;
    let input = crate::rea_projection::project_typed_commitment(c);
    rea_commitment_lifecycle::apply(
        conn,
        ctx,
        prepared.expected.as_ref(),
        input,
        &prepared.observation.action_hash,
        &c.state,
        c.finished,
    )
}

pub(crate) async fn project(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    action_hash: &str,
) -> Result<ApplyOutcome, StorageError> {
    let expected = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(format!("lifecycle pool: {e}")))?;
        rea_commitment_lifecycle::snapshot(&mut conn, ctx, id)?
    };
    let Some(prepared) = prepare(hc, id, action_hash, expected).await? else {
        return Ok(ApplyOutcome::Unchanged);
    };
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(format!("lifecycle pool: {e}")))?;
    apply_prepared(&mut conn, ctx, prepared)
}

/// Explicit reconstruction of an existing public Commitment, never authoring.
/// One bounded ID observation plus the shared 64 exact-record budget. A local
/// lookup miss is not evidence of global absence; ordinary SQL reads stay cheap.
pub(crate) async fn refresh_by_id(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
) -> Result<(), StorageError> {
    let expected = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        rea_commitment_lifecycle::snapshot(&mut conn, ctx, id)?
    };
    let prepared = prepare_refresh(
        id,
        expected,
        conductor_writes::get_rea_commitment(hc, id),
        |hash| {
            let hc = Arc::clone(hc);
            async move { conductor_writes::call_get_record_for_action(&hc, &hash).await }
        },
    )
    .await?;
    if let Some(prepared) = prepared {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        if apply_prepared(&mut conn, ctx, prepared)? == ApplyOutcome::Deferred {
            return Err(StorageError::Internal(
                "REA refresh projection changed concurrently; retry".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) async fn prepare_refresh<F, Fut, Lookup>(
    id: &str,
    expected: Option<Snapshot>,
    lookup: Lookup,
    read: F,
) -> Result<Option<Prepared>, StorageError>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<
        Output = Result<Option<conductor_writes::CarriedRecordWire>, StorageError>,
    >,
    Lookup: std::future::Future<
        Output = Result<Option<shefa_types::ReaCommitmentOutput>, StorageError>,
    >,
{
    let observed = lookup.await?.ok_or_else(|| {
        StorageError::NotFound(format!("Commitment {id} not observed by own conductor"))
    })?;
    if observed.commitment.id != id {
        return Err(StorageError::InvalidInput(
            "REA refresh returned another undertaking".into(),
        ));
    }
    prepare_with_reader(id, &observed.action_hash.to_string(), expected, read).await
}
