//! ContextAssembleExecutor — gathers signals into GateContext via pull sources.
//!
//! Phase 2: ships a `StubResolver` for dev-context use. Real resolvers
//! (`ElohimStorage`, `Dht`, `SourceChain`, `Manifest`) return `Value::Null` with
//! a warning — they do not fail hard so the universal-band DAG can run in
//! dev-context without all infrastructure present.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tracing::warn;

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{ContextAssembleParams, StepType};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;

use super::super::context::GateContext;

// ─── Pull source enum ─────────────────────────────────────────────────────────

/// The resolved source for a pull in a `ContextAssemble` step.
///
/// Phase 2: only `Stub` is fully implemented. All other variants return
/// `Value::Null` with a `tracing::warn!` — they do not panic, so the DAG can
/// run end-to-end in dev-context.
#[derive(Debug, Clone)]
pub enum PullSource {
    /// Returns a pre-configured value — used in tests and dev-context DAGs.
    Stub(Value),
    /// Pulls from elohim-storage (Phase 3+).
    ElohimStorage,
    /// Pulls from the DHT (Phase 3+).
    Dht,
    /// Pulls from the agent's private source chain (Phase 3+).
    SourceChain,
    /// Pulls from the gate manifest (Phase 3+).
    Manifest,
}

// ─── PullResolver trait ───────────────────────────────────────────────────────

/// Maps a query string to a resolved `Value`.
pub trait PullResolver: Send + Sync {
    fn resolve(&self, query: &str) -> Result<Value, GateError>;
}

// ─── StubResolver ────────────────────────────────────────────────────────────

/// A resolver that returns a fixed value for any query string. Used for
/// dev-context DAGs and unit tests.
pub struct StubResolver {
    /// Maps query strings to their configured return values. If the query
    /// is not in the map, `Value::Null` is returned.
    pub values: HashMap<String, Value>,
}

impl StubResolver {
    pub fn new(values: HashMap<String, Value>) -> Self {
        Self { values }
    }

    /// Convenience: a resolver that returns a single value for any query.
    pub fn constant(value: Value) -> Self {
        // We store it under a sentinel — the actual resolution ignores the query
        // and always returns the constant.
        let mut map = HashMap::new();
        map.insert("*".to_string(), value);
        Self { values: map }
    }
}

impl PullResolver for StubResolver {
    fn resolve(&self, query: &str) -> Result<Value, GateError> {
        // Exact key match first, then wildcard, then null.
        if let Some(v) = self.values.get(query) {
            return Ok(v.clone());
        }
        if let Some(v) = self.values.get("*") {
            return Ok(v.clone());
        }
        Ok(Value::Null)
    }
}

// ─── Phase 3+ stub resolvers ──────────────────────────────────────────────────

struct ElohimStorageResolver;
struct DhtResolver;
struct SourceChainResolver;
struct ManifestResolver;

impl PullResolver for ElohimStorageResolver {
    fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "elohim-storage",
            pull.query = query,
            "elohim-storage pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

impl PullResolver for DhtResolver {
    fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "dht",
            pull.query = query,
            "DHT pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

impl PullResolver for SourceChainResolver {
    fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "source-chain",
            pull.query = query,
            "source-chain pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

impl PullResolver for ManifestResolver {
    fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "manifest",
            pull.query = query,
            "manifest pull resolver not yet implemented (Phase 3+); returning null"
        );
        Ok(Value::Null)
    }
}

// ─── ContextAssembleExecutor ──────────────────────────────────────────────────

/// Executor for `StepType::ContextAssemble`.
///
/// Iterates the `pulls` declared in the step params, resolves each via the
/// registered resolver for its `from` source, and writes the result into
/// `GateContext` under the pull's `output_key`.
///
/// Resolver lookup order:
/// 1. Exact match on `pull.from` in the `resolvers` map.
/// 2. If not found, falls back to the appropriate Phase 3+ stub (warn + null).
pub struct ContextAssembleExecutor {
    resolvers: HashMap<String, Box<dyn PullResolver>>,
}

impl ContextAssembleExecutor {
    /// Create an executor with no pre-registered resolvers (Phase 3+ stubs fire
    /// for every pull).
    pub fn new() -> Self {
        Self { resolvers: HashMap::new() }
    }

    /// Register a named resolver. The name is matched against `pull.from`.
    pub fn register(&mut self, source_name: impl Into<String>, resolver: Box<dyn PullResolver>) {
        self.resolvers.insert(source_name.into(), resolver);
    }

