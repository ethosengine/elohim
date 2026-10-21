---
id: placement-stories-t4-report
title: "T4 report: live channel, attention and connectivity stories"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#18
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T4 report: live channel, attention and connectivity stories

**Status:** DONE_WITH_CONCERNS. All seven files parse as Gherkin. None of them closed its blind-reader loop: the loop marks every file `closed: false` after round 6, and the round-6 findings are listed below for the operator to defer.

Per-round counts are given as correctness/interpretability/preference.

| File | New or extended | Scenarios added | Concern tags (new?) | Rounds | Counts per round (C/I/P) | Parse |
|---|---|---|---|---|---|---|
| genesis/a2o/features/dataplane/live-session-own-devices.feature | existing, unchanged | 0 | live-session-direct (existing, no habit names it) | 6 | R1 0/5/4 · R2 1/4/1 · R3 1/4/2 · R4 0/2/2 · R5 1/4/2 · R6 0/4/2 | parsed (suite dry-run, 1627) |
| genesis/a2o/features/federation/local-first-shared-sheet.feature | extended | 1 | live-session-plane (new) | 6 | R1 0/5/3 · R2 0/6/2 · R3 0/6/3 · R4 1/4/2 · R5 0/4/2 · R6 0/5/2 | parsed (@cucumber/gherkin, 8 children; dry-run blocked by RAM guard) |
| genesis/a2o/features/qahal/group-thread.feature | new | 3 | chat-is-a-record (new) | 6 | R1 0/4/2 · R2 1/3/3 · R3 1/2/2 · R4 1/3/3 · R5 1/3/2 · R6 0/2/3 | parsed (suite dry-run, 1627) |
| genesis/a2o/features/qahal/wake-a-sleeping-device.feature | new | 5 (7 after revision; the loop added a forged-signature scenario) | wake-reveals-nothing (new) | 6 | R1 3/3/1 · R2 1/2/4 · R3 0/4/2 · R4 0/3/3 · R5 0/3/3 · R6 1/1/2 | parsed (dry-run + gherkin, 7 children) |
| genesis/a2o/features/dataplane/household-at-rest.feature | extended | 1 | idle-is-free (reused, named by habit idle-is-free) | 6 | R1 0/5/2 · R2 0/4/3 · R3 0/5/2 · R4 0/3/2 · R5 0/3/4 · R6 1/3/2 | parsed (suite dry-run, 1627) |
| genesis/a2o/features/dataplane/peer-reach-through-relay.feature | new | 5 (6 after revision; the loop added an offer-withdrawal scenario) | reach-by-key (new) | 6 | R1 1/6/1 · R2 1/2/3 · R3 1/5/1 · R4 0/3/2 · R5 0/2/2 · R6 0/2/3 | parsed (suite dry-run, 1627) |
| genesis/a2o/features/browser/browser-to-hub-by-key.feature | new | 5 (6 after revision; the loop added a no-membership-pass scenario) | browser-to-hub-by-key (new) | 6 | R1 0/5/3 · R2 2/3/3 · R3 0/4/2 · R4 0/3/3 · R5 1/2/2 · R6 1/7/1 | parsed (suite dry-run, 6 undefined from this file) |

## Findings left for the operator to defer

