# Elohim-Agent Gate Interface — Implementation Plan

**Date:** 2026-04-18
**Spec:** `elohim/elohim-agent/spec/2026-04-18-gate-interface.md`
**Theory:** `elohim/elohim-agent/research/2026-04-18-gate-theory.md`
**Branch target:** `dev`
**Estimated span:** 6–7 weeks to v1 completion, phased.

---

## Guiding Principles (for implementers)

1. **Ship the shape first, fill in the wisdom later.** Every call-site integration lands in its real form during the rehearsal phase; `wisdom-invoke` steps return mocked `Allow { phase: DevContext }` until elohim-activation. Activating wisdom is a config flip, not a rewrite.
2. **Mechanical gates are production from day one.** Deterministic rules and aggregations (7-valence discernment, reach computation) execute real logic, emit real attestations, and ship with full test coverage.
3. **Every layer of the DAG must be inspectable.** Step types are protocol-governed; parameters are CID-addressed ContentNodes. No opaque binaries, no hidden conditionals.
4. **Story-first where possible.** Before implementing a phase, locate or write the a2o scenario it makes pass. For steps without existing scenarios (most of the infrastructure work), capture intent in `.claude/data/dev-intent.jsonl` and run `/close-loop` at the end of the phase.
5. **P2P design gate is mandatory** before adding any new entity, table, or route. The spec's Appendix A captures v1 classifications; any deviation requires re-running the gate.

---

## Phase 0 — Pre-flight (0.5 week)

**Goal:** Validate the spec against existing code, resolve the two open classifications, and prepare the workspace.

### Tasks

- [ ] Deep-read `imagodei` DNA zome structure to resolve ElohimSubstance A-vs-A2 classification.
  - Read: `elohim/holochain/dna/imagodei/zomes/integrity/src/*`
  - Decision: does the existing imagodei agent-identity entry extend via link metadata (A2), or does ElohimSubstance need its own entry type (A)?
  - Update spec Appendix A with the confirmed classification.
- [ ] Deep-read `mishpat` DNA zome structure to decide GateDecisionAttestation placement.
  - Read: `elohim/holochain/dna/mishpat/zomes/integrity/src/*`
  - Decision: extend an existing generic `Attestation` entry type, or define a new `GateDecisionAttestation` entry type? Capacity check confirms mishpat is at 11/~100, ample room either way.
- [ ] Create workspace directories and skeleton files:
  - `elohim/elohim-agent/gate-client/` — new Rust crate
  - `elohim/elohim-agent/elohim-agent-sdk/src/gate-client/` — TypeScript companion module (thin)
- [ ] Draft JSON schemas for the five new ContentNode contentTypes; save to `elohim/sdk/domains/lamad/schemas/`:
  - `gate-process-declaration.schema.json`
  - `universal-band-declaration.schema.json`
  - `gate-rules-declaration.schema.json`
  - `aggregation-spec.schema.json`
  - `escalation-target-spec.schema.json`
- [ ] Register the five contentTypes in `elohim/sdk/domains/lamad/manifest.json` under `contentFormats` / `contentTypes` (whichever the codegen expects).
- [ ] Run `pnpm run lamad:codegen` and confirm codegen succeeds before adding dependent code.

### Exit criteria

- `ElohimSubstance` classification confirmed (A or A2).
- `GateDecisionAttestation` placement decided.
- All five new contentTypes registered and codegen passes.
- `gate-client` crate compiles as an empty skeleton.

---

## Phase 1 — `gate-client` crate scaffolding (1 week)

**Goal:** Produce a runnable `gate-client` crate with the RelationalImpactEvent enum, GateDecision response shape, dev-context mock wisdom-invoke, and tower::Layer integration.

### Tasks

- [ ] Define core types in `gate-client/src/types.rs`:
  - `RelationalImpactEvent` enum (all eight variants per spec §1.2)
  - `GateDecision`, `GateStatus`, `SideEffect`, `Phase`, `DeclineGrounds`, `EscalationTarget`, `Severity`
  - `ConstitutionalReasoning` (re-export from `elohim-agent-service::response`)
  - `SpaceType`, `SpaceContext`
  - `GateError`
- [ ] Define `GateClientConfig` and `Transport` enum with variants `InProcess`, `Http(url)`, `Grpc(url)`.
- [ ] Implement the primary check function with the dev-context mock path:
  - `pub async fn check(event: RelationalImpactEvent) -> Result<GateDecision, GateError>`
  - During dev-context phase: runs real space-type detection, returns exempt for interior events, returns mocked `Allow { phase: DevContext }` for boundary-crossing events.
