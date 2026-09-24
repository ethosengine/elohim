//! The fold store, schema v3: units keyed by an opaque `unit_id`, chunks with an optional vector,
//! and an external-content FTS5 table over LIVE chunks only. Demotion, never deletion.
use std::collections::BTreeMap;
use std::path::Path;

use elohim_epr_index::store::{ChunkRow, Store, UnitChunks, SCHEMA_VERSION};
use elohim_epr_index::IndexError;
use rusqlite::Connection;

fn meta() -> BTreeMap<String, String> {
    [("schema", SCHEMA_VERSION), ("measure", "bafy-measure")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn unit(unit_id: &str, fingerprint: &str, texts: &[&str]) -> UnitChunks {
    UnitChunks {
        unit_id: unit_id.to_string(),
        fingerprint: fingerprint.to_string(),
        stat: None,
        skipped: None,
        chunks: texts
            .iter()
            .enumerate()
            .map(|(i, text)| ChunkRow {
                section: format!("section {i}"),
                text: text.to_string(),
                vector: None,
            })
            .collect(),
    }
}

fn nothing(_: &rusqlite::Transaction<'_>) -> elohim_epr_index::Result<()> {
    Ok(())
}

fn created(dir: &Path) -> (Store, std::path::PathBuf) {
    let path = dir.join("fold.sqlite");
    (Store::create(&path, &meta()).expect("created"), path)
}

fn scalar(path: &Path, sql: &str) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

fn matches(store: &Store, expression: &str) -> Vec<(i64, String, f64)> {
    let mut hits = Vec::new();
    store
        .lexical(expression, |id, unit, rank| {
            hits.push((id, unit.to_string(), rank))
        })
        .expect("lexical reads");
    hits
}

#[test]
fn schema_creates_fts_and_triggers() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = created(dir.path());
    assert_eq!(store.meta().unwrap(), meta());
    let conn = Connection::open(&path).unwrap();
    let named = |kind: &str| -> Vec<String> {
        conn.prepare("SELECT name FROM sqlite_master WHERE type = ?1 ORDER BY name")
            .unwrap()
            .query_map([kind], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let tables = named("table");
    for table in ["chunks", "chunks_fts", "meta", "units"] {
        assert!(tables.contains(&table.to_string()), "{tables:?}");
    }
    assert!(named("index").contains(&"chunks_by_unit".to_string()));
    assert_eq!(
        named("trigger"),
        [
            "chunks_demoted_once",
            "chunks_fts_demote",
            "chunks_fts_insert",
            "chunks_identity_fixed",
            "chunks_never_deleted",
            "units_never_deleted",
        ]
    );
    let columns: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('units')")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        columns,
        [
            "unit_id",
            "fingerprint",
            "stat",
            "chunks",
            "skipped",
            "folded_at",
            "demoted_at"
        ]
    );
    // A fresh store opens readable and passes its integrity check.
    assert!(Store::open_readable(&path, true, true).unwrap().is_some());
    assert!(
        Store::open_readable(&dir.path().join("absent.sqlite"), true, true)
            .unwrap()
            .is_none()
    );
}

#[test]
fn demoted_chunk_leaves_fts() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, path) = created(dir.path());
    store
        .fold_txn(
            1,
            &[],
            &[unit("a", "f1", &["an orchard, planted"])],
            nothing,
        )
        .unwrap();
    assert_eq!(matches(&store, "\"orchard\"").len(), 1);
    store.fold_txn(2, &["a".to_string()], &[], nothing).unwrap();
    assert!(
        matches(&store, "\"orchard\"").is_empty(),
        "the demoted text left the index"
    );
    assert_eq!(
        scalar(&path, "SELECT count(*) FROM chunks WHERE demoted_at = 2"),
        1,
        "the demoted row itself remains"
    );
    assert_eq!(
        scalar(
            &path,
            "SELECT count(*) FROM units WHERE unit_id = 'a' AND demoted_at = 2"
        ),
        1
    );
    assert!(store.live_units().unwrap().is_empty());
    // A returning unit folds new rows; the demoted one stays demoted.
    store
        .fold_txn(
            3,
            &[],
            &[unit("a", "f1", &["an orchard, planted"])],
            nothing,
        )
        .unwrap();
    assert_eq!(matches(&store, "\"orchard\"").len(), 1);
    assert_eq!(scalar(&path, "SELECT count(*) FROM chunks"), 2);
    assert_eq!(store.live_units().unwrap()["a"].fingerprint, "f1");
}

