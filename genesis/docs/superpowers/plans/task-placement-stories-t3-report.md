---
id: placement-stories-t3-report
title: "Placement stories T3: what an app declares and what it inherits"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#17
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T3 report

Status is DONE_WITH_CONCERNS. All six files parse. None of them closed: the loop stopped at round 6 on every file, and the round-6 verdict was REVISE on two of them.

Counts per round are correctness/interpretability/preference. Every feature path below is under `genesis/a2o/features/`.

| File | New / extended | Scenarios added | Concern tags (new?) | Rounds (verdicts) | Per-class counts r1..r6 | Parse |
|---|---|---|---|---|---|---|
| federation/app-inherits-placement.feature | new | 4 in the first draft; the loop added a 5th (Plum jam, round 5) | app-inherits-placement (new) | 6: REVISE, READY, READY, READY, READY, READY | 1/3/4, 0/3/3, 1/4/2, 1/3/3, 0/3/4, 1/4/3 | parsed |
| federation/runtime-closure-shared.feature | new | 3 in the first draft; the uninstall scenario was split, giving 4 | code-residency-shared-closure (new) | 6: REVISE, READY, READY, READY, REVISE, READY | 0/3/4, 0/3/4, 2/4/1, 0/3/4, 1/3/2, 0/3/3 | parsed |
| resilience/household-diversity-dataplane.feature | extended | 3 in the first draft; the offer scenario was split, giving 4 | blob-durability (reused) | 6: REVISE every round | 2/7/1, 1/6/2, 1/7/2, 2/8/2, 0/6/3, 2/6/2 | parsed |
| storage/declared-storage-policy.feature | extended | 2, under a new `Rule:` with its own Background | blob-durability (reused) | 6: REVISE every round | 0/5/5, 0/6/3, 0/6/4, 0/6/5, 0/6/3, 1/6/4 | parsed (8 children) |
| delivery/space-scoped-channel.feature | new | 4 | space-scoped-channel (new) | 6: REVISE, REVISE, REVISE, READY, READY, READY | 0/4/4, 2/3/2, 3/4/2, 0/2/2, 0/2/2, 0/3/5 | parsed |
| delivery/app-bundle-elected-delivery.feature | extended | 1 (the 9th scenario, placed before the boot guard) | app-bundle-elected-delivery (reused) | 6: READY, READY, REVISE, READY, REVISE, READY | 0/6/2, 1/2/2, 2/7/2, 1/5/1, 1/6/3, 1/4/3 | parsed |

About the parse check: each dry run loaded the whole suite (1627 scenarios), because the cucumber config adds its own feature paths. A parse error in any file would have stopped the run, and none did. declared-storage-policy was also parsed on its own with `@cucumber/gherkin`.

## Findings left for the operator to defer

These are the round-6 findings that are still open, by file.

- **app-inherits-placement**:
  - The Background's third Given packs three needs and a derived rule into one step.
  - The step "had time to see which copies one another hold" names nothing observable.
  - The phrase "with nothing changed" in the Scenario 2 title is too broad (correctness).
  - The scenarios never say whether a holding is confirmed or only claimed.
  - "fingerprint" is undefined (optional fix).
- **runtime-closure-shared**:
  - A Background step restates content addressing instead of arranging state.
  - The Background step that starts "And so..." is a deduction, not setup.
  - It is unclear whether the holds report is the feature under test or the tool used to check it.
- **household-diversity-dataplane**:
  - The feature title promises spreading across households, but the hub scenarios are about machines inside one household (correctness, our text).
  - The doorway and storage glosses are repeated in four scenarios instead of once in the Background.
  - The first hub scenario's title makes two claims.
  - Pre-existing text that is not ours:
    - "pure XOR";
    - "humans rows / agent keys";
    - the code identifiers and SELF_CID in comments;
    - "can span distinct households" (correctness).
- **declared-storage-policy**:
  - The Rule Background is said not to restate the policy parameters (correctness). This is disputed: the round-5 edit did restate them, and Gherkin runs the feature Background before a Rule's Background, so it does not replace it.
  - Pre-existing text that is not ours:
    - "shelved" is undefined;
    - "cellar destinations";
    - the meaning of "carry 2 hours";
    - "almost nothing";
    - Maria and the cousin are never introduced.
