---
id: feedback-contract-fc7-producer-checklist
title: Feedback contract, FC-7 — what a producer must do to be on the loop, with brit walked through it
date: 2026-11-27
status: OPEN
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), orchestrating; checklist and walk-throughs by Codex (astra); citations opened by a Claude Haiku subagent; prose by a Claude scribe agent
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-7 this checklist delivers, and whose condition it finds unmet | sha256:abae7be43a4c8fcf | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design most checklist lines come from, cited in rows as L | sha256:0190422f60b73c07 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose sections 4 and 6 say what a producer declares and calls, cited as O | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc5-acts-and-authority | the table of acts that shows which things a producer is asked for have no command yet | sha256:f243475a0de4973a | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc5-acts-and-authority.md"
  - "feedback-contract-fc9-observer-safeguards | the safeguards table whose three headings this checklist shares | sha256:36b9313a84e6fa30 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc9-observer-safeguards.md"
---

# FC-7: what a producer must do to be on the loop, with brit walked through it

This is the deliverable of station FC-7 of the feedback loop contract plan: one checklist of
what a producer must do, must not do and must leave to the person to be on the feedback loop as
the contract stands today. The checklist has 22 numbered lines under three headings. brit is
walked through it as the worked example, and one agent hook (`.claude/hooks/brit-first.py`,
which is not on the loop today) is walked through it to find where the contract has no place for
a producer that is a script. No brit release carrying this code has been published; the
walk-through is of brit's branch `run-ci/feedback-f1` at commit `a99143327`.

## The finding

The 22 lines fall under three headings: four lines state what a producer cannot do, sixteen
state what it must do, and two state what the person controls.

Of brit's 22 results, 14 are PASS, 3 are DIFFERS, 4 are NOT APPLICABLE and 1 is UNSETTLED.
PASS means the source supports it, not that a test ran; the walk-through is of source, not a
released binary. The three DIFFERS lines: on line 16, brit's skip marker leaves the outcome out
of its key for a claim that is not a command claim unless the outcome is `passed` (a command
claim is one that records that a brit command ran, such as `brit/command:status`; the others
name a problem or a refusal), so two different non-passed
outcomes would share one marker. This is latent, because no such pair exists in brit's code
today. On line 18, brit returns without a notice when the spool is full; this differs from the
companion's general principle that absence should be said (O:301; O:527), not from a requirement
of the lifecycle design's spool line (L:690), which says nothing about such a notice. On line
20, the spool is capped but the marker directory has no count, age or removal rule. The
UNSETTLED line is 5: a release that ships the declaration cannot be inspected because none has
been published. One of the four lines first marked failing, line 17, was a mistake in the
lifecycle design and not in brit; the design named the wrong directory and is corrected in the
same commit as this document.

The hook cannot meet three lines for want of an interface: line 13 (no contract says who admits
a script as a producer), line 14 (the contract supplies emitter identities for binaries and
executables, not for a script) and line 19 (the gap note drain hard-codes producer `brit` and no
script-neutral gap interface is specified).

Ten items across the checklist have no enforcement: lines 3, 4, 18, 20 and 22 are unchecked in
full, and one aspect each of lines 5 (publication), 7 (sentence truth), 10 (review), 16
(producer-side suppression) and 17 (location agreement) goes unchecked.

The station's condition is that brit passes every line against its landed release and that the
hook is walked through with no line that needs an interface the contract lacks. The condition is
not met: brit has not landed a release, and the hook meets three lines for which the contract
provides no interface.

## How it was made, and by whom

Codex (astra) derived the checklist and made both walk-throughs, read-only, in the repository at
commit `9f90baf09` and brit at `a99143327`. A Claude Haiku agent that had not written it opened
about 170 of the cited places and read brit's marker and spool functions in full; that agent
weakened or corrected all four of brit's lines originally marked failing. The designing session
(shem session 3d494616, Claude Fable 5.2) read brit's marker code and documentation itself for
line 17 and corrected the lifecycle design. Prose and layout are by a Claude scribe agent.

PASS means the source supports a line, not that a test ran.

## Citation letters

Citations name documents by a letter and a line number. The lifecycle design's line numbers are
those of the commit that adds this document, in which sections 5.5, 6, 7 and 12 were amended.

