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

/// Same native prepare/CAS path as authenticated refresh, with exact hosting
/// attention and Background admission. Candidates and their anchors are hints.
/// bounded-work: four ID observations and 64 shared record reads per worker attempt.
pub(crate) async fn refresh_hosting_by_id(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    epr_id: &str,
    record_reads: &Arc<std::sync::atomic::AtomicUsize>,
    events: Option<&crate::services::events::EventBus>,
) -> Result<bool, StorageError> {
    use crate::conductor_admission::AdmissionClass;
    let expected = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        rea_commitment_lifecycle::snapshot(&mut conn, ctx, id)?
    };
    let prepared = prepare_hosting_refresh_with_reader(
        id,
        epr_id,
        expected,
        conductor_writes::get_rea_commitment_classed(hc, id, AdmissionClass::Background),
        |hash| {
            let hc = hc.clone();
            let reads = record_reads.clone();
            async move {
                if !reserve_hosting_record(&reads) {
                    return Err(StorageError::InvalidInput(
                        "hosting record work budget pending".into(),
                    ));
                }
                conductor_writes::call_get_record_for_action_classed(
                    &hc,
                    &hash,
                    AdmissionClass::Background,
                )
                .await
            }
        },
    )
    .await?;
    let Some(prepared) = prepared else {
        // The observer authenticated a same/older record, not missing evidence.
        // It deliberately returns no prepared scope to inspect here, so remain
        // pending conservatively; the existing finite ladder bounds this cost.
        return Ok(false);
    };
    let withdrawn = prepared.observation.commitment.finished
        || matches!(
            prepared.observation.commitment.state.as_str(),
            "cancelled" | "terminated" | "superseded"
        );
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    let outcome = apply_prepared(&mut conn, ctx, prepared)?;
    if outcome == ApplyOutcome::Advanced {
        if let Some(bus) = events {
            bus.emit(if withdrawn {
                crate::services::events::StorageEvent::ProjectionRevoked {
                    commitment_id: id.into(),
                }
            } else {
                crate::services::events::StorageEvent::ProjectionRegistered {
                    commitment_id: id.into(),
                }
            });
        }
    }
    Ok(outcome != ApplyOutcome::Deferred)
}

/// Validate the authenticated record, not a peer's projected scope or the
/// unsigned fields accompanying a native ID observation. Reuse the same parser
/// and CAS preparation as every other lifecycle observation.
pub(crate) async fn prepare_hosting_refresh_with_reader<F, Fut, Lookup>(
    id: &str,
    epr_id: &str,
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
    let prepared = prepare_refresh(id, expected, lookup, read).await?;
    if let Some(p) = &prepared {
        let c = &p.observation.commitment;
        let input = crate::rea_projection::project_typed_commitment(c);
        let (_, exact_epr) = crate::db::rea_commitments::parse_projection_scope(
            input.in_scope_of.as_deref().unwrap_or(""),
        )?;
        let withdrawn =
            c.finished || matches!(c.state.as_str(), "cancelled" | "terminated" | "superseded");
        let public = serde_json::from_str::<serde_json::Value>(&c.metadata_json)
            .ok()
            .is_some_and(|metadata| {
                matches!(
                    metadata.get("reach").and_then(|v| v.as_str()),
                    Some("commons" | "public")
                )
            });
        if c.action != "project-epr" || exact_epr != epr_id || (!withdrawn && !public) {
            return Err(StorageError::InvalidInput(
                "hosting record does not bind exact public EPR".into(),
            ));
        }
    }
    Ok(prepared)
}

fn reserve_hosting_record(reads: &std::sync::atomic::AtomicUsize) -> bool {
    use std::sync::atomic::Ordering;
    reads
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < 64).then_some(n + 1)
        })
        .is_ok()
}

#[cfg(test)]
mod hosting_record_budget_tests {
    use super::*;
    #[test]
    fn multiple_hosting_ids_share_one_record_budget() {
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let workers = (0..4)
            .map(|_| {
                let reads = reads.clone();
                std::thread::spawn(move || {
                    (0..64).filter(|_| reserve_hosting_record(&reads)).count()
                })
            })
            .collect::<Vec<_>>();
        let admitted: usize = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .sum();
        assert_eq!(admitted, 64);
        assert!(!reserve_hosting_record(&reads));
        assert_eq!(reads.load(std::sync::atomic::Ordering::Relaxed), 64);
    }
}
