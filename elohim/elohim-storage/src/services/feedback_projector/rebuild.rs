//! Rebuild only derived state; retain the old generation and its action references.
use super::*;

/// Serializes generation switches with the single live projector's tick.
/// Notifications only add subscriptions and never acquire this lock.
pub static WRITER: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn start(conn: &mut SqliteConnection, evaluator: &[u8]) -> Result<i32, StorageError> {
    let old = gen_db::published_generation(conn, evaluator)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .ok_or_else(|| StorageError::InvalidInput("no published generation to rebuild".into()))?;
    if gen_db::in_flight_generation(conn, evaluator)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .is_some()
    {
        return Err(StorageError::InvalidInput(
            "generation already rebuilding".into(),
        ));
    }
    conn.transaction::<i32, diesel::result::Error, _>(|c| {
        let generation = gen_db::create_generation(
            c,
            evaluator,
            &old.policy_manifest_cid,
            &old.policy_bytes,
            GEN_REBUILDING,
            &Utc::now().to_rfc3339(),
        )?;
        // Keep every retained reference, even if its original link disappears.
        // Pending members get independent service before fresh link discovery.
        diesel::sql_query(
            "INSERT INTO feedback_application_member
            (generation_id, origin_dna_hash, action_hash, group_key, member_status,
             member_role, author_pubkey, action_timestamp_micros, last_error, discovered_at)
            SELECT ?, origin_dna_hash, action_hash, group_key, 'pending', member_role,
                author_pubkey, action_timestamp_micros, NULL, discovered_at
            FROM feedback_application_member WHERE generation_id = ?",
        )
        .bind::<diesel::sql_types::Integer, _>(generation)
        .bind::<diesel::sql_types::Integer, _>(old.generation_id)
        .execute(c)?;
        Ok(generation)
    })
    .map_err(|e| StorageError::Database(e.to_string()))
}

/// Explicit test opt-in plus a per-peer, consumed arm file. Removal precedes
/// abort so the storage restart cannot re-enter this crash window.
pub(super) fn crash_once() {
    if std::env::var("ELOHIM_TEST_FEEDBACK_CRASH_ONCE").as_deref() != Ok("1") {
        return;
    }
    let Ok(dir) = std::env::var("STORAGE_DIR") else {
        return;
    };
    let arm = std::path::Path::new(&dir).join("feedback-crash-once");
    if std::fs::remove_file(&arm).is_ok() {
        tracing::error!("TEST feedback crash after application row before aggregate write");
        std::process::abort();
    }
}
