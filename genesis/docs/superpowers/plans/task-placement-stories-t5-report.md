---
id: placement-stories-t5-report
title: "T5 report: placement-contract stories (identity, agent standing, inference)"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#19
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T5 report: placement-contract stories

Status is DONE_WITH_CONCERNS. All seven files parse. None of them closed: every file used all six blind-reader rounds and still had findings open at the end. Round counts are listed as correctness/interpretability/preference, with the verdict.

| File | New or extended | Scenarios added | Concern tags (new marked *) | Rounds | Per-class counts per round (c/i/p, verdict) | Parse |
|---|---|---|---|---|---|---|
| genesis/a2o/features/auth/hosted-human/08-told-who-holds-my-key.feature | new | 5 | hosted-compute-contracted (reused, habit hosted-cell-promised) | 6, not closed | R1 0/4/1 REVISE; R2 0/5/2 REVISE; R3 0/7/1 REVISE; R4 0/3/2 READY; R5 1/2/1 READY; R6 0/4/2 READY | parsed (suite dry-run clean) |
| genesis/a2o/features/auth/remote-signing/01-device-signs-hub-runs.feature | new (new dir + .epr-meta) | 4 as briefed; 5 after review added a choosing scenario | device-key-remote-cell * | 6, not closed | R1 0/3/2 READY; R2 0/4/1 READY; R3 0/5/1 READY; R4 1/2/1 READY; R5 1/1/2 REVISE; R6 0/4/2 READY | parsed (suite dry-run clean) |
| genesis/a2o/features/auth/recovery/identity-key-lineage-recovery.feature | extended | 1 as briefed; 2 after review split it | identity-lineage-recovery * | 6, not closed | R1 1/8/0 REVISE; R2 0/7/3 REVISE; R3 0/8/3 REVISE; R4 1/6/5 REVISE; R5 0/9/3 REVISE; R6 1/10/3 REVISE | parsed (suite dry-run clean) |
| genesis/a2o/features/auth/recovery/root-secured-at-first-contact.feature | new | 4 | root-held-by-network * | 6, not closed | R1 1/3/3 REVISE; R2 2/2/3 REVISE; R3 1/2/3 READY; R4 2/1/2 REVISE; R5 0/3/3 READY; R6 1/3/2 REVISE | parsed (suite dry-run clean) |
| genesis/a2o/features/compute/ci-runner-stewarded.feature | extended | 1 | operator-runtime-surface (reused) | 6, not closed | R1 1/4/2 REVISE; R2 0/6/2 READY; R3 0/4/3 READY; R4 1/2/2 READY; R5 1/5/2 REVISE; R6 2/5/2 REVISE | parsed (gherkin parser, 7 children; dry-run refused by the RAM guard) |
| genesis/a2o/features/stewardship/agent-act-attributed.feature | new | 5 as briefed; 7 after review | acts-attributed-to-participants * (the habit exists, but no a2o check named it before) | 6, not closed | R1 0/6/1 READY; R2 1/3/3 REVISE; R3 0/3/2 READY; R4 0/2/3 READY; R5 1/4/4 REVISE; R6 0/3/2 READY | parsed (suite dry-run clean) |
| genesis/a2o/features/compute/inference-grant-revoked.feature | new | 4 | counsel-context-erasable * | 6, not closed | R1 1/4/2 READY; R2 1/2/4 REVISE; R3 1/4/2 REVISE; R4 1/4/2 REVISE; R5 1/3/4 REVISE; R6 0/3/2 READY | parsed (suite dry-run clean) |

There are also three new domain tags: @remote-signing, @agents and @inference. The suite dry-run loads every feature, so its count (1627 scenarios) is the suite total, not a per-file count. A parse error in any loaded file would have stopped the run, so a clean finish shows each file parses.

## Findings left for the operator to defer

These are the round-6 findings that were never resolved. Each is named here so it can be deferred deliberately.

- **08-told-who-holds-my-key**
  - The Background uses Given for steps that only check what registration already did.
  - The story does not say whether the peer outside the pool is part of the mesh fixture or has to be set up.
  - It never says what "tells the application to go on" is as an action.
  - The `E2E_DOORWAY_ALPHA` reference reads like a literal string.
- **01-device-signs-hub-runs**
  - The Background names no doorway or test entry point.
  - "Offered" is used before the text that defines it.
  - Scenario 2's Then block mixes the steps of the signing mechanism with the outcomes it checks.
  - The writer in scenario 3 is only "something other than her phone".
- **identity-key-lineage-recovery**
  - One correctness finding is in our text. The Given "the app opens Grandma's documents only on a device enrolled…" states a rule of the system, not a precondition.
  - The other ten findings are in text that was there before our work, and their owner should decide them: graduated identity, REA, household topology, rotation vs enrollment, structural imago-dei, recognition score, cid, controller-policy, the comment block holding wave status, and claimed contributor presence. The pre-existing scenarios 1 and 2 also lack @concern:identity-lineage-recovery.
