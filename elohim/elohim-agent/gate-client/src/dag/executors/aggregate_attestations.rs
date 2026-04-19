//! AggregateAttestationsExecutor — queries attestations, applies a reduction,
//! and writes a structured outcome to GateContext.
//!
//! Matches `StepType::AggregateAttestations { params }`. Implements the 6th
//! concrete [`StepExecutor`] (Phase 5 adds this one; Phase 2+3 shipped five).
//!
//! # Execution model
//!
//! 1. Fetch the aggregation-spec body via the injected [`ContentNodeResolver`]
//!    using `params.aggregation_spec_cid` as the lookup key.
//! 2. Deserialize the body as [`AggregationSpec`].
//! 3. Read `params.subject_key` from GateContext — gives us the subject string
//!    (agent ID, content CID, etc.).
//! 4. Fetch attestations from the injected [`AttestationResolver`] using the
//!    subject and `spec.attestationKinds`. A dev-context stub returns fixtures.
//! 5. Apply the declared `reduction`:
//!    - **count**: compare total count to `requiredCount`; return bool+count.
//!    - **weighted-sum**: sum weighted kind totals, compare to `threshold`.
//!    - **threshold-ladder**: walk tiers (highest `minCount` first); first
//!      tier met wins; return its `level` string (or null if none met).
//! 6. Write a structured `AggregationOutput` JSON object to `params.output_key`.
//! 7. Return `StepOutcome::Continue`.
//!
//! # Output shape
//!
//! The output written to `output_key` is always a JSON object:
//!
//! ```json
//! {
//!   "level":          "community",   // tier label, bool string, or null
//!   "count":          7,             // total qualifying attestation count
//!   "edge_case":      false,         // true when result is borderline
//!   "reduction_kind": "threshold-ladder"
//! }
//! ```
//!
//! The reach-gate DAG reads `aggregate.edge_case` on its conditional edge to
//! decide whether to invoke wisdom.
//!
//! # Edge-case detection
//!
//! - **count**: edge_case when count is within ±1 of `requiredCount`.
//! - **weighted-sum**: edge_case when sum is within ±10% of threshold.
//! - **threshold-ladder**: edge_case when count exactly equals the winning
//!   tier's `minCount` (boundary entry — subject just crossed the tier).
//!
//! # AttestationResolver
//!
//! The executor is injected with a `Box<dyn AttestationResolver>` at
//! construction time. In Phase 5 dev-context, use [`StubAttestationResolver`]
//! pre-loaded with subject→records fixtures. Phase 6+ wires a DHT-backed
//! resolver here.
//!
//! # Wrong step kind
//!
//! Receiving a step whose `kind()` is not `AggregateAttestations` is a
//! `GateError::DagExecution` — same contract as all other executors.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, info, warn};

use crate::dag::aggregation_spec::{AggregationSpec, Reduction};
use crate::dag::context::GateContext;
use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::executors::ContentNodeResolver;
use crate::dag::{AggregateAttestationsParams, StepType};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;

// ─── AttestationRecord ────────────────────────────────────────────────────────

/// A single attestation record returned by the [`AttestationResolver`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationRecord {
    /// Matches one of `aggregationSpec.attestationKinds`.
    pub kind: String,
    /// The subject this attestation is about.
    pub subject: String,
    /// Agent who issued this attestation.
    pub issuer: String,
    /// ISO 8601 timestamp of issuance.
    pub issued_at: String,
    /// Opaque JSON payload from the attestation body.
    pub payload_json: String,
}

// ─── AttestationResolver trait ────────────────────────────────────────────────

/// Resolves attestations for a given subject and kind filter.
///
/// Kept separate from [`ContentNodeResolver`] because the semantics differ:
/// `ContentNodeResolver` performs a CID-to-body lookup (point query), while
/// `AttestationResolver` performs a filtered set query over the DHT graph.
/// Mixing them would bleed DHT graph semantics into the content-address lookup
/// contract.
///
/// # Phase 5 dev-context
///
/// Use [`StubAttestationResolver`] with pre-configured fixtures.
///
/// # Phase 6+ production
///
/// Inject a resolver backed by real DHT `get_links` queries over the imagodei
/// DNA's attestation link types.
#[async_trait]
pub trait AttestationResolver: Send + Sync {
    /// Fetch attestations for `subject` whose kind is in `kinds`.
    ///
    /// `time_window` is an optional ISO 8601 duration (e.g., "P180D"). The
    /// resolver is responsible for filtering by issuance time. In Phase 5 dev-
    /// context the stub ignores the time window (returns all fixtures).
    ///
    /// Returns `Err(GateError::DagExecution(_))` on resolver failure.
    async fn fetch(
        &self,
        subject: &str,
        kinds: &[String],
        time_window: Option<&str>,
    ) -> Result<Vec<AttestationRecord>, GateError>;
}

