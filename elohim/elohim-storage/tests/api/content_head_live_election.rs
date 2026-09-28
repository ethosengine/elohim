//! `GET /db/content/{id}/head?election=live` over the REAL server (F26).
//!
//! The live answer's conductor arms (live true/false, the column heal) are
//! proven against a scripted election in `services::live_earned`; what only the
//! served route can prove is the wire contract around them:
//!
//! - a plain `/head` read is unchanged — no `earnedSource` key at all;
//! - `?election=live` on a peer whose conductor cannot answer (none here) falls
//!   back to the column and SAYS so (`earnedSource: "cached"`), still 200;
//! - the reach gate still stands in front of the live read.

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
const STEWARDED: &str = "fct-stewarded-atom";
const INTIMATE: &str = "love-map-live-election";

fn row(conn: &mut SqliteConnection, id: &str, reach: &str, earned: Option<i32>) {
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
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.into(),
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
            content::canonical_declared_at.eq(earned.map(|_| 7_i64)),
            content::canonical_earned.eq(earned),
        ))
        .execute(conn)
        .unwrap();
}

async fn serve() -> String {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        row(&mut conn, STEWARDED, "commons", Some(1));
        row(&mut conn, INTIMATE, "intimate", None);
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
async fn plain_head_read_is_unchanged_and_names_no_earned_source() {
    let base = serve().await;
    let (status, body) = get(&base, &format!("/db/content/{STEWARDED}/head")).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["earned"], true);
    assert!(
        body.get("earnedSource").is_none(),
        "a plain read carries no earnedSource: {body}"
    );
}

#[tokio::test]
async fn live_read_without_a_conductor_answers_from_the_column_and_says_so() {
    let base = serve().await;
    let (status, body) = get(
        &base,
        &format!("/db/content/{STEWARDED}/head?election=live"),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["earnedSource"], "cached", "{body}");
    assert_eq!(
        body["earned"], true,
        "the column stands when nobody answered"
    );
    assert_eq!(body["headActionHash"], HEAD);

    // Any other value of the parameter is a plain read.
    let (_, body) = get(
        &base,
        &format!("/db/content/{STEWARDED}/head?election=cached"),
    )
    .await;
    assert!(body.get("earnedSource").is_none(), "{body}");
}

#[tokio::test]
async fn the_reach_gate_still_stands_in_front_of_the_live_read() {
    let base = serve().await;
    let (plain, _) = get(&base, &format!("/db/content/{INTIMATE}/head")).await;
    let (live, body) = get(&base, &format!("/db/content/{INTIMATE}/head?election=live")).await;
    assert_ne!(
        plain, 200,
        "an anonymous caller may not read an intimate head"
    );
    assert_eq!(
        live, plain,
        "the live read is gated exactly like the plain one"
    );
    assert!(body.get("earned").is_none(), "{body}");
}
