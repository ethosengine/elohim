---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and exactly ONE author-time rule: the
  repo-wide binding of the source-file-LoC-ceiling policy (observation tier — never blocks;
  definition lives in the policy registry, .claude/epr-meta/policies.yaml). It anchors the
  cascade and hosts the ignores that cannot decentralize (basename-anywhere) plus the
  subtree/orchestrator exact-path entries kept inline for the first cut.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
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
