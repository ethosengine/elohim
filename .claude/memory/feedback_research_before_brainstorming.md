---
name: Research existing design context before reaching for brainstorming
description: When user reframes my proposal architecturally, that's a signal there is existing design context I haven't absorbed — research/debug first, brainstorm only when truly creating from scratch
type: feedback
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
When the user reframes my technical proposal with their own architectural model (e.g., "actually the substrate should do X, doorway just caches"), that is a signal that **the design has already been extensively discussed**. Reaching for the brainstorming skill at that point is wrong — it forces them to re-narrate decisions they've already made.

**Why:** The brainstorming skill is mandated for *creative work*, but the user distinguishes "creating from scratch" from "the existing design has answers I haven't found yet." A reframe is the latter. Treating it as the former wastes their time and signals I'm not absorbing context.

**How to apply:**
- When user reframes architecture, default to: research current state → systematic-debugging if a concrete bug → docs/CLAUDE.md refinement after the fix
- Brainstorming is appropriate when the user themselves is in exploration mode ("I'm not sure how this should work")
- Also: when fixing something that has come up multiple times, refining the relevant CLAUDE.md is part of the fix — so future-me reads context I currently don't have
