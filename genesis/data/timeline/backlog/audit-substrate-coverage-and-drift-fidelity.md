---
id: "backlog-audit-substrate-coverage-and-drift-fidelity"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Close audit-script coverage gates and drift-store blindspots before next ceremony"
slug: "audit-substrate-coverage-and-drift-fidelity"
written: "2026-05-15"
author: "cartographer"
status: "envisioned"
priority: "high"
relatedNodeIds:
  - feedback_first_memory_team_ceremony
  - feedback_cascade_hidden_test_surface
  - feedback_self_reinforcing_path_bug_class
  - project_signal_driven_audit_ceremonies
tags: [memory-team, substrate-truth, audit-fidelity, run6-surface]
shift_objective: |
  Repair the audit substrate the memory ceremony stasis math depends on, in three bites:
  (1) extend `.claude/scripts/memory-kit/claude-md-audit.py` to scan ALL CLAUDE.md files in
  the tree (currently 25 of 26; sophia submodule will appear as audit-excluded section,
  not silently dropped) and emit an `audit-excluded:` section so cartographer can see
  scope explicitly each cycle;
  (2) extend the drift accumulator to capture line-count drift per CLAUDE.md file — not
  just edit-signal regressions — so a file growing 180→210 lines between cycles emits
  a budget-regression signal even without a content-edit; (3) add a one-shot
  `claude-md-three-way-diff.py` that compares audit-script output vs `find+wc` vs
  prior chronicle's recorded line-count table, exits non-zero on divergence, and
  prints which files disagree so the cascade-hidden-vs-regression question is
  decidable. Land all three under `.claude/scripts/memory-kit/`; add a single
  pre-push gate that runs them in CI-fast mode.
---

# Body

Run #5 reported CLAUDE.md OVER-BUDGET=1; Run #6 librarian's two-tool sweep (audit-script vs find+wc)
returned 3 vs 4. Two independent paths to "stasis-touchable count" disagreed, and the audit-script
was the lower number — so cartographer's stasis math was working from an undercount. The librarian's
Wave 1 surfaced the gap as a finding; this backlog entry mechanizes the fix.

The shape generalizes (historian Precedent 2): cross-substrate metric-path divergence is now a
named-and-recognized pattern. The bite for this cycle: make the canonical audit substrate
self-aware about its own coverage, not silently scope-limited. The drift accumulator's
edit-signal-only blindspot is the same pattern in a different substrate — line-count regression
is invisible to it today.

Out-of-scope for this entry: classifying sophia's 606-line CLAUDE.md (submodule policy, separate
question). In-scope: ensuring the audit reports "1 file excluded: sophia/CLAUDE.md (submodule)"
so it's visible rather than silently dropped.

Acceptance: after this lands, a re-run of `.claude/scripts/memory-kit/claude-md-audit.py` on
the current tree reports total-files-scanned, audit-excluded count with reasons, and a per-file
line count even for files inside budget. The pre-push gate exits non-zero if any line-count
delta vs the previous run's stored values exceeds threshold without an explicit signal-bypass.
