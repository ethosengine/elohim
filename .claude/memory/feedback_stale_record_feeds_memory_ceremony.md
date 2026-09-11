---
name: feedback_stale_record_feeds_memory_ceremony
title: Stale record feeds the memory-ceremony
id: feedback-stale-record-feeds-memory-ceremony
description: "Record stale gospel/memory claims via `epr flow note --kind correction`; ceremony Phase 0 reads them — bites when a claim is found stale."
metadata:
  type: feedback
---

When a session finds a stale claim in a gospel surface (CLAUDE.md, a skill, an agent doc), a
memory note, or a script's rationale comment, record it AT DISCOVERY — not in the final summary:

```
epr flow note --on <surface path> --kind correction --reason 'STALE (<date>): "<claim verbatim>" → <what is true now> (<evidence>)'
```

One note per stale CLAIM, claim quoted verbatim, replacement truth in the same note. The
memory-ceremony's Phase 0 reads the whole record across surfaces with
`epr flow concerns --corrections [--since <date>]` and treats every surface
with a witnessed correction as a pre-triaged candidate BEFORE the population-wide audit.

**Why:** the operator (2026-09-06): the ceremony is "way overdue" but "expensive" and "not
particularly convinced of its effectiveness"; an in-flight place for agents to note "old
things/references that need to be cleaned away" makes it "mechanically more effective and token
efficient." Witnessed staleness carries claim + truth + evidence, so the deep-read starts from a
known delta instead of re-deriving drift from a scan.
**How to apply:** never write "note for the record" in prose alone — emit the flow note. It is
not a new ledger (REA run-note events in `.eprfs/status/flows.jsonl`; `epr flow ledger <path>`
renders one surface). Ceremony absorptions close the entry in the Phase 4c chronicle by surface +
date. Package-first trap: the ceremony skill's source is `.epr-meta/elohim/packages/skills/memory-ceremony.json`
`instructions.body` — editing `.claude/skills/memory-ceremony/SKILL.md` directly is overwritten by
`just codegen agents write` ([[feedback_package_first_projection_drift_direction]]).
See [[memory-ceremony]], [[feedback_deterministic_flag_agent_canon_stasis_pattern]].
