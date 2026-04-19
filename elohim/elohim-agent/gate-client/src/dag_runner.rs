//! DagRunner — orchestrates the universal-band DAG for every gate check.
//!
//! This module owns the singleton that `check()` delegates to.  It:
//!
//! 1. Constructs a [`DagInterpreter`] with all five Phase 2+3 executors registered.
//! 2. Parses the embedded universal-band v1 declaration once at startup.
//! 3. Exposes a `run()` method that accepts a [`RelationalImpactEvent`] plus a
//!    pre-populated initial [`GateContext`] and drives the DAG to completion.
//!
//! # Singleton strategy
//!
//! A `OnceLock<Arc<DagRunner>>` static is used so construction (YAML parse +
//! interpreter setup) happens exactly once per process, with no `Mutex` held on
//! the hot path.  Because `DagInterpreter` and all registered executors are
//! stateless once constructed, `DagRunner` is `Send + Sync`.
//!
//! # ContentNodeResolver injection (Phase 3 widening)
//!
//! The `MechanicalRulesetExecutor` needs a [`ContentNodeResolver`] to fetch
//! `gate-rules-declaration` bodies by CID.  Two construction paths exist:
//!
//! - **`DagRunner::new()`** — the default used by `global_runner()`.  Injects
//!   an empty [`EmbeddedContentNodeResolver`] which will return
//!   `GateError::DagExecution` if a `mechanical-ruleset` step is actually
//!   reached.  Sufficient for the universal-band DAG (which has no such steps).
//!
//! - **`DagRunner::with_content_resolver(resolver)`** — builds a runner with
//!   a caller-supplied resolver.  Use this for:
//!   - App-domain gates (discernment-gate etc.) that include `mechanical-ruleset`
//!     steps with real CIDs.
//!   - Unit tests against hand-constructed rule artifacts.
//!
//! For process-wide configuration before the singleton is initialized, call
//! [`configure_runner`] **before the first [`global_runner()`] call**. If the
//! singleton is already initialized when `configure_runner` is called, the call
//! is a no-op (the singleton is immutable once set).
//!
//! # ContextAssemble resolvers
//!
//! The universal-band DAG's `authorize` and `assemble-context` steps pull from
//! `source-chain` and `manifest` sources.  In DevContext no real resolver is
//! registered — the `ContextAssembleExecutor` routes unregistered sources to
//! Phase 3+ stubs which emit `Value::Null` with a `tracing::warn!`.  This is
//! intentional: the wisdom step receives null context values but still produces
//! its mock Allow, and the DAG completes successfully.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::json;
use tracing::{info_span, Instrument};

use crate::dag::attestation::DecisionAttestationBuilder;
use crate::dag::executor::StepKind;
use crate::dag::executors::{
    AggregateAttestationsExecutor, AttestationResolver, ContentNodeResolver,
    ContextAssembleExecutor, EmbeddedContentNodeResolver, EscalateToReviewExecutor,
    MechanicalRulesetExecutor, NullAttestationResolver, SynthesizeExecutor, WisdomInvokeExecutor,
};
use crate::dag::universal_band::{ACTIVE_UNIVERSAL_BAND_NAME, ACTIVE_UNIVERSAL_BAND_VERSION};
use crate::dag::{
    context::GateContext, discernment_gate::default_discernment_gate_declaration,
    interpreter::DagInterpreter, universal_band::default_universal_band_declaration,
    GateProcessDeclaration,
};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::space;
use crate::types::GateDecision;

// ─── DagRunner ────────────────────────────────────────────────────────────────

/// Orchestrates a single run of the universal-band DAG.
///
/// Constructed once via [`global_runner()`]; reused for every [`run()`] call.
pub struct DagRunner {
    interpreter: DagInterpreter,
    declaration: GateProcessDeclaration,
}

