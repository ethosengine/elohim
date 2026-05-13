---
name: No Hebrew pillar names in epic/manifesto narrative
description: Epic and manifesto-style narrative docs in genesis/docs/content/elohim-protocol/ should not leak internal Hebrew pillar names (mishpat, qahal, shefa, imagodei, lamad, avodah) — translate to accessible English. "Elohim Protocol" / "elohim" (agent role) are protocol-name / load-bearing vocabulary and stay.
type: feedback
originSessionId: 08cb9ec5-d2e3-405a-8267-36f3a26a38f5
---
Hebrew pillar names are internal project jargon. They organize the design space technically (pillars, manifests, specs, plans, code) but should not appear in public-facing narrative documents — epics, manifesto, persona stories, blog posts, social-reach content.

**Why:** The narrative documents must read as accessible to people outside the project. Hebrew pillar names create insider/outsider dynamics, slow comprehension, and trade poetry for jargon. The epic's own voice is biblical-poetic (Sh'ma, Genesis 2:15, 1 Cor 13:6) without naming pillars by Hebrew labels — the citations land where they belong.

**How to apply:**
- When editing or writing files under `genesis/docs/content/elohim-protocol/`, default to English equivalents:
  - mishpat → "governance" / "the community" / "constitutional rule" / "tribunal"
  - qahal → "the community" / "the collective" / "the assembly"
  - shefa → "economic flow" / "mutual credit" / "the gift economy"
  - imagodei → "identity" / "the human's elohim as counsel" / "relational protections"
  - lamad → "learning" / "the practice"
  - avodah → "work" / "contribution"
- "Elohim Protocol" and "elohim" (as the agent role like "Maria's elohim") **stay** — these are load-bearing protocol names, not pillar jargon.
- Technical specs, plans, brainstorm artifacts, code, and internal design docs may keep Hebrew pillar names — they're appropriate there.
- Filenames are organizational (e.g., `lamad.md` is fine as a filename); body text should still use English where possible.
- When dispatching subagents to write epic/manifesto content, include this guidance in the brief.
