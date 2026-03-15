# What Spoke to Me: A Survey of What I'd Love to Work On

_Updated 2026-03-15. Completed items cleared — see git history for previous versions._

---

## Completed

These items have been built since the original survey (2026-02-25). Archived here for context, not action.

- **Elohim Presence**: `ElohimAgentService` + `ElohimPresenceService` with 32 capabilities, 3 learner hooks (discovery completion, content completion, wellbeing check), mock+native backends, config UI. 96.9% coverage. The nudge/play/resolve functions are formalized and integrated with assessments and knowledge mapping.
- **Path Adaptation**: `PathAdaptationService` + `LearnerContextService` + `PathRecommendationService`. Fog-of-war is now adaptive: mastery-based unlock, pre-assessment skip-ahead, discovery-driven path recommendations. No longer purely sequential.
- **Sophia Moment JSON**: 4 instruments authored (values-hierarchy, attachment-style, strengths-finder, epic-domain) with 5 assessment JSONs, self-registering instrument pattern, and full pipeline from genesis JSON through storage to rendering. Minor gaps: `personal-values-discovery.json` instrument metadata not yet authored; constitutional reasoning exists as mastery assessment only.
- **Offline Queue / IndexedDB**: `IndexedDBCacheService` (80.5% coverage), `WriteBufferService` (85.9%), `LocalSourceChainService` (47%). Infrastructure complete — TTL caching, priority queuing with IDB persistence, append-only local chain model.
- **Story-First Loop**: 28 feature files across 8 directories. Pattern established and practiced. `dev-intent.jsonl` fallback in use. Ongoing discipline, not a deliverable.

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

The steward economy has two layers now: a solid Rust pipeline backend and a fragmented Angular frontend that doesn't yet compose the pieces into a lived experience.

**Backend (complete)**: Recognition pipeline (`recognition_pipeline_service.rs`) with 5 composable stages: normalize → resolve stewards → weight by affinity → constitutional limits → settle economic events. Steward affinity service with mastery gate, curation event deltas, and real affinity wired into weighting. All exposed via REST. This layer works end-to-end.

**Frontend (fragmented)**: Five independent API clients exist but nothing ties them together:

| Service | Pillar | Tests | Status |
|---------|--------|-------|--------|
| `RecognitionApiService` | elohim | 2 | Working, not barrel-exported |
| `StewardshipAllocationService` | lamad | 23 | Tested (2026-03-14), thin API client |
| `EconomicEventsApiService` | shefa | 8 | Working, newly barrel-exported |
| `ExchangeApiService` | shefa | 0 | Working, newly barrel-exported |
| `StewardAffinityApiService` | shefa | 0 | New (2026-03-14), not yet consumed |

**Done (2026-03-14)**: Steward affinity lifecycle in Rust — `steward_affinity` table + API, mastery gate, curation events (+0.10 edit, +0.05 review, +0.15 dispute resolution), constitutional floor/ceiling in Stage 4, genesis seeder with `CATEGORY_AFFINITY_MAP`. Design: `genesis/plans/2026-03-14-steward-affinity-lifecycle-design.md`. Also: `StewardshipAllocationService` test coverage (23 tests, all methods covered), `StewardAffinityApiService` thin client created, barrel exports fixed for `EconomicEventsApiService` + `ExchangeApiService`.

**The big question**: Where does the learner *feel* the steward economy? Right now recognition fires silently — a learner completes an exercise, the pipeline distributes recognition to stewards, economic events are created in the database... and nothing is visible. No steward sees their portfolio grow. No learner sees who stewarded what they just learned. No curator sees their affinity reflected back. The pipeline is a tree falling in a forest. The next move isn't more backend — it's a `StewardEconomyCoordinator` in shefa that composes these five clients into reactive state the UI can render: "You just learned from content stewarded by X, who earned Y recognition. Your curation of Z increased your affinity to W."

**Remaining**: Coordinator service (composes the 5 clients). Portfolio UI (steward dashboard). Curation tracking UI. Affinity decay (time-based). `derived_affinity` (network/community signals). Wire recognition trigger to assessment completion in lamad.

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

## Small Gaps to Close
