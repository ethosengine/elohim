---
title: The feedback loop and its contract — the stations
id: feedback-loop-contract-plan
status: active
class: process-meta
process_subdomain: feedback
date: 2026-11-27
steward: human:matthew
author: "Claude (session 3d494616 on shem, Fable 5.2), at the steward's direction of 2026-11-27"
serves: [private-feedback-loop, workspace-device-arrival, pain-is-answered, dev-system-equilibrium]
refines: native-dev-tooling-over-peers-plan
cites:
  - "feedback-as-a-lifecycle-item-design | the design whose stations FB-1 to FB-10 this plan restates as claimable | sha256:6af9042774cc0d7a | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose contract and store rule the FC stations fix as shapes | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "native-dev-tooling-over-peers-plan | the epic's plan this is a child of; its peer link, channel list and bin store are what this work rides | sha256:983afa13e08fe1d3 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - "observer-protocol | source of the safeguards station FC-9 answers line by line | sha256:78afb76e1fb58772 | path: genesis/docs/content/elohim-protocol/observer-protocol.md"
---

# The feedback loop and its contract — the stations

Two designs describe their work in prose: the lifecycle spec
(`genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md`) and its
companion (`genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md`).
`epr flow project` mints from `- [ ]` stations in documents under `plans/` and `specs/` and reads
no prose, so nothing in either was claimable. This file restates their stations as claimable ones
and adds one group the designs only sketch: the contract, which the steward asked for first.

It is a child of [the native developer tooling plan](epr:native-dev-tooling-over-peers-plan). The
peer link, the channel's list and the bin store that this work rides are that epic's.

Three things to know before claiming a station:

- **The contract comes first.** The steward, 2026-11-27, as typed: "let's start a sprint making sure we've got the right primitives/interfaces/compositions to take this spec into it's contract so we can help drive consistency through other sprints.." The FC stations are that sprint. An FB station that builds against an interface the FC stations have not yet fixed says so in its State line.
- **Both designs are Drafts.** Each lists decisions that are the steward's, with a working posture on each. A station whose State line says it waits on a ruling is not claimable until that ruling exists.
- **Nothing here was re-measured.** Each State line repeats a design or the survey behind it, as of 2026-11-27, in a worktree at dev `e4fe04965`.

Gap ids are positional. New stations are appended at the end; stations are ticked, not reordered
or removed. A tick is a claim that a line is met, and still awaits verification.

## Words used

- A **producer** is anything that can report that something went wrong or right: a tool, a hook, a gate, a pipeline, a test suite, a running peer, an app.
- A **term** is a declared name for one kind of observation, from a closed list the producer ships. brit's `claims.json` calls it a claim id.
- The **recorder** is the device's private feedback store and the three `epr feedback` verbs that write to it: `capture`, `ingest-genesis` and `record`.
- The **judge** is the read-only code that gives each term on a device one **reading** (open, cure available, on watch, retired and so on).
- **brit** is this repository's command-line tool for version control. Its feedback work is on brit branch `run-ci/feedback-f1` and has not landed on brit main.
- The steward's three devices are shem, ethosengine and thinkpad.

## Stations: the contract

- [ ] **FC-1 — every primitive the contract uses names the one existing type or schema that defines it, or is marked new** (serves private-feedback-loop)

State: the table is written, on its branch, as `../sprints/2026-11-27-feedback-contract/fc1-primitives-inventory.md`: 38 primitives, each with the home that defines it today or marked new. It was read from the code by one agent, and a second agent that did not write it opened every citation and found no mismatch. It does not yet give each primitive exactly one home: where definitions disagree it lists them all and states the difference, and choosing is left to FC-2 to FC-5 and, for some rows, to the steward. Three pieces have more than one list of values: reach or ceiling (eight rings, five observation scopes, two ceilings in brit's table, four rungs in the lifecycle spec), severity (three levels in `elohim/epr/src/algedonic.rs`, error/warning/info in the Genesis adapter, six standard names in brit's table) and polarity (one list of two values under two meanings). Six are used by the designs and defined by no code or schema: the reading of a term, the summary that may leave a device, the grant a service acts under, the retire act, and the ask, answer and release of a sample. The station stays open until the table is on dev.

Verify: one table in the sprint directory in which each primitive has exactly one defining home, cited by path, and every other use is listed as a use of it or as a difference to resolve. A reader who did not write it opens every cited path and finds what the row says.

