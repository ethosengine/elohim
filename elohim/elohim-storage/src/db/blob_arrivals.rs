//! Blob arrivals (Category C, local): how each blob in this peer's blob store
//! got here, and the watch the retention pass keeps on blobs nothing names.
//!
//! A blob file says nothing about why it is on disk. A bundle this peer put, a
//! shard another peer pushed and a file left by an interrupted transfer look
//! the same. The path that stores the bytes writes one row here saying which it
//! was, and the retention pass (`services::holds`) reads it:
//!
//! - bytes this peer brought here by its own act ([`ArrivalVia::is_own_act`])
//!   may be let go once nothing names them;
//! - bytes another peer placed are held until that peer withdraws them;
//! - bytes with no row are never let go by the pass.
//!
//! A lost row is therefore fail-safe: it keeps bytes, it cannot release them.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::{blob_arrivals, blob_unnamed_watch};
use super::models::current_timestamp;
use crate::error::StorageError;

/// How a blob got into the local blob store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArrivalVia {
    /// This peer was handed the bytes over its own put path.
    SelfPut,
    /// This peer fetched the bytes from another peer.
    SelfFetch,
    /// The release adoption controller pulled them for a release.
    Adoption,
    /// Another peer pushed them here.
    Placed,
}

impl ArrivalVia {
    pub fn label(self) -> &'static str {
        match self {
            ArrivalVia::SelfPut => "self-put",
            ArrivalVia::SelfFetch => "self-fetch",
            ArrivalVia::Adoption => "adoption",
            ArrivalVia::Placed => "placed",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "self-put" => Some(ArrivalVia::SelfPut),
            "self-fetch" => Some(ArrivalVia::SelfFetch),
            "adoption" => Some(ArrivalVia::Adoption),
            "placed" => Some(ArrivalVia::Placed),
            _ => None,
        }
    }

    /// Whether this peer brought the bytes here itself. It can fetch them
    /// again, so once nothing names them they are its own to let go.
    pub fn is_own_act(self) -> bool {
        !matches!(self, ArrivalVia::Placed)
    }
}

/// One recorded arrival of a blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub via: ArrivalVia,
    /// For [`ArrivalVia::Placed`]: who placed it. Empty otherwise.
    pub placed_by: String,
    /// What it arrived for, when the path knew (a release CID for adoption).
    pub named_for: Option<String>,
    pub arrived_at: String,
    /// Set once the placing peer has withdrawn the placement.
    pub withdrawn_at: Option<String>,
}

impl Arrival {
    /// A placement the placing peer has not withdrawn.
    pub fn is_live_placement(&self) -> bool {
        self.via == ArrivalVia::Placed && self.withdrawn_at.is_none()
    }
}

/// The gate between bytes arriving and bytes being let go. Every path that
/// stores a blob holds it shared, across recording the arrival and storing the
/// bytes; the retention pass holds it alone, across its last look at a blob
/// and the delete. So a put that answers "stored" is never undone by a pass
/// that had already decided: either the pass sees the fresh arrival, or the
/// put lands after the delete and stores the bytes again.
pub fn arrival_gate() -> &'static tokio::sync::RwLock<()> {
    static GATE: tokio::sync::RwLock<()> = tokio::sync::RwLock::const_new(());
    &GATE
}

