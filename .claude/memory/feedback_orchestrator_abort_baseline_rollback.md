---
name: Aborting orchestrator runs forces full-chain rebuild on next push
description: Push during an in-flight orchestrator chain superseded the prior orchestrator without persisting its build state — next push rebases against last-SUCCESSFUL orchestrator, pulling in everything that ran in the aborted attempt
type: feedback
originSessionId: 46cb3fbb-28fb-454c-b337-8d4d482a814d
---
When a push during an in-flight orchestrator triggers `disableConcurrentBuilds(abortPrevious: true)`, the aborted orchestrator never persists its updated build state (`build-graph-sha256-*` artifact). The next orchestrator's "Determine Build Plan" stage rolls back its baseline to the last *successful* orchestrator, NOT the last attempted one.

**Why:** the build-graph-walker reads its previous-state artifact via `copyArtifacts` from the prior successful orchestrator. ABORTED runs are skipped. So if you have a chain (#A SUCCESS → #B BUILDING → push aborts #B → #C starts), #C's baseline is from #A, and any commits pushed to land in #B are now in #C's changeset.

**How to apply:**

- A "small" push during an in-flight chain isn't small — it inherits the changeset of the aborted run plus your new commit.
- Plan iteration timing around chain-completion windows, not push-and-go.
- If you must push mid-chain, predict that the next chain will rebuild everything in the prior attempt's changeset. Don't expect graph-walker's "look at this single file change" optimization to apply.
- The pipeline-level Jenkinsfiles also currently ignore graph-walker's step-level rebuild set (e.g. holochain Jenkinsfile rebuilds full pipeline even when only `schema-dna` is in the rebuild set), so the over-build cascades.

Applies to: any shift where you're tempted to push mid-chain to retrigger; any iteration economics calculation that assumes graph-walker = small dispatch.
