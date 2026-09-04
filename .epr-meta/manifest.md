---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and author-time rules including the repo-wide
  binding of the source-file-LoC-ceiling policy (observation tier — never blocks) and the
  governance-escalation-ladder (the agency charter — definitions live in the policy registry,
  .claude/epr-meta/policies.yaml), the brand-vocabulary boundary lint, a context-blind README review
  obligation, and three developer-valueflow authoring signals. It anchors the cascade and hosts the
  ignores and basename-wide rules that cannot decentralize, plus the subtree/orchestrator exact-path
  entries kept inline for the first cut.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
  - id: governance-escalation-ladder
    policy: governance-escalation-ladder@1
  - id: brand-vocabulary-boundary
    policy: brand-vocabulary-boundary@1
  - id: habit-declaration-at-birth
    policy: habit-declaration-at-birth@1
  - id: dev-lifecycle-context-sync-npm
    class: inject
    when: { write: "package.json" }
    dedupe-of: genesis/data/timeline/backlog/2026-08-16-dev-cli-hygiene-script-census.md
    why: >
      You are editing a dev command surface (package.json scripts). The local-dev lifecycle
      context is COUPLED to it and drifts when it changes silently — keep them in sync in the
      same pass: the hc-dev-orchestrator skill (.claude/skills/hc-dev-orchestrator/SKILL.md,
      local stack lifecycle), the root CLAUDE.md Build & Test Commands section, the root
      justfile recipes, and the script census/consolidation ledger this rule points at. If you
      added a script: does one of the eight root verbs already cover it (the census ledger
      tracks the measured baseline and burn-down)? If you renamed/removed one: grep docs+CI
      for the old name. Out-of-date
      instructions are cognitive load — the census doc records what is verified-alive.
  - id: dev-lifecycle-context-sync-just
    class: inject
    when: { write: "justfile" }
    dedupe-of: genesis/data/timeline/backlog/2026-08-16-dev-cli-hygiene-script-census.md
    why: >
      justfile recipes are the consolidation target for the dev CLI (census doc). When a recipe
      changes, sync the coupled context: root CLAUDE.md Build & Test Commands, the
      hc-dev-orchestrator skill, and the census ledger. Native-cargo recipes MUST set
      CARGO_TARGET_DIR at the cargo-pool slot (the disk-guard denies plain native cargo) —
      the pre-2026-08 root justfile drifted exactly here.
  - id: brief-is-a-claim
    class: inject
    when: { write: ".superpowers/sdd/**/task-*-brief.md" }
    why: >
      A dispatched task brief is a claim on promised work. Record that act with
      `epr flow claim --on <gap-id> --as agent:implementer@<model> --brief <this file>` so the
      implementer's commitment is durable and attributable instead of existing only in orchestration
      prose.
    retire-when: >
      when the authoring surface records every task-brief dispatch as a claim by construction before
      the brief can be written
  - id: report-is-a-fulfilment
    class: inject
    when: { write: ".superpowers/sdd/**/task-*-report.md" }
    why: >
      A completed task report discharges promised work. Record that act with
      `epr flow fulfill --on <gap-id> --report <this file> --status <DONE|DONE_WITH_CONCERNS>` so
      completion changes the commitment stock instead of remaining an unjoined report artifact.
    retire-when: >
      when the authoring surface records every discharging task report as a fulfillment by
      construction before the report can be written
  - id: rulings-are-notes
    class: inject
    when: { write: ".superpowers/sdd/**/progress.md" }
    why: >
      A ruling belongs in the valueflow record: use
      `epr flow note --on <gap-id|plan> --kind ruling --reason '...'`. The progress file is a
      projection and never the record, so do not leave a binding decision only as prose here.
    retire-when: >
      when progress projections are rendered entirely from ruling notes and cannot be authored as a
      competing decision record
  - id: readme-blind-reader-review
    class: inject
    when: { write: "README*.md" }
    route-to: { dest: blind-reader }
    parameters: { review-profile: readme }
    why: >
      After the README authoring pass, dispatch a fresh-context blind-reader with ONLY the completed
      README path and the `readme` review profile. The reader must be able to recover who the README
      serves, what the thing is for, prerequisites, its mental model, a first successful path, and the
      next useful action without inheriting the author's repository context. Revise and repeat with a
      new blind reader until READY or the operator explicitly defers named findings.
