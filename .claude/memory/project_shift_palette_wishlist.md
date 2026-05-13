---
name: Shift palette evolves via wishlist feedback loop
description: Agentic developer shifts produce palette deltas — commands Opus wanted to run but couldn't — that refine the allowlist over time without manual curation.
type: project
originSessionId: 9a934a92-144d-4415-9d43-14fcb046e2db
---
For agentic developer shifts in this project, permission prompts are blocking — if Opus hits a novel bash command at 3am, the shift stalls until the human wakes up. So the palette must be predictive, and it must improve between shifts.

**Mechanism:**

1. **At kickoff:** Opus proposes the expected command palette for the current Objective. User approves in bulk. Palette entries are tagged shift-scoped and written to `.claude/settings.local.json`.
2. **During iteration:** Before each bash invocation, Opus pattern-matches the command against the allowlist. If no match, Opus does NOT invoke bash. It logs the intended command + purpose to the journal's wishlist section and either finds an alternative or bails.
3. **At close/bail:** The shift result `.md` includes a "Proposed palette additions for next shift" section — every wishlist entry with its purpose and the reason it was needed. Human reviews this before the next shift; approved entries graduate into the durable palette.

**Why:** The pain of writing a complete palette up-front is that you don't know what you'll need. The pain of building it ad-hoc is blocking prompts mid-night. The wishlist resolves both: you always have enough palette to not block, and you always have an ordered list of what to add next, based on real evidence.

**How to apply:**
- When designing any autonomous-agent system with bash/tool access, include a wishlist mechanism: "commands the agent wanted but didn't have."
- Never let the agent try bash commands outside the palette — it blocks the loop. Always check palette first, log wishlist, work around or bail.
- Shift result documents should always surface the proposed palette additions prominently (not buried in a log).
- The durable repo palette grows from wishlist evidence, not from guesses.
