//! DagRunner — orchestrates the universal-band DAG for every gate check.
//!
//! This module owns the singleton that `check()` delegates to.  It:
//!
//! 1. Constructs a [`DagInterpreter`] with all five Phase 2+3+5 executors registered.
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
//! # App-domain gate dispatch (Phase 6)
//!
//! After the universal-band allows, [`domain_gates_for_event`] inspects the event
//! variant and its fields and returns an **ordered Vec** of gate names to run.
//! Multiple gates may apply per event; they run in declaration order with
//! short-circuit on Decline or Escalate (spec §3.3):
//!
//! | Event variant                                              | Gates (in order)                                        |
//! |------------------------------------------------------------|--------------------------------------------------------|
//! | `ContentPublish`                                           | `["content-safety-gate-v1"]`                           |
//! | `AttestationWrite`                                         | `["content-safety-gate-v1", "discernment-gate-v1-mechanical"]` |
//! | `PeerMessage`                                              | `["content-safety-gate-v1"]`                           |
//! | `CapabilityInvoke { capability: "reach-negotiation", .. }` | `["reach-gate-v1"]`                                    |
//! | All other variants                                         | `[]` (universal-band decision returned as-is)          |
//!
//! All three gates are loaded eagerly at [`DagRunner`] construction via
//! [`GateRegistry`] so the hot path does no parsing.
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

use async_trait::async_trait;
use serde_json::json;
use tracing::{info_span, Instrument};

use crate::dag::attestation::DecisionAttestationBuilder;
use crate::dag::executor::StepKind;
use crate::dag::executors::aggregate_attestations::AttestationRecord;
use crate::dag::executors::{
    AggregateAttestationsExecutor, AttestationResolver, CapabilityDispatcher, ContentNodeResolver,
    ContextAssembleExecutor, EmbeddedContentNodeResolver, EscalateToReviewExecutor,
    MechanicalRulesetExecutor, MockCapabilityDispatcher, NullAttestationResolver,
    SkillInvokeExecutor, SynthesizeExecutor, WisdomInvokeExecutor,
};
use crate::dag::universal_band::{ACTIVE_UNIVERSAL_BAND_NAME, ACTIVE_UNIVERSAL_BAND_VERSION};
use crate::dag::{
    content_safety_gate::default_content_safety_gate_declaration, context::GateContext,
    discernment_gate::default_discernment_gate_declaration, interpreter::DagInterpreter,
    reach_gate::default_reach_gate_declaration, universal_band::default_universal_band_declaration,
    GateProcessDeclaration,
};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::space;
use crate::transport::WisdomTransport;
use crate::types::GateDecision;

// ─── GateRegistry ─────────────────────────────────────────────────────────────

/// A registry entry pairing a gate's [`GateProcessDeclaration`] with its
/// pre-built [`ContentNodeResolver`] (for `mechanical-ruleset` steps) and
/// the CID string used in attestation building.
struct GateEntry {
    declaration: GateProcessDeclaration,
    /// The EmbeddedContentNodeResolver pre-loaded with all CID→body pairs this
    /// gate needs.  Stored as Arc so the interpreter can clone cheaply.
    content_resolver: Arc<dyn ContentNodeResolver>,
    gate_cid: &'static str,
}

/// Eager registry of all known app-domain gates.
///
/// Built once at [`DagRunner`] construction.  [`run_app_domain_gate`] looks up
/// each gate name returned by [`domain_gates_for_event`] to find the right entry.
struct GateRegistry {
    gates: HashMap<&'static str, GateEntry>,
}

