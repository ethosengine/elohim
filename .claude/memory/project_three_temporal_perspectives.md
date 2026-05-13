---
name: Three temporal perspectives — history / roadmap / development
description: The temporal architecture for memory + epics. History is perspective on the past timeline, roadmap on the future timeline, development is the present cycle that bridges them. Memory-kit and converge serve this frame.
type: project
originSessionId: b5ef4833-2583-4482-b36e-b595da75dafe
---
The user named this on 2026-05-13 while reframing memory-kit:

- **History** = perspective on the timeline of the past
- **Roadmap** = perspective on the timeline of epics planned/wip for the future
- **Development** = the cycle on which we work to change the present to achieve the vision of a future of human thriving

The three perspectives are views on a single substrate: the **epic-graph over time**. Each epic at `genesis/docs/content/elohim-protocol/` is a snapshot of the protocol's becoming; the git history of those documents is the actual narrative. History reads the diff backward, roadmap reads it forward, development is the loop that produces each new commit.

**Why:** It dissolves a category confusion in our tooling. We had memory-kit (hygiene), converge (synthesis), epics (narrative), plans (futures), archive (past). Without the temporal frame they were a pile of utilities; with it they're aligned services on a common substrate. The historian and the roadmap-walker are inverse motions on the same timeline.

**How to apply:**
- When designing any memory/planning/narrative tool, ask which perspective it serves and what timeline-slice it operates on
- Treat the epic-graph (git history of `genesis/docs/content/elohim-protocol/`) as a first-class data source, not a static reference
- Frame new tools relative to this triad — historian (past-facing surface agent), roadmap-walker (future-facing projection), development cycle (the present-tense loop that memory-kit + converge already serve)
- Don't conflate the three: development tools (memory-kit/converge) should not pretend to be history (that's historian's job) and should not author roadmap (that's a different epic-authoring motion)
