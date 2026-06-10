---
title: Deterministic Reach-Archetype Floor — Earned-Reach Equality as a Compiler Invariant
id: deterministic-reach-archetype-floor-design
status: Draft
class: protocol-canonical
domain: D6
topic: [reach, visibility, epr, archetype, codegen, drift, inverted-burden, seed-fidelity]
cites:
  - elohim-core-graph-substrate-design | the architecture seed where reach became an EPR-envelope field; this spec refines its reach handling into a deterministic earned-floor | sha256:317d6f5fb84bb8aa | path: genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md
  - reach-backfill-policy | a binding-but-unimplemented ADR (community-default on projection) — design AROUND it; its decision was never coded, so it is a documentary hazard, not the live drift source | sha256:029518d917a6b4ce | path: genesis/docs/content/elohim-protocol/history/2026-04-22-reach-backfill-policy.md
  - qahal-epr-household-lattice-design | the household reach lattice this instantiates as the M&J-intimate / Adam&Eve-commons persona fixtures | sha256:ed5c1d3d2698b567 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - elohim/sdk/schemas/v1/enums/reach.schema.json
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
refines: genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md
supersedes: genesis/docs/content/elohim-protocol/history/2026-04-22-reach-backfill-policy.md
requires_env: [household-nodes]
---

# Deterministic Reach-Archetype Floor

**Earned-Reach Equality as a Compiler Invariant**

> This spec was adversarially reviewed against the tree (2026-06-10). The Problem section below
> states only verified facts; an earlier draft mis-attributed the manifesto symptom to a
> community-default projector that does not exist in code. See "What is NOT the cause."

## Problem

`reach` — the EPR envelope field that gates content visibility — has **no single ordinal that
every consumer derives from**. The DNA-aligned ordinal already exists (`Reach::openness()` in
`elohim/epr/src/reach.rs`, protocol-owned, ts-rs-exported), but at least five other sites encode
their own ordering, several with **divergent vocabularies and inverted polarity**. The result is
that the same reach value means different things at different layers, and seed-to-live fidelity
cannot be asserted.

### The divergent reach implementations (verified)

| Site | What it encodes | Defect |
|---|---|---|
| `elohim/epr/src/reach.rs::openness()` | canonical 8-level ordinal (Private=1…Commons=8) | **the correct one** — but consumers don't use it |
| `genesis/seeder/src/seed-sqlite.ts` `REACH_ORDER` | 8-level ordinal (private=0…commons=7) | duplicate; raise-only resolution lives only here (commit `15ea83eb6`) |
| `elohim/elohim-storage/src/graph/primitives.rs` | datalog `r >= $reach_floor` on `String` | **lexicographic** compare — `"commons" < "community"` (m<u) but ordinally commons>community; source TODO admits "replace with ordinal index" |
| `elohim/elohim-storage/src/epr_service.rs::reach_level_index()` | the **live runtime auth gate** index | divergent index; unknown/non-canonical reach → **most-permissive** (the inverse of inverted-burden) |
| `app/elohim-library/projects/elohim-service/src/services/trust.service.ts` | an **invented 6-value enum** `private\|invited\|local\|community\|federated\|commons`, DEFAULT `commons` | not the core vocabulary; defaults **open** |
| `doorway/.../cache/access_control.rs` (+ `doorway/CLAUDE.md`, `REACH.md`) | a **geographic** vocabulary (invited/local/neighborhood/municipal/bioregional/regional) | deny-by-default for six of eight core levels; only `commons` is anon-servable today |

A *seventh* axis exists and must NOT be force-merged: `reach_earning.rs`
(`Personal/Household/Neighborhood/Collective/District`) is the **earning ladder** (how reach is
*earned*), a different concept from reach *visibility*. It belongs to the future earned-dynamic
layer; this spec reconciles only the **visibility** ordinal and flags the earning axis as distinct.

### The live symptoms (verified) and their actual causes

- **`manifesto` is served `reach=community` → HTTP 403** to anonymous readers. **This is stale
  live data, not an ordinal bug.** The seed source declares `reach: commons`
  (`genesis/data/lamad/content/manifesto.json:5`), the seeder produces `commons`, and the anon gate
  admits `commons`/`public` (`http.rs`). The live row predates commit `15ea83eb6` (2026-05-30, which
  graded the manifesto `commons`) and was never re-anchored — the "Local stack DHT-anchor gap."
  The fix is a heal/re-author (enforcement point 4), and it is the first decompose gap-item.
