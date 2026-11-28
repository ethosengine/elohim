---
id: feedback-contract-fc3-one-call-four-doors
title: Feedback contract, FC-3 — the one call that records an observation, and the four doors that do it today
date: 2026-11-28
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; fields read from the code by Codex (astra); citations opened by a Claude Haiku subagent; prose by a Claude scribe agent
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-3 this table delivers, and whose condition it finds unmet | sha256:295822228cd72766 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design that names the fields of an observation on a device, cited in rows as L | sha256:20f22882feea7ab4 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 6 says the contract has one call, cited as O | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc1-primitives-inventory | the inventory whose rows first found time, subject and the observation record defined differently on the two planes | sha256:334e42fd83e95af4 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc1-primitives-inventory.md"
---

# FC-3: the one call that records an observation, and the four doors that do it today

This is the deliverable of station FC-3 of the feedback loop contract plan. For the one call that records an observation, it tables every field against the four doors that record one today: the three `epr feedback` verbs `capture`, `ingest-genesis` and `record`, which write to a device's private feedback store, and `POST /api/v1/observations`, which writes to storage. Each row says where a door takes the field from, where the field lands in the stored record, and whether the doors agree on its meaning and form. It records what exists. It proposes no interface.

## The finding

The plan's verify condition for this station is: one fixture expresses the same observation for each door, a test on each side reads the fixture, and the interface says for every field which door holds it where and which fields a door lacks. The code does not meet it.

No field is carried by all four doors in one form. Three names appear at all four doors: the time observed, a subject and a kind. Of these, the time observed is the only one that means the same thing at each door, and the doors disagree on its representation: the three `epr feedback` doors carry it as an RFC 3339 string (`C/claim.rs:139`, `C/mod.rs:398`, `C/genesis.rs:36–41`, `R/feedback.rs:41`), while POST carries it as epoch seconds (`W:25`, `H:336`). All four have something called a subject, but POST's is an optional content address (`J:19–21`, `W:29`), while the others carry free text with three different contents: the claim id (`C/claim.rs:141`), `producer:operation` (`C/mod.rs:400`), and `genesis:finding:{fingerprint}` (`C/genesis.rs:94`).

Twelve fields are carried by the three `epr feedback` doors and absent from POST. Inside the detail container, `ingest-genesis` writes fourteen keys of its own, and `record`, which stores authored JSON, can carry any key, nine of which the designs name. Twelve fields belong to POST alone. Four names carry conflicting meanings across doors: `run`, `subject`, `kind` and Genesis's `detail.source`. One fixture cannot express one observation for all four doors today without dropping or translating fields.

## How it was made, and by whom

Codex (astra) read the code, read-only, in this repository at commit `ad0b5ea177`. A Claude Haiku agent that had not written the table opened about 150 of its code citations, confirmed every quoted refusal message, tested the central claim itself, and corrected four citations and one list. The designing session (shem session 3d494616, Claude Fable 5.2) briefed and checked. Prose and layout are by a Claude scribe agent.

CONFIRMED on a row means every cited line was opened and matched. Every line number is true of the commit it was read at and will move as the files change.

## Path abbreviations

Code letters name directories relative to the checkout:

- `C` = `elohim/eprfs/epr-cli/src/feedback`; `R` = `elohim/epr-rea/src`
- `S` = `elohim/elohim-storage/src`; `W` = `S/observation/wire.rs`; `H` = `S/api/observations.rs`
- `J` = `elohim/sdk/schemas/v1/inputs/observation-intent.schema.json`

Two letters name designs, not code: `L` = the feedback lifecycle design; `O` = its companion, one loop for every producer. A row whose only citations are `L` or `O` describes a designed field with no implementation traced.

## How to read a row

*Field* names the field and its design citation where one exists. *Claim capture* and *Older capture* are the two modes of `epr feedback capture` (`C/mod.rs:107`): with a claim environment variable and without. *ingest-genesis* is the sprint-report ingester (`C/mod.rs:130`). *record* is the authored-evidence command (`C/mod.rs:122`). *POST* is the HTTP door at `/api/v1/observations` (`H:4`), dispatched at `S/api/mod.rs:367`. *Stored as* names the destination field or type in the stored record. *Agreement* compares the doors that carry the field, not the absent ones. A dash means absent.

## Fields the three `epr feedback` doors carry -- CONFIRMED

