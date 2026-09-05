//! Rejected ops and warrants in a per-DNA DHT database.
//!
//! A block cites a `DhtOpHash`. This module finds that op and says who authored
//! it, where in their chain it sits, and when — the evidence a household needs
//! before it decides whether to lift the block.
//!
//! Holochain 0.7 splits what earlier versions called `DhtOp` across two tables
//! (`holochain_data-0.7.0/migrations/dht/…initial_schema.up.sql`):
//!
//! * `LimboChainOp` — received, not yet integrated. Carries
//!   `sys_validation_status` and `app_validation_status`; NULL = pending,
//!   1 = accepted, 2 = rejected.
//! * `ChainOp` — integrated. Carries a single NOT NULL `validation_status` on
//!   the same 1/2 convention.
//!
//! Both join `Action` for author, chain sequence, action type and timestamp.
//! `Warrant` / `WarrantOp` carry the peer-visible accusation that produced the
//! block in the first place, so they are read alongside.

use anyhow::{Context, Result};
use holochain_zome_types::prelude::{ActionType, ChainOpType};
use rusqlite::Connection;

use crate::fmt::{self, HashKind};

/// Which table a rejection was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpState {
    /// Received and rejected, never integrated.
    Limbo,
    /// Integrated with a rejected status — this is the state that blocks the
    /// author's cell.
    Integrated,
}

impl OpState {
    pub fn as_str(self) -> &'static str {
        match self {
            OpState::Limbo => "limbo",
            OpState::Integrated => "integrated",
        }
    }
}

/// One rejected op, joined to the action that produced it.
#[derive(Debug, Clone)]
pub struct RejectedOp {
    pub state: OpState,
    pub op_hash: String,
    pub op_type: String,
    pub author: String,
    pub seq: i64,
    pub action_type: String,
    pub action_timestamp_us: i64,
    /// `sys` / `app` status, rendered. `ChainOp` carries only the single
    /// post-integration status, reported as `sys`.
    pub sys_status: Option<i64>,
    pub app_status: Option<i64>,
    /// `when_integrated` for an integrated op, `when_received` for a limbo one.
    pub when_us: i64,
}

impl RejectedOp {
    pub fn render(&self) -> String {
        format!(
            "  [{state}] {op_type} {op_hash}\n        author: {author} seq {seq} ({action_type})\n        authored: {authored}\n        {when_label}: {when}   sys={sys} app={app}",
            state = self.state.as_str(),
            op_type = self.op_type,
            op_hash = self.op_hash,
            author = self.author,
            seq = self.seq,
            action_type = self.action_type,
            authored = fmt::timestamp_us(self.action_timestamp_us),
            when_label = match self.state {
                OpState::Integrated => "integrated",
                OpState::Limbo => "received",
            },
            when = fmt::timestamp_us(self.when_us),
            sys = render_status(self.sys_status),
            app = render_status(self.app_status),
        )
    }
}

/// A warrant: one peer's signed accusation that another peer authored an invalid
/// op. This is what `integrate_dht_ops_workflow` turns into a block.
#[derive(Debug, Clone)]
pub struct WarrantRow {
    pub hash: String,
    pub author: String,
    pub warrantee: String,
    pub timestamp_us: i64,
    pub reason: Option<String>,
    /// Present once the warrant is integrated.
    pub validation_status: Option<i64>,
}

impl WarrantRow {
    pub fn render(&self) -> String {
        format!(
            "  {hash}\n        warrantee: {warrantee}\n        witness:   {author}\n        at:        {at}   status={status}\n        reason:    {reason}",
            hash = self.hash,
            warrantee = self.warrantee,
            author = self.author,
            at = fmt::timestamp_us(self.timestamp_us),
            status = render_status(self.validation_status),
            reason = self.reason.as_deref().unwrap_or("<none recorded>"),
        )
    }
}

fn render_status(v: Option<i64>) -> String {
    match v {
        None => "pending".to_string(),
        Some(1) => "accepted".to_string(),
        Some(2) => "REJECTED".to_string(),
        Some(other) => format!("?{other}"),
    }
}

fn render_op_type(v: i64) -> String {
    match ChainOpType::try_from(v) {
        Ok(t) => t.to_string(),
        Err(bad) => format!("<unknown op_type {bad}>"),
    }
}

fn render_action_type(v: i64) -> String {
    match ActionType::try_from(v) {
        Ok(t) => t.to_string(),
        Err(_) => format!("<unknown action_type {v}>"),
    }
}

/// Column order shared by both rejected-op queries.
const OP_COLUMNS: &str = "o.hash, o.op_type, a.author, a.seq, a.action_type, a.timestamp";

