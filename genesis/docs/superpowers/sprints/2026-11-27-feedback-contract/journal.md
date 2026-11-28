---
id: feedback-contract-sprint-journal
title: Feedback contract — sprint journal
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), designing; shem session cacccb55 integrating
serves: [private-feedback-loop, dev-system-equilibrium]
cites:
  - "feedback-loop-contract-plan | the plan whose FC stations are this sprint | sha256:6a2ac59f7325a637 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the spec the sprint takes into a contract | sha256:cf3dd75b4c907dc7 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
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

### 2026-11-27, the slice's gate run, and the inventory of primitives (FC-1)

The gate run on the slice. The designing session ran every selected gate once at the slice's
tip (`dd59d312f`, on dev's tip `e957207d0`), from 18:42Z to 20:09Z. Eleven of twelve were green,
among them the eprfs workspace's format, lint and tests on the pinned toolchain and the
acceptance suite's own gate. One was red: in the orchestrator's tree suite,
`device-notes-publish.test.mjs` "DEVICE_NOTES_PUSH=brit publishes through brit push". brit 0.3.5
refuses that push because its source and destination refs differ. The test fails the same way
when its file is run alone, and the slice touches nothing under `genesis/orchestrator`. It was
reported to the integrator and not changed. The integrator's account: its own session still
runs brit 0.3.4, which accepts that push, so the test passes in its pre-push and will fail in
every session started since the device's brit moved; it has put the gap to the brit lane. No CI
lane runs the two scenarios the slice un-tagged; local runs are their only evidence.

The inventory. `fc1-primitives-inventory.md` in this directory is station FC-1's table: 38
primitives, each with the type or schema that defines it today, or marked new. Codex (astra) read
it from the code; a Claude Haiku agent that had not written it opened every citation, about 149,
and found no mismatch; the designing session re-read one row against brit's later tip.

What it shows the next stations:

- **Three pieces have several lists of values.** Reach or ceiling has four (eight rings, five
  observation scopes, two ceilings in brit's table, four rungs in the lifecycle spec). Severity
  has three. Polarity has one list of two values that means two different things.
- **Six pieces are in the designs and in no code or schema**: the reading of a term, the summary
  that may leave a device, the grant a service acts under, the retire act, and the ask, answer and
  release of a sample.
- **Smaller differences** a schema will have to settle: time as an RFC 3339 string or as epoch
  seconds, two patterns for an observation kind's name, and a duration pattern that accepts
  anything starting with `P`.

The table chooses nothing. FC-2 to FC-5 choose, and where a choice is the steward's it waits for
him.

### 2026-11-27, a draft declaration schema (FC-2) and the table of acts (FC-5)

Who did what: Codex (astra) drafted the schema with its fixtures and check script, and separately
read the code for the table of acts; a Claude Haiku agent opened the table's code citations; a
Claude scribe agent wrote the table's prose; the designing session set the working rules, ran
every check itself, corrected what the checks and the readers found, and placed the documents.

FC-2. `fc2-declaration-draft.md` and `fc2/` hold a draft schema for the file a producer ships. To
check it against real bytes the designing session built brit 0.3.10 from brit's branch at
`a99143327`, in its own cargo slot, since no installed brit prints the declaration. The schema
accepts that output unchanged (72 claims, no cures) and refuses twenty-four fixtures. It is a
draft outside the SDK, and where it lives is still the steward's to decide. The integrator
advised that a check script in a documents directory would be run by no gate, so the script is
held back and comes as a test in the orchestrator's suite in its own batch.

FC-5. `fc5-acts-and-authority.md` is the table. Its finding is that the station's condition does
not hold today: nine of the eighteen acts that write need no named actor, and where one is needed
the code checks only that a string was given. The designing session's own slice is one of the
nine. Astra's first table had three citations that did not support their cell, found by the agent
that opened them, and the scribe's prose changed the meaning of six cells, found by the designing
session reading it against the code. All nine are corrected.

Not done: no gate ran for these documents beyond the address lint on the staged tree, and the
schema's check is not yet a test.

### 2026-11-27, the schema's check becomes a test

`genesis/orchestrator/producer-declaration-schema.test.mjs` now carries the FC-2 checks, with one
entry in the suite's list of reads. Codex (astra) wrote it from the hand-run script; the designing
session ran it (6 passed), made it fail three ways and restored it, and ran the orchestrator gate
on the commit: the named suite was green in 198 s. The same run's tree suite was red on the one
notes-publish test already recorded above, which depends on the session's brit and not on this
change.

Two corrections from the integrator's reader, applied: brit's code allows two ceilings and all 72
rows it ships say `device`; and the session key is a hash only when a session or invocation is
named, and a count of days otherwise.

The integrator's first push of the founding slice was refused by a different test of the tree
suite, one that fails when two gate runs overlap on the device, and not by the slice. Dev did not
move. The integrator pushes again when the device is quiet.

### 2026-11-27, the safeguards table (FC-9), and the design's judge corrected by its own schema draft

FC-9. `fc9-observer-safeguards.md` sets every line of the Observer Protocol's Part VIII beside
what the feedback designs answer and what is built. Codex (astra) made the mapping; a Claude
Haiku agent opened about 95 citations and searched again for every line marked unanswered,
finding one answer the mapping had missed, in the companion; a Claude scribe agent wrote the
prose; one blind reader found nothing to correct. Of eighteen lines, four are answered and partly
built, six are answered on paper and not built, one does not apply to a tool, and seven have no
answer: enabling stalking or harassment, preserving evidence only with consent or emergency,
cryptographic proof of operation, when observation happens, how patterns get used, access to
one's own data, and deletion on request. The table answers none of them. Where an answer decides
what a person is owed, it is the steward's.

FC-4, and what it found in the design. The designing session asked Codex (astra) for draft
schemas of the reading and the summary, with the order of the nine readings as cases any
implementation can run. Astra reported its draft incomplete and listed why: as the lifecycle
spec's section 6 was written, a cure that is still present reads `cure-available` again after
compaction, and that reading comes before `on-watch` and `retired` in the order, so neither could
ever be read. The fault was in the design, written by the designing session, and not in the
draft.

The designing session amended sections 5.5, 6 and 7 of the lifecycle spec and put them to three
fresh blind readers in turn, each asked whether the judge and the summary could be written from
the page without a guess. The first found the order repaired and four seams between the
sections; the second found four more; the third found two, which were corrected and not read
again. What the spec now says that it did not: one reading is of one claim and failing emitter;
which of the three cure readings holds follows from what records the device still has; a
reopening belongs to the unit it reopens; a summary reports what the device has done, so a claim
whose watch is met sends `cured` until its note is written; a summary carries a cure only when
the cure is a release; and where a watch cannot be met, a person's ruling with `--as` writes the
note. These are the designing session's corrections to its own proposal. No decision of
section 16 is touched.

FC-4 itself is not delivered. The draft is re-briefed against the amended sections.

### 2026-11-27, the integrator's reader finds two more faults in the judge, and FC-4 is drafted

The integrator's reader read the amended lifecycle spec and returned it: fix first. It found two
faults that three blind readers and Codex had missed.

- A claim retired by a person's ruling kept its aggregates, since no recovery link is written for
  them, and so still met the words for `on-watch`, which comes before `retired` in the order. It
  would have read `on-watch` for good, and a second ruling would have written a second note.
  The spec itself says this is how nearly all of today's records close.
- `reopened` was defined as observed again "from an emitter at or after its cure". Where the cure
  is a condition and not a release, that has no meaning, so such a claim failing again could not
  read `reopened`.

Both are corrected in the spec: a note for a claim's current cure ends its watch whether or not
the aggregates are still held, and a second note for the same cure is refused; and where the cure
names no release, a failing observation after compaction reopens the claim. "Nothing observed
since" now reads "no failing observation since". Three lines of the plan that described readings
in the old way are corrected (FB-2, FB-8, FB-10). This correction has been read by nobody but its
author.

The lesson the designing session takes: each of five readers found faults the one before had
not, and the one who found the deepest was the one asked whether the change was safe to land,
not whether the page read well. A design of a state machine wants its cases written down and run
before it is called read.

FC-4 is drafted. `fc4-reading-and-summary-draft.md` and `fc4/` hold the two draft schemas, the
mapping, fifteen facts, forty-one cases and forty-eight fixtures. Codex (astra) wrote them in two
rounds and wrote the test, `genesis/orchestrator/reading-and-summary-schema.test.mjs`; the
designing session added four cases for the reader's two findings, ran the test (8 passed), and
made it fail three ways and restored it, one of those being the reader's first finding put back.
One blind reader read the note; its findings are applied. The orchestrator gate has not been run
on the test: the integrator has asked for no gate runs until its landing is through.

To spare a Storybook build in CI for one fingerprint line, the seam map's cite of the lifecycle
spec is put back as it was and left stale. The corpus's stale count is 372 for that reason, one
more than before.

### 2026-11-27, a third fault of the same kind, and a test that tries every combination

The integrator's reader closed its two findings and returned the revision once more. A unit
whose only remaining records are ones compaction may not replace, a person's own or one tied to
a commitment, had no reading once a cure was present: the reference function in the test ended
by returning nothing. The reader had found it by trying all the combinations of the facts by
hand, and proposed that the test do the same.

The designing session did that. `open` is now what remains when no other reading holds, so the
rule is total; `retired` holds on the note for the current cure alone, which made one fact
unnecessary; and three consequences are written out in the spec, one of them stated as not
designed (how a unit of kept records closes). The test now runs every one of the 16,384
combinations of the fourteen facts and requires one reading for each where something failed, none
where nothing did, and all nine reachable. With the old rule put back, that test fails. Four
cases were added, making forty-five, and every line citation in the cases and facts was
remapped to the spec as it now stands and checked against the quoted words.

What the designing session did wrong, said plainly: it amended a state machine in prose four
times and asked readers to find what was missing, when the complete check was an afternoon's
loop over fourteen booleans. The integrator's reader found three faults that way before the
session wrote the loop.

Still open, by the reader's notes and now stated in the spec: a unit once closed by a ruling and
later cured again closes by a ruling again; and how a unit of kept records closes is not
designed.

### 2026-11-27, the producer checklist (FC-7), and a mistake of the design's that brit's code exposed

`fc7-producer-checklist.md` is the checklist: 22 lines a producer must meet to be on the loop as
the contract stands, with brit walked through each at its branch tip `a99143327` and one agent
hook walked through each to find where a script has no place. Codex (astra) derived it and made
both walk-throughs. It marked four of brit's lines as failing. A Claude Haiku agent then opened
about 170 of the cited places and read brit's marker and spool functions in full, and none of
the four stood as written: one is latent, one differs from a principle and not from the seam,
one is a real difference, and one was the design's own error. The lifecycle spec had said brit's
repeat marker lives under `feedback-notices/`; brit keeps it under `feedback-markers/` and uses
the other directory for a different thing, and brit's own documentation says so. The spec is
corrected. A Claude scribe agent wrote the prose.

The three differences are written in the document for the brit lane as observations to
reconcile and were passed to the integrator the same hour. Nothing in them says what brit should
do.

For the contract, the finding is the hook's: nothing says who admits a script as a producer,
what a script's emitter is, or how a script leaves a gap note. Those are three things FB-7 needs
before the hook can report on the loop.

The same hour the integrator reported that its four-hour landing of other work had failed in the
eprfs gate on two tests that refuse a key file "inside the git repository at /tmp". The cause was
this session's Codex legs: launched with a write sandbox that includes `/tmp`, they make `/tmp`
look like a git repository for moments at a time. The cure was already written down on this
device and the designing session had used half of it. Every write leg since is launched with
`/tmp` excluded, and the banner is checked.

### 2026-11-27, the judge is rebuilt from rules that are run

The integrator's reader closed its third finding and found two more, of a kind no check of
single states can catch: after its first recurrence a claim fixed by a condition read `reopened`
for good, and a claim could read `retired` on a note for a fix it had since reopened against.
The first is the ordinary life of a test that goes red and green.

The designing session stopped amending the prose and withdrew the five commits that carried it
from the integrator's queue. It wrote the judge down once as sixteen numbered rules. Codex
(astra) built a small model of one claim's life in the orchestrator's test and fifteen scripted
lives, 105 steps, with the reading and the summary state stated after every step. The designing
session ran them, read the main lives step by step, and put each of the two faults back: six
lives fail with one, one with the other. A Claude scribe agent rewrote the spec's section 6 from
the sixteen rules; the designing session checked that nothing outside that section and one
bullet of section 7 had changed, defined one term the rewrite had dropped, and put back seven
true statements it had removed.

What a person's own report cannot do is now said where the steward will see it: a claim reported
only by records a person wrote, or records tied to a commitment, reads `open` even after a fix,
because compaction may not replace such records and nothing else closes them. It is missing node
8 of the spec's section 7a and a tenth decision in its section 16.

Not done: nobody but the designing session has read the rewritten section; the integrator's
reader is asked to attack the lives. No gate has run on any of it.

### 2026-11-27, the model is attacked, and four more faults fall out in one pass

The integrator put a reader on the lives, as asked. It copied the model, wrote twenty-one lives
of its own, and changed the model twenty-four ways to see which changes any life would notice.
The five earlier faults each held. It found four new ones and seven changes that nothing
noticed.

- A reopening whose record a person wrote, arriving during the watch, was later retired under
  the very cure it counted against. `on-watch` and both ways of reading `retired` now require
  that nothing has counted against the current cure since it became current.
- The text and the model disagreed about when a reopening against a release ends, and neither
  was exact. A failure now counts against the latest taken release, at or before its emitter,
  that lists the claim, and the reopening ends when a later listing release has been taken.
- The text said a cure is present only while the running release lists the claim, which would
  drop every cure at the next release. The model was right and the text is corrected.
- A claim from an unknown release was sending summaries, which section 5.5 forbids. It sends
  none.

The rules are now seventeen. The reviewer's lives are kept under their own names beside the
others: forty-three lives, 295 steps, forty-nine cases, 55 tests. With the first of the four
faults put back, three lives fail.

What this showed: six rounds of readers on prose found five faults one at a time. One pass with
the model found four, and told the designing session which of its own lives were too weak to
notice a change. Two things the model cannot express are now said in the spec as not designed:
records dated out of the order they arrive in, and two units of one claim side by side.

The integrator also stopped the designing session's gate run 35 seconds in, because its own next
landing had just entered the same suite; only the address lint had run. No gate has run on any
of this since.

### 2026-11-27, the reader closes its four findings and names two more

The integrator's reader ran fourteen more lives and thirty-five changes against the revised
model. Its four findings are closed, and it judged every rule consistent with the model except
R4. Two things remained.

- When a release cure and a condition cure are both present, nothing said which one compaction
  acts on, and the model preferred the condition, after which a later release could not cure
  the claim again. The release cure now takes precedence, as rule R4 states.
- A device that skips the release that listed a claim never sees the cure, though it runs the
  fix. shem itself took brit 0.3.5, 0.3.9 and 0.3.10 in a day, and none of brit's 72 rows has a
  clearing condition, so for brit this is the ordinary case. The designing session did not
  design a cure for it: the answer is either a cumulative list on brit's side of the seam or a
  read of the releases in between on the device's side, and neither is its to choose alone. It
  is section 7a's node 9 and section 16's decision 11, and a life named for the gap pins what
  happens today.

The model now runs forty-eight lives, 325 steps, and 60 tests, and one guard the reader showed
to be redundant is removed. The reader's view is that a draft with a model that says what it
does not model can go on improving on dev.

### 2026-11-27, one more gap stated, and brit's answer on skipped releases

The integrator's reader made a last pass: the precedence finding is closed, the skipped-release
gap is stated adequately, every change to the model it had found unnoticed is now noticed. One
sequence remained. A claim compacted under a condition cure and then reopened stays `reopened`
after the device takes a release that lists the claim, and neither the retire command nor a
ruling moves it. The text and the model agree with each other and the outcome is still wrong. It
is stated in the spec as a second known gap and pinned by a life, to be settled with the first.

The brit lane answered the first the same hour, through the integrator. It prefers a cumulative
`cures` list and will ship it in brit 0.3.12: each entry `{claim, release, state, cite, summary}`,
append-only and pinned by a test, with a claim's state read from its latest entry in the
declaration of the head release. The designing session accepted, as a design position and not a
ruling, and asked for one change that brit took: the second state is `withdrawn`, not
`reopened`, because `reopened` is what a device reads from what it observes and a withdrawal is
the producer taking back its own claim. Brit has fixed that shape. The batch that followed changed the lives
first, then the rules, then the draft schema, and it replaced this statement of the gap and its
life before either reached dev: the two were handed over, accepted, and then folded into one
range with the batch the next entry describes.

### 2026-11-27, brit makes the list of cures cumulative, and one question goes to the steward

The gap a scripted life had pinned, a device that skips the release that carried a fix, went to
the brit lane through the shem integrator with two ways to close it. Brit chose a cumulative
`cures` list for its release 0.3.12 and fixed the shape: `{claim, release, state, cite,
summary}`, append-only and pinned by a test, a claim's state read from its latest entry in the
head release's declaration, releases ordered by semantic-version precedence. It took the
designing session's one request: the second state is `withdrawn`, since `reopened` is what a
device reads from what it observes. It also took a bound on its two marker directories (4,096
entries, seven days, lock files kept) and will write its limits into its own documentation. These
are design positions agreed between two lanes, not rulings.

The model was changed before any sentence of the spec. Codex (astra) did it in three rounds, each
launched with `/tmp` excluded from its sandbox. The first stopped honestly at two steps the rules
did not decide: after a fix is withdrawn and a later release fixes the claim again, the device
holds nothing new to compact, so the claim could not be watched or closed under the new fix. Two
rules were added for that: a release cure ends a reopening of either kind, which also closes the
second known gap; and a claim whose cure has been counted against moves to a later cure when the
retire command is next run, and its watch starts again. The third round stopped at one step
where the designing session's own brief had expected the wrong reading; the rule was right.

The integrator's reader judged the model sound against thirteen more lives. It also said that
one thing the new rule does is not the designing session's to decide, and the designing session
agrees. When a person's own report reopens a claim and a later release then fixes it, the new
rule would let a machine-checked recovery close the claim while that report stands; yet the same
report standing alone keeps a claim open for good. That is a sharper form of decision 10 of the
lifecycle spec's section 16. The model holds it behind one named switch, set so that the
person's report holds the claim open, and scripts both outcomes as paired lives. It is put to the
steward and not settled here.

The spec's sections 5.4, 5.5, 6, 7a, 12 and 16 were then written from one statement of the rules,
by a scribe agent, and checked by the designing session against the six places it was allowed to
change. The draft declaration schema follows brit's five fields and refuses build metadata in a
version; brit 0.3.10's own file is byte-identical and still accepted.

Counts: 61 lives, 432 steps, 63 cases, 16 facts, 80 tests across the two test files.

### 2026-11-28, two sentences where the prose and the model differed

The integrator's fresh reader walked twenty lives through the prose alone and then compared the
result with the model. Everything agreed except two sentences, and the model was right in both.
The spec said a watch starts at the compaction that made an aggregate; the model starts it at
the retire that made the unit's current cure current, so a claim moved to a later cure is watched
again, and it also requires at least one aggregate, without which "every aggregate" is true of
nothing. And the spec said a withdrawal counts only while it is the latest entry; the model
counts every withdrawn entry in the head's declaration, which is what lets a unit move to the
cure that follows it. Both sentences are corrected, with three statements of what brit "does"
changed to what brit has said it will do.

### 2026-11-28, the one call has four doors that agree on nothing (FC-3)

`fc3-one-call-four-doors.md` sets every field of an observation against the four doors that
record one today: `epr feedback capture`, `ingest-genesis` and `record`, which write to a device's
private store, and `POST /api/v1/observations`, which writes to storage. Codex (astra) read the
code; a Claude Haiku agent opened about 150 citations, confirmed every refusal message character
for character, tested the central claim itself and corrected four citations and one list; a
Claude scribe agent wrote the prose; the designing session corrected one row the scribe had
taken from an imprecise line of its brief.

The finding is that the station's condition cannot be met as things stand. No field is carried
by all four doors in one form; the time of observation is the only one that means the same thing
at all four, and it comes in two forms. One blind reader read the table; its two correctness
findings, a headline that said more than the rows and a row under the wrong heading, are fixed.
The same four names, `run`, `subject`, `kind` and one detail key, mean different things by
door. The table proposes no interface. What it gives the next station is the list of what an
interface would have to translate or leave out.

### 2026-11-28, a person's report is its own record

The integrator's reader had named two corners of the first rule that the prose did not mention
and no life exercised. Pinning them showed the model was wrong, not merely silent. It kept one
record per session and applied that to a person's own report as well: a report written in a
session that already held an automatic record replaced that record, and one written after that
record had been compacted left no record. The design says a record a person wrote is kept as it
is. The rule now says an automatic record is at most one per session and every report a person
writes is its own record, which replaces nothing and is not dropped. Codex (astra) changed the
model after the designing session wrote the rule; one earlier life changed its readings, and a
new life with its twin shows two reports in one session under both outcomes of decision 10.

The same batch removed a guard the reader had shown to be redundant, replaced the working names
in the lives' notes with the design's rule numbers and made the test refuse such names, and
settled three points of wording. Nothing of it has been read by anyone but the designing
session. 66 lives, 465 steps, 85 tests across the two files.

### 2026-11-28, a kept record comes about in two ways, and FC-8

A fresh blind reader of the lifecycle spec, asked about the four sentences the last entry
changed, found that the corrected first rule disagreed with section 5.2 and with the table of
section 7, which both still said one record per session. Codex (astra) then read the store,
read-only, and the designing session checked the lines it quoted. The same-source rule compares
the whole source: a repeat that differs only in its time returns the record already held, and
different evidence under one source is refused. A record a person writes carries the source its
author gives it. A commitment link names one occurrence and protects every record joined to it by
reference. So a kept record comes about in two ways, and the rule had stated one. The model
gained the event `commit`, three lives and two checked changes; a commitment tied where the unit
holds no covered record is refused as not modelled, and the spec lists it as the eighth thing the
model does not handle. The designing session added one life by hand, a person's report made
before the cure beside a covered record. It showed something the spec had not said: such a unit
closes on its covered record while the person's record stands, under either value of the
constant that holds decision 10. The spec now says so and says that whether it should is part of
that decision. 70 lives, 488 steps, 91 tests across the two files.

Station FC-8. Codex (astra) read the edit-time resolver, read-only; a Claude Haiku agent opened
all 54 of its citations (50 supported, 4 in part); a Claude scribe wrote the note,
`fc8-pointer-rule.md`; a fresh blind reader read it as intended and five of its points were
taken. The ledger case already had an advisory rule in `.claude/data/.epr-meta`, and one point
was added to it, in a commit of its own. The failure-list case cannot be written as a rule
today. The station stays open, with five missing nodes named and none proposed as work.
