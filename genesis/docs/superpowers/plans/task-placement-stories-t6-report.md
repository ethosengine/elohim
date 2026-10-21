---
id: placement-stories-t6-report
title: "T6 report: section 16 placement stories under the blind-reader loop"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#20
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T6 report: placement stories

All four files parse as Gherkin. None closed under the loop: each reached round 6, the round cap, with interpretability findings still open. Every round had 0 correctness findings except where the per-round counts show otherwise. Counts are written correctness/interpretability/preference (C/I/P).

| File | New or extended | Scenarios added | Concern tags | Rounds | C/I/P per round (verdict) | Parse |
|---|---|---|---|---|---|---|
| `genesis/a2o/features/dataplane/served-under-standing.feature` | extended | 1 Scenario Outline (@wip): an anonymous visitor sees the commons resource and nothing narrower, by page or by bytes (community, familiar, trusted, intimate) | `served-under-standing` (existing) | 6, not closed | R1 0/9/2 REVISE; R2 0/6/2 REVISE; R3 0/8/4 REVISE; R4 0/9/3 REVISE; R5 0/9/3 REVISE; R6 0/9/3 REVISE | parsed (suite dry run, no error) |
| `genesis/a2o/features/doorway/projection-withdrawal.feature` | new | 3: withdrawal clears all three doorways; withdrawal ends the publication and the household keeps the notice; a paused doorway drops its copy before serving again (@requires:owned-substrate) | `served-under-standing` (existing) | 6, not closed | R1 0/5/2 REVISE; R2 1/5/2 READY; R3 0/2/2 READY; R4 0/1/1 READY; R5 0/5/4 READY; R6 0/3/1 READY | parsed (direct @cucumber/gherkin parse: Background + 3 scenarios) |
| `genesis/a2o/features/dataplane/doorway-failover.feature` | extended | 1 Scenario Outline (@requires:owned-substrate @wip): a published page stays readable through the loss of one of the two doorways (each direction) | `doorway-failover` (existing) | 6, not closed | R1 1/6/4 READY; R2 0/7/2 REVISE; R3 0/6/2 READY; R4 0/6/2 READY; R5 1/5/2 REVISE; R6 0/5/2 READY | parsed (suite dry run, no error) |
| `genesis/a2o/features/federation/outward-projection-optional.feature` | new | 2: a household that refuses the outward offer keeps a board that does everything it did (Outline: declines, unanswered); withdrawing an approved outward projection stops what goes out and the board loses nothing | `bridge-optional` (**new**: no habit declares it and no check names it) | 6, not closed | R1 1/2/4 REVISE; R2 0/6/2 READY; R3 2/2/3 REVISE; R4 0/4/4 REVISE; R5 1/2/2 REVISE; R6 0/1/3 READY | parsed (suite dry run, no error) |

A suite dry run loads the whole suite (1627 scenarios), so it cannot count one file's scenarios. A clean finish still means no loaded file failed to parse.

## Findings left for the operator to defer

These are the round-6 findings still open. Most of them are in text that was already in the file before this group's edits. The author did not change that text.

**served-under-standing.feature**: all of these are in older text.
1. The phrase "polled without a refresh instruction" is defined only inside the held @wip outline. The definition should move to the preamble.
2. "provider" is never defined (lines 19, 133).
3. "the collective's challenge address" is never defined (lines 94, 109, 121, 148).
4. Nothing shows where the contract's 72-hour response promise comes from (line 142).
5. "peer membership record" is never said to be written by anyone, or read from anywhere (lines 98, 99, 115, 140).
6. The note separating "public permission" from the public reach level is only in the @wip outline. It should move to the preamble.
7. The byte marker's form is never stated (lines 83, 103).
8. "protocol-form" URL or link is never defined (lines 62, 137).
9. The "project-" prefix in "project-epr" is never defined (lines 16, 133).

**projection-withdrawal.feature**: all of these are in our own text.
1. The description never says the contract records the time of the withdrawal (line 110).
2. The resume step at line 126 joins two timing conditions in one step.
3. "the protocol" is never named (line 18).

**doorway-failover.feature**
1. "ch10" (line 174) is unexplained. Older text.
2. The stage names "simulacra, bootstrap, coordinated or enforced" are unexplained (line 81). Older text.
3. The description never says what makes the "current broken topology" broken (line 29). Older text.
4. "commons" is not explained at lines 119 and 148, which are older text. At line 245, our own text, it is explained.
5. The same-head step's conditional meaning is stated only in the description (lines 44-47). Optional.

Two more items from earlier rounds are also still open. Both are in older text: the typo "returned503" at line 6, and a gloss for "declared head", which the author offered to add inside our own outline.

**outward-projection-optional.feature**
1. The manifest field for outward projection does not exist yet. Only the comment block says so, not the description (lines 7-9 against 30-33).

## Author's notes

- Changes to existing files were append-only. No existing scenario, tag, description or Background was changed. Every new scenario is @wip, either directly or through a feature-level tag. Nothing was staged, committed or pushed.
- Nobody owns the older text in served-under-standing and doorway-failover. Its findings come back every round and will not converge until someone owns that text. That is why both files hit the round cap.
- Choices to revisit:
  1. The new reach outline leaves out "public" (it sits just inside commons) and "self".
  2. HTTP 410 for a withdrawn publication is the author's choice. The spec does not fix a status code.
  3. outward-projection-optional is tagged @act:i without @requires:household-nodes. Should a single-peer story get its own act?
  4. Slice 9 should probably name the `outward` manifest need.
  5. Doorway copy reports, which projection-withdrawal relies on, are not built yet. They are tied to section 15's projection-lifecycle node and slice 9's `projectable`.
- No story proves that the apex name keeps answering during a doorway loss. The new failover outline proves only that the page stays readable at the surviving doorway's own address.
- `bridge-optional` needs a habit atom, or an existing habit's `checks:` entry, before it joins the register.
- Covenant: no variance floor is held for these paths, so the full loop was owed and was run. A floor may be warranted for the older text in the two extended files, so that this lane is not held responsible for it. That is the steward's decision, not this lane's.
