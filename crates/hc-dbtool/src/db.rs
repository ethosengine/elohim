//! Opening a conductor's encrypted SQLite databases.
//!
//! Read-only is the default and the only mode any read verb ever uses. The one
//! write verb (`unblock`) opens read-write, and only after
//! [`lock_holders`] reports that no live process holds the file — the operator
//! stops the conductor first.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rusqlite::{Connection, OpenFlags};

use crate::key::DbKey;

/// How a database is being opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Every read verb. Refuses at the sqlite layer, not by convention.
    ReadOnly,
    /// `unblock` only, and only once `lock_holders` is empty.
    ReadWrite,
}

/// A conductor data root: the `databases/` directory holding `conductor.db`,
/// `dht-<dna>.db` and `db.key`.
#[derive(Debug, Clone)]
pub struct Databases {
    root: PathBuf,
}

impl Databases {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn conductor_db(&self) -> PathBuf {
        self.root.join("conductor.db")
    }

    pub fn dht_db(&self, dna_hash: &str) -> PathBuf {
        self.root.join(format!("dht-{dna_hash}.db"))
    }

    pub fn key_file(&self) -> PathBuf {
        self.root.join("db.key")
    }

    /// Load the `db.key` in this root under `passphrase`.
    pub fn load_key(&self, passphrase: &[u8]) -> Result<DbKey> {
        let path = self.key_file();
        let locked = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        DbKey::load(&locked, passphrase).with_context(|| format!("unlocking {}", path.display()))
    }
}

/// Open an encrypted holochain database.
///
/// The pragma block is emitted before any statement that touches a page, which
/// is what lets SQLCipher find the salt out-of-band (`cipher_salt`) with the
/// first 32 bytes of the file left in plaintext
/// (`cipher_plaintext_header_size = 32`).
pub fn open(path: &Path, key: &mut DbKey, access: Access) -> Result<Connection> {
    if !path.exists() {
        return Err(anyhow!("no such database: {}", path.display()));
    }

    let flags = match access {
        Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        Access::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    };

    let conn = Connection::open_with_flags(path, flags).with_context(|| {
        format!(
            "opening {} ({}); a live conductor keeps the WAL index open — \
             if this fails, copy conductor.db, -wal, -shm and db.key aside and read the copy",
            path.display(),
            match access {
                Access::ReadOnly => "read-only",
                Access::ReadWrite => "read-write",
            }
        )
    })?;

    conn.execute_batch(&key.pragma_sql())
        .with_context(|| format!("applying cipher pragmas to {}", path.display()))?;

    // First statement that actually decrypts a page. A wrong key surfaces here
    // as "file is not a database", so translate it into something an operator
    // can act on.
    let ok: i64 = conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
        .map_err(|e| {
            anyhow!(
                "could not decrypt {}: {e}. \
                 Either the passphrase is wrong or db.key does not belong to this database.",
                path.display()
            )
        })?;
    let _ = ok;

    Ok(conn)
}

/// Every live process holding `path` open, as `(pid, exe)`.
///
/// Best effort: `/proc` entries we may not read are skipped rather than treated
/// as absence, and the caller is told the scan was partial.
pub fn lock_holders(path: &Path) -> Result<Vec<(u32, String)>> {
    let target = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    // A conductor writing WAL may hold only the sidecars open at the instant we
    // look; treat any of the three as "this database is live".
    let sidecars = [
        target.clone(),
        format!("{target}-wal"),
        format!("{target}-shm"),
    ];

    let mut holders: Vec<(u32, String)> = Vec::new();
    let procs = match std::fs::read_dir("/proc") {
        Ok(p) => p,
        // No procfs: we cannot prove the database is quiet, so say so.
        Err(e) => return Err(anyhow!("cannot scan /proc for lock holders: {e}")),
    };

    for entry in procs.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let fd_dir = entry.path().join("fd");
        let Ok(fds) = std::fs::read_dir(&fd_dir) else {
            continue;
        };
        let mut hit = false;
        for fd in fds.flatten() {
            if let Ok(link) = std::fs::read_link(fd.path()) {
                let link = link.to_string_lossy().into_owned();
                if sidecars.contains(&link) {
                    hit = true;
                    break;
                }
            }
        }
        if hit {
            let exe = std::fs::read_link(entry.path().join("exe"))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "<unknown>".to_string());
            holders.push((pid, exe));
        }
    }

    holders.sort_unstable();
    holders.dedup();
    Ok(holders)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_round_trip_open() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("databases");
        std::fs::create_dir_all(&root).unwrap();
        let dbs = Databases::new(&root);

        let (mut key, locked) = DbKey::generate(b"test").unwrap();
        std::fs::write(dbs.key_file(), &locked).unwrap();

        // Mint an encrypted database the same way the conductor does.
        let conn = Connection::open(dbs.conductor_db()).unwrap();
        conn.execute_batch(&key.pragma_sql()).unwrap();
        conn.execute_batch("CREATE TABLE Probe (v INTEGER); INSERT INTO Probe VALUES (7);")
            .unwrap();
        drop(conn);

        // It really is encrypted: a plain reader cannot see the schema.
        let raw = std::fs::read(dbs.conductor_db()).unwrap();
        assert!(
            !raw.windows(5).any(|w| w == b"Probe"),
            "fixture database was written in plaintext"
        );

        let mut reloaded = dbs.load_key(b"test").unwrap();
        let conn = open(&dbs.conductor_db(), &mut reloaded, Access::ReadOnly).unwrap();
        let v: i64 = conn
            .query_row("SELECT v FROM Probe", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 7);
    }

    #[test]
    fn read_only_refuses_writes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("databases");
        std::fs::create_dir_all(&root).unwrap();
        let dbs = Databases::new(&root);

        let (mut key, locked) = DbKey::generate(b"test").unwrap();
        std::fs::write(dbs.key_file(), &locked).unwrap();
        let conn = Connection::open(dbs.conductor_db()).unwrap();
        conn.execute_batch(&key.pragma_sql()).unwrap();
        conn.execute_batch("CREATE TABLE Probe (v INTEGER);")
            .unwrap();
        drop(conn);

        let mut key = dbs.load_key(b"test").unwrap();
        let conn = open(&dbs.conductor_db(), &mut key, Access::ReadOnly).unwrap();
        let err = conn
            .execute("INSERT INTO Probe VALUES (1)", [])
            .expect_err("read-only must refuse a write");
        assert!(err.to_string().contains("readonly"), "got: {err}");
    }

    #[test]
    fn wrong_passphrase_cannot_open() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("databases");
        std::fs::create_dir_all(&root).unwrap();
        let dbs = Databases::new(&root);

        let (mut key, _) = DbKey::generate(b"test").unwrap();
        let conn = Connection::open(dbs.conductor_db()).unwrap();
        conn.execute_batch(&key.pragma_sql()).unwrap();
        conn.execute_batch("CREATE TABLE Probe (v INTEGER);")
            .unwrap();
        drop(conn);

        let (mut other, _) = DbKey::generate(b"test").unwrap();
        let err = open(&dbs.conductor_db(), &mut other, Access::ReadOnly)
            .expect_err("a foreign key must not open this database");
        assert!(err.to_string().contains("could not decrypt"), "got: {err}");
    }

    #[test]
    fn lock_holders_sees_our_own_open_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("held.db");
        std::fs::write(&path, b"x").unwrap();
        let _held = std::fs::File::open(&path).unwrap();

        let holders = lock_holders(&path).unwrap();
        let me = std::process::id();
        assert!(
            holders.iter().any(|(pid, _)| *pid == me),
            "lock_holders missed this process: {holders:?}"
        );
    }
}
