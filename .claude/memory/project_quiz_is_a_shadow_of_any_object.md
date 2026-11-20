---
index: false
name: project-quiz-is-a-shadow-of-any-object
title: "Quiz is a shadow of any object"
description: "Operator 2026-10-08: a quiz is not a separate module or step; every piece of content can carry one as its shadow — model and surface assessments as an attribute of the object"
id: project-quiz-is-a-shadow-of-any-object
metadata:
  node_type: memory
  title: "Quiz is a shadow of any object"
  type: project
  originSessionId: 08597c1b-3ceb-4809-8b33-55c0b88bb4d8
  modified: 2026-10-08T00:32:17.137Z
cites:
  - genesis/docs/content/fct/fct-course/recompose.py
---

Operator, 2026-10-08, while the FCT course was being recomposed into a Khan-style walk:
"every piece of content can have a quiz — so a quiz isn't a separate module, it's more
of a shadow of any object." An idea from the project's beginning, not revisited since.

**Why:** the lamad data model already treats an assessment as an attribute of what it
assesses — a `sophia-quiz-json` atom with a `VALIDATES` edge and `assessesContentId`,
never a thing the learner walks to on its own. But the recomposed FCT path (2026-10-08)
surfaces each module's check and reflection as separate path steps in a "Practice"
column, which reads as modules. That is a rendering concession, not the model.

**How to apply:** keep the `VALIDATES` shadow as the truth and move the surface toward
it: a check belongs on the object it shadows (the step viewer's mastery control becomes
"Check your understanding" when a shadow quiz exists; a story or a lane can carry one
too), and the path walk should stop listing quiz steps once that affordance exists —
one switch in `genesis/docs/content/fct/fct-course/recompose.py` (`assessments()` steps).
Never mint a quiz as a sibling module. Related: [[fct-v2-recomposition-intent]],
[[paths-are-walks-over-epr-graph]].
