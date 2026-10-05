//! What a node persists, split by what it is.
//!
//! A volume total cannot say whether a conductor's bytes are notarized heads
//! or installed code, and a blob count cannot say how much disk small files
//! really take. This walks the conductor data directory and the storage
//! directory and folds every file into a small class:
//!
//! - conductor: `code` (`wasm.db`), `dht`, `cache`, `authored`, `peer_meta`,
//!   `conductor`, or the top-level directory name for anything else;
//! - storage: `content_db`, or the top-level entry name (`blobs`, `cache`, …).
//!
//! Each class carries apparent bytes (file length) and allocated bytes
//! (blocks on disk), so filesystem slack and write-ahead logs are visible as
//! their own numbers. The walk is std-only and synchronous: run it from
//! `spawn_blocking`.

use std::collections::BTreeMap;
use std::path::Path;

/// One class of persisted file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreClass {
    pub store: String,
    /// `main`, `wal` or `shm` for a database; `file` otherwise.
    pub part: &'static str,
    /// Short DNA hash for a per-DNA conductor database; empty otherwise.
    pub dna: String,
}

/// Totals for one class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoreTotals {
    pub apparent_bytes: u64,
    pub allocated_bytes: u64,
    pub files: u64,
}

const DNA_LABEL_LEN: usize = 8;
const STORE_LABEL_MAX: usize = 24;

fn db_part(file_name: &str) -> (&str, &'static str) {
    if let Some(base) = file_name.strip_suffix("-wal") {
        (base, "wal")
    } else if let Some(base) = file_name.strip_suffix("-shm") {
        (base, "shm")
    } else {
        (file_name, "main")
    }
}

fn label(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(STORE_LABEL_MAX)
        .collect();
    if cleaned.is_empty() {
        "other".to_string()
    } else {
        cleaned
    }
}

/// `dht-uhC0kUQPSeKEn….db` → `UQPSeKEn`. The `uhC0k` prefix is the same on
/// every DNA hash, so the label starts after it.
fn dna_label(hash: &str) -> String {
    hash.strip_prefix("uhC0k")
        .unwrap_or(hash)
        .chars()
        .take(DNA_LABEL_LEN)
        .collect()
}

/// Class of a file under the conductor data directory, by its path relative
/// to that directory.
pub fn classify_conductor(rel: &Path) -> StoreClass {
    let mut parts = rel.iter().map(|p| p.to_string_lossy());
    let first = parts.next().unwrap_or_default();
    let file_name = rel
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (base, part) = db_part(&file_name);
    let Some(stem) = base.strip_suffix(".db") else {
        return StoreClass {
            store: label(&first),
            part: "file",
            dna: String::new(),
        };
    };
    if stem == "wasm" {
        return StoreClass {
            store: "code".to_string(),
            part,
            dna: String::new(),
        };
    }
    for (prefix, store) in [
        ("dht-", "dht"),
        ("cache-", "cache"),
        ("authored-", "authored"),
        ("p2p-peer-meta-", "peer_meta"),
        ("p2p-", "peer_meta"),
    ] {
        if let Some(hash) = stem.strip_prefix(prefix) {
            return StoreClass {
                store: store.to_string(),
                part,
                dna: dna_label(hash),
            };
        }
    }
    StoreClass {
        store: label(stem),
        part,
        dna: String::new(),
    }
}

/// Class of a file under the storage directory, by its relative path.
pub fn classify_storage(rel: &Path) -> StoreClass {
    let first = rel
        .iter()
        .next()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (base, part) = db_part(&first);
    if let Some(stem) = base.strip_suffix(".db") {
        return StoreClass {
            store: format!("{}_db", label(stem)),
            part,
            dna: String::new(),
        };
    }
    StoreClass {
        store: label(&first),
        part: "file",
        dna: String::new(),
    }
}

#[cfg(unix)]
fn allocated_bytes(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn allocated_bytes(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}

/// Fold every regular file under `root` into its class. Symlinks are not
/// followed. A directory that cannot be read is skipped, so a missing root
/// yields an empty map.
pub fn walk(root: &Path, classify: fn(&Path) -> StoreClass) -> BTreeMap<StoreClass, StoreTotals> {
    let mut out: BTreeMap<StoreClass, StoreTotals> = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file() {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let totals = out.entry(classify(rel)).or_default();
                totals.apparent_bytes += meta.len();
                totals.allocated_bytes += allocated_bytes(&meta);
                totals.files += 1;
            }
        }
    }
    out
}

/// Bucket labels for [`fold_known_holders`]: a shard known to be held by
/// exactly 1–4 peers, or by five or more.
pub const HOLDER_BUCKETS: [&str; 5] = ["1", "2", "3", "4", "5+"];

