---
title: "THE PLATFORM CATALOG — the monorepo as the nascent platform surface (the AWS-console-in-utero)"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
part_of: "the API/SDK discriminator pass — ONE SDK composed over MANY APIs"
grounds_on:
  - ESCALATED-ARCHITECTURE-2026-06-14.md   # the one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md   # CoverageRollup; limit_owner ∈ {self,commitment,operator,faith}
  - SDK-DESIGN-2026-06-14.md               # the agency-gradient rail (this catalog is its service index)
  - ORACLE-2026-06-14.md                   # the cohesion-governor (this catalog is a rung it reads)
reconciles_with:
  - SDK-DESIGN-2026-06-14.md   # APIs (this doc) = the services; SDK (that doc) = the one grammar over them
  - ORACLE-2026-06-14.md       # the catalog is ORACLE.md rung-4 (architecture), legible per-service
do_not_cite_seal: true
forest_test: "Does the catalog make the platform legible AND the love-shape visible at every service — not buried per-team?"
---

# THE PLATFORM CATALOG
## The monorepo, read as the nascent platform-service surface

> The operator's recognition: *"the monorepo IS the nascent AWS dashboard / GCP surface."* This doc takes
> that literally. It does **not** propose new services — it **names what already nascently exists** in the
> tree as a coherent capability catalog, the way an AWS console lists S3, IAM, Lambda. The whole difference
> from AWS is the closing column of every table: **the ONE primitive each service instantiates.** AWS's
> services each grew their own idioms, auth, and consistency — incoherent sprawl, no shared primitive, no
> self-development governor. Every service in this catalog speaks the *one* `Mishpat::Commitment` grammar,
> rides the *one* agency gradient (`limit_owner`), and is born coherent **because the Oracle is the
> cohesion-governor** (a new capability lands only as a new instantiation of the one primitive).

---

## PART 1 — THE DISCRIMINATOR, MADE CONCRETE

The pass we were missing is a boundary between two words that the codebase already keeps separate but never
*named* as the distinction:

- **API = a capability boundary.** A service over a domain. **MANY** of them. The AWS-services analog. In
  this repo an API is: a coordinator zome (DHT capability), a service module + its HTTP routes
  (`elohim-storage/src/services/*` + `src/http.rs`), a doorway route group, or a bridge crate. Each is a
  *capability* — "do identity," "do storage," "do governance."
- **SDK = the ONE cohesive composition grammar over all the APIs.** Not a service. The primitives an agentic
  developer composes to build *any* bespoke app. Designed in `SDK-DESIGN-2026-06-14.md`: `authorAtom`,
  `person.{commit,grant,revoke}`, `Governor.check()`, `CoverageRollup`, `VeilWalker`, `bindAgent`,
  `Runtime::launch`. **One grammar, laid along the agency gradient.**

The catalog below is the **API column** — the service surface. `SDK-DESIGN` is the **SDK column** — the
grammar. The two are reconciled by one fact: **every API in the catalog is reachable from the one SDK
grammar, and every SDK call carries the gradient in one field.** A developer browses the catalog to find a
capability; they compose it with the one SDK. They never learn a per-service idiom, because there is none —
there is one `Commitment`, one `Governor.check()`, one `limit_owner`.

> **The breadth target vs. the cohesion.** The catalog must eventually reach **AWS-scale option count** —
> auth, storage, graph, messaging, feeds, search, payments, workflow, documents, permissions, realtime/sync,
> notifications, jobs/compute, files (the corpus of almost all human software). But unlike AWS, every option
> is *one primitive re-pointed*. Breadth without sprawl. The platform can grow to a thousand services and
> stay one machine, because the Oracle refuses any service that isn't a new face of the one Commitment.

---

## PART 2 — THE CATALOG (the nascent dashboard, by service)

Each service: **where it lives today**, **the capability it exposes (the web2-corpus slot it re-substrates)**,
**the API surface (real paths/zomes)**, and **the ONE primitive face it instantiates** (from the
ESCALATED §1 six-faces table). The web2 column is the legibility hook — a developer recognizes "this is my
Auth0 / S3 / Stripe," and the primitive column is the love-shape hook — they see *by construction* it is
agency-preserving, REA-economic, governance-bound.