ci-trigger:
  ignore:
    - .claude/
    - .github/
    - .husky/
    - genesis/orchestrator/Jenkinsfile
    - genesis/orchestrator/build-graph.groovy
    - CLAUDE.md
    - AGENTS.md
    - GEMINI.md
    - .no-claude.md
---

# repo root — constitutional base

Carries the cross-cutting `ci-trigger:` ignore set (projected into `.ci-ignore` by
`.claude/scripts/ci-ignore-projector.py`) and one author-time rule. **`.ci-ignore` is GENERATED from
this leg — never hand-edit it.**

## rs-loc-ceiling — repo-wide source-file LoC ceiling (measure class)

Binds `source-file-loc-ceiling@1` from the policy registry: `*.rs` writes are measured against a
soft (edit-time nudge) and hard (fingerprinted architecture finding → `.claude/data/
architecture-findings.jsonl` → modularization-plan dispatch) LoC ceiling. Observation tier — it
never blocks a write. Vendored submodules (brit, rakia) are outside this cascade by construction
(their own `.git` roots terminate the ancestor walk). A region with a legitimately different
ceiling (e.g. a table-driven test harness) overrides locally by re-binding the policy with
`params:` in its own `.epr-meta` — never by editing the registry version in place.

## governance-escalation-ladder — the agency charter (ask class)

Binds `governance-escalation-ladder@1`: agents self-grant `measure`/`inject`/`dispatch` freely;
authoring or promoting an `ask`/`deny` rule requires an operator-ratified policy pin. Bound HERE
rather than a `.claude/epr-meta/.epr-meta` manifest deliberately — `when` patterns match by
BASENAME only (`_matches_when` in `_lib/epr_meta.py`) and the cascade is a strict ancestor walk,
so a manifest placed anywhere other than the repo root can only ever govern writes BENEATH
itself; it cannot reach out to `.epr-meta` files elsewhere in the tree. A repo-wide charter needs
the repo-root anchor — the minimal-diff fallback the design explicitly allows when a
`.claude`-located manifest can't scope repo-wide. See the policy row for the full why.

## brand-vocabulary-boundary — accessible code vocabulary (inject class)

Binds `brand-vocabulary-boundary@1` repo-wide. Architecture and product prose retain the domain
brands; newly introduced code, routes, schemas, configuration identifiers, persistence names, and
wire values receive an advisory to use their semantic capability name. There is no internal
compatibility exemption while the protocol is still in development: role names, zome names, package
ids, discriminators, and wire literals should be renamed too. The rule is net-new-only and never
blocks, so cleanup can proceed as each existing surface is touched without turning legacy vocabulary
into a maintenance toll.

## Developer-valueflow authoring — briefs, reports, and rulings (inject class)

The three authoring signals keep orchestration artifacts joined to the valueflow they change: a task
brief claims a commitment, a discharging task report fulfils it, and a progress-file ruling is first
recorded as a ruling note. The files remain useful projections for people, but none is the durable
record of the act it describes.

## readme-blind-reader-review — newcomer legibility (inject class)

Every `README*.md` authoring pass routes to the general `blind-reader` using the `readme` profile.
The post-write adapter makes the obligation visible after successful edits. It is a semantic review,
not a formatting lint: a context-isolated reader must be able to orient, understand purpose and
boundaries, satisfy prerequisites, reach a first success, and know what to do next. One rich review
loop runs after the completed authoring pass—not one dispatch per edit.