- [ ] Implement `check_blocking` for zome coordinator contexts.
- [ ] Implement `tower_layer()` returning `impl tower::Layer<S>` that wraps HTTP routes.
- [ ] Implement testing helpers under `gate-client::testing`:
  - `mock_allow()`, `mock_decline(grounds)`, `mock_escalate(target)`, `mock_verdict(tag)`
  - `with_mock_decision(decision, f)` for test-scoped overrides
- [ ] Generate TypeScript types from Rust via `ts-rs`:
  - Run `cargo test export_bindings` convention; output to `elohim/elohim-agent/elohim-agent-sdk/src/gate-client/generated/`
- [ ] Write the TypeScript thin client wrapping the HTTP transport:
  - `createGateClient(config)` returning `{ check, queueForReview }`
  - Auto-generated types imported from `generated/`
- [ ] Write unit tests for:
  - Space-type detection (call-site marker, user mode flags, target inference)
  - Exempt short-circuit paths (offline, private-drafting-interior, play-interior)
  - Boundary-crossing dispatch
  - Tower layer integration with a test Axum router
- [ ] Document crate with rustdoc; include the three call-site pattern examples from spec §1.4.

### Exit criteria

- `cargo test -p gate-client` passes.
- `pnpm test` in the elohim-agent-sdk passes for the TS thin client.
- A worked integration test spins up an Axum router with the tower layer, sends a RelationalImpactEvent via HTTP, receives a mocked GateDecision.
- Rustdoc documents the three call-site patterns.

---

## Phase 2 — Protocol-root universal-band-declaration (1 week)

**Goal:** Author, register, and wire the universal band DAG so that dev-context gate invocations route through a real DAG interpreter, executing all deterministic steps and returning a mocked wisdom decision from `wisdom-primary`.

### Tasks

- [ ] Design the v1 universal-band DAG in YAML matching the spec §2.5 shape. Save as seed data for a `universal-band-declaration` ContentNode.
- [ ] Write the DAG interpreter in `gate-client/src/dag.rs`:
  - Parse a GateProcessDeclaration body (JSON/YAML schema)
  - Build a typed step graph
  - Execute steps in topological order with GateContext accumulation
  - Evaluate conditional edges (simple comparison language over context keys)
  - Short-circuit on terminal nodes
- [ ] Implement step executors for the three deterministic types (v1 universal band needs these):
  - `context-assemble` executor — pulls from elohim-storage, DHT (stubbed query resolver for Phase 2, real resolver in Phase 3+), source-chain, manifest references
  - `synthesize` executor — composes GateDecision + side effects from context
  - `escalate-to-review` executor — emits escalation decision
- [ ] Implement the `wisdom-invoke` executor stub:
  - During dev-context: returns `Allow { phase: DevContext }` with placeholder ConstitutionalReasoning
  - Logs the invocation for observability
  - Real implementation deferred to Phase 6 (activation path)
- [ ] Register the universal-band-declaration seed in `genesis/seeder/` data.
- [ ] Add a well-known constitutional pointer (file or manifest field) that names the active universal band CID.
- [ ] Integration test: run `gate_client::check(event)` end-to-end; verify the real DAG executes `authorize` → `assemble-context` → `wisdom-primary` (mocked) → `record-decision`.
- [ ] Add observability: every DAG step logs entry/exit; step timings collected for baseline measurement.

### Exit criteria

- Universal-band DAG is a real ContentNode in seed data.
- DAG interpreter executes the universal band end-to-end.
- `wisdom-primary` returns mocked Allow; every other step runs real.
- Integration test confirms full DAG execution and decision attestation shape (Phase 4 will write the attestation; Phase 2 just validates the shape).

---

## Phase 3 — `discernment-gate-v1-mechanical` (1 week)

**Goal:** First production gate. Ship real, mechanical-only, fully tested.

### Tasks

- [ ] Implement `mechanical-ruleset` step executor:
  - Parameterized by a `rulesCid` ContentNode
  - Fetches rules, parses as a declarative rule list
  - Evaluates rules in order against GateContext keys
  - Returns first-matching outcome, or null if no match
- [ ] Author the seven-valence rules artifact:
  - Serialize the rules from `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` §7.3 as a structured ContentNode with `contentType: gate-rules-declaration`
  - Include all seven valences, magnitudes, evidence types, and rule ordering
  - Preserve the rule 3 vs rule 2 overlap semantics explicitly
