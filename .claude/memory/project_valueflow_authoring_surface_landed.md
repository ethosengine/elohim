---
name: project_valueflow_authoring_surface_landed
title: Valueflow authoring surface landed
description: Valueflow authoring surface LANDED + CLOSED OUT 2026-09-05 — epr flow claim / fulfill --on / context / ledger, ruling|verdict notes, bounded WIP fence stock, flips-need-rulings check, observer hook minting on write, three valueflow-* seat skills; traps (doc-plane cites heal via cite-gen --refresh not reseal; cargo lease etiquette between sessions; blind-reader loop cap) — reach for it before any SDD/epic dispatch
metadata:
  type: project
---

Landed on dev 2026-09-05 (spec genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md,
plan …/plans/2026-09-05-valueflow-authoring-surface-plan.md, closed through its own verbs):

- **Verbs** (installed /opt/rust/cargo/bin/epr, crate elohim/eprfs/epr-cli): `epr flow claim --on <gap-id|intent-cid|path> --as agent:<role>@<model> [--brief] [--serves <habit-id>] [--supersede]`;
  `epr flow fulfill --on <gap-id|commitment-cid> --report <path> --status DONE|DONE_WITH_CONCERNS [--commit sha]...` (BLOCKED/HOLD/NEEDS_CONTEXT refused → `note --kind observation`);
  `epr flow context <path|cid> [--notes N]` = identity · intents · commitments · notes (rolled up from commitments with `via <gap-id>`) · seals · habit · gate (ties printed as `ambiguous`, never guessed) · governance;
  note kinds `ruling` (control) and `verdict --verdict approved|changes-requested` (audit).
- **Skills** (native packages, master=package): `valueflow-authoring` (the 7-verb method + dispatch prompt shape "Invoke skill X. Brief: <path>. Commitment: <gap-id>. Rulings in force: … Base: <sha>."), `valueflow-implementer`, `valueflow-reviewer` (each ends in ONE verb). Root `.epr-meta` inject rules fire on basenames `task-*-brief.md`, `task-*-report.md`, `progress.md`.
- **Actor convention:** full model ids (`agent:implementer@claude-opus-5`, `agent:reviewer@claude-opus-5`, `agent:orchestrator@claude-fable-5-1`).
- **Process:** decompose.py <plan> → `epr flow project` → claim → dispatch seats → fulfil → rule → habit DELTA + habits-project.py. Habits = the fabric's STANDARD layer (REA scope / VSM S5), see spec §3.

**Traps learned:**
- A `cites:` edge is DOC-PLANE (fingerprint in the envelope): `epr flow context` can show it stale while `reseal --all-stale` finds nothing — heal with `cite-gen.py --refresh <doc>` then `--verify`. Quoted cite list items read as DANGLING before 9665aa4c2; now fixed in the parser.
- Cargo lease etiquette: `berth who cargo`; never release another session's lease (permission layer refuses it, correctly) — ask the holder session by cross-session message; a "leases:" line clipped by `head` reads as free when it is not.
- Unquoted `title:` with `: ` inside = invalid YAML → native evaluator refuses, Python permits (governance-plane-single-evaluator finding).
- Blind-reader loops drift (READY → REVISE with disjoint majors); cap at ~3 rounds, take the structural finding, defer the rest by note ([[feedback_reviewer_issue_admissibility]]).
- **Close-out round (same day, operator: "don't leave things open") — ALL BUILT:** `epr flow ledger <atom>` (oldest-first markdown/JSON of claims·fulfils·notes incl. commitment roll-up — progress.md/§11.4 are projections of it); the WIP fence as a bounded commitment (`project` mints it on the covenant atom; ceiling breaches at >= limit so "max 2 active" = limit 3.0; `stocks --stock active-habits --check` — projected level from habits.yaml; NEWEST fence record decides because the sidecar is append-only); `habits-project.py --check` refuses a status flip with no `run:ruling` note on the atom (FLIP-WITHOUT-RULING); PostToolUse `.claude/hooks/valueflow-observer.py` mints claim/fulfil/observation from brief/report frontmatter (`gap`, `actor`, `status`, `commits`) — proven live, zero typed verbs; `note --on <gap-id>` resolves; psephos rename = genesis/data/timeline/backlog/2026-09-05-psephos-naming-drift-backlog.md.
- Two agents committing in ONE worktree race on the index → commit by pathspec (`git commit -m … -- <paths>`), see [[feedback_agent_fleet_and_harness]].
See [[feedback_valueflow_authorship_is_the_process]], [[project_epr_flow_valueflow_projection]], [[project_inequality_curve_as_bounded_standard]].