**live-session-own-devices.feature** (none of this is T4's text; it belongs to the live-session slice-1 author)
- Interpretability: "database" and "chain" are never explained in "A session leaves nothing behind" (lines 101-102).
- Interpretability: "live-session channel started / not started" is never defined (lines 91, 93).
- Interpretability: the header calls the workspace "a fourth node", but the description counts it inside a household of three (line 7 vs lines 20-22).
- Interpretability: the `@requires:` tag meanings exist only in comments (lines 6-7).
- Raised in earlier rounds and still open:
  - Correctness: the rest-scenario baseline is counted before a restart and compared with a window after it (lines 90-97).
  - The difference between "held by" and "granted leave" is unclear, and "is shown nobody as here" (line 87) reads two ways.

**local-first-shared-sheet.feature** (pre-existing text, except the hub/host item)
- Line 62: the B7 conflict never says which value wins.
- Line 37: "reach" is never defined.
- Lines 83-116: the presence/host layer is not in the feature description. Fixing this means editing a description T4 did not write.
- Line 35: "peer" is not glossed.
- Line 8: `@act:iii` is never explained.

**group-thread.feature**
- Background line 56: the steps never say whether the thread starts empty.
- Scenario 3 (line 80) tests two properties and never says that the restart is what tells a record from a signal.

**wake-a-sleeping-device.feature**
- Correctness: Scenario 2 reads the record "for the household", but the company never learns that identity. It should read by delivery address or read the whole record instead (lines 91-96).
- Interpretability: line 79 asserts an internal signature check rather than something an observer can see.

**household-at-rest.feature** (pre-existing text, except the invariant)
- The owned-lane lease (lines 49-50) has no Given step.
- The invariant in Scenario 2's prose is carried by no Then step.
- Correctness: lines 32-34 say "6 a minute" without "per peer", unlike the table.
- `@requires:owned-substrate` is never tied to the lease.

**peer-reach-through-relay.feature**
- "Note" is never introduced as a data concept (lines 85-97).
- "The relay's own storage" (line 97) clashes with "the relay keeps no copy".

**browser-to-hub-by-key.feature**
- Background line 82 bundles six elements into one step.
- The steps never say who signs the membership pass, or where the browser's sign-in key comes from.
- The Scenario 2 title claims a fetch order ("before any doorway") that its steps do not exercise.
- "Jessica was asked to do nothing" names no particular prompt.
- The steps never say whether the doorway forwards the integrity report or stores it.
- The source of the app's list of files is not stated.
- The link to the design spec appears only in comments.

## Author's notes

- Row 1 was already told by live-session-own-devices.feature, so that file is unchanged. Its loop has nothing of T4's to close.
- Additions to existing files are confined to the new scenarios. No existing scenario, tag, description or Background was changed.
- Five concern tags are new: live-session-plane, chat-is-a-record, wake-reveals-nothing, reach-by-key and browser-to-hub-by-key. No habit's `checks:` names them yet. idle-is-free was reused.
- Requires tags:
  - every new feature has `@requires:household-nodes` and `@requires:owned-substrate`;
  - the wake and relay stories add `@requires:device-node`;
  - the browser story adds `@requires:doorway`;
  - none uses shem.
- For whoever wires the steps:
  - the browser story's doorways "alpha" and "beta" have to map onto the mesh's doorways A, B and C;
  - the story's relay on James's node has to map onto the standalone iroh-relay that hc-mesh.sh starts;
  - the shared-sheet story spans two households, but the mesh has only one.
- Already there before T4: household-at-rest.feature has no feature-level `@requires:household-nodes`. I left it as it was.
- Several revisions changed step text. None of these steps has a definition yet, so nothing is broken.

## Correctness pass (2026-10-20)

| File | Fix rounds | Confirming verdict | Correctness / interpretability | Correctness left open | Parse |
|---|---|---|---|---|---|
| genesis/a2o/features/browser/browser-to-hub-by-key.feature | 1 | READY | 0 / 5 | none | PARSED (cucumber-js dry run exit 0, whole suite of 1628 scenarios loaded; direct @cucumber/gherkin parse clean) |
| genesis/a2o/features/dataplane/household-at-rest.feature | 1 | REVISE | 0 / 4 | none | PARSED (@cucumber/gherkin: feature "A household at rest is quiet", 3 children) |
| genesis/a2o/features/qahal/wake-a-sleeping-device.feature | 1 | READY | 0 / 4 | none | PARSED (cucumber dry run clean over the whole suite; direct @cucumber/gherkin parse clean) |

No correctness finding is open for the operator. household-at-rest.feature's REVISE comes only from interpretability findings.