- [ ] Author the discernment-gate-v1-mechanical GateProcessDeclaration:
  - Three-node DAG: assemble → rules → synthesize
  - Terminal emits `Verdict(StoryPointTag)` with MintAttestation + EmitEconomicEvent side effects
  - Serialize as seed data ContentNode with `contentType: gate-process-declaration`
- [ ] Register the gate in the lamad manifest:
  - Add to `gates` vocabulary section with processCid reference
  - Add `"gates": ["discernment-gate-v1-mechanical"]` to `experience-moment` contentType's `coupling.governance`
- [ ] Run `pnpm run lamad:codegen` to pick up new gate registration.
- [ ] Write the Rust port of the TS discernment tests (from the superseded TS plan):
  - Tests for each of rules 1–6
  - Rule 3 vs rule 2 overlap test
  - Steady-state null-return test (rule 7)
  - Non-terminal status safety-net tests (already proven in TS)
  - Edge cases from the superseded plan
- [ ] Integration test: full gate invocation via `gate_client::check(AttestationWrite { /* experience-moment */ })`:
  - Universal band mocks Allow
  - discernment-gate-v1-mechanical runs real
  - Emits correct StoryPointTag + side effects for each rule
- [ ] Wire into the experience-moment coordinator zome (Phase 7 does this more broadly; this task confirms the integration point works).

### Exit criteria

- `discernment-gate-v1-mechanical` executes deterministically against fixtures.
- All seven rules tested with green coverage matching the superseded TS test set.
- Integration test produces correct StoryPointTag + side effects end-to-end.
- Lamad manifest registers the gate; codegen picks it up.

---

## Phase 4 — GateDecisionAttestation + doorway routes (0.75 week)

**Goal:** Persist gate decisions as DHT attestations with storage projections and doorway read access.

### Tasks

- [ ] Define the GateDecisionAttestation entry type in mishpat (per Phase 0 decision):
  - Integrity zome: entry type + validation
  - Coordinator zome: `create_gate_decision_attestation(input) -> EntryHash`
  - Post-commit signal: `GateDecisionCreated { entry_hash, entry }`
- [ ] Implement the storage projection in elohim-storage:
  - Migration creating `gate_decision_attestations` table with `dht_anchor_hash NOT NULL`
  - Handler subscribes to `GateDecisionCreated` signal and inserts row
  - Indexes: `elohim_id`, `gate_name`, `manifest_cid`, `phase`
- [ ] Export View type for HTTP consumption:
  - Write `GateDecisionAttestationView` struct in `views.rs` with `#[serde(rename_all = "camelCase")]` and `#[derive(TS)]`
  - Add to schema contract test
  - Run `cargo test export_bindings` to regenerate TS types
- [ ] Implement doorway routes:
  - `GET /api/gate-decisions/{cid}` — single decision
  - `GET /api/gate-decisions?elohim={id}&gate={name}&phase={phase}` — list with filters
- [ ] Wire the DAG interpreter to emit a decision-attestation at every synthesize terminal:
  - Pre-synthesize: build the attestation payload from GateContext + decision
  - Post-synthesize: call mishpat coordinator to write
  - Return `decision_attestation_cid` in GateDecision
- [ ] Integration test: invoke discernment-gate-v1-mechanical with a test moment; verify a GateDecisionAttestation lands in DHT and is readable via doorway GET.

### Exit criteria

- GateDecisionAttestation is a real DHT entry type with validation.
- Storage projection populates on every gate invocation.
- Doorway GET endpoints return attestation views.
- End-to-end test confirms attestation landing and retrieval.

---

## Phase 5 — `reach-gate` (0.75 week)

**Goal:** Second gate. Aggregation-driven with edge-case wisdom (stubbed).

### Tasks

- [ ] Implement the `aggregate-attestations` step executor:
  - Parameterized by an `aggregationSpecCid` ContentNode
  - Queries DHT for attestations matching the spec's predicates
  - Reduces attestations via declared reduction (count, weighted sum, threshold-crossing)
  - Returns a scalar output (e.g., ReachLevel enum value)
- [ ] Author the reach-gate aggregation-spec ContentNode:
  - Declares which attestation kinds count toward which reach levels
  - Threshold configurations (community / public / protocol)
- [ ] Author the reach-gate GateProcessDeclaration:
  - Four-node DAG: assemble → aggregate → wisdom-invoke-edge → synthesize
  - wisdom-invoke-edge conditional: runs only when `aggregate.edge_case == true`
  - Synthesize emits `Verdict(ReachLevel)`; no side effects
- [ ] Register in lamad manifest `gates` vocabulary.
- [ ] Decide which contentTypes couple to reach-gate:
  - Provisionally: `content`, `attestation`, `economic-event` contentTypes
  - Add to their `coupling.governance.gates` lists