| Field | Claim capture | Older capture | ingest-genesis | record | POST | Stored as | Agreement |
|---|---|---|---|---|---|---|---|
| Producer; `L:270` | `--producer` flag (`C/claim.rs:116`) | `--producer` flag (`C/mod.rs:331`) | fixed `"genesis"` (`C/genesis.rs:93`) | input `source.producer` (`R/feedback.rs:30`) | -- | `source.producer` (`R/feedback.rs:30`) | same |
| Claim id; `L:255` | env `EPR_FEEDBACK_CLAIM` (`C/claim.rs:12,141`) | -- | -- | unrestricted `subject` (`R/feedback.rs:44`) | -- | `subject` (`R/feedback.rs:44`) | capture with claim only |
| Emitter; `L:277` | env `EPR_FEEDBACK_EMITTER` (`C/claim.rs:14,147`) | -- | -- | arbitrary detail (`R/feedback.rs:53`) | -- | `detail.emitter` (`R/feedback.rs:53`) | same |
| Outcome; `L:274` | `--outcome` flag (`C/claim.rs:118`) | `--outcome` flag (`C/mod.rs:373`) | computed (`C/genesis.rs:95,123,145`) | input `outcome` (`R/feedback.rs:45`) | -- | `outcome: Outcome` (`R/feedback.rs:45`) | same |
| Session/run; `L:271,292` | environment hash and day (`C/claim.rs:16,76`) | `--invocation` or PID and time (`C/mod.rs:374`) | CID of `runId` (`C/genesis.rs:54`) | input `source.run` (`R/feedback.rs:32`) | -- | `source.run` (`R/feedback.rs:32`) | conflicting meaning |
| Item; `L:272` | claim, outcome and emitter (`C/claim.rs:129`) | operation; guard adds outcome (`C/mod.rs:392`) | finding or concern CID, or `"run"` (`C/genesis.rs:93,137,158`) | input `source.item` (`R/feedback.rs:34`) | -- | `source.item` (`R/feedback.rs:34`) | same |
| Time observed; `L:276`, `O:328` | computed now (`C/claim.rs:139`) | computed now (`C/mod.rs:398`) | `generatedAt`, normalized (`C/genesis.rs:36`) | input `observedAt` as RFC 3339 (`R/feedback.rs:41`) | `observedAt` as integer >= 0 (`J:27`); default now (`H:336`) | `observed_at` (`R/feedback.rs:41`; `W:25`) | same meaning, different form |
| Count; `L:276,301` | fixed `1` (`C/claim.rs:143`) | fixed `1` (`C/mod.rs:402`) | `occurrences` or `1` (`C/genesis.rs:95,138,159`) | input `count` (`R/feedback.rs:47`) | -- | `count: u64` (`R/feedback.rs:47`) | same |
| Duration; `L:276` | `None` (`C/claim.rs:144`) | `--elapsed-ms` flag (`C/mod.rs:403`) | scenario sum or run `durationMs` (`C/genesis.rs:21,159`) | input `durationMs` (`R/feedback.rs:48`) | -- | `duration_ms: Option<u64>` (`R/feedback.rs:48`) | same |
| Participant; `L:275` | the participant the repository's feedback policy names, which may be none (`C/claim.rs:140`; `Policy.participant`, `R/feedback.rs:101`) | the same, from the policy (`C/mod.rs:399`) | `None` (`C/genesis.rs:94`) | input `participant` (`R/feedback.rs:43`) | -- | `participant: Option<String>` (`R/feedback.rs:43`) | same |
| Subject; `L:273,290` | claim id (`C/claim.rs:141`) | `producer:operation` (`C/mod.rs:400`) | finding, concern or run (`C/genesis.rs:94,138,159`) | input `subject` (`R/feedback.rs:44`) | optional content address `subjectCid` (`J:19–21`) | `subject` (`R/feedback.rs:44`); `subject_cid` (`W:29`) | conflicting meaning |
| Message; `L:282` | -- | -- | -- | input `message` (`R/feedback.rs:51`) | -- | `message: Option<String>` (`R/feedback.rs:51`) | one door only |
| Impact | -- | -- | severity (`C/genesis.rs:95`) | input `impact` (`R/feedback.rs:49`) | -- | `impact: Option<String>` (`R/feedback.rs:49`) | same |
| Detail container; `L:284` | emitter or null (`C/claim.rs:147`) | null (`C/mod.rs:406`) | adapter JSON (`C/genesis.rs:96`) | input as arbitrary JSON (`R/feedback.rs:53`) | -- | `detail: serde_json::Value` (`R/feedback.rs:53`) | same |
| Kind; `O:332` | derived; `R/feedback.rs:71` tags the record as `occurrence` on serialization | same | same | input: the `kind` tag of the authored JSON selects which of the store's record kinds is written (`C/mod.rs:123`, `R/feedback.rs:71`) | required `observationKind` (`J:14` names the key, `J:16` holds the pattern) | enum tag (`R/feedback.rs:71`); `observation_kind` (`W:28`) | conflicting meaning |
| Nine `record`-only detail keys; `L:278–286`: `grant`, `level`, `about`, `judged`, `sessions`, `first-day`, `last-day`, `bytes`, `cure` | -- | -- | -- | arbitrary JSON (`R/feedback.rs:53`) | -- | `detail.*` (`R/feedback.rs:53`) | one door only |
| Fourteen `ingest-genesis` detail keys (citations below) | -- | -- | computed values | also expressible as arbitrary JSON (`R/feedback.rs:53`) | -- | `detail.*` (`R/feedback.rs:53`) | same |

