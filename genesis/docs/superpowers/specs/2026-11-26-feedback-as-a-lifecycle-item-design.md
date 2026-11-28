---
title: "Feedback as a lifecycle item — a felt problem that is named, answered, drained and remembered"
id: feedback-as-a-lifecycle-item-design
status: Draft
class: process-meta
process_subdomain: feedback
date: 2026-11-26
steward: human:matthew
context-tier: disclosed
topic:
  - feedback
  - algedonic
  - claim
  - drain
  - retire-when
  - governed-space
  - standing-consent
  - brit
refines: genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md
derived_from:
  - genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md
  - genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md
serves: [private-feedback-loop, workspace-device-arrival, pain-is-answered]
graduation-trigger: >-
  one real claim felt on one of the steward's devices is cured by a release, compacted on that
  device to a cited summary, retired after its watch with its archival note readable there, and a
  recurrence is shown to reopen it
cites:
  - "governed-space-share-boundary-collective-memory-design | the sibling design this one is a second instance of: rungs a record graduates through, the standing-consent boundary clause, the crossing allow-list as its projection, and fixture standing | sha256:e684da3e7abc4153 | path: genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md"
  - "algedonic-feedback-signal | the typed pain signal and its latch that a claim's breach rides; its rule that pain is not filtered and carries no agree or disagree valence | sha256:cf902beac8dd4aec | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - "private-thought-governed-fruit | the constitutional line this design keeps: a device's activity ledger stays with its holder, only an attestation of outcome crosses by the holder's own act, and no boundary rests on a count | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "sense-respond-governance-classifier | source of the limits on a positive signal: pleasure is a candidate that does not promote itself, and no aggregate may change a rule | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - "reach-ontology-vocabulary-split-spec | why the declaration's field is named ceiling and names a rung: declared reach has no viewer term, and derived layers only narrow | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md"
  - "native-dev-plane-one-derivation-earned-reach-design | ruling D4 reused here: agreement is counted in principals with standing and not in devices | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
  - "standing-read-for-counting-principals-design | defines standing and the rule that unknown counts as none, which this design's counting relies on | sha256:190e756d8446f1d8 | path: genesis/docs/superpowers/specs/2026-11-22-standing-read-for-counting-principals-design.md"
  - "observation-event-layer-design | the application plane's declared observation kinds, whose polarity, diversity threshold and graduation this design reuses and does not rebuild | sha256:002fc7ec733dd10a | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - "commons-pool-as-collective-party-design | source of the division of labour for a person's elohim: it senses, gathers and drafts, and the person decides and sends | sha256:803fa69bf3735eb9 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "native-dev-tooling-over-peers-plan | the epic whose device channel and landing crossings carry a summary between one steward's devices | sha256:983afa13e08fe1d3 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md
  - genesis/data/timeline/backlog/steward-device-arrival-open-items.md
---

# Feedback as a lifecycle item

In brief: when a tool, an app or a runtime tells a person that something went wrong, this
repository already records that privately on the person's device. What it cannot yet do is say
*which problem* each record is, so records pile up by command name and nothing can answer them or
clear them. This document adds five things to what exists: a declared name for each kind of
problem (a claim), a judge that reads claims against the state of the device, a decay that turns
answered records into a cited summary and then into one archival note, the rule by which a
summary may leave the device, and the positive signal a clean-up gives off. It adds no store, no record kind and no signal type. Section 0
holds the steward's words; the design starts at section 1, and section 1a defines the terms. The judge's own terms (unit, covered record, kept record, aggregate, note, current cure, present cure) are defined at the head of section 6.

Status: draft, nothing here is built on the epr side. The brit side of the seam (section 12) is in
progress on the laptop. Section 16 lists what is the steward's to rule.

How it was read: five context-isolated readers in turn, each given only this document. The first
read the first draft and found its two halves carried two data models. Four read this revision;
none found a blocker, and each found three or four places where two builders would diverge, which
were corrected before the next read. The corrections to the fourth read (a claim that reopens
during its watch, a cure with no citing release, and the way out of `reopened`) have not been read
again.

## 0. Provenance

The steward's words (`human:matthew`), as typed. Statements 1 to 4 are of 2026-11-26; 5 to 19 are
of 2026-11-27.

1. "consider that this might be an epr feedback shaped item.. to handle feedback treating as a full lifecyle item would reuse our primitives and help anything built with the elohim protocol continuously improve AND maintain inself."
2. "yeah that design should allow a "problem" to get recorded (algedonic feedback) grow.. to the point it triggers a stasis action.. then the response another peer RCA/documenting the underlying issue.. then with the trace as to what the RCA of that feedback was.. a solutino goes back to the orginial peer.. drains their backlog of feedback as resolved.. and if it doesn't occur again.. that's ideal stasis."
3. ""drain their backlog" is basically helping the orignial peer clean their logs.. more efficiently, because there's no use keeping logs for a problem that's "solved""
4. "there might actually be an epr-rea "reweard" for how many logs get cleared.. making solving a bug a "rewarding" activity that has  reward preportional to the noise that it can resolve."
5. "feedback must carry with it the primitives needed to manage it wheather it's a pool of cli telemetry/error logs, or console logs in a client, or system logs from a runtime"
6. "it should also carry severity or some heierarchy of concren... probably following logging framework standards ontology.. that could be a component of a reach primitive"
7. "what is logged (usually a code change) i just know those can be security sensitive, and are often places where stuff that shouldn't be shared leaks, like PII, so this might be a little to complicated but a way of aggregating claims into reach safely... is a dimension here."
8. "the gas company can drive through your neighborhood and dectect the signal, but they need go to your front door and need permission and consent, to get into your house. That's the grandient here."
9. "a severity signal.. an elohm .. in the house CALLING for help is another valid analogue here that might allow special log export.. honestly for us in dev, this could be us, being in the special dev position of us operating our simulacra and test-fixtures, we operate in a special mode so we can exercise and refine the whole discipline e2e."
10. "we've got alot of vsm and feedback primitives aleready so we need to gather prior art and likely enhance the design rather than building something from scratch.."
11. "for logs to generate meaningful feedback that's pro-social I imagine that logs could be "drained" by another peer that handles a concern.. so like.. say a release handles an issue that can be cited, then basically that release could say.. what logs did I save that shouldn't be an issue anymore... those raw logs should delete down to a citation and a  plain-text summary for the next "retire-when" releases.. when retire-when goes away (ie. issue is proven gone), then it just becomes a archival note owed to the release, and it's history... something like that.. so as the system heals, it also cleans itself, while not losing valuable memory."
12. "how many logs a release cleans for peers has a "value" shape, because clear concise logs, could save a lot of peers from a lot of noise and lost disk space.. it's an algedonic signal that should send a reward signal through the network."
13. "well.. good hygine is it's own reward.. it feels good when you get to take a shower. :P oddly enough.. good hygine is a component of flourishing .. so let's make sure we still can have positive signals, and not preclude the network from deciding if that reward is worth it or not."
14. "or at least we should be able to settle that value by petition of the larger network.. a child cleaning their room might need some kind of extrensic incentives at times, while adults don't...  so similar concern just it COULD be a property if we wanted it to be one, the design just shouldn't preclude it where it wouldn't be possible."
15. "it's mishpat's role.. in deep context to understand what vaule "means" if it settles to something."
16. "honestly.. the whole process of cleaning logs, or resolving them accross peers, it could reuse the whole avohah work lifecycle.. I don't imagine that people would think about it that way naturally, but even in software development teams, logs often coalese into "workitems" that move accross the board, or periodic repeat tasks (like cron jobs) that could have their own level of automation that we should be able to visualize... I think getting there is long term and likely outside the scope of establishing the primitives of this sprint, but that's the long-term composition I'm seeing that would provide a lot of reused primitives.. when we return to avodah for all sort of other "work" tracking/resolution"
17. "like cron jobs... or in insurance annual policy renewals/reviews.. chores have a common shape"
18. "oh btw.. some prior art that we might want to check to resolve too is a2o reports.. they should be inheriting this shape.. at least in the past we've tried to aggregate there to help drive our own development in the past."
19. "so do brit, jenkins, our deployed eprs, and our native nextgen integration jobs (emergent rakia).. are they all benefitting from the unified perspective on epr feedback?"

Two earlier records of the steward's rulings bind this design and are quoted where they apply:
row 9 of the algedonic backlog (2026-10-10, statements (a) to (g)) and its extension "EPR owns
experience and feedback" (2026-11-08), both in
`genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md`.

The rest is the design of an agent seat (Claude, Fable 5.2: first draft by session cacccb55, the
shem integrator; this revision by session 3d494616 on the steward's tenth statement) and is a
proposal until the steward accepts it. The brit lane on the laptop reviewed the seam of section 12.

Sections 5.5, 6 and 7 were amended later on 2026-11-27 by session 3d494616, after a first draft
of a schema for the readings (station FC-4 of the plan) showed that the order of readings could
not be run as written. What changed: a reading is of one unit (a claim, named by its first failing emitter), and is told
by what the device holds; three ids are shown to a person in plain words; a summary reports
what the device has done; and the retire command has one path for a person's ruling. Three fresh
readers checked those sections in turn against one question, whether the judge and the summary
could be written from the page without a guess. The last reader's two findings were corrected
and not read again. The integrator's reader then found two faults those three had missed: a
claim retired by a person's ruling still read `on-watch`, and `reopened` was not defined where
the cure was a condition and not a release. Both are corrected, with cases in the sprint
directory's `fc4/`. The same reader then found a third, a state with no reading at all, and
proposed that the test try every combination of facts; it now does, and `open` is what remains
when no other reading holds. The reader then found two more states that read wrongly, at which point patching the
prose was stopped. Section 6 was rewritten from sixteen numbered rules, R1 to R16, which a small
model in the orchestrator's tests runs through fifteen scripted lives; the rules, the lives and
the section now say one thing, and a rule is changed in the lives first. A scribe agent wrote
the section from the rules and the designing session checked it. The integrator's reader then
ran the model against twenty-one lives of its own and found four more faults in one pass, where
prose had taken six; rules R4, R10, R13 and R14 were changed, R17 was added, and its lives are
kept beside the others. A further pass of fourteen lives and thirty-five changes to the model
closed those four and found two more things: which cure is acted on when two are present, now
stated in R4, and a device that skips a release, since settled with brit as a cumulative `cures` list and modelled
(section 7a, node 9; section 16, decision 11). None of this is a ruling of the steward's.

## 1. What this document says and does not restate

A warning, a refusal or a failure is one observation of a **claim** that its emitter declared in
advance. The device that felt it keeps the observation privately, reads it against what the device
now runs, and lets it decay as the problem is answered. A small summary of it may leave the device
by the house's own act.

This document does not restate: the private feedback store and its governed retention (habit
`private-feedback-loop`); the algedonic signal and its latch (the algedonic feedback signal spec);
measures, bounds, breaches and their dispatch (`.claude/epr-meta/measures.yaml`, `epr flow report`);
the rungs a record graduates through, the standing-consent clause and the crossing allow-list as its
projection (the governed-space spec, sections 6 to 8); observation kinds and their graduation
(`elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`); or the line between private
thought and governed fruit (`genesis/docs/architecture/private-thought-governed-fruit.md`).

It adds five things: the claim and its declaration (section 5), the judge (section 6), the decay
(section 7), the summary and how it leaves a device (section 8), and the positive signal a drain
gives off (section 9).

Two of these fill gaps the SDK names in its own pipeline (`elohim/sdk/README.md`): "CLAIM has no
runtime lifecycle", which the judge's reading answers on a device, and "RENEW has no explicit
cycle", which the decay answers. The companion document
`genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md` places this
design beside the SDK's seven verbs, the approved feedback design of 2026-03-28 and the
observability epic, and extends it to the producers this document does not design.

## 1a. Terms

- **Algedonic**: Stafford Beer's word for a pain or pleasure signal that reaches whoever can act on it without passing through the layers between. In this repository it is a typed signal with three kinds, `algedonic-approach`, `algedonic-breach` and `algedonic-recovered` (`elohim/epr/src/algedonic.rs`).
- **RCA**: root-cause analysis, the responder's account of why a problem happens.
- **Occurrence**: the existing private record of one experience (`Occurrence` in `elohim/epr-rea/src/feedback.rs`). "Occurrences are facts reported by a producer, not diagnoses."
- **Claim**: a named kind of problem an emitter can report, such as "this brit cannot grep a revision". An occurrence is one observation for or against a claim.
- **Emitter**: the released artifact that reported the occurrence, named by its content address: a tool's release record, an app bundle, a runtime image.
- **Declaration**: the emitter's own list of the claims it can report, shipped inside the emitter (for brit, `claims.json` in the release archive).
- **Judge**: the read-only code on the device that compares its occurrences with its declarations and with what the device runs now.
- **Summary**: the only thing in this design that may leave a device: a claim id, an emitter, and counts. Section 5.5 lists every field. In the observability epic's terms it is a person's own report about their own experience, and not a witness; it becomes more than one principal's claim only when distinct people with standing agree. Used alone, "summary" means this payload throughout. Two existing fields share the word and are written out in full where they appear: `Interpretation.summary`, the text field of an interpretation record, and `cures[].summary`, the one sentence a release gives for a cure, called "the cure's sentence" in prose.
- **Protected**: an existing flag on a stored record (`Entry.protected`). Retention does not select a protected record for reclaim.
- **Intent**: the REA record of something wanted and not yet committed to (`elohim/epr-rea/src/model.rs`). `epr feedback propose` already writes one from an occurrence.
- **Standing**: a person's recognised position in a collective, as the standing-read spec defines it (`genesis/docs/superpowers/specs/2026-11-22-standing-read-for-counting-principals-design.md`); unknown counts as none.
- **Rung**: one of the places a record can sit, as the governed-space spec defines them: the device it was made on, the steward's space (one person's admitted devices), a collective's space, a commons pool.
- **The house**: the people of one household and their devices; the unit that gives or withholds consent in section 8.
- **A person's elohim**: the agent that works for one person on their devices. Following the commons-pool split, it may sense, read, gather, draft and carry; it does not decide, send past the house, admit, or recognise. Those stay with the person.
- **Fixture**: a simulated person or household used to exercise the system, such as the Dowell household and the fixture steward Adam. Its records are marked so they are not read as a real person's.
- **Registry-built brit**: a brit installed by `cargo install` from the package registry, whose build ran outside a source checkout and so has no source commit to name.
- **Pin lag**: a session still running an older release of a tool than the one the device has landed.
- **Bin store and launcher**: the bin store is the device's store of installed tool releases (`epr bin`, in `elohim/eprfs/device-runtime/src/bin_store/`). It follows a published release and knows which one the device has landed, its head. The launcher is the bin store's entry point that starts a managed tool at the release a session is pinned to.
- **Principal**: a person with standing. Several devices can belong to one principal, and counts of agreement are taken in principals.
- **Channel, crossing record, crossing kind**: the channel is what travels between one steward's admitted devices on the encrypted peer link run by `epr peer serve`. A crossing record is one flow event on it, labelled with a measure and a kind. The channel carries only the kinds its allow-list names (`berth.channel.crosses` in `genesis/agentic/pool-policy.json`), and a record is at most 64 KiB.

