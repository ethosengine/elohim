---
title: "Middot — the Measure Primitive (Honest Weights and Measures over EPRs)"
id: middot-measure-primitive-design
tier: spec
status: Draft
created: 2026-08-04
maintainers: Matthew Dowell + Claude
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR first-measure-fold-cache-shipped
topic:
  - measures
  - middot
  - lens
  - governance
  - rea
  - observation
  - memoization
  - aggregation
  - code-review
informed-by:
  - .claude/epr-meta/policies.yaml
  - .claude/scripts/_lib/epr_meta.py
  - elohim/brit/Cargo.toml
cites:
  - eprfs-witnessed-interaction-primitive | The landed parent primitive — witnessed events on object CIDs, peer-validated, REA-aggregated; middot inherit its witness-dont-pay and denominate-in-the-witnessable commitments | sha256:6a24773ffd7b83f4 | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
  - plural-mishpat-lenses-over-epr-design | The plurality leg — lens↔EPR bindings plural by construction, observation never collapses, T1/T2 two-layer law; this spec re-reads its lens as a composition of middot plus a contract half | sha256:ab0055896398ef95 | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - lens-version-dag-epr-policy-dependency-design | Version-pin discipline measures inherit — a Measure version is a declared dependency, never recency | sha256:62e0f37f8f57c0ed | path: genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - quilt-evidence-temperature-composition-design | Source of the non-collapsing evidence vector (confirmed/witnessed/claimed) that fold attestations carry, and of the never-sum-into-an-unlabeled-number law applied to measure families | sha256:d278e960b5c8a15d | path: genesis/docs/superpowers/specs/2026-07-24-quilt-evidence-temperature-composition-design.md
  - sense-respond-governance-classifier | Names the common-carrier gap (evaluate accumulates, combine collapses) this spec deliberately leaves to its carrier-extension slice | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - resilience-facings-select-fold-aggregate-design | The select→fold→aggregate machinery middot folds ride when aggregating per-family up reach/VSM tiers | sha256:738c9220d105e9e4 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - resilience-dimensions-proof-suite | Source of the honest-absence lesson — unmeasured must render differently from measured-zero in the per-family measurement vector | sha256:a89f58ec4906e152 | path: genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
  - reach-ontology-vocabulary-split-spec | The reach vocabulary discipline governing which tier ladder folds aggregate along | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - wisdom-layer-floor-ceiling-judgment-culminating-design | The floor/ceiling authority axis this spec makes structural — measures are ceiling-side observation, teeth arrive only via lens contract halves | sha256:f5d694c382a76c1f | path: genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
---

# Middot — the Measure Primitive (Honest Weights and Measures over EPRs)

> **One-line:** a *measure* is a named, versioned, content-addressed observation procedure —
> with a declared family, lineage, unit, and environment-sensitivity — whose applications to
> EPR subjects memoize as recompute-verifiable folds that anyone can reuse, attest, compare,
> and aggregate up reach tiers; measures never carry teeth — governance lenses compose them.

## 0. Naming

**Middot** (singular *middah*, Hebrew מִדָּה — measure, attribute, dimension) is the
**project-internal name** for this primitive family, joining the pillar vocabulary
(lamad, qahal, shefa, mishpat, brit). It carries the Tanakh honest-weights-and-measures
lineage deliberately: *"You shall do no wrong in judgment, in measures of length or weight
or quantity. You shall have just balances, just weights"* (Lev 19:35-36; also Deut 25:13-16,
Prov 11:1 — "a false balance is an abomination to the LORD, but a just weight is his
delight"). Honest measurement is justice literature, not metrology trivia — which is exactly
the claim this primitive makes: a community's ability to trust its shared measurements is a
Mishpat concern.

