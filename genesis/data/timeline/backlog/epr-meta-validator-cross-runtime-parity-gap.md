---
id: "backlog-epr-meta-validator-cross-runtime-parity-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Three P4.1 concern-canon validators have no Rust match arm in ElohimRepositoryValidators — the dormant eprfs engine would escalate their inject-class advisories to ask, not preserve them"
slug: "epr-meta-validator-cross-runtime-parity-gap"
written: "2026-08-02"
author: "fix-integration (seam-concern architecture sprint, Wave C)"
status: "open"
priority: "medium"
tags: [epr-meta, governance, validators, eprfs, cross-runtime-parity, dev-tooling, dormant-defect, concern-canon]
cites:
  - .claude/scripts/_lib/epr_meta.py
  - .claude/epr-meta/policies.yaml
  - elohim/eprfs/epr-cli/src/repository_validators.rs
  - elohim/eprfs/eprfs-meta/src/evaluation.rs
  - genesis/data/timeline/backlog/epr-meta-unregistered-validators.md
---

# Three P4.1 validators have no Rust match arm — a dormant class-preservation asymmetry

## The gap

P4.1 (this sprint) registered three concrete validator predicates in the Python resolver
(`.claude/scripts/_lib/epr_meta.py::REFERENCE_VALIDATORS`): `epr:validator-heal-fills-never-moves`
(C2, bound by `c2-monotonic-authority@2`, `class: inject`), `epr:validator-bounded-work` (C6a,
bound by `c6a-bounded-work@2`, `class: inject`), and `epr:validator-dna-hash-neutrality`
(registered machinery with no canon row, inline-bound at `elohim/holochain/dna/.epr-meta`,
`class: inject`).

None of the three has a matching arm in the Rust engine's composition root,
`elohim/eprfs/epr-cli/src/repository_validators.rs::ElohimRepositoryValidators::evaluate`. That
match statement names exactly six references — `p2p-design-gate`, `sovereignty-ontology-guard`,
`archetype-resource-alignment`, `test-bench-aggregate-capacity`, `eprfs-meta-domain-neutrality`,
`escalation-ladder` — and falls through every other reference (including all three P4.1 names) to
`_ => return ValidatorOutcome::Unavailable` (line 30).

## Why "Unavailable" is not a neutral no-op in the Rust host

`elohim/eprfs/eprfs-meta/src/evaluation.rs`'s `GovernanceRulePredicate::Validator` arm handles
`ValidatorOutcome::Unavailable` with an explicit **ceiling law** (lines 593-609): an unresolvable
validator reference is clamped to `class: Ask` **unconditionally**, with `refer_reason:
"unresolvable-validator"` — regardless of what class the rule itself declared. The comment there
names the reasoning explicitly: "a validator declared for another runtime is implemented
(Pass/Flag) in the host that owns it, so it never reaches this arm there; only genuinely unknown
refs do." That premise is exactly what breaks for these three refs: they are not declared
runtime-scoped anywhere (no `RUNTIME_SCOPED_VALIDATORS`-equivalent list exists in the Rust host),
so a genuinely-implemented-elsewhere reference and a genuinely-unknown one are indistinguishable
to this engine — it treats all three as the latter.

Python's own resolver (`epr_meta.py` lines 344-374) handles the identical "reference not
resolvable here" situation with the opposite discipline: an unresolvable `validator:` ref is
clamped to the rule's **declared** class — `ask`/`deny` routes to review (unchanged), but a
`class: inject` rule stays `inject` (the "must not HARDEN an advisory" comment at lines 355-362
names the exact failure this backlog entry is about, from the other direction). All three
P4.1 rules are `class: inject` by design — narrow, advisory, false-positive-tolerant detectors
whose own doc comments say a miss costs nothing and a false positive costs one ignorable line.

## Why this is dormant, not live, today

`ElohimRepositoryValidators` is not wired into any active gate — no `.husky/pre-push` hook, CI
step, or PreToolUse hook invokes the Rust `epr-cli` binary against a live write today; the
resolver hook and git-gate CLI that DO gate real writes are both Python
(`epr_meta.py::resolve_write`). Nothing currently misfires. The gap is real but inert.

## What would break if it stopped being dormant

If `epr-cli`/`ElohimRepositoryValidators` is ever wired into a live gate (the crate exists
specifically as "a future provider can resolve the same references to content-addressed WASM
without changing `.epr-meta` parsing or policy evaluation" — its own doc comment anticipates
exactly this graduation), every write matching `c2-monotonic-authority@2`'s or
`c6a-bounded-work@2`'s `when` scope (any `.rs` write mentioning `stamp_declared_head` /
`StampMode` / `projection_reconcile` / `GapFill`, or any `.rs` write in a registered seam) would
route to `ask` — a blocking human-judgment prompt — for what both validators' own text declares
should never block. A dormant asymmetry today becomes a live agent-agency regression (the same
"overnight session stalls on a permission prompt" class documented in the sibling entry
`epr-meta-unregistered-validators.md`) the moment the second host goes live, unless this is fixed
first.

## The reverse case, already handled correctly (contrast)

`epr:validator-eprfs-meta-domain-neutrality` is the mirror instance, done right: it IS
implemented in Rust (`repository_validators.rs::eprfs_meta_domain_neutrality`, matched at line 28,
backed by `eprfs-meta-domain-neutrality@1` in `policies.yaml`, `class: ask`), and it is honestly
declared **rust-only** on the Python side — `epr_meta.py::RUNTIME_SCOPED_VALIDATORS = {
"epr:validator-eprfs-meta-domain-neutrality": "rust-only" }` (line 965). Python's resolver checks
this map FIRST (line 346) and skips clean ("Unavailable-by-declaration, NOT unresolvable") rather
than either evaluating a predicate it doesn't have or clamping/escalating a class for a rule it
correctly recognizes belongs to the other runtime. That declared-scoping discipline is exactly
what is missing for the three P4.1 validators in the Rust-side match statement: there is no
Rust-side equivalent of `RUNTIME_SCOPED_VALIDATORS` naming them "python-only" and skipping clean,
so they fall to the generic unresolvable arm and inherit its ask-escalation ceiling law instead.

## Fix sketch (NOT attempted here — Rust engine changes are out of this fix-integration task's
bounded scope; F1-F3 of this pass are Python-fixture, backlog-documentation, and README work only)

Either of two shapes closes the gap, mirroring the reverse case's convention:

1. **Implement the three predicates in Rust** (mechanical translation of the Python detectors —
   `_heal_fills_never_moves`, `_bounded_work`, `_dna_hash_neutrality` — into
   `repository_validators.rs` match arms), so both hosts genuinely evaluate the same predicate; or
2. **Declare them Rust-side runtime-scoped** (a `python-only`-shaped list the Rust
   `GovernanceRulePredicate::Validator` arm consults before falling to the generic
   `ValidatorOutcome::Unavailable`/ask-escalation path), honestly naming the asymmetry rather than
   letting the ceiling law's "only genuinely unknown refs reach here" assumption silently fail for
   these three.

Either path needs a Rust-side test proving the declared/implemented pair for all three
references, mirroring `eprfs_meta_domain_neutrality`'s own test coverage and the reverse-case
precedent this entry cites.

Status: open, unowned.
