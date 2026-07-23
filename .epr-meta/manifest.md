---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and two author-time rules: the repo-wide
  binding of the source-file-LoC-ceiling policy (observation tier — never blocks) and the
  governance-escalation-ladder (the agency charter — definitions live in the policy registry,
  .claude/epr-meta/policies.yaml). It anchors the cascade and hosts the ignores that cannot
  decentralize (basename-anywhere) plus the subtree/orchestrator exact-path entries kept inline
  for the first cut.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
  - id: governance-escalation-ladder
    policy: governance-escalation-ladder@1
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
