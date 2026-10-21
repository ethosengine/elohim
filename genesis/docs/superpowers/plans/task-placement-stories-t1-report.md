---
id: placement-stories-t1-report
title: "T1 report: placement-contract section 16 stories under the blind-reader loop"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#15
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T1 report

Seven new feature files cover all eight T1 stories. No existing file was edited and no new directory was made. Every scenario is `@wip` and has no step definitions. Every file parses. **No file closed the loop**: each one reached round 6, the last reader said READY, and that reader's findings are listed as deferred below. That is why the status is DONE_WITH_CONCERNS.

Counts per round are correctness/interpretability/preference (C/I/P). Scenario counts come from the authored list, plus the scenarios added during revision where the parser showed more children. Files 4, 6 and 7's dry runs reported counts for the whole suite (1627), so their per-file counts below come from the revision notes.

| File (genesis/a2o/features/) | New or extended | Scenarios | Concern tags | Rounds | C/I/P per round | Parse |
|---|---|---|---|---|---|---|
| storage/materialize-what-is-read.feature | new | 4 (Background + 4, one an Outline with 3 rows) | materialize-what-is-read (new as an a2o tag; the habit exists but is unwired) | 6 | R1 REVISE 1/2/4 · R2 REVISE 0/4/2 · R3 READY 1/5/2 · R4 READY 1/1/2 · R5 REVISE 1/2/1 · R6 READY 0/2/2 | ok, 5 children |
| dataplane/reach-on-tree-root.feature | new | 6 (Background + 6; scenarios 1 and 2 are Outlines over matthew, jessica, james) | reach-enforced-everywhere (new as an a2o tag; the habit's only check names reach-enforced-http) | 6 | R1 REVISE 2/3/2 · R2 READY 1/3/1 · R3 READY 0/3/2 · R4 READY 0/3/4 · R5 READY 0/2/3 · R6 READY 0/4/2 | ok, 7 children |
| lms/kept-offline-app-opens.feature | new | 4 (Background + 4) | kept-offline (new, **no habit**) | 6 | R1 READY 0/3/2 · R2 READY 0/3/3 · R3 READY 1/3/1 · R4 READY 0/4/2 · R5 READY 1/2/3 · R6 READY 0/4/3 | ok, 5 children |
| storage/derived-by-identity.feature | new | 7 (Background + 7; a match-confirms scenario and a copying scenario were added in revision) | materialize-what-is-read (new as an a2o tag) | 6 | R1 REVISE 1/3/3 · R2 READY 0/3/4 · R3 READY 0/3/5 · R4 READY 0/3/4 · R5 READY 0/4/3 · R6 READY 1/3/4 | ok (whole-suite dry run, no parse error; R5 direct parse: Background + 7) |
| storage/index-rederivable.feature | new | 3 (Background + 3) | recall-reaches-authority (existing) | 6 | R1 REVISE 1/4/2 · R2 READY 0/4/2 · R3 READY 0/2/4 · R4 READY 1/2/2 · R5 READY 1/3/3 · R6 READY 0/2/4 | ok, 4 children |
| devflow/repository-depth-by-device.feature | new | 5 (Background + 5) | brit-native-daily-driver (existing) | 6 | R1 REVISE 2/3/3 · R2 REVISE 1/2/4 · R3 READY 0/4/3 · R4 REVISE 2/3/3 · R5 READY 0/3/5 · R6 READY 0/2/3 | ok (whole-suite dry run, no parse error; R5 direct parse: 6 children) |
| devflow/agent-session-holds-its-read-set.feature | new | 5 (Background + 5; sealing was split into seal and take-in-part) | brit-native-daily-driver (existing) | 6 | R1 READY 1/4/1 · R2 READY 1/4/3 · R3 READY 1/4/2 · R4 READY 0/3/2 · R5 READY 0/3/3 · R6 READY 0/4/2 | ok (whole-suite dry run, no parse error; R2 direct parse: 6 children) |

## Findings left for the operator to defer

These are the round-6 findings, which nobody revised. All are interpretability findings except one correctness finding (marked).

**materialize-what-is-read**
- "store" is not in the glossary (line 60).
- "object" is not defined as "a file or a folder listing" (lines 45, 56).

**reach-on-tree-root**
- The two kinds of reach (the folder's metadata takes the root's reach; each file keeps its own) are not signalled as the key idea (line 44).
- The Background states reach on the root as fact, while the open design decision is whether reach goes on the root at all (lines 65 and 72).
- Scenario 1 says "opens" where it should state the outcome (line 81).
- Scenario 3 covers one peer and Scenario 2 covers three, and the difference is not explained.

**kept-offline-app-opens**
- "each of the following" implies a table that is not there (line 62).
- Scenario 2 depends on the lesson 1-2 versus 3-4 split, which only the Background states.
- The slice and section numbers are opaque without the spec.
- "and still does" reads as a check the offline laptop makes, when it is a fact about the test world (line 67).

**derived-by-identity**
- **Correctness:** the Background assumes the distribution rule that Scenario 1 is meant to prove, so the proof is circular (line 79 against lines 83-88).
- The recipe's "author" is not named as a role.
- Reach governs both read access and distribution, and the file does not say these are the same list on purpose.
- The concern tag's link to "derived by identity" needs a bridging clause.

**index-rederivable**
- Why at least two matching recipes are needed is never stated (line 72).
- The shape of a "no index" answer is not described (lines 89-90).

**repository-depth-by-device**
- The Scenario 5 title says "unpushed", a git term the glossary avoids.
- The HELD dependency chain lives only in comments.

**agent-session-holds-its-read-set**
- What the cross-referenced ci-runner-stewarded story proves needs a one-line summary.
- The mechanism by which a thrown-away session gives up the fetched content is not named.
- The HELD status lives only in comments in this file.
- "takes" (the changes) is not defined.

## Author's notes

- **Holds.** Each feature header names the section 17 slice it waits on:
  - slice 1: the local read in materialize-what-is-read
  - slice 2: the rest of materialize-what-is-read, and reach-on-tree-root (plus section 19 question 10)
  - slices 3 and 9: kept-offline
  - slice 4: derived-by-identity and index-rederivable
  - slice 7, standing on slices 3 and 5: the two brit stories
- **Concern tags.**
  - recall-reaches-authority and brit-native-daily-driver already exist.
  - materialize-what-is-read and reach-enforced-everywhere are new as a2o tags. Their habits are already declared. reach-enforced-everywhere's habit checks only reach-enforced-http, so this tree route needs to be added to its `checks:`.
  - kept-offline has no habit at all. One needs declaring, or the tag needs remapping.
- **Requires tags.**
  - `@requires:household-hub` is a fixture precondition that matches no cluster-state capability.
  - reach-on-tree-root and derived-by-identity carry `@requires:household-nodes` at feature level.
  - materialize-what-is-read carries it per scenario.
  - The devflow stories carry `@act:host @requires:brit-cli`.
- **Rulings carried.**
  - A device's facts never leave it.
  - Participants ratify.
  - Reach comes before bytes, on both sides.
  - Fetching a derived output is an optimisation, not a condition.
  - Doorways are plural participants, and none is framed as a defect.
- **Doorway in the index story.** index-rederivable deliberately has no `doorway "alpha"` Background step, so it departs from genesis/a2o/CLAUDE.md on purpose.
- **Covenant floor.** No covenant floor is held for this work, so the full loop applied. A covenant floor for the round-6 residue may be warranted. Only the steward can grant one.
- **Not done.** Nothing was staged, committed or pushed.

## Correctness pass (2026-10-20)

| Group | File | Fix rounds | Confirming verdict | Correctness | Interpretability | Correctness still open | Parse |
|---|---|---|---|---|---|---|---|
| T1 | genesis/a2o/features/storage/derived-by-identity.feature | 1 (no fix needed) | READY | 0 | 4 | none | PARSED (`cucumber-js --dry-run` over 1628 scenarios, no parse error) |