// DagRunner auto-derives Send + Sync because:
// - DagInterpreter holds HashMap<StepKind, Arc<dyn StepExecutor>>, where
//   `StepExecutor: Send + Sync` is declared at src/dag/executor.rs.
// - GateProcessDeclaration is plain-data Serialize/Deserialize.

impl DagRunner {
    /// Construct a runner with all Phase 2+3+5 executors registered.
    ///
    /// `MechanicalRulesetExecutor` is injected with an empty
    /// [`EmbeddedContentNodeResolver`] — if a `mechanical-ruleset` step is
    /// reached with a real CID, it will return `GateError::DagExecution`.
    /// `AggregateAttestationsExecutor` is injected with a
    /// [`NullAttestationResolver`] — returns empty Vec with tracing::warn.
    /// Sufficient for the universal-band DAG (no such steps).
    fn new() -> Self {
        Self::with_resolvers(
            Box::new(EmbeddedContentNodeResolver::new(HashMap::new())),
            Box::new(NullAttestationResolver),
        )
    }

    /// Construct a runner with a caller-supplied [`ContentNodeResolver`].
    ///
    /// Uses a [`NullAttestationResolver`] for the `AggregateAttestations` step.
    /// Use [`DagRunner::with_resolvers`] when the DAG also includes
    /// `aggregate-attestations` steps.
    ///
    /// The runner uses the **universal-band** declaration.  To run an
    /// app-domain gate (e.g., the discernment-gate), use
    /// [`DagRunner::with_content_resolver_and_declaration`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate_client::dag_runner::DagRunner;
    /// use gate_client::dag::executors::EmbeddedContentNodeResolver;
    /// use serde_json::json;
    ///
    /// let resolver = EmbeddedContentNodeResolver::single(
    ///     "bafkrei-seven-valence",
    ///     json!({ /* rules artifact body */ }),
    /// );
    /// let runner = DagRunner::with_content_resolver(Box::new(resolver));
    /// ```
    pub fn with_content_resolver(resolver: Box<dyn ContentNodeResolver>) -> Self {
        Self::with_resolvers(
            resolver,
            Box::new(NullAttestationResolver) as Box<dyn AttestationResolver>,
        )
    }

    /// Construct a runner with both a [`ContentNodeResolver`] and an
    /// [`AttestationResolver`].
    ///
    /// Use this when the DAG includes `aggregate-attestations` steps that need
    /// real subject→records resolution (e.g., reach-gate or unit tests with
    /// fixture attestations).
    ///
    /// The runner uses the **universal-band** declaration. To also supply a
    /// custom [`GateProcessDeclaration`], use
    /// [`DagRunner::with_resolvers_and_declaration`].
    pub fn with_resolvers(
        content_resolver: Box<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
    ) -> Self {
        let declaration = default_universal_band_declaration();
        let shared: Arc<dyn ContentNodeResolver> = Arc::from(content_resolver);
        Self::build_interpreter_and_wrap(shared, attestation_resolver, declaration)
    }

    /// Construct a runner with a caller-supplied [`ContentNodeResolver`] **and**
    /// a specific [`GateProcessDeclaration`].
    ///
    /// Uses a [`NullAttestationResolver`]. For gates that also need
    /// `aggregate-attestations`, use [`DagRunner::with_resolvers_and_declaration`].
    ///
    /// # Phase 3 usage
    ///
    /// `run_discernment_gate` uses this constructor to build a runner for the
    /// discernment-gate DAG with the seven-valence rules loaded into the resolver.
    pub fn with_content_resolver_and_declaration(
        resolver: Box<dyn ContentNodeResolver>,
        declaration: GateProcessDeclaration,
    ) -> Self {
        let shared: Arc<dyn ContentNodeResolver> = Arc::from(resolver);
        Self::build_interpreter_and_wrap(shared, Box::new(NullAttestationResolver), declaration)
    }

