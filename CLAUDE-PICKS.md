# What Spoke to Me: A Survey of What I'd Love to Work On

_Written after a deep survey of the codebase, roadmap, recent diffs, and every service in the lamad and imagodei pillars. Updated 2026-02-24 after clearing completed work._

---

## Completed

These items have been resolved and are recorded here for context only.

- **#1 Dashboard Paths Are Empty** — Fixed in `b47a687a`. LearnerDashboardComponent now joins `AgentService.getAgentProgress()` + `PathService.listPaths()` directly, populating the Active Paths section with real progress data. Template didn't need changes.
- **#2 The Fragile BehaviorSubject Cast** — Fixed in `b47a687a`. The `as unknown as BehaviorSubject<unknown>` double-cast in mastery-stats.service.ts:399 was replaced with `this.practice.getPoolSync()`. Tests updated.
- **#4 Practice Service at 37.9% Coverage** — Now at 100% coverage with 56 test cases. All five previously untested areas (challenge cooldown, pool recommendations, challenge submission, discovery counting, pool stats) have dedicated test blocks.
- **#5 The Stashed Work** — Evaluated and cleared. Stash 1 (steward fresh-instance handling) was fully superseded by subsequent multi-account work already in `lib.rs` and Jenkinsfile. Stash 0 (P2P infra: sync verification + relay TODOs) was manually applied to the correct paths (files moved from `orchestrator/` to `genesis/orchestrator/`). Both stashes ready to drop.
- **#7 data-testid Phase 2** — Complete. 168 unique `data-testid` attributes across 40+ component templates (131 in elohim-app, 37 in doorway-app). Selectors registry expanded from 4 groups to 30 (`genesis/a2o/src/framework/pages/selectors.ts`). All 6843 unit tests pass. Remaining work: page objects (~65 needed to consume these selectors in E2E tests).

---

## 1. Path Adaptation Service - The Missing Intelligence

There is no `path-adaptation.service.ts`. The adaptive behavior - adjusting what the learner sees next based on their profile - lives partially in PathService's fog-of-war rules and partially nowhere. The fog-of-war is purely sequential (completed steps + 1), not adaptive to mastery level, interest patterns, or learning velocity.

For the learning experience to feel alive rather than linear, something needs to observe the learner's mastery profile and adjust path recommendations. The `PathService.getPathCompletionByContent()` cross-path method exists but nothing consumes it for adaptive routing.

**Impact**: High (M3 "Know Thyself"). Would transform learning from linear to responsive.
**Effort**: Large. Needs design work on what "adaptive" means for this system.

---

## 2. Discovery Attestation at 17.4% Coverage

`discovery-attestation.service` handles psychometric recognition - when a learner takes a values hierarchy assessment, this service records what they discovered about themselves. At 17.4%, almost nothing is tested. The service connects Sophia assessment results to the learner's self-knowledge profile, which is the emotional heart of "Know Thyself" (M3).

**Impact**: High for M3, low for M1.
**Effort**: Medium. Needs understanding of the attestation model.

---

## 3. Sophia Moment JSON Authoring

The roadmap identifies this as the **critical path bottleneck** for M2-M3. Five discovery instruments are defined as metadata records (values hierarchy, attachment style, strengths finder, constitutional reasoning, personal values) but none have actual Sophia moment JSON authored. Without these, the assessment engine has nothing to render.

This is content authoring work, not engineering. It requires understanding psychometric instrument design and the Perseus/Sophia JSON format. An AI could potentially generate draft instruments, but they'd need human review for psychometric validity.

**Impact**: Critical for M2-M3. Literally the bottleneck.
**Effort**: Large. Cross-disciplinary (psychometrics + JSON schema + content design).

---

## 4. Offline Queue / IndexedDB at 20.3%

The `OfflineOperationQueueService` framework exists for queuing mutations when offline and replaying them when connectivity returns. But IndexedDB integration is only 20.3% complete. For M4 "Take It With You" (Tauri desktop, offline capability), this needs to work. It's not on the critical path for M1 but it's foundational infrastructure that gets harder to retrofit later.

**Impact**: High for M4, low for M1.
**Effort**: Large. IndexedDB testing is notoriously painful.

---

## 5. The Governance Immune System (Read-only Foundation)

The qahal (community/governance) pillar has a complete model layer with constitutional challenges, SLA-guaranteed response times, precedent tracking, and dysfunction detection. The governance service has read-only operations implemented but zero write path. The roadmap puts this at M5-M6.

**Impact**: High for M5-M6.
**Effort**: Very large. This is a separate system.

---

## 6. The Recovery Protocol

`doorway/RECOVERY-SPRINT-PLAN.md` describes a 10-sprint plan for social key recovery. A human loses their device but recovers their identity through a constitutional, verifiable process. Shard tracking, recovery requests, DHT orchestration, work-while-recovering, verification drills. M4+ work.

**Impact**: Transformative for trust and adoption.
**Effort**: Massive. 10 sprints planned.

---

## 7. Steward Economy Services (14-37% Coverage)

The three steward economy services (`stewardship-allocation.service.ts` at 14.8%, `steward.service.ts` at 23.4%, `contributor.service.ts` at 37.7%) are the most severely underbaked services in the codebase. They have elaborate type signatures and method stubs but almost no implementation. These are M5-M6 (economic events, stewardship tracking, request/offer matching).

**Impact**: High for M5-M6, not relevant to M1-M3.
**Effort**: Very large. The models are complex (REA economics).