/// Fold per-shard known-holder counts into [`HOLDER_BUCKETS`].
pub fn fold_known_holders(per_shard: impl IntoIterator<Item = i64>) -> [u64; 5] {
    let mut out = [0u64; 5];
    for n in per_shard {
        if n >= 1 {
            out[(n.min(5) - 1) as usize] += 1;
        }
    }
    out
}

/// How many peers this node KNOWS hold each shard, from `shard_locations`
/// (its own holdings plus what peers announced). This is the node's
/// knowledge, not the network's truth: a shard held by five peers that never
/// announced reads as one holder here.
pub fn known_holder_distribution(
    conn: &mut diesel::SqliteConnection,
) -> Result<[u64; 5], diesel::result::Error> {
    use crate::db::diesel_schema::shard_locations::dsl as sl;
    use diesel::prelude::*;
    // (shard_hash, peer_id) is the primary key, so rows per shard are distinct peers.
    let counts: Vec<i64> = sl::shard_locations
        .group_by(sl::shard_hash)
        .select(diesel::dsl::count_star())
        .load(conn)?;
    Ok(fold_known_holders(counts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn conductor(p: &str) -> (String, &'static str, String) {
        let c = classify_conductor(&PathBuf::from(p));
        (c.store, c.part, c.dna)
    }

    #[test]
    fn the_wasm_database_and_its_log_are_code() {
        assert_eq!(
            conductor("databases/wasm.db"),
            ("code".into(), "main", "".into())
        );
        assert_eq!(
            conductor("databases/wasm.db-wal"),
            ("code".into(), "wal", "".into())
        );
        assert_eq!(
            conductor("databases/wasm.db-shm"),
            ("code".into(), "shm", "".into())
        );
    }

    #[test]
    fn a_dht_database_carries_its_dna() {
        assert_eq!(
            conductor("databases/dht-uhC0kUQPSeKEnh5_dZhp56oFNgj.db-wal"),
            ("dht".into(), "wal", "UQPSeKEn".into())
        );
        assert_eq!(
            conductor("databases/p2p-peer-meta-uhC0kzu8-dTs0qT52.db"),
            ("peer_meta".into(), "main", "zu8-dTs0".into())
        );
    }

    #[test]
    fn other_conductor_files_take_their_top_level_name() {
        assert_eq!(
            conductor("databases/conductor.db"),
            ("conductor".into(), "main", "".into())
        );
        assert_eq!(conductor("ks/store_file"), ("ks".into(), "file", "".into()));
        assert_eq!(
            conductor("wasm-cache/ab/cd"),
            ("wasm_cache".into(), "file", "".into())
        );
    }

    #[test]
    fn storage_files_fold_by_top_level_entry() {
        let s = |p: &str| {
            let c = classify_storage(&PathBuf::from(p));
            (c.store, c.part)
        };
        assert_eq!(s("content.db"), ("content_db".into(), "main"));
        assert_eq!(s("content.db-wal"), ("content_db".into(), "wal"));
        assert_eq!(s("blobs/blobs/9212/sha256-9212"), ("blobs".into(), "file"));
        assert_eq!(s("sync.sled/db"), ("sync_sled".into(), "file"));
    }

    #[test]
    fn holder_counts_fold_into_five_buckets() {
        assert_eq!(fold_known_holders([1, 1, 2, 5, 7, 0]), [2, 1, 0, 0, 2]);
    }

    #[test]
    fn the_walk_sums_bytes_and_files_per_class() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dbs = dir.path().join("databases");
        std::fs::create_dir_all(&dbs).unwrap();
        std::fs::write(dbs.join("wasm.db"), vec![0u8; 1000]).unwrap();
        std::fs::write(dbs.join("dht-uhC0kAAAAAAAAAA.db"), vec![0u8; 300]).unwrap();
        std::fs::write(dbs.join("dht-uhC0kAAAAAAAAAA.db-wal"), vec![0u8; 50]).unwrap();
        let got = walk(dir.path(), classify_conductor);
        let code = got
            .iter()
            .find(|(k, _)| k.store == "code")
            .map(|(_, v)| *v)
            .expect("code class");
        assert_eq!((code.apparent_bytes, code.files), (1000, 1));
        let dht: u64 = got
            .iter()
            .filter(|(k, _)| k.store == "dht")
            .map(|(_, v)| v.apparent_bytes)
            .sum();
        assert_eq!(dht, 350);
        assert!(walk(&dir.path().join("absent"), classify_conductor).is_empty());
    }
}
