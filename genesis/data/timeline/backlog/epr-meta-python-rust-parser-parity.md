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

## Status update (2026-07-10) — shared parity fixture corpus landed (Task 6 / B2)

A first parity suite now exists, scoped to cascade order + nearest-wins conflict resolution (not
yet the full grammar corpus the "Proposed work" section above describes — that remains open).

**Shared fixtures** (real files, resolved verbatim by both parsers, not re-synthesized per
language): `.claude/scripts/_lib/__tests__/fixtures/epr_meta_parity/{root-directory-form,
legacy-nested, cascade-conflict}/`. Three cases: a directory-form root manifest with no nested
cascade; a legacy (non-directory-form) nested `.epr-meta` flat file; and a root+nested id-collision
where the nested manifest overrides one rule's class (nearest-wins) and contributes a new one.

**Python side:** `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` (parity fixture block at
the end) resolves each fixture via `collect_cascade` + `merge_rules` and asserts the ordered
`rules` dict keys against a hard-coded expected list.

**Rust side:** `elohim/eprfs/eprfs-meta/tests/parity.rs` resolves the same three fixture paths via
`eprfs_meta::resolve_path` and asserts the same ordered rule-id/class list.

**A real divergence surfaced in the process, not yet fixed:** `EprMetaResolution::effective_rules`
(the Rust resolver's merged-rules output) is built from a `BTreeMap<String, GovernanceRule>`, so its
iteration order is always **alphabetical by rule id** — an artifact of the dedup structure. Python's
`merge_rules` returns a plain dict, whose key order is **first-seen-position** (root-first cascade
insertion), with nearest-wins only overwriting the VALUE in place. These are not the same ordering
in general (proven by the `cascade-conflict` fixture: cascade/first-seen order is `zeta-root-rule,
collide-rule, alpha-nested-rule`; alphabetical order would be `alpha-nested-rule, collide-rule,
zeta-root-rule`). The parity test therefore does NOT assert on `effective_rules`'s order directly —
it reduces the cascade-ordered `records` field (root-first, one entry per manifest, each carrying its
declared rules in file order) with the identical first-seen/nearest-wins algorithm Python's dict
performs natively. No current consumer of `effective_rules` depends on its order (checked via grep),
so this is not yet a live bug — but it is exactly the kind of silent-fork risk this backlog item
exists to catch, and is worth fixing (e.g. switch `effective_rules` to preserve cascade/nearest-wins
order, mirroring `records`) before any consumer starts relying on rule-priority-by-position.

**Still open:** the fuller corpus this item originally asked for (malformed manifests, policy-binding
expansion, measure-tier, multi-predicate footguns) is not yet covered by the shared fixture corpus —
those remain Python-only today (`epr_meta_policy_test.py`, `epr_meta_eval_test.py`,
`epr_meta_schema_test.py`). Extend the shared corpus there next, and consider fixing the
`effective_rules` ordering artifact above in the same pass.
