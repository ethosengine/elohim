---
id: "backlog-eprfs-meta-python-governance-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Name the .epr-meta governance convergence — eprfs-meta canonical, python compose-gate binds to it"
slug: "eprfs-meta-python-governance-convergence"
written: "2026-07-06"
author: "eprfs-agent capability-projection V2 plan (Task 8 backlog capture)"
status: "backlog"
priority: "medium"
jobs: [elohim]
tags: [eprfs, eprfs-meta, epr-meta, governance, compose-gate, define-once-bind-many, policy-registry]
cites:
  - genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - genesis/docs/superpowers/plans/2026-07-05-collaboration-through-the-protocol-plan.md
  - genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md
  - genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md
  - .claude/scripts/_lib/epr_meta.py
  - elohim/eprfs/eprfs-meta/src/lib.rs
---

## What

`.epr-meta` has TWO engines today, and V2's work on `eprfs-agent` (a sibling domain adapter to
`eprfs-meta` in the same `elohim/eprfs` workspace) makes the split concrete rather than abstract:

1. **Python `_lib/epr_meta.py`** — the live ENFORCER. Drives the Claude PreToolUse hook, the
   git-hook adapter (`.husky/{pre-commit,pre-push}`), `ci-ignore-projector`, and `placement-audit`.
   Resolves the ancestor cascade AND evaluates a candidate edit into a deny/ask/inject/measure
   verdict.
2. **Rust `elohim/eprfs/eprfs-meta/src/lib.rs`** — a RESOLVER only. Parses the same grammar
   (`epr-meta-version/id/root/covers/rules[]/validators[]/cites[]`) and resolves the ancestor
   cascade into an `EprMetaResolution`, but does not evaluate an edit or emit a verdict. Its own
   header already states it does not replace the hook resolver yet.

**Define-once-bind-many** (`genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md`)
already names the doctrine this convergence follows: a rule (here, "what `.epr-meta` MEANS") should
be defined once and bound by many surfaces, never independently re-derived. Applied to the two
engines, the target-state naming is: **`eprfs-meta` becomes canonical** (the graduated
brit/eprfs-native projection substrate `.claude/hooks/.epr-meta` and the eprfs README already point
toward), and the **python `_lib/epr_meta.py` compose-gate becomes a binding** — a projection of the
same resolved law onto the git-hook/Claude-hook enforcement surface — rather than a second,
independently-evolving parser of the grammar.

## Why this is a naming, not a build

This item deliberately does NOT propose building the convergence in V2 (or in any near-term wave).
It exists to put a name and a direction on the drift risk the sibling item
`genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md` already surfaced as a
parity-TEST proposal (guard against silent grammar fork via a `cite_parity.rs`-shaped corpus test).
That item is the near-term safety net (test both parsers agree); THIS item is the longer-horizon
architectural claim (they shouldn't be two parsers forever — one should be truth, the other a
binding). Naming the direction now, without building it, keeps V2's scope genuine (it is not a
governance-substrate plan) while leaving a clear waypoint for whoever picks up eprfs-meta's
graduation from resolver to enforcer.

## Design direction (not built here)

- `eprfs-meta` gains the evaluate-a-candidate-edit capability the python enforcer has today (deny/
  ask/inject/measure verdicts over a resolved cascade), becoming a full enforcer, not just a
  resolver.
- The python `_lib/epr_meta.py` compose-gate is re-pointed to call into (or be generated from) the
  Rust resolution — e.g. via a thin FFI/subprocess/codegen boundary — so there is exactly one place
  the `.epr-meta` grammar and verdict semantics are defined.
- Until that graduation lands, `epr-meta-python-rust-parser-parity.md`'s parity test is the interim
  safety rail; do not let the two parsers diverge silently in the meantime.

## Blocked on

Nothing structurally, but sequenced behind the scale waves
(`eprfs-agent-scale-skill-agentspec-hook-waves.md`) and behind `eprfs-meta` itself graduating past
"resolver only" — this is architecture direction, not a ready-to-execute Objective.

## Provenance

Surfaced by `genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md`
Task 8 (Step 2: "Name the parity contract; do not build in V2"), which itself cites the
define-once-bind-many design and the parent `collaboration-through-the-protocol` plan.
