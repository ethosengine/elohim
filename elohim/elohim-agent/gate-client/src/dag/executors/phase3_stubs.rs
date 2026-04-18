//! Phase 3+ stub resolvers for `ContextAssembleExecutor`.
//!
//! These types implement [`PullResolver`] but return `Value::Null` with a
//! `tracing::warn!` log. They exist so the universal-band DAG can run
//! end-to-end in dev-context without all infrastructure present. Real
//! implementations land in Phase 3+ when elohim-storage, DHT, source-chain,
//! and manifest wiring are available.

use async_trait::async_trait;
use serde_json::Value;
use tracing::warn;

use crate::error::GateError;

use super::context_assemble::PullResolver;

// ─── Phase 3+ stub resolver types ────────────────────────────────────────────

pub(super) struct ElohimStorageResolver;
pub(super) struct DhtResolver;
pub(super) struct SourceChainResolver;
pub(super) struct ManifestResolver;

#[async_trait]
impl PullResolver for ElohimStorageResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "elohim-storage",
            pull.query = query,
            "elohim-storage pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

#[async_trait]
impl PullResolver for DhtResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "dht",
            pull.query = query,
            "DHT pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

#[async_trait]
impl PullResolver for SourceChainResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "source-chain",
            pull.query = query,
            "source-chain pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

#[async_trait]
impl PullResolver for ManifestResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "manifest",
            pull.query = query,
            "manifest pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}
