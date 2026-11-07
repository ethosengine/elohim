//! Bounded opaque retention of inventory envelopes this build cannot judge.
//!
//! An envelope under an unknown signature scheme or epoch is neither authentic nor
//! forged here (`Held`, D7). It is kept — opaque bytes, the topic it arrived on and why
//! it was held — so a later build can judge it, but it is never projected into
//! `peer_blob_inventory` and never counted toward custody. The table is bounded by
//! [`HELD_ENVELOPE_CAP`] per topic: each retain evicts the oldest rows of its topic
//! beyond it, and an identical envelope is held once.
//! Category C (local, operational).

use diesel::prelude::*;

use super::diesel_schema::held_envelopes;
use crate::error::StorageError;
pub use crate::p2p::inventory_gossip::HELD_ENVELOPE_CAP;

/// Retain one held envelope. The cap is per topic: each retain evicts the oldest rows
/// of ITS topic beyond [`HELD_ENVELOPE_CAP`], so a flood on one topic cannot evict
/// what another topic held. An envelope already held (same topic, same bytes) is not
/// stored twice, so a replayed held envelope cannot evict the others either. Topics
/// are this build's fixed gossip topic constants, so the table stays bounded.
pub fn retain(
    conn: &mut SqliteConnection,
    topic: &str,
    reason: &str,
    bytes: &[u8],
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let already: i64 = held_envelopes::table
            .filter(held_envelopes::topic.eq(topic))
            .filter(held_envelopes::bytes.eq(bytes))
            .count()
            .get_result(conn)?;
        if already > 0 {
            return Ok::<(), diesel::result::Error>(());
        }
        diesel::insert_into(held_envelopes::table)
            .values((
                held_envelopes::topic.eq(topic),
                held_envelopes::reason.eq(reason),
                held_envelopes::bytes.eq(bytes),
            ))
            .execute(conn)?;
        let cap = i64::try_from(HELD_ENVELOPE_CAP).unwrap_or(i64::MAX);
        let keep: Vec<i32> = held_envelopes::table
            .filter(held_envelopes::topic.eq(topic))
            .select(held_envelopes::id)
            .order(held_envelopes::id.desc())
            .limit(cap)
            .load(conn)?;
        diesel::delete(
            held_envelopes::table
                .filter(held_envelopes::topic.eq(topic))
                .filter(held_envelopes::id.ne_all(keep)),
        )
        .execute(conn)?;
        Ok(())
    })
    .map_err(|e| StorageError::Database(format!("held_envelopes::retain: {e}")))
}

/// How many envelopes are held.
pub fn count(conn: &mut SqliteConnection) -> Result<i64, StorageError> {
    held_envelopes::table
        .count()
        .get_result(conn)
        .map_err(StorageError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn retain_is_bounded_and_keeps_the_newest() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for i in 0..HELD_ENVELOPE_CAP + 10 {
            retain(
                &mut conn,
                "elohim/inventory/blob/2",
                "scheme:7",
                &i.to_be_bytes(),
            )
            .unwrap();
        }
        assert_eq!(count(&mut conn).unwrap(), HELD_ENVELOPE_CAP as i64);
        let oldest: Vec<u8> = held_envelopes::table
            .select(held_envelopes::bytes)
            .order(held_envelopes::id.asc())
            .first(&mut conn)
            .unwrap();
        assert_eq!(oldest, 10usize.to_be_bytes().to_vec());
    }

    #[test]
    fn a_flood_on_one_topic_never_evicts_another_and_duplicates_are_held_once() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        retain(&mut conn, "elohim/storage/salvage", "scheme:7", b"keep me").unwrap();
        retain(&mut conn, "elohim/storage/salvage", "scheme:7", b"keep me").unwrap();
        assert_eq!(
            count(&mut conn).unwrap(),
            1,
            "the same envelope is held once"
        );
        for i in 0..HELD_ENVELOPE_CAP + 10 {
            retain(
                &mut conn,
                "elohim/storage/custody/2",
                "scheme:7",
                &i.to_be_bytes(),
            )
            .unwrap();
        }
        let salvage: i64 = held_envelopes::table
            .filter(held_envelopes::topic.eq("elohim/storage/salvage"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(salvage, 1, "another topic's flood evicted nothing here");
        assert_eq!(count(&mut conn).unwrap(), HELD_ENVELOPE_CAP as i64 + 1);
    }
}
