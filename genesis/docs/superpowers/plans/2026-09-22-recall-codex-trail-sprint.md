---
title: Recall — Codex's trail — the entry reaches, resumes a concern, and names prose that lags its value
id: recall-codex-trail-sprint
status: in-flight
class: devflow
serves: recall-reaches-authority
date: 2026-09-22
cites:
  - genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md
  - "bounded-recall-mastery-sprint | Bounded recall mastery | sha256:d4c6f66d1685d124 | path: genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md"
  - "governed-discovery-stations-0-3-plan | Governed discovery | sha256:a0be77a4bc3f6dc7 | path: genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md"
---

# Recall — Codex's trail

On 2026-09-13 Codex resumed an interrupted doorway/P2P sprint and left six observations about
the memory system as item 25 of the agentic context-tooling consolidation queue (commits
40ce136dc, 0e61dd311, a4f4d2c40). The habit they serve, `recall-reaches-authority`, has been red
since 2026-09-12. Its own frontier (station 4) names the same fault as Codex's first
observation: discovery fails to reach the authority it is asked for.

## Baseline (2026-09-22, installed `epr`, deterministic `recall sample`)

The bank has six questions, and 1 reaches (`q-corrections`). Three causes, found by reading
`recall/discovery.rs`:

| Question | Target | Why it misses |
|---|---|---|
| q-hook-binary | `.claude/hooks/_observation.py` | `first_screen` discovers `*.md` only |
| q-body-scan | `recall-contract.json` | `*.md` only |
| q-journey-folds | `.claude/epr-meta/measures.yaml` | `*.md` only |
| q-top-red | `CLAUDE.md` at scope `.` | `focus_area` is `None` at root, so the open renders the whole-scope ceremony door; `question_terms` also drops `top`/`red` (<4 chars) |
| q-remine | `memory-ceremony/SKILL.md` | ranks `converge/SKILL.md` first |

## Disposition of each Codex observation

| # | Observation | Disposition |
|---|---|---|
| 1 | Discovery spends its budget before finding the work | **S1**: declared multi-type globs, a short-term vocabulary, light stemming, a metadata-first route at root scope, and a narrowed continuation command when the budget runs out |
| 2 | Operational prose lags the value it describes (`pool-policy.json` `max_concurrent_heavy`) | **S3**: `read`/`source` of JSON/YAML names every place where prose says `key=N` but the document's value for `key` is different, together with the dated sibling that carries the provenance |
| 3 | Joining plan, worktree, unpushed commits and evidence by hand | **S2**: `open --purpose resume --habit <id>` is a read-only view of the habit's last delta and checks, its plans, and the local commits and branches newer than the delta. Newer commits are marked *implemented-but-unverified*. The view keeps no ledger. |
| 4 | Review loops need a visible cost and a closing rule | **S4**: blind-reader findings carry `class: correctness · interpretability · preference`, and the revision loop records rounds and per-class counts. A round is admissible only through its correctness and interpretability findings, and the gate stays: every round still gets a fresh reader. |
| 5 | A remembered green needs an artifact chain back to its source | **Partly by S2.** Commits after the delta that touch the concern's paths show as unverified. Binding build receipts from source to pack to loaded artifact is held. `just mesh start` already refuses a stale wasm→dna→happ, and the sweettest path has no such check. The residue goes back to the queue. |
| 6 | Delivered text is not a running follow-up | **Held** as harness-owned. Both harnesses already have the distinction (`SendMessage`, and Codex's `followup_task`). Recall sessions are private, so showing their continuations in another session's view would project private records. |

## Stations

- **S1 — the entry reaches** (`recall/discovery.rs`, `recall-contract.json` `discovery`): the
  contract declares `first_screen_globs` and `short_terms`. Stems match by shared prefix. At
  root scope, candidates come first from the authority set (root gospel `CLAUDE.md`, plus the
  atom, `checks:` paths and `refs:` paths of each matched habit), before any body scan. A budget
  cut emits `open --scope <densest subdir> --need <same>`. Test: new unit/integration tests
  for each cause, plus deterministic `recall sample` of all six bank questions.
- **S2 — resume a concern** (`recall/journey.rs` · `discovery.rs`): `--purpose resume --habit <id>`
  (refused when the habit is unknown). The view shows the habit's status, active flag, checks,
  last delta and the date parsed from it, and the plan paths from `refs:`. It lists the commits
  since that date that name the habit or touch its paths (`git log`, bounded), the commits on
  this branch that are not on its upstream, and the local branches whose recent commits name the
  habit. When commits postdate the delta it says so explicitly. Jenkins stays out of the view
  (named as an omission). Nothing is written.
- **S3 — prose that lags its value** (`recall` `read`/`source`): a `disagreements` block for JSON
  and YAML sources. It has a fixture test, and a live check against `genesis/agentic/pool-policy.json`.
- **S4 — review cost** (`blind-reader` agent package + `story-harvest` loop text): a finding
  class and a round record, re-projected through the package tooling.

## Done

- `just gate memory-ceremony` EXIT=0 on the merged tree
- the deterministic bank sample reaches ≥5 of 6
- a context-reset fresh reader reaches the interrupted-work question through the entry alone,
  with bytes metered
- judged by a seat of a different tier
- the capstone memory ceremony runs on the new entry, and one delta goes in the habit atom
- item 25 gets its dispositions

The habit's status flips only if the window bound reads under 0.2. This sprint does not expect
that: the window needs real weekly journeys.
