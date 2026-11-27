---
id: feedback-contract-sprint-journal
title: Feedback contract — sprint journal
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), designing; shem session cacccb55 integrating
serves: [private-feedback-loop, dev-system-equilibrium]
cites:
  - "feedback-loop-contract-plan | the plan whose FC stations are this sprint | sha256:220151a0b1a2d79f | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the spec the sprint takes into a contract | sha256:bb8216eb2f736464 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion that sketches the contract and the producers it must serve | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
---

# Feedback contract — sprint journal

## What the sprint is for

The steward, 2026-11-27, as typed: "let's start a sprint making sure we've got the right
primitives/interfaces/compositions to take this spec into it's contract so we can help drive
consistency through other sprints.."

The spec is the lifecycle spec and its companion. The contract is what a producer declares and
calls to join the loop: its terms, one call, one reading, a few acts and a store's limits. The
sprint fixes those as shapes other work can cite, before more of the loop is built against them.
Its stations are FC-1 to FC-10 of the plan
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

FB-1, the founding slice of code, is built by the designing session and not on ethosengine. The
integrator relayed the steward's question to it (as relayed by the integrator, 2026-11-27: "i
think, we should let that planning session work on minting that founding slice what do you
think?") and his answer; the steward then
confirmed to the designing session directly, as typed: "yes, but coordinate with the integrator,
if we could save wall-clock by using emerging integration primitives we've been working on
elsewhere.. the integrator can help you." The brief written for ethosengine is not sent. The slice
stays inside `elohim/eprfs/epr-cli`, changes no record shape, and waits on none of the steward's
open decisions.

## Who does what

- The designing session writes the FC stations' documents and schemas and hands commits to the integrator in closed batches.
- The shem integrator lands on dev. No other session pushes dev.
- The brit lane on thinkpad owns brit's side of the seam; its diffs are read by the designing session before brit main.
- The founding slice of code (FB-1) is built by the designing session in its own worktree. It does not review its own code: the integrator puts an independent reader on every hand-over before any gate. Later code stations are not assigned.

## Entries

### 2026-11-27, opening

- State of the designs: the lifecycle spec and the story landed on dev in the integrator's first landing of this work; the companion and the plan are in the next batch.
- brit: `run-ci/feedback-f1` at `9e14426e0` reads clean against the seam; nothing further is asked before brit main. Read, not built or run.
- Found while reading brit's diff: brit 0.3.9 sends an operation word no epr accepts. The cure on brit's side is in that branch. The cure on the epr side is part of FB-1.
- Reported and not fixed: the app's issue report sends a diagnostic bundle with no preview; six domain manifests declare feedback vocabulary no runtime code reads. Both are read from code by a survey.
- Owed to this sprint by nobody yet: an isolated read of the companion's sections on self-review, the retirement gradient and the levels of the loop, which were written after its one read.
- The steward asked that this work be reconciled with the Observer Protocol before anything was finalised. As typed: "this is us re-discovering the observer epic from the perspctive of logs/telementry/observations we need as developers... and not as visionaries". The companion's section 1b now sets the three side by side, names seven seams and eleven pieces one of them has and the others need, and lists the doors. Stations FC-9 and FC-10 came from it. FC-10 edits canonical documents; he gave his word the same day, as typed: "yeah, let's make sure we get those pointers.. we'd want those edges to surface if an agent returns and is graph walking, or there's an embedder that could benefit from the relationship..." Four of its five pointers are in this batch.
- The first landing attempt was refused. A habit line under `elohim/epr/` put that crate's tree into the closure of about thirteen gate projects for a documents landing, and one of those gates failed on a test of its own. Nothing reached dev. The work was rebuilt as one documents commit and one habit commit, which the integrator carries separately.
- The integrator's reader found one error of fact, since corrected: the feedback store has three write verbs (`capture`, `ingest-genesis`, `record`), not one.
- The steward, on what the day showed, as typed: "Discoverability.. that's been a bit a latecomer in the designs we've pursued". The evidence is in this sprint's own history: five survey lanes and a prior-art pass did not surface the Observer Protocol or the SDK pipeline's named gaps until he pointed at them. Recorded as a datapoint on backlog item `progressive-discovery-substrate-emergent-concern`.

### 2026-11-27, the founding slice is built

Who did what: the designing session wrote a contract for the slice and orchestrated; astra (Codex)
read the code before it was written and reviewed it after; one Opus agent built it in its own
worktree; the integrator's reader reads it before any gate of the integrator's.

What the reading and the building corrected in the design:

- **The participant is not stamped.** The session envelope names a steward and no participant,
  and its lookup takes a lock that can wait five seconds; brit gives the recorder two. The slice
  leaves the participant as the policy names it. The missing node is a read that answers "for
  whom does this act, and under what authority" without that lock.
- **Two producers leave two records.** The store identifies a source by producer, run and item.
  The design said one record per term per session; it is one per producer, term, outcome and
  emitter, under one shared run.
- **The emitter is part of the item**, so a tool replaced in the middle of a session is a second
  source and not refused as conflicting evidence.
- **A holding place with only unreadable files does not read as well.**

The same day the ethosengine coordinator, designing a service that runs unattended under a
steward's grant, asked what a record should name when there is no session. The two sessions
agreed, as design and not as ruling: the participant is the steward the grant traces to; the
grant's address rides one new closed detail key, `grant`; the answering session is never named;
and the read above is designed once, for sessions and for such services. That design is
`2026-11-27-carrier-acts-under-a-steward-grant-design.md` on ethosengine's branch, not on dev
when this was written.

What the independent review found, all fixed before hand-over, each with a test that fails
without the fix: the new `--dir` option was accepted by every feedback verb, which changed what a
legacy capture call does; a release address could be shaped like a path; two gap notes of one
session from two releases collided in the store and blocked the catch-up command for good (a
defect in the orchestrator's own ruling); a note could be read through a link, block on a pipe, or
be deleted after being swapped; and the scan of the holding place had no bound. The review found
the session key equal to brit's.

Version bumps: none. `elohim-epr-cli` is not published, and no lockfile moved.