impl GateRegistry {
    /// Build the registry, eagerly loading discernment-gate, reach-gate, and
    /// content-safety-gate.
    fn build() -> Self {
        use crate::dag::content_safety_gate::CONTENT_SAFETY_GATE_V1_CID;
        use crate::dag::discernment_gate::DISCERNMENT_GATE_V1_CID;
        use crate::dag::reach_aggregation::{REACH_AGGREGATION_V1_BODY, REACH_AGGREGATION_V1_CID};
        use crate::dag::reach_gate::REACH_GATE_V1_CID;
        use crate::dag::seven_valence_rules::{
            SEVEN_VALENCE_RULES_V1_BODY, SEVEN_VALENCE_RULES_V1_CID,
        };

        let mut gates: HashMap<&'static str, GateEntry> = HashMap::new();

        // ── discernment-gate-v1-mechanical ────────────────────────────────────
        {
            let rules_body: serde_json::Value = serde_json::from_str(SEVEN_VALENCE_RULES_V1_BODY)
                .expect("embedded seven_valence_v1.json must parse at startup");
            let mut cid_map = HashMap::new();
            cid_map.insert(SEVEN_VALENCE_RULES_V1_CID.to_string(), rules_body);
            let resolver =
                Arc::new(EmbeddedContentNodeResolver::new(cid_map)) as Arc<dyn ContentNodeResolver>;

            let mut decl = default_discernment_gate_declaration();
            // Phase 3/4 DevContext: skip `assemble`; start at `rules` so the
            // pre-injected context fields are not overwritten by Phase 3 stubs.
            decl.dag.entrypoint = "rules".to_string();

            gates.insert(
                "discernment-gate-v1-mechanical",
                GateEntry {
                    declaration: decl,
                    content_resolver: resolver,
                    gate_cid: DISCERNMENT_GATE_V1_CID,
                },
            );
        }

        // ── reach-gate-v1 ─────────────────────────────────────────────────────
        {
            let agg_body: serde_json::Value = serde_json::from_str(REACH_AGGREGATION_V1_BODY)
                .expect("embedded reach_aggregation_v1.json must parse at startup");
            let mut cid_map = HashMap::new();
            cid_map.insert(REACH_AGGREGATION_V1_CID.to_string(), agg_body);
            let resolver =
                Arc::new(EmbeddedContentNodeResolver::new(cid_map)) as Arc<dyn ContentNodeResolver>;

            // Phase 5 DevContext: skip `assemble`; start at `aggregate` so
            // pre-injected context fields (subject) are not overwritten by the
            // Phase 3 stub that returns Value::Null for the "event" source.
            // Parallel to the discernment-gate's entrypoint override above.
            let mut decl = default_reach_gate_declaration();
            decl.dag.entrypoint = "aggregate".to_string();

            gates.insert(
                "reach-gate-v1",
                GateEntry {
                    declaration: decl,
                    content_resolver: resolver,
                    gate_cid: REACH_GATE_V1_CID,
                },
            );
        }

        // ── content-safety-gate-v1 ────────────────────────────────────────────
        // Wide governance gate — fires on ContentPublish, AttestationWrite, and
        // PeerMessage.  No CID-addressed rules or aggregation spec needed; the
        // wisdom-safety step is backed by WisdomInvokeExecutor (mock Allow in
        // DevContext; real ContentSafetyReview in Phase 7+ activation).
        {
            // Empty content resolver — content-safety-gate has no mechanical-ruleset
            // or aggregate-attestations steps that need CID lookups.
            let resolver = Arc::new(EmbeddedContentNodeResolver::new(HashMap::new()))
                as Arc<dyn ContentNodeResolver>;

            let decl = default_content_safety_gate_declaration();

            gates.insert(
                "content-safety-gate-v1",
                GateEntry {
                    declaration: decl,
                    content_resolver: resolver,
                    gate_cid: CONTENT_SAFETY_GATE_V1_CID,
                },
            );
        }

        Self { gates }
    }

    fn get(&self, gate_name: &str) -> Option<&GateEntry> {
        self.gates.get(gate_name)
    }
}

// ─── DagRunner ────────────────────────────────────────────────────────────────

/// Orchestrates a single run of the universal-band DAG.
///
/// Constructed once via [`global_runner()`]; reused for every [`run()`] call.
///
/// Phase 6: also owns a [`GateRegistry`] with eagerly-loaded app-domain gate
/// declarations for discernment-gate, reach-gate, and content-safety-gate.
///
/// Phase 8: holds a `WisdomTransport` that determines whether wisdom-invoke
/// steps use the hardcoded mock or the in-process `elohim_agent::wisdom`
/// function.  The default (`Mock`) preserves all existing DevContext behaviour.
pub struct DagRunner {
    interpreter: DagInterpreter,
    declaration: GateProcessDeclaration,
    /// Eager registry of app-domain gates (content-safety-gate + discernment-gate + reach-gate).
    registry: GateRegistry,
    /// How wisdom-invoke steps reach the wisdom engine.
    wisdom_transport: WisdomTransport,
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

