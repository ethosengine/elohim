//! `BlockSpan` — reading, and lifting, a conductor's blocks.
//!
//! Holochain 0.7's `integrate_dht_ops_workflow` blocks the *author's cell* from
//! `Timestamp::now()` to `Timestamp::max()` when an op that author wrote is
//! integrated as invalid (`CellBlockReason::InvalidOp`). The peer store then
//! drops that agent's infos and gossip with it never starts again. 0.7 exposes
//! no unblock: no admin call, no HDK host fn. One rejected op per author is
//! enough to partition a household forever, so the household needs to be able to
//! see the row and remove it under its own governance.
//!
//! Schema (`holochain_data-0.7.0/migrations/conductor/…initial_schema.up.sql`):
//!
//! ```sql
//! CREATE TABLE BlockSpan (
//!     id            INTEGER PRIMARY KEY AUTOINCREMENT,
//!     target_id     BLOB NOT NULL,   -- holochain_serialized_bytes(BlockTargetId)
//!     target_reason BLOB NOT NULL,   -- holochain_serialized_bytes(BlockTargetReason)
//!     start_us      INTEGER NOT NULL,
//!     end_us        INTEGER NOT NULL
//! ) STRICT;
//! ```

use anyhow::{Context, Result};
use holochain_zome_types::block::{BlockTargetId, BlockTargetReason, CellBlockReason};
use rusqlite::Connection;

use crate::fmt;

/// One decoded `BlockSpan` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRow {
    pub id: i64,
    /// `dna:agent` in holo_hash b64 form for a cell block, or the IP for an IP
    /// block.
    pub target: String,
    /// Rendered reason, e.g. `InvalidOp(uhCkk…)`.
    pub reason: String,
    /// The op hash the block cites, when the reason carries one. This is the row
    /// to look for in `rejected`.
    pub invalid_op: Option<String>,
    pub start_us: i64,
    pub end_us: i64,
    /// Raw bytes of `target_id`, kept so a caller can group rows without
    /// re-encoding.
    pub target_id_bytes: Vec<u8>,
}

impl BlockRow {
    /// A block whose `end_us` is `Timestamp::max()` never lapses; only a
    /// deliberate lift clears it.
    pub fn is_permanent(&self) -> bool {
        self.end_us >= crate::fmt::MAX_RENDERABLE_US
    }

    pub fn render(&self) -> String {
        format!(
            "  #{id}  {target}\n        reason: {reason}\n        from:   {start}\n        until:  {end}",
            id = self.id,
            target = self.target,
            reason = self.reason,
            start = fmt::timestamp_us(self.start_us),
            end = fmt::timestamp_us(self.end_us),
        )
    }
}

fn decode_target(bytes: &[u8]) -> Result<String> {
    let id: BlockTargetId = holochain_serialized_bytes::decode(bytes)
        .context("decoding BlockSpan.target_id as BlockTargetId")?;
    Ok(match id {
        BlockTargetId::Cell(cell_id) => format!(
            "{}:{}",
            fmt::hash_b64(cell_id.dna_hash().get_raw_39()),
            fmt::hash_b64(cell_id.agent_pubkey().get_raw_39())
        ),
        BlockTargetId::Ip(ip) => format!("ip:{ip}"),
    })
}

fn decode_reason(bytes: &[u8]) -> Result<(String, Option<String>)> {
    let reason: BlockTargetReason = holochain_serialized_bytes::decode(bytes)
        .context("decoding BlockSpan.target_reason as BlockTargetReason")?;
    Ok(match reason {
        BlockTargetReason::Cell(CellBlockReason::InvalidOp(op_hash)) => {
            let h = fmt::hash_b64(op_hash.get_raw_39());
            (format!("InvalidOp({h})"), Some(h))
        }
        BlockTargetReason::Cell(CellBlockReason::BadCrypto) => ("BadCrypto".to_string(), None),
        BlockTargetReason::Ip(r) => (format!("{r:?}"), None),
    })
}

