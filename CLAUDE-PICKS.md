# What Spoke to Me: A Survey of What I'd Love to Work On

_Updated 2026-02-25 after protocol philosophy conversation. Previous completed items archived — see git history for context._

---

## 1. Elohim Presence in the Learning Journey

The protocol's philosophical architecture describes the elohim layer as nudge/play/resolve — AI agents that are separate from humanity but present within the system, holding context across all four pillars and reflecting it back meaningfully. Right now, *I'm* the elohim — sync-check hooks, ambient reminders, story-first nudges — but that's configuration files, not a learner-facing presence.

There's no `ElohimService`, no formalized agent presence in the app. What would it look like for the lamad journey to have an elohim presence — not a chatbot, not anthropomorphized, but something that sees your learning profile, your discovery results, your community relationships, and reflects it back in a way that feels like guidance? The difference between content delivery and encountering something that *knows you*.

This is the "play" function — meaning-making that transcends the rational. It's what makes the system endure across generations. The fruit on the tree.

**Impact**: Transformative. This is what the protocol IS.
**Effort**: Requires deep design. What does non-anthropomorphized AI presence feel like in a UI?
**Depends on**: Path adaptation (#2), discovery attestation infrastructure (done), sophia integration (in progress).

---

## 2. Path Adaptation Service - The Missing Intelligence

There is no `path-adaptation.service.ts`. The fog-of-war is purely sequential (completed steps + 1), not adaptive to mastery level, interest patterns, or learning velocity. For the learning experience to feel alive rather than linear, something needs to observe the learner's profile and adjust path recommendations.

`PathService.getPathCompletionByContent()` cross-path method exists but nothing consumes it for adaptive routing. The discovery attestation service now records rich profile data (values hierarchy, attachment style, strengths) — this should inform what the learner sees next.

**Impact**: High (M3 "Know Thyself"). Transforms learning from linear to responsive.
**Effort**: Large. Needs design work on what "adaptive" means for this system.
**Seeds**: `PathService.getPathCompletionByContent()`, `DiscoveryAttestationService.getDiscoveryProfile()`.

---

## 3. Recovery Protocol - Sovereignty Through Embeddedness

`doorway/RECOVERY-SPRINT-PLAN.md` describes a 10-sprint plan for social key recovery. A human loses their device but recovers their identity through a constitutional, verifiable process involving their actual relational web — family, congregation, neighbors, coop.

After our conversation about relational trust, this is where philosophy becomes tangible. Your attestations of relationship and history of mutual stewardship ARE your safety net, encoded in the protocol. Not an insurance company, not a government program — your actual people, whose elohim can coordinate the response while preserving your dignity.

**Impact**: Transformative for trust and adoption. Where shefa stops being a model layer and becomes real.
**Effort**: Massive. 10 sprints planned. M4+ work.

---

## 4. Sophia Moment JSON Authoring

The **critical path bottleneck** for M2-M3. Five discovery instruments are defined as metadata records (values hierarchy, attachment style, strengths finder, constitutional reasoning, personal values) but none have actual Sophia moment JSON authored. Without these, the assessment engine has nothing to render.

Content authoring work requiring psychometric instrument design + Perseus/Sophia JSON format. AI can generate drafts but they need human review for psychometric validity.

**Impact**: Critical for M2-M3. Literally the bottleneck.
**Effort**: Large. Cross-disciplinary (psychometrics + JSON schema + content design).

---

## 5. The Governance Immune System (Qahal Write Path)

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
