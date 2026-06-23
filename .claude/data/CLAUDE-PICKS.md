# What Spoke to Me: A Survey of What I'd Love to Work On

_Updated 2026-03-23. Completed items cleared — see git history for previous versions._

---

## Completed

These items have been built since the original survey (2026-02-25). Archived here for context, not action.

- **Elohim Presence**: `ElohimAgentService` + `ElohimPresenceService` with 32 capabilities, 3 learner hooks (discovery completion, content completion, wellbeing check), mock+native backends, config UI. 96.9% coverage. The nudge/play/resolve functions are formalized and integrated with assessments and knowledge mapping.
- **Path Adaptation**: `PathAdaptationService` + `LearnerContextService` + `PathRecommendationService`. Fog-of-war is now adaptive: mastery-based unlock, pre-assessment skip-ahead, discovery-driven path recommendations. No longer purely sequential.
- **Sophia Moment JSON**: 4 instruments authored (values-hierarchy, attachment-style, strengths-finder, epic-domain) with 5 assessment JSONs, self-registering instrument pattern, and full pipeline from genesis JSON through storage to rendering. Minor gaps: `personal-values-discovery.json` instrument metadata not yet authored; constitutional reasoning exists as mastery assessment only.
- **Offline Queue / IndexedDB**: `IndexedDBCacheService` (80.5% coverage), `WriteBufferService` (85.9%), `LocalSourceChainService` (47%). Infrastructure complete — TTL caching, priority queuing with IDB persistence, append-only local chain model.
- **Story-First Loop**: 28 feature files across 8 directories. Pattern established and practiced. `dev-intent.jsonl` fallback in use. Ongoing discipline, not a deliverable.
- **Distributed Trust Auth (2026-03-23)**: Full four-layer trust model from design through implementation. Reach-tier differentiated authorization (all 6 tiers: commons through private). Attestation gates on HTTP and P2P (prerequisite mastery, consent, payment). Recognition pipeline wired to P2P delivery (replaces raw economic events). Policy enforcement instantiated and active. EPR Head carries `attestation_requirements`. 5-peer cross-env topology deployed (Matthew+Jessica on alpha, Pete+Timothy on staging). 5 A2O distributed trust scenarios. 500 Rust tests.
- **Ambient Trust Architecture (2026-03-23)**: Per-connection DHT-verified trust negotiation designed and scaffolded. `/elohim/trust/1.0.0` handshake protocol (MessagePack codec with roundtrip tests). `PeerTrustCache` with TTL expiry. `trust_verification` module with three-pillar types and reach ceiling calculation. `verify_credentials()` zome functions in imagodei and mishpat DNAs. Fast-path in `check_reach_authorization` skips DB for cached peers. Conductor integration stubs ready for live DHT verification.
- **EPR Visibility (2026-03-23)**: Reach and resilience badge indicators on content-viewer and lesson-view headers. Progressive disclosure: ambient icons, tooltip on hover, click-through to trust tab. Proves the EPR data pipeline end-to-end from Rust storage to Angular UI.
- **Content Flags & Stewardship Display (2026-03-23)**: Content flags (disputed, outdated, appeal-pending, under-review, partial-revocation) render as subtle colored tags in content-viewer and lesson-view headers — only visible when flags exist, click navigates to trust tab. Stewardship section in trust tab shows allocation cards with steward name, role badge, allocation bar, recognition accumulated, and dispute indicator. Loads via `StewardshipAllocationService.getContentStewardship()`. 79 content-viewer tests passing.
- **EPR Link Navigation Boxes (2026-04-17)**: `EprRelationshipCardComponent` + `EprRelationshipsPanelComponent` in elohim pillar. Content-viewer and lesson-view fetch the EPR Head and render typed relationship cards (PREREQUISITE/TEACHES/CONTAINS/REFERENCES) with reach badges and resilience badges below content body. Click navigates to target concept. 12 new tests, 7201 total passing.

---

## 1. Recovery Protocol - Sovereignty Through Embeddedness

Phase 1 (shard tracking) complete. Frontend `RecoveryCoordinatorService` at 98.7% coverage with interview flow, attestation progress, question generation. Rich domain models.

Phases 2-5 remain: recovery request flow (DHT entry types in imagodei DNA), shard reconstruction, work-while-recovering, verification. Your attestations of relationship and history of mutual stewardship ARE your safety net — your actual people, whose elohim coordinate the response while preserving your dignity.

**Impact**: Transformative for trust and adoption. M4+ work.
**Effort**: Large. 4 remaining phases.
**Done**: Phase 1 shard tracking, frontend coordinator service, recovery models.

---

## 2. The Governance Immune System (Qahal Write Path)

Read-only layer fully functional. **Sprint 2 complete (2026-03-15)**: proposals, votes, and discussions now persist via real HTTP POST routes to elohim-storage. Votes table added (SQLite, UNIQUE per proposal+human). GovernanceService wired to API — localStorage MVP replaced. CollectiveDetailComponent built with members/proposals/discussions tabs. 5 a2o governance scenarios added (first qahal coverage). 99 governance tests passing.

**Done**: Votes table + migration, Rust CRUD handlers (create proposal, cast/get votes, create discussion, post message), generated TypeScript types (VoteView, CastVoteInputView, etc.), GovernanceApiService POST methods, GovernanceService API integration, CollectiveDetailComponent, community routes, a2o scenarios.

**Remaining (Sprint 3 — the immune system)**: Challenges write path (still localStorage), responses, appeals, precedent interpretation, dysfunction detection, elohim resolve function, SLA enforcement.