/// Every rejected op in a DHT database, integrated ones first.
pub fn list(conn: &Connection) -> Result<Vec<RejectedOp>> {
    let mut out = Vec::new();

    let mut stmt = conn
        .prepare(&format!(
            "SELECT {OP_COLUMNS}, o.validation_status, o.when_integrated
             FROM ChainOp o JOIN Action a ON a.hash = o.action_hash
             WHERE o.validation_status = 2
             ORDER BY o.when_integrated"
        ))
        .context("preparing the integrated-rejected query")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(RejectedOp {
                state: OpState::Integrated,
                op_hash: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(0)?, HashKind::DhtOp),
                op_type: render_op_type(r.get(1)?),
                author: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(2)?, HashKind::Agent),
                seq: r.get(3)?,
                action_type: render_action_type(r.get(4)?),
                action_timestamp_us: r.get(5)?,
                sys_status: Some(r.get(6)?),
                app_status: None,
                when_us: r.get(7)?,
            })
        })
        .context("reading integrated rejected ops")?;
    for row in rows {
        out.push(row?);
    }

    let mut stmt = conn
        .prepare(&format!(
            "SELECT {OP_COLUMNS}, o.sys_validation_status, o.app_validation_status, o.when_received
             FROM LimboChainOp o JOIN Action a ON a.hash = o.action_hash
             WHERE o.sys_validation_status = 2 OR o.app_validation_status = 2
             ORDER BY o.when_received"
        ))
        .context("preparing the limbo-rejected query")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(RejectedOp {
                state: OpState::Limbo,
                op_hash: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(0)?, HashKind::DhtOp),
                op_type: render_op_type(r.get(1)?),
                author: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(2)?, HashKind::Agent),
                seq: r.get(3)?,
                action_type: render_action_type(r.get(4)?),
                action_timestamp_us: r.get(5)?,
                sys_status: r.get(6)?,
                app_status: r.get(7)?,
                when_us: r.get(8)?,
            })
        })
        .context("reading limbo rejected ops")?;
    for row in rows {
        out.push(row?);
    }

    Ok(out)
}

/// Every warrant held in a DHT database.
///
/// A warrant may be integrated (`WarrantOp`) or still in limbo, so the join is a
/// LEFT JOIN and an absent status reads as pending rather than as absence.
pub fn warrants(conn: &Connection) -> Result<Vec<WarrantRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT w.hash, w.author, w.warrantee, w.timestamp, w.reason, o.validation_status
             FROM Warrant w LEFT JOIN WarrantOp o ON o.hash = w.hash
             ORDER BY w.timestamp",
        )
        .context("preparing the warrant query")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WarrantRow {
                hash: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(0)?, HashKind::DhtOp),
                author: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(1)?, HashKind::Agent),
                warrantee: fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(2)?, HashKind::Agent),
                timestamp_us: r.get(3)?,
                reason: r.get(4)?,
                validation_status: r.get(5)?,
            })
        })
        .context("reading warrants")?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// The warrant a block cites, looked up by the op hash in
/// `CellBlockReason::InvalidOp`.
///
/// The block lives in `conductor.db` and the warrant in `dht-<dna>.db`, so this
/// is the one join no single connection can make; the caller opens both.
pub fn warrant_by_hash(conn: &Connection, op_hash: &str) -> Result<Option<WarrantRow>> {
    Ok(warrants(conn)?.into_iter().find(|w| w.hash == op_hash))
}

