---
title: "Measure Ontology Slice 1 — dynamics + confidence at the EPR level, local-first"
id: measure-ontology-slice1-epr-local-first
status: Draft
domain: D2
sprint: operator-directed-2026-08-11
requires_env: [household-nodes]
cites:
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - algedonic-phase1-epr-local-first | the structural precedent this plan mirrors — same epr/epr-rea/epr-meta local-first slice shape, same producer→consumer closure discipline, same devspace-before-network sequencing | sha256:a70d7eab56d40189 | path: genesis/docs/superpowers/plans/2026-08-10-algedonic-phase1-epr-local-first-plan.md
  - epr-meta-policy-registry-measure | the SHIPPED measure enforcement tier Task 4 extends rather than forks — policy registry + Precedent-shaped rows, and the graduation path that keeps this DHT-entry-free | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - middot-measure-primitive-design | the measure primitive this ontology extends — measures never carry teeth, honest absence (C4), the family vocabulary the kind/confidence fields attach to | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - resilience-facings-select-fold-aggregate-design | the select→fold→aggregate machinery the L5 closure law binds — the fold that must now return an interval instead of a scalar | sha256:738c9220d105e9e4 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - computation-attestation-graduated-rigor-design | where statistical-method application belongs (spec open question Q2) — the witness/audit/proof/confirmation ladder a contested narrow interval escalates, deliberately NOT built in slice 1 | sha256:d767f5c1eb04c841 | path: genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md
  - frame-witness-primitive-architecture | the witness half of the witnessed-vs-estimated split — what it means for a measure to be honest in the intimate context where the observation occurs | sha256:9acf41622029875e | path: genesis/docs/superpowers/specs/2026-07-15-frame-witness-primitive-architecture-design.md
---

# Measure Ontology Slice 1 — Dynamics + Confidence at the EPR Level, Local-First

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a measure can say what *kind* of quantity it is (level / rate / ratio) and how well it is *known* (witnessed / estimated, with an interval and a basis) — typed in `elohim/epr`, propagated by `epr-rea`'s folds, enforced by the existing `.epr-meta` measure tier, and demonstrably closed against our own development disciplines with real numbers, zero network involvement.

**Architecture:** Three crates, one direction of travel. `elohim/epr` owns the vocabulary and makes the dishonest states *unrepresentable* (an interval that can be detached, a rate without a period). `epr-rea` owns the closure law — a fold over interval-carrying measures returns an interval, never a bare scalar. `.epr-meta`'s **already-shipped** measure tier is *extended*, not forked, to carry the two new fields. The consumer is our own devspace: the session-start scoreboard currently reports levels (`208/120`, `33781/24400`) with no rates and no honesty about which of its numbers are estimates — this slice makes it report a generation ÷ absorption ratio whose absorption term is an honest estimate with a wide interval, which is the first real exercise of the confidence path.

**Tech Stack:** Rust (`elohim/epr`, `elohim/epr-rea`), Python (`.claude/scripts/_lib/epr_meta.py`, `.claude/scripts/habits-status.py`), YAML (`.claude/epr-meta/policies.yaml`), JSON Schema (`elohim/sdk/schemas/v1/`).

## Discovery record (born-linked, per the /plan pre-step)

- **Lexical lens** (`spec-coherence-index.py`): 8 matches. Binding prior art — `epr-meta-policy-registry-measure-design` (the measure tier is **shipped**; extend it), `resilience-facings-select-fold-aggregate-design` (the fold this closure law binds), `frame-witness-primitive-architecture-design` (the witness half of witnessed-vs-estimated).
- **Semantic lens** (MemPalace): **unavailable at authoring, resolved same-day.** The tools did not resolve via ToolSearch because the server was never registered in `.mcp.json` at all (fixed, commit `3245bf8d4`; takes effect on new sessions). Re-run against the restored lens returned **15 ranked hits, top similarity 0.329** — low enough to be its own answer: there is little close prior art for this vocabulary, which is what the lens exists to tell you. **The lexical lens's binding prior art stands and this plan's composition decisions are unchanged.**
  - ⚠ **Standing caveat for whoever executes Task 2.** The palace index was last mined **2026-08-08** and SessionStart reports surface files changed since. The lens is now *reachable but stale*, which defeats the skill's staleness guard — that guard fires on "returns nothing," not on "returns confidently from a frozen index." **Re-mine before Task 2 seals canon**, and until then treat a no-duplicate verdict as provisional.
- **MAP-PATH:** implements in **D2 — Evidence Primitives** (the confidence qualifier types the evidence carrier). Boundary: when the folded quantity becomes economic aggregate it emits into **D9**; that crossing is slice 2, not here.
- **ROADMAP-PRIORITY:** not a ranked Sprint-N. Declared `operator-directed-2026-08-11`, following the `algedonic-phase1` precedent. **It composes onto Sprint 1's surface rather than competing with it** — Sprint 1 (REA rails, 21 OPEN) hardens `epr-rea`'s fold; this slice adds the ontology that fold will carry. Sequence Task 3 after Sprint 1's fold work if both are in flight.
- **Placement audit `--focus`:** `household-nodes` AVAILABLE, 0 BLOCKED-BY-ENV. This slice is fully testable now.
- **P2P design gate:** run 2026-08-11, output recorded in `measure-family-borrows-backlog`. **Zero new DHT entry types.** This plan adds no entry type, no table, no route.

## Global Constraints

