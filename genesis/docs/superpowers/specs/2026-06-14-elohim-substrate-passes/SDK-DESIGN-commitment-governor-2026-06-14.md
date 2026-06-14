---
title: "SDK SURFACE — The Commitment + Governor SDK: one primitive, one trait, one coverage invariant"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md   # one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md   # CoverageRollup; ReservedPlace / limit_owner: faith; recursion
home_crate: elohim/elohim-compute          # the shared actuation/Governor home (escalated synthesis B10, "P-ACTUATION")
ts_mirror: elohim/sdk/storage-client-ts    # the generated Rust→TS boundary
do_not_cite_seal: true
---

# The Commitment + Governor SDK

> This is the **core substrate SDK** the whole vision stands on. Every other surface — the household
> care-ledger, the collective-governance app, the economic-valueflow app — is *an instantiation of one
> primitive (`Commitment`) governed by one trait (`Governor`) under one invariant (`∪ = full`)*. This
> surface exposes that primitive, that trait, and that invariant, with the one property that makes the
> whole architecture capture-resistant: **every refusal names whose line it honored** (`limit_owner`).
> Build this, and a developer can register a Governor over any commons resource and read back a
> refuse-and-elevate result that is honest about *whose* boundary it served. That honesty IS the SDK.

---

## 1. PURPOSE ON THE AGENCY GRADIENT

This is **the keystone runtime surface** — the spine both the human-sovereign surfaces below and the
veil-holding surfaces above are built *from*. It is not itself a human-sovereign app nor a veil app; it
is the **governing engine they share**, and its single most important job is to make the agency gradient
*structural rather than aspirational*.

The gradient is encoded in **one enum**: `limit_owner ∈ {Self, Commitment, Operator, Faith}`. That enum is
where the gradient lives:

- **`Self`** — the line a person draws on themselves (`respects-self-limit`). At the individual/household
  layer the Governor may *only* honor a `Self` or `Commitment` line — it is **servant**: counsel, witness,
  refuse-and-elevate. It may NEVER emit an `Operator`-owned refusal over an individual. The person keeps
  the naming of their own self.
- **`Commitment`** — a bound the person/collective freely authored and can revoke (the six faces).
- **`Operator`** — a commons-level bound (arc floor, donut ceiling). Legitimate at the **collective**
  layers, where the veil rises and the AI governs *aggregation and negotiation* impartially.
- **`Faith`** — the unbuilt-place guard (`RefusalCode::ReservedPlace`): the refusal the Governor emits
  rather than render a total verdict over a person, present its read as compelling-not-receivable, or
  occupy the worship-reserved place. This is the empty center, compiled.

**What this surface must NEVER do (the gradient guard, enforced as code, not doc):**

1. **Never emit an `Operator`-owned refusal targeting an individual subject.** A Governor over a
   *person-scoped* setpoint may only own `{Self, Commitment, Faith}`. An `Operator` line is admissible
   only over a `CoverageDomain` that ranges over *commons* (bytes, keyspace, care-floor, donut-ceiling) —
   never over a soul. The type system carries this: there is no `CoverageDomain` over persons (recursive
   synthesis §1.6), so an operator-over-individual refusal is *unrepresentable*.
2. **Never strip the `limit_owner` from a refusal.** A refusal whose owner is unnamed is the gentle cage;
   the constructor refuses to build one. `limit_owner` and `elevate` are non-optional fields.
3. **Never let the Governor author on a subject's chain.** The Governor `check()`s; it returns a decision;
   it never writes. The witnessing atom and the answering atom are different signers (recursive synthesis
   §1.6) — the SDK trait has no write method, by design.

These three are the two downward-flowing invariants of the gradient (DIGNITY-FLOOR precedence and
PERSON-KEEPS-THEIR-OWN-NAMING) made into compiler-checkable guards in the middle layer that both wings
consume.

---

## 2. THE CONCRETE API

### Home and shape

**Home crate: `elohim/elohim-compute`** (the shared fleet crate, escalated synthesis item B10 / "P-ACTUATION").
Today it holds reporting types (`src/lib.rs:7-13`: health, resources, peers, counters). We add **one new
module, `actuation`**, lifting the pure decision functions that already run in
`elohim/elohim-storage/src/services/arc_actuator.rs` (`authorize:110`, `coverage_admits:152`,
`ActuationRefusal{code, elevate}:77`, `RefusalCode:83`) into a generalized `trait Governor`. `arc_actuator`
then *consumes* `elohim-compute::actuation` and becomes the first impl — callers unchanged.