The fourteen `ingest-genesis` detail keys and their citations: `adapter` (`C/genesis.rs:96,139,160`), `source` (`C/genesis.rs:67,96`), `sut` (`C/genesis.rs:55,96`), `provenance` (`C/genesis.rs:48,96`), `aggregate` (`C/genesis.rs:96`), `suggestedObjective` (`C/genesis.rs:97`), `passed` (`C/genesis.rs:140,161`), `failed` (`C/genesis.rs:140,161`), `pending` (`C/genesis.rs:140,161`), `skipped` (`C/genesis.rs:140,161`), `total` (`C/genesis.rs:161`), `presentInRun` (`C/genesis.rs:140`), `declaredCoverageKnown` (`C/genesis.rs:141`), `declaredCoverage` (`C/genesis.rs:142,162`). The `record` command can also carry any of these as arbitrary JSON.

## Fields only POST carries -- CONFIRMED

| Field | Claim capture | Older capture | ingest-genesis | record | POST | Stored as | Agreement |
|---|---|---|---|---|---|---|---|
| Payload; `O:332` | -- | -- | -- | -- | required `payloadJson` string (`J:32`) | `payload_json` (`W:31`) | one door only |
| Observer | -- | -- | -- | -- | optional `observerCid` string, equals header (`J:10`; `H:254`) | `observer_cid` (`W:22`) | one door only |
| Subject kind | -- | -- | -- | -- | optional `subjectKind` string (`J:23`) | `subject_kind` (`W:30`) | one door only |
| Log CID | -- | -- | -- | -- | computed (`S/observation/manager.rs:123`) | `log_cid: String` (`W:23`) | one door only |
| Log offset | -- | -- | -- | -- | computed (`S/observation/manager.rs:121`) | `log_offset: u64` (`W:24`) | one door only |
| Sequence | -- | -- | -- | -- | computed (`H:330`) | `seq: u64` (`W:26`) | one door only |
| Household | -- | -- | -- | -- | `None` (`H:342`) | `observer_household_cid` (`W:33`) | one door only |
| Collective | -- | -- | -- | -- | `None` (`H:343`) | `observer_collective_cid` (`W:34`) | one door only |
| Region | -- | -- | -- | -- | `None` (`H:344`) | `observer_region` (`W:35`) | one door only |
| Archetype | -- | -- | -- | -- | `None` (`H:345`) | `observer_archetype` (`W:36`) | one door only |
| Compute class | -- | -- | -- | -- | `None` (`H:346`) | `observer_compute_class` (`W:37`) | one door only |
| Signature | -- | -- | -- | -- | empty (`H:347`) | `signature: Vec<u8>` (`W:39`) | one door only |

A reach or scope is carried by no door: the POST body is closed by `additionalProperties: false` (`J:8`) with its properties at `J:10–34`, and neither stored record has such a field (`R/feedback.rs:39`; `W:21`).

## What each door validates

