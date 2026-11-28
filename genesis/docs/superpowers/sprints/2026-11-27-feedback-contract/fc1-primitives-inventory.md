---
id: feedback-contract-fc1-primitives-inventory
title: Feedback contract, FC-1 — where each primitive is defined today
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; inventory read by Codex (astra); citations opened by a Claude Haiku subagent
serves: [private-feedback-loop, dev-system-equilibrium]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-1 this table delivers | sha256:d29a95ac372adc78 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design whose primitives the table looks up, cited in rows as L | sha256:a9cc251c6e355063 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 6 lists the contract pieces, cited in rows as O | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
---

# FC-1: where each primitive of the feedback contract is defined today

This is the deliverable of station FC-1 of the feedback loop contract plan: one table in which
each primitive the contract relies on names the existing type or schema that defines it, or is
marked NEW, with every other definition listed beside it and the difference stated. It records
what exists. It chooses nothing: where two definitions disagree, both are listed, and choosing
one home is the work of stations FC-2 to FC-5 and, for some rows, the steward's.

## How it was made, and by whom

- The inventory was read from the code on 2026-11-27 by Codex (astra), read-only, from this
  repository at commit `9d9322ca1` and from the brit branch `run-ci/feedback-f1` at `e33aa20d`.
  That commit is station FB-1 of the same plan, the first slice of code: it added
  `C/feedback/claim.rs` and `C/feedback/spool.rs`, which several rows cite. Line numbers in the
  two designs are those of commit `dd59d312f`.
- Every citation was then opened by a second agent that had not written the table (a Claude
  Haiku subagent): about 149 citations, no mismatch. It corrected two line numbers, which are
  applied below.
- The designing session then re-read one row, Validity horizon, against brit's later tip
  `a99143327`, because the field's type changed between the two commits. Other brit line numbers
  are those of `e33aa20d`.
- Every line number is true of the commit it was read at and will move as the files change. The
  type, field or key name beside it is what to search for.
- CONFIRMED on a row means every cited line was opened. PARTIAL means the definition was not
  fully traced.

## Path abbreviations

- `R` = `elohim/epr-rea/src`; `C` = `elohim/eprfs/epr-cli/src`; `D` = `elohim/eprfs/device-runtime/src`
- `S` = `elohim/sdk/schemas/v1`; `A` = `S/manifest/app-manifest.schema.json`; `K` = `S/manifest/observation-kind.schema.json`
- `B` = `brit-cli/src` in the brit repository (`https://github.com/ethosengine/brit.git`, the
  submodule at `elohim/brit`), on its branch `run-ci/feedback-f1`, which is ahead of the commit
  this repository pins
- Two letters name designs, not code: `L` = the feedback lifecycle design; `O` = its companion,
  one loop for every producer. A row whose only other definition is at `L` or `O` is defined in
  prose and in no code

## The table

How to read a row. *Defining home today* is the one existing definition a schema would most
naturally start from. *Other definitions found* lists every other place that states the same idea
in its own words, a design included where the design is the only other statement. *Difference to
resolve* says how those disagree, or "none". *Uses that are not definitions* lists up to three
places that read or carry the primitive without stating its shape, so that whoever settles a
definition knows what it touches.