    /// Construct a runner with both resolvers **and** a specific
    /// [`GateProcessDeclaration`].
    ///
    /// Use this for app-domain gates (e.g., `reach-gate`) that include both
    /// `mechanical-ruleset` and `aggregate-attestations` steps.
    pub fn with_resolvers_and_declaration(
        content_resolver: Box<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
        declaration: GateProcessDeclaration,
    ) -> Self {
        let shared: Arc<dyn ContentNodeResolver> = Arc::from(content_resolver);
        Self::build_interpreter_and_wrap(shared, attestation_resolver, declaration)
    }

    /// Internal: build the interpreter with all Phase 2+3+5 executors and wrap it.
    ///
    /// Takes `Arc<dyn ContentNodeResolver>` so it can be shared between both
    /// `MechanicalRulesetExecutor` (spec lookup) and `AggregateAttestationsExecutor`
    /// (aggregation-spec lookup) without a second Box move.
    fn build_interpreter_and_wrap(
        content_resolver: Arc<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
        declaration: GateProcessDeclaration,
    ) -> Self {
        let mut interpreter = DagInterpreter::new();

        // ContextAssembleExecutor — no pre-registered pull resolvers.
        // Phase 3+ stubs handle elohim-storage / dht / source-chain / manifest
        // sources by returning Value::Null (with tracing::warn).
        interpreter.register(
            StepKind::ContextAssemble,
            Arc::new(ContextAssembleExecutor::new()),
        );
        interpreter.register(
            StepKind::WisdomInvoke,
            Arc::new(WisdomInvokeExecutor::new()),
        );
        // Phase 3: MechanicalRulesetExecutor with injected ContentNodeResolver.
        // The content_resolver is an Arc so we can share it with the
        // AggregateAttestationsExecutor below without a second Box move.
        interpreter.register(
            StepKind::MechanicalRuleset,
            Arc::new(MechanicalRulesetExecutor::new(Box::new(
                content_resolver.clone(),
            ))),
        );
        interpreter.register(StepKind::Synthesize, Arc::new(SynthesizeExecutor::new()));
        interpreter.register(
            StepKind::EscalateToReview,
            Arc::new(EscalateToReviewExecutor::new()),
        );
        // Phase 5: AggregateAttestationsExecutor — shares the same
        // ContentNodeResolver (via Arc clone) and takes its own
        // AttestationResolver for DHT graph queries.
        interpreter.register(
            StepKind::AggregateAttestations,
            Arc::new(AggregateAttestationsExecutor::new(
                Box::new(content_resolver),
                attestation_resolver,
            )),
        );

        Self {
            interpreter,
            declaration,
        }
    }

    /// Execute the universal-band DAG for the given event and return the
    /// final `GateDecision`.
    ///
    /// `initial_ctx` should already contain event-specific fields (e.g.,
    /// `eventKind`, `spaceType`) pre-populated by the caller.
    pub async fn run(
        &self,
        event: &RelationalImpactEvent,
        initial_ctx: GateContext,
    ) -> Result<GateDecision, GateError> {
        self.interpreter
            .run(&self.declaration, event, initial_ctx)
            .await
    }
}

// ─── Global singleton ─────────────────────────────────────────────────────────

static RUNNER: OnceLock<Arc<DagRunner>> = OnceLock::new();

/// Returned by [`configure_runner`] when the global singleton was already
/// initialized before the call arrived.
///
/// This happens when [`global_runner()`] (or [`run_with_tracing`]) was called
/// before `configure_runner`. The resolver that was supplied to `configure_runner`
/// was **not** installed. The caller can decide whether to treat this as a fatal
/// startup error or log-and-continue.
#[derive(Debug)]
pub struct AlreadyConfigured;

impl std::fmt::Display for AlreadyConfigured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "gate-client: configure_runner called after the global DagRunner \
             was already initialized — the supplied resolver was not installed"
        )
    }
}

