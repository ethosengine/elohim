---
id: placement-stories-t7-report
title: "T7 report: lifecycle and proof stories (placement contract section 16)"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#21
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T7 report: lifecycle and proof stories

The status is DONE_WITH_CONCERNS. All four files parse, but none of them closed. Each one reached the six-round limit of the blind-reader loop, and the last round still left findings, which are listed under "Deferred findings" below.

Counts are written as C/I/P, meaning correctness, interpretability and preference findings in that round.

| File | New or extended | Scenarios added | Concern tags | Rounds | Per-class counts per round (C/I/P) | Parse |
|---|---|---|---|---|---|---|
| genesis/a2o/features/resilience/member-leaves-replaced-first.feature | new | 3 | `blob-durability` (reused) | 6 (R1 REVISE, R2-R6 READY) | R1 1/5/2, R2 1/3/4, R3 0/3/5, R4 0/2/2, R5 0/4/2, R6 1/3/4 | parsed (Background plus 3 scenarios) |
| genesis/a2o/features/qahal/leaving-a-collective.feature | new | 4 | `leave-is-clean` (new) | 6 (R1 REVISE, R2-R3 READY, R4 REVISE, R5-R6 READY) | R1 0/6/2, R2 2/4/1, R3 0/3/5, R4 0/4/3, R5 0/5/2, R6 0/2/2 | parsed (Background plus 4 scenarios) |
| genesis/a2o/features/resilience/observable-distribution.feature | extended (appended only) | 2 | `blob-durability` (reused) | 6 (all REVISE) | R1 0/9/4, R2 1/7/3, R3 2/6/4, R4 1/9/2, R5 4/7/1, R6 0/11/3 | parsed (27 scenarios, 2 of them new) |
| genesis/a2o/features/qahal/hostile-joiner.feature | new | 4 | `peer-admission` (new) | 6 (R1 READY, R2 REVISE, R3-R5 READY, R6 REVISE) | R1 1/5/2, R2 1/2/4, R3 0/5/4, R4 2/4/1, R5 0/2/1, R6 1/4/2 | parsed (Background plus 4 scenarios) |

I ran `cucumber-js --dry-run`, but the a2o config loads the whole suite whatever path you pass. It reported 1627 scenarios and no parse error, so that total is not a per-file count. Each file was also parsed on its own with `@cucumber/gherkin`.

## Deferred findings (the operator decides whether to defer them)

**member-leaves-replaced-first.feature (round 6)**
1. Interpretability: the story says when a holder starts being honoured but never says when it stops. The checks "at every moment, at least two holders" depend on that.
2. Interpretability: the reason the story is held shows only in the header comment. The tags do not carry it. The description does mention slice 5.
3. Correctness: scenario 2 has two When steps, one per phase of its timeline. The description explains this, and the final retrospective step covers both phases.
4. Interpretability: the sibling feature references do not say what state those features are in.

**leaving-a-collective.feature (round 6)**
1. Interpretability: "slice 5", "missing nodes" and "the floor controller" are not glossed in the paragraph that explains why the story is held.
2. Interpretability: the bare section numbers (§17, §15, §9) point to the design spec without saying what those sections are.

**observable-distribution.feature (round 6).** Almost all of these are about text that was already in the file. The brief barred rewriting existing text, so the owner of that text needs to decide.
1. The HOUSEHOLD/COLLECTIVE/FLOOR/SHORTFALL glossary is only in a comment. It should move into the Feature description.
2. "commons" reach and "provide commitment" are never defined.
3. The `@act:i`, `@act:ii` and `@act:iii` tags are never defined.
4. The "As an operator" line is not tied to the "I" and Matthew in the steps.
5. "EPR" is never expanded.
6. `@missing:household-attribution-on-delivery-peers` is never explained.
7. Line 130 says "doorway-app" where every other step says "doorway".
8. Adam and Pete appear without being introduced.
9. The tier system ("standard-tier") is never explained.
10. The `@requires:` tags (shem, household-nodes, owned-substrate) are never defined.
11. The references to "Omnibar spec §11" and the HyperCard idiom are not glossed.

**hostile-joiner.feature (round 6)**
1. Interpretability: the reason the story is held (circle admission) and the link to the spec are only in comments. They should move into the description.
2. Interpretability: the story never says what "the household thread" is.
3. Interpretability: the story does not state outright that fetching a file by its content address is something a peer normally supports.
4. Correctness: scenario 4's title says "the same requests", but the scenario covers both the peer paths and the doorway path.
5. Interpretability: in "his signature", "his" could be read as the peer. It should say "Matthew's".

## Author's notes

- The loop ran six rounds for each file. I staged and committed nothing. Every new scenario is `@wip` and has no step definitions yet. Every one carries `@act:i` and `@requires:household-nodes`. Scenarios that remove a member, take a peer offline, delete bytes or add a stranger peer also carry `@requires:owned-substrate`. None is tagged shem.
- `blob-durability` is reused, and its habit already checks it. `leave-is-clean` and `peer-admission` are new. No habit checks either of them yet, and each file's header says so.
- Why each story is held:
  - member-leaves-replaced-first: until slice 5 (§15 "Lapsed lease, re-placed first").
  - leaving-a-collective: until slice 5, plus two §15 nodes and the §9 "Leave and withdrawal" read-back.
  - hostile-joiner: until circle admission, which is outside the numbered slices in §17.
  - The observable-distribution additions: until the §15 "Claimed, then held" holding probe. I mapped that probe to slice 5, which is my own reading.
- Spec readings that need a ruling:
  1. In member-leaves-replaced-first, the leaver's peer keeps the household copy until another holder is honoured (§7.2). That puts re-placing the copy ahead of the leaver's immediate exit.
  2. leaving-a-collective assumes that leaving does not mean being forgotten (the membership record and the withdrawal both stay). It also assumes commitments are per space.
  3. hostile-joiner defines admission as a record signed by a member the household allows to admit, checked by every peer that answers and every peer that validates.
  4. The holding check is not designed. The story defines it only as a question about the copy that only someone holding its exact bytes can answer.
- Rule floor: closing the observable-distribution findings would mean rewriting text that was already there. The brief does not allow that, so this file cannot close without the owner of that text or a variance.

## Correctness pass (2026-10-20)

| File | Fix rounds | Confirming reader | Correctness findings open | Parse |
|------|-----------|-------------------|---------------------------|-------|
| `genesis/a2o/features/qahal/hostile-joiner.feature` | 2 (round 1: READY, correctness 2, interpretability 3) | Round 2: READY. Correctness 0, interpretability 3 | None | PARSED (dry run, no Gherkin parse error) |
| `genesis/a2o/features/resilience/member-leaves-replaced-first.feature` | 1 | Round 1: READY. Correctness 0, interpretability 2 | None | PARSED (dry run EXIT=0, 1628 scenarios loaded, no parse error) |

No correctness finding is left open for the operator. The interpretability findings are still open.
