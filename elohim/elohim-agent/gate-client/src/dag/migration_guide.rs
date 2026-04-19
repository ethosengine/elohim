//! # Capability-to-Gate Migration Pattern
//!
//! This module is a **pure-rustdoc reference** for migrating an existing
//! [`ElohimCapability`] into a gate-process declaration. It has no runtime
//! code. The canonical worked example is [`content_safety_gate`] —
//! demonstrating the full migration from `ContentSafetyReview` capability to
//! `content-safety-gate-v1`.
//!
//! [`ElohimCapability`]: https://docs.rs/elohim-agent-service/latest/elohim_agent_service/capability/enum.ElohimCapability.html
//! [`content_safety_gate`]: super::content_safety_gate
//!
//! # Why migrate a capability to a gate?
//!
//! Capabilities and gates answer different architectural questions:
//!
//! | | **Capability** | **Gate** |
//! |---|---|---|
//! | **Shape** | hand-coded imperative handler | declarative DAG in YAML |
//! | **Distribution** | compiled into elohim-agent-service | addressed by CID, resolvable via EPR |
//! | **Governance** | implicit in handler code | protocol-governed; declaration is itself content |
//! | **Inspection** | requires reading Rust source | any peer can fetch and audit the declaration |
//! | **Reach** | per-app; another app must rebuild | cross-app; same judgment, same rules |
//! | **Attestation** | ad-hoc | every decision emits a decision-attestation by construction |
//! | **Challengeability** | not first-class | gates are challengeable protocol artifacts |
//!
//! Migration brings a capability under **governance, inspection, and
//! attestation**. The capability itself does not disappear — the gate *wraps*
//! it and (after Phase 7+ activation) *dispatches to it* via the
//! [`SkillInvoke`] step. The migration changes the **call site** (from
//! direct capability invoke to gate-check) and the **evidence trail** (every
//! invocation now leaves a decision attestation).
//!
//! [`SkillInvoke`]: super::StepType::SkillInvoke
//!
//! # When to migrate a capability
//!
//! A capability is a migration candidate when **all** of the following hold:
//!
//! 1. **It is a judgment**, not a pure transformation — the output is an
//!    allow / decline / escalate / verdict (or a classification that resolves
//!    to one), not a synthesized artifact.
//! 2. **Its behavior should be governable** — reach-modulated, challengeable,
//!    auditable by peers.
//! 3. **Other apps will want the same judgment** — the capability's
//!    understanding is protocol-level, not app-specific.
//! 4. **Attestation matters** — a record of "this judgment was rendered for
//!    this subject at this time" has independent value.
//!
//! ## Good migration candidates
//!
//! | Capability | Judgment shape | Status |
//! |---|---|---|
//! | `ContentSafetyReview` | allow / decline for content body | migrated (`content-safety-gate-v1`) |
//! | `ReachNegotiation` | modulate declared reach | migrated (`reach-gate-v1`) |
//! | `AttestationRecommendation` | endorse / abstain / challenge | candidate (brit attestation gate) |
//! | `AccuracyVerification` | verified / disputed / unverifiable | candidate (accuracy gate) |
//! | `GraduatedIntervention` | intervention level selection | candidate (intervention gate) |
//!
//! ## Non-examples (stay as capabilities)
//!
//! These produce artifacts, not judgments — they **should not** be migrated:
//!
//! | Capability | Output shape | Why it stays a capability |
//! |---|---|---|
//! | `KnowledgeMapSynthesis` | a knowledge map | pure synthesis: no allow/decline |
//! | `PathRecommendation` | an ordered learning path | pure recommendation: no decision |
//! | `MasteryAssessmentDesign` | an assessment artifact | pure design work: no judgment |
//!
//! The heuristic: **if the output could reasonably be a `GateStatus`, it is a
//! gate candidate; if the output is a learner-facing artifact, it is a
//! capability.** A capability that emits a recommendation *to* a gate is
//! valid and common — `reach-gate-v1` dispatches to `ReachNegotiation` via
//! skill-invoke for exactly this reason.
//!
//! # The migration recipe
//!
//! Ten steps, executed in order. Steps 1-8 build the gate; step 9 is the
//! DevContext → ElohimActive flip; step 10 is regression coverage.
//!
//! ## 1. Capture the capability's input/output shape
//!
//! Write (or locate) a JSON schema in
//! `elohim/sdk/domains/lamad/schemas/` describing the capability's inputs
//! and outputs. Gates are declarative artifacts — their inputs and outputs
//! are schema-addressable, not Rust-struct-addressable.
//!
//! For `ContentSafetyReview`: inputs are `contentSubject` + `contentBody`;
//! output is `{"decision": "allow" | "decline", "reasoning": ...}`.
//!
//! ## 2. Decide: rules artifact or wisdom framing?
//!
//! - **Mechanical capability** (deterministic rules, no LLM) — author a
//!   rules artifact CID and use a [`MechanicalRuleset`] step. Example:
//!   `seven-valence-rules` backing `discernment-gate-v1-mechanical`.
//! - **Wisdom-backed capability** (LLM judgment against a constitution) —
//!   author a **framing CID** referencing an existing or new constitution.
//!   Example: `epr:constitutions:content-safety-v1` +
//!   `epr:framings:content-safety-v1` backing `content-safety-gate-v1`.
//!
//! [`MechanicalRuleset`]: super::StepType::MechanicalRuleset
//!
//! ## 3. Author the gate DAG YAML
//!
//! Typical topologies:
//!
//! - **Wisdom-backed, 3 nodes:** `assemble` → `wisdom-invoke` → `synthesize`
//!   (the content-safety-gate shape).
//! - **Mechanical, 3 nodes:** `assemble` → `mechanical-ruleset` → `synthesize`
//!   (the discernment-gate-mechanical shape).
//! - **Aggregation-backed, 4 nodes:** `assemble` → `aggregate-attestations`
//!   → `wisdom-invoke` → `synthesize` (the reach-gate shape).
//!
//! Store the YAML at `src/dag/<gate_name>_v1.yaml` alongside its embed
//! module.
//!
//! ## 4. Embed the YAML and seed the ContentNode
//!
//! Create `src/dag/<gate_name>.rs` that:
//!
//! - Embeds the YAML via `include_str!`.
//! - Exposes `default_<gate_name>_yaml() -> &'static str`.
//! - Exposes `default_<gate_name>_declaration() -> GateProcessDeclaration`.
//! - Declares a stable CID constant `<GATE_NAME>_V1_CID`.
//! - Covers the declaration with unit tests that assert parse success,
//!   event_type, DAG topology, step types, and CID references.
//!
//! See `content_safety_gate.rs` for the template.
//!
//! ## 5. Register in the lamad manifest `gates` vocabulary
//!
//! Add an entry to `elohim/sdk/domains/lamad/manifest.json` under the
//! `gates` key declaring the gate's name, CID, event coupling, and
//! description. Run `pnpm run lamad:codegen` to regenerate
//! `manifest-types.ts`.
//!
//! ## 6. Add couplings to the contentTypes the gate should fire on
//!
//! In the same manifest, extend the `couplings` block so that contentTypes
//! the gate governs declare the coupling. This is how TypeScript consumers
//! discover which gates apply at a given write-path.
//!
//! ## 7. Extend `domain_gates_for_event` to route matching events
//!
//! In [`crate::dag_runner`], extend `domain_gates_for_event` to return the
//! new gate's name for every [`RelationalImpactEvent`] variant the gate
//! should fire on. Wide gates (like content-safety) return their name on
//! multiple variants; narrow gates (like reach-gate) return it on a single
//! [`CapabilityInvoke`] predicate.
//!
//! [`RelationalImpactEvent`]: crate::RelationalImpactEvent
//! [`CapabilityInvoke`]: crate::RelationalImpactEvent::CapabilityInvoke
//!
//! ## 8. Add a GateRegistry entry in DagRunner
//!
//! In `dag_runner.rs`, add a `GateRegistry` entry keyed by the gate name so
//! the hot path finds a pre-parsed [`GateProcessDeclaration`] and a bound
//! [`DagInterpreter`] without re-parsing YAML on every request. Ordering in
//! the registry matches declaration order — wide protocol gates
//! (content-safety) register before narrow app-domain gates (discernment,
//! reach).
//!
//! [`GateProcessDeclaration`]: super::GateProcessDeclaration
//! [`DagInterpreter`]: super::DagInterpreter
//!
//! ## 9. DevContext vs. Phase 7+ activation
//!
//! **DevContext (current):** the `wisdom-invoke` step runs the
//! [`WisdomInvokeExecutor`] mock, which writes `{"decision": "allow"}` to
//! the output key. No content is blocked, no capability is invoked, the
//! whole architecture breathes in its intended shape before live elohim.
//!
//! **Phase 7+ activation:** replace the `wisdom-invoke` step with a
//! [`SkillInvoke`] step whose `capability` field matches the capability
//! name as the agent service registers it. The existing capability handler
//! takes over — no new handler code is required, because the capability
//! already exists. Concretely:
//!
//! ```yaml
//! # Before (DevContext)
//! wisdom-safety:
//!   type: "wisdom-invoke"
//!   params:
//!     constitution_cid: "epr:constitutions:content-safety-v1"
//!     framing_cid: "epr:framings:content-safety-v1"
//!     context_keys: ["contentSubject", "contentBody"]
//!     output_key: "safetyOutput"
//!
//! # After (Phase 7+)
//! wisdom-safety:
//!   type: "skill-invoke"
//!   params:
//!     capability: "content-safety-review"
//!     request_from_keys: ["contentSubject", "contentBody"]
//!     output_key: "safetyOutput"
//! ```
//!
//! The synthesize step is unchanged — it reads `safetyOutput` either way.
//! The flip is a YAML edit + a re-seed; no call-site changes.
//!
//! [`WisdomInvokeExecutor`]: super::executors::WisdomInvokeExecutor
//! [`SkillInvoke`]: super::StepType::SkillInvoke
//!
//! ## 10. E2E tests for both paths
//!
//! Land integration tests in `tests/` that cover:
//!
//! - **DevContext:** `check(event)` returns `Allow` via mocked wisdom.
//! - **Simulated-active:** a test-only [`CapabilityDispatcher`] injected
//!   into the `SkillInvoke` step returns a scripted decline; assert the
//!   gate decline propagates through `synthesize` with the correct grounds.
//!
//! [`CapabilityDispatcher`]: super::executors::CapabilityDispatcher
//!
//! # Worked example: content-safety-gate
//!
//! `content-safety-gate-v1` is the canonical migration. It demonstrates each
//! recipe step for a wisdom-backed wide gate.
//!
//! - **Original capability:** `ElohimCapability::ContentSafetyReview`, a
//!   wisdom-backed classifier that reads content body and returns a safety
//!   classification + reasoning. Defined in
//!   `elohim-agent/elohim-agent-service/src/capability/types.rs`.
//! - **Shape preserved:** inputs (`contentSubject`, `contentBody`) and the
//!   `decision` enum carry over unchanged; only the invocation shell
//!   changes.
//! - **Constitution:** `epr:constitutions:content-safety-v1` (pre-existing
//!   content-safety constitution).
//! - **Framing:** `epr:framings:content-safety-v1` primes the LLM call for
//!   the safety decision shape.
//! - **DAG:** 3 nodes — `assemble` pulls `content.body` and
//!   `content.subject` from the event; `wisdom-safety` is a
//!   [`WisdomInvoke`] step; `synthesize` uses the
//!   `"allow-or-decline-from-wisdom"` builder to translate the
//!   `safetyOutput` into an [`Allow`] or a
//!   [`Decline`] with `category: "content-safety"`.
//! - **Event coupling (wide gate):** fires on `ContentPublish`,
//!   `AttestationWrite`, and `PeerMessage` — three event kinds, one gate.
//!   This is why `event_type: "*"` lives in the declaration and
//!   `domain_gates_for_event` does the routing.
//! - **Side effects:** none. Content-safety is read-only; declines flow
//!   back to the caller as HTTP 403 via the tower layer, as `ExternResult`
//!   errors from zome coordinators, or as early returns in libp2p / sync
//!   paths.
//! - **DevContext today:** [`WisdomInvokeExecutor`] writes
//!   `{"decision":"allow"}` to `safetyOutput`; every content publish,
//!   attestation write, and peer message allows through with a decision
//!   attestation for the rehearsal record.
//! - **Phase 7+ activation:** replace the `wisdom-safety` step with
//!   `skill-invoke { capability: "content-safety-review",
//!   request_from_keys: ["contentSubject", "contentBody"], output_key:
//!   "safetyOutput" }`. The dispatch flows through elohim-agent-service to
//!   the existing `ContentSafetyReview` handler. The synthesize step,
//!   registry entry, event routing, and tower / zome / libp2p call sites
//!   are all unchanged.
//!
//! [`WisdomInvoke`]: super::StepType::WisdomInvoke
//! [`Allow`]: crate::GateStatus::Allow
//! [`Decline`]: crate::GateStatus::Decline
//!
//! # Further reading
//!
//! - [`content_safety_gate`] — the embed module and YAML for the worked
//!   example.
//! - Spec §6.2 and §7.3 at
//!   `elohim/elohim-agent/spec/2026-04-18-gate-interface.md`.
//! - Plan §Phase 6 at
//!   `genesis/docs/plans/2026-04-17-peer-stewarded-availability-phase-1-plan.md`.
