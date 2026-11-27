---
id: feedback-contract-fc5-acts-and-authority
title: Feedback contract, FC-5 — each act, the command that performs it, and who may
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; acts read from the code by Codex (astra); citations opened by a Claude Haiku subagent; prose by a Claude scribe agent
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-5 this table delivers, and whose condition it finds unmet | sha256:c797ac99ca07cf87 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design that names most of the acts, cited in rows as L | sha256:bb8216eb2f736464 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 6 names the three acts of the contract, cited in rows as O | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc1-primitives-inventory | the sibling inventory whose last rows first found the retire and exchange acts missing | sha256:334e42fd83e95af4 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc1-primitives-inventory.md"
---

# FC-5: each act, the command that performs it, and who may

This is the deliverable of station FC-5 of the feedback loop contract plan: a table of every act
a person, a session or a service can take on a feedback term or on the store. Each row names
the command that performs the act today, the records it writes, what the code checks before
allowing it, and whether it requires a named actor. It records what exists. It proposes no
command, no name and no design.

## The finding

The plan's verify condition for this station is that no act is listed as performable without an
attributed actor. The code does not meet it. Nine of the eighteen built acts that write records
do so without requiring a named actor: capture observation, ingest report, author evidence, drain
gap notes, link commitment, repair, reclaim, migrate and qualify. An ordinary append to the
store can also trigger reclamation as a side effect (`G:439`), and append takes no actor.

Where an actor is required, the check is narrow. A missing `--as` argument is refused with
`"--as required"` (`M:289`), but the check tests only that the string is not empty. The function
`parse_acting_participant` (`R/actor.rs:321`), the one place that refuses a `service:` name in an
acting position, is called nowhere in the feedback code.

## How it was made, and by whom

The table was read from the code on 2026-11-27 by Codex (astra), read-only, in this repository at
commit `6cbea440c` (the code is that of `9d9322ca1`). A Claude Haiku agent that had not written
the table then opened about 120 of its code citations, found three that did not support their
cell, and those corrections are applied here. The designing session (shem session 3d494616, Claude
Fable 5.2) opened the design citations. Prose and layout are by a Claude scribe agent from those
sources.

CONFIRMED on a row means every cited line was opened and matched. PARTIAL means the act was not
fully traced through the code. Every line number is true of the commit it was read at and will
move as the files change.

## Path abbreviations

Code letters name directories relative to the checkout:

- `C` = `elohim/eprfs/epr-cli/src/feedback`; `M` = `C/mod.rs`
- `R` = `elohim/epr-rea/src`; `S` = `R/feedback_store.rs`
- `G` = `R/feedback_store/generation.rs`; `T` = `R/feedback_store/retention.rs`
- `D` = `elohim/eprfs/device-runtime/src`

Two letters name designs, not code: `L` = the feedback lifecycle design; `O` = its companion, one
loop for every producer. A row whose only citations are `L` or `O` describes a designed act with
no implementation.

## How to read a row

*Act / status* names the act, its design citation and whether it was CONFIRMED or PARTIAL.
*Command today* is the `epr feedback` subcommand and its dispatch line, or NOT BUILT if no
command exists. *Records written* names the record kinds the act creates and the store method
that writes them. *Enforced authority* is what the code checks before allowing the act, with
quoted refusal messages where the code has them. *Attributed actor* says whether the act requires
a named actor, with the cite that shows it. *Design-code difference* states where the design
asks for something the code does not do, or names an absence.

The dispatch at `M` accepts exactly eighteen verbs: qualify, measure, capacity, reclaim, migrate,
policy, capture, drain-spool, record, ingest-genesis, list, show, triage, group, evidence,
propose, repair and link. Any other verb reaches `M:179`, the `"unknown feedback command"` arm,
which is what every NOT BUILT row cites. The command's own help text (`M:26` to `M:57`) omits
`measure`.

Two counts in this document are both eighteen and are not the same list. There are eighteen
verbs. Twelve of them write, and because `evidence` performs seven different acts, one per
`--relation`, those twelve verbs make eighteen acts that write. The other six verbs only read.