- **L** = `genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md`
- **O** = `genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md`
- **F1, F2, F4, F5, F9** = the named sprint documents under `genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/`
- **C, M, S** = `elohim/eprfs/epr-cli/src/feedback/{claim,mod,spool}.rs`
- **Store** = `elohim/epr-rea/src/feedback_store.rs`
- **B, D, K** = brit's `brit-cli/src/{feedback.rs,feedback/claims.rs,feedback/markers.rs}`
- **H** = `.claude/hooks/brit-first.py`
- **Schema** = sprint `fc2/producer-declaration.draft.schema.json`
- **Test** = `genesis/orchestrator/producer-declaration-schema.test.mjs`

## How to read a row

*#* is the line number. *Requirement and source* is what the contract asks, with the design line
that asks it. *Checked by* names the code or test that enforces the requirement, or says nothing
checks it. *Brit* says what the walk-through found, labelled PASS, DIFFERS, NOT APPLICABLE or
UNSETTLED. *Hook must* says what `.claude/hooks/brit-first.py` would have to do to meet the
line, or names the interface it cannot reach.

## What a producer cannot do

| # | Requirement and source | Checked by | Brit | Hook must |
|---|---|---|---|---|
| 1 | A producer must not derive IDs or parameters from personal input; IDs must be literal declared words (L:246–250). | C:42 checks grammar; D:439 checks membership. | **PASS**: lookup returns declared rows, and typed refusals map to constants (D:462–466, 507–539). | Map fixed refusal categories and never use command text; the hook's current messages interpolate input (H:601–610). |
| 2 | A producer must not automatically capture narrative into occurrences (Store:87–91; M:35). | Store:87. | **PASS**: fixed arguments and claim/emitter environment, null stdin (B:210–232). Spool diagnostics are the expressly allowed exception (L:690). | Keep command and fallback prose out of capture (H:636, 649). |
| 3 | A producer must not treat severity or a declared ceiling as permission to disclose (L:575, 613). | NOTHING CHECKS THIS in a sharing implementation (F9:105). | **NOT APPLICABLE**: the adapter calls local capture, not export (B:210–232). | Preserve this restriction when reporting; current output is hook context (H:670–671). |
| 4 | A producer must not generate surveillance profiles or serve institutional power over individuals (O:531). | NOTHING CHECKS THIS (F9:93–94). | **NOT APPLICABLE** to the local capture path examined (B:191–291). | Report fixed observations without adding profiling; the hook's current operation is command steering (H:628–650). |

## What a producer must do

