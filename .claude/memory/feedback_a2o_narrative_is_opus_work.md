---
name: A2o narrative authoring is Opus work; Haiku is glue-only
description: A2o feature files, scenarios, and frontmatter must be authored by Opus — the human story has to land meaningfully into the technical libraries to convey deep value and interpretability. Haiku is fine for glue (step-def wiring, fixtures, helpers) but never for the story itself.
type: feedback
originSessionId: 60007cbf-4a59-4bce-9be7-6e57d1568cf6
---
A2o features carry the human story — the learner's experience as a specification. The `Feature:` block, the scenario `Scenario:` titles, the Given/When/Then narrative, and any frontmatter / tags / persona setup are load-bearing for vision alignment. Haiku writes mechanical, pattern-matched scenarios that pass tests but don't *carry* the story; the result reads as "scenario shaped object" without the deep value or interpretability the format exists to convey.

**Why:** A2o is the bridge between the manifesto (vision) and the technical libraries (substrate). If the bridge is generic, the substrate has no story to land into and the vision has no testable shape. The whole "story-first" workflow in CLAUDE.md depends on this bridge being authored with intent.

**How to apply:**
- A2o feature/scenario authoring → Opus. Always.
- A2o frontmatter, tags, persona setup → Opus.
- Step definition wiring, fixture builders, helper utilities → Sonnet or Haiku is fine.
- Mechanical lift (e.g., `@wip` tag removal after a passing implementation) → Haiku is fine, but Opus must verify the story still reads true after the implementation lands.
- When dispatching agents for a2o coverage work, split: one Opus agent authors/refines the scenarios and frontmatter; separate Sonnet/Haiku agents handle the glue code beneath.
- Watch for the failure mode: an a2o scenario that passes its test but doesn't connect the technical operation to *why a person would care*. That's a Haiku-shaped artifact masquerading as story.
