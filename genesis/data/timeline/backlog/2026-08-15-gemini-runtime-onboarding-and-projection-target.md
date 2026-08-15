---
id: "backlog-gemini-runtime-onboarding-and-projection-target"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gemini runtime: onboard yourself, then build the gemini projection target so packaged personas (blind-reader, storyteller) run on gemini-3.1-pro"
slug: "gemini-runtime-onboarding-and-projection-target"
written: "2026-08-15"
author: "orchestrator"
status: "open"
priority: "medium"
area: "agents"
domain: "tooling"
tags: [gemini, agents, packages, projections, onboarding, actor-plane, tooling-domain]
shift_objective: |
  Operator directive (2026-08-15): the blind-reader and storyteller personas should be
  runnable on gemini-3.1-pro as well as their pinned claude-opus-4-6. Claude Code cannot
  execute Gemini models as subagents, so this lands as a RUNTIME PROJECTION: the
  elohim-native agent packages grow a gemini projection target, and the Gemini CLI becomes
  a first-class persona runtime beside claude and codex. This item is written FOR a Gemini
  session to claim — Part A onboards you; Part B is the build.

  ── Part A: onboard yourself (do this before touching anything) ──
  1. Environment: your OAuth flow previously died on `spawn xdg-open ENOENT` — the
     workspace is headless. `NO_BROWSER=true` is now exported in devfile.yaml (takes
     effect on workspace restart; until then run with it set inline). Expect to print the
     auth URL and let the operator open it outside the workspace.
  2. Gospel: read /projects/elohim/CLAUDE.md end to end — it is runtime-agnostic law
     despite the filename (build commands, RUSTFLAGS gotcha, cargo-target-pool
     CARGO_TARGET_DIR discipline, seam map, P2P design gate). Whether Gemini should get a
     generated GEMINI.md projection of it is part of Part B's design, not assumed.
  3. Session discipline (non-negotiable, from the shared feedback memory):
     - Work stays in /projects/elohim; never create sibling worktrees.
     - Commit-only: you never push or merge — the integrator is the single push authority.
     - Commits are path-limited (`git commit -m … -- <paths>`); the tree carries other
       sessions' uncommitted modifications — never revert or reformat what you didn't
       write.
     - The .epr-meta compose-gate evaluates every author's commits via the git gate
       (.husky pre-commit → epr-meta-git-gate.py); a refuse blocks, an ask needs
       EPR_META_ACK=1 acknowledged deliberately, advisories go to stderr.
  4. Identity (the actor plane — spec:
     genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md):
     register WHO YOU ARE at session start, in flight:
       epr actor claim --as agent:<role>@gemini-3.1-pro --session <your-session-id> \
         --root /projects/elohim
     Role is your function this session (e.g. `general`, `storyteller`); the model half
     admits dots, so `gemini-3.1-pro` is the canonical slug. Claims never block, stack
     per session (latest wins, history append-only), and `epr govern --session` +
     `epr flow note --session` will attribute your acts to the claim with the git-signing
     human attached as steward. Record mid-run corrections with
     `epr flow note --on <target> --kind correction --reason '…'`.
  5. Trailers: end commit messages with your roster line, e.g.
     `Co-Authored-By: Gemini 3.1 Pro <noreply@google.com>`. The valueflow's
     normalize_co_author maps unknown domains to the lowercased email — honest, no
     misattribution; extending the domain vocabulary map is a deliberate follow-up
     decision, not yours to make in passing.

  ── Part B: the bounded build — gemini projection target ──
  Authoritative surfaces: .epr-meta/elohim/packages/ (packages are the source of truth
  for planted/native packages), elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs
  (the projector: import/project/verify, fidelity gates, --only scoping), the
  elohim-package-authoring skill (.claude/skills/elohim-package-authoring/SKILL.md).
  1. DISCOVER first: what is the Gemini CLI's native persona/agent surface today
     (hierarchical GEMINI.md context files, .gemini/ config, custom commands,
     extensions)? Design the projection shape from what actually exists — do not force
     the .claude/agents frontmatter shape onto a runtime that has a different one.
  2. Extend package-projections.mjs with a `projections/gemini` target: generated,
     content-addressed, provenance-recorded, same verify/fidelity discipline as claude
     and codex (a stale projection must FAIL verify, and --only must scope writes).
  3. First two projections: blind-reader and storyteller (both package-mastered,
     both pinned claude-opus-4-6 for the Claude runtime — the gemini projection carries
     gemini-3.1-pro as its model). Their packages carry an Attribution section naming
     the actor-plane claim discipline; project it faithfully.
  4. Blind-reader's isolation contract is the hard constraint: whatever shape the gemini
     projection takes, the persona must read ONLY the document under review. If the
     Gemini runtime cannot enforce that isolation, say so in the deliverable rather than
     shipping a leaky projection.
  5. Verification: pnpm elohim-agent:packages:verify green (all checks); a live smoke —
     run one blind-reader cold read via Gemini on any sealed spec and return its verdict
     shape; register your own actor claim for the run and note whom you superseded.
  Deliverable: the projector extension + two projections + a short runbook note in the
  research/ or docs/ tree describing the gemini runtime surface you found, committed
  path-limited, no push.
---

Well-specified and disjoint: claimable by any runtime, written for Gemini. Verify every
claim above against disk before acting on it (memories and backlog items age); the actor
plane spec and the elohim-package-authoring skill are the two load-bearing reads.