- **Commit-only, path-limited**: never push; the tree carries other sessions' modifications — commit exactly your files with `git commit -m "..." -- <paths>`.
- **Native cargo discipline**: `RUSTFLAGS=""` for native builds; set `CARGO_TARGET_DIR` from `cargo-pool key` run in the crate directory; never trust piped exit codes — echo `EXIT=$?` on its own line after every cargo run; `cargo test`, not `cargo check`, verifies.
- **Additive only**: `.claude/data/architecture-findings.jsonl` fingerprints are byte-stable; new fields never enter fingerprint inputs; readers use `.get(...)`.
- **Compose, don't fork**: the `.epr-meta` measure tier and the policy registry both ship today. Bind and extend; a second measure mechanism is a plan failure.
- **The interval rides inside the content hash.** Any design that makes a confidence qualifier separately addressable, patchable, or optional-after-the-fact is refused — a narrowable-after-the-fact estimate is not an estimate.

## Source-of-truth declarations (P2P audit answer)

This plan creates **no table, no migration, no route, and no DHT entry type**. The audit hook flags L6's "schema validation" wording; that phrase means *YAML rule validation in `epr_meta.py`*, not a storage schema. Declared explicitly so no implementer has to infer it:

| Surface | Class | Source of truth | Reconstruction |
|---|---|---|---|
| `elohim_epr::measure::*` types (Task 1) | vocabulary only — no persistence | n/a | n/a |
| `fold::with_uncertainty` output (Task 3) | **C — operational** | the input `Quantity` set | pure function; recompute from inputs |
| `.epr-meta` measure-tier `kind`/`per` fields (Task 4) | policy **content**, repo-local | `.claude/epr-meta/policies.yaml` (the `id@version` pin) | Graduates to a Mishpat `Precedent` when epr-meta lifts to the brit/eprfs substrate — per the policy-registry spec, **no new DHT entry type** |
| `architecture-findings.jsonl` added fields (Task 4) | **C — operational** | re-derivable by re-running the measure tier | additive, never in fingerprint inputs |
| `doc_dynamics` ratio (Task 5) | **C — operational** | `git log` over the doc globs | recompute on demand; never stored |

When any of these crosses the network boundary in slice 2, it re-enters the gate — Task 6 §"What slice 2 inherits" names the five unsolved items that crossing requires.

---

### Task 1: `elohim/epr` — the measure-dynamics + confidence vocabulary

**Files:**
- Create: `elohim/epr/src/measure.rs`
- Modify: `elohim/epr/src/lib.rs` (add `pub mod measure;`)
- Test: `elohim/epr/tests/measure_ontology.rs`

**Interfaces:**
- Produces: `MeasureKind` (`Level` | `Rate { per: Period }` | `Ratio`), `Period` (`Second|Minute|Hour|Day|Week|Month|Year`), `Confidence { claim: ClaimKind, interval: Interval, basis: String }`, `ClaimKind` (`Witnessed | InstrumentMeasured | Estimated | Modelled | Imputed`), `Interval { lo: f64, hi: f64 }` with `Interval::exact(v)` and `Interval::unknown()`, and `Quantity { value: f64, kind: MeasureKind, confidence: Confidence }`.
- Consumed by: Task 3 (`epr-rea` folds), Task 4 (epr-meta measure tier serialization).

- [x] **Step 1: Write the failing tests**

```rust
// elohim/epr/tests/measure_ontology.rs
use elohim_epr::measure::*;

#[test]
fn a_rate_cannot_exist_without_a_period() {
    // MeasureKind::Rate carries its period in the type — there is no way to
    // construct a rate that forgot its denominator. This is the unit error
    // that shipped in spatial_capacity.rs made unrepresentable.
    let r = MeasureKind::Rate { per: Period::Year };
    assert_eq!(r.period(), Some(Period::Year));
    assert_eq!(MeasureKind::Level.period(), None);
}

#[test]
fn interval_unknown_is_the_degenerate_case_of_honest_absence() {
    // C4 honest absence is subsumed: "unmeasured" is the interval that
    // admits everything, NOT a separate nullable field.
    let u = Interval::unknown();
    assert!(u.is_unknown());
    assert!(!Interval::exact(3.0).is_unknown());
    assert!(u.contains(f64::MIN_POSITIVE) && u.contains(1e300));
}

#[test]
fn widening_is_free_and_narrowing_is_refused() {
    // The honesty asymmetry: an agent may always widen its own claim.
    // Narrowing is not a mutation — it requires a new observation.
    let c = Confidence::estimated(Interval::new(10.0, 20.0), "self-report");
    let wider = c.widen(Interval::new(5.0, 30.0)).expect("widening is always allowed");
    assert_eq!(wider.interval, Interval::new(5.0, 30.0));
    assert!(c.widen(Interval::new(12.0, 18.0)).is_err(), "narrowing must be refused");
}

#[test]
fn witnessed_and_estimated_are_distinguishable_and_basis_is_required() {
    let w = Confidence::witnessed(Interval::exact(42.0), "wc -c on disk");
    assert_eq!(w.claim, ClaimKind::Witnessed);
    assert!(!w.basis.is_empty(), "a claim without a basis is uninterpretable");
}

#[test]
fn quantity_serializes_with_confidence_inline_not_detachable() {
    // The interval must be INSIDE the canonical bytes. If Confidence were
    // serialized as a sibling document, it could be swapped post-hoc.
    let q = Quantity {
        value: 12.0,
        kind: MeasureKind::Rate { per: Period::Day },
        confidence: Confidence::estimated(Interval::new(8.0, 16.0), "3-week sample"),
    };
    let json = serde_json::to_string(&q).unwrap();
    assert!(json.contains("\"confidence\""), "confidence is inline in the quantity");
    assert!(json.contains("\"per\":\"day\""), "the period survives the wire");
}
```