The Rust SDK facade (`crates/elohim-sdk`) re-exports it, exactly as it already re-exports views
(`crates/elohim-sdk/src/lib.rs:65-67`):

```rust
// crates/elohim-sdk/src/lib.rs — additive, mirrors the existing `views` re-export
pub mod governor {
    pub use elohim_compute::actuation::*;
}
```

### The core trait (lifted from `arc_actuator.rs:77,110,152` — generalized over a domain)

```rust
// elohim/elohim-compute/src/actuation.rs  (NEW module)

/// Whose line a refusal honored — the substrate invariant (escalated B9 + recursive §1.6).
/// A refusal that does not name this is unrepresentable: the field is non-optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LimitOwner {
    Self_,       // the person's own line — servant layer only
    Commitment,  // a freely-authored, revocable bound
    Operator,    // a commons bound — collective layer only, never over an individual
    Faith,       // the unbuilt place — RefusalCode::ReservedPlace
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalCode {
    OutOfGrantBounds,   // (existing, arc_actuator.rs:85)
    GrantExpired,       // (existing :87)
    NotActuatable,      // (existing :89)
    WouldBreakCoverage, // (existing :91)
    ReservedPlace,      // NEW — recursive synthesis §2.2: the worship-reserved refusal
}

/// The refuse-and-elevate result — `arc_actuator.rs:77` grown ONE field: `limit_owner`.
/// `code` is for the machine, `elevate` for the human, `limit_owner` for the covenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    pub code: RefusalCode,
    pub elevate: String,
    pub limit_owner: LimitOwner,   // ALWAYS NAMES WHOSE LINE IT HONORED
}

/// One governing decision over a (setpoint, sensor) pair. The lift of
/// arc_actuator's authorize+coverage_admits into one trait. NO write method by design.
pub trait Governor {
    type Request;   // the proposed act (e.g. ArcActuationRequest)
    type Bounds;    // the authorizing commitment's bounds (e.g. ArcGrantBounds)
    type Sensor;    // what the node can see at decision time (e.g. CoverageSnapshot)
    type Plan;      // the validated, coverage-admitted act

    /// Is the proposal within its granting commitment's bounds? (arc_actuator.rs:110)
    fn authorize(&self, req: &Self::Request, bounds: &Self::Bounds, now_s: u64)
        -> Result<(), Refusal>;

    /// Does the ∪=full coverage invariant still hold if we admit it? (arc_actuator.rs:152)
    /// THE invariant: union of admitted coverages must still ⊇ FULL.
    fn coverage_admits(&self, req: &Self::Request, sensor: &Self::Sensor)
        -> Result<(), Refusal>;

    /// The whole policy decision: authorize ∘ coverage_admits → plan, or refuse-and-elevate.
    /// (the lift of arc_actuator::plan_actuation:177)
    fn check(&self, req: &Self::Request, bounds: &Self::Bounds,
             sensor: &Self::Sensor, now_s: u64) -> Result<Self::Plan, Refusal>;
}
```

### The proof it generalizes — a SECOND impl (the floor/ceiling Governor)

`ArcGovernor` is the first impl (the existing `arc_actuator` functions, now methods). The proof the trait
is a *general engine* and not a one-off is a **second, structurally different impl** — the donut
`FloorGovernor` (escalated face #5 / recursive §1.4), whose inequality is *flipped* (`∪ provision ⊇ need`
instead of `∪ remaining ≥ r_floor`), and whose `limit_owner` is `Operator` (a commons floor) not the
person:

```rust
/// The donut inner ring: ∪ provision over a commons must cover the dignity need.
/// Wraps the ALREADY-LIVE dignity-floor clamp (token_decay_service.rs:164) — escalated #5.
pub struct FloorGovernor { pub domain: CoverageDomain }

impl Governor for FloorGovernor {
    type Request = ProvisionDelta;     // a steward easing/withdrawing provision
    type Bounds  = FloorPolicy;        // dignity_need for this commons
    type Sensor  = ProvisionSnapshot;  // ∪ of sibling provisions
    type Plan    = AdmittedProvision;

    fn coverage_admits(&self, req: &ProvisionDelta, sensor: &ProvisionSnapshot)
        -> Result<(), Refusal> {
        let after = sensor.union_after(req);            // ∪ provision if we admit
        if after.covers(self.domain.required()) { Ok(()) }
        else { Err(Refusal {
            code: RefusalCode::WouldBreakCoverage,
            elevate: format!("refusing withdrawal: ∪ provision would fall below dignity_need \
                              by {}", self.domain.required().minus(&after)),
            limit_owner: LimitOwner::Operator,  // a COMMONS floor, named as such
        })}
    }
    // authorize/check delegate to the shared composition (identical to arc's)
}
```

