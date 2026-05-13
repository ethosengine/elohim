---
name: Audit ceremonies are signal-triggered, not time-triggered
description: Accumulate deterministic signal between audits; trigger the deep audit only when threshold is crossed. Pattern reused from EPR feedback. Cheap accumulator + expensive ceremony is the structure for any always-loaded/gospel-treated surface.
type: project
---

When a surface is treated as authoritative (always-loaded, "gospel"), audit cost vs frequency has a sharp tradeoff. Per-call audit is wasteful when drift is rare. Time-cadence audit (every N days) is wasteful when nothing has changed. **The right pattern is signal accumulation: cheap deterministic counters during normal work + ceremony when threshold crosses.**

The pattern instantiates as:
1. **Accumulator** — runs on existing hook events (e.g. PostToolUse on Edit), increments cheap counters in a JSON store. Cost: single-digit ms per call. Never blocks.
2. **Ceremony** — operator-invoked or surfaced by SessionStart when threshold crossed. Reads accumulated signal + does deep analysis. Read-only by default; operator-gated mutations.
3. **Reset on audit** — ceremony resets the file's signal counters so the next cycle starts net-new.

First instantiation (2026-05-13): `claude-md-review` for the 16 CLAUDE.md files in the elohim repo. Accumulator at `.claude/hooks/claude-md-drift-signal.py`; ceremony at `.claude/scripts/memory-kit/claude-md-audit.py`; signal store at `.claude/memory-kit/claude-md-drift.json`.

**Why:** Same pattern as the EPR feedback system in the protocol (`signal_kind` vocabulary accumulates → threshold → mandatory review). Same pattern as nervous-system sensors (accumulate → threshold → action potential). Same pattern as CI orchestrator (changes accumulate → trigger appropriate pipeline). Reusing the shape across substrates keeps the system internally consistent.

**Trust-compute gradient applies:** The accumulator does the cheapest possible work by default. As signal gets noisier / time-since-audit grows / diff sizes grow, the accumulator can be tuned to compute more (full git-diff in scope, change-velocity tracking, content-similarity to prior audits). Weights are re-tunable in `SCORE_WEIGHTS` without changing the protocol.

**How to apply:**
- Any new "always-loaded" or "gospel-treated" surface (CLAUDE.md, skill descriptions, manifests) should use this pattern, not per-call audit and not fixed-cadence audit
- Resist building hook-based audits when the work is heavy — defer to ceremony, accumulate signal in hooks
- When a ceremony exists, document its signal sources explicitly so the trigger threshold is debuggable
- Do not auto-act on accumulated signal — surface for operator decision. Mutations to authoritative surfaces are always operator-gated