/// Pre-configure the global singleton with a custom [`ContentNodeResolver`]
/// before first use.
///
/// **Must be called before the first [`global_runner()`] call** to take effect.
/// Returns `Ok(())` when the resolver is successfully installed, or
/// `Err(AlreadyConfigured)` when the singleton was already initialized and
/// the supplied resolver was silently discarded.
///
/// # Typical use
///
/// Call during process initialization (e.g., in `main()` or service startup)
/// to inject the production DHT-backed resolver before any gate checks run.
///
/// ```rust,ignore
/// use gate_client::dag_runner::{configure_runner, AlreadyConfigured};
/// use gate_client::dag::executors::EmbeddedContentNodeResolver;
///
/// // In production: inject DHT resolver here.
/// // In tests or dev-context: inject EmbeddedContentNodeResolver.
/// configure_runner(Box::new(my_dht_resolver)).expect("configure_runner must be called before first use");
/// ```
pub fn configure_runner(resolver: Box<dyn ContentNodeResolver>) -> Result<(), AlreadyConfigured> {
    RUNNER
        .set(Arc::new(DagRunner::with_content_resolver(resolver)))
        .map_err(|_| AlreadyConfigured)
}

/// Return the process-global [`DagRunner`], constructing it on first call.
///
/// Construction parses the embedded YAML and registers executors.  All
/// subsequent calls return a clone of the same `Arc` with no blocking.
///
/// If [`configure_runner`] was called before this, the pre-configured runner
/// is returned. Otherwise, a default runner with an empty
/// [`EmbeddedContentNodeResolver`] is constructed.
pub fn global_runner() -> Arc<DagRunner> {
    RUNNER.get_or_init(|| Arc::new(DagRunner::new())).clone()
}

// ─── Initial context builder ──────────────────────────────────────────────────

/// Build the initial [`GateContext`] from a [`RelationalImpactEvent`].
///
/// Pre-populates:
/// - `eventKind` — the kebab-case event discriminator.
/// - `spaceType` — the space type string from [`space::detect_from_event`].
/// - Event-specific primary fields (content_cid, declared_reach, etc.) using
///   consistent camelCase keys matching the YAML's context_keys declarations.
pub fn build_initial_context(event: &RelationalImpactEvent) -> GateContext {
    let mut ctx = GateContext::new();

    // ── Universal fields ──────────────────────────────────────────────────────
    ctx.insert("eventKind", json!(event.kind()));
    let space = space::detect_from_event(event);
    // Serialize SpaceType respecting its serde rename_all = "kebab-case".
    let space_type_json = serde_json::to_value(space.space_type)
        .unwrap_or_else(|_| json!(format!("{:?}", space.space_type)));
    ctx.insert("spaceType", space_type_json);

    // ── Event-specific primary fields ─────────────────────────────────────────
    match event {
        RelationalImpactEvent::ContentPublish {
            content_cid,
            declared_reach,
            author,
        } => {
            ctx.insert("contentCid", json!(content_cid));
            ctx.insert("declaredReach", json!(declared_reach));
            ctx.insert("author", json!(author));
        }
        RelationalImpactEvent::AttestationWrite {
            subject_hash,
            claim_kind,
            issuer,
        } => {
            ctx.insert("subjectHash", json!(subject_hash));
            ctx.insert("claimKind", json!(claim_kind));
            ctx.insert("issuer", json!(issuer));
        }
        RelationalImpactEvent::EconomicEventEmit {
            event_kind,
            provider,
            receiver,
            quantity,
        } => {
            ctx.insert("economicEventKind", json!(event_kind));
            ctx.insert("provider", json!(provider));
            ctx.insert("receiver", json!(receiver));
            ctx.insert("quantity", json!(quantity));
        }
        RelationalImpactEvent::PeerMessage {
            recipient,
            payload_kind,
        } => {
            ctx.insert("recipient", json!(recipient));
            ctx.insert("payloadKind", json!(payload_kind));
        }
        RelationalImpactEvent::SyncToPeers {
            manifest_cid,
            item_count,
        } => {
            ctx.insert("manifestCid", json!(manifest_cid));
            ctx.insert("itemCount", json!(item_count));
        }
        RelationalImpactEvent::AdviceSought {
            requester,
            summary_cid,
            topic,
        } => {
            ctx.insert("requester", json!(requester));
            ctx.insert("summaryCid", json!(summary_cid));
            ctx.insert("topic", json!(topic));
        }
        RelationalImpactEvent::CapabilityInvoke {
            capability,
            requester,
            request_id,
        } => {
            ctx.insert("capability", json!(capability));
            ctx.insert("requester", json!(requester));
            ctx.insert("requestId", json!(request_id));
        }
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space,
            artifact_ref,
        } => {
            ctx.insert("sourceSpace", json!(source_space));
            ctx.insert("artifactRef", json!(artifact_ref));
        }
    }

    ctx
}