/// Every rejected op in this DNA authored by one agent.
///
/// Given a warrant's `warrantee`, this is "what did the blocked peer write that
/// got them warranted" — the evidence a household reads before it decides to
/// lift. Filtered in Rust over the rendered author so the base64 <-> 36-byte-core
/// conversion stays in exactly one place (`fmt::hash_b64_kind`).
pub fn rejected_authored_by(conn: &Connection, author: &str) -> Result<Vec<RejectedOp>> {
    Ok(list(conn)?
        .into_iter()
        .filter(|o| o.author == author)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The subset of the 0.7.0 DHT schema the rejected queries touch.
    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE Action (
                hash BLOB PRIMARY KEY, author BLOB NOT NULL, seq INTEGER NOT NULL,
                timestamp INTEGER NOT NULL, action_type INTEGER NOT NULL
             ) STRICT, WITHOUT ROWID;
             CREATE TABLE ChainOp (
                hash BLOB PRIMARY KEY, op_type INTEGER NOT NULL, action_hash BLOB NOT NULL,
                validation_status INTEGER NOT NULL, when_integrated INTEGER NOT NULL
             ) STRICT, WITHOUT ROWID;
             CREATE TABLE LimboChainOp (
                hash BLOB PRIMARY KEY, op_type INTEGER NOT NULL, action_hash BLOB NOT NULL,
                sys_validation_status INTEGER, app_validation_status INTEGER,
                when_received INTEGER NOT NULL
             ) STRICT, WITHOUT ROWID;
             CREATE TABLE Warrant (
                hash BLOB PRIMARY KEY, author BLOB NOT NULL, timestamp INTEGER NOT NULL,
                warrantee BLOB NOT NULL, reason TEXT
             ) STRICT, WITHOUT ROWID;
             CREATE TABLE WarrantOp (
                hash BLOB PRIMARY KEY, validation_status INTEGER NOT NULL
             ) STRICT, WITHOUT ROWID;",
        )
        .unwrap();

        let action = vec![1u8; 36];
        let author = vec![2u8; 36];
        conn.execute(
            "INSERT INTO Action VALUES (?1, ?2, 42, 1788000000000000, 4)",
            rusqlite::params![action, author],
        )
        .unwrap();
        // Rejected (integrated) — this is the shape that blocks a cell.
        conn.execute(
            "INSERT INTO ChainOp VALUES (?1, 3, ?2, 2, 1788000001000000)",
            rusqlite::params![vec![3u8; 36], action],
        )
        .unwrap();
        // Accepted — must not be reported.
        conn.execute(
            "INSERT INTO ChainOp VALUES (?1, 1, ?2, 1, 1788000002000000)",
            rusqlite::params![vec![4u8; 36], action],
        )
        .unwrap();
        // Rejected in app validation, still in limbo.
        conn.execute(
            "INSERT INTO LimboChainOp VALUES (?1, 2, ?2, 1, 2, 1788000003000000)",
            rusqlite::params![vec![5u8; 36], action],
        )
        .unwrap();
        // Pending — must not be reported.
        conn.execute(
            "INSERT INTO LimboChainOp VALUES (?1, 2, ?2, NULL, NULL, 1788000004000000)",
            rusqlite::params![vec![6u8; 36], action],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO Warrant VALUES (?1, ?2, 1788000005000000, ?3, 'invalid op')",
            rusqlite::params![vec![7u8; 36], author, vec![8u8; 36]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO WarrantOp VALUES (?1, 1)",
            rusqlite::params![vec![7u8; 36]],
        )
        .unwrap();
        conn
    }

    #[test]
    fn only_rejected_ops_are_reported() {
        let rows = list(&fixture()).unwrap();
        assert_eq!(
            rows.len(),
            2,
            "one integrated, one limbo; accepted and pending excluded"
        );

        let integrated = &rows[0];
        assert_eq!(integrated.state, OpState::Integrated);
        assert_eq!(integrated.op_type, "AgentActivity", "op_type 3");
        assert_eq!(integrated.seq, 42);
        assert_eq!(integrated.action_type, "Create", "action_type 4");
        assert!(
            integrated.author.starts_with("uhCAk"),
            "a 36-byte author column must render with its AgentPubKey prefix: {}",
            integrated.author
        );
        assert!(
            integrated.op_hash.starts_with("uhCQk"),
            "a 36-byte op hash column must render with its DhtOpHash prefix: {}",
            integrated.op_hash
        );
        assert!(integrated.render().contains("sys=REJECTED"));

        let limbo = &rows[1];
        assert_eq!(limbo.state, OpState::Limbo);
        assert_eq!(limbo.sys_status, Some(1));
        assert_eq!(limbo.app_status, Some(2));
        assert!(limbo.render().contains("app=REJECTED"));
    }

    #[test]
    fn warrants_join_their_integration_status() {
        let rows = warrants(&fixture()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].reason.as_deref(), Some("invalid op"));
        assert_eq!(rows[0].validation_status, Some(1));
        assert!(rows[0].render().contains("warrantee:"));
    }

    #[test]
    fn a_block_joins_to_its_warrant_and_the_warrantees_rejected_ops() {
        let conn = fixture();
        let warrant_hash = fmt::hash_b64_kind(&[7u8; 36], HashKind::DhtOp);
        let author = fmt::hash_b64_kind(&[2u8; 36], HashKind::Agent);

        // The hash a block cites resolves to a WARRANT, never to a rejected op.
        let found = warrant_by_hash(&conn, &warrant_hash).unwrap().unwrap();
        assert_eq!(found.hash, warrant_hash);
        assert!(
            !list(&conn)
                .unwrap()
                .iter()
                .any(|o| o.op_hash == warrant_hash),
            "a warrant op hash must not appear among the rejected ops"
        );

        // …and the warrantee's own rejected ops are reachable from it.
        assert_eq!(rejected_authored_by(&conn, &author).unwrap().len(), 2);
        assert!(rejected_authored_by(&conn, "uhCAkNOBODY")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn an_unknown_warrant_hash_resolves_to_nothing() {
        assert!(warrant_by_hash(&fixture(), "uhCQkNOPE").unwrap().is_none());
    }

    #[test]
    fn unknown_discriminants_degrade_rather_than_panic() {
        assert_eq!(render_op_type(99), "<unknown op_type 99>");
        assert_eq!(render_action_type(99), "<unknown action_type 99>");
        assert_eq!(render_status(None), "pending");
        assert_eq!(render_status(Some(2)), "REJECTED");
    }
}