    /// Internal: build the interpreter with all Phase 2+3+5+6 executors and wrap it.
    ///
    /// Takes `Arc<dyn ContentNodeResolver>` so it can be shared between both
    /// `MechanicalRulesetExecutor` (spec lookup) and `AggregateAttestationsExecutor`
    /// (aggregation-spec lookup) without a second Box move.
    ///
    /// `capability_dispatcher` defaults to `MockCapabilityDispatcher` when `None`.
    fn build_interpreter_and_wrap(
        content_resolver: Arc<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
        declaration: GateProcessDeclaration,
    ) -> Self {
        Self::build_interpreter_and_wrap_with_dispatcher(
            content_resolver,
            attestation_resolver,
            declaration,
            None,
        )
    }

    /// Like `build_interpreter_and_wrap` but accepts an optional
    /// `CapabilityDispatcher` override for the `SkillInvoke` step.
    ///
    /// `None` → `MockCapabilityDispatcher` (DevContext / tests).
    /// `Some(d)` → the supplied dispatcher (Phase 7+ activation).
    fn build_interpreter_and_wrap_with_dispatcher(
        content_resolver: Arc<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
        declaration: GateProcessDeclaration,
        capability_dispatcher: Option<Arc<dyn CapabilityDispatcher>>,
    ) -> Self {
        Self::build_interpreter_and_wrap_with_dispatcher_and_wisdom(
            content_resolver,
            attestation_resolver,
            declaration,
            capability_dispatcher,
            WisdomTransport::Mock,
        )
    }

