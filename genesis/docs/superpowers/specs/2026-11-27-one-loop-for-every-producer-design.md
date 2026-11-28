---
title: "One loop for every producer — how logs, exceptions, observations and telemetry close the same loop"
id: one-loop-for-every-producer-design
status: Draft
class: process-meta
process_subdomain: feedback
date: 2026-11-27
steward: human:matthew
context-tier: disclosed
topic:
  - feedback
  - observability
  - logging
  - telemetry
  - retention
  - sdk
  - viable-system
refines: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md
derived_from:
  - genesis/docs/content/elohim-protocol/observability/epic.md
  - genesis/plans/2026-03-28-feedback-information-flows-design.md
serves: [private-feedback-loop, pain-is-answered, dev-system-equilibrium]
graduation-trigger: >-
  a second producer besides brit records under a declared name through the same call, and one
  store that had no declared limit gains one with a probe that reads it
cites:
  - "feedback-as-a-lifecycle-item-design | the device-plane design this pass generalises: a declared name per problem, a judge, a drain on cure, and a summary that leaves by consent | sha256:13ae49af0eded2ca | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "observability-epic | the governing vision for a running network: self-sense, witness and fruit kept apart, one cybernetic loop, and fruit and a count as the only things that cross a rung | sha256:b193d1cff3a60988 | path: genesis/docs/content/elohim-protocol/observability/epic.md"
  - "private-thought-governed-fruit | source of the rule that the aggregator is the dragnet and that no boundary rests on a count | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "algedonic-feedback-signal | the typed pain signal a breach rides, and the rule that pain is not filtered or priced down | sha256:cf902beac8dd4aec | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - "sense-respond-governance-classifier | source of the rules that no aggregate changes a rule and that a positive signal is a candidate | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - "requisite-variety-guidestar-epr-family-composition | the test for admitting a primitive only on a second independent need, and the reading of the elohim as an attenuator that asks | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
  - "governed-space-share-boundary-collective-memory-design | the rungs a record sits at and the one-definition-two-carriers posture reused for the contract | sha256:e684da3e7abc4153 | path: genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md"
  - "observation-event-layer-design | the application plane's declared observation kinds and their graduation, the second carrier of the one call | sha256:002fc7ec733dd10a | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - genesis/plans/2026-03-28-feedback-information-flows-design.md
  - genesis/research/elohim-as-viable-system-2026-06-04.md
  - genesis/research/beer-designing-freedom-elohim-critique-2026-06-04.md
  - genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md
  - elohim/sdk/README.md
  - "observer-protocol | the root of the witness shape this method shares: raw capture processed where it was made and destroyed, a four-level response to harm, and the three lists of safeguards | sha256:78afb76e1fb58772 | path: genesis/docs/content/elohim-protocol/observer-protocol.md"
---

# One loop for every producer

In brief: every part of this system that can fail already says so somewhere: a log line, an error,
a signed observation, a number. Almost none of it comes back as a fix that the one who felt it can
see, and almost none of it is ever cleaned up. This document asks one question of each producer:
what does it log, throw, observe and measure today, and where does that go? It then names the
handful of patterns that already recur across the repository, the small contract a producer
implements to join the loop, and what the network gains when local loops close. It designs
nothing new. Each pattern is listed with the places it already exists.

Status: a survey and a reading, with proposals marked as such. Nothing here is built by it.

## 0. Provenance

The steward's words (`human:matthew`, 2026-11-27, as typed). They follow the nineteen statements
recorded in the lifecycle spec.

1. "We should run a pass over this, thinking about primitives to each of these consumers .... so see how to elevate logging/exception/observation and telemetry feedback.. into this distributed epr-vsm method of closing the loop on if that could provide reusable patterns everywhere that enhance the whole effectiveness and efficiency of not just individual tools, but aggregates into the maintenance, cooperation, and resiliency of the whole network.. ."
2. "if there's part of the API/SDK emerging here.."
3. "hey.. one thing to note.. is log stores should be forced to declare their carrying capacity/limits.. logs are something that tend to run-away.. kind of like an email inbox.. if we could create logs and stores that intelligently retire, so logs and stors are born with retention mishpats, that's a deterministic floor, so we don't create a new dump anywhere"
4. "if we have different retire-when policies for the feedback produced (shorter for info, longer for fatal/human feedback (a gradient)... so there are long tails of opportunities for things to be resolved.. that's a reusable pattern everywher."
5. "another thing a device/runtime/peer could do is review their own logs.. resolve them into those summaries themeselves.. those summaries should be able to easily "gradutate" into avodah workitems.. so a helpful developer COULD theoretically havest the whole epr as story w/documentation.. already curated.."
6. "those emergent summaries could enable parent-child relationships from summaries to logs.. that could help aggregate/curate logs into eprs.. anyway just another place where the intelligence distribted in the fabric could help.. in isolation, or then at the collective level, whatever makes sense for the shape..."
7. "maybe not parent-child, but to think of them as the first fractal layers of the vsm graph system."
8. "oh.. one last thought to worry about.. this all also sounds to me like the "witness" shape of the elohim observer epic.. they might actually be related concerns.."
9. "we might want to understand how they relate before we finalize and move on from anything.."
10. "Yes' let's make sure we understand the relation, where the seams are, because they're not exactly the same, but they might benefit from reusing eachohter's infrastrcuture to avoid bespoke duplications...  in the mental model of things.. this is us re-discovering the observer epic from the perspctive of logs/telementry/observations we need as developers... and not as visionaries, it's a good place to make sure we've got this.. so it reconciles and is discoverable as people approach the elohim protocol from different perspctives.. who knows when we might converge on the same concerns again from another direction."

It was read once by a context-isolated reader, who found no contradiction and seven places a
newcomer could not follow; those were repaired and not read again.