**Code and schemas use English, feature-specific names**: the artifact is a `Measure`, its
memoized application a `MeasureFold` (or the consuming feature's own noun), its peer
verification rides the existing `Attestation` machinery. `middot`/`middah` appears in prose,
specs, and internal discussion — never in an enum, table, route, or generated type.

## 1. Problem

Three pressures, one missing primitive:

1. **Conditional strictness has no home.** We want the higher resolution of clippy-pedantic
   findings during `/code-review` without pedantic gating normal development. Today every lint profile is
   statically bound (brit's workspace pedantic with empirically-counted allows; conductor's
   blanket allow; six unrelated ESLint configs). "Temporarily raise, then fall back" would be
   a mode switch with two configs that drift.
2. **Measurement results are not shared.** "The pedantic result of commit X" is recomputed by
   every developer who wants it, or never computed. There is no content-addressed, memoized,
   commons-benefiting derivation cache for *any* measurement.
3. **The repo has five-plus verdict/measurement vocabularies with no common carrier**
   (`deny/ask/inject/measure`, `permit/refuse/refer`, `Allowed/Blocked/Pending`,
   `legitimate/drift/abstain`, `confirmed/witnessed/claimed`, the scoreboard's six verdicts) —
   the severity × authority × evidence tuple is written down everywhere and computed with
   nowhere (already observed by the sense-respond classifier spec).

And the generalization the primitive must serve from birth: lint findings, compute cycles,
listening minutes, view counts, bytes stored, watts consumed, and time spent are **the same
shape** — a named procedure applied to a content-addressed subject yielding a result whose
trustworthiness is an economic question. Substrate signals must aggregate up the viable-system
recursion tiers (household → neighborhood → … → global commons; the weave arc's Beer/VSM
"Freedom Machine" frame) without a platform capturing the ledger.

## 2. Position in the existing ontology (what this spec does NOT re-invent)

The repo has already converged on a two-plane aggregation ontology with opposite laws, plus
an authority discriminator between them:

- **Enforcement plane — collapses by max over a total order.** `combine()` keeps one winner
  from `_SEVERITY`; mishpat's `PRECEDENT_BINDING` ladder; `Reach::openness()`.
- **Observation plane — forbidden from collapsing.** The quilt spec's
  `confirmed/witnessed/claimed` evidence vector ("must not sum into an unlabeled number");
  the plural-lenses spec's "observation is always plural… folds do not collapse; teeth
  resolve only on hard conflict"; `select → fold → aggregate` facings.
- **Floor/ceiling — whether a signal binds at all**, orthogonal to its severity (`role:
  floor|ceiling|lens` bindings; the scoreboard's `env-gated → ceiling` vs `code-red → floor`;
  `shouldEnforce` in the sophia Sonar profile selector). Authority is a precondition on rule
  *authorship* (the agency charter), never a combine-time input.

Three legs of the middot primitive already exist and are inherited, not rebuilt:

| Leg | Home | What is inherited |
|---|---|---|
| The witnessed event | eprfs witnessed-interaction primitive (2026-07-15, landed) | local witness → peer-validated → REA-aggregated on object CID; **Commitment I** *witness, don't pay* (no measurement auto-mints value/reach); **Commitment II** *denominate in the witnessable* (metered joules / delivered bytes count; self-reported attention is advisory forever) |
| Plurality over one subject | plural-mishpat-lenses spec (2026-06-27) | lens↔EPR bindings are plural by construction; observation never collapses; teeth resolve only on hard conflict; T1/T2 two-layer law (every fold declares its T1 anchor set + recompute-and-verify path) |
| Artifact roles | brit standing shapes (`record \| attestation \| fold \| pin`, live in policies.yaml) | the measure definition is a **record**; a memoized result is a **fold**; peer verification is an **attestation**; binding to a subject is a **pin** — zero new shape vocabulary |

**What is genuinely new — the contribution of this spec — is the measure itself as a
structured, content-addressed artifact, and the memoization of its applications.**

## 3. The Measure (middah)

A `Measure` is a record with:

| Field | Meaning | Law |
|---|---|---|
| `id` / CID | content-derived identity; versions pin like registry policies (`<id>@<version>`, declared dependency never recency — the lens-version-DAG rule) | never mutated; new semantics = new version |
| `family` | the orthogonal axis this measure lives on: `clippy`, `rustfmt`, `eslint`, `sonar`, `watts`, `attention-minutes`, `bytes-stored`, `view-events`, … | **family orthogonality:** results in different families sit side-by-side and are never summed into an unlabeled scalar (the quilt law, applied to measures) |
| `covers` | intra-family lineage/subsumption: `clippy-pedantic@1 covers clippy-standard@1` means the pedantic result *byte-for-byte contains* everything the standard result would report | `covers` is an **attestable claim** (testable against the tool), not an assertion; a cached stronger fold may answer a weaker query only via a declared `covers` edge |
| `unit` | the denomination of the result (finding-count + finding-set CID; joules; minutes; bytes) | Commitment II: the unit must be witnessable; self-reported units are marked advisory-forever |
| `procedure` | the executable reference (tool + invocation + result canonicalization) | deterministic given subject + environment, or it declares itself non-deterministic (excluded from fold caching) |
| `env-sensitivity` | which environment facts change the result (toolchain version for clippy; none for bytes-stored) | load-bearing for the fold key (§4) |
| `default-authority` | the tier its raw results carry: **observation** (always, in v1) | a measure never gates; teeth exist only in lenses (§5) |

**Dimensional structure.** Measure-space is a disjoint union of families; within a family,
`covers` induces a partial order (a subsumption lattice: standard ⊑ pedantic). A subject's
measurement state is therefore a **vector indexed by family** — never a scalar — where each
component records the strongest fold available plus its evidence tier.

## 4. The Fold (memoized application)

Applying measure `M@v` to subject `S` under environment `E` is:

- **an REA economic event** — an agent spent compute (denominated in the witnessable:
  metered CPU-seconds/joules where available) to produce
- **a fold** — keyed by the derivation triple `(S-CID × M-CID@v × E-CID)`, whose body is
  content-addressed to a result CID and whose **reconstruction strategy is its definition**:
  re-run the procedure.

`E` (environment fingerprint) is part of the key by necessity — a clippy result depends on
the rustc toolchain; omitting it makes two developers' caches silently disagree. Measures
with `env-sensitivity: none` use a null environment component.

**Trust economics fall out of the shape.** Reading a fold is free; *trusting* one is a choice;
*verifying* one costs a recompute. "Things with less trust get more compute" is literal: to
raise a fold's evidence tier you spend recomputation. The evidence ladder is the quilt
vector, unchanged: **claimed** (one agent computed it) → **witnessed** (an independent peer
recomputed and matched) → **confirmed** (per the consuming context's policy). A community
that wants higher trust in a subject commissions more/stricter measures and more independent
recomputation — and can bind *its own* measure or lens to the same subject and compare its
folds against the ones other communities bring (plural-lenses, unchanged).

**Honest absence.** *Unmeasured* renders differently from *measured-zero* (the
resilience-dimensions lesson): the vector component is absent, never defaulted. A facing
must not present a missing fold as a clean bill.

**Commons memoization.** "I want the pedantic result of commit X" is a cache lookup on the
derivation triple; a miss is a compute event whose result is published so every subsequent
developer (or agent) benefits. The fold is the anti-enclosure form of a CI artifact: owned
by no one, reusable by anyone, verifiable by recompute.

## 5. Layering: measures below governance; lenses compose measures

A **measure carries no teeth, ever.** The plural-lenses spec's *lens* is re-read (not
re-written) as a composition: its observation half is one or more middot; its contract half
is the teeth. So:

- clippy-pedantic-during-`/code-review` = the `clippy-pedantic` measure consumed by an
  **advisory lens** bound to the review context (`binding: persuasive`, `class: inject`) —
  self-grantable under the agency charter, no ratification round-trip;
- a community that wants a pedantic *gate* composes the same measure into a lens with a
  contract half and earns that binding through the governance-escalation ladder
  (`ask`/`deny` requires deliberated registry provenance).

This keeps the floor/ceiling law structural rather than disciplinary: strictness escalation
is never a mode switch on the measure — the measure is always declared, in warm standby;
**the consuming context chooses whether to read it**. Normal dev gates never read the
pedantic measure; nothing "falls back" because nothing switched.

## 6. Aggregation up the ladders

Folds aggregate **per-family, non-collapsing**, up reach/VSM recursion tiers via the existing
`select → fold → aggregate` facings machinery: household sums its attention-minutes,
neighborhood sums households, commons sums neighborhoods — each tier's aggregate is itself a
fold with a declared T1 anchor set and recompute path (the two-layer law). Cross-family
composition (e.g. "quality per watt") is a *derived measure* with its own record, unit, and
recompute path — never an ad-hoc arithmetic in a facing. Consumer-blinded census rules from
the witnessed-interaction spec apply to any aggregate that could expose a subject.

## 7. P2P Design Gate

### Entity: Measure (definition record)
- **Classification**: Operational (C) at the dev-tooling tier — registry rows beside
  `.claude/epr-meta/policies.yaml`, same liftable Precedent shape; graduated home is the
  plural-lenses spec's entity budget in Mishpat DNA (11/~100 headroom) when lenses lift.
- **Content Address Strategy**: Content-Derived (CID); dev-tier carries `contentHash` sha256
  short-form per the cite↔CID convergence.
- **Source of Truth**: repo registry now; Mishpat DHT after graduation.
- **HTTP Route**: none in v1.
- **Anti-pattern check**: no new DHT entry type minted; no UUID; versions are declared
  dependencies.

### Entity: MeasureFold (memoized result)
- **Classification**: Operational (C) — *by construction*: its reconstruction strategy is the
  measure's procedure. It is a T2 substrate artifact (content-addressed blob/sidecar),
  **never a DHT entry** (the two-layer law keeps folds off the notary floor).
- **Content Address Strategy**: Content-Derived — derivation-keyed
  `(subject CID × measure CID@version × env CID)`, result body → result CID.
- **Source of Truth**: recompute; the cache is a projection of the procedure.
- **HTTP Route**: none in v1 (dev-tier cache is filesystem/sidecar; a doorway read route is a
  later slice, designed after zome/projection order per the gate).

### Entity: Fold attestation (peer verification)
- **Classification**: Agent-Scoped with Notarized Attestation (B2) — reuses the existing
  Attestation entry type / witness carrier; only the attestation touches the DHT. Carries
  the evidence-tier transition (claimed → witnessed).
- **Content Address Strategy**: Agent-Scoped Composite (agent × fold CID).

### Entity: Measure/lens ↔ subject binding
- **Classification**: Derived (A2) — the plural-lenses spec's link design, unchanged.

### Design constraints discovered
- `covers` claims need a conformance probe (run both profiles on a fixture, assert
  superset) before a cache may answer a weaker query from a stronger fold.
- Environment fingerprinting needs a canonical `E-CID` recipe per family (rust toolchain:
  `rustc -V` + clippy version + lint-config hash).
- Concern-canon registration (C0-C14 walk + `seam-registry.yaml` rows) lands with the first
  implementation slice, not this spec; C2 is structurally answered (folds are
  derivation-keyed — there is no "latest" to elect), C4 by §4 honest absence, C6a by
  per-run compute budgets, C5 by folds-are-evidence-never-authority.

## 8. MVP slice (first worked instance)

The `/code-review` lint escalation ships as the first middah, three small pieces:

1. **Canon**: registry rows `measure:clippy-pedantic@1` (family `clippy`,
   `covers: clippy-standard@1`, env-sensitivity: toolchain; allow-list seeded from brit's
   empirically-counted pedantic allows) and an advisory lens row
   `review-lint-lens@1` (`binding: persuasive`, `class: inject`) declaring that lens
   findings are advisory-only — never `-D`, never a push gate.
2. **Executor + fold cache**: a small script that computes the derivation triple for the
   commit, checks the fold cache, on miss runs
   `cargo clippy --message-format=json -- -W clippy::pedantic` scoped to changed crates,
   diffs findings to changed lines, and writes the fold content-addressed so the next
   reader gets a cache hit.
3. **Binding**: the `code-reviewer` agent invokes the lens and labels its findings with
   their tier ("pedantic-lens (advisory)"), kept distinct from gate-backed findings —
   the finding record carrying (severity, authority-tier, source-measure, evidence-tier)
   is the vector ontology in practice.

ESLint/Sonar families follow the same shape as later slices once the clippy instance has
proven the fold-cache mechanics.

## 9. Non-goals (v1)

- **No unification of the existing verdict vocabularies** — that is the classifier spec's
  carrier-extension gap; this spec cites it and stays out.
- **No DHT entries** — measures are registry rows, folds are T2 artifacts, only reused
  Attestations touch the notary floor.
- **No enforcement** — every v1 surface is observation-tier; teeth arrive only through
  lens contract halves under the escalation ladder.
- **No self-reported-unit crediting** — Commitment II holds from birth.