/// The canonical CID pointer for the active universal-band.
///
/// Format: `"{name}@{version}"` — a stable string that identifies which
/// universal-band version ran above any given gate decision.
fn active_universal_band_cid_pointer() -> String {
    format!("{ACTIVE_UNIVERSAL_BAND_NAME}@{ACTIVE_UNIVERSAL_BAND_VERSION}")
}

/// Run the universal-band DAG for the given event with tracing spans.
///
/// This is the hot-path entry point called from `check()`.  It:
/// 1. Builds the initial context.
/// 2. Wraps the DAG run in an `info_span` for the gate check as a whole.
/// 3. Each individual DAG step will be traced inside the interpreter.
/// 4. Attaches a `decision_attestation_cid` to the returned decision (Phase 4).
pub async fn run_with_tracing(event: &RelationalImpactEvent) -> Result<GateDecision, GateError> {
    let runner = global_runner();
    let initial_ctx = build_initial_context(event);

    // Capture the context summary CID before the DAG consumes the context.
    let context_summary_cid = initial_ctx.to_summary_cid();

    let span = info_span!(
        "gate_check",
        event_kind = event.kind(),
        band = "universal-band-v1"
    );

    let mut decision = runner.run(event, initial_ctx).instrument(span).await?;

    // Phase 4: compute and attach the attestation CID.
    // The CID is computed in-process; the DHT write is Phase 6+ activation.
    let builder = DecisionAttestationBuilder::new(None, None);
    let (_, cid) = builder.build_with_cid(
        &decision,
        ACTIVE_UNIVERSAL_BAND_NAME,
        &format!("epr:gates:{ACTIVE_UNIVERSAL_BAND_NAME}"),
        event,
        context_summary_cid,
        active_universal_band_cid_pointer(),
    );
    decision.decision_attestation_cid = Some(cid);

    Ok(decision)
}

// ─── App-domain gate dispatch table ──────────────────────────────────────────

/// The dispatch table for app-domain gates in Phase 4 DevContext.
///
/// Each entry maps an event kind (kebab-case string) to the name of the
/// app-domain gate that should fire AFTER the universal band allows.
///
/// # Phase 5+ TODO
///
/// This table is hardcoded for Phase 4 DevContext.  In Phase 5, when multiple
/// app-domain gates ship (reach-gate, content-safety-gate, etc.), this will be
/// replaced by manifest-driven dispatch — reading the `gates` section of
/// `lamad/manifest.json` at runtime to determine which gates apply to each
/// event kind.  The table structure here makes it trivial to add a second entry:
///
/// ```ignore
/// const DOMAIN_GATE_DISPATCH: &[(&str, &str)] = &[
///     ("attestation-write", "discernment-gate-v1-mechanical"),
///     ("content-publish",   "reach-gate-v1"),  // Phase 5
/// ];
/// ```
const DOMAIN_GATE_DISPATCH: &[(&str, &str)] =
    &[("attestation-write", "discernment-gate-v1-mechanical")];

