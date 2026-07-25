---
id: "backlog-eprfs-flow-bindings-slice2"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Flow records (Commitment/Event/ProcessSpec) have no TS/JS bindings — every non-Rust consumer hand-parses flows.jsonl's raw shape"
slug: "eprfs-flow-bindings-slice2"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "low"
relatedNodeIds:
  - "elohim/eprfs/epr-rea"
  - ".eprfs/status/flows.jsonl"
  - "genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md"
tags: [eprfs, epr-rea, bindings, slice-2, ts-rs]
---

Building `saga-status.py` against `.eprfs/status/flows.jsonl` (T6) meant hand-deriving the
JSON shape of `Commitment`/`FlowEvent` records empirically — reading `epr-cli`'s Rust structs and
probing real lines, since no schema or generated type exists for a non-Rust consumer to check
against. Two concrete surprises that a generated binding would have caught immediately instead of
needing an isolated-fixture experiment to confirm: (1) a record's *outer* `cid` renders as a
base32 multibase STRING, but the SAME kind of CID nested inside a record (`fulfills`, `resource`)
renders as a raw byte-array — no single JSON Schema today documents both shapes; (2) a `Dismiss`
event's `resource` field carries the SAME body CID as the `Produce` event it regresses (needed for
chapter association), which is only discoverable by reading `fulfill.rs`'s implementation, not any
published contract.

The cite-fingerprint-CID-convergence design already reserves the CLI (`eprfs cid`) as the single
CID encoder and says "Python decodes, never encodes" — the same discipline this script follows.
A natural "Slice 2" (matching that spec's own "Slice-2 eprfs cid CLI" framing) would be `ts-rs`
bindings for the flow record enum (`FlowRecord`, `Commitment`, `FlowEvent`, `ProcessSpec`) so any
future TS/Python consumer of `flows.jsonl` gets the wire shape for free instead of re-deriving it
from source reading + fixture probing, the way this session had to.