## 2. The experience

The loop the steward described, as six stations. Each is something a person or their device goes
through.

| Station | What happens | The steward's image |
|---|---|---|
| 1 Record | The tool says what went wrong in one line that names the claim, and the device keeps one private record per session | |
| 2 Grow | The same claim seen in more sessions and on more devices is one growing thing, not many lines | |
| 3 Trigger | When its burden crosses a declared bound, the device's steward is told, unfiltered | An elohim in the house calling for help |
| 4 Respond | A steward of the emitter asks who else feels it; houses that chose to answer send a summary; one may release evidence | The gas company reads the signal from the street and knocks at the door |
| 5 Return | A release arrives that cites the problem as handled; the device drops its raw records down to a citation and a plain summary | "what logs did I save that shouldn't be an issue anymore" |
| 6 Stasis | When the problem is proven gone, one archival note remains with the release; a recurrence reopens it | "as the system heals, it also cleans itself, while not losing valuable memory" |

A device with nothing else reachable still does stations 1, 2, 3, 5 and 6 on its own, provided the
curing release reaches it. Station 4 is the only one that needs another party.

## 3. The problem, with measured facts

Read from this device's feedback store (`.eprfs/status/feedback/`) on 2026-11-27 at 15:06Z:

| Fact | Value | Consequence |
|---|---|---|
| Retained records | 1,786, in 5,391,490 bytes (73% of the 7,340,032-byte active allowance; the warning is at 85%) | About 3,000 bytes per record, because each record embeds its policy |
| Occurrences and distinct runs | 1,772 occurrences in 1,771 runs | Every brit command is its own run; nothing groups a session |
| Occurrences with no participant | 1,764 of 1,772 | See the fourth row |
| Occurrences that did not pass | 193 (147 failed, 41 refused, 5 other) | Their subjects are `brit:commit` 55, `brit:log` 40, `brit:build` 34, `brit:other` 20: a verb, with no reason |
| Not-passed occurrences that can be closed under today's rule | 6 of 193 | Recovery needs "a later successful experience by the original participant" (`feedback_store.rs:232-245`); 187 have no participant, so the rule cannot be met and their groups are kept until the store refuses new records |

Further findings, each checked against the code on 2026-11-27:

1. **The recorder costs every command.** brit starts `epr feedback capture` with a 2-second deadline on each command (`brit-cli/src/feedback.rs:55-119` at brit `c2f8193`, the commit this checkout holds). When the session's epr is older than the device's, every command prints `feedback capture gap` (backlog `steward-device-arrival-open-items` row 54). Device status has no condition for the recorder.
2. **Capture cannot hold a reason.** The subject is `{producer}:{operation}` from a fixed list of verbs, with `message: None` and `detail: Null`; producers are fixed to `brit` and `brit-guard` (`elohim/eprfs/epr-cli/src/feedback/mod.rs:308-386`).
3. **The agent-side ledgers fingerprint scrubbed text.** `ci-harvest.py:384` and `deprecation-sentinel.py:1158` hash a normalised line. Of 474 rows in `deprecations.jsonl`, 291 are marked false-positive. `ci-harvest.py:916-925` deletes a confirmed line, so a recurrence after confirmation reads as new.
4. **The SDK's claim shape is evaluated by nothing.** `ClaimDeclaration` (`asserts`, `contradictedBy`, `validityHorizon`; `app-manifest.schema.json:587`) has one reader, a seeder test of its date format.
5. **A linked problem is kept without end.** A group (the records joined to one another by links, which retention keeps or reclaims together) that holds a commitment link is protected unconditionally (`retention.rs:175-178`), so a problem a responder took up is the one kind that cannot be reclaimed.
6. **Feedback does not leave the device.** `feedback_store.rs:1-2`: "This log is not in any peer export allow-list." The policy field `sharing` accepts one value, `separately-authorized` (`feedback.rs:151`). The channel's allow-list names berth events, gate verdicts, landings and artifact custody, and says "every other measure never cross" (`genesis/agentic/pool-policy.json:197,207-212`).
7. **`brit-first.py` reads any brit that is not 0.3.4 or 0.3.5 as a stale table**, older or newer alike (line 151).
8. **The acceptance suite already feeds this store, and has carried nothing on this device.** When a sprint report is built, `genesis/a2o/scripts/lib/feedback.ts` hands it to `epr feedback ingest-genesis`. This device holds 92 sprint reports (2026-10-03 to 2026-11-07) with 46 finding rows under 28 fingerprints, and its store holds no record from that producer: the store began on 2026-11-08 and no report has been built here since. A finding's fingerprint is a hash of a normalised message (`scripts/lib/fingerprint.ts`), the same scheme as finding 3. The reports directory holds 1.4 GB.
9. **One operation word is refused today.** brit 0.3.9 sends `--operation show`; epr's capture accepts a fixed list of words and `show` is not on it (`epr-cli/src/feedback/mod.rs:316-350`), so every `brit show` prints a capture gap. Found on 2026-11-27 while reading the brit diff of section 12.

## 4. What exists

Five read-only surveys were run for this document on 2026-11-27, and a sixth checked the first
draft's file citations. The author re-read the feedback store, its retention, the algedonic spec,
the governed-space spec, backlog row 9 and the observation-kind schema; the other rows are as the
surveys reported them.

| Plane | What exists | State |
|---|---|---|
| Private record | `Feedback::{Occurrence, Interpretation, CommitmentLink, EvidenceLink}`; links carry `Relation::{Attempt, Workaround, VerifiedFix, AvailableRelease, ExperiencedRecovery, RoutineSuccess, Protected}` (`epr-rea/src/feedback.rs`). The `epr feedback` verbs `capture`, `record`, `ingest-genesis`, `list`, `show`, `triage`, `group`, `evidence`, `propose`, `link`, `repair`, `reclaim`, `migrate`, `capacity` and `measure` (`epr-cli/src/feedback/mod.rs`) | Built; habit `private-feedback-loop` green |
| Governed retention | Policy `feedback-local-retention@1`: routine successes wait 7 days, resolved groups wait 30 days "after experienced recovery, never validation or release alone"; a bounded window of receipts (`policies.yaml`, `feedback_capacity.rs`, `retention.rs`) | Built |
| Measures and bounds | Five feedback measures probed by `epr feedback measure`; bounds with soft and hard watermarks, a concern and a dispatch; `epr flow report --breaches`; the `sense:` line at session start | Built; the native probes are two |
| Algedonic signal | `AlgedonicKind` Approach, Breach, Recovered; `Severity` info, warn, critical; mandatory evidence `stock`, `limit`, `bound_ref`; one open signal per (declarer, target, kind); a persisted latch with hysteresis and dwell | Built as a type; no carrier on the DHT |
| Declared observations | `observation_kinds[]` in the domain manifests: `kind`, `retention_class`, `reach`, `diversity_threshold` (distinct households, collectives, regions; a minimum count), `graduates_to`, `graduation_policy` (`summarize`); an evaluator in `elohim-storage/src/graduation/`; `ObservationPolarity` positive and negative | Built on the application plane |
| Judgment feedback | The `FeedbackSignal` DHT entry: squelch, correction, retraction, quarantine, vouch; `standing_impact` | Built; its kind list is in the integrity zome |
| Rungs and consent | Device, the steward's space, a collective's space, a commons pool; a standing-consent clause shaped like the covenant's `variesRule`; the crossing allow-list as a projection of clauses | Governed-space spec, landed 2026-11-26; stations open |
| Counting | Ruling D4: independence "counts principals with standing, never keys or devices"; sibling records "never count in k-of-n" (`pool-policy.json:197`) | Ruled; the standing read is not built |
| Fixture marking | Habit `evidence-class-descends`: every row carries `evidence_class` of `fixture` or `real`, and "a row with no class is refused at write, never defaulted to real" | Declared, unwired |
| Acceptance reports | The a2o sprint report: findings with a fingerprint, a source from a closed list of seven, a severity and a count; a roll-up per `@concern:` tag, including concerns declared and absent from the run. The Genesis adapter stores each as an occurrence, with messages, paths, URLs and screenshots left out (`epr-cli/src/feedback/genesis.rs`) | Built; scenario "Reingesting a Genesis report" in `private-feedback.feature` |
| Device status | `Check {name, state, detail, cure}` printing `degraded … — <cure>`; the running-against-head comparison (`epr-cli/src/device/arrival.rs:24-77, 1691-1716`) | Built |
| The steward's prior rulings | Row 9: feedback is pulled by default and pushed when urgent; an answer carries an anonymous measure, and words are asked for politely and not taken; who may collect depends on the stakes. Extension: "EPR owns the shared feedback contract"; brit implements it | Recorded; not designed until now |

What the first draft proposed and what it becomes:

| First draft | Becomes | Why |
|---|---|---|
| A `Retired` record kind | An archival note written with existing kinds (section 7) | A new kind is unreadable by an older epr, and pin lag is the live problem on this device |
| `valence`: pain or pleasure | `polarity`: `negative` or `positive` | `ObservationPolarity` exists; the algedonic spec reserves "valence" for agree and disagree and forbids it on algedonic kinds |
| `reach`: device, the person's devices, household, the emitter's steward | `ceiling`, naming a rung | The reach vocabulary spec gives declared reach "no viewer term"; the governed-space spec withdrew using a ring to name holders |
| A minimum of three contributing devices | The existing `diversity_threshold`, counted in principals, and not relied on for privacy | D4; and the canon: "identities are free to mint and no boundary may rest on counting them" |
| A consent grant | The governed-space boundary clause | One shape for standing consent |
| Summaries travel upward by severity floors | Pulled by an ask, pushed only by a call (section 8) | Row 9 statements (b) and (c) |
| `<producer>/undeclared` | `<producer>/unclassified` | `elohim:undeclared-behaviour` already names a content-security signal |
| A stored sample of raw lines | Nothing stored automatically | Automatic capture already refuses narrative (`feedback_store.rs:87-89`) |