/// Record that `blob_hash` arrived. Recording the same arrival again is a new
/// arrival: someone brought the bytes here again, so its time is now, the
/// watch on the blob starts over, and a withdrawal is cleared.
pub fn record_arrival(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    via: ArrivalVia,
    placed_by: Option<&str>,
    named_for: Option<&str>,
) -> Result<(), StorageError> {
    let placed_by = placed_by.unwrap_or("");
    conn.transaction(|conn| {
        let written = diesel::insert_or_ignore_into(blob_arrivals::table)
            .values((
                blob_arrivals::blob_hash.eq(blob_hash),
                blob_arrivals::arrived_via.eq(via.label()),
                blob_arrivals::placed_by.eq(placed_by),
                blob_arrivals::named_for.eq(named_for),
                blob_arrivals::arrived_at.eq(current_timestamp()),
            ))
            .execute(conn)?;
        if written == 0 {
            let row = blob_arrivals::table
                .filter(blob_arrivals::blob_hash.eq(blob_hash))
                .filter(blob_arrivals::arrived_via.eq(via.label()))
                .filter(blob_arrivals::placed_by.eq(placed_by));
            diesel::update(row)
                .set((
                    blob_arrivals::withdrawn_at.eq(None::<String>),
                    blob_arrivals::arrived_at.eq(current_timestamp()),
                ))
                .execute(conn)?;
            if named_for.is_some() {
                diesel::update(row)
                    .set(blob_arrivals::named_for.eq(named_for))
                    .execute(conn)?;
            }
        }
        // Whatever the pass had counted against this blob no longer stands.
        diesel::delete(
            blob_unnamed_watch::table.filter(blob_unnamed_watch::blob_hash.eq(blob_hash)),
        )
        .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_arrival: {e}")))
}

/// Every recorded arrival, by blob hash. Rows whose `arrived_via` this build
/// does not know are skipped: an arrival it cannot read is not a reason to let
/// a blob go, and a blob left with no readable row reads as unrecorded.
pub fn arrivals_by_hash(
    conn: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<Arrival>>, StorageError> {
    type Row = (
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    );
    let rows: Vec<Row> = blob_arrivals::table
        .select((
            blob_arrivals::blob_hash,
            blob_arrivals::arrived_via,
            blob_arrivals::placed_by,
            blob_arrivals::named_for,
            blob_arrivals::arrived_at,
            blob_arrivals::withdrawn_at,
        ))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("arrivals_by_hash: {e}")))?;
    let mut out: HashMap<String, Vec<Arrival>> = HashMap::new();
    for (blob_hash, via, placed_by, named_for, arrived_at, withdrawn_at) in rows {
        let Some(via) = ArrivalVia::from_label(&via) else {
            continue;
        };
        out.entry(blob_hash).or_default().push(Arrival {
            via,
            placed_by,
            named_for,
            arrived_at,
            withdrawn_at,
        });
    }
    Ok(out)
}

/// Mark the placement of `blob_hash` by `placed_by` withdrawn. Returns whether
/// a live placement by that peer existed.
pub fn withdraw_placement(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    placed_by: &str,
) -> Result<bool, StorageError> {
    diesel::update(
        blob_arrivals::table
            .filter(blob_arrivals::blob_hash.eq(blob_hash))
            .filter(blob_arrivals::arrived_via.eq(ArrivalVia::Placed.label()))
            .filter(blob_arrivals::placed_by.eq(placed_by))
            .filter(blob_arrivals::withdrawn_at.is_null()),
    )
    .set(blob_arrivals::withdrawn_at.eq(current_timestamp()))
    .execute(conn)
    .map(|n| n > 0)
    .map_err(|e| StorageError::Database(format!("withdraw_placement: {e}")))
}

/// Drop every arrival and watch row of blobs whose files are gone.
pub fn forget(conn: &mut SqliteConnection, blob_hashes: &[String]) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        diesel::delete(blob_arrivals::table.filter(blob_arrivals::blob_hash.eq_any(blob_hashes)))
            .execute(conn)?;
        diesel::delete(
            blob_unnamed_watch::table.filter(blob_unnamed_watch::blob_hash.eq_any(blob_hashes)),
        )
        .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("forget blob arrivals: {e}")))
}

/// The watch on one blob that reads as this peer's own and named by nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnnamedWatch {
    pub first_unnamed_at: String,
    /// Consecutive passes it has read so, this one included.
    pub passes: i32,
}

/// Every watched blob.
pub fn watched(conn: &mut SqliteConnection) -> Result<HashMap<String, UnnamedWatch>, StorageError> {
    let rows: Vec<(String, String, i32)> = blob_unnamed_watch::table
        .select((
            blob_unnamed_watch::blob_hash,
            blob_unnamed_watch::first_unnamed_at,
            blob_unnamed_watch::passes,
        ))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("watched: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(hash, first_unnamed_at, passes)| {
            (
                hash,
                UnnamedWatch {
                    first_unnamed_at,
                    passes,
                },
            )
        })
        .collect())
}

