---
title: Che Browser Feedback L1/L2 are landed + verified but plan ledgers show all items OPEN
created: 2026-06-10
domain: process-meta (doc-lifecycle)
source: che-live-peer-dev-loop brainstorm prior-art audit (2026-06-10)
severity: low
---

Both `2026-05-30-che-browser-feedback-foundation-plan.md` (26 items) and
`2026-05-30-che-browser-completion-oracle-plan.md` (20 items) sit at status Draft
with every checkbox unchecked, and their gap-item ledgers show every item OPEN —
yet the implementation is fully landed (look.ts + tests tracked in git, pnpm
overrides lock playwright 1.59.1, devfile PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD:79,
XDG cache pruned to 631MB exactly per plan, a2o CLAUDE.md Tools entry, L2 visual
gate clauses live in agentic-developer SKILL.md). Re-verified working 2026-06-10:
`pnpm look https://doorway-alpha.elohim.host/` → ok:true, screenshot read.
Budget counts ~46 phantom OPEN items. Sync the claims: check the boxes with
verification evidence, flip plan statuses, re-run decompose.py on both plans.
(`look --as <FixtureHuman>` auth path remains genuinely unverified — keep OPEN.)
