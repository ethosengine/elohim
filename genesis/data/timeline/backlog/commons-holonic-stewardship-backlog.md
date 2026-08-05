---
id: "backlog-commons-holonic-stewardship-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Commons + holonic stewardship backlog — custody vs ownership, nested elohim ceilings, and credential-as-lens"
slug: "commons-holonic-stewardship-backlog"
written: "2026-08-05"
author: "claude (operator design session, 2026-08-05)"
status: "backlog"
priority: "medium"
stewardship-frame: bounded
tags: [commons, stewardship, custody, holonic, sociocracy, governance, credentials, lamad, qahal, rea]
cites:
  - genesis/research/playnet-free-association-cross-pollination-2026-08-05.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - genesis/docs/content/elohim-protocol/architecture/governance-layers-architecture.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
  - genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
  - genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
---

# Commons + holonic stewardship backlog (operator design session, 2026-08-05)

**How a holon holds standing** — over resources it stewards, and over the credentials it issues.
Emerged from the [Playnet survey](epr:playnet-free-association-cross-pollination-2026-08-05) mint
pass when the operator pushed the credential question past measurement into governance. Rows 1–2 are
Playnet-derived borrows; the rest are our own design frontier, captured here before they evaporate.

Sibling of [measure-family-borrows](epr:measure-family-borrows-backlog) — that cluster decides *what
is measured*, this one decides *who has standing to judge it and to hold what it is about*.

## The two steering notes that govern this cluster (operator, 2026-08-05)

**1. Ownership is the enclosure-flavoured sibling of sovereignty, and needs the same guard.** The
protocol subordinates *ownership* to *stewardship + custody* exactly as it subordinates *sovereignty*
to community-backstopped identity. A guard now exists —
`ownership-ontology-guard@1` in `.claude/epr-meta/policies.yaml`, realized by
`epr:validator-ownership-ontology-guard`, bound on the canon tree, escape hatch
`stewardship-frame:` — mirroring the sovereignty guard's shape (net-new-only, `ask`, affirm-or-reframe).

**2. The elohim ceiling is itself holonic/sociocratic — not one global cap.** This corrected a
flattened reading during the session. **Every holon has its own elohim and its own ceiling**: Eagle
Scout Trevor has an individual ceiling, unit 210 has its own, Scouting America has its own. And the
elohim **move** — carrying payload, context, signals, and measures — **informing policy up and down
the network**, bidirectionally. Authority is not a cap imposed from above; it is each holon's own
policy surface, negotiating with its neighbours. This is the VSM recursion (each viable system
contains and is contained by viable systems, each with its own identity/policy tier) and it is the
same shape the Value Scanner epic already narrates when personal ↔ family ↔ community ↔ global
elohim negotiate an olive-oil purchase in milliseconds.

