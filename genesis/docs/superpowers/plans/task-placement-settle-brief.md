---
id: placement-settle-brief
title: Settle the placement-contract spec from the verified claim corrections and the coherence read
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#1
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:d2a76864bcbbebd1 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:2ae965e98bf63d04 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
---

Item P0 of the plan. You are the only writer of the spec while this seat is open.

Inputs (read them; do not re-derive):

- `/tmp/claude-0/-projects-elohim/7e52adff-23e3-4a4c-86f3-735370908262/scratchpad/settle-corrections.md`:
  63 factual claims a reviewer found wrong or partly wrong, each tried by two skeptics. `UPHELD`
  means at least one skeptic agreed the spec is wrong; `REJECTED` means the spec stands.
- `/tmp/claude-0/-projects-elohim/7e52adff-23e3-4a4c-86f3-735370908262/scratchpad/settle-coherence.md`:
  17 findings from a cold read of the whole spec.

Do:

1. For each `UPHELD` correction, re-check the evidence it cites in the code, then apply the BEST wording
   (a skeptic's) or, where empty, the reviewer's, in the spec's own register: plain declarative sentences,
   no hedging, paths quoted where the spec already quotes them. Where a skeptic says the sentence can
   stand with a caveat, add only the caveat. Skip `REJECTED` items.
2. Apply the coherence findings whose class is correctness or interpretability. Leave preference
   findings unless they cost one word.
3. Do not change design rulings (§1 rulings, §7, §10, §12 rules) except where a correction proves a
   fact under them wrong; then fix the fact and name the ruling it touched in your report.
4. Keep section numbers stable. Keep every `cites:` envelope tool-generated: if you add a cite, add it
   as a plain path and run `epr flow cites seal` on the spec.
5. Run `epr flow cites verify` on the spec and quote its line.

Report: write `genesis/docs/superpowers/plans/task-placement-settle-report.md` with frontmatter `gap:`,
`actor:`, `status:` (DONE, DONE_WITH_CONCERNS, or BLOCKED) and a table of every item, marked applied,
skipped, or changed-differently, each with one line of why. No staging, commits or pushes.