| # | Requirement and source | Checked by | Brit | Hook must |
|---|---|---|---|---|
| 5 | Ship a closed declaration with `claims` and `cures` (L:314, 329–340). | Schema:7–24; D:676. Publication: NOTHING CHECKS THIS among checks read. | **UNSETTLED** for an actual release. Packaging generates the file (`scripts/ci/package-brit-release.sh:121`); settle by inspecting the published archive. No brit release carrying this code has been published. | Declare repository terms at `.epr-meta/feedback-claims.json` (L:335), which does not exist in the repository today; H:27 is only a capability table. |
| 6 | Preserve IDs across releases and pin their full set (L:248–250). | D:689–768. | **PASS**: literal snapshot and declared lookup (D:439, 691). | Pin stable rule IDs and replace the independently maintained capability table as FB-7 requires (H:21–27; plan:155–159). |
| 7 | Give each row assertion, contradiction, duration, predecessors, polarity, severity, weight, ceiling, clearing conditions and retirement condition; cure where available (L:319–327). | Schema:37–132; Test:16–31. Sentence truth: NOTHING CHECKS THIS (F2:128). | **PASS** for shape: D:21–49 and literal rows D:104–400. | Supply these declarations; H:27–74 has only capability/form tuples. |
| 8 | Use the six standard severity names (L:601). | Schema:92–103; D:595. | **PASS**: defaults and macros use `warn` and `info` (D:45, 60, 85). | Assign standard names to declared terms; existing output has none (H:649–650). |
| 9 | Distinguish intentional refusals with weight zero and no `clears`; use a command denominator of zero or device (L:692–693). | D:610–620. | **PASS**: D:53–87. | Distinguish intended enforcement from hook malfunction; denial currently has no feedback declaration (H:650). |
| 10 | A release cure cites a declared claim, root-cause evidence and a reviewed sentence (L:329–331). | D:650; Test:28. Review: NOTHING CHECKS THIS. | **NOT APPLICABLE**: cures are empty (D:410). | Carry cures when applicable; none exists in H:27–74. |
| 11 | Observe a cured form passing, rather than equating release availability with recovery (L:326; O:296). | D:659; B:396. | **PASS** for the implemented mechanism: successful mapped forms emit `passed` (B:70–96). | Report observed passage separately from availability; the hook's current version comparison only selects wording (H:572–583). |
| 12 | Call capture with declared term, emitter, producer and outcome (O:328–331; L:686). | C:109–118; M:107–112. | **PASS**: B:210–228. | Add the recorder call; H:628–672 only returns and prints hook output. |
| 13 | Use an admitted reporting producer, distinct from what the claim concerns (L:251, 261). | Store:78. | **PASS**: B:215; `.epr-meta/feedback-policy.json:5`. | INTERFACE GAP: choose an authorized producer identity. `brit-guard` is admitted, but no cited contract binds this hook to it or specifies admission authority (F5:83–88; H:628–650). |
| 14 | Identify the emitter using the allowed address grammar; brit uses launcher identity or its documented fallback (L:268, 687). | C:65 drops invalid values. | **PASS** for normal supplied and fallback identities (K:192–203). | INTERFACE GAP: the contract supplies binary release and executable identities, not this script's identity (L:687). Omission is accepted but loses identity (C:128–131). |
| 15 | Share the specified session key, including invocation scope/day fallback (L:283–288, 689). | C:76, 222; brit implementation B:349. | **PASS**: B:349–384 matches C:76–99. | Preserve inherited session environment; capture computes the key. The hook currently derives none (H:628–672). |
| 16 | Retain one observation per producer, claim, outcome and emitter per session (L:256, 290–291). | C:129–144 constructs identity; no check catches producer-side suppression. | **DIFFERS**, latent: the skip marker leaves the outcome out of its key for a non-command claim unless the outcome is `passed` (K:159–170), so two different non-passed outcomes would share one marker and the second would be skipped before the recorder starts (B:101–104). No such pair exists in brit today. | Keep outcomes distinct; the hook currently has no capture suppression (H:628–672). |
| 17 | Check the repeat marker before spawning, at the agreed notice location (L:688). | B:101; location agreement: NOTHING CHECKS THIS. | **PASS**: the skip marker is under `$XDG_CACHE_HOME/brit/feedback-markers/` (K:156; docs/feedback.md:61–63), which is what the lifecycle design now names (L:688). The design had named `feedback-notices/`, a different directory; the design is corrected in the same commit as this document. | Deduplicate reporting, not enforcement; the brit-specific directory is not applicable (H:650). |
| 18 | Say when observation failed; do not silently read absence as health (O:301, 527). | NOTHING CHECKS THIS end-to-end. | **DIFFERS** from a principle: when the spool is full, brit returns with no notice (B:144–169; docs/feedback.md:101–104). The lifecycle design's spool line (L:690) says nothing about a notice when the spool is full. The companion declares the general principle (O:301; O:527). | Report capture failure separately from the hook's existing parser-coverage message (H:669). |
| 19 | Keep bounded, atomic gap notes; distinguish refusal from an unavailable recorder; leave draining to epr (L:690). | K:255–319; S:105–162, 205. | **PASS**: B:123–170; K:270–319. | INTERFACE GAP: the current drain hard-codes producer `brit` (S:181); no script-neutral gap interface is specified. |
| 20 | Declare limits for every store kept (O:324, 420–435). | NOTHING CHECKS THIS. | **DIFFERS**: the spool is capped (K:16–17) but the marker directory has no count, age or removal rule anywhere in markers.rs or feedback.rs; spool-full and lock files live in the same directory. | Any reporting cache or spool needs limits; the hook currently only prints (H:670–672). |

## What the person controls

| # | Requirement and source | Checked by | Brit | Hook must |
|---|---|---|---|---|
| 21 | Whether automatic capture is permitted by policy (Store:74–75). | Store:74. | **PASS**: uses capture's non-deliberate append (B:212–215; M:112). | Use the same policy-governed path; hook enablement is a different switch (H:629–631). |
| 22 | Standing sharing consent must be the person's statement; an agent cannot author it (L:571). | NOTHING CHECKS THIS in feedback integration (F5:147). | **NOT APPLICABLE**: no consent-writing path in the adapter (B:191–291). | Leave consent authorship to the person; the hook currently writes no clause (H:628–672). |