    /// Full internal builder — accepts dispatcher and `WisdomTransport`.
    ///
    /// All other constructors delegate here.  The `wisdom_transport` determines
    /// whether `WisdomInvokeExecutor` uses the hardcoded mock or the in-process
    /// `elohim_agent::wisdom::invoke_wisdom` function.
    fn build_interpreter_and_wrap_with_dispatcher_and_wisdom(
        content_resolver: Arc<dyn ContentNodeResolver>,
        attestation_resolver: Box<dyn AttestationResolver>,
        declaration: GateProcessDeclaration,
        capability_dispatcher: Option<Arc<dyn CapabilityDispatcher>>,
        wisdom_transport: WisdomTransport,
    ) -> Self {
        let mut interpreter = DagInterpreter::new();

        // ContextAssembleExecutor — no pre-registered pull resolvers.
        // Phase 3+ stubs handle elohim-storage / dht / source-chain / manifest
        // sources by returning Value::Null (with tracing::warn).
        interpreter.register(
            StepKind::ContextAssemble,
            Arc::new(ContextAssembleExecutor::new()),
        );
        // WisdomInvokeExecutor — transport determines mock vs in-process LLM call.
        // Phase::ElohimActive is observed from the actual call outcome, never flagged.
        interpreter.register(
            StepKind::WisdomInvoke,
            Arc::new(WisdomInvokeExecutor::new(wisdom_transport.clone())),
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
        // Phase 6: SkillInvokeExecutor — dispatches to a named ElohimCapability.
        // MockCapabilityDispatcher is the default; Phase 7+ activation injects
        // the real capability handler via `with_capability_dispatcher`.
        let dispatcher = capability_dispatcher
            .unwrap_or_else(|| Arc::new(MockCapabilityDispatcher) as Arc<dyn CapabilityDispatcher>);
        interpreter.register(
            StepKind::SkillInvoke,
            Arc::new(SkillInvokeExecutor::new(dispatcher)),
        );

        Self {
            interpreter,
            declaration,
            registry: GateRegistry::build(),
            wisdom_transport,
        }
    }

    /// Builder: replace the `MockCapabilityDispatcher` with a real dispatcher.
    ///
    /// Use this at process startup (Phase 7+) to wire in the elohim-agent-service
    /// capability handler before the first gate check runs.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate_client::dag_runner::DagRunner;
    ///
    /// let runner = DagRunner::new().with_capability_dispatcher(Arc::new(my_dispatcher));
    /// ```
    pub fn with_capability_dispatcher(self, dispatcher: Arc<dyn CapabilityDispatcher>) -> Self {
        // Re-build the interpreter to swap in the new dispatcher.  The registry
        // is rebuilt too; GateRegistry::build() is cheap (just HashMap + YAML).
        // Preserve the existing wisdom_transport so callers who set InProcess
        // don't lose it when swapping the dispatcher.
        Self::build_interpreter_and_wrap_with_dispatcher_and_wisdom(
            // We cannot recover the content_resolver from the existing interpreter,
            // so default to an empty EmbeddedContentNodeResolver.  Callers who need
            // both a real content resolver AND a real capability dispatcher should
            // use the full constructor path instead.
            Arc::new(EmbeddedContentNodeResolver::new(
                std::collections::HashMap::new(),
            )),
            Box::new(NullAttestationResolver),
            self.declaration,
            Some(dispatcher),
            self.wisdom_transport,
        )
    }

    /// Builder: set the `WisdomTransport` for this runner.
    ///
    /// Use `WisdomTransport::InProcess` at process startup (Phase 8+) to enable
    /// real LLM inference via `elohim_agent::wisdom::invoke_wisdom`.
    ///
    /// The `Phase::ElohimActive` marker is **observed from the actual call outcome**,
    /// not from a config flag.  If no API key is set, the service returns
    /// `phase: dev-context` regardless of this setting.
    ///
    /// # Example (Phase 8 activation)
    ///
    /// ```rust,ignore
    /// use gate_client::dag_runner::DagRunner;
    /// use gate_client::transport::WisdomTransport;
    ///
    /// let runner = DagRunner::new().with_wisdom_transport(WisdomTransport::InProcess);
    /// ```
    pub fn with_wisdom_transport(self, wisdom_transport: WisdomTransport) -> Self {
        Self::build_interpreter_and_wrap_with_dispatcher_and_wisdom(
            Arc::new(EmbeddedContentNodeResolver::new(
                std::collections::HashMap::new(),
            )),
            Box::new(NullAttestationResolver),
            self.declaration,
            None,
            wisdom_transport,
        )
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

// ─── App-domain gate dispatch ─────────────────────────────────────────────────

/// Determine which app-domain gates should run (in order) for the given event.
///
/// Returns a `Vec` of gate names.  An empty Vec means the event is handled by
/// the universal band only.  Multiple names means the gates run in the returned
/// order; any Decline or Escalate short-circuits the remainder (spec §3.3).
///
/// # Dispatch rules (Phase 6)
///
/// | Event variant                                              | Gates (ordered)                                          |
/// |------------------------------------------------------------|----------------------------------------------------------|
/// | `ContentPublish`                                           | `["content-safety-gate-v1"]`                             |
/// | `AttestationWrite`                                         | `["content-safety-gate-v1", "discernment-gate-v1-mechanical"]` |
/// | `PeerMessage`                                              | `["content-safety-gate-v1"]`                             |
/// | `CapabilityInvoke { capability: "reach-negotiation", .. }` | `["reach-gate-v1"]`                                      |
/// | All other variants                                         | `[]`                                                     |
///
/// Governance-level gates (content-safety) run before more specialised domain
/// gates (discernment, reach).
pub(crate) fn domain_gates_for_event(event: &RelationalImpactEvent) -> Vec<&'static str> {
    match event {
        RelationalImpactEvent::ContentPublish { .. } => vec!["content-safety-gate-v1"],
        RelationalImpactEvent::AttestationWrite { .. } => {
            vec!["content-safety-gate-v1", "discernment-gate-v1-mechanical"]
        }
        RelationalImpactEvent::PeerMessage { .. } => vec!["content-safety-gate-v1"],
        RelationalImpactEvent::CapabilityInvoke { capability, .. }
            if capability == "reach-negotiation" =>
        {
            vec!["reach-gate-v1"]
        }
        _ => vec![],
    }
}

/// Run the universal-band DAG followed by all applicable app-domain gates for
/// the given event.
///
/// This is the internal entry point used by `check()` and `check_blocking()`.
/// It embeds the full Phase 6 dispatch logic:
///
/// 1. Run the universal-band DAG.  If it does NOT return Allow, short-circuit.
/// 2. Determine which app-domain gates apply via [`domain_gates_for_event`].
///    Gates are run in declaration order (spec §3.3: governance-level first):
///    - `ContentPublish` → `["content-safety-gate-v1"]`
///    - `AttestationWrite` → `["content-safety-gate-v1", "discernment-gate-v1-mechanical"]`
///    - `PeerMessage` → `["content-safety-gate-v1"]`
///    - `CapabilityInvoke{capability:"reach-negotiation"}` → `["reach-gate-v1"]`
///    - Other events → `[]` (universal-band decision returned as-is)
/// 3. For each gate in order:
///    - Run the gate.
///    - If result is Decline or Escalate: short-circuit immediately; discard any
///      side effects accumulated from prior gates in this run.
///    - If result is Verdict: keep the decision but continue to the next gate
///      (subsequent gates may further refine the outcome).
///    - If result is Allow: continue to the next gate.
/// 4. Return the last gate's decision; or the universal-band Allow if no gates
///    matched.
///
/// # Side effect accumulation
///
/// Side effects accumulate across gates UNLESS a later gate Declines/Escalates.
/// If gate A produces a side effect (e.g., MintAttestation) and gate B then
/// Declines, gate B's decision is returned and gate A's side effects are
/// discarded — Decline means "don't proceed", so no side effects should fire.
///
/// In Phase 6 DevContext, content-safety-gate always returns Allow (mock),
/// so existing Phase 3/4/5 test paths continue to receive their expected
/// Verdict / Allow outcomes unchanged.
///
/// The keyed thread-local (see `lib.rs`: `GATE_CTXS`) allows test callers to
/// inject pre-populated context fields per gate name without changing this
/// function's signature.  In production builds, the map is always empty and the
/// context extension defaults to an empty `GateContext`.
///
/// `attestation_resolver_overrides`: a map from gate name → resolver, consumed
/// once per call.  Gates not present in the map fall back to
/// `NullAttestationResolver`.  In production builds the map is always empty.
pub(crate) async fn run_unified(
    event: &RelationalImpactEvent,
    ctx_extensions: HashMap<String, GateContext>,
    attestation_resolver_overrides: HashMap<String, Arc<dyn AttestationResolver>>,
) -> Result<GateDecision, GateError> {
    // Step 1: universal-band must Allow first.
    let universal_decision = run_with_tracing(event).await?;
    if !universal_decision.is_allowed() {
        return Ok(universal_decision);
    }

    // Step 2: collect the ordered list of applicable domain gates.
    let gate_names = domain_gates_for_event(event);
    if gate_names.is_empty() {
        // No domain gates → return the universal-band decision as-is.
        return Ok(universal_decision);
    }

    // Step 3: run gates in order with short-circuit on Decline / Escalate.
    //
    // `last_decision` tracks the most recent non-short-circuit gate decision.
    // Accumulated side effects from prior gates are discarded if any gate
    // Declines or Escalates.
    let mut last_decision: Option<GateDecision> = None;

    for gate_name in gate_names {
        let ctx_extension = ctx_extensions.get(gate_name).cloned().unwrap_or_default();
        let attestation_resolver_override = attestation_resolver_overrides.get(gate_name).cloned();

        let gate_decision = run_app_domain_gate(
            gate_name,
            event,
            ctx_extension,
            attestation_resolver_override,
        )
        .await?;

        match &gate_decision.status {
            // Decline or Escalate: short-circuit; discard prior side effects.
            crate::types::GateStatus::Decline { .. }
            | crate::types::GateStatus::Escalate { .. } => {
                return Ok(gate_decision);
            }
            // Verdict or Allow: store as the current best decision; continue.
            crate::types::GateStatus::Verdict(_) | crate::types::GateStatus::Allow { .. } => {
                last_decision = Some(gate_decision);
            }
        }
    }

    // Step 4: return the last gate's decision (or universal-band Allow if no
    // gates ran — which cannot happen here since gate_names was non-empty).
    Ok(last_decision.unwrap_or(universal_decision))
}

// ─── Arc<dyn AttestationResolver> adapter ────────────────────────────────────

/// Thin newtype that wraps `Arc<dyn AttestationResolver>` and implements the
/// trait by delegating, allowing the Arc to be erased into a `Box<dyn AttestationResolver>`.
///
/// Needed because `Arc<dyn Trait>` does not automatically coerce to
/// `Box<dyn Trait>` when the trait has async methods (via `async_trait`).
struct ArcAttestationResolverAdapter(Arc<dyn AttestationResolver>);

#[async_trait]
impl AttestationResolver for ArcAttestationResolverAdapter {
    async fn fetch(
        &self,
        subject: &str,
        kinds: &[String],
        time_window: Option<&str>,
    ) -> Result<Vec<AttestationRecord>, GateError> {
        self.0.fetch(subject, kinds, time_window).await
    }
}

// ─── Generic app-domain gate runner ──────────────────────────────────────────

/// Run a named app-domain gate for the given event.
///
/// Looks up the gate declaration in the process-global [`DagRunner`]'s registry
/// and executes the gate DAG with the supplied context extension.  Both
/// `discernment-gate-v1-mechanical` and `reach-gate-v1` delegate through here.
///
/// `run_discernment_gate` is a thin convenience shim over this function that
/// pre-populates context fields specific to the discernment-gate (`momentEntryHash`).
///
/// `attestation_resolver_override`: when `Some`, replaces the default
/// [`NullAttestationResolver`] for the one-off gate runner built here.  This
/// allows test builds to inject fixture attestation records for reach-gate E2E
/// tests without modifying the process-global singleton.  Phase 6+ will replace
/// this with a DHT-backed resolver injected at process startup.
///
/// # Errors
///
/// Returns `GateError::DagExecution` if `gate_name` is not in the registry.
pub(crate) async fn run_app_domain_gate(
    gate_name: &str,
    event: &RelationalImpactEvent,
    gate_context_extension: GateContext,
    attestation_resolver_override: Option<Arc<dyn AttestationResolver>>,
) -> Result<GateDecision, GateError> {
    let runner = global_runner();
    let entry = runner.registry.get(gate_name).ok_or_else(|| {
        GateError::DagExecution(format!(
            "no gate entry for '{gate_name}' in GateRegistry; \
             known gates: content-safety-gate-v1, discernment-gate-v1-mechanical, reach-gate-v1"
        ))
    })?;

    // Build initial context and merge the caller-supplied extension.
    let mut ctx = build_initial_context(event);

    // discernment-gate-specific: inject `momentEntryHash` from `subject_hash`
    // so the `synthesize` step's MintAttestation side effect has the entry hash.
    // This would normally come from the `assemble` step's source-chain resolver.
    // Retained until Phase 6+ wires real resolvers.
    if gate_name == "discernment-gate-v1-mechanical" {
        if let RelationalImpactEvent::AttestationWrite { subject_hash, .. } = event {
            if !ctx.contains("momentEntryHash") {
                ctx.insert("momentEntryHash", json!(subject_hash));
            }
        }
    }

    ctx.merge(gate_context_extension);

    let context_summary_cid = ctx.to_summary_cid();

    // Choose the attestation resolver: injected override takes precedence over
    // the default NullAttestationResolver.
    let attestation_resolver: Box<dyn AttestationResolver> =
        if let Some(arc_resolver) = attestation_resolver_override {
            // Wrap the Arc in a thin Box adapter so we satisfy the Box<dyn> contract.
            Box::new(ArcAttestationResolverAdapter(arc_resolver))
        } else {
            Box::new(NullAttestationResolver)
        };

    // Build a one-off DagRunner for this gate, re-using the registry's resolver.
    // The registry's declaration has the right entrypoint already set.
    let gate_runner = DagRunner::build_interpreter_and_wrap(
        entry.content_resolver.clone(),
        attestation_resolver,
        entry.declaration.clone(),
    );

    let span = info_span!("gate_check", event_kind = event.kind(), band = gate_name);
    let mut decision = gate_runner.run(event, ctx).instrument(span).await?;

    // Compute and attach the attestation CID.
    let builder = DecisionAttestationBuilder::new(None, None);
    let (_, cid) = builder.build_with_cid(
        &decision,
        gate_name,
        entry.gate_cid,
        event,
        context_summary_cid,
        active_universal_band_cid_pointer(),
    );
    decision.decision_attestation_cid = Some(cid);

    Ok(decision)
}

// ─── Discernment-gate convenience shim ───────────────────────────────────────

/// Run both the universal-band DAG **and** the discernment-gate DAG for an
/// `AttestationWrite` event, returning the discernment-gate's `GateDecision`.
///
/// # Contract
///
/// This function runs the universal-band first, short-circuits on non-Allow, and
/// then delegates to [`run_app_domain_gate`] for the discernment-gate.
///
/// Retained as `pub(crate)` for:
/// - Per-rule integration tests in `tests/discernment_gate_integration.rs`
///   (which need pre-populated context injection).
/// - The `pub` testing re-export in `lib.rs` (`run_discernment_gate`).
///
/// Production callers use `check()` → `run_unified()` → `run_app_domain_gate()`.
#[cfg(any(test, feature = "testing"))]
pub(crate) async fn run_discernment_gate(
    event: &RelationalImpactEvent,
    gate_context_extension: GateContext,
) -> Result<GateDecision, GateError> {
    // Step 1: universal-band must Allow first.
    let universal_decision = run_with_tracing(event).await?;
    if !universal_decision.is_allowed() {
        return Ok(universal_decision);
    }

    run_app_domain_gate(
        "discernment-gate-v1-mechanical",
        event,
        gate_context_extension,
        None, // discernment-gate has no attestation step; resolver is unused
    )
    .await
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

    // ─── Phase 8: WisdomTransport::Mock → DevContext in GateDecision ─────────

    #[tokio::test]
    async fn mock_transport_produces_dev_context_in_gate_decision() {
        let runner = DagRunner::new(); // default: WisdomTransport::Mock
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();

        assert!(
            decision.phase.is_dev_context(),
            "Mock transport must produce Phase::DevContext in GateDecision; got: {:?}",
            decision.phase
        );
    }

    // ─── Phase 8: with_wisdom_transport builder preserves Mock default ────────

    #[tokio::test]
    async fn with_wisdom_transport_mock_produces_dev_context() {
        let runner = DagRunner::new().with_wisdom_transport(WisdomTransport::Mock);
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();

        assert!(
            decision.phase.is_dev_context(),
            "with_wisdom_transport(Mock) must produce DevContext; got: {:?}",
            decision.phase
        );
    }

    // ─── Phase 8: canned elohim-active output → ElohimActive in GateDecision ──
    //
    // Injects a pre-cooked elohim-active wisdom output via the test override
    // mechanism and verifies the phase propagates all the way to GateDecision.
    // This exercises the full wisdom-output → SynthesizeExecutor → GateDecision
    // phase propagation path without needing a real LLM backend.

    #[tokio::test]
    async fn canned_elohim_active_wisdom_output_propagates_to_gate_decision() {
        use serde_json::json;

        // The universal-band wisdom step uses constitution_cid="epr:constitutions:elohim-v1"
        // (from the embedded universal-band YAML) and output_key="wisdomOutput".
        // We need to find the right constitution_cid; use the YAML constant.
        let decl = default_universal_band_declaration();

        // Find the WisdomInvoke step to get the actual constitution_cid.
        let wisdom_step = decl
            .dag
            .steps
            .values()
            .find(|s| matches!(s.step, crate::dag::StepType::WisdomInvoke { .. }));

        let (constitution_cid, output_key) = match wisdom_step.map(|s| &s.step) {
            Some(crate::dag::StepType::WisdomInvoke { params }) => {
                (params.constitution_cid.clone(), params.output_key.clone())
            }
            _ => {
                // Universal band may not have a wisdom step in all configurations;
                // skip this test if no wisdom step found.
                return;
            }
        };

        // Inject an elohim-active wisdom output for the wisdom step's key pair.
        crate::__test_set_wisdom_output(
            &constitution_cid,
            &output_key,
            json!({
                "decision": "allow",
                "reasoning": {
                    "primary_principle": "elohim-active-test",
                    "confidence": 0.95,
                    "summary": "Real inference confirmed allowance.",
                    "phase_note": "real-llm"
                },
                "wisdomContext": {},
                "constitutionCid": constitution_cid,
                "framingCid": "bafyframing",
                "phase": "elohim-active"
            }),
        );

        let runner = DagRunner::new();
        let event = content_publish_event();
        let ctx = build_initial_context(&event);

        let decision = runner.run(&event, ctx).await.unwrap();

        assert!(
            decision.phase.is_elohim_active(),
            "Canned elohim-active wisdom output must propagate to Phase::ElohimActive \
             in the final GateDecision; got: {:?}",
            decision.phase
        );
        assert!(
            decision.is_allowed(),
            "Decision must still be Allow; got: {:?}",
            decision.status
        );
    }
}
