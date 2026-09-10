//! Self-uptime heartbeat — the doorway's own 7-day hourly availability strip.
//!
//! ## Classification (p2p-design-gate)
//!
//! Class C, ephemeral operational telemetry ABOUT THIS DOORWAY, at the T4
//! web2-projection layer. It is one process's measure of its own availability —
//! not community-witnessed truth — so it must NEVER become a DHT entry type.
//! It lives in the doorway's own local store (the Mongo it already uses for
//! bootstrap backing) purely so the strip survives a pod restart, and it is
//! reconstructable-by-observation: losing it costs history, never truth.
//!
//! ## Honest degradation
//!
//! - Mongo present → the recorder upserts one `{node_key, bucket_start, status}`
//!   document per hour; the page reads back the last 168 buckets.
//! - Mongo absent → the strip degrades to what the RUNNING PROCESS can honestly
//!   claim: buckets since its own start are `up`, everything older is `nodata`.
//! - A gap inside a period we were demonstrably recording (older than this
//!   process's start, newer than the oldest stored bucket) is `down` — a bucket
//!   with no heartbeat in a window we were heart-beating is a real outage.
//! - A doorway with no stable `DOORWAY_ID`/`NODE_ID` re-keys its history on every
//!   restart; its strip then degrades to the process-derived form above. That is
//!   a deployment fact rendered honestly, not a bug to paper over.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::Args;
use crate::server::AppState;

/// Hourly buckets — the strip's resolution.
pub const UPTIME_BUCKET_SECS: i64 = 3_600;

/// 7 days of hourly buckets (the width the status template renders).
pub const UPTIME_BUCKETS: usize = 168;

/// Doorway-local collection holding the hourly heartbeat.
const UPTIME_COLLECTION: &str = "doorway_uptime";

pub const SEG_UP: &str = "up";
pub const SEG_DEGRADED: &str = "degraded";
pub const SEG_DOWN: &str = "down";
pub const SEG_NODATA: &str = "nodata";

/// The stable identity a strip is keyed by. `DOORWAY_ID` first (stable across
/// restarts), `NODE_ID` second — matching `FederationHealthStats::self_id`, so
/// the page's heading and its uptime history can never name different nodes.
pub fn uptime_node_key(args: &Args) -> String {
    args.doorway_id
        .clone()
        .unwrap_or_else(|| args.node_id.to_string())
}

/// Floor a timestamp to the start of its hourly bucket (epoch seconds).
pub fn bucket_start(ts: DateTime<Utc>) -> i64 {
    ts.timestamp().div_euclid(UPTIME_BUCKET_SECS) * UPTIME_BUCKET_SECS
}

/// The rendered strip plus the percentage derived from it.
#[derive(Debug, Clone)]
pub struct UptimeStrip {
    /// Exactly [`UPTIME_BUCKETS`] segment labels, oldest first.
    pub segments: Vec<String>,
    /// Percentage of OBSERVED buckets that were up. `None` when nothing was
    /// observed at all — never a misleading 0 or 100.
    pub uptime_7d: Option<f32>,
}

/// Pure bucket computation — the whole classification rule in one place so it
/// is testable without Mongo, a clock, or an AppState.
pub fn compute_strip(
    now: DateTime<Utc>,
    started_at: DateTime<Utc>,
    recorded: &HashMap<i64, String>,
) -> UptimeStrip {
    let newest = bucket_start(now);
    let oldest = newest - (UPTIME_BUCKETS as i64 - 1) * UPTIME_BUCKET_SECS;
    let process_from = bucket_start(started_at);
    let first_recorded = recorded.keys().copied().min();

    let mut segments = Vec::with_capacity(UPTIME_BUCKETS);
    let mut observed: u32 = 0;
    let mut up: u32 = 0;

    let mut bucket = oldest;
    while bucket <= newest {
        let seg = match recorded.get(&bucket) {
            Some(status) => status.clone(),
            // The current process vouches for every bucket since its own start.
            None if bucket >= process_from => SEG_UP.to_string(),
            // A gap inside a window we were demonstrably recording: a real outage.
            None if first_recorded.is_some_and(|first| bucket > first) => SEG_DOWN.to_string(),
            None => SEG_NODATA.to_string(),
        };
        if seg != SEG_NODATA {
            observed += 1;
            if seg == SEG_UP {
                up += 1;
            }
        }
        segments.push(seg);
        bucket += UPTIME_BUCKET_SECS;
    }

    let uptime_7d = if observed == 0 {
        None
    } else {
        Some((f64::from(up) * 100.0 / f64::from(observed)) as f32)
    };

    UptimeStrip {
        segments,
        uptime_7d,
    }
}

