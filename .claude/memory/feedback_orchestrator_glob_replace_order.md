---
name: matchesGlob ** corruption — replace order matters
description: build-graph.groovy:matchesGlob silently broke ** for nested files; sentinel placeholders required before single-* substitution
type: feedback
originSessionId: 06d0c187-b5c7-44a6-b8dd-2e89e7827615
---
In `genesis/orchestrator/build-graph.groovy:matchesGlob`, glob → regex compilation must substitute `**` tokens with a placeholder *before* the single-`*` pass. Otherwise the sequence (`**` → `.*`, then `*` → `[^/]*`) corrupts the `.*` from step 1 into `.[^/]*` — silently restricting `src/**` to one-level-deep matches. Three-levels-deep file edits bypass change detection.

**Why:** Found when orchestrator #765 didn't trigger `elohim-edge` for the doorway routing fix (April 28). Files at `doorway/doorway-service/src/server/http.rs` (depth 3 under src) didn't match `doorway/doorway-service/src/**` because the regex compiled to `doorway/doorway-service/src/.[^/]*` instead of `doorway/doorway-service/src/.*`. Same corruption silently affected every `**` pattern across all 9 `build-manifest.json` files for an unknown duration — likely explains earlier "deployed but didn't take" episodes.

**How to apply:** Any future glob-to-regex compilation in Groovy, JS, or anywhere must use sentinel placeholders for compound tokens (`**/`, `**`) before substituting the simpler tokens (`*`, `?`). When auditing CI failures where "the change is in but didn't deploy," check whether change detection actually marked the right step stale — empty `graphPipelines` is the silent symptom. Don't trust `analyzePipelineRequirements` advisory output to catch this; that uses `startsWith` and disagrees with `matchesGlob`, but the divergence advisory only logs a warning, not a failure.