/// Every `BlockSpan` row in a conductor database, oldest first.
pub fn list(conn: &Connection) -> Result<Vec<BlockRow>> {
    let mut stmt = conn
        .prepare("SELECT id, target_id, target_reason, start_us, end_us FROM BlockSpan ORDER BY id")
        .context("preparing BlockSpan query")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })
        .context("reading BlockSpan")?;

    let mut out = Vec::new();
    for row in rows {
        let (id, target_id_bytes, reason_bytes, start_us, end_us) = row?;
        let target = decode_target(&target_id_bytes)?;
        let (reason, invalid_op) = decode_reason(&reason_bytes)?;
        out.push(BlockRow {
            id,
            target,
            reason,
            invalid_op,
            start_us,
            end_us,
            target_id_bytes,
        });
    }
    Ok(out)
}

/// A cell selector for `unblock`: `<dna>:<agent>`, or `<dna>:*` for every agent
/// blocked in that DNA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellSelector {
    pub dna: String,
    /// `None` means the `*` wildcard.
    pub agent: Option<String>,
}

impl CellSelector {
    pub fn parse(s: &str) -> Result<Self> {
        let (dna, agent) = s.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("--cell must be <dna>:<agent> or <dna>:* (got {s:?})")
        })?;
        if dna.is_empty() || agent.is_empty() {
            anyhow::bail!("--cell must be <dna>:<agent> or <dna>:* (got {s:?})");
        }
        Ok(Self {
            dna: dna.to_string(),
            agent: if agent == "*" {
                None
            } else {
                Some(agent.to_string())
            },
        })
    }

    pub fn matches(&self, row: &BlockRow) -> bool {
        let Some((dna, agent)) = row.target.split_once(':') else {
            return false;
        };
        dna == self.dna && self.agent.as_deref().is_none_or(|a| a == agent)
    }
}

/// Delete every `BlockSpan` row matching `selector`, returning the rows removed.
///
/// Rows are deleted by primary key, taken from a prior decode of the same rows —
/// nothing is re-encoded and matched by blob, so a serialization difference can
/// never widen the delete. `BlockSpan` is the only table this tool ever writes.
pub fn delete_matching(conn: &Connection, selector: &CellSelector) -> Result<Vec<BlockRow>> {
    let doomed: Vec<BlockRow> = list(conn)?
        .into_iter()
        .filter(|r| selector.matches(r))
        .collect();
    if doomed.is_empty() {
        return Ok(doomed);
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare("DELETE FROM BlockSpan WHERE id = ?1")?;
        for row in &doomed {
            stmt.execute([row.id])
                .with_context(|| format!("deleting BlockSpan #{}", row.id))?;
        }
    }
    tx.commit().context("committing the lift")?;
    Ok(doomed)
}

/// Create the `BlockSpan` table, verbatim from the 0.7.0 conductor migration.
///
/// Fixture support: the tool builds its own test databases so the read and lift
/// paths are exercised against the real schema and the real msgpack encoding.
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS BlockSpan (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            target_id BLOB NOT NULL,
            target_reason BLOB NOT NULL,
            start_us INTEGER NOT NULL,
            end_us INTEGER NOT NULL
        ) STRICT;
        CREATE INDEX IF NOT EXISTS idx_block_span_start_us ON BlockSpan(start_us);
        CREATE INDEX IF NOT EXISTS idx_block_span_end_us ON BlockSpan(end_us);
        CREATE INDEX IF NOT EXISTS idx_block_span_target_id ON BlockSpan(target_id);",
    )
    .context("creating the BlockSpan schema")?;
    Ok(())
}

