#![cfg(feature = "client")]

use elohim_sdk::{ClientMode, ContentClient, WriteOp, WritePriority};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, oneshot},
};

#[derive(serde::Serialize, serde::Deserialize)]
struct Content {
    id: String,
    version: u64,
}
impl elohim_sdk::ContentReadable for Content {
    fn content_type() -> &'static str {
        "content"
    }
    fn content_id(&self) -> &str {
        &self.id
    }
}

struct Request {
    path: String,
    body: Value,
    respond: oneshot::Sender<Option<(u16, Value)>>,
}

async fn server() -> (String, mpsc::Receiver<Request>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel(16);
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let (header_end, content_length) = loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        break (end + 4, length);
                    }
                };
                while bytes.len() < header_end + content_length {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                }
                let path = String::from_utf8_lossy(&bytes[..header_end])
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_string();
                let body = if content_length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[header_end..]).unwrap()
                };
                let (respond, answer) = oneshot::channel();
                if tx
                    .send(Request {
                        path,
                        body,
                        respond,
                    })
                    .await
                    .is_err()
                {
                    return;
                }
                if let Ok(Some((status, body))) = answer.await {
                    let body = body.to_string();
                    let response = format!("HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            });
        }
    });
    (url, rx, task)
}

fn client(url: &str, browser: bool) -> Arc<ContentClient> {
    Arc::new(ContentClient::new(
        if browser {
            ClientMode::Browser {
                doorway_url: url.into(),
                api_key: Some("test-key".into()),
            }
        } else {
            ClientMode::Native {
                storage_path: "unused".into(),
                sync_url: Some(url.into()),
            }
        },
        "lamad",
    ))
}

async fn queue(
    client: &ContentClient,
    kind: &str,
    id: &str,
    version: u64,
    priority: WritePriority,
) {
    client
        .write_buffer()
        .queue(WriteOp::new(
            kind,
            id,
            json!({"id":id,"version":version}),
            priority,
        ))
        .await
        .unwrap();
}

async fn pending(client: &ContentClient) -> usize {
    client.write_buffer().pending_counts().await.values().sum()
}

fn flush(client: &Arc<ContentClient>) -> tokio::task::JoinHandle<elohim_sdk::Result<()>> {
    let client = client.clone();
    tokio::spawn(async move { client.flush().await })
}

async fn completed(
    task: tokio::task::JoinHandle<elohim_sdk::Result<()>>,
) -> elohim_sdk::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("flush completion deadline")
        .unwrap()
}

