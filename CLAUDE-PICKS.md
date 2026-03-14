# What Spoke to Me: A Survey of What I'd Love to Work On

_Updated 2026-03-14. Completed items cleared — see git history for previous versions._

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

Read-only layer fully functional — list, get, query governance states, challenges, proposals, precedents. Backend DB CRUD functions exist but are **not exposed via HTTP**. UI components (Loomio-style voting, reaction bar, SLA timer) are built but have nowhere to persist.

The gap: wire backend CRUD to POST routes (`/challenges`, `/responses`, `/appeals`, `/precedents`), then connect the existing UI. No a2o scenarios exist for governance — story-first was never applied here, which is why write paths were never driven.

**Impact**: High for M5-M6. The elohim "resolve" function depends on this.
**Effort**: Medium for the wiring (CRUD exists), large for the full immune system (dysfunction detection, precedent interpretation).

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

## Small Gaps to Close
