---
title: "THE ELOHIM PLATFORM MODEL — ONE SDK, MANY APIs"
id: platform-one-sdk-many-apis-design
subtitle: "The discriminator we were missing: a single composition grammar over many capability boundaries, scoped to the corpus of almost all human software, re-substrated and love-shaped by construction — with the monorepo as the nascent platform catalog and the oracle as the cohesion-governor"
date: 2026-06-14
status: design (operator-blessed 2026-06-14)
author: rust-architect (truth layer) — weaving boundary · catalog · corpus · cohesion
weaves:
  - PLATFORM-SDK-boundary-2026-06-14.md     # the API/SDK discriminator + the five-verb grammar + the cohesion guarantee
  - PLATFORM-SDK-catalog-2026-06-14.md      # the monorepo as the nascent service catalog (AWS-console-in-utero)
  - PLATFORM-SDK-corpus-2026-06-14.md       # the corpus-of-human-software composition proof + 3 worked apps
  - PLATFORM-SDK-cohesion-2026-06-14.md     # the grow-without-sprawl self-development loop + the coherence gate
reconciles_with:
  - SDK-DESIGN-2026-06-14.md                # the agency-gradient RAIL (the SDK design stands; this adds structure/scope/governance)
  - ORACLE-2026-06-14.md                    # the cohesion-GOVERNOR (catalog = rung-4; grammar = rung-3)
  - ORACLE-ESCALATION-2026-06-14.md         # the escalation organ that grows the catalog coherently
grounds_on:
  - ESCALATED-ARCHITECTURE-2026-06-14.md    # the ONE primitive: Commitment · six faces · one Governor · ∪=full · two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md    # CoverageRollup; limit_owner ∈ {self,commitment,operator,faith}; no coverage over a soul
monorepo_anchors_verified_2026_06_14:
  - elohim/sdk/src/index.ts                  # the ElohimSDK facade — the embryonic grammar (Layer 1)
  - elohim/sdk/storage-client-ts/src/api/    # qahal.ts — the seed of Layer 2 (per-API clients)
  - elohim/sdk/domains/{elohim,imagodei,lamad,mishpat,qahal,shefa,avodah,infrastructure}/  # Layer 3 vocabularies, uniform shape
  - elohim/holochain/dna/{elohim:186,imagodei:91,mishpat:35,infrastructure:23,node-registry:27,hrea:0} # externs/DNA, verified
  - elohim/elohim-storage/src/services/      # 108 service modules — the HTTP-API tier
  - crates/{elohim-sdk,doorway-client,elohim-storage-client}  # the composition-grammar crates
  - bridges/{valueflows(live),atproto,activitypub(planned)}   # the web2 bridges
do_not_cite_seal: true
forest_test: "Is the protocol legibly ONE SDK over MANY APIs — so a developer learns the values once, finds any capability the corpus demands as a love-shaped equivalent, and the platform grows to AWS-breadth while staying one machine?"
---

# THE ELOHIM PLATFORM MODEL

## One SDK, Many APIs

> The operator named the discriminator we were missing: **"The Elohim Protocol has to be ONE SDK, composed
> on MANY APIs."** An API is a capability boundary — a service over a domain, the AWS-services analog, and
> there are many. The SDK is the *one* cohesive composition grammar over all of them — the primitives an
> agentic developer composes to build any bespoke app. The scope is the corpus of almost all existing human
> software, but **re-substrated** so every app is agency-preserving, REA-economic, governance-bound, and
> capture-resistant *by construction*. The whole difference from AWS is **cohesion**: AWS grew many services
> with no shared primitive and no governor of its own growth — sprawl. Elohim's many APIs all speak the
> **one primitive grammar** and ride the **one agency gradient**, and the platform develops *itself*
> coherently because **a new capability is born only as a new instantiation of the one primitive**, and the
> **oracle is the cohesion-governor**. And the recognition that makes this not-aspirational: **the monorepo
> IS the nascent platform catalog** — today's pillars, zomes, services, crates, and bridges are the
> AWS-console-in-utero, already structured per-service, already each the same shape.
>
> **Net new DNA entry types to make this real: zero. Net new SDK: zero — we name layers that already exist.**

This document weaves four parts into one model: (1) the **boundary** — the API/SDK discriminator and the
composition grammar; (2) the **catalog** — the monorepo read as the nascent service surface; (3) the
**corpus** — the composition proof that any app is buildable and love-shaped; (4) the **cohesion** — how the
platform grows without becoming sprawl. Then it reconciles with the agency-gradient rail (SDK-DESIGN) and the
cohesion-governor (the oracle), and closes on what love requires.

