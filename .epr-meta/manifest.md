---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and four author-time rules: the repo-wide
  binding of the source-file-LoC-ceiling policy (observation tier — never blocks) and the
  governance-escalation-ladder (the agency charter — definitions live in the policy registry,
  .claude/epr-meta/policies.yaml), the brand-vocabulary boundary lint, plus a context-blind README
  review obligation. It anchors the cascade and hosts the ignores and basename-wide rules that
  cannot decentralize, plus the subtree/orchestrator exact-path entries kept inline for the first cut.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
  - id: governance-escalation-ladder
    policy: governance-escalation-ladder@1
  - id: brand-vocabulary-boundary
    policy: brand-vocabulary-boundary@1
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

## readme-blind-reader-review — newcomer legibility (inject class)

Every `README*.md` authoring pass routes to the general `blind-reader` using the `readme` profile.
The post-write adapter makes the obligation visible after successful edits. It is a semantic review,
not a formatting lint: a context-isolated reader must be able to orient, understand purpose and
boundaries, satisfy prerequisites, reach a first success, and know what to do next. One rich review
loop runs after the completed authoring pass—not one dispatch per edit.
