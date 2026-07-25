---
id: "backlog-eprfs-walk-recipe-id"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "epr flow walk/status never surface WHICH recipe claims an artifact — confirmed two ProcessSpecs mint over the same overlapping paths"
slug: "eprfs-walk-recipe-id"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "low"
relatedNodeIds:
  - ".claude/epr-meta/recipes.yaml"
  - "elohim/eprfs/epr-cli/src/flow/project.rs"
  - "elohim/eprfs/epr-cli/src/flow/walk.rs"
tags: [eprfs, epr-rea, recipes, walk, provenance]
---

Verified against an isolated tempdir fixture while building `saga-status.py` (T6): a Commitment's
identity (`classified_as: ["a2o:scenario-green", <path>]`) never carries which recipe minted it,
so when two recipes' `scenario` stage globs overlap the SAME real path — which they do today,
`elohim-dev-pipeline`'s broad `genesis/a2o/features/**/*.feature` and `resiliency-saga`'s narrower
`genesis/a2o/features/dataplane/resiliency-saga/*.feature` both match every saga chapter — the
resulting Commitment atoms dedupe cleanly to one CID (harmless, by design). But the two recipes'
**ProcessSpec** atoms do NOT dedupe (spec identity includes `id`/`version`, so `elohim-dev-pipeline`
and `resiliency-saga` each mint their own spec covering the same joint), and neither `epr flow
walk <path>` nor `epr flow status`'s rendered output shows which recipe(s) are in play for a given
artifact — an agent debugging "why does this feature have two ProcessSpecs claiming it" has no CLI
signal, only a raw `grep` over `.eprfs/status/flows.jsonl`.

`saga-status.py` sidesteps this entirely (it matches chapters to commitments by path, ignoring
recipe identity — see its module docstring), so this isn't blocking anything today. Candidate
follow-up: `flow walk --json` could include a `claimed_by: [recipe-id@version, ...]` field per
edge/spec so an agent (or a future saga-status-style consumer) can tell which recipe's edge is
driving a given frontier entry without hand-grepping the sidecar.
