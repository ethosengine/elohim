---
id: "backlog-eprfs-recipe-edge-enforcement"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "recipes.yaml edges (validators, meaningful) are hashed documentation only — nothing in epr-cli reads them at flow time"
slug: "eprfs-recipe-edge-enforcement"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "low"
relatedNodeIds:
  - ".claude/epr-meta/recipes.yaml"
  - "elohim/eprfs/epr-cli/src/flow/registry.rs"
  - "elohim/eprfs/epr-cli/src/flow/walk.rs"
tags: [eprfs, epr-rea, recipes, honest-about-inert, governance]
---

`.claude/epr-meta/recipes.yaml`'s own header already says it plainly: "Edges below are
declarative documentation minted into the ProcessSpec atom; nothing enforces them in v1
(honest-about-inert)." Verified while building `saga-status.py` (T6): `registry.rs` parses each
edge's `validators: [cite-seal|decompose|a2o-parse|a2o-run]` and `meaningful: true/false` into
the `EdgeSpec` that gets hashed into the ProcessSpec atom, but nothing downstream — not
`walk.rs`'s forward/backward traversal, not `fulfill.rs`'s report→event derivation, not
`status`'s frontier ranking — ever reads an edge's `validators` list to actually gate or check
anything. A recipe author can name `a2o-run` on the `scenario → validation` edge and it changes
zero runtime behavior; it's purely provenance metadata baked into the CID.

This is fine for v1 (the fabric spec calls it out as a deliberate floor, not a bug), but it means
a recipe's edges currently make a promise the tooling doesn't keep. Candidate next step: either
(a) wire `validators` into `epr flow status`/`ready` as an actual gate — a `meaningful: true` edge
whose named validator hasn't run since the upstream last changed shows up as a stale/ungoverned
edge in the frontier, giving the "sealed contract edges" design (see
`eprfs-stale-edge-backlog.md`, same corpus) a validator-class enforcement lane alongside
`cite-seal`; or (b) drop the pretense and rename the field `documented-by` so nobody mistakes it
for enforcement. Low priority — no incident traces to this yet, just a documented gap surfaced
in passing.