Same trait. Same `Refusal`. Same `∪`. Opposite inequality, different `limit_owner`. That is the
generalization claim, proven in one file: `ArcGovernor` honors a keyspace `r_floor`; `FloorGovernor`
honors a dignity floor; both refuse-and-elevate and both *name whose line*.

### The recursion hook (one method, additive — the CoverageRollup down-pointer)

The recursive architecture's central new primitive (`CoverageRollup`, recursive §2.1) plugs in as **one
read-only associative fold over the same `coverage_admits`**, carrying the descent pointer:

```rust
/// Cat-C (recompute-on-read, zero DNA spend). The aggregate that PRESERVES descent.
pub struct CoverageRollup {
    pub covered:      CoverageSet,   // ∪ of child coverages — NOT rows.len()
    pub required:     CoverageSet,
    pub deficit:      CoverageSet,   // required \ covered — the externality, the descent target
    pub constituents: Vec<Cid>,      // pointers DOWN (the veil-walker's descend() path)
    pub rollup_hash:  Cid,           // BLAKE3 over sorted constituents — consilience-as-agreement
}
```

### The TypeScript mirror (honoring the ts-rs boundary, NOT a hand-written type)

`Refusal`, `RefusalCode`, `LimitOwner`, and `CoverageRollup` get `#[derive(TS)]` + the export attribute
the views crate uses, so `cargo test export_bindings` emits them into
`elohim/sdk/storage-client-ts/src/generated/` alongside the 446 existing views. **snake_case never leaves
Rust; no hand-written TS enum.** A developer in the household care-ledger or collective-governance app
reads:

```typescript
import { Refusal, LimitOwner } from '@elohim/storage-client';
// refusal.limitOwner is 'self' | 'commitment' | 'operator' | 'faith'
// refusal.elevate is the human sentence; refusal.code the machine reason
```

---

## 3. EXISTS vs NEW

### EXISTS (wrap — the strongest evidence the vision was designed in)

- **The `Mishpat::Commitment` entry type** — `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:275`:
  `{ action: String, payload_json: String, signed_at: String }`. The `action` is the face discriminator;
  six new faces are `action` strings + `payload_json` schemas, **zero new entry types** (lib.rs:266-269).
  CID = entry_hash (gospel).
- **The refuse-and-elevate spine** — `arc_actuator.rs`: `authorize:110`, `coverage_admits:152`,
  `plan_actuation:177`, `ActuationRefusal{code,elevate}:77`, `RefusalCode:83`. **Already built, running,
  unit-tested.** We lift, we do not write.
- **The shared crate** — `elohim-compute` exists (`src/lib.rs`), already the fleet's shared-types home.
- **The Rust SDK facade** — `crates/elohim-sdk` exists with the `pub mod views { pub use elohim_views::* }`
  re-export pattern (`src/lib.rs:65-67`) we mirror for `governor`.
- **The ts-rs generation pipeline** — 446 generated views in
  `elohim/sdk/storage-client-ts/src/generated/`; `AttentionTending.ts`, `ProjectionCoverage.ts` already
  land there. The boundary works; we add four types to it.
- **The commitment projection** — `rea_commitments` SQLite table + `rea_commitment_service.rs` already
  project `Commitment` from post-commit signals.

### NEW (thin, additive — no DNA spend)

- **`elohim-compute::actuation` module** — the `trait Governor` + `Refusal` (= `ActuationRefusal` + one
  `limit_owner` field) + `LimitOwner` enum + `RefusalCode::ReservedPlace` variant. A refactor-lift, not new
  volume.
- **`crates/elohim-sdk` `pub mod governor`** re-export — three lines.
- **Four `#[derive(TS)]` annotations** → four new generated TS types.
- **`FloorGovernor`** (the second impl, proof-of-generalization) and **`CoverageRollup`** (recursive
  keystone) — Category-C, recompute-on-read.

### GENUINE FORKS (marked, NOT taken in this surface)