// ─── StubAttestationResolver ─────────────────────────────────────────────────

/// Dev-context stub: returns pre-configured fixtures keyed by subject string.
///
/// Used in unit tests and Phase 5 dev-context where no real DHT is available.
/// The time window and kind filter are applied from the fixture set so tests
/// can construct minimal attestation sets without worrying about the resolver
/// interface details.
pub struct StubAttestationResolver {
    /// Maps subject string → list of attestation records.
    pub fixtures: HashMap<String, Vec<AttestationRecord>>,
}

impl StubAttestationResolver {
    /// Construct from a pre-built fixture map.
    pub fn new(fixtures: HashMap<String, Vec<AttestationRecord>>) -> Self {
        Self { fixtures }
    }

    /// Construct from a single subject→records pair. Convenient for unit tests.
    pub fn single(subject: impl Into<String>, records: Vec<AttestationRecord>) -> Self {
        let mut map = HashMap::new();
        map.insert(subject.into(), records);
        Self { fixtures: map }
    }

    /// An empty stub that returns no records for any subject.
    pub fn empty() -> Self {
        Self {
            fixtures: HashMap::new(),
        }
    }

    /// Helper: build an `AttestationRecord` with required fields only.
    pub fn record(kind: impl Into<String>, subject: impl Into<String>) -> AttestationRecord {
        AttestationRecord {
            kind: kind.into(),
            subject: subject.into(),
            issuer: "stub-issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            payload_json: "{}".into(),
        }
    }
}

#[async_trait]
impl AttestationResolver for StubAttestationResolver {
    async fn fetch(
        &self,
        subject: &str,
        kinds: &[String],
        _time_window: Option<&str>,
    ) -> Result<Vec<AttestationRecord>, GateError> {
        let all = self.fixtures.get(subject).cloned().unwrap_or_default();
        // Filter to requested kinds.
        let filtered = all
            .into_iter()
            .filter(|r| kinds.contains(&r.kind))
            .collect();
        Ok(filtered)
    }
}

/// A no-op [`AttestationResolver`] used by the default `DagRunner::new()`.
///
/// Returns an empty `Vec` with a `tracing::warn!` so misconfigured runners
/// fail loudly in logs rather than silently returning wrong results.
pub struct NullAttestationResolver;

#[async_trait]
impl AttestationResolver for NullAttestationResolver {
    async fn fetch(
        &self,
        subject: &str,
        kinds: &[String],
        _time_window: Option<&str>,
    ) -> Result<Vec<AttestationRecord>, GateError> {
        warn!(
            subject,
            ?kinds,
            "NullAttestationResolver: no resolver configured — \
             returning empty attestation set; inject a real resolver via \
             DagRunner::with_attestation_resolver()"
        );
        Ok(vec![])
    }
}

// ─── AggregationOutput ────────────────────────────────────────────────────────

/// The structured value written to `output_key` in GateContext.
///
/// All three reduction kinds produce this shape so DAG edges can use a single
/// `aggregate.edge_case` condition regardless of which reduction was applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregationOutput {
    /// The reduction's verdict label:
    /// - count: `"true"` / `"false"` (as string for consistency)
    /// - weighted-sum: `"true"` / `"false"`
    /// - threshold-ladder: the matched tier's `level` string, or `null` if none
    pub level: Option<String>,
    /// Total count of qualifying attestations (all kinds combined).
    pub count: u64,
    /// True when the result is borderline — triggers `wisdom-invoke-edge` in reach-gate.
    pub edge_case: bool,
    /// Which reduction kind produced this output.
    pub reduction_kind: String,
}

