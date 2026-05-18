---
name: shift-can-fix-unparseable-test-fixtures
description: shift principle #6 "test fixtures off-limits" doesn't apply when the fixture is GRAMMATICALLY UNPARSEABLE and dropping the whole run; restoring parseability is removing a gate, not moving the goalpost
metadata:
  type: feedback
---

Agentic-developer shift principle #6 ("Objective, measure command, files the measure reads, test runners, and test fixtures are off-limits") protects against moving goalposts: editing scenarios so they pass without changing the system. It does NOT prohibit fixing test fixtures that the runner currently can't load AT ALL.

When a Gherkin parse error / TypeScript syntax error / YAML schema violation in a fixture file is causing the runner to drop the entire suite at AST construction, every scenario the runner SHOULD reach is unreachable. Fixing the grammar isn't moving the goalpost — it's removing a gate that's blocking the playing field. The shift's measure can't even start to discriminate good from bad until the fixture parses.

**Why:** Surfaced 2026-05-17 by the genesis-cucumber-parse-recovery shift. Three bare continuation lines in two `.feature` files were causing Cucumber to drop the entire E2E run at AST construction. sprint-report wrote 0 scenarios → cucumber-report-empty class. Every other (non-@wip) scenario the runner should have executed was unreachable. The "right" move per a strict reading of #6 (bail with proposal for a separate shift) would have left the entire E2E surface broken for the duration of that handoff. The generative reading — fix the grammar to restore parseability — fit cleanly and the affected scenarios were @wip anyway.

**How to apply:**

1. The fixture must be **structurally** broken (parse failure, schema violation, encoding error) — not just failing assertions or yielding wrong results. "It doesn't compile" qualifies; "it asserts the wrong thing" does not.
2. Declare the scope expansion **up front** in the journal's kickoff or iteration-1 stanza with the rationale, not as a mid-shift "while I'm here" expansion.
3. Constrain edits **narrowly** — only the lines that have the grammar/structure error, never adjacent scenario logic.
4. If the affected scenarios are `@wip` or otherwise excluded from the graded surface, prefer fixing those over scenarios that are actively graded — the @wip ones aren't moving the bar today; the graded ones are.
5. Smoke-test the fixture locally with the runner's parser (cucumber-js --dry-run, tsc --noEmit, schema-validate) BEFORE pushing.

See also: [[memory-in-repo-two-tier]] for the parallel "fixtures live in the repo so they're co-edited with code" pattern at the memory layer; [[story-first-default]] for the upstream framing that feature files describe the human experience.