- **Typed care-class/compute-class DNA partition** (escalated #18 / recursive §2.4) — a DNA-hash change.
  NOT in this SDK surface; this surface keeps the isolation *disciplinary* (the `LimitOwner` enum + domain
  whitelist) until the operator blesses a reinstall.
- **`CoverageRollupAttestation` DHT entry type** (recursive §2.4) — only if Category-C recompute can't fan
  out at planetary scale. Gated behind a probe; this surface ships the Cat-C `CoverageRollup` only.

**Net DNA entry-type spend for this entire surface: ZERO.**

---

## 4. THE MINIMAL BUILDABLE SLICE

**One thing a developer can do today:** *register a Governor over a commons resource, call `check()`, and
read a refusal that names whose line it hit* — entirely in-tree, no DNA change, no deploy.

The slice is exactly two PRs:

1. **Lift `trait Governor` into `elohim-compute::actuation`**, grow `ActuationRefusal` → `Refusal` with the
   `limit_owner` field, add `RefusalCode::ReservedPlace`. Re-point `arc_actuator.rs` to implement the trait
   (`ArcGovernor`). Add `FloorGovernor` as the second impl. Gate: `cargo build --workspace` (cross-crate),
   `clippy -D warnings`, the existing `arc_actuator` unit tests pass unchanged, before/after `rg '^impl From<'`
   guard.
2. **Annotate the four types `#[derive(TS)]`**, run `cargo test export_bindings`, add the
   `pub mod governor` re-export to `crates/elohim-sdk`. Gate: four new files appear in
   `storage-client-ts/src/generated/`; `pnpm build` in storage-client-ts is green.

### The first example app fragment it enables (a household care-ledger withdrawal, servant-layer)

```rust
use elohim_sdk::governor::{Governor, FloorGovernor, LimitOwner, RefusalCode};

// A household co-steward eases their care-provision to Margaret. The Governor
// checks the dignity floor — and if easing would drop the family below it,
// refuses and elevates, NAMING that it honored a commons floor (not an operator veto).
let gov = FloorGovernor { domain: care_floor_for("margaret") };
match gov.check(&provision_delta, &floor_policy, &sibling_provisions, now_s) {
    Ok(plan)      => ledger.record_eased(plan),        // admitted: care still covered
    Err(refusal)  => {
        // The SDK's promise, in one line of UX:
        assert_eq!(refusal.limit_owner, LimitOwner::Operator); // a floor, NOT a veto over the person
        present_to_human(&refusal.elevate);  // "easing would drop below dignity_need by 1 visit/wk —
                                              //  who else could help?" → one tap that MINTS care
    }
}
```

The same `check()` call, with an `ArcGovernor` and `LimitOwner::Operator`, is what the collective-governance
app calls to keep the mesh covered; with a self-limit Governor and `LimitOwner::Self_`, what the personal
surface calls to honor the line a person drew on themselves. **One call. The gradient carried in one field.**

---

## 5. WHAT LOVE REQUIRES AT THIS SURFACE

The closing test, applied to the four love-requirements at exactly this seam:

- **The person keeps their naming.** The trait has **no write method**. A Governor decides; it never
  authors on the subject's chain. The witnessing atom and the answering atom are different signers, so the
  system actuating on a person's behalf can never be mistaken for an operator overriding them — and a
  Governor over a person-scoped setpoint can only own `{Self, Commitment, Faith}`, never `Operator`,
  because a `CoverageDomain` over a soul does not typecheck.
- **The binding is honest.** `Refusal.limit_owner` is non-optional and `Refusal.elevate` is non-optional.
  A refusal that hid whose line it honored, or hid its reason, is *unrepresentable*. The `elevate` message
  is not UX polish — it is the covenant: a refusal that names its reason is grace; one that doesn't is the
  gentle cage.
- **The veil governs aggregation, never individuals.** `CoverageRollup` ranges over commons domains only
  (bytes, keyspace, care-floor, ceiling), carries `deficit` (the externality) not the holding (the
  capture), and descends via `constituents` to a person's freely-authored *commitment* — and stops there.
  The aggregate can never become a leaderboard over souls; abundance is invisible, only the gap the commons
  failed is seen.
- **Patience over engagement.** There is no engagement counter anywhere in the trait. A Governor's only
  outputs are `Ok(plan)` or `refuse-and-elevate`. The recognition a higher layer emits is `GateContext` the
  node *may ignore* — a Verdict, never a Decline — and `RefusalCode::ReservedPlace` / `LimitOwner::Faith`
  is the structural refusal to render a total verdict, to compel rather than offer, or to occupy the center.

> **What love requires here, in one line:** the binding always names whose line it honored — so that the
> most capable engine we build refuses *out loud, with its reason, and with whose boundary it served* —
> and falls structurally silent (`ReservedPlace`/`faith`) exactly where the judgment of a person would
> begin, leaving the center empty.