## 5. The data model

One layout, one storage model and one payload. It supersedes every fragment of the first draft.
Sections 5 to 11 are specification. Nothing in them is built unless section 4 lists it as built;
section 12 names the station that builds each part.

### 5.1 The claim id

`<producer>/<claim>[:<param>]`, for example `brit/capability-absent:grep.rev`.

- Every legal id is a literal row in the emitter's declaration. A param is one of an enumerated list. No param is derived from input, so an id cannot carry a path, a name or a typed word. A test in the emitter pins the full set.
- Not in the id: text, paths, versions, addresses, counts, times, device names.
- An id is stable across releases: aliases are folded to the primary verb, forms are added and not renamed, and a refusal's id comes from a typed error code and not from message text.
- The first segment names what the claim is about. It is usually the emitter itself. An emitter may also declare a claim about something it observes: brit declares `epr/recorder-unavailable:*`, because brit is what notices the recorder missing. The adapter that reported an occurrence is `source.producer` (section 5.2), and that is the name the policy must admit.
- Output that no declaration covers is one claim per producer, `<producer>/unclassified`. Its growth is the pressure to declare.

### 5.2 What the device stores

For what a producer captures: one `Occurrence` per producer, claim, outcome and emitter per
session, in the existing store, with no field added to the type. A record a person writes through
`epr feedback record` carries the source its author gives it, so it is a record of its own beside
any captured one (section 6, `[R1]`).

| `Occurrence` field | Holds | Kind of field |
|---|---|---|
| `source.producer` | The producer's name, which the policy's `producers` list must admit | Closed |
| `source.run` | The session key, defined below, so one session is one run. A record drained from brit's spool has `spool-<session>` here (section 12). The two records section 7 writes use the fixed words `compacted` and `retired` here | Closed (a session key, `spool-<session>`, or one of those two words) |
| `source.item` | The claim id, the outcome and the emitter, as `<claim id> <outcome> <emitter>`; the emitter is left off when none was given. The store's same-source rule then keeps one captured record per producer, claim, outcome and emitter per session. The rule compares the whole source (producer, run and item): a repeat that differs only in its time returns the record already held, and different evidence under the same source is refused | Closed |
| `subject` | The claim id. Distinct runs and recovery are already computed per producer and subject | Closed |
| `outcome` | The existing enum: `reported`, `passed`, `failed`, `skipped`, `unmeasured`, `refused`, `bridge-declared`, `fallback-declared`, `unsupported`. `passed` is an observation against a negative claim | Closed |
| `participant` | The participant the repository's feedback policy names, as today. Reading it from the session, or from a steward's grant for a service that runs unattended, waits on the read named in section 7a, node 1 | Closed (an actor id) |
| `observed_at`, `count`, `duration_ms` | As today | Measured |
| `detail.emitter` | The emitter's address: 1 to 256 ASCII characters from `A-Z a-z 0-9 . _ : + -`. A release record's address and brit's `unrecorded:` form both fit; a path does not. A value outside the rule is treated as absent and the record is still stored | Closed |
| `detail.grant` | The address of the steward's grant under which an unattended service acted, when the record was written by one. No capture code writes it yet | Closed (a CID) |
| `detail.level` | The surface's own numeric level, where it has one | Measured |
| `detail.about` | What the occurrence concerned, as an EPR address, when the surface knows it | Open |
| `detail.judged` | The input the emitter judged, such as the version that answered | Open |
| `message` | A person's own words, attached deliberately | Open |

`detail` stays an allow-listed map, and the list depends on which of three records it is. On a
session's record: `emitter`, `grant`, `level`, `about`, `judged`. On an aggregate: `emitter`, `sessions`,
`first-day`, `last-day`, `bytes`. On an archival note: `emitter`, `cure`, `sessions`, `first-day`,
`last-day`, `bytes`. In all three `emitter` means the same thing, the emitter that reported the
problem. Section 7 defines the aggregate and the note.
`message` is the type's own top-level field. The first draft used "subject" for three things. From
here `subject` is the claim id and nothing else; the private "what it was about" is `detail.about`.

**The session key** is computed by capture itself from the environment it inherits, by the rule
brit already uses for its own marker: the SHA-256 of the first non-empty of `ELOHIM_SESSION_ID`,
`CLAUDE_SESSION_ID`, `CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID` and `CODEX_SESSION_ID`; when
`EPR_FEEDBACK_INVOCATION` is set, a zero byte and that value are hashed in as well; with neither,
the key is the UTC day. Because capture computes it, every emitter that calls capture in one
session gets the same key.

**A session is the unit of counting.** A second observation of the same claim with the same
outcome in the same session is a no-op: brit's marker skips it before any process starts, and the
store returns the address it already holds. `count` stays 1. Capture leaves `duration_ms` empty on
a claim record, so a repeat matches the first record and is not refused as conflicting evidence.
The emitter is part of the item for the same reason: a tool replaced by a newer release in the
middle of a session reports as a second source, where otherwise the store would refuse it as
different evidence for the first. Two producers that meet one claim in one session leave one
record each, under the same run, because the store identifies a source by producer, run and item.
That is kept: two reports of one event are two observations, and the count of sessions is still
one. Where no session identity exists the key falls back to the day, on purpose:
one record per claim, outcome and emitter per day is the floor.

Older occurrences, whose subject is `brit:<verb>`, read as `brit/unclassified`.

### 5.3 What is derived

Nothing below is stored. The existing `Experience` view (`experiences()` in `elohim/epr-rea/src/feedback.rs`) already
computes, per producer and subject: distinct runs, interpretations, commitments, attempts, and whether a verified fix, an
available release and an experienced recovery are linked. Per claim this design reads from it the
number of sessions, the first and last day, and the bytes its records occupy. These are the
"counters" of the first draft; they are a read, not a second store.

### 5.4 The declaration

One row per claim, shipped in the emitter. The fields, and the existing vocabulary each reuses:

| Field | Meaning | Reuses |
|---|---|---|
| `id` | The claim id, in full | New |
| `asserts`, `contradictedBy`, `validityHorizon` | What the claim says, what observation contradicts it, and how long it may stand unanswered before a review is due. The first two are one sentence each. `validityHorizon` is an ISO 8601 duration such as `P14D` | The field names and the duration form of the SDK `ClaimDeclaration`. There the first two name declared observations; a tool with no observation vocabulary writes sentences |
| `supersedes` | Claim ids this one replaces | `supersedes` on bounds and policies |
| `polarity` | `negative` or `positive` | `ObservationPolarity` |
| `severity` | The emitter's level of concern (section 10) | `algedonic::Severity`, written in the logging standard's names |
| `weight` | What one occurrence costs the one who felt it. Zero means a refusal that is working as designed | New; `duration_ms` is the measured cost where known |
| `ceiling` | The highest rung a summary of this claim may graduate to: `device`, `steward`, `collective`, `commons` | The governed-space rungs |
| `cure` | A named action the device can already take (migrate, follow head, restart), where one exists | `Check.cure` |
| `clears` | The condition under which a cure is present on a device, from a closed list: `release-current:<tool>`, `release-at-least:<tool>@<version>`, `path:<p>`, `bound:<measure@v>`, `observed` (the newest observation of the claim on the device `passed`; a newer one that did not pass ends it). A list; any one holding is enough | New |
| `retireWhen` | How long the problem must stay gone before it is called gone (section 7). One form for now: `quiet:<ISO 8601 duration>`, such as `quiet:P45D`. Null means the retention policy's watch, and a shorter value than the policy's is read as the policy's | The habit register's `retire-when`; the policy's `resolvedDays` |

A release also carries one list of its own, `cures`. Each entry has five fields: `claim` (the claim id), `release` (the producer's version for the release that made the entry), `state` (`cured` or `withdrawn`), `cite` (the commit, landing or flow note that holds the root-cause analysis) and `summary` (the cure's sentence: one line of plain text, reviewed with the release). The list is cumulative: a release's declaration carries every entry of every earlier release and its own. There is at most one entry per claim and release. A claim may go cured, withdrawn and cured again across releases, each step a new entry. Releases are ordered by semantic-version precedence. The device reads the declaration of the head release it has taken; an entry whose release is later than that head is ignored. Brit fixed this shape for its release 0.3.12, at this design's request, in place of a list of what one release had handled, so that a device which skips a release still sees the cure it carried. `withdrawn` is the producer taking back its own statement that the claim is cured, and is a different thing from `reopened`, which is what a device reads from what it observes. The whole file is `{"claims": [<row>, …], "cures": [{claim, release, state, cite, summary}, …]}`.

For this repository's own producers the same file shape lives at `.epr-meta/feedback-claims.json`.
It is a new file beside the existing `.epr-meta/feedback-policy.json`, which keeps its present
fields (among them `producers`, the list capture admits) and gains none: the policy is embedded in
every stored record and an older epr refuses a field it does not know. A client
app declares in its app manifest, which already has the `ClaimDeclaration` shape. A runtime
declares in its domain manifest beside its `observation_kinds`.

The acceptance suite needs no new table. Its claims are the concerns the habit register already
declares: a scenario under a `@concern:` tag is an observation for or against that habit's
invariant, and the habit's own `retire-when:` is the claim's. A habit's `retire-when:` is a sentence and not the `quiet:<duration>` form above; how the one becomes the other is not designed. The adapter already stores one
occurrence per concern per run, passed, failed, skipped or unmeasured. A finding's fingerprint is
not a declared name, so a finding stays a device-local grouping and reads as
`genesis/unclassified` for anything that leaves the device.

### 5.5 The summary

The only payload that may leave a device. Every field is closed or measured.

| Field | Kind |
|---|---|
| `claim`, `emitter`, `polarity` | Closed: `claim` must be one the emitter's declaration lists, `emitter` must be a release whose declaration the receiver holds, and `polarity` must be the one that declaration gives that claim. It does not change with the state |
| `severity` | Closed: one of the three handled levels of section 10. A receiver folds the declaration's name through the same table and compares |
| `state`: `open`, `cured`, `retired`, `reopened` | Closed |
| `class`: `fixture` or `real` | Closed |
| `cure`: the curing release as the producer's declaration names it, the producer and the version in the entry's `release`; an address is not used because a device that skipped that release holds none | Closed. Present in a `cured`, `retired` or `reopened` summary, where it is the cure the device acted on or the cure the reopening counts against, and only when that cure is a release. An `open` summary has none, even when a cure is available and not yet acted on. A claim that cleared by a condition naming no release (section 7) has no `cure` in its summary, because such a condition can hold a path |
| `sessions`, `first-day`, `last-day`, `dropped`, `bytes` | Measured: numbers, and days in place of times |

Always present: `claim`, `emitter`, `polarity`, `severity`, `state` and `class`. The measured
fields are present according to the state, as set out below.

Not in a summary: `about`, `judged`, `message`, any path or argument, the participant, and the
session key. The sending device builds a summary by looking each closed value up in the emitter's
declaration, and a receiver refuses one that fails the same lookup. The first draft's section 6
listed the subject in the summary; that is withdrawn.

Which state a summary carries follows the judge's reading (section 6). Where the numbers come
from, by state: for `open`, `sessions` and the two days are read from the view of section 5.3 and
`dropped` and `bytes` are left out. An `open` summary can also come from a unit that already holds aggregates or a note, when a covered record arrives during its watch or after its note, or, under the default of section 16, decision 10, when a kept record does, or when a claim that reopened has been cured again and no session has yet retired it under the new cure (life `withdrawn-after-note`, where no record arrives at all). Its `sessions` and two days are then combined across what the unit holds, which is the note or, with none, the aggregates, together with its session records: `sessions` summed, `first-day` the earliest and `last-day` the latest; `dropped` and `bytes` are still left out. A receiver can therefore see `open` after `cured` or `retired` for one unit; for `cured`, the numbers are combined across the claim's
aggregates: `sessions`, `bytes` and `dropped` (their `count`) are summed, `first-day` is the
earliest and `last-day` the latest; for `retired`, all five are the archival note's, with
`dropped` its `count`; for `reopened`, they are the note's when one exists, the aggregates'
as for `cured` when the claim reopened during its watch, and the view's as for `open` when it
reopened before compaction, while the device still held per-session records. Where a unit (one claim on this device, named by the first emitter that reported it failing; section 6) that was cured twice holds two notes, "the note" is the later one. A `reopened` summary
keeps the unit's own emitter, so that a receiver can match it to the summaries that came before
it; where the observation counts against a release cure, the release it came from is at or after the `cure` it names; where it counts against a condition cure, it can come from any release (section 6, R10). Where the reopening was caused by a withdrawal and not by an observation, there is no such release: the `cure` it names is the one the producer withdrew. A
summary reports what the device has done and not what the judge proposes. That is why three
readings send `open`, and why a claim whose watch is met still sends `cured`, with the
aggregates' numbers, until the note is written. A summary is written as
labelled slots on one flow event, which is why its field names are hyphenated while a
declaration, which is JSON, uses camel case. One summary fits well inside the channel's 64 KiB
record limit.

