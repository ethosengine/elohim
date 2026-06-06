---
id: feedback-deterministic-flag-agent-canon-stasis-pattern
name: deterministic-flag-agent-canon-stasis-pattern
description: "Operator's preferred architecture for event→agent automation — deterministic flag, background Opus dispatch, canonical cite-sealed backlog with documented status, deterministic suppression on re-encounter, ceremony-pattern stasis sweep"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: f7b519f3-c80d-4332-9e5b-43c404ca08dc
cites:
  - findings-sentinel-pattern-design | the canonical spec this memory points to — compose new event→agent automation from its §1, dont re-derive the pattern from this entry | sha256:c284074fe38e2450 | path: genesis/docs/superpowers/specs/2026-06-06-findings-sentinel-pattern-design.md
  - .claude/hooks/deprecation-sentinel.py
---

When building "react to X automatically" tooling in this repo (2026-06-06, deprecation sentinel), the operator specified a four-layer architecture — reuse it for future event-driven automation (security advisories, flaky tests, lint-debt classes):

1. **Deterministic flag**: cheap hook fingerprints the signal into a JSONL ledger (`.claude/data/*.jsonl`); dedupe by fingerprint.
2. **NEW → full Opus agent dispatch (background)**: scope everything using the feature, canonicalize into `genesis/data/timeline/backlog/` (cite-sealed via cite-gen, `status:` documented), then plan→fanout→implement when bounded; blockers flow back into the canonical entry.
3. **Re-encounter → deterministic citation, never re-fire**: known fingerprint surfaces the canonical entry's current decision (once per session); blocked-and-canonicalized is terminal for automation.
4. **Stasis sweep**: ceremony-pattern skill (measure→dispatch→re-measure, sibling of /memory-stasis-loop) drains to stasis; manual, /loop, or routine.
5. **Full memory decomposition at close** (operator refinement, same day): the ledger is the existing-positives check surface — presence suppresses dispatch, absence fires the dev. FIXED items are DELETED from ledger AND backlog (verifying commit = the record; chronicle graduation rare) — no terminal tombstones; everything in a backlog has a live trajectory or it's not there. Reintroduction reads as NEW → re-fires = regression handling for free.

**Why:** builds on the managed-memory decomposition disciplines (specs/plans → canonical decompositions, disciplined citation tooling) — "fully-automated, no seams" but never agent-storms on known-blocked items.

**How to apply:** CANONICAL SPEC now exists — `genesis/docs/superpowers/specs/2026-06-06-findings-sentinel-pattern-design.md` (compose from §1; don't re-derive from this memory). Instantiation A: deprecation/security trio (`.claude/hooks/deprecation-sentinel.py` + `deprecation-triage` agent + `deprecation-stasis` skill, bf6e38b49). Instantiation B: CI findings (`.claude/scripts/ci-harvest.py` cursor/occurrence/green-streak harvester + `ci-failure-triage` agent, 8041c949e) — adds remote-source harvest triggers, urgent-vs-backlog split, and closure-by-disappearance (sweep-owned deletion, not agent-asserted). Related: [[managed-surface-edit-discipline]].