The rest is an agent seat's reading (Claude, Fable 5.2, session 3d494616 on shem). Its facts come
from five read-only surveys run for it on 2026-11-27, of the build plane, the deployed runtime, the
client apps, the device and its agents, and the repository's own writing on viable systems. The
author read the SDK pipeline, the approved feedback design of 2026-03-28, the observability epic's
sections 1, 4, 6 and 7, and measured the stores of section 7 on this device. Other figures are as
the surveys reported them and were not re-read.

## 1. Where this sits

Five documents already describe the loop, each for one plane. This one adds no sixth description.
It shows they are the same loop and gives each producer its row.

| Document | Plane | What it gives |
|---|---|---|
| The SDK pipeline (`elohim/sdk/README.md`) | Any domain | Seven verbs, CLAIM to RENEW, and the rule that a domain declares claims with a validity horizon and observations with at least one negative |
| Feedback as information flow (approved 2026-03-28) | Content | Three layers: claims, observations, and obligations that accumulate when negative observations shorten a claim's validity |
| The Observer Protocol (`genesis/docs/content/elohim-protocol/observer-protocol.md`) | People in the places they live and work | Witness in place of surveillance: raw capture is processed where it was made and destroyed, and "only structured story remains"; a four-level response to harm; three lists of what an observer cannot do, must do, and what the person controls |
| The observability epic, "Sensed, Never Gathered" | The running network | Three records that stay apart (self-sense, witness, fruit), one cybernetic loop, and what may cross each rung |
| The lifecycle spec (2026-11-26) | A person's device | A declared name per kind of problem, a judge, a drain on cure, and a summary that leaves by consent |

The SDK names three gaps in itself. Two are what the lifecycle spec builds, on the device first:
"CLAIM has no runtime lifecycle", and "RENEW has no explicit cycle. … The loop closure happens
'somehow' — it needs to be a first-class protocol event."

## 1b. Three witnesses, one shape

Statement 8 is right. Statement 10 says how to hold it: this work is the Observer Protocol reached
again, from the side of a developer who needs logs, errors and measures, where the epic was
written from the vision. The observability epic already calls itself "the same witness, turned
inward" on the network. The feedback loop is that witness turned on tools. They are three
arrivals at one shape, and they are not the same thing. This section says where they agree, where
the seams are, and what each could use of the others' so that nothing is built three times.

### The three, side by side

| | The Observer Protocol | The observability epic | The feedback loop |
|---|---|---|---|
| Reached from | The vision of being seen without being surveilled | Running a fleet | A developer's logs and errors |
| What is observed | People, in the places they live and work | Peers, hubs and the network's health | A tool, as a person meets it |
| The sensor | A camera or microphone | A peer's own gauges; a sibling's probe | The tool's typed errors; a hook; a gate; a test run |
| Who observes | The household's elohim, on the household's hardware | The peer itself (self-sense) and its siblings (witness) | The device that ran the tool. The tool only reports |
| Raw material | Frames, held for seconds | Series and log lines | Log lines and one record per command |
| What remains | "Only structured story remains": REA story elements | A signed row under a declared kind; past a threshold, one attestation | An occurrence under a declared term; past a fix, a citation and a note |
| Who controls it | The person: a physical switch, and consent to share | The holon: its declared kinds and their reach | The house: a clause, under a ceiling the producer cannot raise |
| What may leave | A story, by consent | "Fruit and a count, never rows" | A person's own report; a breach, to its target |
| How it escalates | Four levels, the person in control through the third | Approach and breach; a table of what each rung is asked to do | The reading, the ask, the release of one sample, the breach |
| State today | A vision; this pass looked for no implementation of it | A log with one producer and an evaluator nothing calls | One store and one producer, brit |

### What they share

- **See, understand, forget.** The Observer Protocol holds a frame "long enough for Elohim to understand the context, too short for others to extract and surveil". The epic makes a peer's gauges self-sense, "evidence to nobody else". The feedback store refuses to capture narrative automatically. In all three the raw material is for understanding where it was made, and what is kept is small and structured.
- **Local first.** "Process locally on user-controlled hardware"; a peer senses itself; the recorder and judge run on the device that felt it.
- **The person holds the door.** A switch, a declared reach, a clause.
- **Levels.** The Observer Protocol reads patterns "across constitutional layers", individual to family to community and outward. The epic has four rungs. Section 6a runs one loop through them.
- **The observer accounts for itself.** "Maintain cryptographic proof of operation"; a count of what went unobserved; the recorder's own absence as a declared term.

### Where the seams are

1. **Whom it watches.** The Observer Protocol watches people. The other two watch machines on people's behalf. Its safeguards guard a person's dignity in their own home; a tool's records risk leaking what someone was working on. The remedy has the same shape and the stakes differ. Its fourth level, the emergency in which "the victim maintains control except in Level 4 emergencies", is about imminent danger to a person and belongs to that plane only. A tool's breached limit reaches its target without waiting, and that is the whole of the likeness. No tool's severity opens a house.
2. **Open or declared.** The household's elohim recognises patterns in anything it sees and writes a story. The observability plane admits only kinds a manifest declared. The feedback loop admits only declared terms and counts the rest as one unclassified pile. The device's own interpretation (section 6a) is where the feedback loop comes nearest the Observer: an elohim reading raw material and writing what it means.
3. **Who the witness is.** A household's observer is a third party to the people it sees. A sibling peer is a third party to the peer it probes. A tool reporting its own refusal is no third party. The lifecycle spec's rule that a fix is believed only when a later session sees it pass is the witness discipline applied anyway. Whether the device counts as the observer and the tool as the observed is station FC-1's question.
4. **Where the record lives.** A witness row lives on a signed log in the storage peer. A feedback occurrence lives in an unsigned private file on a development device, which may run no peer. This is the largest infrastructure seam: one shape, two stores.
5. **How long things are kept.** Seconds, then destroyed; a retention class per kind; a governed rule with protected classes. Three settings of one dial (section 7).
6. **How far things go.** Constitutional layers; five reaches on the observation ladder; eight rings of content reach; four rungs. The epic lists the first three as unreconciled; the lifecycle spec added the fourth as a word for the same thing.
7. **What it is for.** The Observer Protocol turns what it witnesses into recognised value: care seen and credited. The epic yields attestations that placement and trust read. The feedback loop yields a fix and a clean store, and leaves value to mishpat.