A summary is an attestation of an outcome over private records. The records stay where they were
made.

## 6. The judge

Native, in `epr feedback triage`, at read time. It writes nothing. For each unit on the device it
gives one reading. A unit is one claim on this device, named by the first emitter that reported it failing. It holds
the later records of that claim from that emitter and from newer ones, with or without a cure
between them (life `reviewer3-A1b-commit-newest-diff-participant`, whose first two steps fail under
two releases); a record from a different, older emitter is another unit. "The
claim" in this section and in sections 5.5 and 7 means that unit.

**What the device holds for a unit:**

- Session records, of two kinds. A covered record is one captured automatically that no commitment is tied to: at most one per session in which the claim did not pass, however many times that session captures it. A kept record comes about in two ways. A person writes one: every such record is its own record whenever it is written, it does not change or replace the session's covered record, and it is held whether or not that covered record has already been compacted. Two in one session are two records when their author gives them different sources: the store folds an exact repeat of one source and refuses different evidence under one (section 5.2). Or a commitment is tied to a covered record: it is the same record, kept from then on. The tie is a link in the store; it adds no occurrence and no observation. The link names one occurrence, and a session at the device writes it with `epr feedback link --id <occurrence> --commitment <commitment>`; the model names the tied record by its session. Compaction (section 7) replaces covered records and touches no kept one. A record a person writes that passed is among the things not designed below. `[R1]`
- Aggregates: what compaction leaves in place of covered records, one for each participant, outcome and emitter among the records it replaces, and one that names no participant for records with none: these are the parts of the stored tuple of section 7. The model has one failing outcome and gives an aggregate no emitter, so its aggregates are one for each participant. Each aggregate belongs to the cure the compaction acted on. `[R2]`
- Notes: an archival note belongs to one cure. The unit's current cure is the cure its latest compaction, move or ruling acted on; before any, it has none. `[R3]`

**When a cure is present.** A release cure is present when the claim's latest entry in the head's cumulative `cures` declaration is `cured`. It is identified by that entry's `release`, whether or not the device ever took that release. Going back to an earlier head means reading that head's shorter list. `[R4]` The declaration is the cumulative list that section 5.4 describes: each release carries every entry of every earlier release and its own, so a device that skipped the curing release still sees the cure it carried, provided a later release carried the entry forward. Entries whose release is later than the head are ignored. A condition cure is present while the claim's clearing condition holds. For the condition `observed`, it holds exactly while the newest observation of the claim on the device passed. Each time a condition comes to hold after not holding is a new cure instance. Where a release cure and a condition cure are both present, the present cure is the release cure. `[R4]`

Every `withdrawn` entry in the head's declaration counts against the cure it withdraws, the latest `cured` entry before it, exactly as an observation would, for a unit whose current cure that is; it goes on counting after a later entry cures the claim again, which is what lets such a unit move to the later cure. A `withdrawn` latest entry also means no release cure is present. `[R18]`

The first row in the following table whose condition holds is the reading `[R9-R16]`. A unit with
no observation that did not pass has no reading `[R9]`. `open` is what remains, so every unit that
has an observation that did not pass has exactly one reading `[R16]`.

| Reading | Meaning | What it proposes | Summary state |
|---|---|---|---|
| `reopened` | Something counts against a cure and the claim has not been cured again with respect to that cure. An observation that did not pass, from emitter release *e*, counts against the latest `cured` entry in the head's declaration whose release is at or before *e*, whether or not the device has compacted, and whatever the unit's current cure is. Only if no such entry qualifies, and the observation was recorded after the device compacted under a condition cure, does it count against every condition cure the device compacted under before it, and the claim then reads `reopened` until every one of them has been cured again. A withdrawal counts as `[R18]` says. Cured again: for a release cure, the head's declaration holds a later `cured` entry; for a condition cure, the condition has come to hold again after the observation, or a release cure is present. A later release cure ends a reopening against a release cure, and any release cure ends a reopening against a condition cure `[R10]` | Counter-evidence to the cure | `reopened` |
| `stale-emitter` | The newest observation that did not pass came from a release behind the device's head for that tool and was made after the head landed, and the device holds no later observation from the head release `[R11]` | The local cure: restart the session, or `epr device arrive` | None: the lag is the device's own |
| `cure-available` | A cure is present, and either the unit holds at least one covered session record, or the unit holds aggregates, its current cure has been counted against, and the present cure is a different cure from its current cure. Under the default of section 16, decision 10, the unit must also hold no kept record `[R12]` | Compaction (section 7) | `open` |
| `on-watch` | The unit holds aggregates, `retireWhen` is not met, it holds no note for its current cure, and nothing has counted against its current cure. Under the default of section 16, decision 10, it must also hold no kept record `[R13]` | Nothing | `cured` |
| `retired` | `retireWhen` is met, or the unit holds a note for its current cure; and in either case nothing has counted against its current cure. Under the default of section 16, decision 10, the unit must also hold no kept record `[R14]` | The archival note, until it is written; then nothing | `cured` until the note is written; then `retired` |
| `superseded` | A declared claim names it in `supersedes` `[R15]` | Nothing | None: the superseding claim carries it |
| `review-due` | The validity horizon has passed with no cure present. Cannot hold together with `unrecorded` `[R15]` | An Intent, through the existing `epr feedback propose` | `open` |
| `unrecorded` | The emitter is no known release `[R15]` | Nothing | None: no declaration to check it against |
| `open` | Whatever remains `[R16]` | Nothing | `open` |

A reading has an id, used in records and schemas, and where the id is not plain words a person is
shown plain words instead: `stale-emitter` is shown as "behind", `cure-available` as "cure
available" and `on-watch` as "on watch". The other ids are shown as written.

The summary state in the table's last column is what a unit sends, with one exception: a unit
whose emitter is no known release sends no summary at all, whatever its reading, because a
receiver checks a summary against its emitter's declaration (section 5.5). `[R17]`

**Acting on a reading.** Acting on a reading is a separate, deliberate act, `epr feedback retire
--claim <id>`, made by an attributed session. The store already requires that: "interpretation and
linking require deliberate authorship" (`feedback_store.rs:90-92`). At session start, where the
`sense:` line is already printed, the device lists the claims read `cure-available` and those whose
watch has ended. The session at the device, a person or their agent, then runs the retire command.
Nothing runs it unattended. An aggregate with no participant cannot meet `retireWhen`, because the store matches recovery on a
named participant and an absent one matches nothing. A clearing condition (`clears`) needs no
participant.

On `cure-available` the command compacts every covered session record the unit holds, if any, into
aggregates under the present cure, moves the aggregates the unit already holds to the present
cure, and makes the present cure the current cure. The watch then runs under that cure. Kept
records stay. Notes are not moved: an earlier note stays a note for its own cure. `[R5]` On
`retired` with no note for
the current cure, it writes the note for the current cure. With a note already held for the
current cure it is refused and changes nothing. `[R7]` When `retireWhen` is met is stated in full in section 7. `[R6]` On `on-watch`, the command with a person's
ruling (`--as`) writes the note for the current cure and leaves the aggregates as they are. Without
a ruling it changes nothing on `on-watch`. A person may rule on any `on-watch` reading, not only one whose watch cannot be met. On every other reading a ruling adds nothing: the command does what it does without one. On a reading other than `cure-available`, `retired` and `on-watch`, with or without a ruling, the command changes nothing and says why. `[R8]`
The aggregates a ruling leaves carry no recovery link, so today's retention keeps them; the rule
that lets them go belongs to the new version of the retention policy, which is the steward's to
establish (section 16, decision 1).

**A decision this section does not make.** Section 16, decision 10, asks whether a claim can close while a kept record of it stands, without that record's author. The steward has not ruled. The model holds the question behind one named constant, `KEPT_REPORT_HOLDS_A_CLAIM_OPEN`, which is `true` until he does. With it true, a unit that holds any kept record, one a person wrote or one a commitment was tied to, is not compacted, watched or closed by the machine. It reads none of `cure-available`, `on-watch` and `retired`, whichever cure the record counted against, and whether or not covered records or aggregates are held beside it. `reopened` and `stale-emitter` still come first. Nothing in this design releases the hold, because there is no act by which a record's author closes it (section 7a, node 8). A person's ruling does not reach it either: a ruling acts on `on-watch` `[R8]`, which a held unit does not read. The hold masks only those three readings: a held unit still reads `superseded`, `review-due` or `unrecorded` when their own conditions hold (lives `reviewer4-N2-held-then-superseded`, `reviewer4-N4-held-review-due-no-cure`, `reviewer4-N5-held-unrecorded`; `reviewer4-N3-held-then-stale-emitter` shows `stale-emitter` coming first). With it false, a kept record holds nothing: the unit is compacted, watched and noted on its covered records and its aggregates, a session's retire moves it to a later cure, and its kept records stay as records. Both outcomes are scripted. Every life that reads differently under the two has a twin whose name ends `-if-decision-10-lets-the-machine-close`; among them are `c-false-retired`, `a-persons-report-before-the-cure-beside-a-covered-record`, `a-persons-report-arrives-during-the-watch` and `a-commitment-keeps-one-of-two-records`. Under "the machine may close" one inconsistency remains, which a reviewer pointed out: a claim reported only by kept records still reads `open` (life `f-only-kept`), since there is nothing to compact, so whether a person's word holds a claim open depends on whether automatic records happen to exist beside it. A narrower rule is possible and is not modelled: that only a kept record of a failure that counted against a cure holds. An earlier version of the model tried it, and a reader showed with five lives that its boundary depended on whether a covered record was also held and on whether the device had yet compacted. See section 16, decision 10.

**What follows from the rules:**