- [ ] **FC-2 — the declaration a producer ships has one schema in the SDK, and brit's shipped table validates against it unchanged** (serves private-feedback-loop)

State: drafted, on its branch; the place its schema lives waits on the steward's ruling (companion section 12, decision 2: SDK schema now, or after a second producer has used the call). The draft and its fixtures are in the sprint directory (`../sprints/2026-11-27-feedback-contract/fc2-declaration-draft.md` and `fc2/`). It accepts, unchanged, the `claims --json` output of a brit 0.3.10 built on shem from brit's branch at `a99143327` (72 claims, no cures), and refuses thirty-two fixtures, each wrong in one way. Its `cures` entry follows the cumulative five-field shape brit fixed for 0.3.12 (`claim`, `release`, `state`, `cite`, `summary`), which no brit release writes yet. Those checks are a test in the orchestrator's suite, `genesis/orchestrator/producer-declaration-schema.test.mjs`, so a gate runs them at every landing that touches the test or the fixtures; the test moves with the schema when its home is ruled. Not yet true of the first check below: no brit release archive carries the file, so "the release that carries it" was a local build. brit ships `{"claims": [...], "cures": [...]}` with twelve fields per claim (lifecycle spec section 5.4 and the seam of section 12). The SDK has two neighbouring shapes: `ClaimDeclaration` and `ObservationDeclaration` in `app-manifest.schema.json`, and `observation-kind.schema.json`. No schema describes brit's file. The P2P design gate for the declaration is recorded in the lifecycle spec's section 13: content-derived, part of the producer's own release, no DHT entry.

Verify: a draft schema file in the sprint directory accepts the `claims.json` of the brit release that carries it, byte for byte, and refuses a row with an unknown field, a severity or polarity outside its list, or a `validityHorizon` that is not an ISO 8601 duration. After the ruling, the same file under `elohim/sdk/schemas/v1/manifest/` with its test in the SDK's schema test step.

- [ ] **FC-3 — the one call that records an observation has one written interface and two carriers that agree field for field** (serves private-feedback-loop)

State: the table is written, on its branch, as `../sprints/2026-11-27-feedback-contract/fc3-one-call-four-doors.md`, and it finds the condition below cannot be met today. No field is carried by all four doors in one form. The time of an observation is the only field all four carry with one meaning, as an RFC 3339 string in the three `epr feedback` doors and as epoch seconds in the storage endpoint. All four have a subject, and it is four different things: a claim id, a producer and operation, a finding's fingerprint, a content address. Producer, outcome, run, item, count and duration are carried by the three `epr feedback` doors and absent from the storage endpoint; kind, payload and observer are the endpoint's alone. `run`, `subject`, `kind` and the Genesis adapter's `detail.source` each mean different things by door. So one fixture cannot express one observation for all four without dropping or translating fields, and no interface is written. The table records and proposes no mapping. Four doors exist. Three are verbs of `epr feedback` that write to the device's store: `capture`, which takes the term and the producer's address from two environment variables beside its older arguments; `ingest-genesis`, which takes a whole sprint report; and `record`, for authored evidence. The fourth is `POST /api/v1/observations`, which validates a payload against its declared kind. Nothing states that they are the same call or maps one to another.

Verify: one fixture expresses the same observation for each door; a test on each side reads the fixture; and the interface says, for every field, which door holds it where and which fields a door lacks.

- [ ] **FC-4 — the reading of a term and the summary that may leave a device are each one closed shape, shared by every consumer** (serves private-feedback-loop)

