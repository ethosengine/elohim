---
id: "backlog-fix-audit-script-discovery"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fix audit-script discovery (doubled .claude/.claude/ path, mis-scoped audit)"
slug: "fix-audit-script-discovery"
written: "2026-05-14"
author: "cartographer"
status: "ready"
priority: "high"
relatedNodeIds:
  - "memory:project_signal_driven_audit_ceremonies"
  - "memory:project_shared_lib_pattern"
  - "memory:feedback_cascade_hidden_test_surface"
tags: [memkit, infra, cascade-root, audit]
shift_objective: |
  Fix the audit-script discovery bug across .claude/scripts/memory-kit/.
  Three scripts write reports to a doubled .claude/.claude/<date>/ path; claude-md-audit.py
  audits only 1 of 25+ files in the drift store; agent-audit.py audited 0 agents despite
  staged modifications; cleanup-scan reports 0 specs / 0 plans. Root cause is cwd-vs-repo-root
  resolution: scripts assume cwd is repo root but are invoked from .claude/scripts/. Resolution
  is to route all path resolution through _lib.paths.repo_root_from_file (the canonical bootstrap-
  by-walk-up helper already used by hooks). After the fix, re-run claude-md-audit and agent-audit
  and surface the now-visible drift counters as a Wave-4-or-next-ceremony input. Done when:
  (a) no script writes to .claude/.claude/; (b) claude-md-audit.py audits every file in the
  drift store; (c) agent-audit.py audits every modified .claude/agents/*.md; (d) reports for
  today exist at .claude/memory-kit/2026-05-14/ (single .claude/) and contain non-zero scanned counts.
---

# Fix audit-script discovery

## Why this matters

The signal-driven audit ceremony (landed 2026-05-13) is the substrate that decides when
CLAUDE.md, MEMORY.md, agents/, and skills/ need attention. If the audit can't see what it's
supposed to see, the ceremony is a placebo. The librarian's Wave-1 survey found three
concrete failure modes (doubled path, mis-scoped CLAUDE.md audit, zero-agents audit) and a
suspicious cleanup-scan reading 0 specs / 0 plans. Until these are fixed, every downstream
signal (drift counts, dedupe candidates, missing-CLAUDE-MD lists) is suspect.

## What's blocking

Nothing. The bug is local to .claude/scripts/memory-kit/; the fix is a refactor through
the existing _lib.paths helper. No protocol/DHT touch; no schema work.

## What's ready

- _lib.paths.repo_root_from_file already exists and is canonical (project_shared_lib_pattern)
- Hooks already use it correctly; this is bringing scripts up to the same standard
- Drift accumulator state is preserved at .claude/memory-kit/claude-md-drift.json
  so a re-run after the fix produces immediately-useful signal

## Who knows the area

Librarian (this ceremony's Wave-1 author). Cartographer can dispatch directly.

## Cascade implication

Per feedback_cascade_hidden_test_surface: expect the post-fix audit run to reveal MORE
drift, not less. Track ratio (drift-files-flagged / drift-files-in-store) as the
fix-confirmation signal, not absolute counts.

## Convergence

- Librarian Wave 1: infra bugs section (primary signal)
- Historian Wave 1: cited cascade-hidden-test-surface precedent
- Storyteller Wave 2: JOINT ACTION prerequisite

## Definition of done

1. Single .claude/ path in all script outputs
2. Re-run claude-md-audit; report covers every drift-store entry
3. Re-run agent-audit; report covers every staged .claude/agents/*.md
4. Investigate cleanup-scan 0-specs/0-plans — either fix scope or document as expected
5. Memory entry update: append validation note to project_signal_driven_audit_ceremonies

## Run #2 addendum (2026-05-14, second ceremony)

Run #2 librarian confirmed across two consecutive runs that the following audit signals
are **durable false-positives** of the audit scripts themselves, not real drift:

- **agent-audit `TOOLS-MISMATCH` (19/19 agents)** — declared tools list in agent frontmatter
  (`Task`, `Bash`, `Glob`, etc.) does not match the audit's grep for in-body tool use.
  Structural mismatch between agent-frontmatter convention and the audit's discovery method;
  no agent actually has a tools-list drift.
- **agent-audit `OVER-IMPERATIVE` (18/19 agents)** — directive-density threshold is set
  too low for agents (which by design carry imperative language); same exact agents flagged
  both runs, with no real over-imperative content.
- **memory-review `index→missing` (2 entries)** — script does not resolve `../`-prefixed
  relative paths from `MEMORY.md`. Files exist at
  `memory-kit/horizon-scans/2026-05-14.md` and `skills/memory-ceremony/SKILL.md` but
  the audit reports them as missing.

These three classes become **part of the Definition of Done**: after the audit-script fix,
re-run should produce zero hits on these three false-positive shapes specifically. The
true `TOOLS-MISMATCH` and `OVER-IMPERATIVE` signals (if any) should appear with different
agent IDs than the durable-false-positive set.

A new signal-accumulator counter `audit-script-false-positive-rate` is proposed: when run
N confirms run N-1's classifications without operator review, increment toward a
"fix-the-audit-script" threshold rather than continuing to surface noise.
