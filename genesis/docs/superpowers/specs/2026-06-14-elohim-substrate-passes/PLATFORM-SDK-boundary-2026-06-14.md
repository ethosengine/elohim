---
title: "THE API/SDK BOUNDARY — one composition grammar over many capability APIs"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
part_of: "PLATFORM model — the discriminator that the protocol is ONE SDK composed over MANY APIs"
reconciles_with:
  - SDK-DESIGN-2026-06-14.md            # the agency-gradient RAIL — the gradient is the rail; this is the structure
  - ORACLE-2026-06-14.md                # the cohesion-GOVERNOR of self-development (rung 3 = COMPOSITION)
grounds_on:
  - ESCALATED-ARCHITECTURE-2026-06-14.md # the ONE primitive (Commitment·six faces·one Governor·∪=full·two quilts)
  - RECURSIVE-ARCHITECTURE-2026-06-14.md # CoverageRollup; limit_owner ∈ {self,commitment,operator,faith}
real_monorepo_anchors:
  - elohim/sdk/src/index.ts                          # the ElohimSDK facade (per-service getters — the embryonic grammar)
  - elohim/sdk/storage-client-ts/src/generated/      # 446 ts-rs views — the ONE honest boundary
  - elohim/sdk/storage-client-ts/src/api/            # per-API client modules (qahal.ts today)
  - elohim/sdk/domains/{elohim,imagodei,lamad,mishpat,qahal,shefa,avodah,infrastructure}/  # per-API vocabularies
  - crates/elohim-sdk/src/lib.rs                     # mode-aware Rust facade (ClientMode)
do_not_cite_seal: true
forest_test: "Does one grammar mean a developer learns the values ONCE, and they hold across every capability they will ever touch?"
---

# THE API/SDK BOUNDARY

## One Composition Grammar over Many Capability APIs

> The platform is not many SDKs. It is **one SDK** — a single composition grammar — laid over **many
> APIs**, each a capability boundary over a domain. AWS grew the inverse: many services, each its own
> idiom, its own auth, its own consistency, its own console widget — sprawl with no shared primitive and
> no governor of its own growth. Elohim inverts it: **every API speaks the one primitive and rides the
> one agency gradient**, so there is *one* auth model, *one* consistency model, *one* economic model, and
> *one* governance model across the entire catalog. The grammar is thin and uniform at the top; the APIs
> are many and deep beneath; the ts-rs codegen seam keeps the boundary honest; and the oracle keeps the
> growth coherent. This document names that boundary precisely and locates it in the real package
> structure. **Net new DNA entry types: zero. Net new SDK: zero — we name a layer in the SDK that
> already exists.**

---

## PART 1 — THE DISCRIMINATOR, STATED PRECISELY

### Definitions

- **An API is a capability boundary.** A service over a domain — content, identity, learning, community,
  economy, governance, gateway. There are **many**, and there will be far more (the AWS-services analog:
  every box in the console). In the real tree the APIs are already here, in three honest forms:
  - the **Rust HTTP services** (`elohim-storage`, `doorway`) that expose routes;
  - the **Holochain zomes** (`elohim/holochain/dna/{elohim,imagodei,mishpat,infrastructure,node-registry,hrea}`)
    that hold notarized truth;
  - and their **TypeScript client + vocabulary surfaces** (`elohim/sdk/storage-client-ts/src/api/`,
    `elohim/sdk/domains/<app>/manifest.json`).
  Each is a *capability boundary*: a promise scoped to a domain.

- **The SDK is the ONE composition grammar over all the APIs.** Not a bag of clients — a *grammar*: the
  small set of primitive verbs an agentic developer composes to build any bespoke app, where the verbs
  are the same no matter which API they reach. The grammar is what you learn once. The APIs are what you
  reach for as needed. A developer who knows the grammar can compose three APIs into one app without
  learning three idioms — because there is only one idiom.