- A device that skips the release that made a cure entry still sees the cure (life `skipped-release-still-sees-the-cure`). `[R4]`
- A withdrawal before the device acted removes the cure, and the claim reads `open` until a later entry cures it again (life `withdrawn-before-compaction`), or until a failure arrives from the release whose cure was withdrawn, which the model reads `reopened` (the third thing it does not handle, below). A withdrawal during the watch or after the note reads `reopened`; when a later release cures the claim again it reads `cure-available`, the retire command moves its aggregates to that cure, and it is watched and noted under it, the earlier note being kept (lives `withdrawn-during-watch`, `withdrawn-after-note`). `[R18, R12]`
- A release cure ends a reopening against a condition cure (lives `condition-reopening-ended-by-a-release`, `condition-reopening-by-ruling-then-release`). `[R10]`
- A failure from the release that made the cure reopens the claim even when the unit's current cure is a condition (life `release-cure-qualifies-before-the-condition`). `[R10]`
- A claim cured by a condition can be reopened and cured again any number of times: red, green, red, green under `observed` ends `retired` (life `b-observed-red-green-red-green`). `[R4, R10]`
- A failure under `observed` before any retire returns the claim to `open` (life `i-observed-fails-before-retire`). `[R10]`
- A ruling closes a watch that cannot be met, and a second ruling for the same cure is refused (life `d-ruling`). A unit once closed by a ruling keeps those aggregates, so if it is later reopened and cured again its watch cannot be met by machine and it closes by a ruling again (life `j-ruling-twice`). `[R6, R7, R8]`
- A late failing record from the unit's own emitter that arrives after its note reads `cure-available` until a session compacts it, and then `retired` again; a record from a different, older emitter is another unit (life `g-late-own-emitter`). `[R12, R14]`
- A failure from an older release, arriving after the head's declaration holds a later `cured` entry, is not a reopening: it counts against the older cure, which the later one has cured again. A failure from the release of the latest `cured` entry is one, and stays one until a later entry cures the claim (lives `reviewer-C3-two-cures-then-fail-on-r1-no-pinlag`, `reviewer-A3-reopen-on-current-head`). `[R10]`
- Where kept records hold nothing (the other outcome of decision 10), a kept failure that counts against an earlier cure does not disturb the reading of a watch under the current cure (life `guard-is-for-the-current-cure-only-if-decision-10-lets-the-machine-close`). Under the default it holds the unit at `open`, as any kept record does. `[R13]`
- `stale-emitter` gives way as soon as the device holds a later observation from the head release, and an observation read `stale-emitter` is not counted toward the claim's burden (section 8) (life `h-pin-lag`). `[R11]`
- A release cure takes precedence over a condition cure (life `precedence-release-over-condition`). `[R4]`
- Going back to an earlier head reads the shorter list (lives `downgrade-reads-the-shorter-list`, `downgrade-below-the-cure`). `[R4]`
- Entries later than the head are ignored (life `entries-newer-than-the-head-are-ignored`). `[R4]`
- An unknown emitter's unit sends no summary (life `reviewer-B3-unrecorded-then-cure`). `[R17]`
- A commitment tied to a unit's only covered record leaves nothing to compact: a cure that arrives, or one already waiting, reads `open`, and the retire command changes nothing (lives `a-commitment-makes-the-only-record-kept`, `a-commitment-tied-while-the-cure-waits`). `[R1, R12]`
- A unit that holds a covered record beside a kept one reads `open` under the default of decision 10 and is not compacted. Under the other outcome it is compacted, watched and noted on its covered record, the kept record stays, and the note does not speak for it (lives `a-commitment-keeps-one-of-two-records`, `a-persons-report-before-the-cure-beside-a-covered-record`, and their twins). A unit that holds only kept records is closed by no rule here under either outcome (section 7a, node 8). `[R5, R12, R14]`
- A person's report that arrives during a watch holds the unit at `open` under the default; under the other outcome the unit still reads `on-watch` (life `a-persons-report-arrives-during-the-watch` and its twin). `[R13]`
- A commitment names the record it is tied to. Which record is tied decides which participant's record is left to compact (lives `named-commit-keeps-older-participant`, `named-commit-keeps-newer-participant`, and their twins). In a session that holds a person's record and a covered one it is the covered one that is tied (life `commit-on-a-session-with-a-persons-record-and-a-covered-one`). A commitment that names a session holding only kept records changes nothing, even when another session holds a covered record, and even when the named session's covered record was compacted (lives `commit-on-only-kept-records-changes-nothing`, `reviewer5-U1-named-commit-newest-kept-older-covered`, `reviewer5-U3-named-after-compaction-with-kept`). `[R1, R2]`
- A person's report written in a session that already holds a covered record leaves that record as it is, and one written after that record was compacted is still held (lives `same-session-covered-then-kept`, `kept-failure-in-an-already-compacted-session`, `two-kept-reports-in-one-session`). `[R1]`
- A watch must elapse with no failure after the pass, even a failure that counts against an earlier cure; a later pass starts that quiet again, so such a failure delays the retirement and does not prevent it (life `watch-needs-quiet-after-the-pass-if-decision-10-lets-the-machine-close`). Under the default of decision 10 no life can show this clause: a failure after the pass adds a covered record, which reads `cure-available`, or a kept one, which holds the unit, or it reopens the claim. The integrator's reader searched every continuation of four events after a compaction and found none. `[R6]`

**Also true of a unit**, and carried by the model and the lives:

- A failing observation that reopens a unit belongs to that unit and starts no unit of its own. The unit keeps the name of its first failing emitter. When it is cured again, compaction takes every covered record the unit then holds and moves the aggregates it already holds to the present cure, writing each new aggregate under the emitter its records came from; its second retirement writes a second note, for the second cure, and the first note is kept (life `a-release-life`).
- The passing occurrences that prove a recovery come from a later emitter. They are matched on the claim and the participant, whatever emitter they came from.
- A person rules with `--as`, as sense rulings do (`flow/sense.rs`). A ruling writes no recovery link for the aggregates it closes over, and the note it writes names that person as its participant (life `d-ruling`).
- Records that name no participant, which are nearly all of today's, can be closed only by a person's ruling.

The model does not handle nine things:

- Two entries for one claim and release: the model takes the later in input order; the draft schema's test refuses such a declaration.
- Build metadata in a version: `1.0.0` and `1.0.0+build` have equal precedence; the draft schema forbids build metadata in an entry's `release`.
- A failure from a release whose cure was withdrawn before the device compacted reads `reopened`; `open` is arguable.
- The model takes the order of events as time, where a store dates a record by when it was observed: a pass dated before the failure it would answer, or a record dated before a note it arrives after, is not designed.
- The model holds one unit: a failure from a different, older emitter is another unit, and how two units of one claim stand beside each other for a person is not designed. Nor is which unit an arriving record joins when the device holds two, or whether a failure in another unit of the same claim ends this unit's `observed` condition or its watch.
- An unknown emitter is ordered as the earliest release, where a real unknown build has no place in the order of releases. As written, a unit named by an unknown emitter therefore takes every later failure of the claim, and it sends no summary (R17).
- After a device goes back to a head whose list no longer holds the cure of a claim it has already compacted, a new failure counts against nothing, and the claim can retire with that record never compacted. R13 does not ask that the cure still be present, and no life follows a unit already on watch through such a step.
- A commitment tied to a session for which the unit holds no session record, none yet or its only record compacted: what a commitment tied to an aggregate means is not designed, and the model refuses the event.
- The numbers of a summary (section 5.5): the model gives a summary's state, and nothing checks which numbers it carries. Section 5.5 gives a rule for each state, and two cases show it is not settled. Where a unit holds an earlier note beside later aggregates under another cure (after life `withdrawn-after-note`), its rule for `reopened` matches twice, and its rule for `open` takes the note and so leaves out the sessions compacted under the later cure. Where a late record is compacted under a cure already noted (life `g-late-own-emitter`), the second `retired` summary carries the first note's numbers.

**The design says what a unit reads. It does not yet say all of how a unit is stored.** Two readers who had never seen it, one after the other, could follow the readings and the hold of decision 10, and could not build the store side without guessing (sprint journal, 2026-11-28). What they found is listed here so that a builder asks. None of it is decided here.

Records and counting:

- Several stored records for one session. The count of sessions is one however many occurrences a session leaves (section 5.2). Not designed: which of a session's several occurrences (two producers, two outcomes, a tool replaced during the session, a record drained from the spool under a run of its own) is R1's one covered record, for compaction and for a commitment's tie; whether a spooled record's run counts as the same session in the view of section 5.3; and what compaction does with captured `passed` records.
- A person's record. It carries the source its author gives it, so its run need not be a session key: which session such a record belongs to, which R1 and the rules for a tie ask, is not designed. Nor is a `passed` record a person writes: whether it is a kept record, holds the unit under the default of decision 10, or counts as the pass of a recovery. The model keeps a record only for an observation that did not pass.
- A commitment's tie. The link is written on the device and names an occurrence; a responder sees a summary, which names none. How a responder's commitment comes to be linked on the device is not designed (section 7a, node 2). Today's retention protects every record of the group a link is in (section 3, finding 5): whether a covered record beside a tied one can be compacted, under the other outcome of decision 10, is for the new version of the retention policy to say (section 16, decision 1).

What compaction writes, and reading it back:

- What the judge reads once records are compacted. The model remembers every observation for the life of a unit. The store, after compaction, holds aggregates in their place, and after retirement a note. Whether an aggregate or a note counts as an observation for R9, R10, R11, `observed` and `retireWhen`, whether the view of section 5.3 counts the runs `compacted` and `retired`, and how a rule that needs the release and the time of each past failure is computed from an aggregate, is not designed.
- A second aggregate with the same address. An aggregate's source carries nothing that varies with time. Compacting a late record under a cure that already has an aggregate for the same participant, outcome and emitter gives the same source with a different count, which the store refuses (section 5.2). Whether the command merges, replaces or appends is not designed. Moving an aggregate to another cure (R5) raises the same question: the cure is part of the address, the store appends and retains, and the model changes the cure in place.
- Telling one instance of a condition cure from the next in the stored aggregate and note (section 7, "What 'the cure' is when no release cites the claim"). Which condition is written when two of a claim's `clears` hold at once is not designed either.
- A condition that names a release (`release-current`, `release-at-least`) after a reopening against it. R10 ends such a reopening when the condition has "come to hold again", which has no meaning for a condition that never stopped holding; as written, only a release cure ends it. The lives exercise `observed` and a `path` condition whose truth is given to them, and none reopens a unit against a release-naming condition.

Across devices and surfaces:

- A receiver's check of `emitter`. The lookup is stated (section 5.5, R17), and the model is told whether an emitter is known. How a receiver on another device comes to hold the declaration of the emitter's release is not designed.
- Head and the order of releases are defined for tools in the bin store. For a declaration read from this repository, and for the acceptance suite and CI, whose emitter is a commit (section 12, FB-2, FB-8 and FB-9), what "head", "at or before" and "behind the head" mean is not designed.

Smaller things: the day a validity horizon is counted from; when `path:<p>` and `bound:<measure@v>` hold; which cure a `reopened` summary names when the observation counts against two, and what `cure` holds for a condition that names a release; how the command knows a ruling given with `--as` is a person's and not the session's (section 16, decision 3, states the intent); the instant a watch period is measured from when several participants pass at different times or one passes twice; what the judge does with a claim of `positive` polarity; where a summary's `class` is read from; how a head whose `cures` entries have only the three fields of brit 0.3.10 is read; and a session that fails, passes and fails again, where brit's marker skips the second failure (section 12), so the newest observation the device holds is the pass and `observed` holds.

The rules are run. `genesis/orchestrator/reading-and-summary-schema.test.mjs` carries a small
model of one unit's life and checks every step of the lives in
`genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc4/lives.json`, and it tries every
combination of the facts the readings depend on to confirm that each unit gives exactly one
reading. A change to a rule here is made in the lives first.

## 7. Decay: three steps, and what is kept

