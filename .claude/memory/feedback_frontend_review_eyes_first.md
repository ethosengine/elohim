---
id: feedback-frontend-review-eyes-first
name: frontend-review-eyes-first
title: Frontend review = eyes first
description: "Render before code-reviewing frontend (pnpm look, graphos sheet); canonical = looking-at-frontend skill; can't-find ≠ never-implemented."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e5328acc-ec5f-4701-8d4f-17dc78dd9c5b
---

**GRADUATED 2026-06-11 → `.claude/skills/looking-at-frontend/SKILL.md`** (TDD'd, committed d8f324d78). The skill is the canonical home; invoke it rather than relying on this note. Two rails live ONLY here + the skill's dispatch section: (1) subagents do NOT self-discover skills — any visual-review dispatch prompt must carry the eyes rail explicitly; (2) the agent registry snapshots at session start — agent-body edits (REQUIRED EYES pointers in component-architect/graphos-designer) only reach dispatches in FRESH sessions.

For any frontend review or UI refinement task, step one is to render and look at the actual surface — `pnpm look <url>` from genesis/a2o (screenshot + console/network/DOM capture), `--as <FixtureHuman>` for authed states, multiple viewports — BEFORE dispatching code-reviewer/pattern-hunter or reading source.

**Why:** On 2026-06-11 the operator challenged my proposed frontend-review rails: "it doesn't seem clear to me that you would have actually looked at the app." I had listed agent-eyes only under end-of-task verification. A 2-minute look at alpha root immediately found a real defect (≈14,000px empty gradient where landing content should be, correlated with a 403 on /db/content/manifesto) that no amount of lint/code review would surface.

**How to apply:** Three look surfaces, pick per task (operator 2026-06-11):
1. **The app(s) themselves** — deployed alpha (`doorway-alpha.elohim.host`) or `pnpm start:alpha` local-UI×live-data.
2. **Storybook (the emergent component library)** — `pnpm graphos list [filter]` enumerates (483 stories: `default-*` Library A blank-slate, `designed-*` Library B themed, narrative `i-iv`, `foundations-*`); `pnpm graphos story <story-id>` renders one; `pnpm graphos sheet <component>` renders the FULL cell/theme matrix (both libraries, labeled sections) as one composite image — the fastest design-language absorption. Deployed base default (storybook.elohim.host); `--base http://localhost:6006` for in-branch work with a local `pnpm storybook`.
3. **Graphos design guide** — same storybook; `pnpm graphos story <docs-id>` auto-renders MDX guide pages (viewMode derived from entry type). Look here to absorb the established aesthetic before refreshing/refining a design.

Discipline: render *the view the operator is describing* and confirm I can actually SEE what they're talking about before designing. **Can't-find ≠ never-implemented** — if the described view doesn't show up, suspect a present-moment reachability issue (broken route, failed data load, env) and investigate (capture.json httpErrors, git history, routes) before concluding absence. Then code review with the visual findings as anchors. Tooling rails: [[agent-eyes-look-live-peer-loop]].