| Door | Validation before storing | Refusal messages |
|---|---|---|
| Capture (claim) | Grammar check on the claim environment variable; invalid emitter dropped, not refused; outcome by serde enum parsing | `"EPR_FEEDBACK_CLAIM must be <producer>/<name>[:<param>] in lowercase letters, digits and '-' (param may add '.'), at most 128 characters"` (`C/claim.rs:111`) |
| Capture (older) | Producer, operation and invocation format checked; duration by integer parsing | `"automatic producer requires a registered adapter"` (`C/mod.rs:333`), `"unknown structured operation"` (`:371`), `"invocation must be an opaque alphanumeric token"` (`:386`) |
| ingest-genesis | Report fields, RFC 3339 time, source and severity lists, unsigned counts | `"unknown Genesis finding source"` (`C/genesis.rs:81`), `"unknown Genesis severity"` (`:87`), `"Genesis finding count must be positive"` (`:91`), `"Genesis skipped must remain a subset of pending"` (`:121`) |
| record | Serde `Feedback` parsing rejects unknown fields and types; 2 MiB input cap | `"feedback input limit exceeded"` (`C/mod.rs:123,321`; `R/feedback.rs:38,71`) |
| POST | Header and body identity, size, fields, time, kind and payload checked (`H:254–320`) | `"unknown observation kind {kind}"` (`H:307`), `"{kind}: payload ref_cid must equal subjectCid (an observation names one subject)"`, `"{kind}: field '{field}' is not declared by the kind"` (`S/services/observation_kinds.rs:247`), `"{kind}: field '{field}' must be {base}, got {value}"` (`:297`) |

The three `epr feedback` verbs share a store layer that refuses an invalid occurrence or producer outside policy, an automatic narrative, disabled capture or authorship, conflicting source evidence, an invalid RFC 3339 time, and retention overflow (`R/feedback_store.rs:71–135,186`). Its refusal messages include `"occurrence needs an RFC3339 observation time"` (`R/feedback_store.rs:186`), `"source identity already retained with different evidence"`, and `"invalid occurrence or producer outside policy"`. Capacity mode enforces migration, protection and retention bounds (`R/feedback_store/generation.rs:395–458`).

## What the tables show

No field is carried by all four doors in one form. The time observed is the only field that all four doors carry with one meaning, as an RFC 3339 string in the three `epr feedback` doors and as epoch seconds in POST. A subject and a kind are also present at all four, each meaning something different at POST.

The three `epr feedback` doors carry twelve fields that POST lacks: producer, claim id, emitter, outcome, session/run, item, count, duration, participant, message, impact and the detail container. Inside the detail container, `ingest-genesis` writes fourteen specific keys (report aggregates) and `record` names nine through design citations (arbitrary authored detail). Within those twelve, what only one door carries: claim id and emitter are set only by `capture` with a claim; `capture` without a claim supplies the operation that builds subject and item; report aggregates come only from `ingest-genesis`; arbitrary detail including message comes only from `record`.

POST carries twelve fields the three `epr feedback` doors lack: payload, observer, subject kind, log CID, log offset, sequence and six context slots that are null today (household, collective, region, archetype, compute class, signature). Reach or scope is absent on both sides: the POST body is closed by `additionalProperties: false` at `J:8`.

Four names carry conflicting meanings across the doors:

- `run`: a hash of the session in claim capture (`C/claim.rs:74–76`), the invocation or a process and time in older capture (`C/mod.rs:374`), an address of the report's run id in `ingest-genesis` (`C/genesis.rs:54`), and an input field in `record` (`R/feedback.rs:31–32`).
- `subject`: the claim id in claim capture (`C/claim.rs:141`), `producer:operation` in older capture (`C/mod.rs:400`), `genesis:finding:{fingerprint}` in `ingest-genesis` (`C/genesis.rs:94`), and an optional content address in POST (`J:19–21`, `W:29`).
- `kind`: a serialization attribute tagging the stored record as `occurrence` (`R/feedback.rs:71`) in the three `epr feedback` doors, and a declared observation kind with a namespaced pattern (`J:14–17`) in POST.
- Genesis's `detail.source` (`C/genesis.rs:67`, `:70–79`, `:96`): a category of finding, not the `Source` triple of producer, run and item (`R/feedback.rs:29–35`).

One fixture cannot express one observation for all four doors today without dropping or translating fields. Claim capture fixes count, time and detail and drops duration and message. Older capture lacks claim id and emitter and replaces subject and item with operation-derived values. `ingest-genesis` requires report-shaped input, hashes identities, and writes a run record for every report. `record` can express the other `epr feedback` doors' occurrences directly. POST requires its own observer, observation kind and payload, needs time translation, and has no dedicated claim, outcome or source fields; encoding those inside payload would require a mapping that does not exist.

## Not read

The code reader did not read complete designs beyond the requested sections, individual kind manifests, database migrations, underlying sidecar or transaction implementations, or network publication code. The checking agent did not open `S/api/mod.rs` beyond line 367, `C/mod.rs` lines 1–99, `R/feedback_store/generation.rs`, or the design documents. The design citations `L:` and `O:` in the table were not checked by anyone but their author.