| Step | What the device holds for the claim (the unit of section 6) | What moves it on |
|---|---|---|
| Open | The covered and kept records of section 6 (compaction takes only the covered ones) | The judge reads `cure-available` |
| Cured, on watch | One aggregate occurrence for each participant, outcome and emitter (section 6, R2), carrying its count, sessions and days; for a release cure, one `Interpretation` whose summary is the release's own sentence; one `AvailableRelease` link whose evidence is the release, named by producer and version as the declaration's entry names it | The claim's `retireWhen` is met, or a person's ruling (section 6, R8) |
| Archival | One protected occurrence: the claim, the cure as the declaration names it (the producer's release by version, or the condition as written), what was dropped and over how long | A late covered record from the unit's own emitter reads `cure-available`, and `retired` again once a session compacts it (life `g-late-own-emitter`). A recurrence reads `reopened`. Under the default of section 16, decision 10, a kept record that arrives reads `open`, or `reopened` when it is itself a recurrence (see below) |

A kept record is part of none of these steps: compaction leaves it as it is. Under the default of section 16, decision 10, a unit that holds one is not compacted, watched or noted while it holds it, so a unit at the first step does not leave it. A unit that was already on watch or archival when a kept record arrived keeps its aggregates and, where it has one, its note. If the record is itself a recurrence the unit reads `reopened` first, and `open` once the claim is cured again (life `reviewer5-N1b-note-then-kept-from-curing-release`). Otherwise it reads `open` from then on, or `superseded`, `review-due` or `unrecorded` when one of those holds (section 6), with no recurrence: the device sends `open` after having sent `cured` or `retired` (lives `a-persons-report-arrives-during-the-watch`, `reviewer4-N1-note-then-kept-counting-against-nothing`). Under the other outcome a kept record changes no step, and stays beside the aggregate and the note.

- **The citation and the plain text are existing kinds.** The link is the citation; `Interpretation.summary` holds the cure's sentence. That sentence was written by whoever fixed the problem and reviewed with the release, so no text from the device's own records survives the first step.
- **The two records this section writes.** Both are ordinary occurrences. "Address of" means the store's existing content address (`atom_cid` in `elohim/epr-rea`), which the Genesis adapter already uses for the same purpose.

| Field | The aggregate (written at compaction) | The archival note (written at retirement) |
|---|---|---|
| How many | One for each participant, outcome and emitter among the records it replaces | One for the unit and its cure. Its failing emitter is the one the unit is named by, which is the emitter its summary carries (section 5.5) |
| `source.producer` | The producer | The producer |
| `source.run` | `compacted` | `retired` |
| `source.item` | `compacted:` and the address of (claim id, outcome, participant, failing emitter, cure) | `retired:` and the address of (claim id, failing emitter, cure) |
| `subject` | The claim id | The claim id |
| `outcome` | The one the replaced records shared | `passed` |
| `participant` | The one the replaced records shared | The participant of the retiring session |
| `observed_at` | The last replaced record's time | The time of retirement |
| `count` | The number of records replaced | The number of records dropped in all |
| `duration_ms` | Empty | Empty |
| `detail` | `emitter`, `sessions`, `first-day`, `last-day`, `bytes` (what the replaced records occupied) | `emitter`, `cure`, `sessions`, `first-day`, `last-day`, `bytes` (each combined across the aggregates as section 5.5 says) |
| Protected | No | Yes, by the store's existing protected append |

- **What "the cure" is when no release cites the claim.** `cure-available` can hold through the claim's own `clears` with no entry in any `cures` list. Compaction still runs. Where the condition that held names a release (`release-current`, `release-at-least`), the cure is the device's head release of that tool, and the link is written to it; there is no sentence, so no interpretation is written. Where it names none (`path`, `bound`, `observed`), the cure is the condition as written, and neither link nor interpretation is written. In both cases that value is what the tuple and `detail.cure` hold. Section 6 makes each time a condition comes to hold a new cure instance (R4), and the model numbers them. How one instance is told from the next in these stored values is not designed: as written, two instances of `observed` give a second note the source of the first, which the store's same-source rule returns or refuses (section 5.2).
- **From cured to archival.** When the judge reads `retired`, the retire command writes, for each aggregate, the store's existing recovery link to the earliest `passed` occurrence of the claim by that aggregate's participant in a session after the retire that made the unit's current cure current, dated at that occurrence's time. Then it appends the note. The aggregates, the interpretation and the links are not kept on purpose: until a recovery link exists retention cannot select them, and once it does they are a resolved group past its 30 days, which ordinary retention reclaims. The cure's sentence and citation are not copied into the note, because the release's own `cures` list keeps them and the note names the release. An aggregate whose participant has not seen the claim pass keeps the claim on watch; a person may rule on it.
- **`retireWhen`** is met when the unit holds at least one aggregate, every aggregate it holds names a participant, each such participant has a passing observation of the claim in a later session than the retire that made the unit's current cure current (its latest compaction or move), and the watch period has since elapsed with no observation of the claim that did not pass. An aggregate that names no participant cannot meet it, and a pass that names no participant counts for no aggregate. A unit moved to a later cure therefore starts its watch again. `[R6]` That is the store's existing rule for experienced recovery and the policy's `resolvedDays`, whose floor is 30 days. A claim may ask for longer.
- **The archival note is "owed to the release"** in that it names the release, by producer and version as the declaration's entry does, so the history of a release is the set of notes that name it.
- **A reopen is counter-evidence.** It counts against the cure, reverses the positive signal of section 9, and is owed to the cure's author wherever the `retired` summary went. The canon holds that floor: "Corrections always reach the participant they concern."
- **The first step changes policy.** Retention today permits deletion only for routine successes and for groups 30 days past experienced recovery. Compacting on a cited release drops per-session records earlier, while keeping their count. That needs a new version of the policy, which is the steward's to establish (section 16, decision 1).
- **What may not be dropped:** a flow-log or channel record, including one that carried a summary, and protected evidence. Sending a summary does not hold back the device's own records, which never crossed.
- **What compaction covers:** automatically captured records only. A record a person wrote, and a record tied to a commitment, are kept as they are. The detail keys `about`, `judged` and `level` are dropped with the records that held them.

Surface logs are outside the store. A runtime's log files and a client's console buffer are held
by the surface. This design gives the surface the signal that a claim is cured and leaves the
dropping of matching lines to a later station (section 7a, node 5).

## 7a. Missing nodes

Each is a step between two things that already have names.

1. Chain: a problem is closed. Between "an occurrence is captured automatically" and "an experienced recovery can be linked to it". Missing node: capture stamps the participant the act was for. Under it lies a second missing node, found when the first slice was built (2026-11-27): a read that answers "for whom does this act, and under what authority" from a file, without the session store's lock. The session envelope names a steward and no participant, and its one lookup takes a lock that can wait five seconds, while brit allows the recorder two. The same read serves a service that runs unattended under a steward's grant (the carrier being designed on ethosengine): a session resolves through its envelope, a carrier through its grant. Capture treats an unresolved answer as no participant and still stores the record. Probe: a failing brit command in an attributed session, followed by a passing one in a later session, yields a recovery link. Present state: 1,764 of 1,772 occurrences on shem have no participant; the first slice leaves that as it is.
2. Chain: a problem is answered. Between "a responder's commitment is linked to an occurrence" and "its group may be reclaimed". Missing node: retention reads whether the linked commitment was fulfilled. Probe: a group whose commitment is fulfilled and whose claim is retired is selected for reclaim. Present state: `retention.rs:175-178` protects every linked group. Under the default of section 16, decision 10, a claim with a linked record does not reach `retired`, so this probe waits on that decision or on fulfilment being what releases the hold.
3. Chain: the recorder is itself sensed. Between "the recorder cannot be reached" and "the device's steward is told". Missing node: a device-status condition for the feedback recorder, with a cure per cause. Probe: `just status device` on a device whose recorder could not be reached names how many gap notes wait, since when, and the command that catches up. Present state: the first slice adds that line; the existing `epr binary` line already says when a session's epr is behind the device's.
4. Chain: one session is one run. Between "a tool reports a claim" and "the store counts sessions". Missing node: the session key passed as the invocation. Probe: twenty brit commands in one session leave one record per claim. Present state: 1,771 runs for 1,772 occurrences.
5. Chain: the surface cleans itself. Between "a claim is cured on this device" and "the surface's own log lines for it are dropped". Missing node: an adapter call by which a surface drops what a cure covers. Probe: after a retirement, the runtime's stored lines for that claim count zero. Present state: nothing. The acceptance suite is the nearest case to hand: 1.4 GB of raw results, traces and screenshots on this device, whose findings the store would hold in a few kilobytes.
6. Chain: a claim on the application plane. Between "a claim declaration" and "an observation kind". Missing node: a closed-vocabulary field type in the observation-kind schema, whose types today are `Cid`, `String` and numbers. Probe: a kind whose param is an enumerated list refuses a value outside it. Present state: a `String` field is open.
7. Chain: a development device. Between "a roster row admits a device" and "its records may be released in full". Missing node: a declaration, by the steward, that a device is a development device. Probe: `berth status` shows it, and an export from a device without it carries no open field. Present state: the roster row has no such field.
8. Chain: a person's own report is closed. Between "a person wrote a record, or a record is tied to a commitment" and "the claim it reports is retired". Missing node: a way for a unit whose records compaction may not replace to close. Compaction leaves such records as they are (section 7), so the unit has nothing to put on watch, and by rule R16 of section 6 it reads `open` even after a cure is present. Node 2 above is the half of this for records tied to a commitment; nothing covers a record a person wrote by hand. Probe: a claim reported only by `epr feedback record`, cured by a release, reads something other than `open` after a session acts on it. Present state: no reading, no command and no retention rule reaches it; the model's life `f-only-kept` shows it. Under the default of section 16, decision 10, the same is true of every unit that holds a kept record, whatever else it holds.
9. Chain: a fix is seen by a device that skipped the release that carried it. Between "a release lists a claim in its `cures`" and "a device that runs a later release reads the claim as cured". Cause: a device that goes from release 0 to release 2 runs the fix release 1 carried but would not see the cure unless the declaration says so. Brit chose the cumulative `cures` list and has said it will ship it in release 0.3.12 (relayed by the shem integrator from the brit lane on thinkpad, 2026-11-27). The judge reads the head's declaration (section 6, `[R4]`): because the list is cumulative, every earlier entry is present in a later release's declaration, and a device that skipped a release still reads the cure it carried. Probe: a device that takes release 2 without release 1 reads a claim release 1 cured as `cure-available`. Present state: agreed and modelled (life `skipped-release-still-sees-the-cure`), not shipped: brit 0.3.10, the release on this device, has an empty `cures` list, and no judge exists in code.
10. Chain: a cured claim is compacted and read again. Between "the retire command compacts a unit's records" and "the judge gives the unit the reading the model gives". Missing node: the retire command's contract with the store: what each record it writes is addressed by, and how each rule is computed from aggregates and notes once the session records are gone (section 6, the closing list of what is not designed on the store side). Probe: the model's lives, replayed against a real store, give the model's reading at every step. Present state: not designed; the model remembers every observation and the store does not keep them.

## 8. Leaving the device

The canon sets the frame. "Activity ledgers are held by the holon they describe," and the plane
beyond sees "only what crossed by a participant's own act." A device's occurrences are such a
ledger. So nothing in this section happens because an emitter declared it may: the emitter's
`ceiling` only limits how far a house's own act can carry a summary. The distance travelled is the
lesser of the two.

### The steward's image, mapped

| In the image | In this design |
|---|---|
| The signal read from the street | A summary, answered to an ask |
| It places a neighbourhood and not a house | Closed and measured fields only; `about` is not sent |
| The knock at the front door | A polite ask for evidence, addressed to the claim |
| Permission and consent | The person sees the exact bytes and releases one sample, or declines |
| Inside the house | Open fields: `about`, `judged`, a person's words |
| Someone inside calling for help | The call: the person's elohim drafts, the person sends |

### Three ways a summary moves

1. **Between one steward's devices.** A summary is carried on the existing channel as a crossing record: one flow event with a `measure:feedback-claim@1` slot, a `reason:<kind>` slot whose kind is `open`, `cured`, `retired` or `reopened`, and the summary's other fields as labelled slots. The allow-list gains one line, `"feedback-claim@1": ["open", "cured", "retired", "reopened"]`. The `misses` kind is the precedent for the form and for a reader beside it (`elohim/eprfs/eprfs-peer/src/allow.rs`, `elohim/eprfs/epr-cli/src/flow/misses.rs`). It lets a device count the steward's other devices and learn of a cure a sibling has seen. A sibling's record is that steward's own and is counted as no second principal. This needs the steward's standing consent for that kind of record at that rung (decision 2).
2. **Pulled into a collective's space (the drive-by).** A steward of the emitter, or of a collective, who is in good standing asks how a claim is felt. Each device holding it answers with a summary if a clause in the house's boundary covers that kind of record for that collective. Otherwise the person's elohim drafts the answer and the person decides. Row 9: "WHO may collect depends on the stakes"; a sensitive claim asks for an asker of higher standing.
3. **Pushed (the call from inside).** When a claim's burden breaches its bound, the person's elohim drafts a call for help: the summary, and whatever evidence it would send, as exact bytes. Sending is the person's, each time, unless a standing clause covers it.

Words and evidence travel only by one further act, **the polite ask**. A responder who needs an open
field asks for it by claim. The ask goes down the path the summaries came up, so the responder
learns who holds the claim only from whoever chooses to answer. Row 9: "the asker makes a polite
ask of that person rather than taking them." What was released and received leaves a trace, and
this design promises no clean take-back.

### What protects the house

- **Closed fields, checked at both ends.** The declaration is the schema of what crosses.
- **`about` is not sent and not hashed.** In a small household a hash of a short name can be guessed back.
- **The house's own act.** Past the steward's devices nothing moves without a clause or a decision.
- **A security-sensitive claim sets `ceiling: device` or `steward`.**
- **Counting is a dial and not a wall.** A collective's boundary may ask that a summary graduate further only when several principals with standing hold it, using `diversity_threshold`. Devices and unknown identities count zero. Because identities are free to mint, this design does not rest any privacy on that count.

### The standing clause, and the open house

- **Standing consent** is a clause in the house's boundary, in the shape the governed-space spec proposes: the party, their words, the kinds of record, the rung, an expiry, and revocation by a later clause from the same party. A clause records a person's own statement. An agent cannot write one for them.
- **Every exported record carries `class`**, `fixture` or `real`, following habit `evidence-class-descends`. A record with no class is refused.
- **The open house** is a house whose every resident has fixture standing. Its records are born `fixture`, and its clause may cover every field, open ones included. This is where the whole loop is exercised first: the six stations, the ask, the release and the drain, with fixture personas answering.
- **The steward's own development devices** become open the same way, by a clause in his own words, once a device can be declared a development device (section 7a, node 7). Statement 9 describes that mode. It is not yet a clause.
- A caller, an emitter or a responder cannot open a house. No severity does either. The protocol's rule for entry without consent is written for another plane, in the Observer Protocol's four-level response: "The victim maintains control except in Level 4 emergencies", which are cases of imminent danger to a person. This design builds no such path and maps no tool's severity onto it.

### Feedback and the algedonic signal are different things

The extension to row 9 draws the line: "A measured bound crossing may produce algedonic escalation;
an ordinary exception with no declared bound remains feedback, not an invented breach."

- A summary is feedback. It moves by consent, as above.
- A breach is an algedonic signal. It carries only a stock, a limit and the bound that declared them, and it goes to its target unfiltered. For a claim felt on a device the target is that device's steward, through the existing `sense:` line and `epr flow report --breaches`.
- The bound is declared where bounds live, in `.claude/epr-meta/measures.yaml`: a measure `feedback-claim-burden@1` (sessions times weight over a window, per claim), probed natively, with soft and hard watermarks, a concern and a dispatch. Crossing the soft mark is an approach and the hard mark a breach, by the existing latch. This replaces the first draft's "effective severity rises one step".

## 9. The positive signal, and value

Statement 13 is the order of things: the device's own cleanliness comes first.

1. **On the device.** A drain returns bytes, and the capacity bound that was approaching recovers. That recovery is already a signal (`algedonic-recovered`, by the latch). The capacity report gains one line per cure: what it dropped and how much it returned. This needs no one else.
2. **At a rung.** A summary whose state is `retired` is the positive signal: it carries the cure, named as the declaration names it, and the numbers `sessions`, `dropped` and `bytes`. Summed per cure by those at that rung, it is what a release removed from its peers: noise and disk. Statement 12 calls this a value shape, and it is readable without any further design.
3. **As value.** Whether that sum settles into anything, and what it would mean, is not decided here. Statement 15 places it with mishpat; statement 14 asks only that it stay possible. So the summary carries what a later settlement would need: the cure as the declaration names it, measured sizes, the class, and counts taken in principals. Two carriers exist if the network wants them: the observation kind `shefa:appreciation-emitted`, which names a recipient, a magnitude and a context, and the REA `satisfies` and `fulfills` relations.

Limits that existing rulings already set on any such settlement: a pleasure signal is "a promotion
*candidate*" and "never auto-promotes"; "no aggregate-triggered path may mutate a rule"; one
steward on both sides is household upkeep and not reward; a reopen reverses what a drain earned;
and only the benefiting device's own judge counts a drain.

## 10. Severity

- **The declaration is written in the logging standard's names**, the six OpenTelemetry severities, each standing for its number range. Every common logging framework maps onto them, so an adapter translates a level and does not invent one.
- **The device handles three levels**, the existing `algedonic::Severity`:

| Standard name (number range) | Handled as |
|---|---|
| `trace` (1-4), `debug` (5-8) | Not feedback; stored only where a declaration names them |
| `info` (9-12) | `info` |
| `warn` (13-16) | `warn` |
| `error` (17-20), `fatal` (21-24) | `critical` |

- The surface's own number is kept on the device in `detail.level`.
- **Severity is the emitter's opinion; weight is the cost to the one who felt it.** Both are kept, because an emitter can call something `info` that blocks a person for an hour.
- **Severity does not widen anything.** A `critical` claim whose ceiling is the device stays on the device. The reach vocabulary spec states the same law for derived layers: they "may only narrow, never widen".

## 11. Surfaces and loops

Four surfaces are designed here, one adapter each. An adapter only senses and calls the native
recorder. Two more, the CI pipelines and the local gates, are named in the inventory that follows
the table and are stationed in section 12 without further design.

| Surface | Emitter | How an occurrence is sensed |
|---|---|---|
| Command line | The release record's address, given by the launcher | Typed codes in the tool (section 12) |
| Client app | The app bundle's address | An SDK call per declared claim; unhandled and console errors fold into `unclassified`; held on the person's own peer |
| Runtime | The binary's or image's digest | A declared measure crossing its bound is the occurrence, read from counters the runtime already keeps; log lines are not parsed |
| Acceptance suite | The suite's commit and the system under test, both already in the report | The existing Genesis adapter, one occurrence per concern per run; its raw results, traces and screenshots stay in the reports directory |

Three loops at three speeds. The three names are this document's own; the repository's word for
how fast a loop must answer is its respite, from the observability epic.

1. **Reflex.** The declaration names a `cure` the device can take. It is proposed, and is run without a person only after the steward rules (decision 4).
2. **Homeostasis.** A bound breaches and the existing stasis loop is dispatched (station 3).
3. **Evolution.** A summary reaches the emitter's steward, a cure returns as a new release, and the device drains (stations 4 to 6).

### Who feeds the store today

Statement 19 asks whether every producer benefits. On 2026-11-27 two of five call the recorder,
found by searching for its callers across the orchestrator, the CI scripts, the hooks, rakia,
storage and the doorway. The policy file agrees: its admitted producers are `human`, `genesis`,
`brit` and `brit-guard`.

| Emitter | Today | Under this design | What is missing |
|---|---|---|---|
| brit | Calls the recorder on every command; names records by verb | Claims from its `claims.json` | In progress (FB-1, FB-2) |
| Acceptance suite | Calls `ingest-genesis` when a sprint report is built; has carried nothing on this device | Concerns as claims | FB-8 |
| Jenkins pipelines | Does not call it. `ci-harvest.py` reads build logs into `.claude/data/ci-findings.jsonl` (182 rows), keyed by a hash of scrubbed text, and deletes a row once it is confirmed fixed | An emitter whose claims are the stages its pipelines declare. A red stage is an observation, a green one on a later commit is `passed`, and the fix is a landing, which already crosses | FB-9 |
| Local gates and rakia | Do not call it. A verdict is an attestation on a closure and already crosses as `gate-verdict@1`; a red reaches sessions live through the gate tap. Rakia's sources do not mention feedback | Claims are the projects and steps `build-manifest.json` declares. The verdict is already the summary. What is absent is an occurrence on the device that felt the red, so that a red which keeps returning is one growing claim | FB-10 |
| Deployed peers | Do not call it. A node has the observation plane, the algedonic type with no carrier between nodes, and a self-report ledger that is empty | The runtime surface in the table above | Not stationed here; habit `pain-is-answered` |

Each of the three outside paths loses the cure: the CI ledger deletes it, the gate tap is live
only, and the runtime ledger holds nothing. For the two build surfaces the declared, closed list
of claim names already exists, which is the one thing section 5.1 requires.

The recorder is itself an emitter. Its absence and its cost are claims:
`epr/recorder-unavailable:{not-found,timeout,refused,spawn-failed}` and `epr/recorder-slow`.

## 12. Stations, and the seam with brit

Each station adds one capability. Stations 1 to 3 and 5 to 6 need one device. Station 4 has two
parts, and only the first runs on what is built. The stations are prose here on purpose: nothing
is claimable from this document. A plan restates them as claimable stations once the steward
accepts the design.

- **FB-1 Record.** Capture reads the claim and emitter from the environment and stores them as section 5.2 says. It derives the session key itself, from the environment it inherits and by the rule in the seam below, and does not use the `--invocation` argument on a claim record. When a claim is present it does not check the `--operation` word against its fixed list and does not store `--elapsed-ms`. Producers come from the policy and not from a fixed pair. `epr feedback drain-spool` turns each gap note brit left into one record and removes the note. Device status gains a `feedback recorder` line that counts the gap notes waiting and names that command. Not in the first slice: stamping the participant (section 7a, node 1), and the launcher setting the release record's address. Probe: twenty captures of one claim in one session leave one record per producer, claim, outcome and emitter, and a device with gap notes waiting reads `degraded` with its cure.
- **FB-2 Grow and judge.** The device reads declarations (brit's `claims.json` from the head release in the bin store; this repository's from `.epr-meta/feedback-claims.json`) and `epr feedback triage` gives the readings of section 6. Probe: an occurrence from an older emitter reads `stale-emitter`, and `cure-available` once the head lists it in `cures`.
- **FB-3 Trigger.** The measure `feedback-claim-burden@1`, its bound and a native probe. Probe: a claim past its hard mark appears in `epr flow report --breaches` with its concern.
- **FB-4a Respond, among the steward's devices.** The crossing kind and its reader beside gate misses in `epr peer intents`. Depends on decision 2. Probe: a claim open on one device is counted on a second with no subject or text in the record.
- **FB-4b Respond, by ask.** The ask, the answer and the polite ask at a collective's rung. Depends on the governed-space stations for signed acts and for admission by membership. Exercised first in an open house.
- **FB-5 Return.** `epr feedback retire`: compaction, the release's sentence and the link. Depends on decision 1. Probe: after the curing release lands, the claim's records are one aggregate, one interpretation and one link, and the dropped count matches.
- **FB-6 Stasis.** The archival note, the `reopened` reading, and the capacity report's line per cure. Probe: the note survives a reclaim; a recurrence from the curing emitter reads `reopened`.
- **FB-7 Second emitter.** `brit-first.py` reads brit's claims from the head release and keeps no list of its own; a brit older than its table reads as pin lag with its cure, a newer one as a stale table.

- **FB-8 The acceptance suite as an emitter.** The judge reads the adapter's per-concern occurrences against the habit register, and the adapter records the suite's commit as the emitter. Probe: a concern failed in one report and passed in two later ones reads `on-watch`, then `retired` after its watch; and a report built on this device leaves records in its store, which today holds none.

- **FB-9 CI stages as an emitter.** An adapter beside `ci-harvest.py` records a red or green stage as an observation of the claim its pipeline's manifest declares, with the pipeline definition at its commit as the emitter. Probe: a stage red on three builds and green on the next two reads as one claim felt three times, then on watch; the CI ledger's row for it can be dropped without losing that history. Not designed beyond this line.
- **FB-10 Gates as an emitter.** The gate runner records a red or green step the same way, by gate project and step. Probe: a gate red on two pushes and green after a landing reads `on-watch` and names the landing. Not designed beyond this line.

The first slice for the epr side is FB-1 and FB-2. An `epr-rea` or `eprfs` source change has
selected the Edge pipeline before; the landing's fleet-roll preflight answers whether this one
does, and that check is the integrator's.

### The seam with brit

Agreed with the brit lane on 2026-11-27, and built on brit branch `run-ci/feedback-f1`:

- **Claim and emitter reach capture as environment**, `EPR_FEEDBACK_CLAIM` and `EPR_FEEDBACK_EMITTER`. An older epr ignores them and records as before, so brit needs no version gate. The arguments are unchanged, and the `--operation` word is drawn only from the list every installed recorder accepts, with `other` for any command outside it; a test in brit pins that list. This is the cure for finding 9 of section 3 on the brit side.
- **The emitter** is the release record's address, which the launcher passes as `EPR_BIN_RECORD`, because a binary cannot contain its own record's address. A registry-built brit falls back to `unrecorded:<version>+exe.<sha12>` over the executable's bytes.
- **A marker, checked before any process is started**, keyed on claim, emitter and session key, skips repeats. This removes the start and the 2-second wait that every brit command pays today. The marker is brit's alone: a file brit creates under `$XDG_CACHE_HOME/brit/feedback-markers/`, named by the key; for a command claim, and for any `passed` observation, the key also takes in the outcome. (A different directory beside it, `feedback-notices/`, holds brit's once-a-session notice that the recorder left a gap. An earlier version of this line named that directory for the marker, which was this design's mistake and not brit's.) Brit has said it will bound that directory and `feedback-notices/` from 0.3.12: at 4,096 entries it will remove files older than 7 days and leave lock files, and when still full it will skip writing, which costs a repeated spawn or a repeated gap line and not a record. epr neither reads nor writes it; epr's own guard against repeats is the store's same-source rule (section 5.2).
- **The session key** is brit's existing `gap_notice_key()` (brit 0.3.9; the brit lane reports commits `67f4068d8` and `c84b34665`, which this checkout's older brit does not hold): the SHA-256 of the first non-empty of `ELOHIM_SESSION_ID`, `CLAUDE_SESSION_ID`, `CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID`, `CODEX_SESSION_ID`; with `EPR_FEEDBACK_INVOCATION` set, a zero byte and that value are hashed in as well; with neither, the UTC day. epr's capture computes the same key and writes it as `session-<hex>`, `invocation-<hex>` or `day-<days since 1970-01-01, UTC>`, the hex being the 64 characters of the digest and each value trimmed of surrounding space before it is hashed (`session_key` in `elohim/eprfs/epr-cli/src/feedback/claim.rs`). The agent-side hook derives the same key, so the two producers' records share one run; each leaves its own record (section 5.2).
- **The recorder cannot record its own absence**, so brit writes the claim, cause and time to a spool: `$XDG_STATE_HOME/brit/feedback-spool/<32 hex>.json`, one file per session and recorder identity, at most 1,024 bytes, written by rename, at most 256 files. A recorder that is missing, times out or cannot be started is skipped for the rest of that session. A recorder that refuses one record is not unavailable, so its file is keyed on the session, the recorder and the refused claim: only that claim is not retried, and one session can leave several `refused` files. The file does not say which claim was refused. Its keys are `v`, `claim`, `cause`, `at`, `emitter`, `session` (hashed), `recorder` (`path`, `size`, `mtime`) and `stderr` (the recorder's first error line). brit does not delete; epr drains. `stderr` and `recorder.path` are open fields. `epr feedback drain-spool` reads at most 256 notes a run, without following a link, turns each valid one into an occurrence (producer `brit`, run `spool-<session>`, item `<claim> failed <emitter>`, the note's own time, no participant) and removes the note it read and no other file; `stderr` and `recorder.path` are not stored. A note it cannot read as one is left where it is, and device status then says so.
- **`claims.json` lists every legal id in full**, and `brit feedback claims --json` prints it.
- **A refusal by design** (`brit/refused:<verb>.<form>`: a push deletion, a force push) declares weight 0 and no `clears`. Anything a later release could remove is `brit/capability-absent`.
- **`brit/command:<verb>`** is today's per-command capture: weight 0, ceiling `device`. It is the denominator, and it is what retention already treats as a routine success.
- **The human line** gains one suffix, ` [fb brit/capability-absent:grep.rev]`.
- **Brit has said that from 0.3.12 the declaration's `cures` list will be cumulative**, as section 5.4 describes. No release does this yet.