impl From<AggregationOutput> for Value {
    fn from(o: AggregationOutput) -> Self {
        json!({
            "level":          o.level,
            "count":          o.count,
            "edge_case":      o.edge_case,
            "reduction_kind": o.reduction_kind,
        })
    }
}

// ─── Reduction engine ─────────────────────────────────────────────────────────

/// Apply the `count` reduction.
fn apply_count(records: &[AttestationRecord], required_count: u64) -> AggregationOutput {
    let count = records.len() as u64;
    let met = count >= required_count;
    // Edge case: count within ±1 of requiredCount.
    let edge_case = count.abs_diff(required_count) <= 1;
    AggregationOutput {
        level: Some(if met { "true".into() } else { "false".into() }),
        count,
        edge_case,
        reduction_kind: "count".into(),
    }
}

/// Apply the `weighted-sum` reduction.
fn apply_weighted_sum(
    records: &[AttestationRecord],
    weights: &HashMap<String, f64>,
    threshold: f64,
) -> AggregationOutput {
    let count = records.len() as u64;
    // Sum weight × occurrence-count per kind.
    let mut kind_counts: HashMap<&str, u64> = HashMap::new();
    for r in records {
        *kind_counts.entry(r.kind.as_str()).or_default() += 1;
    }
    let sum: f64 = kind_counts
        .iter()
        .map(|(kind, &cnt)| weights.get(*kind).copied().unwrap_or(0.0) * cnt as f64)
        .sum();
    let met = sum >= threshold;
    // Edge case: sum within ±10% of threshold (or ±1 unit if threshold is tiny).
    let epsilon = (threshold * 0.10_f64).max(1.0);
    let edge_case = (sum - threshold).abs() <= epsilon;
    AggregationOutput {
        level: Some(if met { "true".into() } else { "false".into() }),
        count,
        edge_case,
        reduction_kind: "weighted-sum".into(),
    }
}

/// Apply the `threshold-ladder` reduction.
///
/// Tiers are walked from highest `minCount` to lowest (the spec says "ordered
/// tiers, highest-qualified wins"). The caller is responsible for supplying
/// tiers in any order; we sort by `min_count` descending here.
fn apply_threshold_ladder(
    records: &[AttestationRecord],
    tiers: &[crate::dag::aggregation_spec::LadderTier],
) -> AggregationOutput {
    let count = records.len() as u64;

    // Sort descending by minCount so we check the highest tier first.
    let mut sorted: Vec<_> = tiers.iter().collect();
    sorted.sort_by(|a, b| b.min_count.cmp(&a.min_count));

    let mut matched_level: Option<&str> = None;
    let mut matched_min_count: Option<u64> = None;

    for tier in &sorted {
        if count >= tier.min_count {
            matched_level = Some(&tier.level);
            matched_min_count = Some(tier.min_count);
            break;
        }
    }

    // Edge case: count exactly equals the winning tier's minCount (boundary entry).
    let edge_case = matched_min_count.map(|mc| count == mc).unwrap_or(false);

    AggregationOutput {
        level: matched_level.map(|s| s.to_owned()),
        count,
        edge_case,
        reduction_kind: "threshold-ladder".into(),
    }
}

// ─── AggregateAttestationsExecutor ───────────────────────────────────────────

/// Executor for `StepType::AggregateAttestations`.
///
/// Injected with a [`ContentNodeResolver`] (for fetching the aggregation-spec
/// body by CID) and an [`AttestationResolver`] (for fetching attestation
/// records from the DHT graph). Both are stateless once constructed; the
/// executor is `Send + Sync` and can be wrapped in `Arc<dyn StepExecutor>`.
pub struct AggregateAttestationsExecutor {
    content_resolver: Box<dyn ContentNodeResolver>,
    attestation_resolver: Box<dyn AttestationResolver>,
}

impl AggregateAttestationsExecutor {
    /// Construct with caller-supplied resolvers.
    pub fn new(
        content_resolver: Box<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
    ) -> Self {
        Self {
            content_resolver,
            attestation_resolver,
        }
    }