- [x] **Step 2: Run tests to verify they fail**

```bash
cd elohim/epr && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test --test measure_ontology
echo "EXIT=$?"
```

Expected: FAIL — `unresolved import elohim_epr::measure`.

- [x] **Step 3: Write the minimal implementation**

```rust
// elohim/epr/src/measure.rs
//! Measure dynamics + confidence. Two orthogonal questions about a number:
//! WHAT KIND of quantity is it (level / rate / ratio), and HOW WELL is it known
//! (witnessed / estimated, with an interval and a basis).
//!
//! Both ride INSIDE the quantity's canonical bytes. A confidence that could be
//! detached could be narrowed after the fact, which is not an estimate.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period { Second, Minute, Hour, Day, Week, Month, Year }

/// A level is a stock; a rate is a flow and CANNOT forget its denominator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MeasureKind {
    Level,
    Rate { per: Period },
    Ratio,
}

impl MeasureKind {
    pub fn period(&self) -> Option<Period> {
        match self { MeasureKind::Rate { per } => Some(*per), _ => None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Interval { pub lo: f64, pub hi: f64 }

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self { Interval { lo, hi } }
    pub fn exact(v: f64) -> Self { Interval { lo: v, hi: v } }
    /// Honest absence: the interval that admits everything.
    pub fn unknown() -> Self { Interval { lo: f64::NEG_INFINITY, hi: f64::INFINITY } }
    pub fn is_unknown(&self) -> bool { self.lo.is_infinite() && self.hi.is_infinite() }
    pub fn contains(&self, v: f64) -> bool { v >= self.lo && v <= self.hi }
    pub fn width(&self) -> f64 { self.hi - self.lo }
    /// True iff `other` is at least as wide as self on both sides.
    pub fn is_widening_of(&self, other: &Interval) -> bool {
        other.lo <= self.lo && other.hi >= self.hi
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind { Witnessed, InstrumentMeasured, Estimated, Modelled, Imputed }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Confidence {
    pub claim: ClaimKind,
    pub interval: Interval,
    /// What grounds this claim. A bare ± is uninterpretable.
    pub basis: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfidenceError {
    #[error("narrowing an interval requires a new observation, not a mutation")]
    NarrowingRefused,
}

impl Confidence {
    pub fn witnessed(interval: Interval, basis: impl Into<String>) -> Self {
        Confidence { claim: ClaimKind::Witnessed, interval, basis: basis.into() }
    }
    pub fn estimated(interval: Interval, basis: impl Into<String>) -> Self {
        Confidence { claim: ClaimKind::Estimated, interval, basis: basis.into() }
    }
    pub fn unknown(basis: impl Into<String>) -> Self {
        Confidence { claim: ClaimKind::Estimated, interval: Interval::unknown(), basis: basis.into() }
    }
    /// Widening is always free. Narrowing is refused — it needs a new observation.
    pub fn widen(&self, to: Interval) -> Result<Confidence, ConfidenceError> {
        if self.interval.is_widening_of(&to) {
            Ok(Confidence { interval: to, ..self.clone() })
        } else {
            Err(ConfidenceError::NarrowingRefused)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: f64,
    #[serde(flatten)]
    pub kind: MeasureKind,
    pub confidence: Confidence,
}
```

Add to `elohim/epr/src/lib.rs`:

```rust
pub mod measure;
```

- [x] **Step 4: Run tests to verify they pass**

```bash
cd elohim/epr && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test --test measure_ontology
echo "EXIT=$?"
```

Expected: PASS, 5 tests.

- [x] **Step 5: Run the crate gate**

```bash
cd elohim/epr && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test && RUSTFLAGS="" cargo clippy -- -D warnings && RUSTFLAGS="" cargo fmt --check
echo "EXIT=$?"
```

Expected: all green. If `thiserror` is not already a dependency of `elohim/epr`, add it to `Cargo.toml` in this step (check first — it is used widely in the workspace).

- [x] **Step 6: Commit**

```bash
git add elohim/epr/src/measure.rs elohim/epr/src/lib.rs elohim/epr/tests/measure_ontology.rs elohim/epr/Cargo.toml
git commit -m "feat(epr): measure dynamics + confidence vocabulary — kind, interval, basis" -- elohim/epr
```

---

### Task 2: The ontology canon — formalize and seal the spec additions