The brit diff (branch `run-ci/feedback-f1`, commits `e33aa20d2` and `9e14426e0`) was read against this section on 2026-11-27 and agrees with it. It was read, not built or run, by this document's author.

Asked of the brit lane by this revision and accepted by it on 2026-11-27, before its release:

| Change | From | To |
|---|---|---|
| Field name and values | `valence`: pain, pleasure | `polarity`: `negative`, `positive` |
| Field name and values | `reach`: device and wider | `ceiling`: `device`, `steward`, `collective`, `commons` |
| New optional field | | `retireWhen`, written as null for now |
| New shape of `claims.json` | a list of claims | `{"claims": […], "cures": [{claim, cite, summary}]}`; `cures` may be empty. This is brit 0.3.10's shape; from 0.3.12 each entry is the five fields of section 5.4 |
| New emission | | The first time per session that a cured form succeeds, one `passed` observation of the cured claim. Only the cherry-pick forms can be observed this way today, and a test in brit refuses a cure cited for any other claim until its form is mapped |
| Field type | `validityHorizon`: the number 14 | `"P14D"` |

`severity` stays as brit wrote it, in the standard's six names; epr folds them.

## 13. Design gate

The repository's P2P design gate, run 2026-11-27. No DHT entry type, no DNA change and no HTTP
route. Head-plane cost is none. Local-first reading: one device with nothing reachable records,
judges, breaches to its own steward, and drains once the curing release reaches it.

