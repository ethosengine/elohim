//! Which agent this conductor *is*.
//!
//! A `BlockSpan` row names the cell being refused, not the conductor doing the
//! refusing, so a list of blocks read on its own cannot distinguish "I refuse to
//! talk to X" from "X is me". `InstalledApp.agent_pub_key` is the conductor's own
//! side of that sentence, and `AppRole` maps each role to the DNA whose
//! `dht-<hash>.db` holds the rejected op.
//!
//! Schema (`holochain_data-0.7.0/migrations/conductor/…initial_schema.up.sql`):
//!
//! ```sql
//! CREATE TABLE InstalledApp (app_id TEXT PRIMARY KEY, agent_pub_key BLOB NOT NULL,
//!                            status TEXT NOT NULL, …);
//! CREATE TABLE AppRole (app_id TEXT, role_name TEXT, dna_hash BLOB NOT NULL, …);
//! ```

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::fmt::{self, HashKind};

/// One installed app: this conductor's identity for that app, and its roles.
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub app_id: String,
    pub agent_pub_key: String,
    pub status: String,
    /// `(role_name, dna_hash)`, in role-name order.
    pub roles: Vec<(String, String)>,
}

impl InstalledApp {
    pub fn render(&self) -> String {
        let mut s = format!(
            "  {app_id}  [{status}]\n        agent: {agent}",
            app_id = self.app_id,
            status = self.status,
            agent = self.agent_pub_key
        );
        if self.roles.is_empty() {
            // AppRole can legitimately be empty: the authoritative role list is
            // the `role_assignments_blob` on InstalledApp, and this table is the
            // denormalized index. Say so rather than implying the app has no
            // roles.
            s.push_str("\n        roles: (AppRole index empty; see role_assignments_blob)");
        }
        for (role, dna) in &self.roles {
            s.push_str(&format!("\n        role {role}: {dna}"));
        }
        s
    }
}

/// Every installed app in a conductor database.
pub fn list(conn: &Connection) -> Result<Vec<InstalledApp>> {
    let mut stmt = conn
        .prepare("SELECT app_id, agent_pub_key, status FROM InstalledApp ORDER BY app_id")
        .context("preparing the InstalledApp query")?;
    let heads = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(1)?, HashKind::Agent),
                r.get::<_, String>(2)?,
            ))
        })
        .context("reading InstalledApp")?
        .collect::<Result<Vec<_>, _>>()?;

    let mut role_stmt = conn
        .prepare("SELECT role_name, dna_hash FROM AppRole WHERE app_id = ?1 ORDER BY role_name")
        .context("preparing the AppRole query")?;

    let mut out = Vec::with_capacity(heads.len());
    for (app_id, agent_pub_key, status) in heads {
        let roles = role_stmt
            .query_map([&app_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    fmt::hash_b64_kind(&r.get::<_, Vec<u8>>(1)?, HashKind::Dna),
                ))
            })
            .context("reading AppRole")?
            .collect::<Result<Vec<_>, _>>()?;
        out.push(InstalledApp {
            app_id,
            agent_pub_key,
            status,
            roles,
        });
    }
    Ok(out)
}

/// This conductor's own agent keys, deduplicated.
pub fn own_agent_keys(conn: &Connection) -> Result<Vec<String>> {
    let mut keys: Vec<String> = list(conn)?.into_iter().map(|a| a.agent_pub_key).collect();
    keys.sort();
    keys.dedup();
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE InstalledApp (
                app_id TEXT PRIMARY KEY, agent_pub_key BLOB NOT NULL, status TEXT NOT NULL
             ) STRICT;
             CREATE TABLE AppRole (
                app_id TEXT NOT NULL, role_name TEXT NOT NULL, dna_hash BLOB NOT NULL,
                PRIMARY KEY (app_id, role_name)
             ) STRICT;",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO InstalledApp VALUES ('elohim', ?1, 'Running')",
            rusqlite::params![vec![9u8; 36]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO AppRole VALUES ('elohim', 'node-registry', ?1)",
            rusqlite::params![vec![8u8; 36]],
        )
        .unwrap();
        conn
    }

    #[test]
    fn apps_carry_the_conductors_own_agent_key_and_roles() {
        let apps = list(&fixture()).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_id, "elohim");
        assert!(apps[0].agent_pub_key.starts_with("uhCAk"));
        assert_eq!(apps[0].roles.len(), 1);
        assert_eq!(apps[0].roles[0].0, "node-registry");
        assert!(apps[0].roles[0].1.starts_with("uhC0k"));
        assert!(apps[0].render().contains("role node-registry:"));
        assert!(!apps[0].render().contains("AppRole index empty"));
    }

    #[test]
    fn an_empty_role_index_is_named_not_implied_absent() {
        let conn = fixture();
        conn.execute("DELETE FROM AppRole", []).unwrap();
        let apps = list(&conn).unwrap();
        assert!(apps[0].render().contains("AppRole index empty"));
    }

    #[test]
    fn own_agent_keys_are_deduplicated() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO InstalledApp VALUES ('elohim-side', ?1, 'Running')",
            rusqlite::params![vec![9u8; 36]],
        )
        .unwrap();
        assert_eq!(own_agent_keys(&conn).unwrap().len(), 1);
    }
}