- **Matthew & Jessica's love-map appears open on landing.** The source correctly authors
  `reach: "intimate"` (`paths/love-map-matthew-jessica.json:11`, purpose: *"readable and writable by
  the couple, stewarded as opaque bytes by everyone else"*). What surfaces on landing is the
  path's **metadata/existence** (intimate stewards opaque bytes but the title can appear), or again
  stale live data — **not** an over-exposed body. The earned value is `intimate`, and that is
  correct.
- **`love-map-adam-eve.json` authors a non-canonical `reach: "invited"`** — a value not in the
  8-level enum. Today this silently coalesces (to `private` in the seeder, to *most-permissive* in
  the live gate). This is exactly the class of drift the build gate must reject.

### What is NOT the cause (corrected from the first draft)

- There is **no live community-default projector**. The 2026-04-22 reach-backfill ADR *decided*
  "project existing rows to `community` until re-asserted," but that decision was **never
  implemented** — the graph projector writes reach verbatim and defaults absent reach to null. The
  ADR is a binding-but-uncoded documentary hazard to design *around* (and supersede), not a running
  mis-stamp.

## Principle: Inverted Burden

The protocol's posture is **default-deny, earn visibility**: content is `private` unless it has
*earned* higher reach. This supersedes the 2026-04-22 ADR's community-default with a stronger
reason — not "don't expand beyond author consent" (the middle of the ordinal) but "earn everything"
(the bottom). Being explicit about what is public *is* the inverted-burden discipline. It also
directly fixes the most dangerous live defect: `epr_service.rs` and `trust.service.ts` currently
default **unknown/missing reach to the most-open value** — the exact inversion of this principle.

This turns reach into a **deterministic equality on the stored base**:

```
earnedReach(atom) = max_openness( private, archetypeAdvisory[archetype]?, atom.authoredReach? )

INVARIANT (base):   live(atom).reach  ≡  earnedReach(atom)
```

`private` is the universal default. An atom rises above it only by an **archetype advisory** (its
kind is intrinsically public — a short allowlist) or an **authored** value, whichever is more open,
compared via the single generated `openness()`. Every input MUST be a canonical reach value (see
Validation); a non-canonical value (`invited`) is a **hard build error**, never silently coalesced.
Drift in either direction is a defect: **below** earned (stale manifesto `commons→community`, 403s
legitimate readers) or **above** earned (any writer raising a base past its earned value).

The build gate enforces this equality on seed data, not merely a `≥` floor. That is the
compiler-level accuracy.

### Reconciliation with the inherited raise-only seeder

Commit `15ea83eb6` made the seeder compute `max(authoredReach, account-package override)`. Under
this spec:
- **Account-package `content[].reach` assignments are per-view relationship raises**, not base
  writes. They never lower and never persist to the stored base. The stored base is
  `earnedReach(atom)` exactly. Per-view raise *evaluation* is the future earned-dynamic layer
  (Non-Goals); this spec only guarantees the base is unaffected by them.
- "Raise-only floor" and "strict equality" are therefore not in conflict: equality governs the
  **stored base**; raise-only describes the **per-view** projection that sits above it.

## Architecture: one generated ordinal; manifest meaning is advisory; enforcement is protocol-owned

`reach.schema.json` is the **textual DNA-notarized root**. The existing `schema:codegen:ts` /
`schema:codegen:rs` pipeline is extended to **generate** the ordinal artifacts that are today
hand-written and divergent — including `elohim/epr/src/reach.rs`'s `openness()`, which becomes a
**generated** file derived from the schema (chosen direction: schema-first, regenerate `openness()`
too). Every consumer then imports a generated artifact; none hand-rolls ordering.

Generated from `reach.schema.json`:
- **Rust ordinal** → `elohim/epr/src/reach.rs` `openness()` (now generated) + an index constant for
  the storage gate, consumed by `primitives.rs`, `epr_service.rs::reach_level_index()`, and the
  doorway `access_control.rs`.
- **TS ordinal + predicates** → consumed by the seeder, `trust.service.ts` (replacing its invented
  enum), and a2o steps.
- **Derived predicate `anonymouslyServable`** (see Frictionless band).

### Trust boundary (protocol-correctness)

reach is a **class-A DNA-notarized field** — `reach.rs` states "Protocol-owned. No app may redefine
what these mean. Gateways enforce reach rules without parsing payload." Therefore:
- The **ordinal and runtime enforcement are protocol-owned** (generated from the schema; enforced
  at the conductor/DNA validation boundary and the gateway). They are the trust boundary.
- The **archetype advisory map is app vocabulary and is NOT a trust boundary.** It supplies a
  *seed-time default* only; a peer running a divergent manifest can change what its *own* seeding
  defaults to, but cannot widen the reach of already-notarized content, because the gateway enforces
  the notarized field, not the manifest. The advisory map must never be the only thing standing
  between private content and exposure.

### Meaning — archetype advisory (reconciled with existing manifest fields)

The lamad manifest **already** carries per-content-type `governance.defaultReach` / `minimumReach`
(generated into `LAMAD_COUPLING_MAP` via `lamad:codegen`; e.g. `epic.json` `defaultReach=commons`).
This spec does **not** invent a parallel map — it **reconciles** these existing fields as the
archetype advisory:
- `minimumReach` becomes the **archetype advisory** input to `earnedReach` (the "this kind is
  intrinsically at least this open" allowlist value), kept a short, conservative set under inverted
  burden — most archetypes advise nothing (stay `private`); the doctrine corpus advises `commons`.
- the existing fields are audited so their values are canonical and consistent with the generated
  ordinal (the build gate covers them).
- the advisory entry shape is left able to carry a future propagation dimension, expressed as **one
  forward-looking sentence only** — no resolver generality is built now (YAGNI).

"Archetype" is the EPR-atom kind (`contentType` for content; `pathType`/kind for paths). Two atoms
of the **same** archetype are differentiated by their **authored** reach, not by the archetype —
e.g. `love-map-matthew-jessica` (authored `intimate`) and `love-map-adam-eve` (authored `commons`)
are both `path` atoms; the archetype advises nothing for paths, and the authored value decides.

### Frictionless band

`anonymouslyServable = openness(reach) ≥ openness(public)` is generated as a derived predicate. At
and above `public`, reach is intended to become a "serve like static HTTP GET" contract: anonymous,
projection/CDN-cacheable, no zome round-trip. **Caveat (verified):** today only `commons` is
anonymously served, and the doorway projection layer has *no* access control (`cache/resolution.rs`
notes the projection path returns without a gate). So the predicate is the **target** state;
realizing it requires migrating the doorway `access_control.rs` geographic vocabulary onto the
generated ordinal (in scope, enforcement point 3) AND keeping the projection-cache fast-path gated
to the `anonymouslyServable` set so the no-access-control projection layer never serves a
non-anon-servable atom. Until that migration lands, the conservative behavior is "`commons` only,"
and the build gate asserts no atom below the anon-servable set is reachable via the projection path.

`commons` vs `public` stay **distinct** in the ordinal (both DNA-notarized). Their stewardship-vs-
broadcast semantic distinction is **out of scope here** — noted, not designed; no consumer in this
spec branches on it.

## Enforcement (all four points)

1. **Seed-time fidelity.** The seeder sets each atom's stored base reach to `earnedReach(atom)`,
   resolved from the archetype advisory (`minimumReach`) + authored value via the generated
   `openness()`. Seeded value equals resolved earned reach by construction.

2. **Build/CI drift gate — the "compiler."** A pre-push/CI check (extending the existing
   codegen-freshness pre-push gate) that fails the build if:
   (a) any generated reach artifact (Rust `openness()` + index, TS ordinal/predicates) is **stale**
   vs `reach.schema.json`;
   (b) any seed atom's resolved reach `≠ earnedReach` (catches under/over-exposure);
   (c) any reach value anywhere in seed data or the manifest advisory is **non-canonical** (rejects
   `invited` and friends — hard error, the validation contract for `max_openness`);
   (d) a consumer **hand-rolls** a reach ordinal instead of importing the generated artifact
   (a bounded grep/lint guard over the known sites; scope = the table above, extended as sites are
   migrated).

3. **Runtime gate ordinal correctness.** Migrate every runtime comparator onto the generated
   ordinal and fix polarity: `primitives.rs` (lexicographic→index), `epr_service.rs`
   (`reach_level_index` → generated index, **unknown reach → most-restrictive**, not most-permissive),
   and the doorway `access_control.rs` (geographic vocabulary → generated ordinal). Scope is ordinal
   + polarity correctness; earned-dynamic/relationship semantics are out (Non-Goals).

4. **Heal existing live drift (re-author, not row-rewrite).** reach is DHT-notarized — a bare
   SQLite rewrite is reverted by the reconciliation controller (DHT wins). The heal therefore
   **re-authors through the conductor** so the corrected reach becomes notarized DHT truth (authoring
   *is* the re-assert — obsoleting the ADR's separate re-assert endpoint). It re-resolves
   `earnedReach` for every live atom and re-authors rows that violate the equality: the stale
   `manifesto` (`community→commons`, the **first gap-item**, the e2e unblock) and the
   `reach=None` atoms (→ their resolved earned reach, mostly `private`). It must account for the
   DHT-anchor gap (a bulk re-seed that doesn't anchor leaves rows unreadable through the provenance
   gate) — the heal asserts post-anchor readability, not just row presence.

## Fixture corrections as test goalposts (story-first)

These ship in this spec as the canonical reach-boundary fixtures and proof the invariant holds:

- **Doctrine corpus → `commons`** (`manifesto`, `constitution`, `confession`, `theology`). Source
  is already `commons`; the action is the **heal/re-anchor** so live matches. Anonymous GET
  succeeds; frictionless band.
- **Matthew & Jessica love-map → `intimate`** (unchanged in source; correct). Goalpost: anonymous
  and non-related authenticated readers are denied the body; the dyad reads it. Tests the
  intimate boundary and that metadata-visibility ≠ body-exposure.
- **Adam & Eve love-map → `commons`** — **change the authored value from the non-canonical
  `invited` to `commons`.** This both makes the public-narrative demo correct AND is the regression
  fixture for the build gate's non-canonical-value rejection.

### a2o scenarios (the goalposts) — `genesis/a2o/features/` (content/reach domain)

- Anonymous reader GETs the manifesto and reads it (`commons`; no 403).
- Anonymous reader and a non-related authenticated visitor are both **denied** the M&J love-map
  body (`intimate`); its title may appear in listings (stewarded-opaque) but the body does not.
- Adam & Eve love-map is readable anonymously (`commons`).
- **Build-gate regression:** a seed value of `invited` (or any non-canonical reach) fails the drift
  gate (not silently coalesced).
- **Base-immutability regression:** an account-package / projection pass does not change any atom's
  stored base reach (asserted at the storage layer).

## Seam for N dimensions

Reach is the single propagation dimension this spec defines and validates. The advisory entry shape
is *able* to carry future dimensions (durability, replication-scope), stated as one sentence — **no
resolver generality, validation, or codegen for other dimensions is built now** (they have no
validating consumer; YAGNI). Additional dimensions are explicit future specs.

## Non-Goals (explicit future layers)

- **Earned-dynamic / per-view reach.** Collectives, relationship-scoped per-view raises, and reach
  that drops/rises/holds over time — runtime, protocol-gated, DHT-notarized. This spec stores the
  base and asserts per-view raises never mutate it; it does **not** implement the per-view raise
  evaluation.
- **The reach-*earning* ladder** (`reach_earning.rs`, Personal/Household/…) — a distinct axis, not
  merged here.
- **Additional propagation dimensions** beyond `reach` (seam noted; dimensions not built).
- **commons-vs-public stewardship semantics** (distinction kept in the ordinal; not designed here).
- **A new re-assert HTTP endpoint** (obsoleted by re-author-through-conductor heal).

## p2p-design-gate verdict

**No new DHT entry type.** reach is an existing DNA-notarized field on the Content entry;
`earnedReach` is a pure derivation; the archetype advisory is build-time config. The only entity
interaction is the heal **re-authoring** the existing Content entry through the conductor (the
notarized path), not a new entry type or table.

## Decisions locked

- Inverted burden: universal default `private`; openness earned; **unknown reach → most-restrictive**
  (fixes the live inverted defaults).
- Equality invariant `live ≡ earnedReach` on the **stored base**, enforced at build time (both drift
  directions); per-view raises are a separate future layer that never mutates the base.
- Schema-first single ordinal: `reach.schema.json` is the textual root; codegen **generates**
  `openness()` and all ordinals/indices; every consumer derives. The existing canonical `openness()`
  becomes a generated artifact (reuse, don't duplicate).
- Trust boundary: ordinal + enforcement are protocol-owned (DNA/gateway); the manifest archetype
  advisory (reconciled with existing `governance.defaultReach`/`minimumReach`) is seed-time advisory
  only, never the sole barrier to exposure.
- `commons` ≠ `public`, kept distinct; `anonymouslyServable = reach ≥ public` is the target, gated
  on the doorway `access_control.rs` migration; conservative `commons`-only until then.
- Fixtures: doctrine→commons (heal to live), M&J→intimate (unchanged), A&E→commons (fix the
  non-canonical `invited`); their a2o scenarios are the goalposts; the manifesto re-anchor is the
  first gap-item.