### Tier 0 — Identity & Access (the agency root)

| Service | Lives in (monorepo) | Capability (web2 slot re-substrated) | API surface | ONE-primitive face |
|---|---|---|---|---|
| **Identity** | `elohim/holochain/dna/imagodei/` (48 `#[hdk_extern]`); `services/imagodei_lookup.rs`, `presence_service.rs`, `relationship_service.rs` | Auth0 / Okta / Firebase Auth — **but the person's own key signs; no account is owned by a provider** | `/api/v1/identity{,/me,/register}`, `/api/v1/presence{,/{id}/claim,/verify-claim}`, `/api/v1/relationships`; zome `create_human`/`get_my_human` | **person-keeps-their-naming** invariant (SDK §1) — signer is the person's key; `limit_owner: self` |
| **Permissions / Capability** | `services/steward_affinity_service.rs`, `steward/{grants,gates,credentials,access}` routes; `sealed_against_self.rs` | IAM / RBAC / API keys — **but a capability is a revocable Commitment, not an admin grant** | `/api/v1/steward/{grants,gates,credentials,access}`, `/api/v1/account/keys`, `/_capability` | **capability-as-commitment** (`revokes-capability`/`rotates-wrap`) — person-held, revocable grip |
| **Recovery** | imagodei recovery v2; `/api/v1/account/{recovery,pending-recovery,self-revocation,revocations}`, `recovery_flow_projector.rs` | "Forgot password" / account recovery — **but recovery is a witnessed quorum, not a support ticket** | `/api/v1/account/recovery/{id}/vote`, `/portal-hosts` | **covenant/quorum face** — recovery quorum as a bounded witnessed Commitment |
| **Attestation** | `attestation_projector.rs`, `/api/v1/attestations{,/unified,/{id}/revoke}`; mishpat attestation flows | Verified-credentials / KYC / OAuth scopes — **but attestation is peer-witnessed, revocable, on-DHT** | `/api/v1/attestations/**` | **the witnessing atom** (different EPR, different signer) — attribution that survives transmission |

### Tier 1 — Storage, Content & Graph (the substrate floor)

| Service | Lives in (monorepo) | Capability (web2 slot) | API surface | ONE-primitive face |
|---|---|---|---|---|
| **Storage / Two-Quilt** | `elohim/elohim-storage/`; `services/{distribution,replication_prioritizer,shard_manifest_backfill}.rs`; `IrohBlobStore` | S3 / GCS / Blob — **but bytes are RS(4,7) custody-tracked, no single owner** | `/api/v1/blob/{hash}/distribution/details`, content PUT/GET (GET-only on content-addressed) | **custody-as-coverage** (`custody-blob`, `∪ custody ⊇ corpus, ≥ r_floor/shard`) |
| **Content / EPR** | `services/{content_service,epr_service,epr_store,epr_compose}.rs`; `elohim/sdk/epr-ts/`; lamad domain | CMS / Contentful / Notion API — **but content is content-addressed (CID = identity), reach earned at authoring** | `/api/v1/content`, `/api/v1/epr/{cid}/nav-context`, zome `create_content` (169 externs) | **the authoring atom** — `authorAtom()` build→CID→sign; reach earned, not granted |
| **Content Graph** | `elohim-storage/src/graph_engine.rs` (`ContentGraphResolver` + `NativeGraphResolver`) | Neo4j / Neptune / graph API — **but read-only by construction, recompute-on-read, fully local** | `relationship_service` BFS over `RELATES_TO` | **Category-C derived** — no write method by design (two peers compute identical edges, no consensus) |
| **EPR Social Graph** | `elohim-storage/src/graph/` (Cozo `GraphEngine` on sled); `graph_views/` | Social-graph API / follow-graph | EPR nodes + couplings/memberships/delegations as edges | **the recursion substrate** — CoverageRollup walks these edges (SDK §coverage-rollup) |
| **Sync / Realtime** | `crates/elohim-sdk/src/sync/`, `elohim-storage/src/sync/` (Automerge CRDT, 60s converge); `connection.ts` strategy | Firebase Realtime / Liveblocks / WebSocket — **but CRDT convergence is a substrate property, capture-resistant** | `/elohim/sync/2.0.0` plane; `/api/v1/cache/stream` | **head-coverage** (`covers-head`, quorum Q + freshness ≤ T) — convergence as governed coverage |