**Files:**
- Create: `genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md`
- Modify: `genesis/data/timeline/backlog/measure-family-borrows-backlog.md` (mark rows 12–18 as spec'd, cite the new spec)

**Interfaces:**
- Consumes: the type names from Task 1 — every canon line anchors to an enforcing construct that now exists.
- Produces: the `epr:` slug `measure-dynamics-confidence-ontology-design`, cited by Tasks 3–6.

This is the "formalize and seal" half of the operator's dispatch. The rule the algedonic phase-1 canon established holds: **every do/do-not line names the construct that enforces it.** A canon line with no enforcing construct is a wish, and gets written as an explicit open question instead.

- [x] **Step 1: Write the spec with these six laws, each anchored**

| # | Law | Enforced by |
|---|-----|-------------|
| L1 | A rate declares its period; a level does not have one | `MeasureKind::Rate { per }` — unrepresentable otherwise (Task 1) |
| L2 | Confidence rides inside the quantity's canonical bytes | `Quantity` `#[serde(flatten)]` + the canonical-bytes test (Task 1 Step 1, Task 6) |
| L3 | Honest absence is the degenerate interval, not a nullable field | `Interval::unknown()` + `is_unknown()` (Task 1) |
| L4 | Widening is free; narrowing requires a new observation | `Confidence::widen` → `NarrowingRefused` (Task 1) |
| L5 | A fold over interval-carrying quantities returns an interval | `fold::with_uncertainty` closure test (Task 3) |
| L6 | A measure declaration without `kind` is refused at the gate | `.epr-meta` measure-tier schema validation (Task 4) |

Also record, as **explicit open questions** (they have no enforcing construct in slice 1, and saying so is the point):

- **Q1 — interval arithmetic is deliberately naive in slice 1.** `[a,b] + [c,d] = [a+c, b+d]` assumes perfect correlation and therefore *over-*widens for independent terms. Over-widening is the safe direction (it never manufactures precision), but a correlation-aware fold is slice-2 work and must not be faked here.
- **Q2 — statistical-method application is ceiling work, not substrate work.** Reconciling divergent estimates belongs at the elohim ceiling as a `ComputationAttestation` at a graduated-rigor tier, where a contested narrow interval escalates exactly as a contested clustering does. Slice 1 ships the *inputs* to that, never the reconciliation.
- **Q3 — the uncertainty work-queue (measure-family row 18) is not built here.** Decomposing an aggregate's uncertainty by contribution needs L5 in place first.
- **Q4 — network semantics are out of scope by design.** See Task 6.

- [x] **Step 2: Seal the cites**

```bash
python3 .claude/scripts/memory-kit/cite-gen.py --seal genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
echo "EXIT=$?"
```

Expected: path-cites converted to envelopes with `sha256:` fingerprints; verification passes. If it flags title-default descriptions, author relationship hints with `cite-describe.py`.

- [x] **Step 3: Verify the spec is auditable**

```bash
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | tail -20
```

Expected: the new spec appears with a status, not as a no-status orphan.

- [x] **Step 4: Commit**

```bash
git add genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md genesis/data/timeline/backlog/measure-family-borrows-backlog.md
git commit -m "docs(spec): measure dynamics + confidence ontology — six laws, each anchored to its enforcing construct" -- genesis/docs/superpowers/specs genesis/data/timeline/backlog
```

---

### Task 3: `epr-rea` — the closure law, folds propagate uncertainty

**Files:**
- Modify: `elohim/epr-rea/src/fold.rs`
- Test: `elohim/epr-rea/tests/uncertainty_closure.rs`

**Interfaces:**
- Consumes: `elohim_epr::measure::{Quantity, Interval, Confidence, ClaimKind, MeasureKind}` (Task 1).
- Produces: `fold::with_uncertainty(items: &[Quantity]) -> Result<Quantity, FoldError>` — returns a `Quantity` whose `confidence.interval` is the propagated interval and whose `claim` is the **weakest** claim kind among its inputs.

The weakest-claim rule is the honesty half: one imputed input makes the whole aggregate imputed. An aggregate cannot be better-known than its worst-known term, and letting it claim otherwise is exactly the false-precision failure.

- [x] **Step 1: Write the failing tests**

```rust
// elohim/epr-rea/tests/uncertainty_closure.rs
use elohim_epr::measure::*;
use elohim_epr_rea::fold;

fn est(v: f64, lo: f64, hi: f64) -> Quantity {
    Quantity { value: v, kind: MeasureKind::Level,
        confidence: Confidence::estimated(Interval::new(lo, hi), "fixture") }
}
fn wit(v: f64) -> Quantity {
    Quantity { value: v, kind: MeasureKind::Level,
        confidence: Confidence::witnessed(Interval::exact(v), "fixture") }
}

#[test]
fn a_fold_over_intervals_returns_an_interval() {
    let out = fold::with_uncertainty(&[est(10.0, 8.0, 12.0), est(20.0, 18.0, 22.0)]).unwrap();
    assert_eq!(out.value, 30.0);
    assert_eq!(out.confidence.interval, Interval::new(26.0, 34.0));
}

#[test]
fn the_aggregate_takes_the_weakest_claim_of_its_inputs() {
    // One estimate makes the whole sum an estimate. False precision is the
    // mechanism behind "lies, damn lies, and statistics".
    let out = fold::with_uncertainty(&[wit(10.0), est(20.0, 18.0, 22.0)]).unwrap();
    assert_eq!(out.confidence.claim, ClaimKind::Estimated);
}

#[test]
fn one_unknown_term_makes_the_aggregate_unknown_not_wrong() {
    let unknown = Quantity { value: 0.0, kind: MeasureKind::Level,
        confidence: Confidence::unknown("never measured") };
    let out = fold::with_uncertainty(&[wit(10.0), unknown]).unwrap();
    assert!(out.confidence.interval.is_unknown(),
        "an unmeasured term must not silently contribute zero");
}

#[test]
fn folding_mixed_kinds_is_refused() {
    let level = wit(10.0);
    let rate = Quantity { value: 5.0, kind: MeasureKind::Rate { per: Period::Day },
        confidence: Confidence::witnessed(Interval::exact(5.0), "fixture") };
    // This is the spatial_capacity.rs defect as a compile-adjacent guard:
    // a cumulative level and a per-day rate have no honest sum.
    assert!(fold::with_uncertainty(&[level, rate]).is_err());
}

#[test]
fn folding_is_deterministic_under_input_reordering() {
    // Sibling of the bound_stock determinism decision: the closure law is
    // worthless if two peers fold to different intervals.
    let a = [est(10.0, 8.0, 12.0), est(20.0, 18.0, 22.0), est(30.0, 29.0, 31.0)];
    let mut b = a.clone();
    b.reverse();
    assert_eq!(fold::with_uncertainty(&a).unwrap(), fold::with_uncertainty(&b).unwrap());
}
```

- [x] **Step 2: Run tests to verify they fail**

```bash
cd elohim/epr-rea && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test --test uncertainty_closure
echo "EXIT=$?"
```

Expected: FAIL — `fold::with_uncertainty` not found.

- [x] **Step 3: Implement**

```rust
// append to elohim/epr-rea/src/fold.rs
use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Quantity};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FoldError {
    #[error("cannot fold mixed measure kinds: {0:?} and {1:?}")]
    MixedKinds(MeasureKind, MeasureKind),
    #[error("cannot fold an empty set")]
    Empty,
}

/// Rank claim kinds weakest-last so a fold can take the minimum honestly.
fn claim_rank(c: ClaimKind) -> u8 {
    match c {
        ClaimKind::Witnessed => 0,
        ClaimKind::InstrumentMeasured => 1,
        ClaimKind::Estimated => 2,
        ClaimKind::Modelled => 3,
        ClaimKind::Imputed => 4,
    }
}

/// The closure law (L5): a fold over interval-carrying quantities RETURNS an
/// interval, and never claims to be better-known than its worst input.
///
/// Slice-1 interval arithmetic is deliberately naive — `[a,b]+[c,d]=[a+c,b+d]`
/// assumes perfect correlation and therefore OVER-widens for independent terms.
/// Over-widening never manufactures precision; a correlation-aware fold is
/// slice-2 work (spec open question Q1). Summation is commutative and the
/// weakest-claim reduction is a min, so the result is order-independent by
/// construction — no sort required.
pub fn with_uncertainty(items: &[Quantity]) -> Result<Quantity, FoldError> {
    let first = items.first().ok_or(FoldError::Empty)?;
    let kind = first.kind;
    for q in items {
        if q.kind != kind {
            return Err(FoldError::MixedKinds(kind, q.kind));
        }
    }
    let value = items.iter().map(|q| q.value).sum();
    let lo = items.iter().map(|q| q.confidence.interval.lo).sum();
    let hi = items.iter().map(|q| q.confidence.interval.hi).sum();
    let weakest = items
        .iter()
        .max_by_key(|q| claim_rank(q.confidence.claim))
        .map(|q| q.confidence.claim)
        .unwrap_or(ClaimKind::Imputed);
    Ok(Quantity {
        value,
        kind,
        confidence: Confidence {
            claim: weakest,
            interval: Interval::new(lo, hi),
            basis: format!("fold of {} terms", items.len()),
        },
    })
}
```

- [x] **Step 4: Run tests to verify they pass**

```bash
cd elohim/epr-rea && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test --test uncertainty_closure
echo "EXIT=$?"
```

Expected: PASS, 5 tests. Note `Interval::unknown()` sums to infinity on both ends, which is why the unknown-term test passes without a special case — verify that is what actually happens rather than assuming it.

- [x] **Step 5: Run the crate gate**

```bash
cd elohim/epr-rea && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test && RUSTFLAGS="" cargo clippy -- -D warnings && RUSTFLAGS="" cargo fmt --check
echo "EXIT=$?"
```

- [x] **Step 6: Commit**

```bash
git add elohim/epr-rea/src/fold.rs elohim/epr-rea/tests/uncertainty_closure.rs
git commit -m "feat(epr-rea): uncertainty-propagating fold — closure law, weakest-claim, mixed-kind refusal" -- elohim/epr-rea
```

---

### Task 4: `.epr-meta` measure tier — extend the shipped tier, do not fork it

**Files:**
- Modify: `.claude/epr-meta/policies.yaml` (add `kind` to the measure-tier policy shape)
- Modify: `.claude/scripts/_lib/epr_meta.py` (`validate_meta` — refuse a measure rule with no `kind`)
- Test: `.claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py`

**Interfaces:**
- Consumes: the vocabulary names from Task 1 — the YAML `kind:` values are exactly `level` / `rate` / `ratio`, and a `rate` requires `per:`. Wire parity with the Rust serde names is law.
- Produces: `.epr-meta` `class: measure` rules that declare a `kind` (and `per`, for `kind: rate`) inside their `measure:` block, refused at load if they don't.

The measure tier already exists and is wired for source-file LoC ceilings. This task teaches it the new `kind` field; it does **not** introduce a second measurement mechanism.

- [x] **Step 1: Write the failing test**

```python
# .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py
from _lib.epr_meta import validate_meta

def test_measure_rule_without_kind_is_refused():
    meta = {"rules": [{"id": "loc", "class": "measure", "when": {"write": "*.rs"}}]}
    errs = validate_meta(meta)
    assert any("kind" in e for e in errs), f"expected a kind advisory, got {errs}"

def test_rate_without_period_is_refused():
    meta = {"rules": [{"id": "churn", "class": "measure", "kind": "rate",
                       "when": {"write": "*.rs"}}]}
    errs = validate_meta(meta)
    assert any("per" in e for e in errs), f"expected a period advisory, got {errs}"

def test_level_with_kind_is_accepted():
    meta = {"rules": [{"id": "loc", "class": "measure", "kind": "level",
                       "when": {"write": "*.rs"}}]}
    assert validate_meta(meta) == []

def test_kind_vocabulary_matches_the_rust_serde_names():
    # Wire parity is law — read the Rust source, never a copied constant.
    src = open("elohim/epr/src/measure.rs").read()
    for name in ("level", "rate", "ratio"):
        assert f'"{name}"' in src or name in src, f"{name} missing from Rust vocabulary"
```

- [x] **Step 2: Run to verify it fails**

```bash
cd /projects/elohim && python3 -m pytest .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py -v
echo "EXIT=$?"
```

Expected: FAIL — `validate_meta` returns `[]` for a measure rule with no `kind`.

- [x] **Step 3: Implement the validation**

In `.claude/scripts/_lib/epr_meta.py`, inside `validate_meta`'s per-rule loop, add:

```python
KIND_VOCAB = {"level", "rate", "ratio"}

if rule.get("class") == "measure":
    kind = rule.get("kind")
    if kind is None:
        errors.append(
            f"rule {rule.get('id')!r}: a measure rule must declare kind: "
            f"level|rate|ratio (L6) — an undeclared kind is how a rate gets "
            f"compared to a cumulative level"
        )
    elif kind not in KIND_VOCAB:
        errors.append(f"rule {rule.get('id')!r}: unknown kind {kind!r}; expected one of {sorted(KIND_VOCAB)}")
    elif kind == "rate" and not rule.get("per"):
        errors.append(f"rule {rule.get('id')!r}: kind: rate requires per: (second|minute|hour|day|week|month|year)")
```

Then add `kind: level` to the existing `source-file-loc-ceiling` policy in `.claude/epr-meta/policies.yaml` — a LoC ceiling is a level, and declaring it is the migration of the shipped tier onto the new vocabulary.

- [x] **Step 4: Run to verify it passes, then run the whole `_lib` suite**

```bash
cd /projects/elohim && python3 -m pytest .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py -v && python3 -m pytest .claude/scripts/_lib/__tests__/ -q
echo "EXIT=$?"
```

Expected: new tests PASS and **no existing test regresses** — if any existing `.epr-meta` fixture has a measure rule without `kind`, fix the fixture, do not weaken the validation.

- [x] **Step 5: Commit**

```bash
git add .claude/epr-meta/policies.yaml .claude/scripts/_lib/epr_meta.py .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py
git commit -m "feat(epr-meta): measure tier declares kind; rate requires a period" -- .claude
```

---

### Task 5: The local consumer — dogfood on our own development disciplines

**Files:**
- Modify: `.claude/scripts/habits-status.py` (add the generation ÷ absorption ratio to the headline)
- Create: `.claude/scripts/_lib/doc_dynamics.py`
- Test: `.claude/scripts/_lib/__tests__/doc_dynamics_test.py`

**Interfaces:**
- Consumes: the `kind` / `confidence` vocabulary (Tasks 1, 4) — the Python side mirrors the same field names.
- Produces: `doc_dynamics.generation_absorption_ratio(window_days: int) -> dict` returning `{"value": float, "kind": "ratio", "confidence": {"claim": ..., "interval": {"lo":..,"hi":..}, "basis": ...}}`.

**This is the task that makes the slice real rather than synthetic.** Our session-start scoreboard today reports only levels — `cleanup pressure 208/120`, `MEMORY.md 33781/24400` — with no rates and no honesty about which numbers are estimates. The Meadows survey (§6) measured every available ratio above 1.0, which is a formal overshoot reading against our own declared capacities.

The honest part, and the reason this is a genuine exercise of the confidence path: **the numerator is witnessed and the denominator is not.** Documents authored per week is countable from `git log`. "Absorption" is not a single countable event — it is decompose-completions plus `held/` moves plus archive plus in-place compaction, and no ledger counts all four. So the absorption term is an **estimate with a wide interval**, and the ratio inherits that by L5. If the resulting ratio is honest, its interval will be wide enough to be uncomfortable. That is the correct outcome, not a bug to tune away.

- [x] **Step 1: Write the failing tests**

```python
# .claude/scripts/_lib/__tests__/doc_dynamics_test.py
from _lib.doc_dynamics import generation_absorption_ratio

def test_ratio_is_declared_as_a_ratio_kind():
    r = generation_absorption_ratio(window_days=28)
    assert r["kind"] == "ratio"

def test_absorption_is_honestly_an_estimate_not_a_witness():
    # No ledger counts all four absorption paths, so claiming witnessed here
    # would be exactly the false precision this ontology exists to prevent.
    r = generation_absorption_ratio(window_days=28)
    assert r["confidence"]["claim"] == "estimated"
    assert r["confidence"]["basis"], "an estimate without a basis is uninterpretable"

def test_interval_brackets_the_point_value():
    r = generation_absorption_ratio(window_days=28)
    lo, hi = r["confidence"]["interval"]["lo"], r["confidence"]["interval"]["hi"]
    assert lo <= r["value"] <= hi
    assert hi > lo, "a nonzero-width interval — this quantity is not exactly known"

def test_zero_absorption_yields_unknown_not_infinity():
    r = generation_absorption_ratio(window_days=0)
    assert r["confidence"]["interval"]["hi"] == float("inf")
```

- [x] **Step 2: Run to verify it fails**

```bash
cd /projects/elohim && python3 -m pytest .claude/scripts/_lib/__tests__/doc_dynamics_test.py -v
echo "EXIT=$?"
```

Expected: FAIL — module not found.

- [x] **Step 3: Implement**

```python
# .claude/scripts/_lib/doc_dynamics.py
"""Generation ÷ absorption for the doc corpus — the harvest/regeneration index
applied to our own development discipline.

Meadows' rule: overshoot is indicated when the ratio crosses 1.0, not when the
stock is visibly gone. The numerator is witnessed (git log counts adds); the
denominator is ESTIMATED, because absorption happens through four paths and no
ledger counts all four. The interval is wide on purpose.
"""
import subprocess

DOC_GLOBS = ["genesis/docs/superpowers/specs", "genesis/docs/superpowers/plans"]
# Absorption paths we can count vs. cannot. The gap between them IS the interval.
#   counted:   files deleted or moved to held/ (git log --diff-filter=D/R)
#   uncounted: in-place compaction, decompose-to-zero-residue, archive sweeps
ABSORPTION_UNCOUNTED_MULTIPLIER = (1.0, 3.0)  # lo, hi — see basis string

def _git_count(window_days: int, diff_filter: str) -> int:
    if window_days <= 0:
        return 0
    out = subprocess.run(
        ["git", "log", f"--since={window_days} days ago", "--diff-filter=" + diff_filter,
         "--name-only", "--pretty=format:", "--"] + DOC_GLOBS,
        capture_output=True, text=True, check=False,
    ).stdout
    return len({line for line in out.splitlines() if line.strip()})

def generation_absorption_ratio(window_days: int = 28) -> dict:
    generated = _git_count(window_days, "A")
    absorbed_counted = _git_count(window_days, "DR")
    lo_mult, hi_mult = ABSORPTION_UNCOUNTED_MULTIPLIER
    lo_absorb = absorbed_counted * lo_mult
    hi_absorb = absorbed_counted * hi_mult
    def ratio(num, den):
        return float("inf") if den <= 0 else num / den
    # A LARGER absorption denominator gives a SMALLER ratio, so the bounds swap.
    value = ratio(generated, (lo_absorb + hi_absorb) / 2 if absorbed_counted else 0)
    return {
        "value": value,
        "kind": "ratio",
        "confidence": {
            "claim": "estimated",
            "interval": {"lo": ratio(generated, hi_absorb), "hi": ratio(generated, lo_absorb)},
            "basis": (
                f"generation witnessed from git log over {window_days}d "
                f"({generated} added); absorption estimated from {absorbed_counted} "
                f"counted delete/rename events × [{lo_mult},{hi_mult}] to allow for "
                f"in-place compaction, decompose-to-zero-residue, and archive sweeps "
                f"that no ledger counts"
            ),
        },
    }
```

Then in `.claude/scripts/habits-status.py`, add one headline line beside the existing memory-budget block:

```python
from _lib.doc_dynamics import generation_absorption_ratio
r = generation_absorption_ratio(28)
iv = r["confidence"]["interval"]
flag = "⚠" if iv["lo"] > 1.0 else ("~" if iv["hi"] > 1.0 else "✅")
print(f"  doc dynamics: generation/absorption {r['value']:.2f} "
      f"[{iv['lo']:.2f}–{iv['hi']:.2f}] {flag} (28d, absorption estimated)")
```

The three-state flag is the ontology paying its first dividend: `⚠` only when the *whole interval* is above 1.0 (unambiguous overshoot), `~` when the interval straddles 1.0 (we genuinely do not know), `✅` when it is entirely below. A point estimate could not express the middle state, and the middle state is where we most likely are.

- [x] **Step 4: Run tests and eyeball the real number**

```bash
cd /projects/elohim && python3 -m pytest .claude/scripts/_lib/__tests__/doc_dynamics_test.py -v && python3 .claude/scripts/habits-status.py | head -20
echo "EXIT=$?"
```

Expected: tests PASS, and the headline prints a real ratio with a real interval. **Record the actual printed value in the commit message** — this is the evidence the loop closed with real pain, per the habits covenant's evidence rule.

- [x] **Step 5: Commit**

```bash
git add .claude/scripts/_lib/doc_dynamics.py .claude/scripts/_lib/__tests__/doc_dynamics_test.py .claude/scripts/habits-status.py
git commit -m "feat(devspace): generation/absorption ratio with honest interval — first real consumer of the measure ontology" -- .claude
```

---

### Task 6: Canonical-bytes guard + the network boundary, explicitly not crossed

**Files:**
- Modify: `elohim/epr/tests/canonical_bytes.rs`
- Modify: `genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md` (add §"What slice 2 inherits")
- Modify: `genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md` (row 9 gains its slice-1 dependency)

**Interfaces:**
- Consumes: everything from Tasks 1–5.
- Produces: the slice-2 contract — what the network boundary layer may assume, and what it must not.

- [x] **Step 1: Write the canonical-bytes guard (L2's real enforcement)**

```rust
// append to elohim/epr/tests/canonical_bytes.rs
use elohim_epr::measure::*;

#[test]
fn confidence_is_inside_the_canonical_bytes() {
    // L2: if two quantities differ ONLY in their confidence, they must produce
    // different canonical bytes. If they collide, the interval is detachable and
    // an estimate could be narrowed after the fact without changing its address.
    let base = |c: Confidence| Quantity { value: 12.0, kind: MeasureKind::Level, confidence: c };
    let wide = base(Confidence::estimated(Interval::new(8.0, 16.0), "3-week sample"));
    let narrow = base(Confidence::estimated(Interval::new(11.0, 13.0), "3-week sample"));
    assert_ne!(
        serde_json::to_vec(&wide).unwrap(),
        serde_json::to_vec(&narrow).unwrap(),
        "confidence must be inside the canonical bytes, never a detachable sibling"
    );
}
```

- [x] **Step 2: Run the guard**

```bash
cd elohim/epr && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test --test canonical_bytes
echo "EXIT=$?"
```

Expected: PASS. If this test can be made to fail by a serialization change, L2 has no teeth and the design is wrong — treat a failure here as a design bug, not a test bug.

- [x] **Step 3: Write the slice-2 contract into the spec**

Add a section stating exactly what the network boundary layer inherits and what it must still solve:

**Inherited (slice 1 guarantees these hold locally):** every quantity declares its kind; a rate carries its period; confidence rides inside the canonical bytes; folds return intervals and take the weakest claim; honest absence is the degenerate interval.

**NOT inherited — slice 2 must solve each before any network rung:**
1. **Per-fold anonymity** — a rollup over a sparse holon re-identifies its members. k-anonymity floor or DP noise budget per fold level; the noise budget is itself a depleting stock and wants its own `kind: level`. This **blocks** the network rung (commons-holonic row 10).
2. **Correlation-aware interval arithmetic** — slice 1's naive addition over-widens for independent terms (Q1). Safe locally; wasteful at scale.
3. **Cross-peer determinism at a band edge** — the same ULP exposure the `bound_stock` determinism decision names (algedonic phase-2 row 5). Two peers must not disagree on whether an interval crosses a threshold.
4. **Statistical-method application at the ceiling** — a `ComputationAttestation` at a graduated-rigor tier, where a contested narrow interval escalates a tier (commons-holonic row 14). Slice 1 ships inputs only.
5. **Index lenses** — World3 / GNP / GDP / Donut as named lenses over the aggregate (commons-holonic rows 9, 13). Needs 1–4.

- [x] **Step 4: Verify the whole slice together**

```bash
cd /projects/elohim
for c in elohim/epr elohim/epr-rea; do (cd $c && export CARGO_TARGET_DIR="$(cargo-pool key)" && RUSTFLAGS="" cargo test && RUSTFLAGS="" cargo clippy -- -D warnings && RUSTFLAGS="" cargo fmt --check); echo "EXIT[$c]=$?"; done
python3 -m pytest .claude/scripts/_lib/__tests__/ -q
echo "EXIT=$?"
```

Expected: every leg green. Do not accept a piped-through exit code — read each `EXIT[...]` line.

- [x] **Step 5: Commit and record the delta**

```bash
git add elohim/epr/tests/canonical_bytes.rs genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
git commit -m "feat(epr): canonical-bytes guard for inline confidence; slice-2 network contract recorded" -- elohim/epr genesis
```

- [x] **Step 6: Decompose the plan into budget line-items**

```bash
python3 .claude/scripts/memory-kit/decompose.py genesis/docs/superpowers/plans/2026-08-11-measure-ontology-slice1-epr-local-first-plan.md
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | tail
echo "EXIT=$?"
```

- [x] **Step 7: One-line delta in `genesis/manifests/habits.yaml`**

Per the repo covenant, the deliverable is the delta, not a summary. Record what the slice proved with evidence (the actual generation/absorption ratio and interval printed in Task 5 Step 4), not what it intended.

---

## Self-Review (at authoring)

**1. Spec coverage.** measure-family rows 12 (kind), 16 (confidence qualifier), 17 (propagating folds + widen/narrow asymmetry) → Tasks 1, 3, 4. Row 13 (harvest/regeneration) → the ratio in Task 5 is exactly this index applied to the doc corpus. Rows 14 (turnover/coverage) and 18 (uncertainty work-queue) are **deliberately out of slice 1** and recorded as spec open questions Q3 — they need L5 landed first. Rows 9/13/14/15 of commons-holonic are network-layer and are named in Task 6's non-inherited list. The p2p-gate output needs no task: it added no entity.

**2. Placeholder scan.** No TBDs; every code step carries real code; every test asserts a named behavior. Two honest unknowns are stated as such rather than hidden: `thiserror` may already be an `elohim/epr` dependency (Task 1 Step 5 says check), and `Interval::unknown()` summing to infinity is asserted-then-verified rather than assumed (Task 3 Step 4).

**3. Type consistency.** `MeasureKind`, `Period`, `Interval`, `ClaimKind`, `Confidence`, `Quantity` are defined once in Task 1 and used with identical names in Tasks 3, 4, 6. The Python mirror in Tasks 4–5 uses the same wire strings (`level`/`rate`/`ratio`, `estimated`/`witnessed`), and Task 4's parity test reads the Rust source rather than a copied constant.

**4. Known weakness, stated not hidden.** The semantic prior-art lens was unavailable (MemPalace unreachable this session). The lexical lens found the binding prior art and this plan composes from it — but re-run `mempalace_check_duplicate` before Task 2 seals canon, because a near-duplicate ontology spec surfacing after the seal is far more expensive than one found before it.
