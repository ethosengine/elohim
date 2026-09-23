---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and author-time rules including the repo-wide
  binding of the source-file-LoC-ceiling policy (measure class — never blocks) and the
  governance-escalation-ladder (the agency charter — definitions live in the policy registry,
  .claude/epr-meta/policies.yaml; bound for both manifest forms), the brand-vocabulary boundary
  lint, a context-blind README review obligation, three developer-valueflow authoring signals, and
  two placement routes for the prose classes that kept being born at the repo root (handoffs,
  vulnerability-cluster sheets). The body records the top-level allow-list, which the gate cannot
  express. It anchors the cascade and hosts the
  ignores and basename-wide rules that cannot decentralize, plus the subtree/orchestrator exact-path
  entries kept inline for the first cut.
policy-recipe: .claude/epr-meta
policy-recipes:
  default:
    name: default
    dir: .claude/epr-meta
    measures: .claude/epr-meta/measures.yaml
    policies: .claude/epr-meta/policies.yaml
    why: >
      The declared DEFAULT recipe, named as an intentional act rather than left implicit
      (operator plurality ruling, 2026-09-10, recorded on
      genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md). This policy
      set is ONE lens over this substrate, never the lens: a second recipe may be added here
      and evaluated simultaneously over the same records, with every outcome labelled by its
      recipe. Rows carry a PRECEDENT_BINDING level and lineage so a set can fork, propagate
      and be ratified upward without a rewrite; ratification recommends, never overwrites
      local tightening. `policy-recipe:` is the key the native report reads
      (elohim/eprfs/epr-cli/src/flow/report.rs declared_default -> Recipe::at_dir): its value is
      the DIRECTORY holding the recipe's `measures.yaml` + `policies.yaml`. The entry below is
      that same recipe under its declared name `default` — joined to the scalar by its `dir:`,
      and carrying `name:` so a label reader can take the name from the key or the field. The
      declared name and the reported label agree at `default`; a second recipe is added here.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
  - id: governance-escalation-ladder
    policy: governance-escalation-ladder@1
  - id: governance-escalation-ladder-dir-form
    policy: governance-escalation-ladder@1
    when: { write: "manifest.md" }
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
    when: { write: "task-*-brief.md" }
    route-to: { dest: "epr flow claim" }
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
    when: { write: "task-*-report.md" }
    route-to: { dest: "epr flow fulfill" }
    why: >
      A completed task report discharges promised work. Record that act with
      `epr flow fulfill --on <gap-id> --report <this file> --status <DONE|DONE_WITH_CONCERNS>` so
      completion changes the commitment stock instead of remaining an unjoined report artifact.
    retire-when: >
      when the authoring surface records every discharging task report as a fulfillment by
      construction before the report can be written
  - id: rulings-are-notes
    class: inject
    when: { write: "progress.md" }
    route-to: { dest: "epr flow note --kind ruling" }
    why: >
      A ruling belongs in the valueflow record: use
      `epr flow note --on <gap-id|plan> --kind ruling --reason '...'`. The progress file is a
      projection and never the record, so do not leave a binding decision only as prose here.
    retire-when: >
      when progress projections are rendered entirely from ruling notes and cannot be authored as a
      competing decision record
  - id: handoff-routes-to-sprints
    class: dispatch
    when: { write: "handoff*.md", new: true }
    route-to: { dest: genesis/docs/superpowers/sprints/ }
    parameters:
      dispatch-agent: librarian
      dispatch-prompt: >
        Classify the proposed new handoff document. Report exactly one of ROUTE (the sprint result
        under genesis/docs/superpowers/sprints/ that should carry it), MERGE (the existing sprint
        result, backlog entry or memory file that already holds its content) or DROP (session
        narration that git history already keeps), with the shortest supporting reason. Do not
        create, move or rewrite files; report to the editing agent and let development continue.
    why: >
      Handoff documents are not an artifact class here: the handoff pattern was retired in
      df250665f (2026-06-11) and the decompose discipline replaced it on 2026-06-23
      (.claude/handoffs/archive/README.md). A concluded session's durable carrier is its sprint
      result in genesis/docs/superpowers/sprints/. Decompose the rest: open work to
      genesis/data/timeline/backlog/, a lesson to a memory file or to
      genesis/docs/content/elohim-protocol/history/, and the narration to git history.
    retire-when: >
      when no handoff*.md has been born outside genesis/docs/superpowers/sprints/ for two
      consecutive quarters, so the convention holds without a prompt
  - id: vulnerability-cluster-routes-to-backlog
    class: dispatch
    when: { write: "vulnerability_cluster*.md", new: true }
    route-to: { dest: genesis/data/timeline/backlog/ }
    parameters:
      dispatch-agent: librarian
      dispatch-prompt: >
        Check whether this security concern is already tracked by a security-*.md entry in
        genesis/data/timeline/backlog/ (its CLUSTERS.md groups them). Report exactly one of ROUTE
        (the security-<slug>.md name it should be born under), MERGE (the existing entry to
        extend) or DROP, with the shortest supporting reason. Do not create, move or rewrite
        files; report to the editing agent and let development continue.
    why: >
      A vulnerability cluster is a security concern, and security concerns live as
      security-<slug>.md entries in genesis/data/timeline/backlog/, where the deprecation-stasis
      loop reconciles them against .claude/data/deprecations.jsonl and drains them. Thirteen
      VULNERABILITY_CLUSTER_*.md sheets born at the repo root on 2026-07-30 had to be swept out
      by hand. Name the entry security-<slug>.md and write it in the backlog.
    retire-when: >
      when security concerns reach the backlog only through the deprecation sentinel's own
      writer, so no hand-authored cluster sheet has a reason to exist
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
      Each finding carries a class (correctness | interpretability | preference); the loop closes when a
      fresh reader returns no correctness or interpretability finding, and preference findings never
      reopen it. Record the loop's cost where the work already reports — the commit message or the
      owning habit delta — as rounds, per-class counts per round, and findings resolved, so review
      efficiency is measured rather than felt. Every round still uses a new reader.
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
`.claude/scripts/ci-ignore-projector.py`) and the author-time rules listed above. **`.ci-ignore` is GENERATED from
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