---

## 1 — THE ONE-SDK-MANY-APIS MODEL (the boundary discriminator)

### 1.1 The line itself: grammar vs capability

The boundary we were missing is **not** "TypeScript vs Rust" — that line already exists, and it is the ts-rs
codegen seam (`#[derive(TS)]` → `cargo test export_bindings` → `storage-client-ts/src/generated/`, 446 views,
snake_case never crossing). The boundary is **grammar vs capability**:

```
        ┌───────────────────────────────────────────────────────────────┐
   SDK  │  THE GRAMMAR  (thin · uniform · learned ONCE)                  │
 (one)  │  authorAtom · commit(face) · runGovernor · rollupCoverage ·    │
        │  bindCapability — five verbs, the SAME across every API         │
        └───────────────────────────────────────────────────────────────┘
   ── the API/SDK boundary ── (the ts-rs codegen seam carries it honestly) ──
        ┌──────────┬──────────┬──────────┬──────────┬──────────┬─────────┐
  APIs  │ content  │ identity │ learning │ community│ economy  │ gateway │  …many
 (many) │ (elohim) │(imagodei)│ (lamad)  │ (qahal)  │ (shefa)  │(doorway)│  …more
        └──────────┴──────────┴──────────┴──────────┴──────────┴─────────┘
        each API = a capability boundary · ALL speak the one primitive · ALL ride the gradient
```

Above the line: the five composition verbs that never change. Below it: the many capability surfaces that grow
without bound. The grammar is what a developer learns once; the APIs are what they reach for as needed.

### 1.2 The composition grammar — five verbs, mapped to the one primitive

The grammar exposes exactly what the night's escalation made universal. An agentic developer builds **any** app
by composing these five verbs across **any** API, and only these five:

| Verb | What it does | The ONE primitive it touches | Real anchor |
|---|---|---|---|
| **`authorAtom`** | Sign an EPR atom (knowledge·value·governance three-leg coupling) with the *person's own key* | the EPR atom — the unit of truth in every domain | `elohim/sdk/epr-ts/src/epr.ts` |
| **`commit(face)`** | Record a bounded, witnessed, revocable `Mishpat::Commitment` under one of the six faces | the one Commitment; the action discriminator selects the face | `content_store_integrity` REA primitives |
| **`runGovernor`** | Ask the one `trait Governor` to admit / refuse-and-elevate, carrying `limit_owner ∈ {self,commitment,operator,faith}` | the one control-plane spine | `elohim-compute::actuation` (lifted from `arc_actuator.rs`) |
| **`rollupCoverage`** | Aggregate-with-descent over a `CoverageDomain` (commons only, never a soul), reading the `deficit` | the `∪=full` coverage invariant + CoverageRollup | `graph_views/recursion/` |
| **`bindCapability`** | Grant a scoped, revocable capability across the seam (servant below / veil above), refusing cross-band scopes | the covenant face + agency gradient | SDK-DESIGN covenant-harness; `bindAgent` |

### 1.3 The cohesion guarantee — one model, four times, across ALL APIs

Because every API is composed from the one primitive and rides the one gradient, the four cross-cutting models
AWS forces every service to reinvent are **uniform by construction**:

- **One auth model** — the signer is always the person's own key on the atom; a capability is always a scoped,
  revocable `bindCapability`; no per-API API-key, no per-service IAM dialect. `limit_owner` on every refusal
  names whose line it honored.
- **One consistency model** — DHT = notary, P2P transport = data-ops, doorway = web2 projection. Every API
  reconciles eagerly through the same `ReconcileController` shape; no API invents its own consistency story.
- **One economic model** — every value-touching capability use is an REA `EconomicEvent` `bounded_by` a
  `Commitment`; care-class and compute-class stay categorically isolated. No API can skip the economy.
- **One governance model** — reach is earned and adjudicated through the same three-leg requirement and the
  same `runGovernor`/`rollupCoverage` shape, in every domain. No API self-asserts authority.

This is the cohesiveness the operator named: a value learned in one corner of the catalog holds in every other
corner — **because the corners share the one primitive, not merely a style guide.**

### 1.4 The three layers, named in the real tree

The SDK already *has* these layers; they have simply never been named as the grammar/API discriminator:

