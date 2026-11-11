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
//! [`take_for_authors`] hands it back to be replayed through full admission; a claim that
//! still fails is dropped by the replaying handler with its reason. Forged or badly
//! signed claims are refused by admission and never reach this table. Category C,
//! local.

use diesel::prelude::*;

use super::diesel_schema::held_unbound_claims;
use crate::error::StorageError;

/// Claims held per topic (oldest evicted first).
pub const HELD_UNBOUND_CAP: usize = 256;

/// Claims held per author (oldest of that author evicted first), so one sender —
/// or one key among many a Sybil mints — cannot occupy a whole topic's window alone.
pub const HELD_UNBOUND_PER_AUTHOR: usize = 32;

/// How long an unbound claim waits for its author's binding: 4 hours. A cold fleet
/// installs its conductors hours after storage boots (alpha re-genesis: about 1 h),
/// and the author's binding must then be minted, gossiped and DHT-confirmed
/// (`p2p::transport_authority::confirm`: attempts over ~33 min, then a sweep every
/// 10 min) before its held claims replay. The table stays bounded by the per-topic
/// and per-author caps whatever the TTL.
pub const HELD_UNBOUND_TTL_MS: i64 = 4 * 60 * 60 * 1000;

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
        let per_author = i64::try_from(HELD_UNBOUND_PER_AUTHOR).unwrap_or(i64::MAX);
        let keep_author: Vec<i32> = held_unbound_claims::table
            .filter(held_unbound_claims::author_peer.eq(&claim.author_peer))
            .select(held_unbound_claims::id)
            .order(held_unbound_claims::id.desc())
            .limit(per_author)
            .load(conn)?;
        diesel::delete(
            held_unbound_claims::table
                .filter(held_unbound_claims::author_peer.eq(&claim.author_peer))
                .filter(held_unbound_claims::id.ne_all(keep_author)),
        )
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

/// Remove and return the unexpired held claims of `authors` (the transport ids just
/// bound) whose author `resolves`. Expired claims are purged and never returned. Only
/// the named authors' rows are read (indexed), so a binding or link recorded for one
/// author never makes this node re-judge anyone else's claims.
/// bounded-work: at most `HELD_UNBOUND_PER_AUTHOR` per author are held at all.
pub fn take_for_authors(
    conn: &mut SqliteConnection,
    now_ms: i64,
    authors: &[String],
    resolves: &mut dyn FnMut(&mut SqliteConnection, &str) -> bool,
) -> Result<Vec<HeldClaim>, StorageError> {
    type Row = (i32, String, String, String, Option<String>, Vec<u8>);
    if authors.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<Row> = super::read_then_write_transaction(conn, |conn| {
        purge(conn, now_ms)?;
        held_unbound_claims::table
            .filter(held_unbound_claims::author_peer.eq_any(authors))
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
    .map_err(|e| StorageError::Database(format!("held_unbound::take_for_authors: {e}")))?;
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
        // One author's flood is capped per author before the topic cap.
        assert_eq!(
            count(&mut conn).unwrap(),
            HELD_UNBOUND_PER_AUTHOR as i64 + 1
        );
        // Only the named, resolving authors' claims are taken.
        let none = take_for_authors(&mut conn, 1, &["author-2".into()], &mut |_, _| false).unwrap();
        assert!(none.is_empty());
        let taken = take_for_authors(&mut conn, 1, &["author-1".into()], &mut |_, _| true).unwrap();
        assert_eq!(taken, vec![claim("t/a", "author-1", b"x")]);
        assert_eq!(count(&mut conn).unwrap(), HELD_UNBOUND_PER_AUTHOR as i64);
        for i in 0..HELD_UNBOUND_CAP + 5 {
            hold(&mut conn, &claim("t/b", &format!("sybil-{i}"), b"y"), 0).unwrap();
        }
        // Distinct authors, identical bytes: held once per topic.
        assert_eq!(
            count(&mut conn).unwrap(),
            HELD_UNBOUND_PER_AUTHOR as i64 + 1
        );
        for i in 0..HELD_UNBOUND_CAP + 5 {
            hold(
                &mut conn,
                &claim("t/c", &format!("sybil-{i}"), &i.to_be_bytes()),
                0,
            )
            .unwrap();
        }
        let in_c: i64 = held_unbound_claims::table
            .filter(held_unbound_claims::topic.eq("t/c"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(
            in_c, HELD_UNBOUND_CAP as i64,
            "many senders still meet the topic cap"
        );
    }
}