/// The status this doorway records for the CURRENT bucket. A process that is
/// not running writes nothing at all — which is exactly why an unrecorded
/// bucket inside a recording window reads as `down` above.
fn current_status(state: &AppState) -> &'static str {
    let conductor_ok = state
        .pool
        .as_ref()
        .map(|pool| pool.is_healthy())
        .unwrap_or(false);
    if conductor_ok || state.args.dev_mode {
        SEG_UP
    } else {
        SEG_DEGRADED
    }
}

/// Read back the buckets covering the rendered window. One bounded read per
/// page render; empty (never an error) when Mongo is absent or the query fails.
pub async fn load_recorded(state: &AppState, now: DateTime<Utc>) -> HashMap<i64, String> {
    let mut out = HashMap::new();
    let Some(ref mongo) = state.mongo else {
        return out;
    };

    let oldest = bucket_start(now) - (UPTIME_BUCKETS as i64 - 1) * UPTIME_BUCKET_SECS;
    let node_key = uptime_node_key(&state.args);
    let coll = mongo
        .inner()
        .database(mongo.db_name())
        .collection::<bson::Document>(UPTIME_COLLECTION);

    let cursor = coll
        .find(bson::doc! {
            "node_key": &node_key,
            "bucket_start": { "$gte": oldest },
        })
        .await;

    match cursor {
        Ok(mut cursor) => {
            use futures_util::TryStreamExt;
            loop {
                match cursor.try_next().await {
                    Ok(Some(doc)) => {
                        let bucket = doc
                            .get_i64("bucket_start")
                            .ok()
                            .or_else(|| doc.get_i32("bucket_start").ok().map(i64::from));
                        if let (Some(bucket), Ok(status)) = (bucket, doc.get_str("status")) {
                            out.insert(bucket, status.to_string());
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!("self-uptime history read failed: {e}");
                        break;
                    }
                }
            }
        }
        Err(e) => tracing::warn!("self-uptime history query failed: {e}"),
    }

    out
}

/// Upsert the current hourly bucket. Idempotent: repeated calls inside the same
/// hour rewrite the same document.
async fn record_current_bucket(state: &AppState) {
    let Some(ref mongo) = state.mongo else {
        return;
    };
    let now = Utc::now();
    let bucket = bucket_start(now);
    let node_key = uptime_node_key(&state.args);
    let status = current_status(state);

    let coll = mongo
        .inner()
        .database(mongo.db_name())
        .collection::<bson::Document>(UPTIME_COLLECTION);

    if let Err(e) = coll
        .update_one(
            bson::doc! { "node_key": &node_key, "bucket_start": bucket },
            bson::doc! { "$set": {
                "node_key": &node_key,
                "bucket_start": bucket,
                "status": status,
                "updated_at": now.to_rfc3339(),
            }},
        )
        .upsert(true)
        .await
    {
        tracing::warn!("self-uptime heartbeat write failed: {e}");
    }
}

/// Spawn the hourly self-heartbeat recorder. One write per hour, plus one at
/// boot (tokio's first `tick()` fires immediately) so a restart is recorded
/// without waiting an hour. No-op without Mongo — the strip degrades honestly.
pub fn spawn_self_heartbeat_task(state: Arc<AppState>) {
    if state.mongo.is_none() {
        tracing::info!(
            "self-uptime heartbeat disabled (no Mongo) — 7-day strip renders process-derived buckets"
        );
        return;
    }
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(UPTIME_BUCKET_SECS as u64));
        loop {
            interval.tick().await;
            record_current_bucket(&state).await;
        }
    });
    tracing::info!("self-uptime heartbeat enabled (hourly bucket, 7-day window)");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(epoch: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(epoch, 0).single().expect("valid epoch")
    }

    /// A known start time and a known set of recorded hours must produce a
    /// deterministic 168-bucket strip and percentage.
    #[test]
    fn buckets_from_known_start_and_recorded_hours() {
        // now = bucket 1_000_000 * 3600; process started 2 buckets ago.
        let now_bucket = 1_000_000 * UPTIME_BUCKET_SECS;
        let now = at(now_bucket + 1_800); // mid-bucket
        let started_at = at(now_bucket - 2 * UPTIME_BUCKET_SECS + 60);

        let mut recorded = HashMap::new();
        // Five older recorded buckets: 4 up, 1 degraded — with a one-bucket gap
        // between them that predates this process (a real outage).
        recorded.insert(now_bucket - 10 * UPTIME_BUCKET_SECS, SEG_UP.to_string());
        recorded.insert(now_bucket - 9 * UPTIME_BUCKET_SECS, SEG_UP.to_string());
        // (gap at -8)
        recorded.insert(
            now_bucket - 7 * UPTIME_BUCKET_SECS,
            SEG_DEGRADED.to_string(),
        );
        recorded.insert(now_bucket - 6 * UPTIME_BUCKET_SECS, SEG_UP.to_string());
        recorded.insert(now_bucket - 5 * UPTIME_BUCKET_SECS, SEG_UP.to_string());

        let strip = compute_strip(now, started_at, &recorded);

        assert_eq!(strip.segments.len(), UPTIME_BUCKETS);

        // Index of a bucket N hours before now, in an oldest-first strip.
        let idx = |hours_ago: usize| UPTIME_BUCKETS - 1 - hours_ago;

        // Everything older than the oldest record is honestly unknown.
        assert_eq!(strip.segments[idx(11)], SEG_NODATA);
        assert_eq!(strip.segments[0], SEG_NODATA);

        // Recorded buckets render exactly as recorded.
        assert_eq!(strip.segments[idx(10)], SEG_UP);
        assert_eq!(strip.segments[idx(9)], SEG_UP);
        assert_eq!(strip.segments[idx(7)], SEG_DEGRADED);
        assert_eq!(strip.segments[idx(6)], SEG_UP);
        assert_eq!(strip.segments[idx(5)], SEG_UP);

        // Gap inside the recording window, before this process started → down.
        assert_eq!(strip.segments[idx(8)], SEG_DOWN);
        assert_eq!(strip.segments[idx(4)], SEG_DOWN);
        assert_eq!(strip.segments[idx(3)], SEG_DOWN);

        // This process vouches for its own lifetime (buckets -2, -1, and now).
        assert_eq!(strip.segments[idx(2)], SEG_UP);
        assert_eq!(strip.segments[idx(1)], SEG_UP);
        assert_eq!(strip.segments[idx(0)], SEG_UP);

        // Observed = 11 buckets (-10..0); up = 7, degraded = 1, down = 3.
        let observed = strip
            .segments
            .iter()
            .filter(|s| s.as_str() != SEG_NODATA)
            .count();
        assert_eq!(observed, 11);
        let pct = strip.uptime_7d.expect("observed buckets ⇒ a percentage");
        assert!(
            (pct - (7.0 * 100.0 / 11.0)).abs() < 0.01,
            "expected 7/11 up, got {pct}"
        );
    }

    #[test]
    fn no_history_degrades_to_process_lifetime() {
        let now_bucket = 900_000 * UPTIME_BUCKET_SECS;
        let now = at(now_bucket + 10);
        let started_at = at(now_bucket - 3 * UPTIME_BUCKET_SECS);
        let strip = compute_strip(now, started_at, &HashMap::new());

        assert_eq!(strip.segments.len(), UPTIME_BUCKETS);
        // Four buckets covered by this process; the rest honestly unknown.
        assert_eq!(
            strip
                .segments
                .iter()
                .filter(|s| s.as_str() == SEG_UP)
                .count(),
            4
        );
        assert_eq!(
            strip
                .segments
                .iter()
                .filter(|s| s.as_str() == SEG_NODATA)
                .count(),
            UPTIME_BUCKETS - 4
        );
        // 4/4 observed buckets up — never reported as 100% of seven days.
        assert_eq!(strip.uptime_7d, Some(100.0));
    }

    #[test]
    fn bucket_start_floors_to_the_hour() {
        assert_eq!(bucket_start(at(3_600)), 3_600);
        assert_eq!(bucket_start(at(3_659)), 3_600);
        assert_eq!(bucket_start(at(7_199)), 3_600);
        assert_eq!(bucket_start(at(7_200)), 7_200);
    }
}