```
LAYER 1 — THE GRAMMAR (thin, uniform, ONE)        HOME: elohim/sdk/src/index.ts (the ElohimSDK facade) ·
  the five verbs · re-exports generated types only       crates/elohim-sdk/src/lib.rs (mode-aware Rust facade)
LAYER 2 — THE PER-API CLIENTS (many, generated)   HOME: elohim/sdk/storage-client-ts/src/api/ (qahal.ts seed) ·
  one client per capability · no hand transforms         storage-client-ts/src/generated/ (446 ts-rs views)
LAYER 3 — THE PER-API VOCABULARIES (many, declarative) HOME: elohim/sdk/domains/{elohim,imagodei,lamad,mishpat,
  what each payload MEANS (types, signals, graph)        qahal,shefa,avodah,infrastructure}/manifest.json
```

The boundary move is to re-shape the facade's per-capability getters around the five grammar verbs rather than
per-service method bags — so `sdk.content`, `sdk.humans`, `sdk.governance` each expose *the same five verbs*.
The facade stops being a service container and becomes a **grammar dispatcher**. The codegen keeps it honest:
the grammar has no types to drift, because it only composes generated ones.

---

## 2 — THE CAPABILITY CATALOG (the monorepo as the nascent platform surface)

The operator's recognition, taken literally: *"the monorepo IS the nascent AWS dashboard / GCP surface."* We
do not propose new services — we **name what already nascently exists** as a coherent capability catalog. The
strongest evidence it is real: `elohim/sdk/domains/README.md` already opens with the frame — *"Each
subdirectory is a protocol domain"* — prints a catalog table, and enforces a **uniform per-entry layout**
(`manifest.json` + `schemas/` + `types/` + `scripts/`). This is the AWS-console-in-utero *with the property
AWS never had: every entry has the same shape.*

### 2.1 The catalog, by tier (each service → web2 slot it re-substrates → real path → the ONE primitive face)

| Tier | Service | Lives in (verified path) | Web2 slot re-substrated | ONE-primitive face |
|---|---|---|---|---|
| **0 Identity & Access** | Identity | `dna/imagodei/` (91 externs); `services/presence_service.rs`, `relationship_service.rs` | Auth0 / Okta — *person's own key signs; no provider owns the account* | **person-keeps-naming** (`limit_owner: self`) |
| | Permissions / Capability | `services/steward_affinity_service.rs`; `steward/{grants,gates,credentials}` routes | IAM / RBAC / API-keys — *capability is a revocable Commitment, not an admin grant* | **capability-as-commitment** |
| | Recovery | imagodei recovery v2; `recovery_flow_projector.rs` | "Forgot password" — *witnessed quorum, not a support ticket* | **covenant/quorum face** |
| | Attestation | `attestation_projector.rs`; mishpat flows | Verified-credentials / KYC — *peer-witnessed, revocable, on-DHT* | **the witnessing atom** |
| **1 Storage / Content / Graph** | Storage / Two-Quilt | `services/{distribution,replication_prioritizer}.rs`; `IrohBlobStore` | S3 / GCS — *RS(4,7) custody-tracked, no single owner* | **custody-as-coverage** (`custody-blob`) |
| | Content / EPR | `services/{content_service,epr_service}.rs`; `epr-ts/` (186 elohim externs) | CMS / Contentful — *content-addressed (CID=identity), reach earned at authoring* | **the authoring atom** |
| | Content Graph | `graph_engine.rs` (`ContentGraphResolver` + `NativeGraphResolver`) | Neo4j / Neptune — *read-only by construction, recompute-on-read, local* | **Category-C derived** (no write method) |
| | EPR Social Graph | `graph/` (Cozo `GraphEngine` on sled); `graph_views/` | Social-graph / follow-graph API | **the recursion substrate** (CoverageRollup walks it) |
| | Sync / Realtime | `sync/` (Automerge CRDT, ~60s); `/elohim/sync/2.0.0` | Firebase Realtime / Liveblocks — *CRDT convergence is structural* | **head-coverage** (`covers-head`) |
| **2 Economy / Governance** | Economy / REA | `shefa`; `services/{economic_event,agreement,token_mint}_service.rs`; `bridges/valueflows/` | Stripe / ledger — *REA-accounted, care-class isolated from compute-class* | **care-as-commitment** (`provide-care`) |
| | Stewardship | `services/{stewardship,steward_standing,standing_projector}.rs` | Royalties / attribution-payout — *committed, not claimed; no ownership* | **CustodianCommitment** |
| | Governance | `dna/mishpat/` (35 externs); `qahal`; `services/governance_health.rs` | Snapshot / Aragon / OPA — *qahal-collective, constitution-bound* | **the Governor + constitution band** (`operator`) |
| | Limit-Governance | `services/{limit_gradient_registry,token_decay}.rs`; the donut | Quotas / rate-limits / billing-caps — *the line is `self` or commons (`operator`)* | **self-limit** (`respects-self-limit`) |
| **3 Compute / Coverage / Resilience** | Compute / Delegation | `elohim/elohim-compute/`; `services/arc_actuator.rs` | Lambda / Cloud Run — *bounded, witnessed, revocable Commitment* | **`delegates-compute`** (FIRST instance) |
| | Coverage / Arc | `services/{arc_actuator,replicates_commons_validator}.rs`; `bounds_validator.rs` | Auto-scaling — *`∪=full`, refuse-and-elevate, names whose line* | **arc-as-coverage** (`commits-arc-coverage`) |
| | Resilience | `services/{resilience,household_resilience}.rs` | Health-checks / status-page — *`not-yet-seen` is never `at-risk`* | **CoverageRollup** (`deficit`, never the holding) |
| | Node / Topology | `dna/node-registry/` (27 externs); `services/{peer_topology_view,cluster_view}.rs` | EC2-inventory / k8s nodes — *a laptop is a full participant* | **the runtime floor** (`Runtime::launch` w/ zero peers) |
| **4 Gateway / Federation / Bridges** | Gateway / Doorway | `doorway/doorway-service/`; `routes/{api,blob,storage_proxy}.rs` | API Gateway / Cloudflare — *thin, single-target, no fan-out, no domain logic* | **the seam** (`IConnectionStrategy`) |
| | Federation | `doorway/services/{federation,discovery,did_resolver}.rs`; `/elohim/view-federation/2.0.0` | Federation / SSO / DNS — *agents configure doorway, not the reverse* | **head-coverage observed** |
| | Web2 Bridges | `bridges/valueflows/` (live); `bridges/{atproto,activitypub}` (planned) | Zapier / webhooks — *translate INTO the EPR-REA substrate, never weaken it* | **the bridge pattern** |

