---
epr-meta-version: 1
id: epr-ontology-guidestar
covers: subtree
purpose: >
  The assertion layer of the epr crate family — what kind of claim this is (`kind`), who may see
  it (`reach`), what it answers to (`coupling`), how well it is known (`measure`), who observed it
  (`witness`), who decides (`verdict`), and when a bound is crossed (`algedonic`). This crate and
  its sibling `epr-rea` carry the protocol's measure and limit ontology, and until 2026-08-12 they
  were the only decision-bearing crates in the tree with neither a seam-registry nor an `.epr-meta`
  — the layer built to catch inadequate models did not cover the crates most able to cause one.
  Two edit-time signals live here, both advisory, both grounded in a defect that actually happened.
rules:
  - id: ontology-admission-guidestar
    class: inject
    when:
      write: "*.rs"
      new: true
    dedupe-of: "genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md (the composition law this crate is authored under)"
    why: >
      A NEW module in this crate is a new ontological primitive, which is the one moment the
      admission rule applies. Read the guidestar first —
      genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md.
      Its §3a: ADMIT a primitive when a SECOND independent framework needs the same missing
      distinction; never on the first, never speculatively. Bespoke code appearing at a seam is the
      EVIDENCE a distinction is missing — one framework wanting it is a hold, two is an admit. The
      rule is not new: `elohim/elohim-storage/src/services/measure.rs:6` already states it ("graduates
      to a shared crate when a second consumer appears").
      Its §6 refusal list, in short: not a union ontology (agents carry domain variety, so this crate
      needs no Georgist rent term, no Keynesian multiplier, no Robeynsian band as a native type); not
      a variety metric with no reader; not a second measure ontology; not a trait per validator
      (content addressing and compile-time binding are incompatible); not a design that only works if
      the model is good.
      Fires only on a NEW file, deliberately — an advisory on every edit would nag, and this one is
      meant to be read exactly when a primitive is born.
    retire-when: >
      when the guidestar's §3 acceptance criterion exists as a runnable check (two agents
      representing different domains negotiating a contract a third party can walk back) — at that
      point admission is decided by a failable test rather than by a reader remembering a rule, and
      an advisory pointing at the rule has nothing left to add
  - id: hashed-atom-additive-field-discipline
    class: inject
    when:
      write: "*.rs"
      contains-any: ["skip_serializing_if"]
    dedupe-of: "elohim/epr/tests/canonical_bytes.rs (the golden vectors that prove an added field moved no existing address)"
    why: >
      This file carries a field on a HASHED atom, so its serde shape IS an address. Adding a field
      to a struct inside the canonical bytes re-addresses every atom ever authored with it unless the
      field is `Option<T>` + `skip_serializing_if` with `None` meaning exactly the pre-existing
      semantics. Precedent: `Confidence.unknown_reason` (spec Q17, 2026-08-12) and
      `Bound.{sense,source}` (same day) both landed additively and were asserted at the BYTE level,
      not by a golden captured from already-changed code.
      Second half, easy to miss: one meaning must have exactly ONE encoding. If `Some(Default)` and
      `None` mean the same thing, one promise gets two CIDs — so `Bound::validate()` REFUSES
      `Some(Ceiling)` / `Some(Declared)` and the builders normalize instead. A new optional field
      wants the same refusal, or it silently admits a redundant spelling.
    retire-when: >
      when a proptest or codegen check proves address-stability structurally for every hashed atom
      (add a field, assert prior fixtures' CIDs are unmoved) rather than per-author discipline — the
      test then catches what this advisory can only remind
cites:
  - requisite-variety-guidestar-epr-family-composition
---

# `elohim/epr` — the assertion layer

Governed by the requisite-variety guidestar. The law, in one line:

> Requisite variety, performance, and composable policy · measure · judgement · control, to derive
> emergent (queryable/aggregatable) projections from any scope, micro to global.

This crate owns the **assertion** axis of that law: what kind of claim, who may see it, what it
answers to, how well it is known. Its variety must be sufficient for any systems model an agent
brings — *not* by containing every domain's ontology, but by making an agent's representation
**disputable**: claim kind, interval, basis, coupling, witness, verdict, reach.

Two things this crate is currently known to lack, both recorded in the guidestar §5 rather than
here so they stay ranked with everything else: the control plane cannot signal **insufficiency**
(`AlgedonicEvidence` is ceiling-signed, so a floor breach has no expressible pain), and `declarer`
carries no distinction between **self-report and proxy-report** (a forest cannot declare, and the
model cannot say who speaks for it).
