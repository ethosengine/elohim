//! Bounded holding of well-signed gossip claims whose author is not bound YET.
//!
//! Source of truth: gossip claims (custody announcements, salvage ads, inventory
//! envelopes) whose signature verified under the author's own transport key, but whose
//! author no cross-signed chain (`p2p::transport_authority`) binds to an agent when they
//! arrive: the author's `AgentPeerBinding` is not minted or received yet, or its
//! transport link has not arrived. Custody is not re-announced when unchanged, so a
//! refusal here would lose the claim for good and leave a freshly recast household
//! undercounted.
//!
//! A held claim is never projected and never counted. It is bounded per topic
//! ([`HELD_UNBOUND_CAP`]), held once per distinct bytes, and expires after
//! [`HELD_UNBOUND_TTL_MS`]. When a binding or link for its author is recorded,
//! [`take_resolvable`] hands it back to be replayed through full admission; a claim that
//! still fails is dropped by the replaying handler with its reason. Forged or badly
//! signed claims are refused by admission and never reach this table. Category C,
//! local.

use diesel::prelude::*;

use super::diesel_schema::held_unbound_claims;
use crate::error::StorageError;

/// Claims held per topic (oldest evicted first).
pub const HELD_UNBOUND_CAP: usize = 256;

/// How long an unbound claim waits for its author's binding (15 minutes).
pub const HELD_UNBOUND_TTL_MS: i64 = 15 * 60 * 1000;

/// The plane a held claim arrived on, so its replay re-runs the same witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldPlane {
    Libp2p,
    Iroh,
}

impl HeldPlane {
    fn as_str(self) -> &'static str {
        match self {
            Self::Libp2p => "libp2p",
            Self::Iroh => "iroh",
        }
    }
}

/// One held claim, handed back for replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldClaim {
    pub topic: String,
    pub author_peer: String,
    pub plane: HeldPlane,
    pub gossipsub_source: Option<String>,
    pub bytes: Vec<u8>,
}

/// Hold one well-signed unbound claim. Expired rows are purged first; an identical
/// claim (same topic, same bytes) is held once; the topic keeps its newest
/// [`HELD_UNBOUND_CAP`].
pub fn hold(
    conn: &mut SqliteConnection,
    claim: &HeldClaim,
    now_ms: i64,
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        purge(conn, now_ms)?;
        let already: i64 = held_unbound_claims::table
            .filter(held_unbound_claims::topic.eq(&claim.topic))
            .filter(held_unbound_claims::bytes.eq(&claim.bytes))
            .count()
            .get_result(conn)?;
        if already > 0 {
            return Ok::<(), diesel::result::Error>(());
        }
        diesel::insert_into(held_unbound_claims::table)
            .values((
                held_unbound_claims::topic.eq(&claim.topic),
                held_unbound_claims::author_peer.eq(&claim.author_peer),
                held_unbound_claims::plane.eq(claim.plane.as_str()),
                held_unbound_claims::gossipsub_source.eq(claim.gossipsub_source.as_deref()),
                held_unbound_claims::bytes.eq(&claim.bytes),
                held_unbound_claims::held_at_ms.eq(now_ms),
            ))
            .execute(conn)?;
        let cap = i64::try_from(HELD_UNBOUND_CAP).unwrap_or(i64::MAX);
        let keep: Vec<i32> = held_unbound_claims::table
            .filter(held_unbound_claims::topic.eq(&claim.topic))
            .select(held_unbound_claims::id)
            .order(held_unbound_claims::id.desc())
            .limit(cap)
            .load(conn)?;
        diesel::delete(
            held_unbound_claims::table
                .filter(held_unbound_claims::topic.eq(&claim.topic))
                .filter(held_unbound_claims::id.ne_all(keep)),
        )
        .execute(conn)?;
        Ok(())
    })
    .map_err(|e| StorageError::Database(format!("held_unbound::hold: {e}")))
}

fn purge(conn: &mut SqliteConnection, now_ms: i64) -> QueryResult<usize> {
    diesel::delete(
        held_unbound_claims::table
            .filter(held_unbound_claims::held_at_ms.lt(now_ms.saturating_sub(HELD_UNBOUND_TTL_MS))),
    )
    .execute(conn)
}

/// Remove and return every unexpired held claim whose author `resolves` (now has a
/// binding). Expired claims are purged and never returned. The caller replays each
/// through full admission.
/// bounded-work: at most `HELD_UNBOUND_CAP` per topic are held at all.
pub fn take_resolvable(
    conn: &mut SqliteConnection,
    now_ms: i64,
    resolves: &mut dyn FnMut(&mut SqliteConnection, &str) -> bool,
) -> Result<Vec<HeldClaim>, StorageError> {
    type Row = (i32, String, String, String, Option<String>, Vec<u8>);
    let rows: Vec<Row> = super::read_then_write_transaction(conn, |conn| {
        purge(conn, now_ms)?;
        held_unbound_claims::table
            .select((
                held_unbound_claims::id,
                held_unbound_claims::topic,
                held_unbound_claims::author_peer,
                held_unbound_claims::plane,
                held_unbound_claims::gossipsub_source,
                held_unbound_claims::bytes,
            ))
            .order(held_unbound_claims::id.asc())
            .load::<Row>(conn)
    })
    .map_err(|e| StorageError::Database(format!("held_unbound::take_resolvable: {e}")))?;
    let mut taken = Vec::new();
    for (id, topic, author_peer, plane, gossipsub_source, bytes) in rows {
        if !resolves(conn, &author_peer) {
            continue;
        }
        diesel::delete(held_unbound_claims::table.find(id))
            .execute(conn)
            .map_err(|e| StorageError::Database(format!("held_unbound::take: {e}")))?;
        taken.push(HeldClaim {
            topic,
            author_peer,
            plane: if plane == "libp2p" {
                HeldPlane::Libp2p
            } else {
                HeldPlane::Iroh
            },
            gossipsub_source,
            bytes,
        });
    }
    Ok(taken)
}

/// How many claims are held.
pub fn count(conn: &mut SqliteConnection) -> Result<i64, StorageError> {
    held_unbound_claims::table
        .count()
        .get_result(conn)
        .map_err(StorageError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    fn claim(topic: &str, author: &str, bytes: &[u8]) -> HeldClaim {
        HeldClaim {
            topic: topic.into(),
            author_peer: author.into(),
            plane: HeldPlane::Iroh,
            gossipsub_source: None,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn held_once_bounded_per_topic_and_taken_only_when_resolvable() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        hold(&mut conn, &claim("t/a", "author-1", b"x"), 0).unwrap();
        hold(&mut conn, &claim("t/a", "author-1", b"x"), 0).unwrap();
        assert_eq!(count(&mut conn).unwrap(), 1);
        for i in 0..HELD_UNBOUND_CAP + 5 {
            hold(&mut conn, &claim("t/b", "author-2", &i.to_be_bytes()), 0).unwrap();
        }
        assert_eq!(count(&mut conn).unwrap(), HELD_UNBOUND_CAP as i64 + 1);
        let taken = take_resolvable(&mut conn, 1, &mut |_, a| a == "author-1").unwrap();
        assert_eq!(taken, vec![claim("t/a", "author-1", b"x")]);
        assert_eq!(count(&mut conn).unwrap(), HELD_UNBOUND_CAP as i64);
    }
}
