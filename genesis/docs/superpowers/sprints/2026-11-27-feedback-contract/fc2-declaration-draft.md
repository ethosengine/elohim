---
id: feedback-contract-fc2-declaration-draft
title: Feedback contract, FC-2 — a draft schema for the declaration a producer ships
date: 2026-11-27
status: DRAFT
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), called the designing session below, orchestrating and checking; schema, script and hand-written fixtures by Codex (astra), the schema then changed in two places by the designing session
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-2 this draft answers | sha256:295822228cd72766 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design whose section 5.4 lists the declaration fields the schema closes | sha256:20f22882feea7ab4 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 12 holds the open decision on where this schema lives | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc1-primitives-inventory | the inventory that found reach and polarity mean something else in the SDK, which is why this draft does not reference them | sha256:334e42fd83e95af4 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc1-primitives-inventory.md"
---

# FC-2: a draft schema for the declaration a producer ships

This is the draft deliverable of station FC-2 of the feedback loop contract plan. A producer (a
tool, an app, a peer) that joins the feedback loop ships one file with its release that lists the
problems and commands it can report, each under a fixed id. brit is the first producer to ship
one. This directory holds a JSON Schema for that file and the files it was checked with.

It is a draft and it is not in the SDK. Where the schema lives is the steward's decision (the
companion design, section 12, decision 2: in the SDK now, or after a second producer has used the
call), and he has not ruled. Nothing reads this schema yet.

## What is here

| File | What it is |
|---|---|
| `fc2/producer-declaration.draft.schema.json` | The schema: a file is `{"claims": [...], "cures": [...]}`, with no other key at any level |
| `fc2/fixtures/accepted/` | Three small valid files written by hand, and `brit-0.3.10-a99143327.claims.json`, brit's own output |
| `fc2/fixtures/refused/` | Thirty-two files, each wrong in exactly one way, named for it |

The checks are a test, `genesis/orchestrator/producer-declaration-schema.test.mjs`, which the
orchestrator's gate runs at every landing that touches the test or this `fc2/` directory. It is
kept there, and not beside the schema, because no gate runs a script kept in a documents
directory. It requires every accepted file to pass, brit's file to keep its recorded SHA-256, and
each refused file to be refused once, for the reason its name states. A refused fixture's file
name is a plain description of its one fault (`severity-outside-list.json`, `duplicate-id.json`);
the test holds a table from each file name to the schema keyword and the place in the file that
must be what refuses it, and a fixture with no entry in that table fails. The test moves with the
schema when the schema's home is ruled.

To check the schema without the test: any validator for JSON Schema 2020-12 accepts the four
files in `fc2/fixtures/accepted/` and refuses twenty-eight of the thirty-two in
`fc2/fixtures/refused/`. The other four (`duplicate-id.json`, `supersedes-undeclared-id.json`,
`cure-undeclared-id.json`, `cure-duplicate-claim-release.json`) pass the schema and are refused
only by the four rules named below,
which the test carries.

The first version of these checks was a script run by hand, before the test existed. The Evidence
section describes those runs.

## What the schema decides, and on whose word

These were set by the designing session as working rules for a draft. None is a ruling of the
steward's.

- **It describes the file brit writes, exactly.** Keys are spelled as brit serialises them.
  `retireWhen` is always present and may be `null`; `cure` is left out when there is none and is
  never `null`.
- **Its closed lists are the lifecycle design's, which are wider than brit's use of them.**
  `severity` has the six standard logging names (`trace`, `debug`, `info`, `warn`, `error`,
  `fatal`), `polarity` two (`negative`, `positive`), `ceiling` four (`device`, `steward`,
  `collective`, `commons`). brit's code allows two of the four, `device` and `steward`, and all 72 rows it ships today say `device`. A ceiling is the furthest
  a summary of the claim may travel: no further than the device, the steward's own devices, a
  collective, or the commons.
- **It does not point at the SDK's existing lists for reach or polarity.** Station FC-1 found
  they mean something else: reach is eight visibility rings, and the SDK's polarity says whether
  an observation supports or strains a claim. Whether the feedback contract should share a list
  with either is an open difference, recorded in FC-1's table, and this draft does not settle it.