/// Determine which app-domain gate (if any) should run for the given event kind.
///
/// Returns `Some(gate_name)` if a domain gate should fire, `None` if the event
/// should be handled by the universal band only.
pub(crate) fn domain_gate_for_event(event: &RelationalImpactEvent) -> Option<&'static str> {
    let kind = event.kind();
    DOMAIN_GATE_DISPATCH
        .iter()
        .find_map(|(k, g)| if *k == kind { Some(*g) } else { None })
}

/// Run the universal-band DAG followed by the appropriate app-domain gate (if
/// any) for the given event.
///
/// This is the internal entry point used by `check()` and `check_blocking()`.
/// It embeds the full dispatch logic:
///
/// 1. Run the universal-band DAG.  If it does NOT return Allow, short-circuit.
/// 2. Determine whether an app-domain gate applies (Phase 4: AttestationWrite →
///    discernment-gate).
/// 3. If a domain gate applies, run it and return its decision.  Otherwise,
///    return the universal-band decision.
///
/// The optional `ctx_extension` parameter allows test callers (via the
/// `#[cfg(any(test, feature = "testing"))]` thread-local in `lib.rs`) to inject
/// pre-populated context fields (e.g., `moment`, `priorAttestations`) that would
/// normally be populated by the `assemble` step's real resolvers in Phase 5+.
pub(crate) async fn run_unified(
    event: &RelationalImpactEvent,
    ctx_extension: Option<GateContext>,
) -> Result<GateDecision, GateError> {
    // Step 1: universal-band must Allow first.
    let universal_decision = run_with_tracing(event).await?;
    if !universal_decision.is_allowed() {
        return Ok(universal_decision);
    }

    // Step 2: check if an app-domain gate applies.
    let Some(_gate_name) = domain_gate_for_event(event) else {
        // No domain gate → return the universal-band decision as-is.
        return Ok(universal_decision);
    };

    // Step 3: run the app-domain gate.  For Phase 4 DevContext the only gate
    // is discernment-gate-v1-mechanical (triggered by AttestationWrite).
    run_discernment_gate(event, ctx_extension.unwrap_or_default()).await
}

// ─── Discernment-gate dispatch ────────────────────────────────────────────────

