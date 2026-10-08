//! `/apps/{x}` is judged exactly as `/db/content/{x}` or `/blob/{x}` would be.
//!
//! Before this, `/apps/{slug}` resolved through the in-memory slug index and
//! `/apps/{address}` straight to the blob, and neither consulted a reach
//! verdict: a restricted app bundle served to anyone who knew its slug or its
//! address, and the capability probe disclosed its `X-Blob-Hash`. These tests
//! drive the REAL server over HTTP (`HttpServer::serve` on a loopback port) for
//! an anonymous caller, against one restricted and one commons html5-app row.

use std::io::Write as _;
use std::sync::Arc;

use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::AppContext;
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;

/// A minimal app bundle: an index.html and one script. `marker` keeps two
/// bundles byte-distinct, so each row references its own blob.
fn bundle(marker: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zw.start_file("index.html", opts).unwrap();
        write!(zw, "<!doctype html><title>{marker}</title>").unwrap();
        zw.start_file("main.js", opts).unwrap();
        zw.write_all(b"console.log('app')").unwrap();
        zw.finish().unwrap();
    }
    buf
}

struct Served {
    base: String,
    restricted_hash: String,
    open_hash: String,
}

async fn serve() -> Served {
    let blob_store = Arc::new(
        BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
            .await
            .unwrap(),
    );
    let restricted_hash = blob_store.store(&bundle("household")).await.unwrap().hash;
    let open_hash = blob_store.store(&bundle("open")).await.unwrap().hash;

    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        for (id, slug, reach, hash) in [
            ("household-app", "household", "community", &restricted_hash),
            ("open-app", "open", "commons", &open_hash),
        ] {
            create_content(
                &mut conn,
                &AppContext::default_lamad(),
                CreateContentInput {
                    id: id.into(),
                    title: id.into(),
                    description: None,
                    content_type: "app".into(),
                    content_format: "html5-app".into(),
                    blob_hash: Some(hash.clone()),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: reach.into(),
                    created_by: None,
                    tags: Vec::new(),
                    content_body: Some(serde_json::json!({ "slug": slug }).to_string()),
                    dht_anchor_hash: Some(format!("uhCkk-{id}")),
                },
            )
            .unwrap();
        }
    }
    let server =
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool.clone());
    server.load_slug_index().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(server).serve(listener));
    Served {
        base,
        restricted_hash,
        open_hash,
    }
}

#[tokio::test]
async fn a_restricted_app_is_refused_by_slug_and_by_address_alike() {
    let s = serve().await;
    let client = reqwest::Client::new();
    for identifier in ["household-app", "household", s.restricted_hash.as_str()] {
        let resp = client
            .get(format!("{}/apps/{identifier}/index.html", s.base))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 403, "/apps/{identifier}/index.html");
        let body = resp.text().await.unwrap();
        assert!(!body.contains("<!doctype html>"), "{identifier}: {body}");
        assert!(body.contains("requiredReach"), "{identifier}: {body}");
    }
    // The same row and the same bytes on their own routes: refused alike.
    let row = client
        .get(format!("{}/db/content/household-app", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(row.status(), 403);
    let blob = client
        .get(format!("{}/blob/{}", s.base, s.restricted_hash))
        .send()
        .await
        .unwrap();
    assert_eq!(blob.status(), 403);
}

#[tokio::test]
async fn the_capability_probe_does_not_disclose_a_restricted_bundle() {
    let s = serve().await;
    let client = reqwest::Client::new();
    for identifier in ["household-app", s.restricted_hash.as_str()] {
        let resp = client
            .head(format!("{}/apps/{identifier}/_capability", s.base))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 403, "{identifier}");
        assert!(resp.headers().get("x-blob-hash").is_none(), "{identifier}");
    }
    let open = client
        .head(format!("{}/apps/open-app/_capability", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(open.status(), 200);
    assert_eq!(
        open.headers().get("x-blob-hash").unwrap(),
        s.open_hash.as_str()
    );
}

#[tokio::test]
async fn a_commons_app_still_serves_by_slug_and_by_address() {
    let s = serve().await;
    let client = reqwest::Client::new();
    for identifier in ["open-app", "open", s.open_hash.as_str()] {
        let resp = client
            .get(format!("{}/apps/{identifier}/index.html", s.base))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "/apps/{identifier}/index.html");
        assert!(resp.text().await.unwrap().contains("<title>open</title>"));
    }
}