/// Insert a block the way `holochain_data::conductor::block` does.
///
/// Fixture support only — writing a block is not an operator verb.
pub fn insert(
    conn: &Connection,
    target_id: &BlockTargetId,
    reason: &BlockTargetReason,
    start_us: i64,
    end_us: i64,
) -> Result<()> {
    let id_bytes = holochain_serialized_bytes::encode(target_id)
        .map_err(|e| anyhow::anyhow!("encoding BlockTargetId: {e}"))?;
    let reason_bytes = holochain_serialized_bytes::encode(reason)
        .map_err(|e| anyhow::anyhow!("encoding BlockTargetReason: {e}"))?;
    conn.execute(
        "INSERT INTO BlockSpan (target_id, target_reason, start_us, end_us) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id_bytes, reason_bytes, start_us, end_us],
    )
    .context("inserting a fixture BlockSpan row")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_zome_types::prelude::*;

    fn hash(prefix: [u8; 3], seed: u8) -> Vec<u8> {
        let mut v = prefix.to_vec();
        v.extend(std::iter::repeat_n(seed, 36));
        v
    }

    fn dna(seed: u8) -> DnaHash {
        DnaHash::from_raw_39(hash([0x84, 0x2d, 0x24], seed))
    }

    fn agent(seed: u8) -> AgentPubKey {
        AgentPubKey::from_raw_39(hash([0x84, 0x20, 0x24], seed))
    }

    fn op(seed: u8) -> DhtOpHash {
        DhtOpHash::from_raw_39(hash([0x84, 0x24, 0x24], seed))
    }

    /// A fixture conductor database with the real schema and three blocks: two
    /// agents blocked in one DNA, one agent blocked in another.
    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        for (d, a, o) in [(1u8, 10u8, 100u8), (1, 11, 101), (2, 10, 102)] {
            insert(
                &conn,
                &BlockTargetId::Cell(CellId::new(dna(d), agent(a))),
                &BlockTargetReason::Cell(CellBlockReason::InvalidOp(op(o))),
                1_788_000_000_000_000,
                i64::MAX,
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn list_decodes_target_reason_and_span() {
        let conn = fixture();
        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 3);

        let first = &rows[0];
        assert_eq!(
            first.target,
            format!(
                "{}:{}",
                fmt::hash_b64(dna(1).get_raw_39()),
                fmt::hash_b64(agent(10).get_raw_39())
            )
        );
        assert_eq!(
            first.reason,
            format!("InvalidOp({})", fmt::hash_b64(op(100).get_raw_39()))
        );
        let cited = fmt::hash_b64(op(100).get_raw_39());
        assert_eq!(first.invalid_op.as_deref(), Some(cited.as_str()));
        assert!(
            first.is_permanent(),
            "integrate_dht_ops_workflow writes Timestamp::max as end_us"
        );
        assert!(first.render().contains("never (Timestamp::max"));
    }

    #[test]
    fn unblock_one_cell_leaves_the_others() {
        let conn = fixture();
        let selector = CellSelector::parse(&format!(
            "{}:{}",
            fmt::hash_b64(dna(1).get_raw_39()),
            fmt::hash_b64(agent(10).get_raw_39())
        ))
        .unwrap();

        let removed = delete_matching(&conn, &selector).unwrap();
        assert_eq!(removed.len(), 1, "exactly the named cell");
        assert_eq!(removed[0].id, 1);

        let left = list(&conn).unwrap();
        assert_eq!(left.len(), 2);
        assert!(left.iter().all(|r| !selector.matches(r)));
    }

    #[test]
    fn unblock_wildcard_lifts_every_agent_in_one_dna() {
        let conn = fixture();
        let selector =
            CellSelector::parse(&format!("{}:*", fmt::hash_b64(dna(1).get_raw_39()))).unwrap();

        let removed = delete_matching(&conn, &selector).unwrap();
        assert_eq!(removed.len(), 2, "both agents blocked in that DNA");

        let left = list(&conn).unwrap();
        assert_eq!(left.len(), 1, "the other DNA's block is untouched");
        assert_eq!(
            left[0].target,
            format!(
                "{}:{}",
                fmt::hash_b64(dna(2).get_raw_39()),
                fmt::hash_b64(agent(10).get_raw_39())
            )
        );
    }

    #[test]
    fn unblock_matching_nothing_removes_nothing() {
        let conn = fixture();
        let selector = CellSelector::parse("uhC0kNOPE:uhCAkNOPE").unwrap();
        assert!(delete_matching(&conn, &selector).unwrap().is_empty());
        assert_eq!(list(&conn).unwrap().len(), 3);
    }

    #[test]
    fn cell_selector_rejects_malformed_input() {
        assert!(CellSelector::parse("no-colon").is_err());
        assert!(CellSelector::parse(":agent").is_err());
        assert!(CellSelector::parse("dna:").is_err());
        assert_eq!(
            CellSelector::parse("dna:*").unwrap(),
            CellSelector {
                dna: "dna".into(),
                agent: None
            }
        );
    }
}