## Asked of a producer with no interface yet

These are excluded from the checklist proper and not scored as producer failures.

- **G1.** Promise/concern linkage and burden reporting: the optional tool-row pointer remains proposed, and the burden probe is unbuilt (O:321–322; F1:67; plan:125–129). Brit and the hook lack the shared interface.
- **G2.** Read the judge, retire, compact, watch, archive and clean surface logs (O:336–341; F4:31–33; F5:138–139, 148). Neither producer can complete these.
- **G3.** Consent-controlled summaries, exact-byte sample release and decline, asks, answers, receipts and withdrawal; exported class and closed-field checking (L:365–367, 545–575; F5:140–147). Neither has the exchange interface.
- **G4.** Session or service attribution beyond policy participant (L:266–269; F5:95). Both inherit this missing read.
- **G5.** The seven unanswered safeguards of FC-9: harassment, retention consent, operational proof, observation control, downstream use, access and requested deletion (F9:35–45). Neither producer can satisfy an unspecified contract.

## What the tables show

**(a)** Ten items across the checklist have no enforcement. Five lines are unchecked in full: 3, 4, 18, 20 and 22. Five lines have one unchecked aspect: publication in 5, sentence truth in 7, review in 10, producer-side suppression in 16 and location agreement in 17.

**(b)** Brit has three DIFFERS lines (16, 18, 20) and one UNSETTLED line (5). The published artifact inspection that would settle line 5 cannot be performed because no release carrying this code has been published. One line first marked failing (17) was a mistake in the lifecycle design and is corrected in the same commit.

**(c)** The hook has interface gaps at lines 13, 14 and 19, plus G1 through G5. The declaration location for a repository's own terms is specified at `.epr-meta/feedback-claims.json` (L:335); the file does not exist in the repository today.

## For the brit lane

These are the three differences the walk-through found, set beside what each differs from. The
"seam" is what the two sides wrote down that brit and epr each do: section 12 of the lifecycle
design.

- Line 16: brit's skip marker leaves the outcome out of its key for a claim that is not a command claim, unless the outcome is `passed` (K:159–170). The design asks for one record per producer, claim, outcome and emitter per session (L:256; L:290–291). The difference is latent: no call site in brit today gives a non-command claim two different non-passed outcomes.
- Line 18: when the spool is full, brit returns with no notice (B:144–169; docs/feedback.md:101–104). The companion declares that absence should be said and unobserved should not be read as healthy (O:301; O:527). The lifecycle design's spool line (L:690) says nothing about a notice when the spool is full, so this differs from a principle of the companion and not from the seam as that line writes it.
- Line 20: the spool is capped (K:16–17), but the marker directory has no count, age or removal rule (markers.rs, feedback.rs). The companion says a store should be born with its limits (O:298; O:431).

## What the brit lane answered

The three differences above went to the brit lane through the shem integrator on 2026-11-27, and
it answered the same day, for its release 0.3.12. What follows is brit's own statement of what
it will do, as relayed; none of it is built or checked here.

- Line 20, and line 18's notice: `feedback-markers/` and `feedback-notices/` each get a bound.
  At 4,096 entries brit removes files older than 7 days and never lock files, and when the
  directory is still full it skips writing, which costs a repeated recorder spawn or a repeated
  gap line and not a record. The limits go into brit's `docs/feedback.md` beside the spool's.
- Line 16 was not raised as a change: it is latent, and the store's own rule keeps one record per
  session.
- Line 5 and line 10 gain something the checklist did not ask for: from 0.3.12 the declaration's
  `cures` list is cumulative, each entry `{claim, release, state, cite, summary}` with the state
  `cured` or `withdrawn`, so that a device which skips a release still sees the fix it carried
  (see `fc2-declaration-draft.md`).

The walk-through above is of brit at `a99143327`, before any of this.

## Not read

The published brit archive was not read; no brit release carrying this code has been published.
The lifecycle design's lines 545–568 were not read in full. Most of the FC-1 primitives table
was not read. Brit's docs/feedback.md outside lines 38–110 was not read. Brit's transitive
callers were not read. No build or test was run. The checking agent did not confirm the brit
checkout's commit; the designing session did, having built that checkout, whose binary reports
source HEAD `a99143327`.
