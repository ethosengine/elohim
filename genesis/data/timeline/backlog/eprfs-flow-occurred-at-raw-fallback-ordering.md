---
id: "backlog-eprfs-flow-occurred-at-raw-fallback-ordering"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "flow read ordering: an unparseable occurred_at compares as a raw string and can outrank every valid timestamp"
slug: "eprfs-flow-occurred-at-raw-fallback-ordering"
written: "2026-10-31"
author: "claude-opus-5-5 (dialogue-stasis session 4ec0c39f); found by gpt-6-astra review, ruled by the stasis keeper ladder at C1"
status: "open"
priority: "low"
tags: [eprfs, epr-cli, flow, ordering, occurred-at, robustness]
cites:
  - elohim/eprfs/epr-cli/src/flow/read.rs
---

# An unparseable `occurred_at` can sort ahead of every real timestamp

`elohim/eprfs/epr-cli/src/flow/read.rs:76` orders flow notes by an `OccurredAtKey` that mixes a
`Parsed` RFC 3339 instant with a `Raw` string fallback. When the two kinds meet, the comparison falls
back to strings. A raw `"zzz"` therefore sorts after any value beginning with a digit, including RFC
3339 timestamps, so a malformed note can
stay "latest" until a later raw value happens to sort higher.

## How it was found and ruled

- **Found.** An independent review of the dialogue-stasis slice 6 pin check (commit c9ae5c52d,
  2026-10-31) flagged this as finding F3.
- **Ruled.** The stasis keeper ladder, in its first exercise on real findings, ruled at C1 (haiku):
  the mechanism is real here. As of that ruling, the stasis pin path does not use this fallback: it parses
  `occurred_at` itself and treats an unparseable candidate as `pin-unreadable`, which is inert. The
  general read path keeps the defect. The ruling was to park it here instead of changing shared flow
  code inside the slice. Trace: `genesis/local-dev/dialogue-stasis/ladder-review-6n/results.json`
  (local).

## What closing it means

Readers that select "latest by occurred_at" (for example rulings, corrections and habits) must not
let an unparseable value outrank a parseable one. Options:

- sort unparseable values first (oldest);
- refuse them at write time;
- report them as an observation gap.

A probe: a fixture flows file holding a valid ruling at `2026-10-31T00:00:00Z` and a note with
`occurred_at: "zzz"`. The probe passes when "latest" selects the valid ruling or reports the
malformed note; it fails when the malformed note is selected without a report.