/// Run both the universal-band DAG **and** the discernment-gate DAG for an
/// `AttestationWrite` event, returning the discernment-gate's `GateDecision`.
///
/// # Phase 3 contract
///
/// Phase 3 routes ALL `AttestationWrite` events through the discernment-gate.
/// Phase 4 folds this dispatch into `check()` via `run_unified()`.  This
/// function is retained as `pub(crate)` for:
/// - Per-rule integration tests in `tests/discernment_gate_integration.rs`
///   (which need pre-populated context injection).
/// - Explicit manual dispatch in unit tests that need fine-grained control.
///
/// # Context injection
///
/// The discernment-gate's `assemble` step pulls `moment` and `priorAttestations`
/// via Phase 3 stubs (returns `Value::Null` with a warn log).  For testing, the
/// caller MUST pre-populate these keys in `gate_context_extension` before calling
/// this function.  The extension map is merged into the initial context AFTER the
/// universal-band fields are set but BEFORE the discernment-gate DAG runs.
///
/// # Execution order
///
/// 1. Run the universal-band DAG (same as `check()`).  If it does NOT return
///    Allow, short-circuit and return that decision.
/// 2. Build the discernment-gate runner with the embedded seven-valence rules
///    pre-loaded into its `EmbeddedContentNodeResolver`.
/// 3. Merge `gate_context_extension` into the initial context.
/// 4. Run the discernment-gate DAG and return its decision.
pub(crate) async fn run_discernment_gate(
    event: &RelationalImpactEvent,
    gate_context_extension: GateContext,
) -> Result<GateDecision, GateError> {
    use crate::dag::seven_valence_rules::SEVEN_VALENCE_RULES_V1_BODY;
    use crate::dag::seven_valence_rules::SEVEN_VALENCE_RULES_V1_CID;

    // Step 1: universal-band must Allow first.
    let universal_decision = run_with_tracing(event).await?;
    if !universal_decision.is_allowed() {
        return Ok(universal_decision);
    }

    // Step 2: build the embedded resolver with the seven-valence rules.
    //
    // The rules JSON is embedded at compile time via `include_str!`.
    let rules_body: serde_json::Value =
        serde_json::from_str(SEVEN_VALENCE_RULES_V1_BODY).map_err(|e| {
            GateError::DagExecution(format!(
                "failed to parse embedded seven_valence_v1.json: {e}"
            ))
        })?;

    let mut cid_map = std::collections::HashMap::new();
    cid_map.insert(SEVEN_VALENCE_RULES_V1_CID.to_string(), rules_body);
    let resolver = Box::new(EmbeddedContentNodeResolver::new(cid_map));

    // Step 3: build initial context and pre-populate it with the extension.
    //
    // Phase 3 context injection strategy:
    //
    // The discernment-gate's `assemble` step pulls `moment` and `priorAttestations`
    // via Phase 3 stubs (returns null) because real elohim-storage and source-chain
    // resolvers are not yet wired.  If the DAG ran the `assemble` step, it would
    // overwrite any pre-populated context keys with null — defeating the purpose
    // of the extension.
    //
    // To avoid this, `run_discernment_gate` skips the `assemble` step by using a
    // modified declaration whose entrypoint is `rules` (not `assemble`).  The
    // caller supplies the moment and priorAttestations context directly via the
    // `gate_context_extension` parameter.
    //
    // Phase 4 will wire real resolvers into ContextAssembleExecutor so `assemble`
    // can pull from DHT and elohim-storage.  At that point, the DAG will run from
    // `assemble` again and the context extension parameter will be removed.
    let mut ctx = build_initial_context(event);
    ctx.merge(gate_context_extension);

    // The `synthesize` step's MintAttestation side effect requires `momentEntryHash`
    // to be in the context.  This would normally be populated by the `assemble`
    // step pulling the entry hash from the source chain.  Since Phase 3 skips the
    // `assemble` step, we derive it from the event's `subject_hash` field — which
    // carries the entry hash of the experience-moment being attested.
    //
    // Phase 4: once the `assemble` step is wired to the real source-chain resolver,
    // this explicit insertion will be removed.
    if let RelationalImpactEvent::AttestationWrite { subject_hash, .. } = event {
        if !ctx.contains("momentEntryHash") {
            ctx.insert("momentEntryHash", json!(subject_hash));
        }
    }

    // Capture the context summary CID before the DAG consumes the context.
    let context_summary_cid = ctx.to_summary_cid();

    // Build the discernment-gate declaration with entrypoint set to `rules`.
    let mut effective_declaration = default_discernment_gate_declaration();
    effective_declaration.dag.entrypoint = "rules".to_string();

    let discernment_runner =
        DagRunner::with_content_resolver_and_declaration(resolver, effective_declaration);

    // Step 4: run the discernment-gate DAG starting at `rules`.
    let span = info_span!(
        "gate_check",
        event_kind = event.kind(),
        band = "discernment-gate-v1-mechanical"
    );
    let mut decision = discernment_runner.run(event, ctx).instrument(span).await?;

    // Phase 4: compute and attach the attestation CID for the discernment-gate decision.
    // The CID is computed in-process; the DHT write is Phase 6+ activation.
    use crate::dag::discernment_gate::DISCERNMENT_GATE_V1_CID;
    let builder = DecisionAttestationBuilder::new(None, None);
    let (_, cid) = builder.build_with_cid(
        &decision,
        "discernment-gate-v1-mechanical",
        DISCERNMENT_GATE_V1_CID,
        event,
        context_summary_cid,
        active_universal_band_cid_pointer(),
    );
    decision.decision_attestation_cid = Some(cid);

    Ok(decision)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RelationalImpactEvent;
    use crate::types::GateStatus;

    fn content_publish_event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "bafytest".into(),
            declared_reach: "public".into(),
            author: "agent-test".into(),
        }
    }

    // ─── global_runner() returns the same Arc on repeated calls ───────────────

    #[test]
    fn global_runner_is_a_singleton() {
        let r1 = global_runner();
        let r2 = global_runner();
        // Both Arcs point to the same allocation.
        assert!(
            Arc::ptr_eq(&r1, &r2),
            "global_runner must return the same Arc"
        );
    }

    // ─── build_initial_context populates universal fields ─────────────────────

    #[test]
    fn initial_context_has_event_kind() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert_eq!(
            ctx.get("eventKind"),
            Some(&serde_json::json!("content-publish"))
        );
    }

    #[test]
    fn initial_context_has_space_type() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert!(
            ctx.get("spaceType").is_some(),
            "spaceType must be in initial context"
        );
    }

    #[test]
    fn initial_context_has_content_cid_for_content_publish() {
        let event = content_publish_event();
        let ctx = build_initial_context(&event);
        assert_eq!(ctx.get("contentCid"), Some(&serde_json::json!("bafytest")));
    }

    // ─── DagRunner::run produces an Allow decision ───────────────────────────

    #[tokio::test]
    async fn dag_runner_run_produces_allow() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            decision.is_allowed(),
            "universal-band must Allow in DevContext"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_decision_is_not_exempt() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            !decision.is_exempt(),
            "DAG decision must not be exempt (only the short-circuit path is exempt)"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_phase_is_dev_context() {
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert!(
            decision.phase.is_dev_context(),
            "DevContext phase must be set on DAG-produced decisions"
        );
    }

    #[tokio::test]
    async fn dag_runner_run_reasoning_phase_note_is_wisdom_sourced() {
        // The allow-with-wisdom builder propagates phase_note = "wisdom-sourced"
        // from the wisdom mock output.  This proves the real DAG ran rather than
        // the Phase 0 stub (which uses ConstitutionalReasoningSummary::mocked()).
        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();
        assert_eq!(
            decision.reasoning.phase_note, "wisdom-sourced",
            "phase_note must be 'wisdom-sourced' to prove the real DAG ran; \
             got: {:?}",
            decision.reasoning.phase_note
        );
    }

    // ─── run_with_tracing wraps in a span and returns Allow ──────────────────

    #[tokio::test]
    async fn run_with_tracing_returns_allow() {
        let event = content_publish_event();
        let decision = run_with_tracing(&event).await.unwrap();
        assert!(
            matches!(decision.status, GateStatus::Allow { .. }),
            "run_with_tracing must return Allow in DevContext"
        );
    }

    // ─── All 8 event variants produce Allow ──────────────────────────────────

    #[tokio::test]
    async fn all_event_variants_produce_allow_via_dag() {
        let events = vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "cid-1".into(),
                declared_reach: "public".into(),
                author: "agent-a".into(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "hash-1".into(),
                claim_kind: "brit".into(),
                issuer: "agent-b".into(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".into(),
                provider: "agent-c".into(),
                receiver: "agent-d".into(),
                quantity: "5".into(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "agent-e".into(),
                payload_kind: "handshake".into(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "manifest-cid".into(),
                item_count: 3,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "agent-f".into(),
                summary_cid: "sum-cid".into(),
                topic: "conflict".into(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".into(),
                requester: "agent-g".into(),
                request_id: "req-1".into(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "private-draft".into(),
                artifact_ref: "artifact-cid".into(),
            },
        ];

        let runner = DagRunner::new();
        for event in &events {
            let kind = event.kind();
            let ctx = build_initial_context(event);
            let decision = runner
                .run(event, ctx)
                .await
                .unwrap_or_else(|e| panic!("DAG run failed for {kind}: {e}"));
            assert!(decision.is_allowed(), "DAG must Allow {kind} in DevContext");
        }
    }
}
