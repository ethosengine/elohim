---
id: "backlog-epr-meta-python-rust-parser-parity"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Guard the two .epr-meta parsers from grammar drift — Python _lib/epr_meta.py (enforcer) vs Rust eprfs-meta (resolver)"
slug: "epr-meta-python-rust-parser-parity"
written: "2026-07-05"
author: "collaboration-through-the-protocol plan (Task 5 in-flight capture)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## What

Two independent parsers now read the `.epr-meta` frontmatter grammar:

1. **Python `_lib/epr_meta.py`** — the live ENFORCER. Drives the Claude PreToolUse hook, the new
   git-hook adapter (`_lib/epr_meta_git.py` → `.husky/{pre-commit,pre-push}`), `ci-ignore-projector`,
   and `placement-audit`. It resolves the cascade AND evaluates edits into deny/ask/inject/measure verdicts.
2. **Rust `elohim/eprfs/eprfs-meta/src/lib.rs`** — a RESOLVER only (lands with `feat/eprfs`). It parses
   the same grammar (`epr-meta-version/id/root/covers/rules[]/validators[]/cites[]`) and resolves the
   ancestor cascade into `EprMetaResolution`, but does NOT evaluate a candidate edit or emit a verdict.
   Its own header states: "It does not replace the existing hook resolver yet."

The eprfs README + `.claude/hooks/.epr-meta` both name the Rust path as the eventual transport the
governance layer graduates toward (brit/eprfs direction). Until that graduation, the two parsers can
**silently fork** on the grammar — the likely divergence points are eprfs-meta's rule-class defaulting
(`Inject`) and its heuristic predicate detection vs the Python `validate_meta`'s closed vocabulary.

## Why it matters

If the grammars drift, the edit-time gate (Python) and any future eprfs-native resolution can disagree
about what a manifest MEANS — a governance-coherence bug that would surface as "the gate allowed a write
the Rust layer would reject" (or vice versa). The whole point of the harness-agnostic move is that all
surfaces evaluate the SAME law; a second parser is a second law unless pinned.

## Proposed work

Add a parity test (mirror brit's `cite_parity.rs` shape, which already pins brit's cite-graph verdicts
byte/label-exact against the live Python oracle): feed a corpus of representative `.epr-meta` manifests
(valid, malformed, policy-binding, measure-tier, multi-predicate) through both parsers and assert the
resolved rule sets match. Wire it into the eprfs gate. Revisit/retire only when eprfs-meta becomes the
single enforcer (Python decommissioned) — not before.

## Provenance

Surfaced by the `collaboration-through-the-protocol` plan's understanding pass
(`genesis/docs/superpowers/plans/2026-07-05-collaboration-through-the-protocol-plan.md`).
