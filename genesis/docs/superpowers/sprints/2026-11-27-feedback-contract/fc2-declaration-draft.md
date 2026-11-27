---
id: feedback-contract-fc2-declaration-draft
title: Feedback contract, FC-2 — a draft schema for the declaration a producer ships
date: 2026-11-27
status: DRAFT
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), called the designing session below, orchestrating and checking; schema, script and hand-written fixtures by Codex (astra), the schema then changed in two places by the designing session
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-2 this draft answers | sha256:37fd9ff8f91c8771 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design whose section 5.4 lists the declaration fields the schema closes | sha256:bb8216eb2f736464 | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "one-loop-for-every-producer-design | the companion whose section 12 holds the open decision on where this schema lives | sha256:e7361b750b95dce3 | path: genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md"
  - "feedback-contract-fc1-primitives-inventory | the inventory that found reach and polarity mean something else in the SDK, which is why this draft does not reference them | sha256:ac239b35827d1a21 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc1-primitives-inventory.md"
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
| `fc2/fixtures/refused/` | Twenty-four files, each wrong in exactly one way, named for it |

The script that ran these checks is not in the repository yet. No gate would run a script kept in
a documents directory, so it comes in a following batch as a test in the orchestrator's suite,
which a gate does run. Until then the checks under Evidence are this session's own runs, and
nothing repeats them. A refused fixture's file name is a plain description of its one fault
(`severity-outside-list.json`, `duplicate-id.json`); the script holds a table from each file name
to the schema keyword and the place in the file that must be what refuses it, and a fixture with
no entry in that table fails.

To check the schema without that script: any validator for JSON Schema 2020-12 accepts the four
files in `fc2/fixtures/accepted/` and refuses twenty-one of the twenty-four in
`fc2/fixtures/refused/`. The other three (`duplicate-id.json`, `supersedes-undeclared-id.json`,
`cure-undeclared-id.json`) pass the schema and are refused only by the three script rules named
below. The script takes one file (`node check.mjs <file>`) or runs every fixture
(`--fixtures`), and requires each refused file to be refused once, for the reason its name states.

## What the schema decides, and on whose word

These were set by the designing session as working rules for a draft. None is a ruling of the
steward's.

- **It describes the file brit writes, exactly.** Keys are spelled as brit serialises them.
  `retireWhen` is always present and may be `null`; `cure` is left out when there is none and is
  never `null`.
- **Its closed lists are the lifecycle design's, which are wider than brit's use of them.**
  `severity` has the six standard logging names (`trace`, `debug`, `info`, `warn`, `error`,
  `fatal`), `polarity` two (`negative`, `positive`), `ceiling` four (`device`, `steward`,
  `collective`, `commons`). brit's table uses two of the four ceilings. A ceiling is the furthest
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
- **Three rules a JSON Schema cannot state are in the check script, each under its own name**: ids are
  unique (`uniqueClaimIds`), every superseded id is declared (`declaredSupersedes`), and every
  cure names a declared id (`declaredCureClaims`).

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

## Where the sources disagree

The schema follows what brit's code serialises. These differences are recorded and not resolved.

| Field | The lifecycle design | brit's code | brit's own documentation |
|---|---|---|---|
| `retireWhen` | `null` or a quiet duration; called optional | Always serialised; every current row is `null` | Optional, currently `null` |
| `cure` | "where one exists" | Left out when absent | Left out when absent |
| `weight` | What an occurrence cost the one who felt it; no range | An integer from 0 to 255 that the producer assigns | A small non-negative integer |
| `ceiling` | Four rungs; `steward` described as the default | Tests allow two rungs; the default is `device` | Four rungs; brit uses two |
| `validityHorizon` | An ISO 8601 duration | A string; tests allow only whole days | An ISO 8601 duration, currently `P14D` |
| `id` | The grammar, no length | The grammar, no length check | The grammar |

## Not proven

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
