//! Backfill command: projects existing relational epr_atoms into the CozoDB graph.
//!
//! Designed for one-time migration of atoms ingested before the graph-native
//! projection fan-out was wired. Idempotent: re-running projects the same CID again
//! (CozoDB `:put` is an upsert). Cursor-paginated to avoid unbounded memory usage.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use elohim_storage::graph::backfill::{backfill_graph, BackfillOpts};
//! // engine and conn come from your startup context
//! let report = backfill_graph(&mut conn, &engine, BackfillOpts::default()).unwrap();
//! tracing::info!(projected = report.projected, failed = report.failed, "backfill complete");
//! ```

use crate::db::diesel_schema::epr_atoms;
use crate::epr_codec::decode_epr_head;
use crate::graph::{engine::GraphEngine, projector::GraphProjector};
use diesel::prelude::*;
use tracing::{debug, trace, warn};

/// Atom batch row: (cid, kind, payload_bytes, supersedes)
type AtomBatchRow = (String, String, Vec<u8>, Option<String>);

pub struct BackfillOpts {
    /// Resume from this CID (exclusive). `None` starts from the beginning.
    pub from_cid: Option<String>,
    /// Number of atoms to fetch per batch.
    pub batch_size: usize,
}

impl Default for BackfillOpts {
    fn default() -> Self {
        Self {
            from_cid: None,
            batch_size: 1000,
        }
    }
}

#[derive(Debug, Default)]
pub struct BackfillReport {
    pub projected: usize,
    pub failed: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum BackfillError {
    #[error("diesel: {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("graph: {0}")]
    Graph(#[from] crate::graph::engine::GraphError),
}

/// Project all relational epr_atoms into the CozoDB graph.
///
/// Fetches atoms in CID-ordered batches (cursor pagination). For each Content-kind atom,
/// decodes the EprHead from `payload_bytes` and projects it via `GraphProjector`.
/// Decode or projection failures are counted in `report.failed` and do NOT abort
/// the backfill — remaining atoms continue to be processed.
///
/// Supersedence edges are also projected: if `atom.supersedes` is set, a SUPERSEDES
/// edge is written from the predecessor CID to the atom's CID.
pub fn backfill_graph(
    diesel: &mut SqliteConnection,
    engine: &GraphEngine,
    opts: BackfillOpts,
) -> Result<BackfillReport, BackfillError> {
    let projector = GraphProjector::new(engine);
    let mut report = BackfillReport::default();
    let mut cursor = opts.from_cid;

    loop {
        let batch = fetch_atoms_batch(diesel, cursor.as_deref(), opts.batch_size)?;
        if batch.is_empty() {
            break;
        }

        for (cid, kind, payload_bytes, supersedes) in &batch {
            // Only Content-kind atoms carry an EprHead as payload. Other kinds
            // (EconomicEvent, FeedbackSignal, etc.) have non-EprHead payloads.
            if kind != "Content" {
                trace!(cid = %cid, kind = %kind, "backfill: skipping non-Content atom");
                continue;
            }
            match decode_epr_head(payload_bytes) {
                Ok(head) => {
                    match projector.project_head(cid, &head) {
                        Ok(()) => {
                            report.projected += 1;
                            debug!(cid = %cid, "backfill: projected node");
                        }
                        Err(e) => {
                            report.failed += 1;
                            warn!(cid = %cid, err = %e, "backfill: graph projection failed");
                        }
                    }
                    // Write supersedence edge if present (non-fatal on error).
                    if let Some(predecessor) = supersedes {
                        if let Err(e) = projector.project_supersedence(predecessor, cid) {
                            report.failed += 1;
                            warn!(predecessor = %predecessor, successor = %cid, err = %e, "backfill: supersedence edge failed");
                        }
                    }
                }
                Err(e) => {
                    report.failed += 1;
                    warn!(cid = %cid, err = %e, "backfill: EprHead decode from payload_bytes failed");
                }
            }
        }

        // Advance cursor to the last CID in this batch.
        cursor = batch.last().map(|(c, _, _, _)| c.clone());

        // If the batch was smaller than batch_size, we've reached the end.
        if batch.len() < opts.batch_size {
            break;
        }
    }

    Ok(report)
}

/// Fetch a batch of (cid, kind, payload_bytes, supersedes) from epr_atoms in CID order.
///
/// Returns `payload_bytes` (not `canonical_bytes`) because Content-kind EPRs encode the
/// EprHead as their payload — canonical_bytes is the IPLD envelope serialization which
/// `decode_epr_head` cannot parse.
///
/// Uses `cid > after_cid` cursor pagination (lexicographic, safe for CIDv1 strings).
fn fetch_atoms_batch(
    diesel: &mut SqliteConnection,
    after_cid: Option<&str>,
    batch_size: usize,
) -> Result<Vec<AtomBatchRow>, BackfillError> {
    let limit = batch_size as i64;

    let rows: Vec<AtomBatchRow> = if let Some(cursor) = after_cid {
        epr_atoms::table
            .filter(epr_atoms::cid.gt(cursor))
            .order(epr_atoms::cid.asc())
            .limit(limit)
            .select((
                epr_atoms::cid,
                epr_atoms::kind,
                epr_atoms::payload_bytes,
                epr_atoms::supersedes,
            ))
            .load(diesel)?
    } else {
        epr_atoms::table
            .order(epr_atoms::cid.asc())
            .limit(limit)
            .select((
                epr_atoms::cid,
                epr_atoms::kind,
                epr_atoms::payload_bytes,
                epr_atoms::supersedes,
            ))
            .load::<AtomBatchRow>(diesel)?
    };

    Ok(rows)
}
