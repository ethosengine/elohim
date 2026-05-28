//! Per spec §6: walk every active replicates-dwelling steward_mutual commitment;
//! for each, check whether the counter-commitment exists; emit
//! reciprocity-imbalance FeedbackSignal when past grace_period without counter.
//!
//! Idempotent (running twice produces the same log state). The first concrete
//! instance of a per-scale audit aggregator — collective + commons follow.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::db::models::NewMutualityAuditLogRow;
use crate::db::mutuality_audit_log;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::commitment_fetcher::{CommitmentFetcher, CommitmentRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReciprocityStatus {
    Matched,
    Pending,
    Breached,
}

impl ReciprocityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Matched => "Matched",
            Self::Pending => "Pending",
            Self::Breached => "Breached",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SweepReport {
    pub commitments_examined: u32,
    pub matched: u32,
    pub pending: u32,
    pub breached: u32,
    pub signals_emitted: u32,
}

pub struct MutualityAuditService {
    pub pool: DbPool,
    pub hc_client: Option<Arc<HcClient>>,
}

impl MutualityAuditService {
    pub fn new(pool: DbPool, hc_client: Option<Arc<HcClient>>) -> Self {
        Self { pool, hc_client }
    }

    /// Walk every active replicates-dwelling steward_mutual commitment; classify;
    /// emit signal on breach; persist log row.
    pub async fn run_sweep<F: CommitmentFetcher>(
        &self,
        fetcher: &F,
        commitments_authored_locally: &[CommitmentRecord],
        now: DateTime<Utc>,
    ) -> Result<SweepReport, StorageError> {
        let mut report = SweepReport {
            commitments_examined: 0,
            matched: 0,
            pending: 0,
            breached: 0,
            signals_emitted: 0,
        };
        for c in commitments_authored_locally {
            if c.action != "replicates-dwelling" {
                continue;
            }
            // CommitmentRecord stores the replicates-dwelling policy in `bounds`
            // (already a serde_json::Value) plus top-level provider/recipient/cid.
            // There is no payload_json/signed_at; map from existing fields and use
            // valid_from as the authoring timestamp.
            let provider_role = c.bounds["provider_role"].as_str().unwrap_or("");
            if provider_role != "steward_mutual" {
                continue;
            }
            report.commitments_examined += 1;

            let provider = c.provider.clone();
            let recipient = c.recipient.clone();
            let grace_period_days = c.bounds["grace_period_days"].as_u64().unwrap_or(14) as i32;

            let signed_at = DateTime::parse_from_rfc3339(&c.valid_from)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now);
            let days_since = (now - signed_at).num_days() as i32;

            // Look for counter-commitment (recipient → provider, same action).
            let counter = self.find_counter(fetcher, &recipient, &provider).await?;

            let status = match counter {
                Some(_) => ReciprocityStatus::Matched,
                None if days_since <= grace_period_days => ReciprocityStatus::Pending,
                None => ReciprocityStatus::Breached,
            };

            let signaled_at = if matches!(status, ReciprocityStatus::Breached) {
                self.emit_reciprocity_imbalance(&recipient, &c.cid).await?;
                report.signals_emitted += 1;
                Some(now.to_rfc3339())
            } else {
                None
            };

            match status {
                ReciprocityStatus::Matched => report.matched += 1,
                ReciprocityStatus::Pending => report.pending += 1,
                ReciprocityStatus::Breached => report.breached += 1,
            }

            let pool = self.pool.clone();
            let cid = c.cid.clone();
            let prov = provider.clone();
            let recip = recipient.clone();
            let status_str = status.as_str();
            let sig = signaled_at.clone();
            let swept_iso = now.to_rfc3339();
            tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
                let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
                mutuality_audit_log::insert(
                    &mut conn,
                    &NewMutualityAuditLogRow {
                        commitment_cid: &cid,
                        provider_dwelling_hub_id: &prov,
                        recipient_dwelling_hub_id: &recip,
                        reciprocity_status: status_str,
                        days_since_authored: days_since,
                        grace_period_days,
                        signaled_at: sig.as_deref(),
                        swept_at: &swept_iso,
                    },
                )
            })
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))??;
        }
        Ok(report)
    }

    async fn find_counter<F: CommitmentFetcher>(
        &self,
        _fetcher: &F,
        _recipient: &str,
        _provider: &str,
    ) -> Result<Option<CommitmentRecord>, StorageError> {
        // The CommitmentFetcher trait fetches by CID; counter-lookup requires a
        // by-pair query. Sprint 3 follow-up: extend CommitmentFetcher with a
        // find_counter method OR query Mishpat directly via hc_client.
        // For Sprint 3 this returns None always — bilateral counter lookups land
        // as part of the conductor-bridge wiring follow-up. The audit-service
        // shape is testable via mocks; production wiring is the gap.
        Ok(None)
    }

    async fn emit_reciprocity_imbalance(
        &self,
        _target_hub_id: &str,
        _evidence_commitment_cid: &str,
    ) -> Result<(), StorageError> {
        // Sprint 3: FeedbackSignal emission via hc_client when wired; until then
        // this is a no-op stub that simply logs.
        if let Some(_hc) = &self.hc_client {
            tracing::info!(
                target = "elohim_storage::mutuality_audit_service",
                "would emit reciprocity-imbalance FeedbackSignal (conductor bridge wiring pending)"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::MockCommitmentFetcher;
    use crate::test_util::test_pool;

    #[tokio::test]
    async fn empty_commitments_produces_empty_report() {
        let svc = MutualityAuditService::new(test_pool(), None);
        let fetcher = MockCommitmentFetcher::new();
        let report = svc.run_sweep(&fetcher, &[], Utc::now()).await.unwrap();
        assert_eq!(report.commitments_examined, 0);
        assert_eq!(report.signals_emitted, 0);
    }
}