Three terms in the *Enforced authority* column come from the repository's feedback policy, a
small file the store reads before every write. *Automatic capture* is a switch in it: a write
that no one deliberately authored is refused with `"policy refuses automatic capture"` when it is
off (`S:74`). *Deliberate evidence* is the matching switch for authored writes (`S:71`). An
*admitted producer* is a name on the policy's list of producers; an occurrence from any other
producer is refused (`S:78`). A *retained* record is one still in the store, and a *CID* is its
content address, the identifier `show` takes.

## Acts that write

| Act / status | Command today | Records written | Enforced authority | Attributed actor | Design-code difference |
|---|---|---|---|---|---|
| Capture observation `O:331` — CONFIRMED | `epr feedback capture`, `M:107` | Occurrence; `append` (`M:112`) | The store requires automatic capture and an admitted producer (`S:74–85`). | No. The record's participant is whichever one the repository's feedback policy names, which may be none (`C/claim.rs:140`). | The design accepts this for now: reading the participant from the session, or from a grant for an unattended service, waits on a read that does not exist yet (`L:251`). |
| Ingest report `O:331` — CONFIRMED | `epr feedback ingest-genesis`, `M:130` | Occurrences; `ingest` (`C/genesis.rs:169`) | Automatic capture and admitted producers (`S:74–85`). | No (`C/genesis.rs:94`). | Findings, concerns and run records carry no participant. |
| Author evidence `O:331` — CONFIRMED | `epr feedback record`, `M:122` | All four Feedback record kinds; `append_protected` (`M:128`) | The store refuses with `"policy refuses deliberate evidence"` (`S:72`); commitment existence is checked (`M:126`). | No `--as` argument. The Interpretation and EvidenceLink records it can carry hold an author or observer string that is checked only for being non-empty (`S:195`, `S:220`). | Deliberate invocation does not establish authorship. |
| Drain gap notes `L:619` — CONFIRMED | `epr feedback drain-spool`, `M:114` | Occurrence; `drain` (`C/spool.rs:212`); removes the note (`:214`) | Automatic capture and admitted producers (`S:74–85`). | No (`C/spool.rs:189`). | None with the design, which gives a drained record no participant. That is the station's condition unmet by design, not by omission. |
| Group `O:305` — CONFIRMED | `epr feedback group`, `M:143` | Interpretation; `append` (`M:155`) | The store refuses with `"interpretation needs evidence, author and summary"` (`S:196`). | Yes (`M:150`). | The code does not check that the author experienced the term (`O:305`). |
| Link attempt `L:197` — CONFIRMED | `epr feedback evidence --relation attempt`, `M:157` | EvidenceLink; `append` (`M:169`) | Deliberate policy and observer required (`S:71`, `S:220`). | Yes (`M:165`). | The code does not check that the actor has standing to link. |
| Link workaround `L:197` — CONFIRMED | `epr feedback evidence --relation workaround`, `M:157` | EvidenceLink; `append` (`M:169`) | Observer required (`S:220`). | Yes (`M:165`). | The code does not check that the actor has standing to link. |
| Link verified fix `L:197` — CONFIRMED | `epr feedback evidence --relation verified-fix`, `M:157` | EvidenceLink; `append` (`M:169`) | Observer required (`S:220`). | Yes (`M:165`). | Fix verification itself is not checked (`S:211`). |
| Link available release `L:413` — CONFIRMED | `epr feedback evidence --relation available-release`, `M:157` | EvidenceLink; `append` (`M:169`) | Observer required (`S:220`). | Yes (`M:165`). | Release availability itself is not checked (`S:211`). |
| Link recovery `L:435` — CONFIRMED | `epr feedback evidence --relation experienced-recovery`, `M:157` | EvidenceLink; `append` (`M:169`) | Both occurrences must name the observer as their participant and must have passed; they must share a subject and a producer and come from different runs; and the recovery must not be earlier than the original (`S:232`). | Yes (`S:220`). | The timestamp comparison permits equality (`S:241`). |
| Classify routine success `L:197` — CONFIRMED | `epr feedback evidence --relation routine-success`, `M:157` | EvidenceLink; `append` (`M:169`) | The link must point an occurrence at itself; that occurrence must have passed, must not come from the producer `human`, and must carry no message (`S:223`). An observer is required (`S:220`). | Yes (`S:220`). | None. |
| Protect evidence `L:440` — CONFIRMED | `epr feedback evidence --relation protected`, `M:157` | EvidenceLink; `append` (`M:169`) | Observer required (`S:220`). | Yes (`S:220`). | `record --protect` also protects evidence with no actor (`M:128`). |
| Propose response `O:342` — CONFIRMED | `epr feedback propose`, `M:171` | Intent; `propose` (`C/response.rs:46`, `:65`) | Response policy and a retained occurrence (`C/response.rs:31`, `:40`). | Yes (`C/response.rs:35`). | The required `--as` is not a registered-session check. |
| Link commitment `L:197` — CONFIRMED | `epr feedback link`, `M:173` | CommitmentLink; `append` (`M:177`) | An existing commitment (`C/response.rs:13`) and no conflicting link (`S:207`). | No (`M:177`). | The deliberate flag is not attribution. |
| Repair `L:197` — PARTIAL | `epr feedback repair`, `M:172` | CommitmentLink; `handoff` (`C/repair.rs:52`) | Repair policy, local contract and matching intent (`C/repair.rs:18`, `:23`, `:43`). | No before handoff (`C/repair.rs:52`). | The link precedes launcher admission; downstream is not traced. |
| Reclaim `L:435` — CONFIRMED | `epr feedback reclaim`, `M:100` | Receipt, checkpoint and generation; `receipt` (`G:374`), `commit` (`G:472`), signal (`:636`) | The capacity policy must list the permission `reclaim-eligible-feedback` (`R/feedback_capacity.rs:70`), and only groups old enough to be eligible are selected (`T:193`). | No (`G:271`). | Deletion the policy authorises is attributed to nobody. An ordinary append also starts a reclaim when the store nears its bound (`G:439`), so the same deletion can follow any write. |
| Migrate `L:197` — CONFIRMED | `epr feedback migrate`, `M:101` | Copied atoms and checkpoint; `migrate` (`G:335`) | The store refuses with `"migration requires --legacy-writers-quiesced"` (`G:299`). | No (`G:297`). | Quiescence is caller-asserted. |
| Qualify — CONFIRMED | `epr feedback qualify`, `M:70` | Fixture Occurrences and Intent; `qualify` (`C/qualify.rs:137`, `:156`) | Capture and proposal policy (`C/qualify.rs:93`, `:103`). | No. A fixture actor is supplied internally (`C/qualify.rs:161`). | Verb absent from designs. |

