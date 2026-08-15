#![cfg(feature = "client")]

use std::path::PathBuf;

use elohim_sdk::{
    ClientMode, ContentClient, ContentReadable, ContentWriteable, SdkError, WritePriority,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TestContent {
    id: String,
}

impl ContentReadable for TestContent {
    fn content_type() -> &'static str {
        "content"
    }

    fn content_id(&self) -> &str {
        &self.id
    }
}

impl ContentWriteable for TestContent {}

#[tokio::test]
async fn native_without_sync_url_refuses_flush_without_dropping_queued_writes() {
    let client = ContentClient::new(
        ClientMode::Native {
            storage_path: PathBuf::from("unused-in-memory-test-path"),
            sync_url: None,
        },
        "test",
    );

    client
        .save(&TestContent {
            id: "preserved-write".into(),
        })
        .await
        .expect("the write should queue before a flush is attempted");

    let pending_before = client.write_buffer().pending_counts().await;
    assert_eq!(pending_before.get(&WritePriority::Normal), Some(&1));

    let error = client
        .flush()
        .await
        .expect_err("a Native client without a sync URL has no writable backend");
    assert!(matches!(
        error,
        SdkError::InvalidMode(message) if message.contains("without sync_url")
    ));

    let pending_after = client.write_buffer().pending_counts().await;
    assert_eq!(pending_after.get(&WritePriority::Normal), Some(&1));
}