- [ ] Tests:
  - Aggregation correctness against fixture attestation graphs
  - Threshold boundaries (just-below, just-above, at-threshold)
  - Edge-case detection routing to wisdom-invoke
  - Reach-level enum coverage
- [ ] Integration test: invoke reach-gate via `CapabilityInvoke { ReachNegotiation }`; verify ReachLevel returned.

### Exit criteria

- `reach-gate` computes correct reach for fixture graphs.
- Edge-case routing exercises wisdom-invoke (stub) path.
- Integration with `ReachNegotiation` capability confirmed.

---

## Phase 6 — `content-safety-gate` + `skill-invoke` + migration pattern (1 week)

**Goal:** Third gate; reference implementation for all future LLM-backed gates; worked example of capability-to-gate migration.

### Tasks

- [ ] Implement the `skill-invoke` step executor:
  - Parameterized by a capability name + request params from GateContext keys
  - Invokes the named ElohimCapability via the agent-service's existing dispatcher
  - Captures the capability's response into GateContext
  - During dev-context: the capability may itself be stubbed depending on the capability's own dev status
- [ ] Author the content-safety-gate GateProcessDeclaration:
  - Three-node DAG: assemble → wisdom-invoke → synthesize
  - wisdom-invoke references the constitution priming for content-safety reasoning
  - synthesize emits `Allow` or `Decline { grounds: ContentSafetyGrounds }`
- [ ] Register in lamad manifest; bind to all contentTypes that carry `ContentPublish` / `AttestationWrite` / `PeerMessage` events.
- [ ] Migrate the existing `ContentSafetyReview` capability to the skill-invoke pattern:
  - Capability handler preserved; no breaking change
  - In elohim-active phase: content-safety-gate's wisdom-invoke delegates to the capability via skill-invoke
  - In dev-context phase: wisdom-invoke returns mocked Allow
- [ ] Document the migration pattern in the gate-client rustdoc as the canonical example for other capability-to-gate migrations.
- [ ] Integration tests:
  - Dev-context: wisdom-invoke returns Allow; decision attestation records the mock
  - Stub-elohim-active mode (test-only): simulate a real wisdom response; verify the DAG composes decline reasoning correctly
  - Capability delegation pathway exercises both code paths

### Exit criteria

- `content-safety-gate` ships shape-complete; integrates with existing capability.
- `skill-invoke` step works and is documented as the migration pattern.
- Test coverage confirms both dev-context and simulated-active behavior.

---

## Phase 7 — Doorway + zome integration (0.5 week)

**Goal:** Wire the tower::Layer across doorway POST routes and integrate gate_client into at least one representative zome coordinator.

### Tasks

- [ ] Add `gate_client::tower_layer()` to doorway's main Axum router.
- [ ] Instrument doorway's POST routes to construct appropriate RelationalImpactEvents per route:
  - `/content` → ContentPublish
  - `/attestation` → AttestationWrite
  - `/economic-event` → EconomicEventEmit
  - `/agent/invoke` → CapabilityInvoke
- [ ] Verify dev-context passes for existing callers (the gate should be transparent — everything returns Allow unless a real mechanical gate declines).
- [ ] Integrate `gate_client::check_blocking` into at least two representative zome coordinators:
  - `lamad::create_content_node` for content publishing
  - `lamad::create_experience_moment` for discernment-gate exercise
- [ ] End-to-end test: browser-driven content publish flows through doorway → gate check → zome → DHT → storage projection. Confirm dev-context attestation lands.

### Exit criteria

- Doorway tower::Layer wraps POST routes.
- Two representative zome coordinators invoke the gate.
- No existing browser flow is broken; gate is transparent in dev-context.
- E2E flow produces a real GateDecisionAttestation.

---

## Phase 8 — Pre-activation hardening (0.5 week)

**Goal:** Polish, documentation, and activation readiness.

### Tasks

- [ ] Performance baseline: measure cold-start and warm-cache gate-check latencies. Baseline expectations for post-activation reality check.
- [ ] Write operator documentation for the flag flip from dev-context to elohim-active:
  - Config change in `gate-client::configure()` to point wisdom-invoke at a real elohim-agent-service
  - Phase marker flip from `DevContext` to `ElohimActive` in gate-client config
  - Confirmation steps (smoke test, observability checks)
- [ ] Write a sibling spec skeleton for the Challenge + Indemnification flow (follow-on work):
  - `genesis/docs/superpowers/specs/YYYY-MM-DD-gate-challenge-indemnification-design.md` stub
  - Reference hook points this spec provides (decision-CID linkability, substance-CID, phase field)