### What each could use of the others'

The first four rows are built and have one user or none. The rest are designed in one place and
needed in the others.

| Piece | Where it exists | Used today by | Who else needs it |
|---|---|---|---|
| A declared kind with a retention class, a reach, a diversity threshold and a graduation | `observation_kinds` and its schema | Eight kinds, one with a producer | A tool's terms on the application plane; the Observer's story elements |
| A signed log per observer | `elohim-storage/src/observation` | One producer | Feedback, on any device that runs a peer |
| Graduation past a diversity threshold | `elohim-storage/src/graduation`, with no caller | Nothing | Fruit from several people's own reports; the Observer's patterns across layers |
| The pain signal, its latch and its recovery | The `elohim-epr` crate | The feedback store's capacity; the dialogue-stall sensor | Every producer's breach; a running peer's |
| A claim's runtime life: a reading, a decay, a reopening | The lifecycle spec; not built | | The SDK's content claims (its gaps 1 and 3); a peer's health promises; a story that should retire |
| Closed, measured and open fields, with the declaration as the schema of what may leave | The lifecycle spec; brit | brit | Observation kinds, whose `String` fields are open; any story the Observer shares |
| A store born with its limits, retiring on a gradient | The feedback retention rule; retention classes | One store | Every store of section 7. The Observer's seconds are its far end |
| Consent as a recorded clause in a person's words | The covenant's `variesRule`, built; the governed-space boundary clause, proposed | Agent sessions | The Observer's "what stories get shared"; the house's clause for feedback |
| A response in levels with the person in control | The Observer Protocol | | The feedback loop's exits and the epic's rungs are its first three levels under other names |
| A mark that says fixture or real | Habit `evidence-class-descends`, unwired | | All three |
| The elohim as observer, discerner and narrator | The SDK's three postures | Learning | The judge, the interpretation, and the three audiences of a report |

Two directions of reuse matter most, and neither is decided here.

- **One store where there is a peer.** On a device that runs a storage peer, a feedback occurrence is an observation row of a declared kind and should be written as one. A development device with no peer keeps the private file. That is the governed-space posture again: one definition, two carriers.
- **One claim lifecycle for all three.** The reading and the decay were designed here for tools. Nothing in them is about tools. A peer's health promise and a household story that should retire have the same life.

### What this changes here

- **Raw records are ephemeral by default.** The Observer Protocol makes it something an observer must do: "Destroy visual data after story extraction." For logs it is the direction section 7's gradient should lean: a raw line that has been interpreted has done its work.
- **Part VIII is the checklist.** Its three lists, what an observer cannot do, what it must do and what the person controls, are the three columns a producer's conformance should answer. The plan's station FC-9 carries this.
- **The emergency was already written.** The lifecycle spec had said entry without consent was "a question for governance and is not designed here". It now cites the Observer Protocol's rule for its own plane and builds no such path.

### Doors

Someone will reach this ground again from another side, on foot or by walking the cite graph, so
each door names the others in one line. The steward, 2026-11-27, as typed: "yeah, let's make sure we get those pointers.. we'd want those edges to surface if an agent returns and is graph walking, or there's an embedder that could benefit from the relationship..."
Four of the five doors carry their pointers as of this document's landing; the
fifth is the remainder of station FC-10.

| Arriving with | Read first | It points to the others |
|---|---|---|
| The vision of a witnessed household | The Observer Protocol | Yes: to the observability epic, and to the lifecycle spec and this document |
| A fleet to run; "is this peer healthy" | The observability epic | Yes: to the Observer Protocol, and to the lifecycle spec and this document |
| A log, an error list, a metric, a findings ledger | The lifecycle spec and this document | Yes, here |
| "Where does this concern live" | The seam map: section 3.16, a row in its routing table, and a row in its crosswalk for logging and error tracking | Yes |
| A domain to build | The SDK pipeline (`elohim/sdk/README.md`) | Not yet. It sits outside the documentation tree, and its pointer lands with station FC-2, which edits the SDK anyway |

## 1a. Terms