The policy's own scope is `write: ".epr-meta"`, which matches only the flat manifest file. The
directory form, `<dir>/.epr-meta/manifest.md`, has a different basename, so the charter never saw
it. `governance-escalation-ladder-dir-form` binds the same `@1` policy a second time with a
`when: { write: "manifest.md" }` override. That closes the gap without a new registry row. The
validator checks the path first (`is_manifest_path`), so a `manifest.md` that is not an `.epr-meta`
manifest draws nothing. A subtree under another `root: true` manifest inherits neither binding;
such a subtree must bind the ladder in its own manifest.

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

## Placement at the repo root (dispatch class, plus an allow-list the gate cannot express)

Two prose classes kept being born at the top level and swept out by hand: session handoffs
(`HANDOFF-*.md`) and vulnerability-cluster sheets (`VULNERABILITY_CLUSTER_*.md`). Each has a named
route at birth. `handoff-routes-to-sprints` sends a handoff to `genesis/docs/superpowers/sprints/`,
and `.claude/handoffs/.epr-meta` widens that same rule id to every markdown birth there.
`vulnerability-cluster-routes-to-backlog` sends a cluster sheet to a `security-*.md` backlog entry.
Both dispatch a report-only librarian review and never block.

**Top-level allow-list — authoritative, not gate-enforced in v1.** The repository root holds only:

- directories: `app/`, `bridges/`, `che-devworkspaces/`, `crates/`, `doorway/`, `elohim/`,
  `genesis/`, `patches/`, `scripts/`, `sophia/`, `steward/`, `vendor/`, plus `docs/`, which is a
  tombstone holding only its own routing manifest;
- workspace-root files: `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `justfile`,
  `deny.toml`, `VERSION`, `devfile.yaml`, `Jenkinsfile`, `sophia.Jenkinsfile`, `README.md`,
  `CLAUDE.md`, `AGENTS.md`, `.gitmodules`, `.gitignore`, `.ci-ignore`, `.npmrc`;
- tool dotfiles and dot-directories (`.claude/`, `.codex/`, `.agents/`, `.epr-meta/`, `.eprfs/`,
  `.github/`, `.husky/`, `.cargo/`, `.vscode/`, `.mcp.json`).

`storage-iroh/`, `iroh-relay/`, `relay-addr-beacon/`, `docs/`, `rakia/` and `tools/` were relocated
or retired on 2026-09-23. Anything else belongs inside one of the homes above. The gate cannot
enforce this list. `when.write` matches the basename only, so it cannot tell a depth-1 file from
any other. A root rule also cascades to every descendant, and `no-new-subdirs` and `require-sibling`
fire at any depth, so either one here would toll every new module directory in the tree. Review
holds the list. A tool that writes machine-local state at the root (a dependency store, a build
pool, a crash log) needs a `.gitignore` line, not a rule: the resolver sees only Write and Edit.

## readme-blind-reader-review — newcomer legibility (inject class)

Every `README*.md` authoring pass routes to the general `blind-reader` using the `readme` profile.
The post-write adapter makes the obligation visible after successful edits. It is a semantic review,
not a formatting lint: a context-isolated reader must be able to orient, understand purpose and
boundaries, satisfy prerequisites, reach a first success, and know what to do next. One rich review
loop runs after the completed authoring pass—not one dispatch per edit.
