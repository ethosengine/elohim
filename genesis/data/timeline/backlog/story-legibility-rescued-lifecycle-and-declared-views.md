---
id: "backlog-story-legibility-rescued-lifecycle-and-declared-views"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Two rescued devflow stories read as REVISE to a cold reader: their core vocabulary is not grounded"
slug: "story-legibility-rescued-lifecycle-and-declared-views"
written: "2026-10-29"
author: "claude (overnight integration pass, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "medium"
ci_status: none
jobs: []
tags: [a2o, story, blind-reader, legibility, devflow, lifecycle, declared-views]
---

# Two rescued devflow stories read as REVISE to a cold reader

Both features came back from the 0ee5dfde8 power-loss rescue with the big train
(port/g-avodah and port/g-native). Each had one context-isolated blind-reader pass on
2026-10-29. Both verdicts were **REVISE** with **0 correctness** findings: the scenarios
prove what their titles promise, but a stranger cannot follow the vocabulary.

The integration made one technical edit. It removed the feature-level `@wip` and the
stale "the SDK arrives in a separate port" paragraph from the lifecycle feature,
because that port has now landed. The reader confirmed that this edit reads cleanly. The
findings below are about the prose as it was rescued. Revising that prose is story
authoring, so the steward sets the variance floor before a revision loop starts.

## `genesis/a2o/features/devflow/portable-app-lifecycle.feature`

Verdict REVISE: 0 correctness, 6 interpretability, 2 preference.

1. The Rakia/Ark glossary does not say what kind of thing each one is, or what
   "admits power and resources" means.
2. "Device consent" (scenario 3) is not defined.
3. "Compatible local capabilities" (scenario 4) names no capability.
4. "Declared stream position" needs a glossary entry: what a position is, and who declares it.
5. Minors: "recoverable executable bytes", "lane", who creates an "explicit continuation",
   and the `# Category:` comments, which could be machine-readable tags.

## `genesis/a2o/features/devflow/declared-local-views.feature`

Verdict REVISE: 0 correctness, 7 interpretability, 2 preference.

1. "Local view declaration", the central concept, is not grounded. What form does a
   declaration take?
2. "Structured acceptance and measurement readings" (scenario 3) are not described, yet
   that scenario is the direct proof of the feature's "So that".
3. The fixtures are fictional, but only a comment says so (line 9), and many consumers
   strip comments.
4. "Subjects" and "checkout" are unexplained.
5. Minors: the native binary is not named, and neither is the "workflow store". Other
   minors: "moved plots", "source address", the `@concern:idle-is-free` link, and
   scenario 2's two When/Then pairs.

## Done when

The steward rules the variance floor. Each feature then gets a revision, followed by a
fresh blind reader each round until its verdict is READY. The declared-views and
app-lifecycle cucumber profiles must stay green throughout. Today they run 8 scenarios,
32 steps (declared-views), and 2 scenarios, 6 steps (app-lifecycle).