**Impact**: High for M5-M6. The elohim "resolve" function depends on Sprint 3.
**Effort**: Medium for challenges wiring, large for the full immune system.

---

## 3. Steward Economy Services

The steward economy backend is now fully wired — including P2P delivery recognition. The gap has shifted from plumbing to experience.

**Backend (complete + P2P wired)**: Recognition pipeline fires on every P2P content delivery (2026-03-23), replacing raw economic events with the full 5-stage pipeline: normalize → resolve stewards → weight by affinity → constitutional limits → settle. Steward affinity service with mastery gate, curation deltas, real affinity wired into weighting. All exposed via REST.

**Frontend progress**: Reach and resilience badges (2026-03-23) are the first visible surface — learners can see stewardship health via the resilience indicator next to content titles. Five API clients still exist independently:

| Service | Pillar | Tests | Status |
|---------|--------|-------|--------|
| `RecognitionApiService` | elohim | 2 | Working, not barrel-exported |
| `StewardshipAllocationService` | lamad | 23 | Tested, thin API client |
| `EconomicEventsApiService` | shefa | 8 | Working, barrel-exported |
| `ExchangeApiService` | shefa | 0 | Working, barrel-exported |
| `StewardAffinityApiService` | shefa | 0 | Not yet consumed |

**The remaining question**: The resilience badge shows stewardship *health* but not the lived experience. The next move is a `StewardEconomyCoordinator` in shefa that composes these five clients into reactive state: "You just learned from content stewarded by X, who earned Y recognition." The EPR context menu (Surface 3, designed not built) is where steward portfolio detail will live.

**Remaining**: Coordinator service. Portfolio UI (steward dashboard). Curation tracking UI. Affinity decay. `derived_affinity`. Assessment completion → recognition trigger.

**Impact**: High for M5-M6. The economic experience is what makes stewardship feel real.
**Effort**: Medium. All plumbing exists; this is composition and UX.

---

## 4. ElohimGate — Adaptive Friction Through Trust Signals

The gate is an async mutation interceptor in elohim-storage. Every write passes through it. Four tiers of inference (None/Light/Deep/Constitutional), four outcomes (PassThrough, Enriched, Pause with confirm flow, Settlement with constitutional boundary).

**Backend (Sprints 1-4, complete)**: The gate has all six trust senses wired to real data:

| Signal | Source | Module |
|--------|--------|--------|
| mastery_depth | Content mastery records (Bloom's taxonomy × freshness) | `mastery_depth.rs` |
| steward_standing | Stewardship allocations (active ratio + recognition - disputes) | `steward_standing.rs` |
| relationship_density | Human relationships (log-scaled, weighted by verification/consent) | `relationship_density.rs` |
| governance_health | Allocation governance states (active/disputed/pending ratio) | `governance_health.rs` |
| behavioral_trust | Observation history (trust deltas with time decay) | `behavioral_trust.rs` |
| intent_divergence | Anomaly detection (mutation rate + trust trend divergence) | `anomaly_detection.rs` |

Feedback loop closed: gate evaluates → observations stored → behavioral trust updated → gate re-evaluates with new context. PendingConfirmationCache for pause/confirm flow. InferenceRouter with priority-based engine selection and fallback. 344 tests.

**Not yet built**:
- **Angular gate client** (Sprint 5): Service to handle `GateEvaluationView` responses, pause confirm UX, trust context display. Types already generated (`GateEvaluationView.ts`, `TrustContextView.ts`).
- **SSE streaming** (Sprint 6): Real-time gate evaluation push to Angular. No existing SSE patterns in codebase — needs full wire-up.
- **Inference sidecar integration** (Sprint 7): Connect elohim-agent-sdk at :8095 for Deep/Constitutional tier inference. `SidecarEngine` HTTP client exists but sidecar isn't deployed yet.

**Impact**: This is the elohim's primary sensory system — how it perceives the health of a mutation before it lands. Everything downstream (nudge, play, resolve) depends on the gate seeing clearly.
**Effort**: Backend complete. Sprint 5 (Angular client) is medium. Sprint 6-7 are larger.

---

## 5. Ambient Trust — Conductor Integration (Layer B+C Stubs)

The ambient trust architecture is scaffolded (protocol, cache, types, fast-path) but conductor integration stubs need filling. This is the work that turns "server auth on every peer" into "distributed trust negotiated between peers."

**Scaffolded (2026-03-23)**: `trust_verification.rs` types + reach ceiling calculation. `trust_protocol.rs` handshake codec. `trust_cache.rs` per-connection cache. Fast-path in `check_reach_authorization`. `verify_credentials()` zome functions in imagodei + mishpat.

**Remaining**: Fill the four verification helper stubs (`verify_membership_cids`, `verify_relationship_cids`, `verify_attestation_cids`, `verify_stewardship_cids`) with actual `hc_client.call_zome()` calls. This requires ActionHash parsing from CID strings and mapping conductor responses back to the verified types. Then: populate the handshake with real credentials from the local agent's memberships/relationships.

**Design**: `genesis/plans/2026-03-23-ambient-trust-verification-design.md`
**Plan**: `genesis/plans/2026-03-23-ambient-trust-verification-plan.md`

**Impact**: This completes the P2P trust model. Without it, reach checks work but hit the DB every time and don't verify against the DHT.
**Effort**: Medium. Infrastructure exists. The work is mapping conductor wire formats to trust types.

---

## Small Gaps to Close

- **Context menu (Surface 3)**: Three-dot menu on content with flag/feedback modal + deep navigation to stewardship/governance/attestation detail. Designed, not built.