    fn execute_params(
        &self,
        params: &ContextAssembleParams,
        ctx: &mut GateContext,
    ) -> Result<(), GateError> {
        for pull in &params.pulls {
            let value = if let Some(resolver) = self.resolvers.get(&pull.from) {
                resolver.resolve(&pull.query)?
            } else {
                // Route to the appropriate Phase 3+ stub by source name.
                match pull.from.as_str() {
                    "elohim-storage" => ElohimStorageResolver.resolve(&pull.query)?,
                    "dht" => DhtResolver.resolve(&pull.query)?,
                    "source-chain" => SourceChainResolver.resolve(&pull.query)?,
                    "manifest" => ManifestResolver.resolve(&pull.query)?,
                    other => {
                        warn!(
                            pull.source = other,
                            pull.query = pull.query,
                            "unknown pull source; returning null"
                        );
                        Value::Null
                    }
                }
            };
            ctx.insert(pull.output_key.clone(), value);
        }
        Ok(())
    }
}

impl Default for ContextAssembleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for ContextAssembleExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::ContextAssemble { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "ContextAssembleExecutor received wrong step kind".to_string(),
                ))
            }
        };
        self.execute_params(params, ctx)?;
        Ok(StepOutcome::Continue)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{ContextAssembleParams, Pull};
    use crate::events::RelationalImpactEvent;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn assemble_step(pulls: Vec<Pull>) -> StepType {
        StepType::ContextAssemble {
            params: ContextAssembleParams { pulls },
        }
    }

    // ─── Happy path: stub resolver writes output_key ──────────────────────────

    #[tokio::test]
    async fn stub_resolver_writes_output_key() {
        let mut exec = ContextAssembleExecutor::new();
        let mut values = HashMap::new();
        values.insert("my-query".to_string(), json!({"score": 42}));
        exec.register("my-source", Box::new(StubResolver::new(values)));

        let step = assemble_step(vec![Pull {
            from: "my-source".into(),
            query: "my-query".into(),
            output_key: "scoreData".into(),
        }]);

        let mut ctx = GateContext::new();
        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        assert_eq!(ctx.get("scoreData"), Some(&json!({"score": 42})));
    }

    // ─── Multiple pulls write multiple keys ───────────────────────────────────

    #[tokio::test]
    async fn multiple_pulls_write_multiple_keys() {
        let mut exec = ContextAssembleExecutor::new();
        let mut values = HashMap::new();
        values.insert("q1".to_string(), json!("v1"));
        values.insert("q2".to_string(), json!("v2"));
        exec.register("src", Box::new(StubResolver::new(values)));

        let step = assemble_step(vec![
            Pull { from: "src".into(), query: "q1".into(), output_key: "key1".into() },
            Pull { from: "src".into(), query: "q2".into(), output_key: "key2".into() },
        ]);

        let mut ctx = GateContext::new();
        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert_eq!(ctx.get("key1"), Some(&json!("v1")));
        assert_eq!(ctx.get("key2"), Some(&json!("v2")));
    }

    // ─── Unknown source falls back to null without erroring ──────────────────

    #[tokio::test]
    async fn unknown_source_returns_null_without_error() {
        let exec = ContextAssembleExecutor::new();
        let step = assemble_step(vec![Pull {
            from: "totally-unknown".into(),
            query: "anything".into(),
            output_key: "out".into(),
        }]);

        let mut ctx = GateContext::new();
        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        assert_eq!(ctx.get("out"), Some(&Value::Null));
    }

    // ─── Phase 3+ stub sources return null without erroring ──────────────────

    #[tokio::test]
    async fn phase3_stub_sources_return_null_and_continue() {
        let exec = ContextAssembleExecutor::new();
        for source in ["elohim-storage", "dht", "source-chain", "manifest"] {
            let step = assemble_step(vec![Pull {
                from: source.into(),
                query: "any-query".into(),
                output_key: "result".into(),
            }]);

            let mut ctx = GateContext::new();
            let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
            assert!(
                matches!(outcome, StepOutcome::Continue),
                "source '{source}' should return Continue"
            );
            assert_eq!(
                ctx.get("result"),
                Some(&Value::Null),
                "source '{source}' should write null"
            );
        }
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = ContextAssembleExecutor::new();
        let wrong_step = StepType::WisdomInvoke {
            params: crate::dag::WisdomInvokeParams {
                constitution_cid: "c".into(),
                framing_cid: "f".into(),
                context_keys: vec![],
                output_key: "o".into(),
            },
        };
        let mut ctx = GateContext::new();
        let result = exec.execute(&wrong_step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(
            matches!(err, GateError::DagExecution(_)),
            "expected DagExecution, got: {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("wrong step kind"),
            "error message should mention wrong step kind, got: {msg}"
        );
    }

    // ─── Empty pulls list → Continue with unchanged context ──────────────────

    #[tokio::test]
    async fn empty_pulls_list_continues_with_unchanged_context() {
        let exec = ContextAssembleExecutor::new();
        let step = assemble_step(vec![]);
        let mut ctx = GateContext::new();
        ctx.insert("pre", json!("existing"));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        assert!(matches!(outcome, StepOutcome::Continue));
        assert_eq!(ctx.get("pre"), Some(&json!("existing")));
    }

    // ─── Constant stub resolver returns same value for any query ─────────────

    #[tokio::test]
    async fn constant_stub_resolver_returns_value_for_any_query() {
        let resolver = StubResolver::constant(json!(99));
        assert_eq!(resolver.resolve("anything").unwrap(), json!(99));
        assert_eq!(resolver.resolve("other-query").unwrap(), json!(99));
    }
}