## Acts that read

None of these writes records. Attribution does not apply.

| Act / status | Command today | Records written | Enforced authority | Attributed actor | Design-code difference |
|---|---|---|---|---|---|
| Read experiences `L:197` — CONFIRMED | `epr feedback list`, `M:131` | None | Nothing actor-specific. | n/a | None. |
| Read record `L:197` — CONFIRMED | `epr feedback show`, `M:132` | None | A retained CID is required (`M:139`). | n/a | None. |
| Judge reading `L:368` — CONFIRMED | `epr feedback triage`, `M:142` | None | No actor required (`C/response.rs:78`). | n/a | Returns experiences and waiting reasons, not the nine readings the design describes (`C/response.rs:152`). |
| Inspect capacity `L:197` — CONFIRMED | `epr feedback capacity`, `M:93` | None | Policy validation (`M:77`). | n/a | Per-cure accounting is absent (not traced to a specific line). |
| Measure `L:199` — CONFIRMED | `epr feedback measure`, `M:80` | None | Closed measure names (`M:82`). | n/a | None. |
| Explain policy — CONFIRMED | `epr feedback policy`, `M:104` | None | Policy validation (`M:77`). | n/a | Verb absent from designs. |

## Acts the designs name that have no command

Eleven of these twelve rows are acts the designs describe and nothing performs. The twelfth,
widening a ceiling, is an act the design forbids; it is listed so that its absence reads as
intended. Each of these verbs reaches the `"unknown feedback command"` arm at `M:179`. None writes records,
none enforces authority, and none requires an actor. The lifecycle design gives one command for
both compact/begin watch and retire/archival note: `epr feedback retire --claim <id>` (`L:392`),
which performs whichever transition the reading proposes.

