---
name: valueflow-authoring
description: "Orchestrate developer work as an explicit REA valueflow: intend, claim, produce, verify, fulfil, rule, and ratchet, with one command per verb and durable actor attribution."
metadata:
  runtime: codex
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/valueflow-authoring.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/valueflow-authoring"
---

# Valueflow Authoring

Treat the REA authorship loop as the development process itself: intend, claim, produce, verify, fulfil, rule, ratchet. The protocol verbs are the only designed friction; do not surround them with duplicate ledgers or prose that has to be reconciled later. Keep the standing WIP fence: at most two habits may be active at once.

There is no parameter ABI. The brief path, commitment, rulings in force, and base commit are the parameters. Pass them in every dispatch using this exact shape:

`Invoke skill X. Brief: <path>. Commitment: <gap-id>. Rulings in force: ... Base: <sha>.`

## Method

Each verb is one command or one dispatch. Do not combine verbs into an opaque script.

1. **Intend.** Decompose the plan into gap items, then run `epr flow project` once so those intents enter the valueflow.
2. **Claim.** Run `epr flow claim --on <gap-id> --as agent:implementer@<model> --brief <path>` once.
3. **Produce.** Dispatch one implementer with the `valueflow-implementer` skill and the exact dispatch prompt shape above.
4. **Verify.** Dispatch one reviewer with the `valueflow-reviewer` skill, the same brief and commitment, the rulings in force, and the base SHA.
5. **Fulfil.** For a discharging status, run `epr flow fulfill --on <gap-id> --report <path> --status <DONE|DONE_WITH_CONCERNS>` once.
6. **Rule.** Record a binding decision once with `epr flow note --on <gap-id|plan> --kind ruling --reason '...'`.
7. **Ratchet.** Append one evidence delta line to the habit atom, then re-project the habit register once.

## Brief and report frontmatter

Every `task-*-brief.md` carries `gap: <gap-id>` and `actor: agent:<role>@<model>` in YAML frontmatter. Every `task-*-report.md` carries those same fields plus `status: DONE|DONE_WITH_CONCERNS|NEEDS_CONTEXT|BLOCKED|HOLD` and optional `commits: [sha, ...]`. The PostToolUse observer mints the claim or fulfilment when the file is written (and an observation note for a non-discharging status), so a seat's terminal verb becomes a no-op fallback when the observer already recorded it.

## Do not

- Do not restate constants from the brief, plan, rulings, or repository governance in a dispatch prompt. Pass their addresses and let the seat read them.
- Do not write rulings only as prose in a progress file or report. The ruling note is the record; prose is a projection.
- Do not exceed the WIP fence to make dispatch easier.
- Do not invent a parameter ABI around the prompt shape.