- [ ] Update `CLAUDE.md` or relevant onboarding docs to note the new gate primitive and the gate-client library's expected use at every relational-impact write path.
- [ ] Inspect the experience-story supersession: confirm `rakia/docs/plans/2026-04-18-experience-story-discernment-gate.md` is now fully superseded and note it in the superseding header.
- [ ] Story harvest via `/story-harvest`: capture any engineering constraints discovered during implementation as a2o scenarios.

### Exit criteria

- Performance baseline recorded.
- Activation runbook documented.
- Follow-on specs stubbed and linked.
- Story harvest complete.

---

## Cross-Cutting Concerns

### Testing strategy

- **Unit tests** per step executor, per DAG interpreter behavior, per space-type detection path.
- **Integration tests** per gate (discernment, reach, content-safety) covering happy paths and edge cases.
- **End-to-end tests** per call-site pattern (zome, doorway POST, in-process).
- **Fixture-driven gate exercise** — every gate has a fixture suite covering its rule or aggregation space.
- **Dev-context integrity** — a test that the gate-client, when configured for dev-context, never actually calls a real wisdom backend even if one is available.

### Observability

- Every DAG step emits a log event with step-id, input keys, output summary, elapsed time.
- GateDecisionAttestation writes emit metrics: total decisions, declines, escalations, verdicts by gate.
- Inspection-cache hit / miss rates logged per elohim.
- Trust-context staleness detected (re-inspection triggered) logged as a metric.

### Backwards compatibility

- Existing ElohimCapability invocations continue working; the tower::Layer wraps them transparently, returning Allow in dev-context.
- Existing content-safety-review callers are not broken by the content-safety-gate migration; the capability handler is preserved and the gate delegates via skill-invoke only in elohim-active phase.

### Phase-marker discipline

- Every GateDecisionAttestation written during implementation carries `phase: DevContext`.
- Reputation aggregation (when implemented in a later spec) MUST filter by `phase: ElohimActive` only.
- Tests asserting behavior include explicit assertions on phase value.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Dev-context mocks hide real failures | Medium | Medium | Ship integration tests against a real LLM even during rehearsal; log all wisdom-invoke calls so regressions surface when activation happens |
| Inspection cache staleness | Low | Medium | Subscribe to governance ratification signals; invalidate author-dependent caches |
| Graduated depth attack | Low | High | Universal band ALWAYS runs `wisdom-primary` — shortcutting only affects post-universal-band manifest inspection |
| Side-effect execution divergence | Medium | Medium | decision-attestation is written BEFORE side effects execute; failed effects are visible as decision-to-outcome gaps |
| ImagoDei zome pressure from ElohimSubstance | Low | Low | Phase 0 classification resolves A vs A2; if A needed, imagodei has ample entry-type headroom |
| lamad codegen breakage from new contentTypes | Low | Medium | Phase 0 validates codegen succeeds before dependent work; if breakage, localize to schema and manifest changes |

---

## Superseded / Related Work

- **Supersedes:** `rakia/docs/plans/2026-04-18-experience-story-discernment-gate.md` (the full TS-discernment plan; metadata schemas and manifest registration already landed and remain correct).
- **Consumes:** `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` §5–§7 (rules content for discernment-gate-v1-mechanical).
- **Compositional sibling:** `rakia/docs/plans/build-attestation-integration.md` (GateDecisionAttestation specializes from brit's attestation pattern).
- **Future follow-ons:**
  - Challenge + Indemnification spec (defines the accountability loop that this plan hooks into)
  - Elohim Participant-Type spec in imagodei (canonicalizes substance schema)
  - Open-event-type extensibility (v1.1 upgrade path)

---

## Definition of Done

The plan is complete when:

1. All three gates (`discernment-gate-v1-mechanical`, `reach-gate`, `content-safety-gate`) are registered and executable.
2. The universal-band DAG runs for every RelationalImpactEvent, with dev-context `wisdom-invoke` mocked.
3. Every gate invocation writes a `GateDecisionAttestation` with `phase: DevContext`.
4. At least two zome coordinators and the doorway HTTP surface invoke the gate.
5. Full test coverage per Phase exit criteria.
6. Activation runbook is documented; flag-flip from DevContext → ElohimActive is one config change, not a code change.
7. All companion artifacts (schemas, manifest registrations, ts-rs generated types) are committed.
8. Story-harvest pass has extracted any discovered engineering constraints into a2o scenarios.

The gate is **architecturally real** at this point. Wisdom arrives later, through activation.
