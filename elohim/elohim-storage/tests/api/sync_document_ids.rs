//! URL encoding must preserve the identity already written by the sync producer.
//! Seed DocStore directly: writing and reading the same wrong HTTP key is no oracle.

use std::sync::Arc;
use std::time::Duration;

use automerge::{transaction::Transactable, Automerge, ReadDoc};
use base64::{engine::general_purpose::STANDARD, Engine};
use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::sync::{DocStore, StreamTracker, SyncManager};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::task::AbortHandle;

const APP: &str = "elohim";
const IDS: &[&str] = &[
    "node:household mental load",
    "node:colon:care",
    "node:progress 100%",
    "node:room/care",
    "node:room%2Fcare",
    "node:care/heads",
];

struct Fixture {
    base: String,
    store: Arc<DocStore>,
    client: reqwest::Client,
    server: AbortHandle,
    _dir: TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        DocStore::at_path(dir.path().join("sync.sled"))
            .await
            .unwrap(),
    );
    for id in IDS {
        let mut doc = Automerge::new();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "logicalId", *id)?;
            Ok(())
        })
        .unwrap();
        store.save(APP, id, &doc).await.unwrap();
    }
    let blobs = Arc::new(BlobStore::new(dir.path().join("blobs")).await.unwrap());
    let sync = Arc::new(SyncManager::new(
        store.clone(),
        Arc::new(StreamTracker::new()),
    ));
    let server = HttpServer::new(blobs, "127.0.0.1:0".parse().unwrap()).with_sync_manager(sync);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!(
        "http://{}/sync/v1/{APP}/docs",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(Arc::new(server).serve(listener)).abort_handle();
    Fixture {
        base,
        store,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
        server,
        _dir: dir,
    }
}

async fn response_json(response: reqwest::Response, context: &str) -> Value {
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{context}: {body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn encoded_document_ids_resolve_the_directly_saved_document() {
    let fixture = fixture().await;
    // The first assertion fails on the original dispatcher: a real HTTP GET of
    // node:household mental load returns 404 although its logical key exists.
    for id in IDS {
        let encoded = utf8_percent_encode(id, NON_ALPHANUMERIC).to_string();
        let url = format!("{}/{encoded}", fixture.base);
        let body = response_json(fixture.client.get(&url).send().await.unwrap(), id).await;
        let stored = fixture.store.get(APP, id).await.unwrap().unwrap();
        assert_eq!(body["docId"], *id);
        assert_eq!(body["heads"], json!(stored.heads));
        assert!(!stored.heads.is_empty());

        let body = response_json(
            fixture
                .client
                .get(format!("{url}/heads"))
                .send()
                .await
                .unwrap(),
            id,
        )
        .await;
        assert_eq!(body["docId"], *id);
        assert_eq!(body["heads"], json!(stored.heads));

        let body = response_json(
            fixture
                .client
                .get(format!("{url}/changes"))
                .send()
                .await
                .unwrap(),
            id,
        )
        .await;
        assert_eq!(body["docId"], *id);
        assert_eq!(body["newHeads"], json!(stored.heads));
        let mut downloaded = Automerge::new();
        for change in body["changes"].as_array().unwrap() {
            downloaded
                .load_incremental(&STANDARD.decode(change.as_str().unwrap()).unwrap())
                .unwrap();
        }
        assert_eq!(
            downloaded
                .get(automerge::ROOT, "logicalId")
                .unwrap()
                .unwrap()
                .0
                .to_str(),
            Some(*id),
        );
        assert!(fixture.store.get(APP, &encoded).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn encoded_changes_update_only_the_directly_saved_logical_key() {
    let fixture = fixture().await;
    for id in IDS {
        let encoded = utf8_percent_encode(id, NON_ALPHANUMERIC).to_string();
        let original = fixture.store.get(APP, id).await.unwrap().unwrap();
        let mut doc = Automerge::load(&original.data).unwrap();
        let old_heads = doc.get_heads();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "httpEdit", *id)?;
            Ok(())
        })
        .unwrap();
        let changes: Vec<_> = doc
            .get_changes(&old_heads)
            .iter()
            .map(|change| STANDARD.encode(change.raw_bytes()))
            .collect();
        let body = response_json(
            fixture
                .client
                .post(format!("{}/{encoded}/changes", fixture.base))
                .json(&json!({ "changes": changes }))
                .send()
                .await
                .unwrap(),
            id,
        )
        .await;
        assert_eq!(body["docId"], *id);
        let stored = fixture.store.get(APP, id).await.unwrap().unwrap();
        let expected_heads: Vec<_> = doc
            .get_heads()
            .iter()
            .map(|head| hex::encode(head.0))
            .collect();
        assert_eq!(stored.heads, expected_heads);
        assert_eq!(body["newHeads"], json!(expected_heads));
        let saved = Automerge::load(&stored.data).unwrap();
        assert_eq!(
            saved
                .get(automerge::ROOT, "httpEdit")
                .unwrap()
                .unwrap()
                .0
                .to_str(),
            Some(*id)
        );
        assert!(fixture.store.get(APP, &encoded).await.unwrap().is_none());
    }
}