- **Producer**: anything that can report that something went wrong or right: a tool, a hook, a gate, a pipeline, a test suite, a running peer, an app.
- **brit**: this repository's command-line tool for version control, used in place of Git. It is the one producer that already records under declared names.
- **The recorder**: the device's private feedback store and the three `epr feedback` verbs that write to it: `capture` for one occurrence from a tool, `ingest-genesis` for a whole sprint report from the acceptance suite, and `record` for evidence a person or agent authors. "Calls it" in section 4 means a producer runs one of them. Section 6 describes the call.
- **A person's elohim**: the protocol's local agent that works for one person or household. Written with the article and in lower case, it means that agent and not this repository or its paths.
- **FB-1 to FB-10**: the stations of the lifecycle spec's section 12, each one buildable capability. Those named here: FB-1, capture records under a declared name once per session, and device status shows when the recorder is down; FB-2, the device reads declarations and gives each a reading; FB-8, the acceptance suite's results are read against the habit register; FB-9, a red or green CI stage is recorded under its declared stage; FB-10, a red or green gate step is recorded under its project and step.
- **Self-sense, witness, fruit**: the epic's three records. Self-sense is what a peer knows about itself and keeps. A witness is what another party signs about what it saw. Fruit is a threshold crossed by enough distinct witnesses. "A self-report graduates to nothing."
- **A person's own report**: testimony about one's own experience, given by one's own act. The SDK's name for the instrument is `community-report`. The lifecycle spec's "summary" is one. It is not a witness, and it becomes fruit only when distinct people with standing agree.
- **Term**: a declared name for one kind of observation, from a closed list shipped with the producer. The lifecycle spec and brit call it a claim id. In the SDK's grammar it is an observation term, and the promise it bears on is the claim.
- **Promise**: what a term supports or strains: a habit's invariant on the development plane, a content type's claim on the application plane.
- **Bound**: a declared limit on a measured quantity, with a soft and a hard mark.
- **Algedonic signal**: "the typed signal a holon raises when a stock approaches or breaches a declared bound" (the epic). It goes to its target unfiltered. Everything else here is feedback and moves by consent.
- **Rung**: a place a record can sit: the device, the steward's own devices, a collective, the commons.
- **Respite**: the epic's word, after Meadows, for how long a disturbance leaves before it must be answered. "A system whose response time exceeds its respite time is out of control."
- **Store**: any place records pile up: a log file, a ledger, a report directory, a table.
- **Mishpat**: the protocol's governance of precedent. A retention mishpat is the recorded rule that says what a store may hold, for how long, and what alone permits deletion.

## 2. Four raw inputs, and what each becomes

| Raw input | What it is | What it becomes | What must be declared first | Left raw, it costs |
|---|---|---|---|---|
| A log line | Narrative for whoever is debugging, in free text | Nothing by itself. If it matters it is one of the next three. Otherwise it is counted under the producer's `unclassified` term and stays on the device | Nothing | Storage and attention, without end. Between about 1,500 and 1,800 `warn!` and `error!` call sites in storage and doorway, depending on whether tests are counted, feed a log nobody is asked to read |
| An exception or refusal | A typed failure at a boundary the producer controls | One negative observation of a declared term, kept once per session: self-sense | The term, the promise it strains, a cure where one exists | The same failure looks new each time, and three different failures look alike (section 4, client apps) |
| An observation of another | What one party saw of another's behaviour | A witness: a signed row under a declared kind, on the observer's own log | The kind, its reach, its retention class, the diversity that makes fruit | A self-report passes for evidence, or evidence does not become fruit. Eight kinds are declared and one has a producer |
| Telemetry | A number sampled over time | A measure read against a bound. Crossing the soft mark is an approach, the hard mark a breach | The measure, the bound, the promise the bound protects | A gauge nobody compares. 51 of 65 declared bounds read "numb" on this device today |

Two rules hold across all four. The epic: "Never raw series, never per-member rows, never a join
across two holons." The extension to backlog row 9: "an ordinary exception with no declared bound
remains feedback, not an invented breach."

## 3. One loop, three vocabularies

The lifecycle spec names six stations a felt problem passes through in order, numbered 1 (Record)
to 6 (Stasis). They are the third column.