State: drafted, on its branch, as `../sprints/2026-11-27-feedback-contract/fc4-reading-and-summary-draft.md` and `fc4/`: two draft schemas, the mapping from reading to summary state and to the words a person is shown, sixteen facts, 73 cases, 132 scripted lives of one claim (sixty-nine of them written by the integrator's reader to break the model or ported from its lives, and thirty-five of them twins that show the other outcome of an undecided question) and forty-eight fixtures, checked by `genesis/orchestrator/reading-and-summary-schema.test.mjs`, which runs every combination of the facts and requires each to give exactly one reading, and runs each life through a small model, checking the reading after every event. Getting there corrected the design five times over, the last time by rewriting the spec's section 6 from numbered rules, now eighteen, that the model runs; the reader's attack on the model then found four more faults in one pass, which changed four rules and added one. The first draft could not run the order of readings in the lifecycle spec's section 6 as written, because a cure still present after compaction read `cure-available` ahead of `on-watch` and `retired`; sections 5.5, 6 and 7 were amended. The integrator's reader then found that a claim retired by a person's ruling still read `on-watch`, and that `reopened` had no meaning where the cure was a condition and not a release; both are corrected and have cases. The same reader then found a state with no reading at all, a cure present and nothing left for compaction to replace; `open` is now what remains when no other reading holds. The gap those lives first pinned, a device that skips the release that carried a fix, went to the brit lane, which chose a cumulative `cures` list for brit 0.3.12; the model reads it, and a fix can now be withdrawn. One behaviour is held behind a named switch because it is the steward's to decide (the lifecycle spec's section 16, decision 10): whether a machine-checked recovery may close a claim while a person's own report of its return still stands. No judge and no summary exists in code. Like FC-2, where its schema lives waits on the steward's ruling (companion section 12, decision 2), and until then the deliverable is a draft in the sprint directory. The lifecycle spec gives nine readings with an order of precedence (section 6) and the summary's fields with their sources by state (section 5.5), in prose and tables. The SDK names the missing piece `ClaimInstance`.

Verify: the readings and the summary are a draft schema in the sprint directory, with the precedence as a table-driven test that any implementation can run, and the story `genesis/a2o/features/devflow/feedback-drains-when-cured.feature` uses only readings the schema lists.

- [ ] **FC-5 — each act on a term names the existing command or record that performs it and the authority it needs** (serves private-feedback-loop)

State: the table is written, on its branch, as `../sprints/2026-11-27-feedback-contract/fc5-acts-and-authority.md`, and it finds the condition below unmet. `epr feedback` dispatches eighteen verbs. Of the eighteen acts that write, nine require no named actor: capture, report ingestion, authored records, the catch-up command, commitment linking, repair, reclaim, migrate and qualify; and an ordinary append can start a reclaim. Where an actor is required (group, the evidence links, propose), the check is that a string was given: nothing in the feedback code calls the function that refuses a `service:` name in an acting position. Eleven acts the designs name have no command, among them retire, the ask, the answer and the release of one sample; a twelfth row, widening a ceiling, is an act the design forbids. The table records and proposes nothing; which of the nine should name an actor, and what an actor check should be, is not decided here. One of the nine is this plan's own slice: FB-1's capture leaves the participant to the repository's policy until the read of its missing node exists.

Verify: a table, checked against the code, in which every act has one command or is marked not built, names the record kinds it writes, and names who may perform it. No act is listed as performable without an attributed actor.

- [ ] **FC-6 — a store's limits are one declared shape, and a native probe counts the stores no rule covers** (serves dev-system-equilibrium)

State: not built; waits on the steward's ruling (companion section 12, decision 1). One store keeps the full shape today: the private feedback store, under policy `feedback-local-retention@1`. The companion's section 7 lists the stores measured on shem and proposes the retirement gradient, whose short end is a new permission.

Verify: the shape is schema; the feedback store's existing rule validates against it; and `epr flow sense` reports a count of stores that no rule covers, with the feedback store read as covered.

- [ ] **FC-7 — what a producer must declare and call to be on the loop is one checklist, with brit as its worked example** (serves private-feedback-loop)

State: the checklist is written, on its branch, as `../sprints/2026-11-27-feedback-contract/fc7-producer-checklist.md`, and the condition below is not met. It has 22 lines: four things a producer cannot do, sixteen it must do, two it leaves to the person. Walked through it at its branch tip `a99143327`, brit passes fourteen, four do not apply, one cannot be settled because no release carrying the declaration has been published, and three differ: its repeat marker leaves the outcome out of its key for some claims (latent: no call site triggers it today), a full spool is silent, and its marker directory has no bound. A fourth line first read as brit's failure was this design's mistake, a wrong directory name in the lifecycle spec, now corrected. The agent hook `.claude/hooks/brit-first.py`, which is not on the loop, cannot meet three lines because the contract has no place for a producer that is a script: who admits it, what its emitter is, and a gap-note path that names only brit. Ten things the checklist asks are checked by nothing. The companion's section 4 gives each producer's first move in one line.

Verify: brit passes every line of the checklist against its landed release, and one other producer, an agent hook, is walked through it with no line that needs an interface the contract lacks.

- [ ] **FC-8 — a session that is about to create a log, an error list, a findings ledger or a store is pointed at the contract before it writes** (serves dev-system-equilibrium)