async fn request(rx: &mut mpsc::Receiver<Request>) -> Request {
    tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

fn success(request: Request) {
    let count = request.body.as_array().unwrap().len();
    request
        .respond
        .send(Some((
            200,
            json!({"inserted":count,"skipped":0,"errors":[]}),
        )))
        .unwrap();
}

#[tokio::test]
async fn configured_sinks_retain_rejected_disconnected_and_application_failed_writes() {
    for browser in [false, true] {
        for response in [
            Some((503, json!({"error":"busy"}))),
            None,
            Some((200, json!({"inserted":0,"skipped":0,"errors":["rejected"]}))),
            Some((200, json!({"inserted":0,"skipped":1,"errors":[]}))),
            Some((200, json!({"inserted":0,"skipped":0,"errors":[]}))),
            Some((200, json!({"unexpected":"response"}))),
        ] {
            let (url, mut rx, server) = server().await;
            let client = client(&url, browser);
            queue(&client, "content", "retained", 1, WritePriority::Normal).await;
            let first = flush(&client);
            let rejected = request(&mut rx).await;
            assert_eq!(rejected.path, "/db/lamad/content/bulk");
            rejected.respond.send(response).unwrap();
            assert!(
                completed(first).await.is_err(),
                "failed write must be reported"
            );
            assert_eq!(pending(&client).await, 1);
            let retry = flush(&client);
            let accepted = request(&mut rx).await;
            assert_eq!(accepted.body, json!([{"id":"retained","version":1}]));
            let stored = accepted.body[0].clone();
            success(accepted);
            completed(retry).await.unwrap();
            let reader = client.clone();
            let readback = tokio::spawn(async move { reader.get::<Content>("retained").await });
            let read = request(&mut rx).await;
            assert_eq!(
                read.path,
                if browser {
                    "/api/v1/cache/content/retained"
                } else {
                    "/db/lamad/content/retained"
                }
            );
            read.respond.send(Some((200, stored))).unwrap();
            let returned = readback.await.unwrap().unwrap().unwrap();
            assert_eq!((returned.id.as_str(), returned.version), ("retained", 1));
            assert_eq!(pending(&client).await, 0);
            server.abort();
        }
    }
}

#[tokio::test]
async fn acknowledged_group_is_not_replayed_after_another_group_fails() {
    for browser in [false, true] {
        let (url, mut rx, server) = server().await;
        let client = client(&url, browser);
        for kind in ["content", "path", "other"] {
            queue(&client, kind, kind, 1, WritePriority::Normal).await;
        }
        let first = flush(&client);
        let acknowledged = request(&mut rx).await;
        let acknowledged_path = acknowledged.path.clone();
        success(acknowledged);
        request(&mut rx)
            .await
            .respond
            .send(Some((503, json!({}))))
            .unwrap();
        assert!(completed(first).await.is_err());
        assert_eq!(pending(&client).await, 2);
        let retry = flush(&client);
        for _ in 0..2 {
            let remaining = request(&mut rx).await;
            assert_ne!(remaining.path, acknowledged_path);
            success(remaining);
        }
        completed(retry).await.unwrap();
        assert_eq!(pending(&client).await, 0);
        server.abort();
    }
}

#[tokio::test]
async fn cancellation_keeps_pending_write_for_retry() {
    let (url, mut rx, server) = server().await;
    let client = client(&url, false);
    queue(&client, "content", "cancelled", 1, WritePriority::Normal).await;
    let first = flush(&client);
    let held = request(&mut rx).await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    drop(held);
    assert_eq!(pending(&client).await, 1);
    let retry = flush(&client);
    success(request(&mut rx).await);
    completed(retry).await.unwrap();
    assert_eq!(pending(&client).await, 0);
    server.abort();
}

#[tokio::test]
async fn acknowledgement_preserves_newer_priority_changed_write_and_serializes_flushes() {
    for replacement_priority in [WritePriority::Normal, WritePriority::High] {
        let (url, mut rx, server) = server().await;
        let client = client(&url, false);
        queue(&client, "content", "updated", 1, WritePriority::Normal).await;
        let first = flush(&client);
        let held = request(&mut rx).await;
        queue(&client, "content", "updated", 2, replacement_priority).await;
        let second = flush(&client);
        assert!(tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err());
        success(held);
        completed(first).await.unwrap();
        let replacement = request(&mut rx).await;
        assert_eq!(replacement.body, json!([{"id":"updated","version":2}]));
        success(replacement);
        completed(second).await.unwrap();
        assert_eq!(pending(&client).await, 0);
        server.abort();
    }
}

#[tokio::test]
async fn full_buffer_accepts_replacement_but_refuses_new_identity() {
    use elohim_sdk::cache::{WriteBuffer, WriteBufferConfig};
    let buffer = WriteBuffer::new(WriteBufferConfig {
        max_size: 1,
        ..Default::default()
    });
    buffer
        .queue(WriteOp::new(
            "content",
            "same",
            json!(1),
            WritePriority::Normal,
        ))
        .await
        .unwrap();
    buffer
        .queue(WriteOp::new(
            "content",
            "same",
            json!(2),
            WritePriority::High,
        ))
        .await
        .unwrap();
    assert!(buffer
        .queue(WriteOp::new(
            "content",
            "different",
            json!(3),
            WritePriority::Normal
        ))
        .await
        .is_err());
    assert_eq!(buffer.backpressure().await, 100);
    let batch = buffer.take_batch().await;
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].data, json!(2));
    buffer.clear().await;
    assert_eq!(buffer.backpressure().await, 0);
}

#[tokio::test]
async fn skipped_existing_different_payload_does_not_acknowledge_new_content() {
    let (url, mut rx, server) = server().await;
    let client = client(&url, false);
    queue(&client, "content", "existing", 2, WritePriority::Normal).await;
    let attempt = flush(&client);
    let sent = request(&mut rx).await;
    assert_eq!(sent.body[0]["version"], 2);
    sent.respond
        .send(Some((200, json!({"inserted":0,"skipped":1,"errors":[]}))))
        .unwrap();
    assert!(completed(attempt).await.is_err());
    let reader = client.clone();
    let readback = tokio::spawn(async move { reader.get::<Content>("existing").await });
    request(&mut rx)
        .await
        .respond
        .send(Some((200, json!({"id":"existing","version":1}))))
        .unwrap();
    assert_eq!(readback.await.unwrap().unwrap().unwrap().version, 1);
    assert_eq!(pending(&client).await, 1);
    let preserved = client.write_buffer().take_batch().await;
    assert_eq!(preserved[0].data["version"], 2);
    server.abort();
}

#[tokio::test]
async fn partially_inserted_group_retains_every_item_without_guessing_error_identity() {
    let (url, mut rx, server) = server().await;
    let client = client(&url, true);
    queue(&client, "content", "one", 1, WritePriority::Normal).await;
    queue(&client, "content", "two", 1, WritePriority::Normal).await;
    let attempt = flush(&client);
    let sent = request(&mut rx).await;
    assert_eq!(sent.body.as_array().unwrap().len(), 2);
    sent.respond
        .send(Some((
            200,
            json!({"inserted":1,"skipped":0,"errors":["unstructured failure"]}),
        )))
        .unwrap();
    assert!(completed(attempt).await.is_err());
    assert_eq!(pending(&client).await, 2);
    let preserved = client.write_buffer().take_batch().await;
    assert!(preserved.iter().any(|op| op.id == "one"));
    assert!(preserved.iter().any(|op| op.id == "two"));
    server.abort();
}
