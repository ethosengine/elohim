---
id: feedback-contract-fc9-observer-safeguards
title: Feedback contract, FC-9 — the Observer Protocol's safeguards, line by line
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; mapping by Codex (astra); citations opened by a Claude Haiku subagent; prose by a Claude scribe agent
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-9 this table delivers, and whose condition seven rows do not meet | sha256:a7d1d8a98839c206 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "observer-protocol | the epic whose Part VIII lists the eighteen safeguards this table goes through, cited as OP | sha256:78afb76e1fb58772 | path: genesis/docs/content/elohim-protocol/observer-protocol.md"
  - "feedback-as-a-lifecycle-item-design | the design searched for an answer to each safeguard, cited in rows as L | sha256:4f57fdff063a4d6c | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 1b sets the feedback loop beside the Observer Protocol, cited as C | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc5-acts-and-authority | the table of what is built, on which every Built today cell rests, cited as F | sha256:f243475a0de4973a | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc5-acts-and-authority.md"
---

# FC-9: the Observer Protocol's safeguards, line by line

This is the deliverable of station FC-9 of the feedback loop contract plan: a table with a row
for every line of the Observer Protocol's Part VIII ("The Constitutional Safeguards"), saying
whether the line applies to a tool's feedback, what the feedback designs answer, whether that
answer is built, and what gap remains. The comparison is made because the Observer Protocol is
the protocol's statement of how a person may be observed, and a tool's feedback record can carry
a person's words and actions: the store holds the participant, their message, what the feedback
is about, what was judged, and the reporting artifact's identity (L:251–258).

## The finding

The plan's condition for this station is that each line is either answered with a cited section
or said not to apply with a reason. Of the eighteen lines, four are answered and partly built
(OP:243, OP:251, OP:255, OP:262). Six are answered on paper with nothing built (OP:244, OP:246,
OP:247, OP:252, OP:253, OP:260). One does not apply to a tool's feedback (OP:245). Seven have
no answer in the designs:

- "Enable stalking or harassment" (OP:248)
- "Preserve evidence only with consent or emergency" (OP:254)
- "Maintain cryptographic proof of operation" (OP:256)
- "When observation happens (physical switches)" (OP:259)
- "How patterns get used (participation optional)" (OP:261)
- "Access to their own data (always available)" (OP:263)
- "Deletion rights (forgotten when requested)" (OP:264)

Two of the seven, OP:259 and OP:263, have something built that serves them in part (a policy
switch; the `list` and `show` commands) with no passage in either design that states them as a
person's control or right.

This document records those seven gaps and answers none of them. Answering them is design work
that has not been done, and where an answer would be a decision about what a person is owed, it
is the steward's to make.

## How it was made, and by whom