State: partly built, on its branch, and the condition below is not met; the finding is `../sprints/2026-11-27-feedback-contract/fc8-pointer-rule.md`. This is how the contract reaches other sprints. The lightest existing means is a co-located `.epr-meta` rule that adds a pointer on the edit; a rule that refuses is not proposed. The ledger case had such a rule already: `.claude/data/.epr-meta` advises a session that writes a new `*.jsonl` file there, and one point was added to it that names the producer checklist of FC-7 and the lifecycle spec. It is shown for each new ledger and not once per session, and only when a Write tool call makes the file: a ledger born by a script's first append is seen by no hook. The failure-list case cannot be written as a rule today. The matcher has a file's name, whether the file is new, and literal substrings; it cannot tell a list an edit adds from one the file already had, and no one manifest reaches every first-party crate. The note names five missing nodes and proposes none as work. Its lighter reading is that a list of failure names is found by a producer's declaration (FC-2) and not by reading source.

Verify: an edit that adds a new append-only ledger under `.claude/data/`, or a new closed list of failure names in a first-party crate, shows the pointer once per session, and an edit that does neither does not.

- [ ] **FC-9 — the contract answers each line of the Observer Protocol's safeguards, or says why a line does not apply to tools** (serves private-feedback-loop)

State: the table is written, on its branch, as `../sprints/2026-11-27-feedback-contract/fc9-observer-safeguards.md`, and it finds the condition below unmet. Part VIII has eighteen lines. Four are answered by the designs and partly built; six are answered on paper and not built; one, physical privacy switches, does not apply to a tool; and seven have no answer in either design: enabling stalking or harassment, preserving evidence only with consent or emergency, cryptographic proof of operation, when observation happens, how patterns get used, access to one's own data, and deletion on request. On the last, the lifecycle spec says the opposite of silence: a record a person wrote, a record tied to a commitment and protected evidence are kept. The table records the seven and answers none. Answering them is design work not yet done, and where an answer decides what a person is owed it is the steward's. The Observer Protocol's Part VIII has three lists: what an observer cannot do, what it must do, and what the person controls. The companion's section 1b maps the feedback loop onto the Observer Protocol's shape and names three places the likeness stops. The lifecycle spec was written without reading that epic.

Verify: one table with a row for each line of Part VIII, in which the feedback loop's answer cites the section of the lifecycle spec that holds it, or the row says the line does not apply and why. The checklist of FC-7 has the same three columns.

- [ ] **FC-10 — each of the five doors onto this ground names the others in one line** (serves private-feedback-loop)

State: four of five built, on the steward's word of 2026-11-27, as typed: "yeah, let's make sure we get those pointers.. we'd want those edges to surface if an agent returns and is graph walking, or there's an embedder that could benefit from the relationship..." The Observer Protocol and the observability epic each name the lifecycle spec and the companion; the seam map routes a tool's refusal, a recurring error and a growing log in its section 3.16, its routing table and its crosswalk. Each edit was a sentence or a table row, and the cites this work made stale were refreshed, on the ground that the only change to each target was an added pointer: six specs and two research documents (`genesis/research/commons-composition-at-scale-2026-10-15.md` and `commons-facility-at-scale-2026-10-15.md`) whose cite of the seam map had been fresh, and this work's own documents. Cites that were already stale before these edits were left as they were. Not built: the SDK pipeline (`elohim/sdk/README.md`) names none of the three. It is outside the documentation tree, so its pointer lands with FC-2.