### Tier 2 — Economy & Governance (the value/policy spine)

| Service | Lives in (monorepo) | Capability (web2 slot) | API surface | ONE-primitive face |
|---|---|---|---|---|
| **Economy / REA** | `shefa` pillar + domain; `services/{economic_event_service,agreement_service,resource_service,token_mint_service,token_ledger_service}.rs`; `bridges/valueflows/` | Stripe / Ledger / accounting API — **but value is REA-accounted, care-class isolated from compute-class** | `/api/v1/{economic-events,agreements,commitments,exchange/{offers,requests}}` | **care-as-commitment** (`provide-care`, floor/ceiling) — witnessed care → minted recognition |
| **Stewardship** | `services/{stewardship_service,steward_standing,standing_projector}.rs` | Royalties / attribution-payout — **but stewardship is committed, not claimed; no ownership** | `/api/v1/stewardship/**`, `/api/v1/steward-affinity/**` | **CustodianCommitment** — stewardship as a bounded promise, not a title |
| **Governance / Constitution** | `mishpat` DNA (28 externs); `qahal` pillar; `services/{governance_health,constitutional_ratio_registry}.rs` | Snapshot / Aragon / policy-engine (OPA) — **but governance is qahal-collective, constitution-bound** | `/api/v1/governance/{proposals,challenges,precedents,discussions,sensemaking}/**` | **the Governor + constitution band** — `Governor.check()` from an inherited constitution (`limit_owner: operator`) |
| **Limit-Governance** | `services/{limit_gradient_registry,token_decay_service,floor_protections}.rs`; the donut | Quotas / rate-limits / billing-caps — **but the line is the person's own (`self`) or the commons' (`operator`)** | `/admin/arc-policy/actuate`, donut floor/ceiling | **self-limit** (`respects-self-limit`, subject == author) — the donut outer ring as coverage |
| **Recognition / Care-mint** | `services/{token_mint_service,recognition_pipeline}`; `/api/v1/recognition/distribute`, `/contributors/{id}/recognition` | Loyalty / reputation API — **but minted from witnessed care, never engagement** | `/api/v1/recognition/distribute` | **care-as-commitment** — observe→mint seam; no engagement counter exists |

### Tier 3 — Compute, Coverage & Resilience (the control plane)