| Primitive | Defining home today | Other definitions found | Difference to resolve | Uses that are not definitions |
|---|---|---|---|---|
| Term / claim id — CONFIRMED | `C/feedback/claim.rs:40`, `valid_claim`: `<producer>/<name>[:<param>]`; `[a-z0-9-]`, parameter additionally `.`, maximum 128 bytes | `B/feedback/claims.rs:104`, literal `DECLARATIONS`; `:575`, grammar assertions | Syntax validation does not establish membership in the producer’s closed declaration. | `C/feedback/claim.rs:141` |
| Observation-kind name — CONFIRMED | `K:12`, `"^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$"` | `S/inputs/observation-intent.schema.json:16`, `"^[a-z0-9-]+:[a-z0-9-]+$"` | Input permits initial digits/hyphens that declaration grammar excludes. | `O:320` |
| Asserts — CONFIRMED | `A:593`, `ClaimDeclaration.asserts`, observation reference | `B/feedback/claims.rs:23`, string; `:57`, sentence | Reference to an observation versus prose assertion. | `L:304` |
| Contradicted by — CONFIRMED | `A:597`, `contradictedBy`, negative observation reference | `B/feedback/claims.rs:24`; `:58`, sentence | Reference versus prose contradiction. | `L:304` |
| Promise / concern — PARTIAL | `.epr-meta/dev-system-equilibrium.habit.md:3` (`id`) and `:4` (`invariant`) | `A:587`, content `ClaimDeclaration` | Habit invariant and content claim have no shared tool-row pointer. | `C/feedback/genesis.rs:136`; `O:321` |
| Outcome — CONFIRMED | `R/feedback.rs:15`, `Outcome`: `"reported", "passed", "failed", "skipped", "unmeasured", "refused", "bridge-declared", "fallback-declared", "unsupported"` | — | none | `C/feedback/claim.rs:118`; `C/feedback/genesis.rs:123` |
| Severity — CONFIRMED | `elohim/epr/src/algedonic.rs:126`, `Severity`: `"info", "warn", "critical"` | `C/feedback/genesis.rs:86`: `"error", "warning", "info"`; `B/feedback/claims.rs:584`: `"trace", "debug", "info", "warn", "error", "fatal"`; `S/feedback-signals/algedonic-approach.schema.json:52` and `algedonic-breach.schema.json:46`: `"info", "warn", "critical"` | Three distinct lists, including `warning`/`warn` and `error`/`critical`. | `L:533` |
| Polarity — CONFIRMED | `S/enums/observation-polarity.schema.json:7`: `"positive", "negative"` | `B/feedback/claims.rs:581`: `"negative", "positive"`; `:595`, command/refusal assignments | Same values mean support/strain of a claim versus positive command/negative problem classification. | `A:628`; `L:340` |
| Weight — CONFIRMED | `B/feedback/claims.rs:30`, `weight: u8`; `:46`, default `1` | `L:308`, the design: what one occurrence costs the one who felt it, zero for a refusal that is working as designed | The design describes a cost that was felt; the code holds an integer the producer assigns. | — |
| Ceiling / reach — CONFIRMED | `S/enums/reach.schema.json:7`: `"private", "self", "intimate", "trusted", "familiar", "community", "public", "commons"` | `elohim/epr/src/reach.rs:18`, same list; `K:33`: `"agent-private", "household", "community", "commons", "commons-attested"`; `B/feedback/claims.rs:586`: `"device", "steward"`; `L:309`: `"device", "steward", "collective", "commons"` | Visibility rings, observation scopes, and graduation ceilings differ. | `L:542` |
| Validity horizon — CONFIRMED | `A:601`, string, ISO duration described, pattern `"^P"` | `B/feedback/claims.rs:25` at brit `a99143327`: `&'static str`, default `"P14D"` (`:41`), checked by `iso_day_duration` (`:601`). At brit `e33aa20d` the same field was a `u16` of days | Both are strings now; the SDK pattern only requires a leading `P`, while brit checks a day duration. | `elohim/sdk/README.md:69` |
| Supersedes — CONFIRMED | `B/feedback/claims.rs:26`, claim-id list | `.claude/epr-meta/policies.yaml:46`, nullable predecessor | Claim list versus policy predecessor. | `B/feedback/claims.rs:625`; `L:381` |
| Retire-when — CONFIRMED | `B/feedback/claims.rs:32`, optional string | `.epr-meta/dev-system-equilibrium.habit.md:29`, prose condition; `R/feedback_capacity.rs:18`, `resolved_days`; `L:312`, `"quiet:<ISO 8601 duration>"` | General condition, nullable string, and recovery-age days are different contracts. | `B/feedback/claims.rs:589` |
| Local cure — CONFIRMED | `C/device/arrival.rs:45`, `Check.cure` | `B/feedback/claims.rs:34`, optional cure string | none | `L:310` |
| Clears — CONFIRMED | `B/feedback/claims.rs:615`, accepted forms `"observed"`, `"release-current"`, `"path"`, `"release-at-least"`, `"bound"` | `L:311`, the same five forms, written with their parameters (`release-current:<tool>`, `release-at-least:<tool>@<version>`); brit's check requires the same parameters | none | `B/feedback/claims.rs:51` |
| Release cures — CONFIRMED | `B/feedback/claims.rs:404`, `Cure { claim, cite, summary }` | — | none | `B/feedback/claims.rs:415`; `L:314` |
| Producer declaration — CONFIRMED | `B/feedback/claims.rs:413`, `Manifest { claims, cures }` | `A:587`, `ClaimDeclaration`; `A:614`, `ObservationDeclaration`; `K:4`, `ObservationKindDeclaration` | Separate shapes declare overlapping vocabulary without a common schema. | `B/feedback.rs:30` |
| Emitter — CONFIRMED | `C/feedback/claim.rs:62`, 1–256 ASCII characters: `A-Z a-z 0-9 . _ : + -` | `C/feedback/spool.rs:47`, optional string | Spool field itself carries no grammar. | `C/feedback/claim.rs:147` |
| Producer — CONFIRMED | `R/feedback.rs:30`, `Source.producer`; `:102`, admitted `Policy.producers` | — | none | `R/feedback_store.rs:78` |
| Session key — CONFIRMED | `C/feedback/claim.rs:76`: `session-` or `invocation-` followed by a SHA-256 in hex; or, when the environment names no session, `day-` followed by a count of days | `B/feedback.rs:286`, same algorithm; `D/session.rs:15`, native session string | Feedback key is derived from, not identical to, native session identity. | `C/feedback/claim.rs:123`; `D/envelope.rs:26` |
| Run / item — CONFIRMED | `R/feedback.rs:29`, `Source { producer, run, item }` | `C/feedback/genesis.rs:54`, run-id CID; `C/feedback/claim.rs:129`, claim/outcome/emitter item | Report identity versus session identity. | `R/feedback_store.rs:122` |
| Participant — CONFIRMED | `R/feedback.rs:43`, optional reported participant; `:101`, policy participant | `D/envelope.rs:36`, roster human `steward` | Steward and reported participant are distinct; capture reads policy. | `C/feedback/claim.rs:140` |
| Actor id — CONFIRMED | `R/model.rs:25`, `AgentRef`, documented Holochain hash | `R/actor.rs:41`, `agent:`; `:47`, `human:`; `:53`, `service:`, which the comment there says a record may name as provenance and `parse_acting_participant` (`:321`) refuses wherever something claims, authors or attests | One string slot carries a hash identity and three repository attribution forms, one of which may be named and may not act. | `C/feedback/response.rs:48` |
| Recovery observer — CONFIRMED | `R/feedback.rs:90`, `EvidenceLink.observer` | `R/feedback_store.rs:232`, recovery requires matching participants, subject, producer, later different run | none | `C/feedback/mod.rs:165` |
| Service grant — PARTIAL | **NEW / NOT FOUND** in `D`, `R`, and CLI grant searches | `D/envelope.rs:49`, covenant-seal CID; `C/covenant.rs:525`, covenant clauses—not service grants | No traced definition supplies `detail.grant` authority. | `L:254` |
| Measure / bound — CONFIRMED | `C/flow/measures.rs:380`, `Bound`, pinned measure; its `soft` and `hard` watermarks at `:409` and `:411` | `R/model.rs:115`, commitment `Bound`, limit/unit/percentage; `elohim/epr/src/measure.rs:30`, `"level", "rate", "ratio"` | Measure watermarks and commitment limits use different shapes. | `.claude/epr-meta/measures.yaml:2128` |
| Graduation threshold/window — CONFIRMED | `K:35`, diversity counts; `:49`, seconds | `K:55`, policy: `null, "self-threshold", "diversity-threshold", "summarize"`, a sibling field of the same declaration | none: the counts, the window and the policy are three fields of one declaration, not competing definitions | `O:322` |
| Store limits — CONFIRMED | `R/feedback_capacity.rs:9`, `Parameters` | `R/feedback.rs:107`, legacy byte limits; `C/feedback/spool.rs:22`, batch `256` | Governed capacity, legacy ceilings, and spool limits are separate shapes. | `.epr-meta/feedback-capacity.json:2` |
| Retention policy — CONFIRMED | `.claude/epr-meta/policies.yaml:30`, `feedback-local-retention@1` | `K:29`: `"operational", "contextual", "archival", "attestation-feeding", "wisdom"` | Numeric eligibility policy versus retention classes. | `R/feedback_store/retention.rs:187` |
| Observation / one call — CONFIRMED | `R/feedback.rs:39`, `Occurrence` | `S/inputs/observation-intent.schema.json:7`, `ObservationIntent`; `elohim/elohim-storage/src/observation/wire.rs:21`, `Observation` | CLI outcome/source record versus kind/payload/log record. | `C/feedback/mod.rs:122`; `:130` |
| Time — CONFIRMED | `R/feedback.rs:41`, `observed_at` string | `S/inputs/observation-intent.schema.json:27`, epoch-seconds integer | RFC3339 versus epoch seconds. | `R/feedback_store.rs:185` |
| Subject / private detail — CONFIRMED | `R/feedback.rs:44`, subject; `:51`, message; `:53`, arbitrary JSON detail | `S/inputs/observation-intent.schema.json:19`, `subjectCid`; `:32`, stringified payload | Claim id, content address, and adapter payload are not interchangeable. | `C/feedback/claim.rs:141`; `L:260` |
| Term reading — PARTIAL | **NEW / NOT FOUND** in feedback modules/schema search | `L:373`, the design's nine readings: `open`, `stale-emitter`, `unrecorded`, `cure-available`, `on-watch`, `retired`, `reopened`, `review-due`, `superseded`; `R/feedback.rs:232`, `Experience`; `C/feedback/response.rs:152`, waiting/response rows, not those readings | No closed judge state or precedence implementation found. | `L:373`; `:383` |
| Summary states — PARTIAL | **NEW / NOT FOUND** in feedback modules/schema search | `L:342`: `"open", "cured", "retired", "reopened"` | No coded summary state definition found. | `L:352` |
| Summary class/counters — PARTIAL | **NEW / NOT FOUND** as a summary shape | `L:343`: `"fixture", "real"`; `:345`, sessions/days/dropped/bytes; `R/feedback.rs:237`, observed runs | Existing counters do not define the outbound summary. | `L:353` |
| Retire act — PARTIAL | **NEW / NOT FOUND** in CLI dispatch/schema search | `R/feedback_store/retention.rs:193`, reclaim eligibility—not retirement | Reclamation lacks the contract’s watch/note transition. | `O:341`; `L:392` |
| Propose act — CONFIRMED | `C/feedback/response.rs:25`, `propose`, writes `Intent` at `:46` | `R/feedback.rs:118`, response `"report", "propose", "repair"` | Response modes are not the contract’s three acts (`O:341`: retire; propose; ask, answer and release one sample). | `C/feedback/mod.rs:171` |
| Ask / answer / release sample — PARTIAL | **NEW / NOT FOUND** in feedback dispatch and schemas | `R/feedback.rs:113`, sharing boundary; `:151`, `"separately-authorized"` | Boundary exists; exchange records and acts were not found. | `O:343` |