/// Count one more pass on which `blob_hash` read as unnamed, starting its
/// watch if this is the first. Returns the watch after counting.
pub fn note_unnamed(
    conn: &mut SqliteConnection,
    blob_hash: &str,
) -> Result<UnnamedWatch, StorageError> {
    conn.transaction(|conn| {
        diesel::insert_or_ignore_into(blob_unnamed_watch::table)
            .values((
                blob_unnamed_watch::blob_hash.eq(blob_hash),
                blob_unnamed_watch::first_unnamed_at.eq(current_timestamp()),
                blob_unnamed_watch::passes.eq(0),
            ))
            .execute(conn)?;
        diesel::update(
            blob_unnamed_watch::table.filter(blob_unnamed_watch::blob_hash.eq(blob_hash)),
        )
        .set(blob_unnamed_watch::passes.eq(blob_unnamed_watch::passes + 1))
        .execute(conn)?;
        let (first_unnamed_at, passes): (String, i32) = blob_unnamed_watch::table
            .filter(blob_unnamed_watch::blob_hash.eq(blob_hash))
            .select((
                blob_unnamed_watch::first_unnamed_at,
                blob_unnamed_watch::passes,
            ))
            .first(conn)?;
        Ok::<UnnamedWatch, diesel::result::Error>(UnnamedWatch {
            first_unnamed_at,
            passes,
        })
    })
    .map_err(|e| StorageError::Database(format!("note_unnamed: {e}")))
}

/// Whether the watch on `blob_hash` still stands at `min_passes` or more, and
/// no other peer's placement of it stands. The last look before a blob is let
/// go: an arrival since the count clears the watch, and a placement makes the
/// bytes someone else's.
pub fn watch_stands(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    min_passes: u64,
) -> Result<bool, StorageError> {
    let passes: Option<i32> = blob_unnamed_watch::table
        .filter(blob_unnamed_watch::blob_hash.eq(blob_hash))
        .select(blob_unnamed_watch::passes)
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("watch_stands: {e}")))?;
    let counted = passes.is_some_and(|p| u64::try_from(p).unwrap_or(0) >= min_passes);
    if !counted {
        return Ok(false);
    }
    let placed: i64 = blob_arrivals::table
        .filter(blob_arrivals::blob_hash.eq(blob_hash))
        .filter(blob_arrivals::arrived_via.eq(ArrivalVia::Placed.label()))
        .filter(blob_arrivals::withdrawn_at.is_null())
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Database(format!("watch_stands: {e}")))?;
    Ok(placed == 0)
}

/// Count one more pass for every blob in `blob_hashes`, in one transaction.
/// Returns each blob's count after counting.
pub fn note_unnamed_many(
    conn: &mut SqliteConnection,
    blob_hashes: &[String],
) -> Result<HashMap<String, i32>, StorageError> {
    conn.transaction(|conn| {
        let now = current_timestamp();
        for blob_hash in blob_hashes {
            diesel::insert_or_ignore_into(blob_unnamed_watch::table)
                .values((
                    blob_unnamed_watch::blob_hash.eq(blob_hash),
                    blob_unnamed_watch::first_unnamed_at.eq(&now),
                    blob_unnamed_watch::passes.eq(0),
                ))
                .execute(conn)?;
        }
        diesel::update(
            blob_unnamed_watch::table.filter(blob_unnamed_watch::blob_hash.eq_any(blob_hashes)),
        )
        .set(blob_unnamed_watch::passes.eq(blob_unnamed_watch::passes + 1))
        .execute(conn)?;
        let rows: Vec<(String, i32)> = blob_unnamed_watch::table
            .filter(blob_unnamed_watch::blob_hash.eq_any(blob_hashes))
            .select((blob_unnamed_watch::blob_hash, blob_unnamed_watch::passes))
            .load(conn)?;
        Ok::<HashMap<String, i32>, diesel::result::Error>(rows.into_iter().collect())
    })
    .map_err(|e| StorageError::Database(format!("note_unnamed_many: {e}")))
}

