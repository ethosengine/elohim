---
epr-meta-version: 1
id: eprfs-measure-plane-reuse
purpose: >
  The eprfs workspace, where the development system's measure plane is evaluated natively
  (epr-cli's flow report, sense and stocks). One edit-time signal: before a level is judged by a
  bespoke comparison, or a finding is kept in a private ledger, name the native primitive that
  already holds the contract. Makes no coverage claim (no `covers:`); finer manifests still cascade.
rules:
  - id: limit-model-reuse-rs
    class: inject
    when:
      write: "*.rs"
      contains-any: ["struct Bound", "enum Compare", ">= hard", "> hard", ">= soft", "> soft",
                     "bound.limit", "band_edge", ".jsonl\""]
    dedupe-of: "elohim/epr-rea/src/model.rs (Bound: limit, threshold_pct, Sense; breached_by / approached_by) + elohim/eprfs/epr-cli/src/flow/{report,sense}.rs (the one judge and the one sensing writer)"
    why: >
      REUSE the one limit model before judging a level by hand. A level against a limit is judged
      by elohim_epr_rea::Bound (`breached_by` / `approached_by`, which honours Sense — a floor
      never emits pain) or by the native report over a declared lens (`epr flow report --bound`,
      `--breaches`). A finding is a fold written through `epr flow sense` (a `probe:` on the
      measure row), never a private .jsonl ledger with its own cursor and closure rule. Precedent:
      the 2026-10-22 substrate-bypass census — flow/measures.rs's own Bound/Compare,
      stocks.rs:941's ceiling-only re-judge, gate-cycle.mjs re-judging the lens the report judges
      (spec genesis/docs/superpowers/specs/2026-10-22-sense-organ-design.md). If this code is a
      deliberate variation, say so where the detector can read it: `epr flow sense rule --measure
      limit-models@1 --subject <file> --class intentional --why <the contract difference>`.
      (Advisory `inject` nudge — never blocks, names no validator: whether this is a
      re-implementation is a contract judgement, which the ruling records.)
    retire-when: "when limit-models-open@1 has read 0 for a quarter — the habit it serves, sensing-rides-the-measure-plane, is then a fact about the build"
validators: []
---

`elohim/eprfs/` governance package. Habits declared here:
`materialize-what-is-read`, `sensing-rides-the-measure-plane`.
