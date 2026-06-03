---
kind: chronicle
status: noted
date: 2026-06-02
ceremony: substrate-currency
surfaces_rewritten:
  - .claude/agents/rust-architect.md
  - .claude/skills/epr-content-addressing/SKILL.md
  - .claude/agents/librarian.md
coherence_verdict: GREEN
next_topic_sampled: "Add a new EPR view type to elohim-storage + expose via /api/v1/epr/{cid} + reach earned at compose-time"
---

## What changed

Three gospel-tier surfaces rewritten in one dynamic-workflow cycle (15 agents).
**rust-architect.md** (+38) — cartographer's coverage-gap lens was the dominant driver:
eight absent substrate components added (`graph/` CozoDB engine distinct from `graph_views/`,
the iroh Observation plane, EPR predecessor/back_prop lifecycle, conductor agent-info gossip,
`replicates_dwelling_service` + prioritizer, `RateHistory` + `economic_events.bounded_by`,
IntegrityNotify as a third reconcile collaborator, a mishpat coordinator sampling); historian
added 7 missing canonical-discipline citations; librarian flagged 5 process-status phrases
(`cutover gate`, `post-Phase-11`, "currently README-only", `backburnered`, iroh-rollout
temporal framing) excised per `[[feedback_agent_prompts_no_process_status]]`.
**epr-content-addressing/SKILL.md** (+79) — storyteller's own narrative lens drove it: three
causality reframes (reach-earned-not-declared, trust-as-efficiency, stewardship-not-possession)
plus the mechanical `holochain/sdk`→`elohim/sdk` and `epr-ref.ts`→`@elohim/service` reorg fixes.
**librarian.md** (+48) — cartographer's verified substrate-drift drove it; standout was resolving
a lens-disagreement on the map-drift accumulator: the original cited `map-drift.json` (wrong);
the live hook `map-drift-signal.py` writes `map-currency-drift.json` (confirmed against
`placement-audit.py:678`), absent on disk only because lazily-created with no current drift.

## Coherence-check sampling

Sampled topic: authoring a new EPR view + `/api/v1/epr/{cid}` route + compose-time reach —
deliberately spanning the two substrate-heavy rewrites' shared surface. Fresh-context Explore
agent traced 15+ citations and 8 core paths: GREEN. No contradictions, gaps, or stale citations.
The one observation (`request_offer_service.rs` named alongside the live `exchange_service.rs`)
is the intended retired-name reframe, not drift — no action.

## Wisdom worth carrying forward

Raw audit rank ≠ real-drift rank this cycle: two of the top-4 ranked surfaces
(`doorway/doorway-service/CLAUDE.md` + its worktree dup, 19 findings each) were pure
relative-path-resolution false positives — every flagged path actually resolved — and belong to
`/hygiene-sweep`, not the four-lens ceremony. The genuinely ceremony-worthy drift lived in the
agent prompts (process-status + one genuinely-gone path) and the reference skill (directory-reorg
rot, ranked #5). Next Phase-1 triage: de-rate CLAUDE.md path-drift as hygiene-sweep noise and
rank surfaces by REAL drift after the librarian-prologue de-rating, not by raw finding count.
