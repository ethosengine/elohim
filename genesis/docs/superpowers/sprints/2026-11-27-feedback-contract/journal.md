---
id: feedback-contract-sprint-journal
title: Feedback contract — sprint journal
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), designing; shem session cacccb55 integrating
serves: [private-feedback-loop, dev-system-equilibrium]
cites:
  - "feedback-loop-contract-plan | the plan whose FC stations are this sprint | sha256:2c3e5df3647bf24c | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the spec the sprint takes into a contract | sha256:120869ada259e93d | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion that sketches the contract and the producers it must serve | sha256:49f57459097ec394 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
---

# Feedback contract — sprint journal

## What the sprint is for

The steward, 2026-11-27, as typed: "let's start a sprint making sure we've got the right
primitives/interfaces/compositions to take this spec into it's contract so we can help drive
consistency through other sprints.."

The spec is the lifecycle spec and its companion. The contract is what a producer declares and
calls to join the loop: its terms, one call, one reading, a few acts and a store's limits. The
sprint fixes those as shapes other work can cite, before more of the loop is built against them.
Its stations are FC-1 to FC-8 of the plan
(`../../plans/2026-11-27-feedback-loop-contract-plan.md`).

## What the steward has and has not said

- He stands as steward of record for the designing session (covenant clause `bafyreihb7a…aucuu`, in his words).
- He directed that the two specs be committed, joined to the plan and pushed, and that this sprint start. The designing session reads that as direction to proceed with the contract. It is not a ruling on any of the nine decisions in the lifecycle spec's section 16 or the three in the companion's section 12.
- The session's covenant still asks its purpose and the paths it may write. He has not stated paths, so no purpose clause is recorded.

## Order

1. FC-1, the inventory of primitives. It decides where FC-2 to FC-5 put things.
2. FC-2, the declaration's schema, checked against brit's real table.
3. FC-3 to FC-5, the call, the reading and the acts.
4. FC-7, the checklist, with brit as the worked example.
5. FC-8, the pointer that carries the contract into other sprints.
6. FC-6, a store's limits, when the steward has ruled on bringing stores under rules.

FB-1 and FB-2 are briefed for the ethosengine coordinator and held until FC-3 has fixed the call
they build against. The brief is `genesis/local-dev/f4-ask-20261123/BRIEF-feedback-fb1-fb2.md` on
shem, outside the repository.

## Who does what

- The designing session writes the FC stations' documents and schemas and hands commits to the integrator in closed batches.
- The shem integrator lands on dev. No other session pushes dev.
- The brit lane on thinkpad owns brit's side of the seam; its diffs are read by the designing session before brit main.
- Code on the epr side is built on ethosengine, relayed through the integrator.

## Entries

### 2026-11-27, opening

- State of the designs: the lifecycle spec and the story landed on dev in the integrator's first landing of this work; the companion and the plan are in the next batch.
- brit: `run-ci/feedback-f1` at `9e14426e0` reads clean against the seam; nothing further is asked before brit main. Read, not built or run.
- Found while reading brit's diff: brit 0.3.9 sends an operation word no epr accepts. The cure on brit's side is in that branch. The cure on the epr side is part of FB-1.
- Reported and not fixed: the app's issue report sends a diagnostic bundle with no preview; six domain manifests declare feedback vocabulary no runtime code reads. Both are read from code by a survey.
- Owed to this sprint by nobody yet: an isolated read of the companion's sections on self-review, the retirement gradient and the levels of the loop, which were written after its one read.
- The steward asked that this work be reconciled with the Observer Protocol before anything was finalised: "this is us re-discovering the observer epic from the perspctive of logs/telementry/observations we need as developers... and not as visionaries". The companion's section 1b now sets the three side by side, names seven seams and eleven pieces one of them has and the others need, and lists the doors. Stations FC-9 and FC-10 came from it. FC-10 edits canonical documents; he gave his word the same day ("we'd want those edges to surface if an agent returns and is graph walking"), and four of its five pointers landed with this batch.
- The first landing attempt was refused. A habit line under `elohim/epr/` put that crate's tree into the closure of about thirteen gate projects for a documents landing, and one of those gates failed on a test of its own. Nothing reached dev. The work was rebuilt as one documents commit and one habit commit, which the integrator carries separately.
- The integrator's reader found one error of fact, since corrected: the feedback store has three write verbs (`capture`, `ingest-genesis`, `record`), not one.
- The steward, on what the day showed: "Discoverability.. that's been a bit a latecomer in the designs we've pursued". The evidence is in this sprint's own history: five survey lanes and a prior-art pass did not surface the Observer Protocol or the SDK pipeline's named gaps until he pointed at them. Recorded as a datapoint on backlog item `progressive-discovery-substrate-emergent-concern`.
