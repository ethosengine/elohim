---
id: "backlog-native-govern-prior-channel"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Native govern — a prior-content channel so a landed edit can be classified as a delta"
slug: "native-govern-prior-channel"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane C review follow-up, finding W1)"
status: "backlog"
priority: "low"
tags: [governance, frames, epr-govern, sovereignty-guard, lane-c]
relatedNodeIds:
  - guards-sense-frames
cites:
  - genesis/a2o/reports/post-station-4-2026-09-25/rulings.md
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/eprfs/epr-cli/src/govern.rs
  - elohim/eprfs/epr-cli/src/frames.rs
  - .claude/hooks/sovereignty-guard-signal.py
shift_objective: |
  Give `epr govern` a way to receive the prior content of a write that has already landed
  (for example `--prior-file PATH`, or a prior read from stdin beside the content), so the
  sovereignty guard's POST hook can ask the native evaluator to classify the EDIT's delta rather
  than the whole landed document. Done when a hook ledger row whose `net_new` comes from an Edit
  carries a `classification_cid` minted over the same delta, with `classification_scope: "delta"`,
  and a test in frame_probe_test.py proves it against a stub and against the pool `epr`.
---

# Native govern — a prior-content channel (finding W1)

**The gap.** The sovereignty guard's POST hook writes one ledger row per landing. The row's
`net_new` and `phrases` describe the edit's delta: the hook rebuilds the pre-edit text from
`old_string` → `new_string`. Its `classification_cid` does not. The detached classify child runs
`epr govern --new --content-stdin` over the landed bytes. `govern.rs` reads the prior from
disk (`std::fs::read(&target)`), and after landing the file on disk already holds the new
content. So the child passes `--new`, and the native classifier judges the whole document
against an empty prior.

Ruling R-C14 names this honestly instead of hiding it. The row carries
`classification_scope: "document"`, and the hook's docstring says the CID classifies the whole
document. The row is correct but coarse: the CID can cover apex framing that the edit did not
introduce.

**The shape of the fix.** `govern` already accepts the content through `--content-stdin` or
`--content-file`. A matching way to pass the prior (a file path is the simplest; a second stdin
stream needs framing) would let the child set `GovernanceWrite.prior_content` to the
reconstructed pre-edit text. `frames::classify` already diffs prior against post. The child
would then record `classification_scope: "delta"` whenever it passed a prior.

**Why it waits.** The rows are advisory, never an accusation, and the scope field makes them
readable today. This is a precision gain for the drift review, not a correctness fix.