- **space-scoped-channel**:
  - The `E2E_DOORWAY_ALPHA` token is never explained.
  - It is unclear whether the rota app is a real app or an example.
  - "ceremony" is undefined.
- **app-bundle-elected-delivery**:
  - The single-candidate rule lives in our scenario, not in the vocabulary section.
  - Pre-existing text that is not ours:
    - the vocabulary's claim that reverting leaves no candidate contradicts its own positional rule (correctness);
    - "ran clean";
    - "admin endpoint";
    - "Lane N / N1-N6";
    - the overview does not list the 9th scenario.

## Author's notes

- **Tags**:
  - The three new files are `@wip @requires:household-nodes @act:i`. None is tagged `shem`.
  - The scenarios added to existing files carry `@wip` at scenario level. The two in declared-storage-policy also carry `@concern:blob-durability`.
  - In the existing files, only lines our group added were edited.
- **New concern tags**: no habit's `checks:` names any of them. `app-inherits-placement` is the habit that spec §17 proposes, and it is the operator's to mint at slice 9.
- **Where each story waits**:
  - app-inherits-placement: slices 9 and 5.
  - runtime-closure-shared: slices 9 and 10.
  - space-scoped-channel: slice 9.
  - The hub scenarios: slice 5, plus a hub to add. `elohim-hub` has no code.
  - The loss-model scenarios: slices 9 and 5.
  - The offline-peer scenario: no slice. It uses delivery mechanics that already exist.
- **Scope limits**:
  - space-scoped-channel covers only a space whose channel runs ahead of the commons. §19 question 11 is still open.
  - The hub scenarios keep §7.2's no-move rule.
- **Covenant**: no variance floor is held for these paths, and none was self-granted. None of the files closed, so blind-reader rounds are still owed under the a2o rule.
- Nothing is staged or committed.

## Correctness pass (2026-10-20)

| File | Fix rounds (verdict, correctness/interpretability) | Confirming reader | Correctness still open | Parse |
|---|---|---|---|---|
| `genesis/a2o/features/delivery/app-bundle-elected-delivery.feature` | R1 READY 1/5; R2 REVISE 2/5 | REVISE, 2 correctness / 5 interpretability | 2 (below) | PARSED (cucumber-js dry-run, whole suite of 1628 scenarios; also @cucumber/gherkin) |
| `genesis/a2o/features/federation/app-inherits-placement.feature` | R1 READY 0/3 | READY, 0 / 3 | none | PARSED (cucumber-js dry-run, 1628 scenarios) |
| `genesis/a2o/features/storage/declared-storage-policy.feature` | R1 REVISE 2/4; R2 REVISE 0/6 | REVISE, 0 / 6 | none | PARSED (@cucumber/gherkin, 8 top-level children; cucumber-js dry-run) |
| `genesis/a2o/features/resilience/household-diversity-dataplane.feature` | R1 REVISE 3/7; R2 REVISE 2/6 | REVISE, 2 / 6 | 2 (below) | PARSED (@cucumber/gherkin, 3 top-level children, no edits) |

Correctness findings still open, for the operator:

- **app-bundle-elected-delivery, lines 98-107**: the description names nine scenarios by position, but the file has ten. The non-canary-peer-away scenario (line 206) is never mentioned. Repair: add a sentence for it between the eighth and "the last".
- **app-bundle-elected-delivery, line 2**: "Lane N / stations N1-N6" is a placeholder. Repair: use the real lane name and stations, or remove the reference.
- **household-diversity-dataplane, line 120**: the Then says jessica's own photos stay on her laptop, but no Given sets those photos up. Repair: add a Given that she has saved her own photos on her peer.
- **household-diversity-dataplane, Rule at line 36**: the Rule claims a universal invariant, but its scenarios only prove it on the hub-addition path. Repair: narrow the Rule title to the hub case, or add a scenario for three peers with no hub.