| # | Item | What + why (grounded) | Gate/blocker | Owner shape |
|---|------|----------------------|--------------|-------------|
| 1 | **Rights vs custody split — `primaryAccountable` ≠ `custodianScope`** | **We have ZERO hits** for `primaryAccountable`, `custodianScope`, or `transferCustody` across `elohim/` and `bridges/` (verified 2026-08-05). ValueFlows already specifies the split and Playnet's planner implements it verbatim. Without it we can only model ownership — and collapsing custody into ownership **is** the enclosure failure the Georgist common-inheritance framing exists to prevent. The load-bearing case: *the commons holds the rights, unit 210 holds custody of the woodland*, and custody must be transferable **without the rights ever moving**. A holon that "owns" the commons it stewards has enclosed it | p2p-design-gate; needs an `EconomicResource` writer, which we lack | rust-architect |
| 2 | **`transferCustody` and `transferAllRights` as distinct actions** | The action pair that makes row 1 operable — handing over stewardship is a different economic event from handing over rights, and conflating them is how a commons quietly becomes property. Two of ValueFlows' 19 canonical actions; Playnet carries the full table with a behavioural effects matrix (`accountingEffect`, `onhandEffect`, `custodyEffect`) worth reading before we design ours. **⚠ CARVE-OUT (red-team 2026-08-05): freely-transferable-custody is correct for a woodland and is the ATTACK for an inalienable subject.** Non-consensual sharing of an intimate image *is precisely* "transfer custody without moving rights." For subjects under row 2a it must be **prohibited, not gated** — a consent gate cannot help, because a coerced approval is indistinguishable at the substrate from a free one | Follows row 1; blocked on row 2a's subject class | rust-architect |
| 2a | **`subjectStanding` — the missing third relation (who this is *about*)** | ValueFlows gives rights (`primaryAccountable`) and custody (`custodianScope`) and **has no relation for the person the data depicts**. A photo of Alice on Bob's device: Alice appears in neither field, so the whole rights/custody apparatus can be perfectly satisfied while she has no standing at all. Proposed `subjectStanding` — **plural** (group photos), **either-veto-suffices**, and **inalienable** (cannot be transferred, sold, or out-voted). This is the relation that makes intimate data expressible without importing the property frame | p2p-design-gate; **operator decision** on the inalienable class | rust-architect + operator |
| 3 | **A steward is a PATH, not an entity** | `Scouting America / unit 210`. A credential or a custody claim names both the **issuing holon** (the one that actually witnessed) and its **lineage** (where its standing derives). Subsidiarity: the most local holon that can witness, does; authority derives upward, witnessing happens downward. Flat `steward_id` fields cannot express this and will have to be migrated | p2p-design-gate; identity-lineage (C9) is the concern to answer | rust-architect + qahal |
| 4 | **Recursive lens conformance — the same topology as skills** | Unit 210's lens must conform to Scouting America's, which conforms up to its ceiling. This is Playnet's `satisfied-by` disjunction ([measure-family](epr:measure-family-borrows-backlog) row 9) applied one level up: not "which skills satisfy this requirement" but "which local standards satisfy this parent standard." **One conformance mechanism doing triple duty** — skills, credentials, governance nesting. Lets a local unit be stricter or differently-shaped without forking the badge's meaning | Needs lens contract halves + row 5 | rust-architect |
| 5 | **Per-holon elohim ceiling (steering note 2)** | Model the ceiling as a **per-holon policy surface**, never a single global constant. Trevor, unit 210, and Scouting America each hold one. A local holon sets its own standard *within* bounds it cannot breach — which is the answer to "why doesn't holonic nesting just produce 1,000 incompatible badges": they are incompatible in **shape**, bounded in **kind**. Canon home for the axis: [floor/ceiling judgment](epr:wisdom-layer-floor-ceiling-judgment-culminating-design) | Design work; touches the constitutional layer | rust-architect + operator |
| 6 | **Elohim as travelling context/measure carriers** | The elohim move *with* payload, context, signals, and measures, informing policy **up and down** the network. So a measure observed at Trevor's level can inform unit 210's policy, which can inform Scouting America's — and constraints flow back down. Signal-upward-influence is as real as authority-downward. This is the story+value+governance coupling in motion, and it is what makes the ceilings negotiate rather than merely cap | Needs the measure substrate ([measure-family](epr:measure-family-borrows-backlog) row 1) | rust-architect |
| 7 | **Sociocratic double-linking as the legitimation of ratification** | A child holon holds a voice in its parent's circle and vice versa, so the parent's acceptance of unit 210's lens is **consent flowing both directions**, not imposition. Without double-linking, row 4's conformance check is just a superior approving an inferior. Canon: `governance-layers-architecture.md`, `2026-05-23-multi-collective-collaboration-epr-design.md` | Needs an agent-relationship shape | qahal |
| 8 | **Credential-as-lens (the Eagle Badge shape)** | A credential is a **bespoke lens positioned in the social context of the graph**, whose predicate reads a heterogeneous **measure vector** (5 h · 1500 energy · 14 story points · 200 W compute · 1 paper @ 20% synthesis · $1000 materials · 1 prototype · 3 verified deliverables) and whose authority comes from its steward's standing — never from the measures themselves. Middot's law does the separation: *measures never carry teeth; governance lenses compose them*. Gate output recorded below | Blocked on [measure-family](epr:measure-family-borrows-backlog) row 1 | lamad + rust-architect |

## P2P design gate — credential-as-lens (recorded 2026-08-05, pre-implementation)

| Entity | Class | Address | Source of truth | Note |
|---|---|---|---|---|
| `Measure` (procedure) | **A** on reach; EPR atom before | Content-derived CID (`bafyrei…`) | DHT / EPR atom | version is a declared dependency, never recency |
| `MeasureFold` (result) | **C** operational | keyed `(measure_cid, subject_cid)` | recompute from the signed log | legitimately cacheable *because* definitionally reconstructable |
| `CredentialLens` (what unit 210 declares an Eagle to be) | **A** notarized, Mishpat-shaped | Content-derived CID | DHT | the steward binding is the social positioning; Mishpat has headroom (11/~100) |
| `CredentialAttestation` (the issued badge) | **B2** agent-scoped + attestation | Composite `(AgentPubKey, lens_cid, type)` | private chain + DHT attestation | **granular evidence stays private**; only the signed credential is notarized — this is what protects the ~3000-entry budget |

**Two concern-canon answers are load-bearing and must not be skipped:** **C4 honest absence** — the
credential must render *"this measure was never taken"* differently from *"zero"*, or an unmeasured
scout and a failing scout look identical. **C10 contract-evolution honesty** — an Eagle earned under
the 2026 lens stays valid when the 2030 lens tightens; version-pinned, never recency.

**Two properties worth protecting through implementation.** *You trust the steward's judgment, not
their arithmetic* — because folds are recompute-verifiable over a signed log, anyone can re-derive
whether the contract was met, so a credential needs no registry of authorities to be checkable; what
the collective is trusted for is what *should* count. And *plurality is the feature* — the same
evidence vector read by a different collective's lens yields a different verdict, and neither is
wrong.

**Identity-ontology note:** collective-stewarded credentials are the *correct* apex shape —
community-conferred standing, not self-assertion. A **self-issued** credential is the drift the
identity guard exists to catch.