- **The scope is the corpus of almost all human software** — auth, storage, graph, messaging, feeds,
  search, payments, workflow, documents, permissions, realtime/sync, notifications, jobs/compute, files —
  **re-substrated** so each is agency-preserving + REA-economic + governance-bound + capture-resistant
  *by construction*, because each is composed from the one primitive, never assembled from the web2 stack.

### The line itself (where the boundary cuts)

```
        ┌───────────────────────────────────────────────────────────────┐
   SDK  │  THE GRAMMAR  (thin · uniform · learned once)                  │
 (one)  │  authorAtom · commit(face) · runGovernor · rollupCoverage ·   │
        │  bindCapability — five verbs, the SAME across every API        │
        └───────────────────────────────────────────────────────────────┘
   ── the API/SDK boundary ── (ts-rs codegen seam — snake_case never crosses) ──
        ┌──────────┬──────────┬──────────┬──────────┬──────────┬─────────┐
  APIs  │ content  │ identity │ learning │ community│ economy  │ gateway │  …many
 (many) │ (elohim) │(imagodei)│ (lamad)  │ (qahal)  │ (shefa)  │(doorway)│  …more
        └──────────┴──────────┴──────────┴──────────┴──────────┴─────────┘
        each API: a capability boundary · ALL speak the one primitive · ALL ride the gradient
```

The boundary is **not** "TypeScript vs Rust" (that line already exists and is the ts-rs seam). The boundary
is **grammar vs capability**: above it, the five composition verbs that never change; below it, the many
capability surfaces that grow without bound. The ts-rs seam *carries* the boundary honestly — but the
boundary is a layering distinction, not a language one.

---

## PART 2 — HOW THE GRAMMAR WORKS (the five verbs an agentic developer composes)

The grammar exposes exactly the primitives the night's escalation made universal. An agentic developer
builds **any** app by reaching for these five — and only these five — across **any** API:

| Verb | What it does | The ONE primitive it touches | Real anchor |
|---|---|---|---|
| **`authorAtom`** | Sign an EPR atom (the three-leg knowledge·value·governance coupling) with the *person's own key* | the EPR atom — the unit of truth in every domain | `elohim/sdk/epr-ts/src/epr.ts`; SDK-DESIGN atom-authoring |
| **`commit(face)`** | Record a bounded, witnessed, revocable `Mishpat::Commitment` under one of the six faces (custody·care·head·arc·self-limit·capability·covenant) | the one Commitment, action discriminator selects the face | `content_store_integrity` REA primitives; ESCALATED §"six faces" |
| **`runGovernor`** | Ask the one `trait Governor` to admit/refuse-and-elevate, carrying `limit_owner ∈ {self,commitment,operator,faith}` | the one control-plane spine | `elohim-compute::actuation` (lifted from `arc_actuator.rs`); SDK-DESIGN commitment-governor |
| **`rollupCoverage`** | Aggregate-with-descent over a `CoverageDomain` (commons only — never a soul), reading the `deficit` | the `∪=full` coverage invariant + CoverageRollup | `graph_views/recursion/`; RECURSIVE CoverageRollup |
| **`bindCapability`** | Grant a scoped, revocable capability across the seam (servant below / veil above), refusing cross-band scopes | the covenant face + agency gradient | SDK-DESIGN covenant-harness; `bindAgent` |

**A worked composition — three APIs, one app, one grammar.** An agentic developer builds a "household
care journal that earns reach in the collective commons." They compose:

1. **identity API (imagodei)** — `authorAtom` an observation of who was cared for (signer = the person's key).
2. **content API (elohim)** — `authorAtom` the journal entry coupling that observation to a value measure
   and a governance face; `commit('provide-care')` records the witnessed care.
3. **economy + community APIs (shefa/qahal)** — `rollupCoverage('care-floor')` shows the collective's
   *deficit* (never a leaderboard); `runGovernor` confers reach by adjudicating the three-leg evidence;
   `bindCapability` binds the household elohim as a *servant* (`witness`, never `govern`).

Five verbs. Five APIs touched. **One grammar.** The developer never learned a per-API auth scheme, a
per-API consistency rule, or a per-API economic event format — because the grammar carried them. That is
the whole difference from AWS.

### The cohesion guarantee — one model, four times, across ALL APIs

Because every API is composed from the one primitive and rides the one gradient, the four cross-cutting
models that AWS forces every service to reinvent are **uniform by construction**:

- **One auth model** — the signer is always the person's own key on the atom; capability is always a
  scoped, revocable `bindCapability`; there is no per-API API-key, no per-service IAM dialect.
  (`limit_owner` on every refusal names whose line it honored.)
- **One consistency model** — DHT = notary, P2P transport = data-ops, doorway = web2 projection
  (the three-layer truth model). Every API reconciles eagerly through the same `ReconcileController` shape;
  no API invents its own consistency story.
- **One economic model** — every capability use that touches value is an REA `EconomicEvent` bounded by a
  `Commitment`; the signal harness *is* the render-to-protocol bridge (`elohim/sdk/CLAUDE.md`). No API can
  skip the economy; the grammar carries it.
- **One governance model** — reach is *earned and adjudicated* through the same three-leg requirement and
  the same `runGovernor`/`rollupCoverage` shape, in every domain. No API self-asserts authority.

This is the cohesiveness the operator named: **the many APIs all speak the one primitive grammar and ride
the one agency gradient**, so a value learned in one corner of the catalog holds in every other corner.

---

## PART 3 — THE LAYERING (thin uniform top, many APIs beneath, codegen keeps it honest)

### Three layers, named in the real package structure

The SDK already *has* these three layers in `elohim/sdk/` — they have simply never been *named* as the
grammar/API discriminator. This proposal names them so the boundary becomes legible and enforceable.

```
LAYER 1 — THE GRAMMAR (thin, uniform, ONE)
  the five composition verbs · re-exports generated types only · ZERO new generated types
  HOME: elohim/sdk/src/index.ts (the ElohimSDK facade today) + @elohim/agency (the gradient wrapper)
        crates/elohim-sdk/src/lib.rs (the Rust mode-aware facade today)

LAYER 2 — THE PER-API CLIENTS (many, generated, honest)
  one client module per capability boundary; all hand-written transforms forbidden
  HOME: elohim/sdk/storage-client-ts/src/api/  (qahal.ts today — one of many to come)
        elohim/sdk/storage-client-ts/src/generated/  (446 ts-rs views — the boundary made honest)

LAYER 3 — THE PER-API VOCABULARIES (many, declarative, app-owned)
  what each capability's payload MEANS (content-types, signal-kinds, projections, graph, rendering)
  HOME: elohim/sdk/domains/{elohim,imagodei,lamad,mishpat,qahal,shefa,avodah,infrastructure}/manifest.json
```

The current `ElohimSDK` facade (`elohim/sdk/src/index.ts:60`) is *already* the embryo of Layer 1: it
exposes per-capability getters (`sdk.content`, `sdk.relationships`, `sdk.paths`, `sdk.humans`). Today
those getters are per-service CRUD. The boundary move is to **re-shape them around the five grammar verbs**
rather than per-service method bags — so `sdk.content.create(...)` becomes a *composition* of
`authorAtom` + `commit(face)`, and the same five verbs appear on every getter. The facade stops being a
service container and becomes a **grammar dispatcher**.

### The codegen keeps the boundary honest

The ts-rs path — Rust views `#[derive(TS)]` → `cargo test export_bindings` →
`storage-client-ts/src/generated/` (446 views today) → camelCase TS — **is the integrity of the API/SDK
boundary.** It guarantees that Layer 2 (per-API clients) can only ever expose types the truth layer
actually promised; Layer 1 (the grammar) re-exports those types and *adds zero of its own*. The boundary
cannot drift, because the grammar has no types to drift — it only composes generated ones. (This is the
SDK-DESIGN rule verbatim: `@elohim/agency` "re-exports generated types only; zero new generated types,"
SDK-DESIGN §"npm packages.") snake_case never crosses the boundary; neither does an ungenerated type.

### Why this is the opposite of AWS

| AWS | Elohim platform |
|---|---|
| many services, each its own SDK idiom | many APIs, **one grammar** |
| per-service auth (IAM dialects, API keys) | one auth model (own-key + scoped capability), carried by the grammar |
| per-service consistency surprises | one consistency model (DHT-notary / P2P-data / doorway-projection) |
| each service its own console widget, no governor | the **oracle** is the cohesion-governor (Part 5) |
| sprawl: no shared primitive | one primitive (`Commitment`·six faces·one Governor·∪=full) under every API |

---

## PART 4 — RECONCILING WITH THE AGENCY-GRADIENT RAIL (SDK-DESIGN-2026-06-14)

The two designs are **orthogonal and complementary** — one names the *rail*, one names the *structure*:

- **SDK-DESIGN-2026-06-14 names the RAIL** — the *vertical* axis. It lays the SDK along the agency
  gradient (human-sovereign below → keystone → veil-holding above), carried in one field (`limit_owner`),
  one type-shape rule (no `CoverageDomain` over a soul), one absent method (no `govern(person)`).
- **This document names the STRUCTURE** — the *horizontal* axis. It lays the SDK across the many capability
  APIs (content, identity, learning, community, economy, gateway), carried in one grammar of five verbs.

They meet at a single fact: **the gradient field rides every grammar verb.** When a developer calls a
grammar verb against *any* API, `limit_owner` travels with the call. So the structure (which API) and the
rail (which gradient position) are independent choices a developer makes per-call, and the boundary
guarantees the rail's two downward invariants hold *regardless of which API* the verb reaches. The
`household.govern()` and `veil.govern('margaret')` calls that do-not-compile in SDK-DESIGN §4 do-not-compile
**in every API**, because the grammar — not any one API — owns the absent method. The rail is enforced once,
at the grammar layer, across the whole catalog. That is why the structure *needs* the rail: a horizontal
grammar with no vertical invariant would be AWS with better ergonomics; the gradient is what makes the
grammar love-shaped, and the grammar is what makes the gradient hold everywhere.

---

## PART 5 — THE ORACLE AS THE COHESION-GOVERNOR (how the platform grows itself coherently)

A catalog that grows by accretion becomes AWS. The boundary defined here is only coherent *if there is a
governor of its growth* — and that governor already exists in the corpus: **the oracle is the cohesion-
governor of platform self-development** (ORACLE-2026-06-14). The binding is exact:

- **A new capability is born only as a new instantiation of the one primitive.** The boundary's law is:
  to add an API to the catalog, you do not invent a service — you instantiate the grammar over a new
  domain (a new manifest in `elohim/sdk/domains/`, a new client in `storage-client-ts/src/api/`, composed
  from the same five verbs). A "new API" that does *not* speak the one primitive is, by the boundary's
  definition, **not admissible to the catalog.** This is the structural answer to AWS sprawl.
- **The oracle's rung 3 (COMPOSITION) is this boundary.** The oracle's seven-rung ladder is
  WHY → primitives → **composition** → architecture → runtime → diagnostics → behavior (ORACLE §1
  Movement 1). This document *is* the composition rung's governing doc: the grammar is how primitives
  compose into capabilities. When the oracle's comparator (ORACLE §3d) detects a capability that drifted
  from the one-primitive law — an API that grew a private auth scheme, or a per-soul coverage scalar — it
  flags a `vision-gap` born-linked to *this* rung, and the cartographer's resolution *edits the boundary*.
- **Friction-escalation keeps the rungs coherent as it grows.** When a developer hits friction composing
  the grammar over a new domain (the verb doesn't fit the capability), the oracle escalates the *pattern*,
  not the instance: friction → escalate → update the rung → the next capability lands coherently
  (ORACLE-ESCALATION). The platform develops *itself* coherently because the governor of cohesion is a
  first-class organ, not a hope.

So the cohesion is **enforced twice**: structurally (the grammar's five verbs are the only admission gate
to the catalog) and reflexively (the oracle measures drift from the one-primitive law and routes it to a
decision that edits the boundary). The monorepo as nascent platform catalog grows the way a body grows —
every new organ from the same genome — not the way a mall grows.

---

## PART 6 — THE SMALLEST REAL FIRST STEP

**Name the boundary in the SDK package structure: split the grammar layer from the per-API client layer,
visibly, in the real tree — without adding a single capability or type.**

Concretely, the one-commit move:

1. **Add `elohim/sdk/src/grammar/` (Layer 1, the thin top).** Five files —
   `author-atom.ts`, `commit-face.ts`, `run-governor.ts`, `rollup-coverage.ts`, `bind-capability.ts` —
   each a thin function that **re-exports and composes existing generated types only** (zero new generated
   types, per the SDK-DESIGN rule). For the first step, four of the five delegate to capabilities that
   already exist in-tree (`epr-ts` authoring, `storage-client` REA create, `qahal` client); `run-governor`
   delegates to the `elohim-compute::actuation` lift once SDK-DESIGN Step 0b lands. No DNA. No new view.
2. **Re-point the `ElohimSDK` facade getters at the grammar** (`elohim/sdk/src/index.ts`). The getters keep
   their per-capability names (`sdk.content`, `sdk.humans`) but each now exposes *the same five verbs*,
   proving the grammar is uniform across APIs. The existing per-service methods stay (additive, reversible)
   and become thin compositions of the grammar verbs.
3. **Add `elohim/sdk/src/api/` as the explicit Layer-2 home** and move the existing per-API client modules
   (`storage-client-ts/src/api/qahal.ts` is the seed) under one named directory, so the boundary —
   grammar above, per-API clients below — is **visible in the directory tree**, not just in prose.

Gate: `pnpm --filter @elohim/holochain-sdk build` green; `cargo test export_bindings` byte-identical
(the boundary added zero generated types); a one-file demo composing two APIs through the five verbs.
This is a few hundred lines of re-shaping, additive and reversible, spending zero DNA and zero new types —
and it makes the discriminator *real in the tree*: from this commit on, "is this the grammar or an API?"
has a directory answer, and every future capability is admitted through the grammar or not at all.

---

## PART 7 — WHAT LOVE REQUIRES

The corpus of almost all human software gets re-substrated toward flourishing **not app-by-app, but by
making the easier tool the love-shaped one.** A single composition grammar is the mechanism: when there is
*one* grammar over *all* the APIs, an agentic developer learns the values **once** — own-key authorship,
witnessed-and-revocable commitment, refuse-and-elevate with whose-line-named, coverage-of-the-commons that
can never become coverage-of-a-soul, the empty place left empty — and those values **hold across every
capability they will ever touch.** They cannot accidentally build a capture-shaped app, because the only
verbs the grammar offers are agency-preserving, and the only APIs the catalog admits are instantiations of
the one primitive. The old system is not defeated; it is *subsumed* — because building anything on Elohim
is both **easier** (one cohesive SDK, not many idioms) and **love-shaped by default** (the primitives carry
the values, the gradient rides every verb, the oracle keeps the growth coherent).

> **The closing test, in one line:** love requires that a developer learn the values exactly once — at the
> first verb they ever call — and never be able to leave them behind, no matter how many capabilities they
> compose, because the one grammar carries the one primitive and rides the one gradient across every API
> the platform will ever grow.

---

*All package-structure moves named here (the `grammar/` split, the facade re-point, the `api/` directory)
are additive, reversible, and operator-GATED. This is a proposal for operator blessing — not cite-sealed,
not a decision, not code. It reconciles with the agency-gradient rail (SDK-DESIGN-2026-06-14) and binds the
oracle as the cohesion-governor (ORACLE-2026-06-14).*