    async fn execute_params(
        &self,
        params: &AggregateAttestationsParams,
        ctx: &mut GateContext,
    ) -> Result<(), GateError> {
        // Step 1: fetch the aggregation-spec body.
        let body = self
            .content_resolver
            .resolve_body(&params.aggregation_spec_cid)
            .await?;

        // Step 2: parse the spec.
        let spec = AggregationSpec::from_json(&body)?;

        debug!(
            spec_cid = params.aggregation_spec_cid,
            spec_name = spec.spec_name,
            subject_key = params.subject_key,
            "AggregateAttestationsExecutor: fetched spec"
        );

        // Step 3: resolve the subject from context.
        let subject = ctx
            .get(&params.subject_key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GateError::DagExecution(format!(
                    "AggregateAttestationsExecutor: subject_key '{}' is missing or \
                     not a string in GateContext",
                    params.subject_key
                ))
            })?
            .to_owned();

        // Step 4: fetch attestations.
        let records = self
            .attestation_resolver
            .fetch(
                &subject,
                &spec.attestation_kinds,
                spec.time_window.as_deref(),
            )
            .await?;

        debug!(
            spec_name = spec.spec_name,
            subject,
            record_count = records.len(),
            "AggregateAttestationsExecutor: fetched attestation records"
        );

        // Step 5: apply reduction.
        let output = match &spec.reduction {
            Reduction::Count { required_count } => apply_count(&records, *required_count),
            Reduction::WeightedSum { weights, threshold } => {
                apply_weighted_sum(&records, weights, *threshold)
            }
            Reduction::ThresholdLadder { tiers } => apply_threshold_ladder(&records, tiers),
        };

        info!(
            spec_name = spec.spec_name,
            output_key = params.output_key,
            level = output.level.as_deref().unwrap_or("null"),
            count = output.count,
            edge_case = output.edge_case,
            reduction_kind = output.reduction_kind,
            "AggregateAttestationsExecutor: wrote output"
        );

        // Step 6: write structured output to context.
        ctx.insert(params.output_key.clone(), Value::from(output));

        Ok(())
    }
}