Codex (astra) read the four documents, read-only, in the repository at commit `d5d889317`. A
Claude Haiku agent that had not written the table opened about 95 of its citations and
re-searched the designs for each row marked no answer found. That agent found one answer the
table had missed (C:531, the companion's declared limit on surveillance profiles) and one "built"
cell that contradicted its own citation. The designing session (shem session 3d494616, Claude
Fable 5.2) opened the two citations that agent flagged. Prose and layout are by a Claude scribe
agent.

The "built today?" column rests on the designs' own account and on the FC-5 table, not on a
fresh reading of the code.

## Citation letters

Citations name documents by a letter and a line number at commit `d5d889317`. The lifecycle
design's sections 5.5 and 6 were amended after that commit, so its line numbers from about 340
on have since moved down; the section named beside a citation is the surer guide.

- **OP** = `genesis/docs/content/elohim-protocol/observer-protocol.md`
- **L** = `genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md`
- **C** = `genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md`
- **F** = `genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc5-acts-and-authority.md`

## How to read a row

*The safeguard* is the Observer Protocol's own words and line number. *Applies to a tool's
feedback?* says whether the safeguard is relevant, and in what form, citing the companion's
mapping. *The design's answer* names the section and line of the lifecycle design or its
companion that addresses the safeguard, or says no answer was found and what was searched.
*Built today?* says whether the answer is implemented, citing the FC-5 table. *Gap* names what
is missing.

## What observers cannot do

This is Part VIII's first list (OP:242), headed "Immutable."

| The safeguard | Applies to a tool's feedback? | The design's answer | Built today? | Gap |
|---|---|---|---|---|
| "Store video beyond 3-second processing window" (OP:243) | Yes, in changed form. Tool records use governed retention instead of seconds (C:136). | Compact automatic records after cure; retire after recovery and watch (L:413–441). | Partly. Retention exists (L:198); compaction does not (F:138). | No implemented cure-to-summary transition (F:138). |
| "Share data without explicit consent" (OP:244) | Yes (OP:244; L:474). | A person releases exact bytes or declines; standing consent must be their own statement (L:474,500). The companion declares this as a limit (C:531). | No. Release and feedback consent integration absent (F:142,147). | Consent-controlled sharing is unbuilt (F:142–147). |
| "Override physical privacy switches" (OP:245) | Does not apply. Feedback sensors are errors, hooks, gates and tests, not cameras (C:113). The Observer Protocol's own line reads: "These switches are physical, mechanical—no software can override them" (OP:45). The software counterpart, a policy switch, is treated under OP:259. | n/a | n/a | n/a |
| "Serve institutional power over individuals" (OP:246) | Yes. Collection must remain the person's choice (L:481). | The agent may gather and draft, but the person decides; the collector needs standing and house authorization (L:139,481). The companion declares this as a limit (C:531). | No. Ask and answer authority absent (F:140–141). | Institutional collection constraints remain on paper (F:140–141). |
| "Generate surveillance profiles" (OP:247) | Yes. Machine records risk exposing personal activity (C:132). | Declared as a limit in the companion, section 9, item 8: the system "must not become what the Observer Protocol forbids an observer to be: something that shares 'without explicit consent', serves 'institutional power over individuals', or generates 'surveillance profiles'" (C:531). Nothing in either design or in the code enforces it. | No. Read access has nothing actor-specific (F:120–121). | The prohibition is a declaration with no enforcement. |
| "Enable stalking or harassment" (OP:248) | Yes. Tool records can expose what someone worked on (C:132). | No answer found. Disclosure safeguards do not address harassment or stalking (L:490–504,657–663,711–738). | No. Documented read controls are not actor-specific (F:120–121). | No prevention rule for misuse of retained personal information (L:251–258; F:120–121). |

## What observers must do

This is Part VIII's second list (OP:250), headed "Required."

| The safeguard | Applies to a tool's feedback? | The design's answer | Built today? | Gap |
|---|---|---|---|---|
| "Process locally on user-controlled hardware" (OP:251) | Yes (C:125). | The device keeps observations privately; the judge runs there at read time (L:100–103,368–369). | Partly. The private store exists, but triage lacks the designed readings (L:197; F:122). | The full local judge remains unbuilt (F:122). |
| "Destroy visual data after story extraction" (OP:252) | Yes, in changed form. Interpreted raw logs replace visual frames (C:166). | After compaction, no device-record text survives; only automatic records qualify (L:416,441). | No. Compaction and surface cleanup absent (F:138,148). | Authored records remain; surface logs await an adapter (L:441–445). |
| "Respect graduated privacy by constitutional layer" (OP:253) | Yes, in changed form. Feedback uses rungs, whose correspondence remains unreconciled (C:137). | A ceiling caps graduation; a house's act can narrow it; severity cannot widen it (L:309,461–465,542). | No. Rungs and consent have open stations (L:203). | Rung-based crossing controls remain unbuilt (L:203; F:140–147). |
| "Preserve evidence only with consent or emergency" (OP:254) | Yes, in changed form. Tools have no imminent-personal-danger exception (C:132). | No answer found. Section 8's consent governs departure, not retention (L:241–258,440–441,474–504). | No. `record --protect` requires no actor (F:106). | Protection can preserve evidence without establishing the affected person's consent (F:97,106). |
| "Generate REA valueflows transparently" (OP:255) | Yes, in changed form. Feedback yields fixes; value recognition stays with mishpat (C:138). | Measured drain remains available for later settlement; intents, commitments and cure events provide REA joins (L:519–526,740–746). | Partly. Proposal and commitment linking exist (F:107–108). | Drain accounting and the composed work view remain absent (F:123; L:741). |
| "Maintain cryptographic proof of operation" (OP:256) | Yes (C:128). | No answer found. Content addresses and proposed outcome attestations do not constitute cryptographic operational proof (L:417,646–648). | No. Authored evidence checks strings, not cryptographic authorship (F:97). | Occurrences live in an unsigned private file (C:135). |

## What humans control

This is Part VIII's third list (OP:258), headed "Sovereign."

| The safeguard | Applies to a tool's feedback? | The design's answer | Built today? | Gap |
|---|---|---|---|---|
| "When observation happens (physical switches)" (OP:259) | Yes, in changed form. Tool observation uses software sensors (C:113). | No answer found. Per-session counting is not an observation switch (L:275–285). | Partly. Policy can disable automatic capture or deliberate evidence (F:83–88). | Software policy exists, but there is no specified human control over its changes (F:83–88; OP:45). |
| "What stories get shared (consent required)" (OP:260) | Yes (L:474). | Personal fields excluded from summaries; the person chooses exact evidence bytes to release (L:347–349,474–488). | No. Sample release and consent integration absent (F:142,147). | Designed selection and consent are unbuilt (F:142,147). |
| "How patterns get used (participation optional)" (OP:261) | Yes. Shared counts describe a household's experience (L:657–659). | No answer found. The nearest passages limit aggregation and rule changes (C:524–526), not what a recipient does with a shared pattern. Sharing permission and settlement limits do not establish that control (L:500,523–526,657–663). | No. Exchange and consent integration absent (F:140–147). | Permission to disclose does not specify permitted downstream uses (L:484–500). |
| "Whether to participate (can withdraw anytime)" (OP:262) | Yes (OP:262; L:500). | Standing sharing consent expires and can be revoked by the same person (L:500). | Partly. Capture can be disabled; feedback revocation integration absent (F:83–88,147). | Sharing withdrawal does not cover all recording; released evidence has no promised take-back (L:488,500). |
| "Access to their own data (always available)" (OP:263) | Yes. The store holds personal fields including participant and message (L:251–258). | No answer found. Listing read verbs does not guarantee continuous personal access (L:197–198). | Partly. `list` and `show` read retained records (F:120–121). | Access covers retained CIDs, not an unconditional availability guarantee (F:121). |
| "Deletion rights (forgotten when requested)" (OP:264) | Yes. The store holds personal fields including participant and message (L:251–258). | No answer found. The design affirmatively keeps records: "A record a person wrote, and a record tied to a commitment, are kept as they are" (L:441), and protected evidence may not be dropped (L:440). There is no provision for deletion on request. | No. Reclamation selects policy-eligible groups, not person-requested deletion (F:110). | Retention eligibility does not provide deletion on request (F:110; L:440–441). |

## What the tables show

The eighteen rows fall into four groups.

**Answered and partly built** (four): "Store video beyond 3-second processing window" (OP:243),
"Process locally on user-controlled hardware" (OP:251), "Generate REA valueflows transparently"
(OP:255), "Whether to participate (can withdraw anytime)" (OP:262).

**Answered on paper, not built** (six): "Share data without explicit consent" (OP:244), "Serve
institutional power over individuals" (OP:246), "Generate surveillance profiles" (OP:247),
"Destroy visual data after story extraction" (OP:252), "Respect graduated privacy by
constitutional layer" (OP:253), "What stories get shared (consent required)" (OP:260).

**Does not apply to a tool's feedback** (one): "Override physical privacy switches" (OP:245).

**No answer found** (seven): "Enable stalking or harassment" (OP:248), "Preserve evidence only
with consent or emergency" (OP:254), "Maintain cryptographic proof of operation" (OP:256), "When
observation happens (physical switches)" (OP:259), "How patterns get used (participation
optional)" (OP:261), "Access to their own data (always available)" (OP:263), "Deletion rights
(forgotten when requested)" (OP:264).

## Not read

The checking agent did not open the code citations inside FC-5, nor Observer Protocol lines
outside 240–272 apart from OP:45, which the designing session opened. No files were changed; no
builds, tests, reviews or experiments were run.
