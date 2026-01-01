//! Debug Stream - Real-time debugging events for elohim-storage
//!
//! Provides a WebSocket endpoint at `/debug/stream` that emits events for:
//! - Import processing (batch queued, chunk processed, errors)
//! - Conductor client (connection, disconnection, zome calls)
//! - Cell discovery (attempts, success, failure)
//! - Blob operations (store, retrieve)

use futures_util::{SinkExt, StreamExt};
use hyper_tungstenite::tungstenite::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Debug event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugEvent {
    pub timestamp: String,
    pub source: String,
    pub event_type: String,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DebugEvent {
    pub fn new(event_type: &str, level: &str, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            source: "storage".to_string(),
            event_type: event_type.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn info(event_type: &str, message: &str) -> Self {
        Self::new(event_type, "info", message)
    }

    pub fn warn(event_type: &str, message: &str) -> Self {
        Self::new(event_type, "warn", message)
    }

    pub fn error(event_type: &str, message: &str) -> Self {
        Self::new(event_type, "error", message)
    }

    pub fn debug(event_type: &str, message: &str) -> Self {
        Self::new(event_type, "debug", message)
    }
}

/// Debug event broadcaster
#[derive(Clone)]
pub struct DebugBroadcaster {
    tx: broadcast::Sender<DebugEvent>,
}

impl DebugBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }

    /// Emit a debug event
    pub fn emit(&self, event: DebugEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DebugEvent> {
        self.tx.subscribe()
    }

    // Convenience methods

    pub fn cell_discovery_start(&self, admin_url: &str, app_id: &str) {
        self.emit(DebugEvent::info("cell_discovery",
            &format!("🔍 Starting cell discovery: app={} admin={}", app_id, admin_url)));
    }

    pub fn cell_discovery_success(&self, role: &str) {
        self.emit(DebugEvent::info("cell_discovery",
            &format!("✅ Cell discovered for role '{}'", role)));
    }

    pub fn cell_discovery_failed(&self, error: &str) {
        self.emit(DebugEvent::error("cell_discovery",
            &format!("❌ Cell discovery failed: {}", error)));
    }

    pub fn conductor_connecting(&self, url: &str) {
        self.emit(DebugEvent::info("conductor",
            &format!("🔌 Connecting to conductor: {}", url)));
    }

    pub fn conductor_connected(&self) {
        self.emit(DebugEvent::info("conductor", "✅ Conductor connected"));
    }

    pub fn conductor_disconnected(&self, error: &str) {
        self.emit(DebugEvent::warn("conductor",
            &format!("⚠️ Conductor disconnected: {}", error)));
    }

    pub fn import_batch_start(&self, batch_id: &str, batch_type: &str, total: usize) {
        self.emit(DebugEvent::info("import",
            &format!("📥 BATCH_START: {} type={} items={}", batch_id, batch_type, total))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "batch_type": batch_type,
                "total_items": total
            })));
    }

    pub fn import_chunk_start(&self, batch_id: &str, chunk_idx: usize, chunk_size: usize) {
        self.emit(DebugEvent::debug("import",
            &format!("🔄 CHUNK[{}]: Sending {} items to conductor", chunk_idx, chunk_size))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "chunk_index": chunk_idx,
                "chunk_size": chunk_size
            })));
    }

    pub fn import_chunk_success(&self, batch_id: &str, chunk_idx: usize, duration_ms: u64) {
        self.emit(DebugEvent::info("import",
            &format!("✅ CHUNK[{}]: Success in {}ms", chunk_idx, duration_ms))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "chunk_index": chunk_idx,
                "duration_ms": duration_ms
            })));
    }

    pub fn import_chunk_skipped(&self, batch_id: &str, chunk_idx: usize, reason: &str) {
        self.emit(DebugEvent::warn("import",
            &format!("⚠️ CHUNK[{}]: SKIPPED - {}", chunk_idx, reason))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "chunk_index": chunk_idx,
                "reason": reason
            })));
    }

    pub fn import_chunk_error(&self, batch_id: &str, chunk_idx: usize, error: &str) {
        self.emit(DebugEvent::error("import",
            &format!("❌ CHUNK[{}]: Error - {}", chunk_idx, error))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "chunk_index": chunk_idx,
                "error": error
            })));
    }

    pub fn import_batch_complete(&self, batch_id: &str, processed: usize, errors: usize, duration_ms: u64) {
        let status = if errors == 0 { "✅ completed" } else { "⚠️ completed with errors" };
        self.emit(DebugEvent::info("import",
            &format!("📦 BATCH_COMPLETE: {} {} ({}/{} in {}ms)",
                batch_id, status, processed, processed + errors, duration_ms))
            .with_data(serde_json::json!({
                "batch_id": batch_id,
                "processed": processed,
                "errors": errors,
                "duration_ms": duration_ms
            })));
    }

    pub fn zome_call(&self, zome: &str, fn_name: &str, duration_ms: u64, success: bool) {
        let icon = if success { "✓" } else { "✗" };
        self.emit(DebugEvent::debug("zome_call",
            &format!("{} {}::{} ({}ms)", icon, zome, fn_name, duration_ms))
            .with_data(serde_json::json!({
                "zome": zome,
                "function": fn_name,
                "duration_ms": duration_ms,
                "success": success
            })));
    }
}

impl Default for DebugBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle WebSocket connection for debug stream
pub async fn handle_debug_websocket(
    ws_stream: hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>,
    broadcaster: Arc<DebugBroadcaster>,
) {
    let (mut ws_write, mut ws_read) = ws_stream.split();
    let mut rx = broadcaster.subscribe();

    // Send welcome
    let welcome = DebugEvent::info("connected", "Debug stream connected to elohim-storage");
    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = ws_write.send(Message::Text(json.into())).await;
    }

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(evt) => {
                        if let Ok(json) = serde_json::to_string(&evt) {
                            if ws_write.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let lag = DebugEvent::warn("lag", &format!("Dropped {} events", n));
                        if let Ok(json) = serde_json::to_string(&lag) {
                            let _ = ws_write.send(Message::Text(json.into())).await;
                        }
                    }
                    Err(_) => break,
                }
            }
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Text(text))) => {
                        // Handle ping command
                        if text.contains("ping") {
                            let pong = DebugEvent::debug("pong", "pong");
                            if let Ok(json) = serde_json::to_string(&pong) {
                                let _ = ws_write.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    debug!("Debug client disconnected");
}