| Act / status | Command today | Records written | Enforced authority | Attributed actor | Design-code difference |
|---|---|---|---|---|---|
| Compact / begin watch `L:394` — CONFIRMED | Not built; `M:179` | None. Design intends: aggregate, interpretation and release link (`L:413`). | None. Design requires an attributed session and a new policy (`L:393`, `:439`). | No. | Existing reclamation removes records without building an aggregate (`G:282`). |
| Retire / archival note `O:341`; `L:435` — CONFIRMED | Not built; `M:179` | None. Design intends: a protected occurrence (`L:432`). | None. Design requires an attributed session, and a person ruling with `--as` where no machine-checkable condition exists (`L:392–398`). | No. | No watch-to-note transition exists. |
| Ask `L:481`, `:484` — PARTIAL | Not built; `M:179` | None. | None. Design requires asker standing (`L:481`). | No. | Exchange absent. |
| Answer `L:481` — PARTIAL | Not built; `M:179` | None. | None. Design requires clause or person (`L:481`). | No. | Exchange absent. |
| Release sample `O:343` — PARTIAL | Not built; `M:179` | None. | None. Design requires that the person sees exact bytes (`L:474`). | No. | Release and receipt absent. |
| Decline `L:474` — PARTIAL | Not built; `M:179` | None. | None. Design requires person decides (`L:474`). | No. | Refusal trace absent. |
| Share sibling summary `L:480` — PARTIAL | Not built; `M:179` | None. | None. Design requires standing consent (`L:480`). | No. | Summary transport absent. |
| Draft/send help call `L:482` — PARTIAL | Not built; `M:179` | None. | None. Design requires person or standing clause (`L:482`). | No. | Call absent. |
| Widen ceiling `L:542` — CONFIRMED | Not built; `M:179` | None. | None. The design says `"never widen"` (`L:542`). | No. | Not a permitted act in the design. |
| Establish/revoke sharing consent `L:500` — PARTIAL | Not built; `M:179` | None. | None. Design requires same person's clause (`L:500`). | No. | Generic covenant revocation exists (`C/../covenant.rs:549`) but feedback integration is absent. |
| Drop surface logs `L:455` — PARTIAL | Not built; `M:179` | None. | None. Design defers to a later adapter (`L:455`). | No. | Surface cleanup absent. |
| Run declared cure `L:718` — PARTIAL | Not built; `M:179` | None. | None. Design requires a steward ruling (`L:718`). | No. | Explicitly deferred. |

## What the table shows

Writes possible without an attributed actor, in the order the reader of the code ranked them,
most consequential first:

- Reclaim deletes policy-eligible groups and writes receipts with no acting attribution, including reclamation triggered as a side effect of an ordinary append (`G:439`). Migrate copies atoms and asserts quiescence with no actor (`G:297`).
- Commitment linking and repair's pre-admission link write without an actor.
- Author evidence (`record`), capture, ingestion and spool drain each write records with no attribution. `record --protect` protects evidence with no actor (`M:128`).
- Qualify writes disposable fixture records with an internally supplied actor.

Designed acts that have no command:

- Compaction and watch, archival retirement.
- Ask, answer, release of a sample, decline.
- Summary sharing, help calls, sharing consent and revocation, surface cleanup, declared-cure execution.
- Ceiling widening is prohibited by the design, not a missing permission.

Code verbs not named by either design document that was read:

- `policy` and `qualify`.

Two further facts from the table: the recovery link's timestamp comparison permits equality
(`S:241`), so a recovery observed at the same instant as the original occurrence is accepted. The
`record --protect` path protects evidence with no actor (`M:128`), a second route to the same
effect that `evidence --relation protected` performs with one.

## Not read

Application-plane implementations, brit's emitter, peer transport, what happens downstream of a
repair's hand-off, and most of the covenant machinery and of the device runtime. The checking
agent spot-checked rather than fully read `C/qualify.rs`, `G` (generation.rs), `T`
(retention.rs), `D/envelope.rs` and `elohim/eprfs/epr-cli/src/covenant.rs` outside the cited
ranges. Only source reads and searches were performed; no files
were changed, no builds, tests, reviews or experiments were run.