- **Occurrence.** Private. One per claim per session; this device held 1,772 after nineteen days, and the session key cuts that by the number of commands per session. Addressed by content, as today. Held in the device's sidecar; the extension to row 9 notes that its binding to a source chain is to be designed with the browser and native adapters together.
- **Declaration.** Content-derived, as part of the emitter's own release record or manifest. Which declaration applies is the one in the release a device runs, a declared dependency and not the latest.
- **Summary.** Attested-private: an outcome proof over private records. Between a steward's devices it is a channel record of at most 64 KiB. At a collective's rung its carrier is the collective's governed space or a declared observation kind. It becomes a notary fact only under the observability epic's rule, when "the target commitment is itself notarized and cross-holon".
- **Archival note.** Private, protected, about 3,000 bytes each at the store's present overhead. Counted by the existing measure `feedback-protected-bytes@1`, so its growth is visible.
- **Clause.** The governed-space boundary clause; not redesigned here.
- **The ask and the call.** Messages between peers. Their concern canon is owed before FB-4b is built.

**Stakes.** Every entity behaves at every declared network stage. A fixture house runs at the
simulacra stage by declaration, and its class travels with every record. One cost is
floor-protected at every stage: counter-evidence from a reopen reaching the cure's author.

**Worst-holder test.** The dragnet is the aggregator. Here that is whoever reads summaries at a
rung: an emitter's steward. They could see, per claim, how many sessions in a house felt it and on
which release; within a small collective a claim and a release together can point at one device.
They could join nothing to it, because no `about`, participant or session key is sent. They could
compel nothing, because open fields move only by a person's release. The bounds are hard boundary 2
(the house's own act), boundary 3 (a clause is revocable) and boundary 6 (inside a collective,
members see each other; outside it, nothing). The default ceiling is `steward` for that reason.

## 14. Concern canon

For the judge's readings and the summary as a message.

| Class | State | Note |
|---|---|---|
| Plane location | answered | The device's private store and the peer channel; not the DHT, not the deployment manifests |
| Anti-self-election | answered | Only the benefiting device's judge counts a drain; a responder's statement drains nothing |
| Monotonic authority | partial | A declaration is addressed by content; nothing yet refuses an older declaration replacing a newer |
| Liveness | answered | Every station but 4 runs alone |
| Honest absence | partial | `unrecorded` and `unclassified` are distinct readings; a missing declaration and a refused one are not yet |
| Evidence not authority | answered | A summary is a claim by one principal; a sibling's adds no principal |
| Bounded work | answered | One record per claim per session; the spool is capped at 256 files |
| Idempotent effect | answered | The same source returns the same address; a retire that finds nothing to do writes nothing |
| Advertise equals serve | partial | A device whose policy lacks the crossing kind refuses it; a status line for the mismatch is owed by the governed-space plan |
| Observability per decision | partial | Each reading is printed with its reason; a refusal to answer an ask needs a labelled line (FB-4b) |
| Identity lineage | unbound | Nothing says what becomes of summaries sent through a device later revoked; the governed-space spec carries the same gap |
| Contract evolution | answered | No record kind is added; new values ride an allow-listed map and environment an older epr ignores |
| Backpressure | answered | A full store refuses and keeps what it has; capture does not spool or recurse |
| Consent and authorization | partial | The clause has no home until the governed-space boundary can hold one |
| Graduated authority | answered | Compaction, then watch, then archive; a person may rule on any claim on watch, and only a ruling closes a watch that cannot be met |
| Witnessed residual | partial | A capture gap is spooled and drained; a dropped crossing is counted by the channel |

## 15. Habit

This work extends habit `private-feedback-loop` (green), whose guard already holds the distinction
this design depends on: "A proposal, validation attestation, available release and experienced
recovery are distinct acts." Its checks gain the probes of FB-1, FB-2, FB-5 and FB-6. The recorder
condition serves `workspace-device-arrival`. FB-4b and the runtime surface serve `pain-is-answered`
(red).

A habit for the drain itself is proposed and not declared: that a problem cured on a device is
compacted, watched and retired there, with one note kept. Its check would be the a2o scenario of
section 15a under its own `@concern:` tag. Declaring it is the steward's choice.

## 15a. The scenario the first slices must pass

Drafted as `genesis/a2o/features/devflow/feedback-drains-when-cured.feature`, beside
`private-feedback.feature` and under `@concern:private-feedback-loop`, with every scenario marked
`@wip`: a fixture household feels a problem, it grows across sessions, the device's person is told,
a second device counts it, the tool's steward asks and the household answers what it chose to, a
curing release drains the records to a citation and a sentence, and after the watch one note
remains. Further scenarios show a recurrence reopening it and a session on an out-of-date tool
being told the cure is its own. A station takes its scenarios off `@wip` as it lands; the story is
not a gate on starting FB-1.

## 16. Decisions, each with a working posture

A posture is what the stations build to unless the steward corrects it. None is his ruling.

1. **Compacting on a cited release.** Posture: yes, as statement 11 describes, by a second version of `feedback-local-retention` that adds this one permission and keeps every other rule. Needs from him: the establishing statement, since the policy row records an operator's authority.
2. **Summaries between his own devices.** Posture: carried, as closed and measured fields only. Needs from him: the clause in his own words, or his word to add the kind to the channel's allow-list, which today says no other measure crosses.
3. **A drain with no person present.** Posture: an attributed session may retire a claim whose condition is machine-checked; a person rules where none is, and may rule on any claim on watch (section 6, R8). Corrects the first draft, which had the judge itself write.
4. **Running a declared cure with no person.** Posture: proposed and not run. Not built until he rules.
5. **Past his own devices.** Posture: declared in the format, exercised in an open house, and built for real people only after the governed-space stations it depends on and his ruling.
6. **Standing consent for a real person, and his own devices as open.** Posture: in the format and not built. Needs from him: the clause, and whether a device may be declared a development device.
7. **Severity.** Posture: declared in the standard's six names, handled as the existing three. If he wants `error` and `fatal` kept apart, that is one more level on the algedonic type.
8. **Whether a drain settles into value.** Posture: not decided here; left possible (section 9). Statement 15 places it with mishpat.
9. **A habit for the drain.** Posture: proposed in section 15.
10. **Can a claim close while a kept record of it stands, without that record's author?** Posture: not decided; the model answers no until he rules (section 6, "A decision this section does not make"). A kept record is one a person wrote, or a captured record that a responder's commitment was tied to. A captured record has no author; for it the question is the third thing asked below. With the constant `KEPT_REPORT_HOLDS_A_CLAIM_OPEN` true (the default), a claim that holds any kept record is not compacted, watched or closed by the machine: it reads `open`, or one of `reopened`, `stale-emitter`, `superseded`, `review-due` and `unrecorded` when its own condition holds (section 6). Not even a person's ruling closes it, since a ruling acts only on a claim that is on watch. Nothing in this design releases it: there is no act by which a person closes their own report, and retention does not read whether a linked commitment was fulfilled (section 7a, nodes 8 and 2). With it false, a session's retire closes the claim around its kept records, without their author, and the records are still kept; a claim reported only by kept records still reads `open`, because there is nothing to compact. Both outcomes are scripted as paired lives, which section 6 names. Needs from him, three things. Which of the two outcomes. Whether a narrower rule is wanted in their place, in which only a report that a fix failed holds the claim; it is not modelled, and an earlier attempt could not state its boundary in a sentence. And what releases a held claim: for a record a person wrote, an act of its author; for a record tied to a commitment, the commitment's fulfilment, which is how a responder's work on the cause would drain the claims it took up (his statements 2 and 3, section 0).
11. **A device that skips a release does not see the fix that release carried.** Posture: settled with brit as a cumulative `cures` list (section 5.4; section 7a, node 9). Needs from him: nothing, unless he wants the seam otherwise; the entry's second state is named `withdrawn` at this design's request.

The first draft carried a "Resolved" list. Its answers were the integrator's, not the steward's:
the first draft's author records that the steward typed "resolve those answers and get back with
thinkpad" to that session on 2026-11-27, and this document's author did not see that exchange.
Two of those answers are not stated elsewhere in this revision. They are carried forward as
working postures the steward has not ruled on: the default `validityHorizon` for tool friction is
fourteen days, and unclassified output does not leave the device. Every other answer is restated
or replaced by sections 5 to 8 and the decisions above.

## 17. Non-goals

A diagnostics ledger; fingerprints over free text; one flow note per emission; a sentinel that
scans output; a claims dashboard; an agent dispatched per claim; compaction of the flow log;
replacing the five agent-side findings ledgers; an algedonic kind on the DHT; any design of what a
drain is worth; entry to a house without consent.

**The long-term composition (statements 16 and 17).** A claim that is taken up is a piece of work, and the
work pillar (avodah) is where work is tracked and shown. This design does not build that view. It
keeps the join in the records that view would read: a claim past its horizon becomes an Intent
through `epr feedback propose`; a responder's commitment is tied to the occurrence by the existing
commitment link; a cure is the event that fulfils it; upkeep that returns on a schedule, a chore, is the
same claim reopened, with its history in its archival notes. Those are REA records, which is what
avodah composes over. Nothing in this sprint depends on it.