### 2.2 The breadth target vs the cohesion

The catalog must eventually reach **AWS-scale option count** (the whole corpus). But unlike AWS, every option
is *one primitive re-pointed*. **Breadth without sprawl.** A developer browses three ways: by capability ("I
need storage"), by web2-analog (the migration Rosetta stone: "my Auth0 → Identity, my Stripe → Economy/REA,
my Lambda → Compute"), and by the agency gradient (the SDK rail). Like an AWS console has S3 → buckets →
lifecycle → replication, each service has browsable depth (Storage → blob → shard → custody → coverage) — but
every option, at every depth, is **the same primitive**. That is the cohesion AWS never had.

---

## 3 — THE CORPUS PROOF (any app is buildable, and love-shaped by construction)

The proof obligation: the corpus of standard primitives nearly every human application is built from maps onto
Elohim APIs, **re-substrated**, with the value riding along *unrequested*. Every "Elohim API" cell cites a live
surface; the "carries-for-free" column is what the developer never asked for.

### 3.1 The primitive-mapping table (the corpus, re-substrated)

| # | Web2 primitive | Elohim API (real surface) | Carries for FREE |
|---|---|---|---|
| 1 | Identity / auth | `api/identity.rs`; `imagodei` zome; `elohim_gate.rs` (`TrustContext`) | **Agency floor**: person keeps their naming (`limit_owner: self`); no server holds the credential of record |
| 2 | CRUD / storage | `content_service.rs`; trust-plane quilt (`Content` entry) | **Capture-resistance**: content-addressed, offline-first; relational `GET /thing`+UUID is *unscaffoldable* |
| 3 | Files / blobs | `api/blob.rs`; byte-plane quilt RS(4,7); `custody-blob` | **REA + capture-resistance**: holding bytes IS a witnessed revocable care commitment; a laptop participates |
| 4 | Graph / relationships | `graph_engine.rs` `ContentGraphResolver`; `relationship_service.rs` BFS | **Capture-resistance**: graph recomputable locally; `GraphSpec` excludes `MASTERY_OF` (no leak into identity) |
| 5 | Messaging / chat | gossip plane + `p2p_iroh` (BLAKE3 topic_id); `api/comments.rs` | **Agency + capture-resistance**: no server reads the channel; doorway projects a single target only |
| 6 | Feeds / timelines | `reach_earning.rs` (`ReachVerdict`); `CoverageRollup` | **No engagement-maximization**: metric is `deficit`, not impressions; a feed *cannot* become an attention casino |
| 7 | Search / discovery | `content_service.rs` search; tag co-occurrence; `/elohim/epr/2.0.0` | **Capture-resistance**: no central index owns ranking; results recomputable, identical across peers |
| 8 | Payments / ledger | `economic_event_service.rs` (`bounded_by`); `bridges/valueflows/` | **REA by construction**: every flow is a witnessed reciprocal commitment w/ standing + revocation + audit |
| 9 | Permissions / RBAC | `elohim_gate.rs`; `mishpat`; `trait Governor` + `Refusal{limit_owner}` | **Agency gradient compiled**: no `govern(person)` method — the dangerous permission is *absent*, not guarded |
| 10 | Workflow / state machine | `rea_commitment_service.rs` `update_state`; `recovery_flow_projector.rs` | **Governance-bound**: every transition witnessed and revocable; approvals are consent attestations on DHT |
| 11 | Documents / content | `content_service.rs`; lamad manifest format→renderer; `ContributorPresence` | **Agency**: attribution moves with consent, not platform fiat; no CMS owns the canonical copy |
| 12 | Realtime / sync | `sync/` (Automerge CRDT); `/elohim/sync/2.0.0` | **Capture-resistance**: no server arbitrates truth — peers converge; doorway only observes the head |
| 13 | Notifications | `events.rs` `EventBus`; `IntegrityNotify` signals; gossip broadcast | **Patience over engagement**: signals carry truth-deltas, not bait; `surfaceRecognition` does nothing on `NeedDeeper` |
| 14 | Jobs / compute | `elohim-compute`; `delegates-compute`; `arc_actuator.rs` (Governor's first impl) | **REA + no-overwhelm**: bounded reciprocal agreement; the Governor refuses-and-elevates, never overwhelms |

**The reading of the table:** fourteen rows, **zero new DNA entry types, zero new primitives** — every cell is
the *same* Commitment + Governor + CID + CoverageRollup grammar pointed at a different noun, and every cell's
"carries-for-free" is a value the developer *did not request.* The cohesion is not a shared style guide (AWS
has those); it is that **the APIs share the one primitive, so they share the values the primitive carries.**

### 3.2 The worked proof — three apps, one grammar, love-shaped unrequested

- **Marketplace ("the Postgres+Stripe+admin-dashboard shape").** Sellers/buyers = Identity (#1); listings =
  `exchange_service.rs` (`match_offer`/`match_request`); photos = `custody-blob` (#3); settlement =
  `economic_event_service.rs` `bounded_by` (#8); reviews = the **`vouch` signal_kind** (NOT a star table);
  feed = `deficit`-surfaced reach EPRs (#6); disputes = `mishpat` consent flows. **For free:** there is no
  `ban(user)` to build (no `govern(person)` method exists); reviews can't astroturf a leaderboard (metric is
  `deficit`, not stars-descending); if the dev's server dies, listings live on the quilt and participants keep
  their keys. The developer asked for a marketplace and *built* an agency-preserving, capture-resistant commons.

- **Project-tracker ("Jira").** Task = a **Commitment**; assignee = the committing agent; status column = the
  Commitment lifecycle (`update_state`); sprint = an Agreement; burndown = a **CoverageRollup** (`∪=full`); team
  = a `qahal` collective. **For free:** "assigning" someone is *impossible as a command* — a task is a
  Commitment the assignee *makes* (witnessed, revocable). A project tracker that structurally cannot become
  surveillance — and the developer just wanted a kanban board.

- **Community feed ("Twitter/Mastodon").** Post = content-EPR; follow = pre-authorized reach standing; timeline
  = reach-gated EPRs surfaced by `deficit`; "like" = a `vouch` signal_kind; moderation = `mishpat` consent +
  `squelch`/`quarantine` signal_kinds (already whitelisted), governed by a Governor; realtime = Automerge sync.
  **For free:** the timeline has no engagement ranking to build (`deficit`-first is the only metric); moderation
  is witnessed and revocable (`squelch` is a named signal with `limit_owner`, not a shadow-ban). A social feed
  that structurally cannot be an outrage machine — and the developer just wanted a microblog.

Three shapes — transactional, workflow, social — **one grammar.** Each inherited the full value stack
unrequested. The corpus of human software is *reachable* from the one primitive, and *love-shaped* on arrival.

---

## 4 — THE SELF-DEVELOPMENT COHESION (grow without sprawl; the oracle as governor)

### 4.1 The growth law — a new capability is born only as an instantiation of the one primitive

The monorepo IS the nascent catalog (`api/`, `services/`, zomes, `bridges/`, `domains/`). The whole difference
from AWS is the **growth law**: adding a capability is exactly one of four moves, in escalating cost, and never
anything else:

1. **A `signal_kind` addition** + `resource_classified_as` whitelist entry — a new social move on existing data
   (cheapest; `vouch` is the worked precedent). Cost: a string + a validator arm + a projector.
2. **A Commitment `action` discriminator** — a new face of the one Commitment (`replicates-dwelling` precedent).
   Cost: an action name + a coverage invariant. **Zero entry types.**
3. **A `Governor` impl** — a new setpoint on the one control spine (`ArcGovernor`→`FloorGovernor` precedent).
4. **A `CoverageRollup` predicate** — a new aggregation over the one recursion (Category-C, recompute-on-read).

A new DNA entry type is the *fifth, near-forbidden* move — operator-confirmed, never solo (the entry budget is
precious — verify per DNA before assuming room). The `p2p-design-gate` skill is the per-capability gate; **the
oracle is the catalog-wide gate** that catches a capability drifting toward a parallel stack.

### 4.2 AWS sprawl vs Elohim cohesion — the four mechanisms inverted

| AWS sprawl mechanism | Elohim cohesion mechanism |
|---|---|
| per-service teams, no shared primitive | **every API is one `Mishpat::Commitment` face** — a new coverage domain + a `Governor` impl, never a new entry type |
| idiom drift (4 auth models: IAM/ACL/resource-policy/KMS) | **one auth model: the person's own key signs; `limit_owner` names whose line every refusal honored** — unrepresentable to drift |
| no self-development governor (coherence is a style-guide aspiration enforced by tired reviewers) | **the oracle IS the governor**: friction → escalate the pattern → update the rung → the new capability lands coherently |
| values absent by construction (the customer's problem) | **values compiled into the primitive**: the deficit metric, the agency gradient, the empty center ride along for free |

The decisive inversion: **AWS's coherence is an aspiration enforced by humans in review; Elohim's coherence is
an invariant enforced by the type system and the catalog gate.** A new API cannot be "almost the same machine"
because the only way to enter the catalog is to *be* an instantiation of the one primitive. The grammar is
closed under composition — any API the SDK can reach already speaks the primitive, so the SDK never needs a
per-service special case (the boto3 disease).

### 4.3 The self-development loop (the oracle's escalation organ at catalog-growth altitude)

```
 ① FRICTION  (a developer reaches for a capability the catalog lacks)
 ② GROUND    (the meta-process surfaces the RIGHT rung; the re-substration table asks "which face is this?")
 ③ DECIDE    (two mandatory outputs: the primitive instantiation {face, coverage_domain} AND the
              gradient placement {limit_owner band} — NOT a new entry type, NOT a new auth model, NOT a team idiom)
 ④ UPDATE    (the composition rung gains the face's clause; a new domains/{x}/manifest.json to the SAME schema;
              the coherence gate verifies BOTH declarations before merge)
 ⑤ ABSORB    (the same Governor.check() reaches it; the same limit_owner carries its gradient; the same
              CoverageRollup aggregates it — zero special-case)
 ⑥ HAND BACK (the catalog grew by ONE coherent service; the friction goes to stasis, never re-fires)
```

This is ORACLE-ESCALATION's GROUND→DECIDE→UPDATE→HAND-BACK, with one altitude-specialization: the DECIDE step's
ARCHITECTURE output is constrained to *"the new capability is face F on coverage domain D at gradient position
G."* If GROUND finds no existing face and no way to make one, that is a VISION-level gap (does the primitive
need a genuinely new face?) — parked as an explicit, witnessed operator decision, surfaced once, never nagged.
**The catalog cannot grow a non-coherent service by accident; only by an explicit decision to extend the
primitive itself.**

### 4.4 The coherence gate (the smallest real first step for cohesion)

> A new API does not enter the catalog until it declares (a) which primitive instantiation it is — the
> Commitment face + coverage domain — and (b) its agency-gradient placement — the `limit_owner` band it rides.
> A catalog entry missing either declaration fails the gate.

This rides three things already on disk: the **uniform manifest schema** (every `domains/{x}/manifest.json`
already has `contentTypes`/`signals`/`gates`/`graph` — add two required keys: `primitive_instantiation` and
`gradient_placement`); the **running validation harness** (`pnpm run schema:validate`, already pre-push) gains
one assertion (reject a missing key; reject a per-soul `coverage_domain` — the RECURSIVE unrepresentable-total
rule checked at catalog-entry time); and the **escalation organ** (a failed gate is a PRIMITIVES-rung friction
that teaches the cohesion, not just rejects). **Net: two manifest keys, one assertion, two `domains/README.md`
columns.**

---

## 5 — RECONCILE (how this composes with the rail and the governor)

**The SDK design stands. This adds three things it under-specified: the API-boundary structure, the corpus
scope, and the cohesion-governance.**

- **With SDK-DESIGN (the agency-gradient RAIL) — orthogonal axes, one matrix.** SDK-DESIGN names the *vertical*
  axis: the SDK laid along the agency gradient (human-sovereign below → keystone → veil above), carried in one
  field (`limit_owner`), one absent method (no `govern(person)`). This model names the *horizontal* axis: the
  SDK laid across the many capability APIs, carried in one grammar of five verbs. **They meet at one fact: the
  gradient field rides every grammar verb.** Which API (structure) and which gradient position (rail) are
  independent per-call choices, and the boundary guarantees the rail's downward invariants hold *regardless of
  which API* the verb reaches — `household.govern()` and `veil.govern('margaret')` do-not-compile in *every*
  API, because the grammar owns the absent method. A horizontal grammar with no vertical invariant would be AWS
  with better ergonomics; the gradient is what makes the grammar love-shaped, and the grammar is what makes the
  gradient hold everywhere. The corpus proof's functional axis (auth/storage/feeds/payments) is a column; the
  gradient is the row; the cell is the Governor call with its `limit_owner`. The two axes are one matrix.

- **With the ORACLE (the cohesion-GOVERNOR) — the catalog is a rung, the grammar is a rung.** The oracle's
  seven-rung ladder is WHY → primitives → **composition** → **architecture** → runtime → diagnostics →
  behavior. The composition grammar (§1) *is* rung-3; the capability catalog (§2) *is* rung-4. The catalog's
  growth law (§4) is ORACLE-ESCALATION's escalation organ pointed at platform growth — friction → escalate the
  pattern → update the rung → the next capability lands coherently. The coherence gate is the sixth
  instantiation of the operator's `flag → agent → canon → stasis` sentinel pattern, where the canon written is
  *a new coherent catalog entry*. The oracle measures drift from the one-primitive law (a capability that grew
  a private auth scheme, or a per-soul coverage scalar) and routes it to a decision that edits the boundary —
  so cohesion is enforced twice: structurally (the gate is the only admission) and reflexively (the oracle
  measures and corrects drift).

- **With the two architecture syntheses (the ONE primitive).** Every catalog service is a face of the one
  `Mishpat::Commitment` (ESCALATED's six faces); every feed/graph/search is a `CoverageRollup` over the one
  recursion (RECURSIVE); the "no per-soul coverage domain" rule the gate checks is RECURSIVE's
  unrepresentable-total-account invariant, enforced one layer earlier (at catalog entry, not just at runtime).
  **Zero new DNA entry types** — every new catalog service is a face, never an entry type.

No contradiction across the four parts or the two rails: the boundary is the *structure*, the catalog is the
*surface*, the corpus is the *proof*, the cohesion is the *governance* — and all four ride the one primitive on
the one gradient.

---

## 6 — WHAT LOVE REQUIRES (and the convictions it still defers)

Love requires that the **easy path produces a love-shaped app even when the developer was not trying to build
one.** The proof is in the *absences* the corpus table surfaces: the marketplace developer could not write
`ban(user)` if they tried (no `govern(person)` method to call); the project-tracker manager could not surveil
their team (`CoverageDomain` will not typecheck a per-soul scalar); the feed builder could not rank by
engagement (the only metric is the commons' `deficit`). These developers reached for ordinary capabilities —
listings, tasks, posts — and the love rode along *because it was compiled into the primitive they composed, not
because they chose it.* That is the whole difference from a platform that merely *permits* good apps: Elohim's
catalog *can only build* love-shaped ones.

Three love-requirements, each made structural rather than aspirational:

- **A developer learns the values exactly once — at the first verb they ever call — and can never leave them
  behind**, because the one grammar carries the one primitive across every API the platform will ever grow.
- **A developer browsing for any capability the corpus demands finds it as a re-substrated, love-shaped
  equivalent whose values are visible *in the index itself*** — the one-primitive column is non-optional on
  every catalog row, so the values are a column, not a compliance page.
- **The platform can grow to hold the whole corpus and stay one machine** — because every new capability enters
  only as a new face of the one primitive on the one gradient, so the values never dilute as the surface
  expands; growth is demand-pulled through the friction organ (weighted by `deficit` toward what the commons
  lacks), never supply-pushed by a roadmap quota.

The old system is not defeated app-by-app; it is **subsumed**, because building anything on Elohim is *both
easier* (one cohesive SDK, the relational-DB default literally unscaffoldable) *and love-shaped by default*. The
corpus of human software re-substrates toward flourishing because the easier path is the loving one.

**The convictions this model still defers (held, not resolved):** (1) the seventh→eighth-face question — when a
genuinely-new capability has *no* existing face, the gate parks it as an operator-confirmed extension of the
primitive itself; this model defers *which* capabilities will demand that and whether the entry budget can hold
them. (2) The `run-governor` verb depends on the `elohim-compute::actuation` lift (SDK-DESIGN Step 0b) — until
that lands, the grammar's fifth verb delegates to surfaces only partially extracted. (3) The breadth claim
(AWS-scale option count) is *proven reachable* by the 14-row table but not yet *built wide* — the catalog today
is ~5 tiers, not 300 services; the deferred work is the long instantiation, one coherent face at a time. None of
these are contradictions; they are the honest edge of a model whose smallest first steps are all additive,
reversible, and operator-gated.

> **The closing test, in one line:** love requires that an agentic developer who set out to build *anything*
> from the corpus of human software — and who never read the manifesto — ships an app that keeps every person
> sovereign at the atom of their own life, accounts every value flow as a witnessed reciprocal promise, holds
> the commons open by its deficit, and leaves no lever by which it could be turned against the people it
> serves; because the one cohesive SDK they reached for, over the many APIs they composed, *had no other shape
> to give them.*

---

### THE SMALLEST REAL FIRST STEP (woven from the four parts' steps, in dependency order)

1. **Name the boundary in the tree (from `boundary`):** add `elohim/sdk/src/grammar/` (five thin re-export-only
   verb files), re-point the `ElohimSDK` facade getters at the five verbs, name `elohim/sdk/src/api/` as the
   explicit Layer-2 home. One commit, zero DNA, zero new generated types.
2. **Make the catalog legible (from `catalog`):** write `elohim/sdk/platform-catalog.json` — §2's tables as
   data keyed to real paths — and generate a type-safe `PlatformCatalog` view via the existing `export_bindings`
   seam; anchor it as the oracle's rung-4.
3. **Prove the corpus runs (from `corpus`):** `elohim/sdk/domains/CORPUS-MAP.md` (the §3.1 table as a
   cite-sealed catalog doc) + `create-elohim-app --template=marketplace` (runs against today's substrate, with
   no `ban()`/star-leaderboard/admin-god-mode *to even write*).
4. **Gate the cohesion (from `cohesion`):** add `primitive_instantiation` + `gradient_placement` as two required
   keys on the domain-manifest schema, one assertion in the running `schema:validate` path (reject a per-soul
   coverage_domain; reject a missing placement), and two columns on `domains/README.md`. A failed gate is a
   PRIMITIVES-rung friction that feeds the oracle.

Each step is additive, reversible, operator-gated, spends zero DNA — and together they make the discriminator
*real in the tree*: the grammar is a directory, the catalog is a manifest, the corpus runs as a template, and
no future capability is admitted except as a coherent instantiation of the one primitive.

---

*All package-structure moves, manifest keys, schema assertions, catalog docs, and codegen wiring named here are
additive, reversible, and operator-GATED. This is a proposal for operator blessing — woven from the four part
designs and reconciled with the agency-gradient rail (SDK-DESIGN-2026-06-14) and the cohesion-governor
(ORACLE-2026-06-14 / ORACLE-ESCALATION-2026-06-14) — NOT cite-sealed, NOT a decision, NOT code.*
