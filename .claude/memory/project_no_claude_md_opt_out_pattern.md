---
name: `.no-claude.md` opt-out marker — captured decisions for heuristic false-positives
description: When the claude-md-review audit flags a directory as MISSING-CLAUDE-MD but the operator concludes no doc is needed, a `.no-claude.md` marker file in that directory carries the rationale. Future audits surface the marker (with rationale excerpt) rather than re-flagging. Pattern generalizes to other false-positive-prone heuristics.
type: project
---

The `claude-md-review` audit flags directories that look like they need a CLAUDE.md (≥15 files OR ≥4 subdirs, ≥2 distinct architectural extensions, nearest ancestor ≥2 levels up). Some flags are real; some are false-positives (parent doc covers it, design-asset directory, component-flat-tree pattern).

**Pattern**: when the operator decides "no, this directory doesn't need a CLAUDE.md," they drop a `.no-claude.md` marker file in that directory with the rationale. The audit:
- Excludes the directory from MISSING-CLAUDE-MD candidacy
- Lists it in a separate "Opted out" section with a rationale excerpt
- Preserves the decision chain across audit cycles

**Marker file format**:
```markdown
---
decided: 2026-05-13
revisit-if: directory grows to contain .ts/.rs/.py source code
---

# No CLAUDE.md needed here

This directory contains static design assets... <full rationale>
```

The frontmatter is parsed by `_lib.frontmatter` to extract `decided` date and `revisit-if` triggers. The body's first non-heading paragraph becomes the rationale excerpt in the audit report.

**Why this matters:**
- Heuristic-based audits will always have false positives. Without an opt-out mechanism, operators end up annoyed by the same flags every cycle, or worse — start ignoring the audit entirely
- Marker files capture the *reasoning* not just the *decision* — surrounding CLAUDE.md updates can reference what's been considered
- The decision is reversible: remove the marker and the directory becomes a candidate again. Matches the lifecycle principle of "archive, never delete"

**How to apply:**
- Generalize to other heuristic audits: `skill-audit` could honor `.no-skill-audit` markers for skills that legitimately have short descriptions; `dedupe-memory-scan` could honor markers on entries that look duplicate but are intentionally redundant
- Keep markers small — they should be operator-readable in a few seconds. The audit only shows an excerpt anyway
- Stamp `revisit-if` conditions so the marker has an expiry signal: "remove this marker if X happens"
- Markers in the repo are team-shareable (track with git) — onboarding contributors see the prior decisions