Verify: a reader who starts at any one of the five (the Observer Protocol, the observability epic, the seam map's section 3.16, the SDK pipeline, the lifecycle spec) reaches each of the others within two links, and `epr flow cites verify` reports no stale cite among the documents that cite the edited ones.

## Stations: the loop on a device

- [ ] **FB-1 — capture records an observation under its declared term once per session, in the session's participant's name, and device status says when the recorder is down** (serves private-feedback-loop, workspace-device-arrival)

State: partly built, on branch `feedback/fb1` (2026-11-27), not landed. Built, inside `epr-cli` and with no record shape changed: capture by declared term from the environment, the session as the run, producers admitted by the policy, `epr feedback drain-spool`, and a `feedback recorder` line in device status. Not built: the participant. No read answers "for whom does this act" without the session store's lock, and the recorder has two seconds; the design names that read as a missing node (section 7a, node 1) and the carrier design on ethosengine proposes it once for sessions and for services under a grant. So the station stays open. On shem on 2026-11-27, 1,764 of 1,772 occurrences carried no participant and 1,771 runs held 1,772 occurrences. brit's side is built on `run-ci/feedback-f1`.

Verify: twenty captures of one term in one session leave one record per producer, term, outcome and emitter (scenario "A refused command names its problem, and one session leaves one record"); a device with gap notes waiting reads `degraded` in `just status device` with its cure, and reads well after the catch-up command (scenario "A device that could not keep its records says so, and says how to catch up"); an older epr still reads the store. Still to verify, with the participant: a failing command in an attributed session followed by a passing one in a later session yields a recovery link.

- [ ] **FB-2 — the device reads the declarations of the producers it runs and gives each term one reading** (serves private-feedback-loop)

State: not built; the shapes are FC-2 and FC-4.

Verify: an occurrence from a release behind the device's head reads `stale-emitter`; once the store holds a later observation from the head release it reads `cure-available`, if the head's declaration lists the claim in `cures`.

- [ ] **FB-3 — a term whose burden crosses its declared bound is told to the device's steward at session start** (serves private-feedback-loop)

State: not built. The measure plane, bounds and the session headline exist; no measure reads a term's burden.

Verify: a term past its hard mark appears in `epr flow report --breaches` with its concern, and an observation read `stale-emitter` is not counted.

- [ ] **FB-4a — a summary of a term is carried between the steward's own devices** (serves private-feedback-loop)

State: not built; waits on the steward's word (lifecycle spec section 16, decision 2). The channel's list today says no other measure crosses.

Verify: a term open on one device is counted on a second, and the record that carried it holds no session, no person and nothing typed.

- [ ] **FB-4b — a steward of a producer may ask who feels a term, and a household answers only what it chose to** (serves pain-is-answered)

State: not built; waits on the governed-space plan's stations for signed acts and admission by membership, and on the steward's ruling (lifecycle spec section 16, decisions 5 and 6).

Verify: the scenarios of the story that name "adam" pass in a fixture household.

- [ ] **FB-5 — a release that cites a term lets the device drain that term's records to a citation and a sentence** (serves private-feedback-loop)

State: not built; waits on the steward's ruling on a second version of the retention policy (lifecycle spec section 16, decision 1).

Verify: after the curing release lands and a session retires the term, its records are one aggregate, one interpretation and one link, and the dropped count matches.

- [ ] **FB-6 — a term proven gone leaves one note that names the release that fixed it, and a recurrence reopens it** (serves private-feedback-loop)

State: not built; depends on FB-5.

Verify: the note survives a reclaim; a recurrence from the curing release reads `reopened` and names that release.

- [ ] **FB-7 — the agent-side brit hook reads brit's declaration and keeps no list of its own** (serves private-feedback-loop)

State: not built. `.claude/hooks/brit-first.py` holds its own table and reads any brit but two versions as a stale table.

Verify: a brit older than the hook's table reads as pin lag with its cure, a newer one as a stale table, and the hook's refusal and brit's leave one record each in a session, under one shared run (the store identifies a source by producer, run and item, so two producers are two records).

- [ ] **FB-8 — the acceptance suite's results are read against the habit register** (serves private-feedback-loop)

State: not built. The suite already calls the recorder when a sprint report is built; shem's store holds no record from it.

Verify: a concern failed in one report and passed in two later ones reads `cure-available`, then `on-watch` once a session has retired it, then `retired` after its watch, and a report built on a device leaves records in that device's store.

- [ ] **FB-9 — a red or green CI stage is recorded under the stage its pipeline declares** (serves pain-is-answered)

State: not built; designed in one line (lifecycle spec section 12).

Verify: a stage red on three builds and green on the next two reads as one term felt three times, then on watch.

- [ ] **FB-10 — a red or green gate step is recorded under its gate project and step** (serves private-feedback-loop)

State: not built; designed in one line.

Verify: a gate red on two pushes and green after a landing reads `cure-available` and names the landing, and `on-watch` once a session has retired it.

## Carried elsewhere

- Signed acts and admission by membership, which FB-4b needs: stations GS-6 and GS-7 of `genesis/docs/superpowers/plans/2026-11-26-governed-space-collective-memory-plan.md`.
- The boundary clause that states standing consent: station GS-4 of the same plan.
- The running peers' own steps toward observing themselves: the observability epic's section 7, which this plan does not station.
