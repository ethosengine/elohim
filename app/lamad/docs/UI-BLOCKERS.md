# UI Implementation Blockers & Considerations

## Overview
This document tracks architectural friction points encountered during the implementation of the Khan Academy-inspired "Territory Mastery" UI. While no hard blockers prevented the MVP implementation, the following areas may require API refinement for scale.

## 1. Global Completion Performance
**Context:** The "Mastered Elsewhere" feature relies on checking every step's `resourceId` against a global set of completed IDs.
**Current Implementation:** `PathService` calculates this client-side by intersecting `path.steps` with `agentService.getCompletedContentIds()`.
**Concern:** For paths with 100+ steps and agents with 1000+ completed nodes, this O(N*M) operation happens on every path load.
**Recommendation:** Move the "completion status" enrichment to the backend/Holochain zome. The `get_path_context` zome call should return the completion status for each step directly.

## 2. Chapter Metadata in Flat Paths
**Context:** The UI gracefully handles both flat paths and chapter-based paths. However, `PathChapter` metadata (like `estimatedDuration` per chapter) is calculated client-side for flat paths if we want to group them logically.
**Recommendation:** Ensure `LearningPath` model always optionally supports `chapters`. If a path is flat, the API might consider returning a single "default" chapter to simplify UI logic, or the UI continues handling both structures (as currently implemented).

## 3. Nested Path Deep Linking
**Context:** When a step represents a nested path (`stepType: 'path'`), the UI renders a card to "Start Sub-Journey".
**Friction:** The current `PathStep` model stores `resourceId` which points to the nested path ID. The UI has to fetch that path's metadata separately to show its title/duration.
**Recommendation:** `PathStep` should include a lightweight `nestedPathSummary` (title, duration, stepCount) to avoid N+1 queries when rendering a list of steps that includes nested paths.

## 4. Breadcrumb Context
**Context:** Deep breadcrumbs (`Territory > Path > Chapter > Step`) rely on `PathNavigator` having full context.
**Status:** Implemented successfully using `sidebarChapters` state.
**Future Need:** If we implement deep linking directly to a step without loading the full path context first (e.g. standalone step view), we lose the breadcrumb hierarchy. Ensure `PathService.getPathStep` always returns enough parent context (Chapter ID/Title) to reconstruct breadcrumbs.