#[async_trait]
impl StepExecutor for AggregateAttestationsExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::AggregateAttestations { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "AggregateAttestationsExecutor received wrong step kind".to_string(),
                ))
            }
        };
        self.execute_params(params, ctx).await?;
        Ok(StepOutcome::Continue)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::executors::EmbeddedContentNodeResolver;
    use crate::dag::{AggregateAttestationsParams, StepType, WisdomInvokeParams};
    use crate::events::RelationalImpactEvent;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "subject-cid-1".into(),
            claim_kind: "brit".into(),
            issuer: "agent-a".into(),
        }
    }

    fn step(spec_cid: &str, subject_key: &str, output_key: &str) -> StepType {
        StepType::AggregateAttestations {
            params: AggregateAttestationsParams {
                aggregation_spec_cid: spec_cid.into(),
                subject_key: subject_key.into(),
                output_key: output_key.into(),
            },
        }
    }

    fn count_spec_body(required_count: u64) -> Value {
        json!({
            "specName": "reach-level-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit", "reach-endorsement"],
            "reduction": {
                "kind": "count",
                "requiredCount": required_count
            }
        })
    }

    fn weighted_sum_spec_body(threshold: f64) -> Value {
        json!({
            "specName": "weighted-reach-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit", "reach-endorsement", "deep-engagement"],
            "reduction": {
                "kind": "weighted-sum",
                "weights": {
                    "brit": 2.0,
                    "reach-endorsement": 1.0,
                    "deep-engagement": 3.0
                },
                "threshold": threshold
            }
        })
    }

    fn ladder_spec_body() -> Value {
        json!({
            "specName": "reach-tier-v1",
            "version": "1.0.0",
            "attestationKinds": ["brit"],
            "reduction": {
                "kind": "threshold-ladder",
                "tiers": [
                    { "level": "protocol",  "minCount": 20 },
                    { "level": "public",    "minCount": 10 },
                    { "level": "community", "minCount": 3  }
                ]
            }
        })
    }

    fn brit_records(subject: &str, count: usize) -> Vec<AttestationRecord> {
        (0..count)
            .map(|i| AttestationRecord {
                kind: "brit".into(),
                subject: subject.into(),
                issuer: format!("agent-{i}"),
                issued_at: "2026-01-01T00:00:00Z".into(),
                payload_json: "{}".into(),
            })
            .collect()
    }

    fn executor(
        spec_cid: &str,
        spec_body: Value,
        subject: &str,
        records: Vec<AttestationRecord>,
    ) -> AggregateAttestationsExecutor {
        let content_resolver = Box::new(EmbeddedContentNodeResolver::single(spec_cid, spec_body));
        let attestation_resolver = Box::new(StubAttestationResolver::single(subject, records));
        AggregateAttestationsExecutor::new(content_resolver, attestation_resolver)
    }

    // ─── COUNT reduction ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn count_hit_returns_true_level() {
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 7),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        let outcome = exec
            .execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await;
        assert!(outcome.is_ok());
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("true"));
        assert_eq!(out.get("count").and_then(|v| v.as_u64()), Some(7));
        assert_eq!(
            out.get("reduction_kind").and_then(|v| v.as_str()),
            Some("count")
        );
    }

    #[tokio::test]
    async fn count_miss_returns_false_level() {
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 2),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("false"));
        assert_eq!(out.get("count").and_then(|v| v.as_u64()), Some(2));
    }

    #[tokio::test]
    async fn count_exact_hit_is_edge_case() {
        // count == requiredCount exactly → edge_case true
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 5),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn count_one_below_is_edge_case() {
        // count == requiredCount - 1 → edge_case true
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 4),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
        // Still false because count < required
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("false"));
    }

    #[tokio::test]
    async fn count_one_above_is_edge_case() {
        // count == requiredCount + 1 → edge_case true
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 6),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("true"));
    }

    #[tokio::test]
    async fn count_far_above_not_edge_case() {
        // count >> requiredCount → edge_case false
        let exec = executor(
            "cid-count",
            count_spec_body(5),
            "subject-1",
            brit_records("subject-1", 20),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(false));
    }

    #[tokio::test]
    async fn count_zero_records_returns_false() {
        let exec = executor("cid-count", count_spec_body(5), "subject-1", vec![]);
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("false"));
        assert_eq!(out.get("count").and_then(|v| v.as_u64()), Some(0));
    }

    // ─── WEIGHTED-SUM reduction ───────────────────────────────────────────────

    fn mixed_records(subject: &str) -> Vec<AttestationRecord> {
        // 2 brit (×2.0 = 4.0) + 1 deep-engagement (×3.0 = 3.0) = 7.0
        vec![
            StubAttestationResolver::record("brit", subject),
            StubAttestationResolver::record("brit", subject),
            StubAttestationResolver::record("deep-engagement", subject),
        ]
    }

    #[tokio::test]
    async fn weighted_sum_hit_above_threshold() {
        // sum = 7.0 >= 5.0 → true
        let exec = executor(
            "cid-ws",
            weighted_sum_spec_body(5.0),
            "subject-1",
            mixed_records("subject-1"),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ws", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("true"));
        assert_eq!(
            out.get("reduction_kind").and_then(|v| v.as_str()),
            Some("weighted-sum")
        );
    }

    #[tokio::test]
    async fn weighted_sum_miss_below_threshold() {
        // sum = 7.0 < 20.0 → false
        let exec = executor(
            "cid-ws",
            weighted_sum_spec_body(20.0),
            "subject-1",
            mixed_records("subject-1"),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ws", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("false"));
    }

    #[tokio::test]
    async fn weighted_sum_within_epsilon_is_edge_case() {
        // sum = 7.0, threshold = 7.5 — diff = 0.5, epsilon = max(0.75, 1.0) = 1.0 → edge
        let exec = executor(
            "cid-ws",
            weighted_sum_spec_body(7.5),
            "subject-1",
            mixed_records("subject-1"),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ws", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn weighted_sum_far_from_threshold_not_edge_case() {
        // sum = 7.0, threshold = 50.0 — diff = 43.0 >> epsilon → not edge
        let exec = executor(
            "cid-ws",
            weighted_sum_spec_body(50.0),
            "subject-1",
            mixed_records("subject-1"),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ws", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(false));
    }

    // ─── THRESHOLD-LADDER reduction ───────────────────────────────────────────

    #[tokio::test]
    async fn ladder_protocol_tier_at_20() {
        let exec = executor(
            "cid-ladder",
            ladder_spec_body(),
            "subject-1",
            brit_records("subject-1", 20),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ladder", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("protocol"));
        // Exactly at the boundary → edge_case true
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn ladder_public_tier_between_10_and_20() {
        let exec = executor(
            "cid-ladder",
            ladder_spec_body(),
            "subject-1",
            brit_records("subject-1", 15),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ladder", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("public"));
        // count=15 != min_count=10 → not edge_case
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(false));
    }

    #[tokio::test]
    async fn ladder_community_tier_at_exactly_3() {
        let exec = executor(
            "cid-ladder",
            ladder_spec_body(),
            "subject-1",
            brit_records("subject-1", 3),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ladder", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("community"));
        // Exactly at tier boundary → edge_case true
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn ladder_below_lowest_tier_returns_null_level() {
        let exec = executor(
            "cid-ladder",
            ladder_spec_body(),
            "subject-1",
            brit_records("subject-1", 2),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ladder", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        // No tier met → level is null
        assert!(
            out.get("level").map(|v| v.is_null()).unwrap_or(false),
            "expected null level, got: {out}"
        );
        // No boundary crossed → edge_case false
        assert_eq!(out.get("edge_case").and_then(|v| v.as_bool()), Some(false));
    }

    #[tokio::test]
    async fn ladder_zero_records_below_all_tiers() {
        let exec = executor("cid-ladder", ladder_spec_body(), "subject-1", vec![]);
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-ladder", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert!(out.get("level").map(|v| v.is_null()).unwrap_or(false));
        assert_eq!(out.get("count").and_then(|v| v.as_u64()), Some(0));
    }

    #[tokio::test]
    async fn ladder_tiers_unsorted_input_still_picks_highest() {
        // Supply tiers in ascending order (lowest first) — executor should still
        // pick the highest qualified tier.
        let unsorted_spec = json!({
            "specName": "unsorted-ladder",
            "version": "1.0.0",
            "attestationKinds": ["brit"],
            "reduction": {
                "kind": "threshold-ladder",
                "tiers": [
                    { "level": "community", "minCount": 3  },
                    { "level": "public",    "minCount": 10 },
                    { "level": "protocol",  "minCount": 20 }
                ]
            }
        });
        let exec = executor(
            "cid-unsorted",
            unsorted_spec,
            "subject-1",
            brit_records("subject-1", 25),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("subject-1"));

        exec.execute(&step("cid-unsorted", "subject", "out"), &mut ctx, &event())
            .await
            .unwrap();
        let out = ctx.get("out").unwrap();
        assert_eq!(
            out.get("level").and_then(|v| v.as_str()),
            Some("protocol"),
            "highest qualifying tier must win regardless of input order"
        );
    }

    // ─── Output shape invariants ──────────────────────────────────────────────

    #[tokio::test]
    async fn output_always_has_all_fields() {
        // Verify the four required fields are always present.
        for (body, records) in [
            (count_spec_body(3), brit_records("s", 5)),
            (weighted_sum_spec_body(5.0), mixed_records("s")),
            (ladder_spec_body(), brit_records("s", 12)),
        ] {
            let exec = executor("cid-x", body, "s", records);
            let mut ctx = GateContext::new();
            ctx.insert("subject", json!("s"));

            exec.execute(&step("cid-x", "subject", "out"), &mut ctx, &event())
                .await
                .unwrap();
            let out = ctx.get("out").unwrap();
            assert!(out.get("level").is_some(), "missing 'level'");
            assert!(out.get("count").is_some(), "missing 'count'");
            assert!(out.get("edge_case").is_some(), "missing 'edge_case'");
            assert!(
                out.get("reduction_kind").is_some(),
                "missing 'reduction_kind'"
            );
        }
    }

    // ─── Error paths ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = AggregateAttestationsExecutor::new(
            Box::new(EmbeddedContentNodeResolver::single("c", json!({}))),
            Box::new(StubAttestationResolver::empty()),
        );
        let wrong_step = StepType::WisdomInvoke {
            params: WisdomInvokeParams {
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
            Ok(_) => panic!("expected Err for wrong step kind but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("wrong step kind"), "got: {msg}");
    }

    #[tokio::test]
    async fn missing_spec_cid_returns_error() {
        let exec = AggregateAttestationsExecutor::new(
            Box::new(EmbeddedContentNodeResolver::single(
                "known-cid",
                count_spec_body(5),
            )),
            Box::new(StubAttestationResolver::empty()),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("s"));

        let result = exec
            .execute(&step("unknown-cid", "subject", "out"), &mut ctx, &event())
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for unknown CID but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("unknown-cid"), "got: {msg}");
    }

    #[tokio::test]
    async fn missing_subject_key_returns_error() {
        let exec = executor("cid-count", count_spec_body(5), "s", vec![]);
        let mut ctx = GateContext::new();
        // subject key is NOT in context

        let result = exec
            .execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for missing subject key but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("subject_key") && msg.contains("subject"),
            "error must name the missing key, got: {msg}"
        );
    }

    #[tokio::test]
    async fn malformed_spec_body_returns_error() {
        let exec = AggregateAttestationsExecutor::new(
            Box::new(EmbeddedContentNodeResolver::single(
                "cid-bad",
                json!({"notASpec": true}),
            )),
            Box::new(StubAttestationResolver::empty()),
        );
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("s"));

        let result = exec
            .execute(&step("cid-bad", "subject", "out"), &mut ctx, &event())
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for malformed spec body but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
    }

    // ─── StubAttestationResolver ──────────────────────────────────────────────

    #[tokio::test]
    async fn stub_resolver_returns_fixtures_for_subject() {
        let records = brit_records("agent-1", 5);
        let resolver = StubAttestationResolver::single("agent-1", records.clone());
        let result = resolver
            .fetch("agent-1", &["brit".into()], None)
            .await
            .unwrap();
        assert_eq!(result.len(), 5);
    }

    #[tokio::test]
    async fn stub_resolver_filters_by_kind() {
        let records = vec![
            StubAttestationResolver::record("brit", "agent-1"),
            StubAttestationResolver::record("reach-endorsement", "agent-1"),
            StubAttestationResolver::record("deep-engagement", "agent-1"),
        ];
        let resolver = StubAttestationResolver::single("agent-1", records);
        let result = resolver
            .fetch("agent-1", &["brit".into()], None)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, "brit");
    }

    #[tokio::test]
    async fn stub_resolver_unknown_subject_returns_empty() {
        let resolver = StubAttestationResolver::single("agent-1", brit_records("agent-1", 3));
        let result = resolver
            .fetch("agent-unknown", &["brit".into()], None)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn stub_resolver_empty_returns_empty() {
        let resolver = StubAttestationResolver::empty();
        let result = resolver.fetch("any", &["brit".into()], None).await.unwrap();
        assert!(result.is_empty());
    }

    // ─── Kind filtering: only matching kinds count toward reduction ───────────

    #[tokio::test]
    async fn kind_filtering_excludes_non_spec_kinds() {
        // Spec only counts "brit"; attach "reach-endorsement" records too.
        // The attestation_resolver stub filters by kinds, so only brit records
        // should reach the reduction.
        let spec_body = json!({
            "specName": "brit-only-count",
            "version": "1.0.0",
            "attestationKinds": ["brit"],
            "reduction": { "kind": "count", "requiredCount": 3 }
        });
        let records = vec![
            StubAttestationResolver::record("brit", "s"),
            StubAttestationResolver::record("brit", "s"),
            StubAttestationResolver::record("reach-endorsement", "s"), // excluded by kind filter
        ];
        let exec = executor("cid-kind-filter", spec_body, "s", records);
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!("s"));

        exec.execute(
            &step("cid-kind-filter", "subject", "out"),
            &mut ctx,
            &event(),
        )
        .await
        .unwrap();
        let out = ctx.get("out").unwrap();
        // Only 2 brit records visible → count=2, miss (< 3)
        assert_eq!(out.get("count").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(out.get("level").and_then(|v| v.as_str()), Some("false"));
    }

    // ─── NullAttestationResolver ──────────────────────────────────────────────

    #[tokio::test]
    async fn null_resolver_returns_empty_vec() {
        let resolver = NullAttestationResolver;
        let result = resolver
            .fetch("any-subject", &["brit".into()], None)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // ─── Edge: subject key present but not a string ───────────────────────────

    #[tokio::test]
    async fn non_string_subject_key_returns_error() {
        let exec = executor("cid-count", count_spec_body(3), "s", vec![]);
        let mut ctx = GateContext::new();
        ctx.insert("subject", json!(42)); // number, not string

        let result = exec
            .execute(&step("cid-count", "subject", "out"), &mut ctx, &event())
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err for non-string subject key but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("subject"), "got: {msg}");
    }
}