- **A claim id follows the grammar that `epr feedback capture` enforces** (`epr` is this
  repository's command-line tool; `capture` is the verb that stores one observation): `<producer>/<name>[:<param>]`,
  lower-case letters, digits and hyphens, a parameter that may also hold a full stop, at most 128
  bytes. The same rule applies to ids inside `supersedes` and `cures`.
- **`validityHorizon` is an ISO 8601 duration.** `P14D`, `PT1H`, `P2W` and `P1Y2M3DT4H5M6S` pass;
  `P`, `14D` and free text do not. The SDK's neighbouring schema only asks for a leading `P`.
- **`clears` is one of five forms**: `observed`, `release-current:<tool>`,
  `release-at-least:<tool>@<version>`, `path:<path>` and `bound:<measure>@<version>`. The four
  that take a parameter are refused without it. A `clears` entry is a condition under which the
  problem counts as cured on a device, such as running a release at least as new as a named one.
  `retireWhen` is `null` or `quiet:<duration>`: how long the problem must stay gone before it is
  called gone.
- **Four rules a JSON Schema cannot state are in the test, each under its own name**: ids are
  unique (`uniqueClaimIds`), every superseded id is declared (`declaredSupersedes`), every
  cure entry names a declared id (`declaredCureClaims`), and a claim has at most one cure entry
  per release (`uniqueCureEntries`).
- **A `cures` entry has five required fields**: `claim`, `release`, `state`, `cite` and
  `summary`. `release` is the producer's version for the release that made the entry, in
  semantic-versioning form without build metadata, since two versions that differ only in build
  metadata cannot be ordered. `state` is `cured` or `withdrawn`. `cite` and `summary` may not be
  empty. The list is cumulative: a release's file carries every earlier release's entries and its
  own. This is the shape brit fixed for its release 0.3.12, in place of a list of what one
  release handled, so that a device which skips a release still sees the fix it carried. A
  schema for one file cannot check that a later release kept an earlier one's entries; brit pins
  its own list with a test, as it does its claim ids.
- **`reopened` is not a state a producer writes.** It is what a device reads from what it
  observes. A producer that takes back its own claim to have fixed something writes
  `withdrawn`. A file with `"state": "reopened"` is refused.

## Evidence

Everything below was done on 2026-11-27, on shem, one of the steward's devices (a development
workspace).

- brit 0.3.10 was built from its branch `run-ci/feedback-f1` at `a99143327`, into its own
  cargo slot. `brit feedback claims --json` printed 72 claims and no cures, 23,289 bytes, SHA-256
  `b30fd3017fb24bf8eedd602d17b93a21f7309a387aee87f40315d5845308d5ea`. That output is the fixture
  named above, unchanged.
- The schema, the script and the hand-written fixtures were written by Codex (astra) from brit's
  source, before brit's real output existed on the device.
- The designing session then ran the script itself: brit's real output passes; all fixtures pass
  (accepted files accepted, each refused file refused once, for the keyword and path its name
  states). It also made twelve single changes to brit's real output (an unknown field, a severity
  of `warning`, a missing `retireWhen`, an id with a capital letter, an id with a trailing
  newline, a duplicate id, a cure for an undeclared id, and others); each was refused for the
  expected reason. Those twelve are not kept as fixtures.
- The designing session changed two things in Codex's schema: the end-of-string anchors, to the
  `$` the SDK's schemas use, and the wording of one comment. The checks above were run after
  those changes.

## The change of 2026-11-27 to `cures`

The first version of this draft took a `cures` entry as brit 0.3.10's code defines it: `claim`,
`cite`, `summary`. The scripted lives of station FC-4 then showed that a device which skips the
release that listed a fix never sees it. The question went to the brit lane through the shem
integrator, with two ways to close it. Brit chose a cumulative list and fixed the five-field
shape above for 0.3.12, took this design's request to name the second state `withdrawn`, and
will state in its documentation that releases are ordered by semantic-version precedence and that
a claim's state is that of its latest entry in the head release's declaration. The schema, eight
new refused fixtures and one new rule in the test follow that shape. brit 0.3.10's own file is
unchanged and still accepted, because its list is empty.

## Where the sources disagree

The schema follows what brit's code serialises. These differences are recorded and not resolved.

| Field | The lifecycle design | brit's code | brit's own documentation |
|---|---|---|---|
| `retireWhen` | `null` or a quiet duration; called optional | Always serialised; every current row is `null` | Optional, currently `null` |
| `cure` | "where one exists" | Left out when absent | Left out when absent |
| `weight` | What an occurrence cost the one who felt it; no range | An integer from 0 to 255 that the producer assigns | A small non-negative integer |
| `ceiling` | Four rungs; `steward` described as the default | Tests allow two rungs; the default is `device`; all 72 shipped rows say `device` | Four rungs; "Brit uses `device` or `steward`" |
| `validityHorizon` | An ISO 8601 duration | A string; tests allow only whole days | An ISO 8601 duration, currently `P14D` |
| `id` | The grammar, no length | The grammar, no length check | The grammar |

## Not proven

- That brit 0.3.12 writes entries this schema accepts. 0.3.12 does not exist yet; every fixture
  that holds a cure entry was written by hand to the shape brit described.
- That a published brit release archive carries the same bytes. No release with this file has
  been cut; the output checked here came from a local build of the branch.
- That the schema's patterns behave the same in a Rust validator. Only Ajv 8.18.0, a JavaScript
  JSON Schema validator, was run.
- Anything about the truth of a row: that its sentences are right, that its cures were reviewed,
  or that ids are only ever added. The schema checks shape.
- A second producer. The station's promise is one schema for every producer, and one producer
  has been checked against it.

## What happens after the ruling

If the steward chooses the SDK: the schema moves to `elohim/sdk/schemas/v1/manifest/`, its test
joins the SDK's schema test step, and the SDK pipeline's README gains a one-line pointer to the feedback designs. That pointer is
the unbuilt fifth of station FC-10 of the same plan, under which each of five documents on this
ground names the others.
