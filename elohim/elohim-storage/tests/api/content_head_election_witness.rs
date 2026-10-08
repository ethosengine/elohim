//! The head answers carry the election that chose them (2026-10-08: two
//! doorways answered `lamad-spa` with two CIDs and nothing on the wire said
//! which election either had read).
//!
//! Over the REAL server:
//!
//! - `GET /db/content/{id}/head` carries `canonicalDeclaredAt` /
//!   `canonicalLinkHash` straight from the row's election columns, and `null`
//!   for a row with no election recorded;
//! - `GET /epr-head/{id}` carries the same clock, tier and tiebreak as the
//!   unaddressed `election` witness, and omits the key when none is recorded.

use std::sync::Arc;

use diesel::prelude::*;
use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::diesel_schema::content;
use elohim_storage::db::AppContext;
use elohim_storage::http::HttpServer;
use elohim_storage::services::Services;
use elohim_storage::test_util::test_pool;
use serde_json::Value;

const HEAD: &str = "uhCkkc4f18wPJTfxqSdzRUdXJjLD5DRRn7YbF_yHh0xyRENnbCK-A";
const LINK: &str = "uhCkkElectedDeclarationLink0123456789012345678901234567";
const ELECTED: &str = "election-witness-elected";
const UNELECTED: &str = "election-witness-unelected";
// 2026-10-08T14:00:00.123456Z in microseconds since the Unix epoch.
const CLOCK: i64 = 1_791_468_000_123_456;

fn row(conn: &mut SqliteConnection, id: &str, election: Option<(i64, i32, &str)>) {
    create_content(
        conn,
        &AppContext::default_lamad(),
        CreateContentInput {
            id: id.into(),
            title: id.into(),
            description: None,
            content_type: "concept".into(),
            content_format: "markdown".into(),
            blob_hash: None,
            blob_cid: Some("bafkreiwitness0000000000000000000000000000000000000001".into()),
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".into(),
            created_by: None,
            tags: vec![],
            content_body: Some("body".into()),
            dht_anchor_hash: Some(HEAD.into()),
        },
    )
    .unwrap();
    diesel::update(content::table.filter(content::id.eq(id)))
        .set((
            content::declared_head_action_hash.eq(Some(HEAD)),
            content::canonical_declared_at.eq(election.map(|e| e.0)),
            content::canonical_earned.eq(election.map(|e| e.1)),
            content::canonical_link_hash.eq(election.map(|e| e.2)),
        ))
        .execute(conn)
        .unwrap();
}

async fn serve() -> String {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        row(&mut conn, ELECTED, Some((CLOCK, 1, LINK)));
        row(&mut conn, UNELECTED, None);
    }
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool.clone())
        .with_services(Arc::new(Services::new(pool)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(server).serve(listener));
    base
}

async fn get(base: &str, path: &str) -> (u16, Value) {
    let resp = reqwest::get(format!("{base}{path}"))
        .await
        .expect("request");
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

#[tokio::test]
async fn content_head_carries_the_election_clock_and_tiebreak_from_the_columns() {
    let base = serve().await;
    let (status, body) = get(&base, &format!("/db/content/{ELECTED}/head")).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["canonicalDeclaredAt"], CLOCK, "{body}");
    assert_eq!(body["canonicalLinkHash"], LINK, "{body}");
    assert_eq!(body["earned"], true, "{body}");

    let (status, body) = get(&base, &format!("/db/content/{UNELECTED}/head")).await;
    assert_eq!(status, 200, "{body}");
    assert!(body["canonicalDeclaredAt"].is_null(), "{body}");
    assert!(body["canonicalLinkHash"].is_null(), "{body}");
}

#[tokio::test]
async fn epr_head_carries_the_election_witness_beside_an_unmoved_cid() {
    let base = serve().await;
    let (status, elected) = get(&base, &format!("/epr-head/{ELECTED}")).await;
    assert_eq!(status, 200, "{elected}");
    assert_eq!(
        elected["election"],
        serde_json::json!({
            "canonicalDeclaredAt": "2026-10-08T14:00:00.123456Z",
            "earned": true,
            "linkHash": LINK,
        }),
        "{elected}"
    );

    let (status, unelected) = get(&base, &format!("/epr-head/{UNELECTED}")).await;
    assert_eq!(status, 200, "{unelected}");
    assert!(
        unelected.get("election").is_none(),
        "no recorded election omits the key: {unelected}"
    );
    assert!(elected["cid"].is_string(), "{elected}");
}

/// The elector has exactly one producer — this peer's own conductor on a
/// `?election=live` read. With no conductor (none here) the field is ABSENT on
/// both reads, and the content head's `electorSource` says why.
#[tokio::test]
async fn without_a_conductor_no_elector_is_named_and_the_source_says_so() {
    let base = serve().await;
    let (status, plain) = get(&base, &format!("/db/content/{ELECTED}/head")).await;
    assert_eq!(status, 200, "{plain}");
    assert!(plain.get("elector").is_none(), "{plain}");
    assert!(plain.get("electorSource").is_none(), "{plain}");

    let (status, live) = get(&base, &format!("/db/content/{ELECTED}/head?election=live")).await;
    assert_eq!(status, 200, "{live}");
    assert!(live.get("elector").is_none(), "{live}");
    assert_eq!(live["electorSource"], "cached", "{live}");

    let (status, head) = get(&base, &format!("/epr-head/{ELECTED}?election=live")).await;
    assert_eq!(status, 200, "{head}");
    assert!(head["election"].is_object(), "{head}");
    assert!(head["election"].get("elector").is_none(), "{head}");
}