- **root-secured-at-first-contact**
  - One correctness finding: "her identity root is held under a recovery quorum" sits in the When block of scenario 3.
  - The story calls itself a protocol contract for every app, but its steps check "the app's home screen".
  - It does not say how long the "just secured" message stays.
  - Key signing is never explained.
- **ci-runner-stewarded**
  - All seven open findings are in text that was there before our work.
  - Two are correctness findings. First, the only scenario that shows a runner's stage actually running is held. Second, "the first provider accepts" names an action, not an outcome.
  - The rest are interpretability: "accepted" is not defined, the withdrawal sentence runs on, "steward" shifts meaning, and "Then the approval is refused" names no one who refuses.
  - One is in our scenario: "each checked against the fingerprint" does not say who checks.
- **agent-act-attributed**
  - A paragraph break is missing at line 29.
  - It is unclear whether the model is recorded on the act or read from the agent's name.
  - The story never says that any participant may edit any contribution.
- **inference-grant-revoked**
  - Scenario 2 has no When step that makes the request it then shows refused.
  - It never says why Jessica's own peer cannot run inference.
  - It says her elohim "runs on her phone", but also that her phone is off or asleep much of the time.

## Author's notes

- I wrote one governance file, `genesis/a2o/features/auth/remote-signing/.epr-meta`. It is modeled on `auth/graduation/.epr-meta`: it covers the subtree and adds no rule beyond the parent's. I did not stage, commit or push anything.
- **Hub key disclosure is its own station (08), not a scenario in 07.** Station 07 proves the promise recorded on the notary. Station 08 proves what the person is told when an app opens, backed by a disclosure record. It reuses 07's provider as the holder that must be named. Follow-up for the series owner: `hosted-human/README.md` needs a station row and a list of step phrases for 08.
- **Lineage file:** the concern and @wip tags are on our scenarios only, so the existing scenarios do not fall under the new concern.
- **Runner scenario:** tagged @wip @requires:owned-substrate, like the scenarios beside it. Its own description extends reach from publishing to fetching.
- **Root at first contact:** its mesh needs are mixed. The provisional-notice scenario needs no network. The other three carry @requires:household-nodes. The story names no quorum size or timing, because §19 question 13 is still open.
- **Agent attribution:** the stewardship `.epr-meta` rule (grant-scenario-must-exercise-the-appeal) fired. No authority acts over anyone in this story, so the description says so, and an objection that reaches the sponsor takes the place of an appeal. The story needs no mesh tag.
- **What each new file waits on.** Every new file is @wip, and its header names the slice it waits on:
  - 08: the disclosure record (§15), with slice 10 as the hub runtime.
  - remote-signing: slice 10.
  - lineage and first contact: the KeyRotation mint path. Lineage's data half also needs slice 2.
  - runner scenario: slice 2.
  - agent attribution: the agent-attribution plane.
  - inference: slice 5's `inference` grant scope plus counsel-context erasure.
- **Edits outside my scope, left for their owners:**
  - `compute/.epr-meta` is stale. It still says "two feature files" and does not cover `inference-grant-revoked.feature`.
  - remote-signing carries §19 question 3 in its header: is a hub holding the key acceptable before remote signing exists?
- **No variance floor was held.** The loop ran its full six rounds without one, so these files end unclosed rather than closed under a floor.

## Correctness pass (2026-10-20)

| File | Fix rounds | Confirming reader | Correctness / interpretability | Open correctness finding | Parse |
|---|---|---|---|---|---|
| `genesis/a2o/features/compute/ci-runner-stewarded.feature` | 2 (R1 READY 1/4, R2 READY 1/4) | READY | 1 / 4 | Yes, see below | PARSED (cucumber-js dry run, 1628 scenarios loaded, no Gherkin error) |
| `genesis/a2o/features/auth/recovery/identity-key-lineage-recovery.feature` | 2 (R1 REVISE 1/6, R2 READY 0/9) | READY | 0 / 9 | None | PARSED (@cucumber/gherkin, 5 top-level children) |
| `genesis/a2o/features/auth/recovery/root-secured-at-first-contact.feature` | 1 (R1 REVISE 0/4) | REVISE (interpretability only) | 0 / 4 | None | PARSED (cucumber-js dry run, 1628 scenarios loaded, no parse error) |

**Open correctness finding, for the operator (ci-runner-stewarded, Scenario 2, lines 82-84).** The Then block mixes the results of two actions. The grant's reach and expiry follow from granting capacity (the When on line 80). Acceptance and the report follow from submitting the stage (the When on line 81). "The first provider accepts" also reads as a new action, not an observation, while the description (lines 38-39) says nobody decides by hand. Suggested repair: put the grant-property assertion straight after the grant action, or fold it into Scenario 1's hosting-grant pattern. Then word acceptance as an observation, for example "Then the first provider's check passes under that capacity grant", so Scenario 2 covers the submit, accept and report chain.
