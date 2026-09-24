//! FTS5 runtime probe — the go/no-go for the peer-side lexical fold.
//!
//! elohim-storage links exactly one `libsqlite3-sys`: lair's bundled SQLCipher
//! build, compiled with `-DSQLITE_ENABLE_FTS5`. Diesel's sqlite backend and
//! rusqlite both resolve to that one library (`links = "sqlite3"` permits only
//! one). These tests prove FTS5 is not merely compiled in but executes at
//! runtime through both front doors, and that `bm25()` ranking is available —
//! the lexical fold stands on all three.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_types::Text;

#[derive(QueryableByName)]
struct Hit {
    #[diesel(sql_type = Text)]
    t: String,
}

#[test]
fn fts5_is_compiled_into_the_linked_sqlite_via_diesel() {
    let mut conn = SqliteConnection::establish(":memory:").expect("open :memory: via diesel");
    diesel::sql_query("CREATE VIRTUAL TABLE p USING fts5(t)")
        .execute(&mut conn)
        .expect("CREATE VIRTUAL TABLE ... USING fts5 must succeed in the linked SQLite");
    conn.batch_execute("INSERT INTO p(t) VALUES ('stewardship of the commons')")
        .expect("insert one row into the fts5 table");

    let hits: Vec<Hit> = diesel::sql_query("SELECT t FROM p WHERE p MATCH ?")
        .bind::<Text, _>("commons")
        .load(&mut conn)
        .expect("MATCH query over fts5");

    assert_eq!(
        hits.iter().map(|h| h.t.as_str()).collect::<Vec<_>>(),
        vec!["stewardship of the commons"]
    );
}

#[test]
fn fts5_is_reachable_through_rusqlite_on_the_same_libsqlite3() {
    let conn = rusqlite::Connection::open_in_memory().expect("open :memory: via rusqlite");
    conn.execute_batch(
        "CREATE VIRTUAL TABLE p USING fts5(t);
         INSERT INTO p(t) VALUES ('commons');
         INSERT INTO p(t) VALUES ('the commons of the commons are commons');
         INSERT INTO p(t) VALUES ('unrelated row');",
    )
    .expect("fts5 table + rows through rusqlite");

    let mut stmt = conn
        .prepare("SELECT t FROM p WHERE p MATCH ?1 ORDER BY bm25(p)")
        .expect("prepare bm25-ordered MATCH");
    let rows: Vec<String> = stmt
        .query_map(["commons"], |r| r.get(0))
        .expect("run MATCH")
        .collect::<Result<_, _>>()
        .expect("collect rows");

    assert_eq!(rows.len(), 2, "only the two matching rows, got {rows:?}");
    assert!(!rows.contains(&"unrelated row".to_string()));
}

#[test]
fn bm25_rank_is_available() {
    let conn = rusqlite::Connection::open_in_memory().expect("open :memory: via rusqlite");
    conn.execute_batch(
        "CREATE VIRTUAL TABLE p USING fts5(title, body);
         INSERT INTO p(title, body) VALUES ('garden', 'a note about soil');
         INSERT INTO p(title, body) VALUES ('garden garden', 'garden beds and garden paths');",
    )
    .expect("fts5 table with two columns");

    let mut stmt = conn
        .prepare("SELECT title, bm25(p) FROM p WHERE p MATCH 'garden' ORDER BY bm25(p)")
        .expect("prepare bm25 select");
    let ranked: Vec<(String, f64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("run bm25 select")
        .collect::<Result<_, _>>()
        .expect("collect ranked rows");

    assert_eq!(ranked.len(), 2);
    // bm25() is negative-is-better: the denser match ranks first with a lower score.
    assert_eq!(ranked[0].0, "garden garden");
    assert!(
        ranked[0].1 < ranked[1].1,
        "bm25 must order the denser match first: {ranked:?}"
    );
    assert!(ranked.iter().all(|(_, s)| s.is_finite() && *s < 0.0));

    // The `rank` hidden column is bm25 by default and must agree with the explicit call.
    let rank_order: Vec<String> = conn
        .prepare("SELECT title FROM p WHERE p MATCH 'garden' ORDER BY rank")
        .expect("prepare rank select")
        .query_map([], |r| r.get(0))
        .expect("run rank select")
        .collect::<Result<_, _>>()
        .expect("collect rank rows");
    assert_eq!(
        rank_order,
        ranked.into_iter().map(|(t, _)| t).collect::<Vec<_>>()
    );
}