| Service | Lives in (monorepo) | Capability (web2 slot) | API surface | ONE-primitive face |
|---|---|---|---|---|
| **Compute / Delegation** | `elohim/elohim-compute/`; `services/{arc_actuator,arc_policy,peer_capacity_service,hub_capacity_service}.rs` | Lambda / Cloud Run / job-runner — **but compute is a bounded, witnessed, revocable Commitment** | `/api/v1/compute/dashboard`, `/admin/self-healing` | **`delegates-compute`** (the FIRST primitive instance) — bounded reciprocity + revocation |
| **Coverage / Arc** | `services/{arc_actuator,replicates_commons_validator,replicates_dwelling_service}.rs`; `bounds_validator.rs` | Auto-scaling / replication-policy — **but coverage is `∪ = full`, refuse-and-elevate, names whose line** | `/api/v1/diagnostics/validate-bounds`, `replicates-dwelling` action | **arc-as-coverage** (`commits-arc-coverage`, `∪ arcs ⊇ FULL`) — the Governor's first impl |
| **Resilience** | `services/{resilience,household_resilience,vulnerability,risk_alert,hazard}.rs` | Health-checks / SLA / status-page — **but `not-yet-seen` is never `at-risk` (honesty fold)** | `/api/v1/resilience/{content_id}{,/household,/verify}` | **CoverageRollup** — `deficit` (the commons' failure), never the holding (SDK §coverage-rollup) |
| **Node / Topology** | `node-registry` DNA; `services/{peer_topology_view,peer_diversity,cluster_view,hub_resolver}.rs` | EC2-inventory / k8s nodes — **but a laptop is a full participant (hub-optional floor)** | `/api/v1/cluster`, `/api/v1/nodes/shape`, `/api/v1/households/{id}/devices` | **the runtime floor** — `Runtime::launch` succeeds with ZERO peers, no hub |

### Tier 4 — Gateway, Federation & Web2 Bridges (the narrowly-scoped concession)

| Service | Lives in (monorepo) | Capability (web2 slot) | API surface | ONE-primitive face |
|---|---|---|---|---|
| **Gateway / Doorway** | `doorway/doorway-service/`; `routes/{api,apps,blob,storage_proxy}.rs` | API Gateway / Cloudflare / CDN — **but thin web2 translation, single-target, no blob fan-out, no domain logic** | doorway `/`, `/app/:port`, `/auth/*`, `/import/*` | **the seam** — `IConnectionStrategy`; projects a single target, never authors truth |
| **Federation** | `doorway/services/{federation,discovery,did_resolver,pkarr_resolver}.rs`; `/elohim/view-federation/2.0.0` | Federation / SSO-federation / DNS — **but agents configure doorway, not the reverse** | `/api/v1/federation/**`, `/elohim/view-federation/2.0.0` | **head-coverage observed** — doorway observes a truth it cannot author |
| **Web2 Bridges** | `bridges/valueflows/` (live); `bridges/{atproto,activitypub}` (planned per CLAUDE.md) | Zapier / webhooks / protocol adapters — **but bridges translate INTO the EPR-REA substrate, never weaken it** | `valueflows-bridge` (VF-GraphQL ↔ REA) | **the bridge pattern** — external protocol → canonical Commitment, capture-resistant on entry |

---

## PART 3 — HOW A DEVELOPER BROWSES IT (the dashboard, in use)

The catalog **is** the dashboard. An agentic developer (or a human) browses it three ways, all rooted in
files that exist or land cheaply:

1. **By capability ("I need storage").** They open the catalog, find Tier 1 → Storage/Two-Quilt, read the
   one-primitive column, and *immediately know* the love-shape: bytes are custody-tracked, no owner. They
   compose it with the one SDK call (`putContent`/`getContent`, SDK-DESIGN PART 2). They never read a
   per-service auth doc — auth is `person.commit` + `limit_owner`, the same everywhere.
2. **By web2-analog ("where's my S3 / Stripe / Auth0?").** The web2 column is the migration index. A
   developer porting an app scans it like a Rosetta stone: "my Auth0 → Identity (Tier 0), my Stripe →
   Economy/REA (Tier 2), my Lambda → Compute (Tier 3)." Every analog resolves to a re-substrated, love-shaped
   equivalent. This is the **lived contrast at the developer layer** (manifesto): the old stack is not
   defeated app-by-app — it is *subsumed*, because the cohesive SDK is easier AND love-shaped by default.
3. **By the agency gradient (the SDK rail).** The catalog cross-references `SDK-DESIGN`'s gradient: Tier 0
   Identity is the human-sovereign floor (`limit_owner: self`); Tiers 1–3 are the keystone (the Governor, the
   rollup); Tier 4 is the veil/seam. A developer choosing a service *sees its gradient position* — and the
   two dangerous calls (`household.govern`, `veil.govern(person)`) **do not exist in the catalog**, because
   no service exposes them.

> **The dashboard's many layers (the AWS-options analog).** Like an AWS console has S3 → buckets → lifecycle
> → replication → access-points, each catalog service has depth: Storage → blob → shard → custody → coverage
> → reconciliation. The depth is browsable (the route lists above are the nascent "all options" view). But
> every option, at every depth, is the **same primitive**. That is the cohesion AWS never had.

---

## PART 4 — THE COHESION GOVERNOR (why the catalog can't sprawl)

AWS sprawled because nothing governed how a new service was born — each team invented its own idioms. The
catalog cannot sprawl, because **the Oracle is the cohesion-governor** (`ORACLE-2026-06-14.md`). The binding,
made concrete for this catalog:

- **The catalog IS a rung.** It is rung-4 (architecture) of `ORACLE.md`'s seven-rung ladder
  (WHY → primitives → composition → architecture → runtime → diagnostics → observed-behavior). It sits below
  the SDK (composition, rung-3) and the one Commitment (primitives, rung-2), and above the diagnostics that
  measure each service. Reading the ladder top-down, the catalog is *where the vision becomes services*.
- **A new service is born only as a new face of the one Commitment.** The ESCALATED §1 six-faces table is the
  birth-certificate registry. A proposed new capability that is NOT a re-pointing of `Mishpat::Commitment`
  (a new DHT entry type, a new auth idiom, a CID-as-column) is exactly what the `p2p-design-gate` skill and
  the Oracle's `vision-comparator` refuse. **Net new DNA entry types to add the entire designed SDK: zero**
  (SDK-DESIGN PART 1). The catalog grows by *instantiation*, not invention.
- **Friction-escalation keeps the rungs coherent as it grows.** When a developer hits friction composing two
  services (the seam doesn't fit), the Oracle's escalation organ surfaces it as a `vision-gap` → the
  cartographer edits the rung → the next service lands coherently. The platform develops *itself* —
  observed-behavior teaches the architecture, exactly the System-4 loop (`ORACLE` Movement 3).

---

## PART 5 — THE SMALLEST REAL FIRST STEP

**The catalog doc itself, as a machine-readable manifest keyed to monorepo paths — the nascent dashboard.**

Not a refactor, not new services: a single `platform-catalog.json` (or the markdown tables above, lifted into
the existing `elohim/sdk/domains/README.md` lineage which *already* tabulates domain→pillar→DNA→purpose). The
manifest carries, per service: `{ name, tier, lives_in: [paths], web2_analog, api_surface: [routes|zomes],
primitive_face, gradient_position }`. Concretely:

1. **Write `elohim/sdk/platform-catalog.json`** — the tables in PART 2, as data. Keyed to the *real* paths
   this doc cites (verified against the tree: `services/*.rs`, the DNA zomes, the ~90 routes in `http.rs`,
   `bridges/valueflows/`). One file. Reversible. Zero code, zero DNA.
2. **Make it the rung-4 anchor of `ORACLE.md`.** When the Oracle ladder lands, rung-4 (architecture) cites
   this manifest — so the executive walk from vision to service passes *through* the legible catalog.
3. **Generate the browse view from it.** The same `cargo test export_bindings` / codegen seam that produces
   446 TS views can emit a `PlatformCatalog` type — so the dashboard is a first-class, type-safe surface a
   developer (or an agentic dev) queries, not a wiki page that rots.

This is the AWS-console-manifest, in utero: today it is one JSON describing what the monorepo already exposes;
tomorrow it is the surface every new service registers into, governed by the Oracle. The first step proves the
*legibility* claim with no risk — and every later capability is a new row, never a new machine.

---

## PART 6 — WHAT LOVE REQUIRES

The catalog's love-requirement is **legibility-with-the-values-visible**. AWS's catalog makes capability
legible but hides the values (each service's politics — lock-in, surveillance, the owner's grip — is buried
per-team, discoverable only by the harmed). This catalog's discipline is the inverse: **the one-primitive
column is non-optional on every service**, so a developer cannot find a capability without also seeing,
*at the point of choosing it*, that it is agency-preserving (the person's key signs), REA-economic (care
isolated from compute), governance-bound (the Governor names whose line), and capture-resistant (no owner, no
fan-out, no soul-domain). The values are not a compliance page; they are a *column in the index*.

And the cohesion serves the **builder**, not the platform: the developer composes one grammar over many
services and gets love-by-construction *for free* — the easier tool is also the more humane one, so the
corpus of human software re-substrates toward flourishing simply because developers reach for the more
coherent thing. The platform grows toward flourishing not by decree but by **gravity** — the Oracle ensures
every new service is born as a face of the one Commitment, so the catalog can reach AWS-scale breadth and
still be one machine that loves its makers.

> **The closing test, in one line:** love requires that a developer browsing for any capability the corpus of
> human software demands finds it as a re-substrated, love-shaped equivalent whose values are visible *in the
> index itself* — so that building anything on Elohim is both easier and more humane by default, and the
> platform grows toward flourishing because the easier path is the loving one.

---

*All file moves, `--seal` acts, and codegen wiring named here are operator-GATED. This is a proposal grounded
in the real monorepo (paths verified 2026-06-14) and the night's corpus, for operator blessing — not yet
cite-sealed, not a decision, not code.*
