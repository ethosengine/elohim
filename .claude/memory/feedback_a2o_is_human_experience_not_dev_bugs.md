---
name: a2o features describe human experiences, not dev infrastructure bugs
description: Story-harvest captures human-facing engineering constraints; serialization bugs, schema sync, type-check failures belong in unit tests / pre-push hooks / memory — NOT in feature files
type: feedback
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
**The bar for an a2o feature is "this describes what a human goes through."**

Things that DO belong in `.feature` files:
- "I lost my key and my emergency contacts help me recover"
- "When I graduate to peer-steward, my doorway login hands off to my own infrastructure"
- "My elohim acts as defender on my behalf and I see a clear record"
- Parameter-bearing constraints that the human experiences: "recovery completes within N minutes even at low connectivity," "my key rotation propagates to peers within M seconds"

Things that do NOT belong in feature files (no matter how painful the bug was):
- Internal serialization round-trip bugs (`serde_json::Value` at zome boundary)
- Schema codegen distribution sync (4 copies in lockstep)
- Type-system conventions ($id format, naming patterns)
- 503 stub contracts between layers
- Build-system mandates (fresh-tree, cargo fmt, clippy)

These are things that **should just work**. They're caught by unit tests, schema-contract tests, pre-push hooks, ESLint rules, type-check failures — or as memory entries that future sprints reference. Documenting them as Gherkin scenarios is the wrong shape: a learner doesn't experience a serialization round-trip, they experience an account that works or doesn't.

**Why:** a2o is the protocol-vision-to-acceptance bridge. Conceptual scenarios in `genesis/docs/content/elohim-protocol/` describe what learners experience; executable scenarios in `genesis/a2o/features/` test that the system delivers it. Dev infrastructure concerns aren't part of that bridge — they're plumbing that supports it.

**How to apply:**

- When story-harvesting after a debug session or sprint finish, ask: "Did the human's experience change because of this discovery, or did it just teach me something about the codebase?" Only the former harvests.
- If the constraint affects operator presets, peer diversity config, or human-observable timing/behavior — yes, harvest. If it's a code convention or a layer-internal contract — no, doesn't belong in features.
- The right place for dev-plumbing constraints: pre-push hooks (process), unit tests (mechanical), schema contracts (type), memory entries (judgment), CONVENTIONS.md docs (convention).
- Failure-then-fix cycle on plumbing produces a memory + a guard test. NOT a feature file.