| The epic's loop | The SDK's verb | The lifecycle spec's station | On a device, for a tool |
|---|---|---|---|
| (the reference the loop compares against) | CLAIM | The declaration | The release ships its terms and the promise each bears on |
| Observe | ENCOUNTER | 1 Record | A person meets the limit; one record for the session |
| Interpret | WITNESS | 2 Grow | Sessions and devices are counted, not lines |
| Compare with commitment | (the comparator, the elohim's part) | 3 Trigger; the judge | The burden is read against its bound; the judge gives a reading |
| Form intent, obtain authority, act | DEMONSTRATE | 4 Respond | A review obligation; an ask; a fix by whoever stewards the producer |
| Re-observe | WITNESS | 5 Return | The fixed release arrives and the command is seen to pass |
| Attest effect, settle value | SETTLE | 5 Return, and the positive signal | Records drain to the citation; the release is owed its note |
| Learn | RENEW | 6 Stasis | The term is retired, or reopened and counted against the fix |

The epic states when a loop is closed, and every plane should use its sentence: "A loop is not
closed because a gauge moved, a signal fired or a restart was sent. It closes when a new
observation proves the required condition holds, or proves it did not and the next path has
begun."

### A viable-system reading

The repository's fullest mapping onto Stafford Beer's model is a research reading
(`genesis/research/elohim-as-viable-system-2026-06-04.md`), not doctrine, and the commons-design
skill assigns two of the systems differently. Read with that caution, the parts of this method
fall as follows.

| Beer's system | In this method |
|---|---|
| 1, operations | The producers |
| 2, damping between them | One record per session; the marker; the latch's hold time. These stop a repeated line from becoming a storm |
| 3, here-and-now control | Bounds, the judge, retention |
| 3*, audit past the reporting line | A run of the acceptance suite; a blind read; a released sample. Each looks at operations directly and does not trust a self-report |
| 4, outside and future | A person's reports reaching the steward of the producer; the root-cause analysis; the fix |
| 5, identity and policy | The steward's rulings, the covenant, and mishpat, which statement 15 of the lifecycle spec names as where value is settled |
| The algedonic channel | A breach, to its target, unfiltered |

Section 6a carries this reading through the levels, from a session to the commons.

The research critique's second gap is "no un-mediated algedonic channel". This method does not
close it. It does give one unmediated path, the one the epic names: "the first receiver of every
signal is the person who felt it", on their own device.

## 4. The producers

What each producer already has that could serve as its closed list of terms, what happens to a
failure today, what is lost, and the smallest first move. "Calls it" means the producer calls the
feedback recorder.

| Producer | A closed list it already has | A failure today | What is lost | First move |
|---|---|---|---|---|
| brit | 72 literal ids in `claims.json` (branch `run-ci/feedback-f1`) | Calls it on every command; records by verb | The reason; the session | Landing with brit 0.3.10, and the lifecycle spec's FB-1 |
| Agent hooks | Each hook's few fixed lines; rule ids such as `a2o-story-blind-reader-review` | Printed on every call. `brit-first.py` says a fallback was "recorded" and writes nothing. One rule appears 2,068 times as separate rows in a 13,105-row decision log | Any count; any sense that the line is the same line | The hook reports under its rule id through the same call, once per session |
| The device and its peer link | About 19 arrival checks, each with one cure; about 45 bin-store refusal kinds; the channel's drop and nack reasons | Recomputed on each read; only the last follow attempt is kept | How long a device has been degraded, and how often | A check that reads degraded is one observation, and its cure line is the declaration's cure |
| Local gates and rakia | About 65 gate projects, three of them attested, and about 50 steps in 28 `build-manifest.json` files (two counts of the same files differ by one project and three steps); the verdict words; a failure class from a closed list (test, compile, format, lint, env, step) | A verdict keyed on a closure is attested and already crosses between a steward's devices. A red reaches sessions live as a class, a fingerprint and a step, with no text. The live log keeps 20 runs. Refusal and reuse reasons are free text | That the same step keeps going red. Why a verdict was not reused, as anything countable | The gate runner records a red or green step by project and step (FB-10) |
| Jenkins pipelines | The stages each pipeline declares; six result words; eleven harvest categories | `ci-harvest.py` keeps 182 rows by a hash of scrubbed text: 103 open, the oldest for 173 days. One row has been seen 233 times and is marked blocked as "working as designed". Another has had its fix landed for months and stays open, because only a row marked triaged can be confirmed fixed. A confirmed row is deleted | The history of what fixed what. Any way for an open row to drain | An adapter beside the harvester (FB-9) |
| The acceptance suite | The concerns the habit register declares | Calls it, through `ingest-genesis`, when a sprint report is built; 92 reports and 1.4 GB here, none in the store | The join | FB-8 |
| Running peers | `label()` enums on decision outcomes, by the rule that "every decision outcome increments a labeled counter"; eight observation kinds | About 240 metric names and a JSON log. An outside poller keeps a ledger that deletes a row after five clean polls and holds none today. No peer raises an approach or breach | The cure; the witness; who was told | The epic's steps 2 to 4: the peer samples itself into its own log, a labelled outcome becomes a row of a declared kind, and the graduation evaluator is given a caller |
| Client apps | 62 claims and 46 observation terms in six domain manifests, counting the 38 claims and 15 terms that lamad's manifest holds behind a reference; seven capability states declared on 59 elements; a few typed errors | No global error handler. The content viewer discards the error object, so a held read, an outage and a missing item all show "Content not found". Desktop commands return strings | Which failure it was. Whether it was ever fixed | A handler that maps the typed errors and capability states to declared terms, held on the person's own peer |
| The recorder itself | `epr/recorder-unavailable:*`, declared by brit | A gap line on every command when the session's epr is behind | That the sense organ is down | The device-status condition of FB-1 |

Two findings from the survey need saying apart, because they are present harms and not gaps.

- **The app's issue report sends a bundle the person has not seen.** This is read from the code by the client survey (`diagnostic-collector.service.ts` and `issue-report.service.ts` in `app/elohim-app`), not exercised in a running app. Choosing "report" collects recent warnings and errors, the current address, the health response and correlation ids, and submits them with the person's text. There is no preview. That is the opposite of the rule the lifecycle spec takes from backlog row 9, that words and evidence are released by a person who is shown the exact bytes.
- **The declared feedback vocabulary of the application plane is read by nothing.** Six manifests declare claims and observations as the 2026-03-28 design requires. No runtime code reads them; they are generated into types and stop there.

## 5. The patterns

Each is something the repository already does in at least two unrelated places. That is the test
the requisite-variety guidestar sets: a primitive is admitted only on a second independent need. The third column is where it is absent and costs
something.

| # | Pattern | Already here | Absent |
|---|---|---|---|
| 1 | **Declare before you emit.** A producer ships a closed list of the names it can report | Observation kinds; `label()` enums; arrival checks; refusal kinds; capability states; gate projects; brit's table | Hook lines; desktop command errors; HTTP error bodies that are `{"error": text}` |
| 2 | **Say it once per session.** A session is the unit of counting | The deprecation sentinel; the managed-surface flag; the bin-head notice; ram-guard's cursor; the stasis sensor, which speaks again at twice the last count | The rust-style line after nearly every command; the story-review rule; `brit-first` |
| 3 | **The line names its cure.** | Arrival's cure line; a mesh `REFUSED` line; the bin-head notice | Content errors; desktop strings |
| 4 | **Self-sense, witness and fruit stay apart.** A count is not a set of witnessed events | The epic; the feedback store's guard; a sibling's gate verdict counted in no quorum | Any dashboard that sums self-reports |
| 5 | **Reading and acting are separate.** A read changes nothing; an attributed act writes | `epr flow report`, which lists breaches and leaves the acting to others; sense rulings; the store refuses a link without deliberate authorship; the judge and `retire` | The outside pollers, which read and delete in one pass |
| 6 | **The loop closes on a new observation, not on the fix.** | Recovery is distinct from an available release; the latch's recovered state; three green builds; five clean polls | The issue report, where no code tells the reporter its status changed; every client error |
| 7 | **A fix cites what it fixes, and the citation outlives the records.** | A correction that closes by reference; graduation that carries its observation references; the architecture ledger, which keeps closed rows | Three of five findings ledgers delete the row and keep nothing |
| 8 | **A store is born with its limits, and what it holds retires on a gradient.** | Section 7 | Section 7 |
| 9 | **A breach goes to its target unfiltered. Everything else moves by consent.** | The capacity latch; ram and disk ceilings; the stasis incident states | No running peer raises one; a breached bound dispatches nothing |
| 10 | **What crosses a rung is fruit and a count. A person's own report crosses by their act.** | The crossing allow-list; the governed-space rungs; the epic's table | The issue-report bundle |
| 11 | **Absence is said.** Unobserved is not read as healthy | `UNMEASURED`; "numb"; skipped is not zero; a declared concern absent from a run | Any rate shown without its denominator |
| 12 | **The adapter senses; the native recorder records and judges.** | The Genesis adapter; brit's two variables | Pollers that hold the ledger and the close rule in a script |
| 13 | **The recorder is a producer.** Its absence and cost are declared terms | `recorder-unavailable`; the stasis sensor's "hook timed out"; "attested: claimed" | Most sensors |
| 14 | **A person is shown at most two open concerns. The rest is a count.** | The habit register's limit of two; the headline naming numb bounds three at a time | The session-start banner |
| 15 | **Whoever felt it interprets it first.** The device reads its own records before anyone else is asked to | The store's interpretation record, written by `epr feedback group`; the epic's operator, which "carries the sensing, the folding, the signing and the first escalation"; the algedonic-designer agent, which reads raw exhaust and proposes sensors | Nothing reads a producer's unclassified pile; outside pollers interpret a peer's state for it |

Pattern 7 has an anti-pattern worth its own name: **closure by disappearance**. A ledger that
deletes a row when the problem stops cannot say what fixed it, cannot notice it returning, and
cannot credit anyone.

## 6. The contract a producer implements

Statement 2 asks whether an API is emerging. The grammar is the SDK's and exists. What emerges is
its smallest runtime form: five things a producer declares, one call, one reading and three acts.

**Declares**

| What | Shape that exists today |
|---|---|
| Its terms | `claims.json` rows for a tool; `vocabulary.observations` and `observation_kinds` for an app or a peer |
| The promise each term bears on | A habit id or a content claim. Today's findings ledgers call this the `concern` address. A proposed optional pointer on a tool's row, so brit's table joins the habit `brit-native-daily-driver` without changing shape |
| A bound on accumulated burden | A measure and a bound in `measures.yaml`; `diversity_threshold` and a graduation window for a kind |
| What a release fixes | The `cures` list: term, citation, one sentence |
| The limits of any store it keeps | Section 7 |

**Calls**

One call records one observation: a declared term, the producer's address, an outcome and a time.
It has four doors today and should have one definition.

- On the development plane, three verbs of `epr feedback`: `capture`, with the term and the producer's address in two environment variables, which any language can run by starting a process; `ingest-genesis`, which takes a sprint report and writes one observation per finding and per concern; and `record`, for evidence someone authors.
- On the application plane, `POST /api/v1/observations`, which already refuses a payload that does not match its declared kind.

**Reads**

The state of a term on this device or peer: the judge's reading. This is the SDK's missing
`ClaimInstance`.

**Acts**

- `retire`: the drain, the watch and the note. This is the SDK's missing `RenewalEvent`.
- `propose`: the review obligation, which the 2026-03-28 design already expresses as an REA commitment.
- Ask, answer and release one sample: the only way words and evidence leave.

**Homes, proposed**

| Piece | Home | Why there |
|---|---|---|
| The declaration's schema | `elohim/sdk/schemas/v1/manifest/`, beside the observation-kind schema | "JSON Schemas — the authoritative wire contract" |
| The shared types: term, reading, outcome, severity | The `elohim-epr` crate, beside its algedonic and measure types | Both planes may depend on it |
| The store and the judge | `elohim/epr-rea` and `epr-cli`, where they are | Built; the habit is green |
| TypeScript types | Generated into `@elohim/epr-ts` | No hand-written wire shape |
| Adapters | In each producer, a few lines | Pattern 12 |

The posture is the governed-space spec's for the same question: one definition, two carriers for
now. A pass through the P2P design gate is owed before any schema is written.

## 6a. Self-review, and what a closed loop leaves behind

Statements 5 to 7 rest on records that exist.

**A device reviews its own records.** The work the commons-pool split gives a person's elohim is
exactly this: sense, read, gather, draft. On a device it means reading what piled up, most of all
the unclassified pile, and writing an interpretation: which occurrences belong together, a plain
account of what they are, and optionally a proposed objective. The store already has that record
(`Interpretation`, with its occurrences, author, summary and objective) and the command that
writes it. An interpretation is the device's own text over its own open fields, so it stays on
the device like the records it reads. What it changes is who does the first reading: the one who
felt the problem, and not a steward sifting other people's logs.

**The same loop, one level up.** Statement 7 corrects statement 6: a summary over a device's
records is less a parent with children than the first level at which the whole loop runs again,
over the output of the loop beneath it. Beer's model says this of every viable system: it
contains viable systems and is contained in one. The repository already names the levels (the
epic's four rungs; the constitutional layers that the research reading calls "recursion made
literal"), and the survey for this document found none that runs one loop through them. This is a first statement of it,
offered as a reading with the same caution as section 3's.

| Level | The records it holds | What it compares them against | Its fruit, which the next level meets as one occurrence |
|---|---|---|---|
| A session | One occurrence per term | Nothing; it only records | Its records |
| A device | Its sessions' occurrences | The term's bound; the judge's reading | An interpretation, and the summary of section 5.5 of the lifecycle spec |
| One person's devices | Each device's summary | The same term's burden across the devices | One account for the person |
| A household or a collective | Each person's own report | Its own bound, counted in people with standing | An attestation that something changed |
| The commons | Attestations, and counts of what went unobserved | Its commitments | An act of governance |

Three things follow from reading it this way.

- **One contract serves every level.** Section 6's declare, call, read and act do not change. At each level the producer is the holon one level down, and the term is declared by whoever stewards that level.
- **Each level keeps its own rows.** What rises is fruit and a count. A level's fruit points to the records beneath it by address, and those addresses resolve only for whoever already holds them. That is how a pile of records becomes one addressable thing, an EPR, without the pile moving.
- **Each level has its own comparator, bound and retention rule.** The interpreting can be done alone or together, as statement 6 says: by a person's elohim on one device, by a household's operator, or by a collective, "whatever makes sense for the shape". Section 7's limits and gradient apply at each level separately.

**An interpretation graduates into work by a person's act.** Two paths exist, one per plane.
`epr feedback propose` turns a retained need into an REA Intent, "without admitting repair". In
the app, an issue report has `promoteToWorkStory`. Either is the join to the work pillar,
avodah, whose view of work is not built (lifecycle spec, statements 16 and 17).

**A closed loop is already a curated story.** When a term has run its course, the device and the
release between them hold, in order:

| Piece | Record that holds it |
|---|---|
| What was promised and what strained it | The declaration's two sentences and the promise it names |
| How often it was felt, and for how long | The counts |
| What it was, in plain words | The interpretation |
| The work taken up | The Intent and the commitment linked to it |
| The root cause and the fix | The citation in the release's `cures` list |
| Proof it held | The later observation that passed, and the watch |
| That it is over | The archival note |

That is a story with its documentation: a problem, its diagnosis, its fix and its proof, each
addressed and each written by the party that knew it. The repository's `story-harvest` skill
exists to turn a failure-then-fix cycle into a regression scenario; this trail is its input,
already gathered. A developer who wants the whole record of a fix, as a scenario or as a page,
reads it and does not have to reconstruct it. Nothing new is needed for this but the loop itself
closing.

## 7. A store is born with its limits

Statements 3 and 4 are one rule in two parts: a store declares its limits, and inside those
limits what it holds retires on a gradient. One store in the repository already keeps the first
part. The private feedback
store may not exist without a policy row that an operator established. That row
(`feedback-local-retention@1`) states its capacity, the size of one record, what may be reclaimed
and after how long, what is protected, the measures that read it, and this sentence: "Pressure is
evidence; this delegated retention policy alone permits deletion." A full store refuses new
records and keeps what it has.

That is the pattern, and it has five parts.

1. **Capacity** in bytes and a cap on one record.
2. **What may go, and when**: classes with a waiting period each.
3. **What is protected** from any automatic reclaim.
4. **A measure and a bound** on its fill, so the store's own approach to full is a signal.
5. **A recorded rule that alone permits deletion**, established by a person. Full means refuse, not evict.

Measured on this device on 2026-11-27. Sizes are as the filesystem or the surveys reported them,
and are rounded. Growth rates were not measured. By size alone the order is the campaign
directory, the bin store, the peer payload store and the acceptance reports; by rows that nothing
ever trims, the flow log and the hooks' decision log.

| Store | Size or rows | Declared limit |
|---|---|---|
| Private feedback store | 1,786 records, 5.4 MB | Yes: all five parts |
| brit's recorder spool | New | Yes: 256 files, 1,024 bytes each |
| The peer link's drop table | At most 32 rows | Yes: capped rows and groups |
| The app's log ring | 100 entries | Yes, and lost on reload |
| The cargo build pool | Not measured here; the volume it shares is 88% full | Yes: soft and hard marks in policy, with a guard that reclaims and refuses |
| Observation logs | Not measured; none on this device | Declared as a retention class per kind. Whether anything enforces it was not checked |
| The gate's live run log | 20 runs | Yes: pruned to 20 at each run |
| Gate-step readings | 1.7 MB | Yes: trimmed at 2 MiB down to 1 MiB, declared beside its measure |
| Rakia's executor logs | Per receipt | Yes: one day or five runs |
| Notes harvested from CI | 84 | Declared as 30 days in policy, and the policy says "Declared, not yet enforced" |
| Jenkins build logs | Not measured here | The orchestrator keeps 100 builds; the app and edge pipelines declare no limit |
| The flow log | 24,500 rows, 21 MB | None. Append-only, and every read verifies every record |
| The hooks' decision log | 13,105 rows, 2.1 MB | None |
| The berth ledger | 7,178 rows, about 3 MB | None |
| The cargo pool's steward log | 24,324 lines; bytes not measured | None |
| The findings ledgers | 182, 330 and 474 rows; under 0.2 MB together | None; three close by deleting |
| Acceptance reports | 1.4 GB | None. The recovery evidence in it is kept by rule, with no bound on that rule |
| The campaign directory `genesis/local-dev` | 27 GB | None |
| The bin store of tool releases | 11 GB | Not found by this survey |
| The peer payload store | 3.5 GB | The artifact tier has "no replication count or garbage collection" (governed-space spec) |
| Worktrees | 233 directories; bytes not measured | None |
| The fleet's log and metric stack | Not measured: it is not on this device | Retention not found in the repository |

**The deterministic floor, proposed.** A census the repository already has the pieces for: a
policy row carries a scope path, and a native probe can walk the places stores live and count
those no policy row covers. That count is a measure, its bound is zero, and its breach is a line
at session start like any other. A store with no row then shows as pain on the day it is born.
This document does not declare that measure. Naming the first stores to bring under a rule, and
establishing each rule, are the steward's acts, because a retention rule records a person's
authority.

One caution from the memory plane, which has the opposite default on purpose: "Demotion, never
deletion". So a retention mishpat should say what kind of store it governs. As a proposal:
evidence and memory demote, and operational records retire.

### The retirement gradient

Statement 4 asks that what a store holds retire at different speeds, so that the things most
worth answering wait longest for an answer. The repository already has this shape in two places
and has not joined them.

- The feedback store's rule has three steps today: a routine automatic success may go after 7 days; a resolved group after 30; a record a person wrote, an unanswered failure and protected evidence do not go by time at all.
- An observation kind declares a retention class from a list of five: operational, contextual, archival, attestation-feeding, wisdom.

The gradient, as a proposal, orders records by how much an answer is still worth. The waiting
periods are the retention rule's to state, and none is set here.

| Record | Waits | Why |
|---|---|---|
| A routine success | Shortest (7 days today) | It is the denominator; its count is what matters |
| An unanswered `info` observation | Short | Low cost to the one who felt it; if nobody answers soon, nobody will |
| An unanswered `warn` | Longer | A release cycle or two is a fair chance |
| An unanswered `critical` | Long | The tail is where a fix eventually arrives |
| A person's own words | Not by time; by their own act or a ruling | They chose to say it |
| Counter-evidence and protected records | Not at all | The floor |

Two things keep the short end honest. A record that ages out unanswered leaves its count behind,
as section 5's pattern 7 asks of a fix: the device still knows that a problem of this kind was
felt this many times and never answered, which is itself the signal that a term is noise or a
producer is not listening. And today's rule does not let an unanswered failure go by time at
all, so the short end of this gradient is a new permission, not a restatement. It would be a
change to the retention rule, and the steward's to establish.

## 8. What the network gains, and what backs each claim

| Gain | How local loops produce it | Backed by |
|---|---|---|
| **Maintenance.** The system cleans itself as it heals | A fix drains the records it answers; a store refuses to become a dump | One green habit (`private-feedback-loop`), for one store. The rest is proposal |
| **Cooperation.** A problem felt by many is answered once | A person's report reaches the producer's steward by an ask; the fix returns as a release that already crosses; a sibling's verdict is reused | Sibling reuse of a gate verdict is built. Reports between people are not |
| **Resilience.** Pain reaches someone who can act, and a peer that falls silent is noticed | A breach is raised by the one who feels it; siblings witness; absence is counted | Habits `pain-is-answered` and `runtime-death-witnessed` are both red. The epic lists the signal as having "no production emitter" |
| **Efficiency.** Verification is spent once | The trust canon's test: if trust does not reduce overhead, "the design has a leak" | A principle; no measure reads it |

The epic's sentence is the honest summary of where this stands: "The network observes itself the
way a body does: locally, continuously, and without a central organ." That is the aim. On the
development plane one loop of it works for one tool.

## 9. What this method must not do

Taken from the documents it stands on.

1. **It must not become the aggregator.** "The dragnet is never the sensors; it is the aggregator behind them." Nothing joins two holons' records. What crosses is fruit and a count.
2. **It must not call an exception a breach,** or filter a breach. Pain "cannot be filtered, aggregated away, or priced down by any elohim."
3. **No count changes a rule.** "No aggregate-triggered path may mutate a rule." A positive signal is a candidate and "never auto-promotes". Counts are of people with standing, "never keys or devices".
4. **It must say unobserved,** "never as healthy and never omitted".
5. **It must not mint a second ontology of measures,** or a new word where the repository has one.
6. **It must not put a person in the operator's seat.** "No screen at any rung asks a person to read a series, acknowledge an alert or restart a thing."
7. **It must not make the elohim a container for the person.** The elohim "is a high-variety attenuator that asks the human how the attenuation should be done".
8. **It must not become what the Observer Protocol forbids an observer to be:** something that shares "without explicit consent", serves "institutional power over individuals", or generates "surveillance profiles".

## 10. Order, by measured pain

A proposal for sequence, cheapest first among the things that hurt now on this device.

1. **The recorder condition and one record per session** (the lifecycle spec's FB-1). Every brit command pays for this today.
2. **Hooks as the second producer.** Two hook lines are printed after most commands and counted nowhere.
3. **A preview on the app's issue report.** It is a present breach of the consent rule and is small.
4. **The first three stores without a rule.** The hooks' decision log, the acceptance reports and the campaign directory are the largest on this device.
5. **Gates, then Jenkins** (FB-10, FB-9). Their terms exist and their fixes already cross.
6. **A global error handler in the app** that reads the vocabulary its manifests already declare.
7. **The running peers**, by the epic's own steps 2 to 4.

## 11. What this pass corrects in the lifecycle spec

- It did not cite the SDK pipeline or the 2026-03-28 design, whose gaps it fills. Section 1 of this document is that citation.
- Its "three loops at three speeds", named reflex, homeostasis and evolution, are its own words. The repository's word for the speed of a loop is its respite.
- Its "summary" is a person's own report in the epic's terms, and not a witness. The spec's concern canon already says a summary is "a claim by one principal"; this names the kind of record.
- What it calls a claim id is an observation term in the SDK's grammar. The names stay as brit built them; the join to a promise is the optional pointer of section 6.

## 12. The steward's decisions

1. Whether stores are to be brought under retention rules, in what order, and each rule's establishment (section 7). With it: whether an unanswered failure may age out by severity, which today's feedback rule does not permit.
2. Whether the contract of section 6 should become SDK schema now or after a second producer has used the call.
3. Whether the issue report's missing preview is to be fixed ahead of the rest.

## 13. Non-goals

A logging framework; a metrics pipeline; a dashboard; parsing log text into events; replacing the
fleet's monitoring stack, which the epic sequences on its own terms; any schema or code.
