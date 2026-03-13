# What Spoke to Me: A Survey of What I'd Love to Work On

_Updated 2026-03-13. Completed items cleared — see git history for previous versions._

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

`StewardshipAllocationService` exists at 14.8% coverage with content stewardship, portfolio queries, allocation CRUD, dispute management, recognition distribution. REA API clients exist (economic-events, exchange, flow-planning) but no unified coordinator. The allocation model is rich — contribution types, governance lifecycle, temporal effectiveness — but largely untested.

**Impact**: High for M5-M6.
**Effort**: Large. REA economics coordination layer needed.

---

## Small Gaps to Close

<<<<<<< Updated upstream
The qahal pillar has a complete model layer with constitutional challenges, SLA-guaranteed response times, precedent tracking, and dysfunction detection. Read-only operations implemented but zero write path. This is where the elohim "resolve" function lives — dispute resolution governed by community norms, not external courts.

**Impact**: High for M5-M6. The third elohim function (resolve) depends on this.
**Effort**: Very large. Separate system. M5-M6 roadmap.

---

## 6. Offline Queue / IndexedDB

`OfflineOperationQueueService` framework exists for queuing mutations when offline. IndexedDB integration only 20.3% complete. Foundational for M4 "Take It With You" (Tauri desktop, offline capability). Gets harder to retrofit later.

**Impact**: High for M4, low for M1.
**Effort**: Large. IndexedDB testing is notoriously painful.

---

## 7. Steward Economy Services

Recognition pipeline service built in Rust (`elohim-storage/src/services/recognition_pipeline_service.rs`) with 5 composable stages: normalize (event type weights) → resolve (steward allocations + affinity) → weight (proportional with affinity coefficient) → limit (constitutional checks, v0 passthrough) → settle (economic events + recognition accumulation). Exposed via `POST /api/v1/recognition/distribute`. Angular thin client in elohim pillar.

**Remaining**: v0 affinity defaults to 1.0 — wire stored_affinity from node_stewardship and derived_affinity from human profiles. Constitutional limit enforcement (stage 4). Future distribution models documented in `genesis/research/economic/future-distribution-models.md`.

**Impact**: High for M5-M6.
**Effort**: Medium remaining. REA coordination layer wired; deeper economics is research.

---

## 8. Exercise the Story-First Loop

Take one of the above items, write the a2o scenario FIRST, feel the vision, then implement to make it pass. Test whether the story-first workflow we just built actually changes the quality of what gets produced. Prove that thinking from the learner's perspective rather than the service's perspective changes the code.

**Impact**: Meta — validates the development workflow itself.
**Effort**: Small (additive to whatever we pick next).