/// Stop watching blobs that are named again: their count starts over.
pub fn clear_watch(
    conn: &mut SqliteConnection,
    blob_hashes: &[String],
) -> Result<(), StorageError> {
    diesel::delete(
        blob_unnamed_watch::table.filter(blob_unnamed_watch::blob_hash.eq_any(blob_hashes)),
    )
    .execute(conn)
    .map(|_| ())
    .map_err(|e| StorageError::Database(format!("clear_watch: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<SqliteConnection>> {
        crate::test_util::test_pool().get().unwrap()
    }

    #[test]
    fn an_arrival_recorded_again_is_a_new_arrival_and_starts_the_watch_over() {
        let mut conn = conn();
        record_arrival(&mut conn, "sha256-aa", ArrivalVia::SelfPut, None, None).unwrap();
        diesel::sql_query("UPDATE blob_arrivals SET arrived_at = '2026-01-01T00:00:00Z'")
            .execute(&mut conn)
            .unwrap();
        note_unnamed(&mut conn, "sha256-aa").unwrap();
        note_unnamed(&mut conn, "sha256-aa").unwrap();

        record_arrival(&mut conn, "sha256-aa", ArrivalVia::SelfPut, None, None).unwrap();
        let again = arrivals_by_hash(&mut conn).unwrap()["sha256-aa"][0].clone();
        assert!(again.arrived_at.as_str() > "2026-01-01T00:00:00Z");
        assert!(watched(&mut conn).unwrap().is_empty());
    }

    #[test]
    fn a_placement_made_again_after_a_withdrawal_stands_again() {
        let mut conn = conn();
        record_arrival(
            &mut conn,
            "sha256-ab",
            ArrivalVia::Placed,
            Some("agent-x"),
            None,
        )
        .unwrap();
        assert!(withdraw_placement(&mut conn, "sha256-ab", "agent-x").unwrap());
        assert!(!arrivals_by_hash(&mut conn).unwrap()["sha256-ab"][0].is_live_placement());
        record_arrival(
            &mut conn,
            "sha256-ab",
            ArrivalVia::Placed,
            Some("agent-x"),
            None,
        )
        .unwrap();
        assert!(arrivals_by_hash(&mut conn).unwrap()["sha256-ab"][0].is_live_placement());
    }

    #[test]
    fn a_placement_is_withdrawn_only_by_the_peer_that_placed_it() {
        let mut conn = conn();
        record_arrival(
            &mut conn,
            "sha256-bb",
            ArrivalVia::Placed,
            Some("agent-x"),
            None,
        )
        .unwrap();
        assert!(!withdraw_placement(&mut conn, "sha256-bb", "agent-y").unwrap());
        assert!(arrivals_by_hash(&mut conn).unwrap()["sha256-bb"][0].is_live_placement());
    }

    #[test]
    fn a_blob_can_arrive_more_than_one_way_and_each_is_its_own_record() {
        let mut conn = conn();
        record_arrival(&mut conn, "sha256-cc", ArrivalVia::SelfPut, None, None).unwrap();
        record_arrival(
            &mut conn,
            "sha256-cc",
            ArrivalVia::Adoption,
            None,
            Some("release-1"),
        )
        .unwrap();
        let arrivals = &arrivals_by_hash(&mut conn).unwrap()["sha256-cc"];
        assert_eq!(arrivals.len(), 2);
        assert!(arrivals.iter().all(|a| a.via.is_own_act()));
        assert!(arrivals
            .iter()
            .any(|a| a.named_for.as_deref() == Some("release-1")));
    }

    #[test]
    fn the_watch_counts_consecutive_passes_and_starts_over_once_cleared() {
        let mut conn = conn();
        assert_eq!(note_unnamed(&mut conn, "sha256-dd").unwrap().passes, 1);
        let second = note_unnamed(&mut conn, "sha256-dd").unwrap();
        assert_eq!(second.passes, 2);
        clear_watch(&mut conn, &["sha256-dd".to_string()]).unwrap();
        assert!(watched(&mut conn).unwrap().is_empty());
        assert_eq!(note_unnamed(&mut conn, "sha256-dd").unwrap().passes, 1);
    }

    #[test]
    fn forgetting_a_blob_drops_its_arrivals_and_its_watch() {
        let mut conn = conn();
        record_arrival(&mut conn, "sha256-ee", ArrivalVia::SelfFetch, None, None).unwrap();
        note_unnamed(&mut conn, "sha256-ee").unwrap();
        forget(&mut conn, &["sha256-ee".to_string()]).unwrap();
        assert!(arrivals_by_hash(&mut conn).unwrap().is_empty());
        assert!(watched(&mut conn).unwrap().is_empty());
    }
}
