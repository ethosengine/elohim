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
  - "feedback-as-a-lifecycle-item-design | the design whose stations FB-1 to FB-10 this plan restates as claimable | sha256:bb8216eb2f736464 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
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

State: not built. The companion's section 6 lists the pieces (terms, the promise a term bears on, a bound, a release's cures, a store's limits; one call; one reading; three acts) and proposes homes. The surveys found several pieces defined more than once: severity as three levels in `elohim/epr/src/algedonic.rs`, as error/warning/info in the Genesis adapter and as six standard names in brit's table; polarity in the SDK's enum and, under another meaning, on brit's rows; reach as eight rings, as five observation reaches and as four rungs.

Verify: one table in the sprint directory in which each primitive has exactly one defining home, cited by path, and every other use is listed as a use of it or as a difference to resolve. A reader who did not write it opens every cited path and finds what the row says.

- [ ] **FC-2 — the declaration a producer ships has one schema in the SDK, and brit's shipped table validates against it unchanged** (serves private-feedback-loop)

State: not built; the place its schema lives waits on the steward's ruling (companion section 12, decision 2: SDK schema now, or after a second producer has used the call). Until then the deliverable is a draft schema in the sprint directory. brit ships `{"claims": [...], "cures": [...]}` with twelve fields per claim (lifecycle spec section 5.4 and the seam of section 12). The SDK has two neighbouring shapes: `ClaimDeclaration` and `ObservationDeclaration` in `app-manifest.schema.json`, and `observation-kind.schema.json`. No schema describes brit's file. The P2P design gate for the declaration is recorded in the lifecycle spec's section 13: content-derived, part of the producer's own release, no DHT entry.

Verify: a draft schema file in the sprint directory accepts the `claims.json` of the brit release that carries it, byte for byte, and refuses a row with an unknown field, a severity or polarity outside its list, or a `validityHorizon` that is not an ISO 8601 duration. After the ruling, the same file under `elohim/sdk/schemas/v1/manifest/` with its test in the SDK's schema test step.

- [ ] **FC-3 — the one call that records an observation has one written interface and two carriers that agree field for field** (serves private-feedback-loop)

State: not built as an interface. Four doors exist. Three are verbs of `epr feedback` that write to the device's store: `capture`, which takes the term and the producer's address from two environment variables beside its older arguments; `ingest-genesis`, which takes a whole sprint report; and `record`, for authored evidence. The fourth is `POST /api/v1/observations`, which validates a payload against its declared kind. Nothing states that they are the same call or maps one to another.

Verify: one fixture expresses the same observation for each door; a test on each side reads the fixture; and the interface says, for every field, which door holds it where and which fields a door lacks.

- [ ] **FC-4 — the reading of a term and the summary that may leave a device are each one closed shape, shared by every consumer** (serves private-feedback-loop)

State: not built; like FC-2, where its schema lives waits on the steward's ruling (companion section 12, decision 2), and until then the deliverable is a draft in the sprint directory. The lifecycle spec gives nine readings with an order of precedence (section 6) and the summary's fields with their sources by state (section 5.5), in prose and tables. The SDK names the missing piece `ClaimInstance`.

Verify: the readings and the summary are a draft schema in the sprint directory, with the precedence as a table-driven test that any implementation can run, and the story `genesis/a2o/features/devflow/feedback-drains-when-cured.feature` uses only readings the schema lists.

- [ ] **FC-5 — each act on a term names the existing command or record that performs it and the authority it needs** (serves private-feedback-loop)

State: partly built. `epr feedback propose`, `group`, `evidence` and `reclaim` exist. `retire`, the ask, the answer and the release of one sample do not. The store refuses a link written without deliberate authorship.

Verify: a table, checked against the code, in which every act has one command or is marked not built, names the record kinds it writes, and names who may perform it. No act is listed as performable without an attributed actor.

- [ ] **FC-6 — a store's limits are one declared shape, and a native probe counts the stores no rule covers** (serves dev-system-equilibrium)

State: not built; waits on the steward's ruling (companion section 12, decision 1). One store keeps the full shape today: the private feedback store, under policy `feedback-local-retention@1`. The companion's section 7 lists the stores measured on shem and proposes the retirement gradient, whose short end is a new permission.

Verify: the shape is schema; the feedback store's existing rule validates against it; and `epr flow sense` reports a count of stores that no rule covers, with the feedback store read as covered.

- [ ] **FC-7 — what a producer must declare and call to be on the loop is one checklist, with brit as its worked example** (serves private-feedback-loop)

State: not built. The companion's section 4 gives each producer's first move in one line.

Verify: brit passes every line of the checklist against its landed release, and one other producer, an agent hook, is walked through it with no line that needs an interface the contract lacks.

- [ ] **FC-8 — a session that is about to create a log, an error list, a findings ledger or a store is pointed at the contract before it writes** (serves dev-system-equilibrium)

State: not built. This is how the contract reaches other sprints. The lightest existing means is a co-located `.epr-meta` rule that adds a pointer on the edit; a rule that refuses is not proposed.

Verify: an edit that adds a new append-only ledger under `.claude/data/`, or a new closed list of failure names in a first-party crate, shows the pointer once per session, and an edit that does neither does not.

- [ ] **FC-9 — the contract answers each line of the Observer Protocol's safeguards, or says why a line does not apply to tools** (serves private-feedback-loop)

State: not built. The Observer Protocol's Part VIII has three lists: what an observer cannot do, what it must do, and what the person controls. The companion's section 1b maps the feedback loop onto the Observer Protocol's shape and names three places the likeness stops. The lifecycle spec was written without reading that epic.

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

Verify: an occurrence from a release behind the device's head reads `stale-emitter`, and `cure-available` once the head's declaration lists it in `cures`.

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

Verify: a brit older than the hook's table reads as pin lag with its cure, a newer one as a stale table, and the hook's refusal and brit's leave one record in a session.

- [ ] **FB-8 — the acceptance suite's results are read against the habit register** (serves private-feedback-loop)

State: not built. The suite already calls the recorder when a sprint report is built; shem's store holds no record from it.

Verify: a concern failed in one report and passed in two later ones reads `on-watch`, then `retired` after its watch, and a report built on a device leaves records in that device's store.

- [ ] **FB-9 — a red or green CI stage is recorded under the stage its pipeline declares** (serves pain-is-answered)

State: not built; designed in one line (lifecycle spec section 12).

Verify: a stage red on three builds and green on the next two reads as one term felt three times, then on watch.

- [ ] **FB-10 — a red or green gate step is recorded under its gate project and step** (serves private-feedback-loop)

State: not built; designed in one line.

Verify: a gate red on two pushes and green after a landing reads `on-watch` and names the landing.

## Carried elsewhere

- Signed acts and admission by membership, which FB-4b needs: stations GS-6 and GS-7 of `genesis/docs/superpowers/plans/2026-11-26-governed-space-collective-memory-plan.md`.
- The boundary clause that states standing consent: station GS-4 of the same plan.
- The running peers' own steps toward observing themselves: the observability epic's section 7, which this plan does not station.