## What the table shows

Many rows have more than one definition. Three of them are closed lists of values that differ
from one definition to the next, which is the kind of difference a schema cannot leave open:

- **Reach, or ceiling.** Eight visibility rings (`S/enums/reach.schema.json`, `elohim/epr/src/reach.rs`),
  five observation scopes (`K:33`), two ceilings in brit's table, and four rungs in the lifecycle
  design.
- **Severity.** Three levels in the algedonic code and its two signal schemas, three differently
  spelled names in the Genesis adapter, and six names in brit's table.
- **Polarity.** One list of two values with two meanings: whether an observation supports or
  strains a claim (the SDK), and whether a row is a command that worked or a problem met (brit).

Used by the designs and defined by no code or schema:

- The reading of a term (the nine readings listed in its row, and the order that decides which
  one holds).
- The summary that may leave a device (its states, its class and its counters).
- The grant a service acts under, the retire act, and the ask, the answer and the release of a
  sample.

Most other rows carry a smaller difference, each stated in its row. Three examples a schema
will have to settle: observation time is an RFC 3339 string in the
store and epoch seconds in the observation intent; the observation-kind name allows a leading
digit in the input schema and not in the declaration schema; the SDK's duration pattern accepts
any string that starts with `P`.

## Not read

Every row of brit's declaration, generated bindings, and the grant implementation proposed in the
carrier design, which is not on dev.
