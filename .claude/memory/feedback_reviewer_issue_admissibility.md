---
name: reviewer-issue-admissibility
description: Autonomous review loops deadlock on issues the implementer cannot fix — constrain reviewers to tree-fixable findings
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ec566588-36bb-4cb6-a181-0caedd86b2a0
---

In autonomous implement→review→fix loops (SDD workflows), spec reviewers will deadlock the loop by raising **process-historical** issues no fixer can resolve: "commit X staged an extra file" (history is append-only on shared branches — see [[concurrent-sessions-shared-worktree]]), or "RED-state evidence not preserved in git" (TDD transients never leave git artifacts). Both sides behave correctly and the loop never converges (observed 2026-06-04, quilt-policy SDD run: 2 full fix rounds burned on two unfixable complaints while the code was substantively compliant).

**Why:** a reviewer prompted to "verify every step" has no concept of fixability; an implementer correctly refusing to rewrite history reads as non-compliance.

**How to apply:** every reviewer prompt in an autonomous loop gets an *issue-admissibility* clause: (1) judge tree+history AS-IS, never demand history rewrites; (2) evidence pasted in the implementer's report satisfies transient-state steps; (3) a minimal, separately-committed, documented pre-existing-bug fix is a NOTE, not an issue; verdict = compliant when only NOTE-class findings remain. Implementer prompts get the dual: prerequisite fixes go in a SEPARATE scoped commit, history is append-only.