#[test]
fn demote_is_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, path) = created(dir.path());
    store
        .fold_txn(1, &[], &[unit("a", "f1", &["text"])], nothing)
        .unwrap();
    store.fold_txn(2, &["a".to_string()], &[], nothing).unwrap();
    let conn = Connection::open(&path).unwrap();
    let refused = conn
        .execute(
            "UPDATE chunks SET demoted_at = 9 WHERE demoted_at IS NOT NULL",
            [],
        )
        .unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("a demoted chunk stays demoted"),
        "{refused}"
    );
    assert!(conn
        .execute("UPDATE chunks SET demoted_at = NULL", [])
        .is_err());
}

#[test]
fn identity_fixed() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, path) = created(dir.path());
    store
        .fold_txn(1, &[], &[unit("a", "f1", &["text"])], nothing)
        .unwrap();
    let conn = Connection::open(&path).unwrap();
    for column in ["text", "unit_id", "section"] {
        let refused = conn
            .execute(&format!("UPDATE chunks SET {column} = 'moved'"), [])
            .unwrap_err();
        assert!(
            refused.to_string().contains("are fixed"),
            "{column}: {refused}"
        );
    }
}

#[test]
fn never_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, path) = created(dir.path());
    store
        .fold_txn(1, &[], &[unit("a", "f1", &["text"])], nothing)
        .unwrap();
    let conn = Connection::open(&path).unwrap();
    for table in ["chunks", "units"] {
        let refused = conn
            .execute(&format!("DELETE FROM {table}"), [])
            .unwrap_err();
        assert!(
            refused.to_string().contains("demotion, never deletion"),
            "{refused}"
        );
    }
}

#[test]
fn fold_txn_is_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, path) = created(dir.path());
    let failed = store.fold_txn(
        1,
        &[],
        &[
            unit("a", "f1", &["one", "two"]),
            unit("b", "f2", &["three"]),
        ],
        |tx| {
            tx.execute("UPDATE meta SET value = 'moved' WHERE key = 'measure'", [])?;
            Err(IndexError::Refused("the caller's step refused".into()))
        },
    );
    assert!(matches!(failed, Err(IndexError::Refused(_))));
    assert_eq!(scalar(&path, "SELECT count(*) FROM chunks"), 0);
    assert_eq!(scalar(&path, "SELECT count(*) FROM units"), 0);
    assert_eq!(store.meta().unwrap(), meta(), "nothing of the run landed");
    assert!(matches(&store, "\"three\"").is_empty());

    // The same run, committed: every unit, every chunk, the caller's step — together.
    store
        .fold_txn(
            1,
            &[],
            &[
                unit("a", "f1", &["one", "two"]),
                unit("b", "f2", &["three"]),
            ],
            |tx| {
                tx.execute("UPDATE meta SET value = 'moved' WHERE key = 'measure'", [])?;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(scalar(&path, "SELECT count(*) FROM chunks"), 3);
    assert_eq!(store.meta().unwrap()["measure"], "moved");
    let tally = store.tally().unwrap();
    assert_eq!((tally.chunks, tally.demoted, tally.bytes), (3, 0, 11));
    assert_eq!(store.shard().unwrap().atoms, 3);
}

#[test]
fn lexical_orders_by_bm25() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = created(dir.path());
    store
        .fold_txn(
            1,
            &[],
            &[
                unit(
                    "sparse",
                    "f1",
                    &["a long passage about many things and one orchard among them all"],
                ),
                unit("dense", "f2", &["orchard orchard orchard"]),
                unit("none", "f3", &["nothing to see"]),
            ],
            nothing,
        )
        .unwrap();
    let mut hits = matches(&store, "\"orchard\"*");
    assert_eq!(hits.len(), 2, "only matching live chunks are visited");
    // bm25() is lower-is-better: the dense unit ranks first.
    hits.sort_by(|a, b| a.2.total_cmp(&b.2));
    let order: Vec<&str> = hits.iter().map(|h| h.1.as_str()).collect();
    assert_eq!(order, ["dense", "sparse"]);
    assert!(hits[0].2 < hits[1].2);
    let (section, text) = store.chunk(hits[0].0).unwrap();
    assert_eq!(
        (section.as_str(), text.as_str()),
        ("section 0", "orchard orchard orchard")
    );
}

#[test]
fn scan_visits_live_chunks_with_their_optional_vector() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = created(dir.path());
    let mut with_vector = unit("v", "f1", &["vectored"]);
    with_vector.chunks[0].vector = Some(vec![1, 2, 3, 4]);
    store
        .fold_txn(1, &[], &[with_vector, unit("n", "f2", &["bare"])], nothing)
        .unwrap();
    let mut seen = Vec::new();
    let visited = store
        .scan(|_, unit, vector| seen.push((unit.to_string(), vector.map(<[u8]>::to_vec))))
        .unwrap();
    assert_eq!(visited, 2);
    seen.sort();
    assert_eq!(
        seen,
        [
            ("n".to_string(), None),
            ("v".to_string(), Some(vec![1, 2, 3, 4]))
        ]
    );
}
