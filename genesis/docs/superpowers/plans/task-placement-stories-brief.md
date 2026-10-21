---
id: placement-stories-brief
title: The stories that fall out of the placement contract — all of spec §16, under the full blind-reader loop
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#15
actor: agent:author@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:431aa747f916ce8b | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - genesis/a2o/.epr-meta
---

Items T1–T7 (gaps `#15`–`#21`), one seat per group. Each seat writes its group's stories from spec §16 and
the spec sections those stories prove, then closes each file through the review loop below, then writes
`task-placement-stories-t<N>-report.md` with `gap:` set to its own item.

## How a story is written here

Follow `genesis/a2o/CLAUDE.md` (Feature File Conventions, What belongs in feature files, Watch-outs). Read two
recent stories first as exemplars of register and structure:
`genesis/a2o/features/compute/ci-runner-stewarded.feature` and
`genesis/a2o/features/federation/local-first-shared-sheet.feature`.

- A story describes what a person goes through, never a layer-internal contract. Write the Feature
  description so a reader with no other context learns who it serves, why the outcome matters, and every
  term it uses.
- Every new scenario is `@wip`: it has no step definitions yet, and none are written in this lane.
- Tag the feature `@concern:<id>` as the §16 row names it. Reuse an existing concern tag wherever an existing
  habit's `checks:` already names one that fits; say in your report which tags are new.
- `@requires:household-nodes` where the §16 row says it needs the mesh; scenario-level `@requires:<cap>` for
  mixed features. The household (matthew, jessica, james) is a three-node cluster; never tag `@requires:shem`
  for household-testable stories.
- Use the household's named personas and the cast conventions of the exemplars.
- A story about something the code does not do yet is still written as the behavior the person will see; the
  feature header says plainly that it is held until its slice lands, naming the slice of spec §17.
- The stories carry the spec's rulings faithfully: placement converges rather than every peer guessing alike;
  eligible is not enlisted; a device's facts never leave it; doorways are plural participants, never a defect;
  the identity root is the network's; the elohim drafts and the participants ratify.

## Ownership

Your group owns only the files listed for it. In a file that already exists, add your scenarios and the
smallest Background change they need; never rewrite or reorder an existing scenario, tag or description.
A new subdirectory gets its `.epr-meta` written first, modeled on a sibling's, or you place the story in an
existing directory instead.

## The review loop (no variance granted: the full loop)

For each file you wrote or extended:

1. A fresh `blind-reader` receives only the file path and the `a2o-story` profile. Nothing else.
2. Revise from its correctness and interpretability findings. In an existing file, revise only your own
   additions; record findings about pre-existing text as "pre-existing, not ours".
3. Repeat with a new reader until one returns no correctness or interpretability finding. Preference
   findings never reopen the loop.
4. Before closing, check the file parses: from `/projects/elohim/genesis/a2o`, run
   `npx cucumber-js --dry-run <path relative to genesis/a2o>` in a subshell and confirm it reports the
   scenarios (undefined steps are expected) rather than a parse error.

Report per file: the number of rounds, the per-class finding counts in each round, the findings resolved, and
any findings left for the operator to defer, named.

No staging, commits or pushes.
