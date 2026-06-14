---
title: "THE CORPUS-OF-HUMAN-SOFTWARE COMPOSITION PROOF — one SDK, many APIs, every app re-substrated"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
reconciles_with:
  - SDK-DESIGN-2026-06-14.md                 # the agency-gradient rail (one SDK, limit_owner carries the gradient)
  - ESCALATED-ARCHITECTURE-2026-06-14.md     # the one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md     # CoverageRollup; limit_owner ∈ {self,commitment,operator,faith}
  - ORACLE-2026-06-14.md                     # the cohesion-governor of self-development
the_one_claim: "The corpus of almost all human software composes from the one primitive grammar; each
  app inherits agency-preservation + REA-economy + governance + capture-resistance for free, because the
  primitives carry the values and the developer never asked for them."
do_not_cite_seal: true
forest_test: "Does the easy path produce a love-shaped app even when the developer was not trying to build one?"
---

# THE CORPUS-OF-HUMAN-SOFTWARE COMPOSITION PROOF

> The operator named the discriminator we were missing: **API = a capability boundary (many of them);
> SDK = the one cohesive composition grammar over all of them.** AWS has many services and *no shared
> primitive* — each grew its own auth, its own consistency, its own idioms, so the dashboard is
> incoherent sprawl that develops itself by accretion. Elohim has many APIs too, but they **all speak
> the one primitive grammar and ride the one agency gradient**, so the platform develops *itself*
> coherently. This document is the proof of composition: it maps the corpus of standard
> app-building-blocks — the primitives nearly every piece of human software is built from — onto Elohim
> APIs, RE-SUBSTRATED, and then decomposes three bespoke apps end-to-end to show each is **buildable AND
> love-shaped by construction.** The monorepo's pillars, zomes, crates, and bridges ARE the nascent
> service catalog. This pass proves the catalog is *complete enough to build almost anything* — and that
> anything built on it is agency-preserving, REA-economic, governance-bound, and capture-resistant
> *whether or not the developer asked.*

---

## PART 0 — THE DISCRIMINATOR, MADE OPERATIONAL

The reconciliation with the rail (`SDK-DESIGN-2026-06-14.md`) is exact and three-line:

- **The SDK rail organizes by the agency gradient** (human-sovereign below, keystone, veil above). That
  is the *moral* axis — who may govern whom. **This pass adds the orthogonal *functional* axis** — what
  capability you are building (auth, storage, feeds, payments…). Every cell of the functional axis is
  *located somewhere on the gradient*: `auth` sits at the human-sovereign floor; `payments` at the
  keystone (REA); `permissions/RBAC` and `feeds` straddle keystone→veil (coverage governs them). **The
  two axes are one matrix.** A capability API is a column; the gradient is the row; the cell is the
  Governor call with its `limit_owner`.
- **An API is one capability boundary** — a service over a domain — and there are *many* (the monorepo
  already ships ~50 HTTP route modules at `elohim/elohim-storage/src/api/` and ~80 domain services at
  `elohim/elohim-storage/src/services/`). **The SDK is the *one* grammar that composes them**: the
  `@elohim/*` facade the rail laid out (`SDK-DESIGN-2026-06-14.md` Part 2), every capability reachable
  through *one* type-generated boundary (`cargo test export_bindings` → `storage-client-ts`), every
  write routed through *one* Governor, every datum addressed by *one* CID grammar.
- **The cohesion governor is the oracle** (`ORACLE-2026-06-14.md`): a new capability is *born only as a
  new instantiation of the one primitive* — a `signal_kind` addition, a Commitment `action`
  discriminator, a Governor impl, a CoverageRollup predicate — **never a new web2 stack, never a new
  entry type budget spend.** Part 4 below makes this the catalog's growth law.

**The single structural fact that makes this not-AWS:** in AWS, "add a capability" means "add a service
with its own everything." In Elohim, **"add a capability" means "point the one Commitment+Governor+CID
grammar at a new noun."** That is why the platform develops itself coherently — there is one primitive
to instantiate, one gradient to ride, and one oracle to keep the instantiations coherent as they grow.

---

## PART 1 — THE PRIMITIVE-MAPPING TABLE (the corpus, re-substrated)

The corpus of standard primitives nearly every human application is built from, each mapped to its
Elohim API/primitive **and the value it carries by construction.** Every "Elohim API (real surface)"
column cites a live path in the monorepo today — this is the nascent catalog, not a wishlist. The
"carries-for-free" column is the proof obligation: the developer reaches for the capability and the
value rides along unrequested.

| # | Web2 primitive | Elohim API (real surface, cited) | The re-substration | Carries for FREE |
|---|---|---|---|---|
| 1 | **Identity / auth** (username+password, OAuth, sessions, JWT) | `api/identity.rs` (`register_human`, `get_me`); `imagodei` zome (`create_human`, `get_human_by_agent_key`); `services/elohim_gate.rs` (`TrustContext`, `InferenceTier`, capability tiers) | A person is an **agent-keyed EPR**, not a row. "Login" is *holding your own signing key*; "session" is a `TrustContext` whose mutations escalate by tier (`classify`/`escalate`/`compute`), not a bearer token a server owns. | **Agency floor**: the person keeps their naming (`limit_owner: self`); no server holds the credential of record; `getFeltStatus` can tell you whether you hold your own name or borrow a doorway's (rail atom-authoring). |
| 2 | **CRUD / storage** (rows, documents, KV) | `services/content_service.rs` (`create`/`get`/`update`/`delete`); `api/source_chain.rs`; the trust-plane quilt (`Content` entry, `content_store_integrity/src/lib.rs:521`) | A "record" is a **commitment-witnessed EPR atom** on the lean trust-plane DHT; "write" is `authorAtom()` signed by the person's key → CID → notarized. Storage is a *projection* of distributed truth (P1), not the truth. | **Capture-resistance**: the record is content-addressed (no server owns the namespace); offline-first (works with zero peers); the write names whose line it honored. The relational `GET /thing`+UUID default is *unscaffoldable* (rail dx-onramp). |
| 3 | **Files / blobs** (S3, object store) | `api/blob.rs`; `services/replication_prioritizer.rs`, `replicates_dwelling_service.rs`; the heavy byte-plane quilt — RS(4,7) erasure coding (`sharding.rs:97`), `IrohBlobStore`; `custody-blob` REA action | A blob is **RS(4,7)-sharded, CID-addressed, custody-tracked** on the byte-plane quilt — any 4 of 7 shards reconstruct. "Upload" is a `custody-blob` commitment; "who has my file" is *queried* against REA, not stored in a second ledger. | **REA-economy + capture-resistance**: holding bytes IS a witnessed, revocable care commitment that mints recognition; no datacenter is the single point of custody; a laptop participates (`feedback_household_nodes_is_the_stable_floor`). |
| 4 | **Graph / relationships** (joins, social graph, knowledge graph) | `graph_engine.rs` `ContentGraphResolver` + `NativeGraphResolver` (content↔content); `graph/` Cozo `GraphEngine` (EPR-projection); `services/relationship_service.rs` (depth-bounded BFS) | Relationships are **first-class edges** — Category-A notarized (explicit) and Category-C recompute-on-read (computed, never persisted). Two peers compute identical edges with no doorway, no consensus (`ResolvedNeighborhood`, `inference_source`). | **Capture-resistance**: the graph is recomputable locally — no platform owns "the social graph"; `GraphSpec` *excludes* `MASTERY_OF` so a content walk can never leak into learner-identity (agency floor compiled into the whitelist). |
| 5 | **Messaging / chat** (DMs, channels) | `p2p` gossip plane + `p2p_iroh` (`iroh-gossip`, BLAKE3 topic_id); `api/comments.rs`; `services/session_exchange.rs` | A message is a **reach-gated EPR addressed to a topic/agent** over the gossip plane; a "channel" is a topic_id; delivery is P2P, runtime-selected (libp2p ⊕ iroh). | **Agency + capture-resistance**: no server reads the channel (the substrate moves bytes, doorway only projects a single target — `project_doorway_single_target_no_fanout`); reach gates who may address whom *at authoring*. |
| 6 | **Feeds / timelines** (home feed, activity stream) | `services/reach_earning.rs` (`ReachVerdict`, `evaluate`); EPR + `CoverageRollup` (`graph_views/recursion/`); `api/epr.rs` reach-scoped list | A feed is a **reach-gated EPR set rolled up by CoverageRollup** — what reaches you is what you *pre-authorized standing trust* for, plus what the rollup surfaces by *deficit* (the commons' gap), never by engagement rank. | **No engagement-maximization (the central capture-resistance win)**: the metric is `deficit`, not impressions; reach is *earned at authoring* (`project_reach_earned_at_authoring`), so a feed *structurally cannot* become an attention casino. |
| 7 | **Search / discovery** | `content_service.rs` `search`/`get_by_tag`; `NativeGraphResolver` tag co-occurrence (`inference_source="tag"`); EPR resolution plane (`epr:{id}`) | Search is **local-first query over the projection + recompute-on-read graph**; discovery is content-addressed resolution (`/elohim/epr/2.0.0`). | **Capture-resistance**: no central index owns ranking; results are recomputable locally and identical across peers (no secret ranking model). |
| 8 | **Payments / ledger** (Stripe, double-entry) | `services/economic_event_service.rs` (`create_event`, `bounded_by`); `rea_commitment_service.rs`; `bridges/valueflows/` (VF-GraphQL ↔ REA); `api/economic_events.rs` | A payment is an **REA EconomicEvent fulfilling a Commitment**, bounded by an Agreement — value, provider, receiver, resource, all on one ledger. Care and compute are *categorically isolated* (`signal_kind` + `resource_classified_as`). | **REA-economy by construction**: every value flow is a witnessed reciprocal commitment with on-chain standing + revocation + audit (`project_rea_compute_commitment_primitive`); web2 money bridges *in* via valueflows, never *replaces* the substrate ledger. |
| 9 | **Permissions / RBAC** (roles, ACLs, policy) | `elohim_gate.rs` (capability tiers); `mishpat` zome (consent, `GateDecisionAttestation`); `trait Governor` + `Refusal{limit_owner}` (rail commitment-governor); `services/elohim_gate.rs` `classify`/`escalate` | A permission is a **bounded capability grant under a Commitment, gated by a Governor that refuses-and-elevates and NAMES whose line it honored.** RBAC's "role" is a `bindAgent(scope, grant)` with a verb whitelist; the gradient band selects the constitution. | **Agency gradient compiled**: `limit_owner ∈ {self,commitment,operator,faith}` is non-optional on every refusal; `CoverageDomain` cannot range over a soul; **there is no `govern(person)` method** — the dangerous permission is *absent*, not merely guarded (rail spine). |
| 10 | **Workflow / state machine** (Temporal, jobs, approvals) | `rea_commitment_service.rs` `update_state` (Commitment lifecycle: proposed→active→fulfilled); `recovery_flow_projector.rs`; `ReconcileController`; `mishpat` proposal/vote flows | A workflow is a **Commitment lifecycle governed by a Governor**; a "step" is a state transition projected by a projector-per-flow; an "approval gate" is a `GateDecisionAttestation`. | **Governance-bound**: every transition is witnessed and revocable; approvals are consent attestations on the DHT, not a hidden admin toggle; the flow's blast radius = its granted scope (rail covenant). |
| 11 | **Documents / content** (CMS, wiki, docs) | `content_service.rs`; lamad manifest (`elohim/sdk/domains/lamad/manifest.json`) format→renderer map; `epr-composite` core format; sophia for assessments | A document is a **content-EPR with a manifest-declared format**; authorship + contributor presence are content-derived primitives that *survive transmission* (transfer-on-claim slots). | **Agency**: attribution moves with consent (`ContributorPresence`), not platform fiat; no CMS owns the canonical copy — it is content-addressed and re-publishable (`republish_epr_validator.rs`). |
| 12 | **Realtime / sync** (collaborative edit, CRDT) | `elohim-storage/src/sync/` (Automerge CRDT, `doc_store.rs`, `stream.rs`); `/elohim/sync/2.0.0` delta plane | Realtime is **Automerge CRDT delta sync over the P2P sync plane** — convergence is structural (every ~60s), so divergence is *impossible to serve* (ESCALATED coherence pass). | **Capture-resistance**: no server arbitrates "the truth" — peers converge; the doorway only *observes* a head it cannot author (rail coherence/availability). |
| 13 | **Notifications** (push, email, in-app) | `events.rs` `EventBus`; `IntegrityNotify` signals; `recognition_pipeline_service.rs` triggers; gossip plane broadcast | A notification is a **post-commit signal routed through the dispatcher → projector**, or a reach-gated gossip broadcast; "you were recognized" is a minted-recognition event. | **Patience over engagement**: signals carry truth-deltas, not engagement bait; `surfaceRecognition` returns `null` on `NeedDeeper` (do nothing, no nag — rail veil-walker). |
| 14 | **Jobs / compute** (serverless, queues, cron) | `elohim-compute` crate; `api/compute.rs`; `delegates-compute` REA action; `arc_actuator.rs` (the `trait Governor` first impl); `services/sla_service.rs` | A job is a **`delegates-compute` Commitment** bounded by capacity; scheduling is coverage-gated (`∪ = full`); compute breach signals are *isolated* from care attribution (compute-class ≠ care-class). | **REA + no-overwhelm**: compute is a bounded reciprocal agreement with standing + revocation; the Governor refuses-and-elevates rather than overwhelming (`project_self_healing_control_plane_vision`); displaces X-API-Key admin grants. |

**The reading of the table.** Fourteen rows; **zero new DNA entry types**; **zero new primitives** — every
cell is the *same* Commitment+Governor+CID+CoverageRollup grammar pointed at a different noun, and every
cell's "carries-for-free" column is a value the developer *did not request.* That last column IS the
proof of the discriminator: the cohesion is not that the APIs share a style guide (AWS has style guides);
it is that **they share the one primitive, so they share the values the primitive carries.**

---

## PART 2 — WORKED EXAMPLE A (full decomposition): a peer marketplace

**The bespoke app a developer wants to build:** "a marketplace where my community lists offers and
requests, people transact, and there's a reputation system." The classic web2 build is: Postgres
(listings, users, transactions), Stripe (payments), an auth provider, a recommendation feed, an admin
dashboard with ban powers, a star-rating table. Here is the same app, decomposed into Elohim primitives,
end to end — using **only surfaces that exist in the monorepo today.**

| Marketplace need | Elohim primitive | Real surface | Decomposition |
|---|---|---|---|
| Sellers & buyers (accounts) | Identity (#1) | `imagodei` `create_human`; `api/identity.rs` | Each participant is an agent-keyed EPR. No accounts table; the buyer signs their own offers. |
| Listings (offers/requests) | Exchange + CRUD (#2) | `services/exchange_service.rs` (`create_offer`, `create_request`, `match_offer`, `match_request`) | A listing is an offer/request EPR; matching is `match_request`/`match_offer` — already in-tree, not a new build. |
| Photos on a listing | Files/blobs (#3) | `api/blob.rs`; byte-plane quilt; `custody-blob` | Photos are RS(4,7)-sharded blobs; the listing EPR carries `blob_cid` pointers. Whoever hosts the photo holds a `custody-blob` care commitment. |
| Payment / settlement | Payments (#8) | `economic_event_service.rs` `create_event` `bounded_by`; `bridges/valueflows` for fiat-in | A sale is an REA EconomicEvent fulfilling the matched Commitment, bounded by an Agreement. Fiat rails bridge *in* via valueflows; the ledger of record is REA. |
| Reputation / reviews | Feedback signal (#6/#9) | `signal_kind` (`vouch`!) — `content_store_integrity/src/feedback_signal.rs`; `services/standing.rs`, `steward_standing.rs` | A review is a **`vouch` signal_kind** (already the canonical worked example) — NOT a star-rating table. Standing is projected from vouches; the vocabulary extends without a new entry type. |
| "Recommended for you" feed | Reach-gated feed (#6) | `reach_earning.rs`; `CoverageRollup`; EPR reach-scoped list | The feed is reach-gated EPRs surfaced by `deficit` (whose request the commons hasn't met), not by engagement rank. |
| Trust / dispute resolution | Governance (#9/#10) | `mishpat` proposals/votes; `GateDecisionAttestation`; `qahal_service.rs` `create_collab_agreement` | Disputes are consent-attestation flows in mishpat; the community resolves via witnessed governance, not an admin's ban button. |

**What the developer got for free, having asked for none of it:**
- **No admin god-mode.** There is no `ban(user)` to build — `mishpat` consent flows and `Governor` refusals
  are the only levers, and **no `govern(person)` method exists** (rail spine). The marketplace *cannot* be
  weaponized against a participant by the operator. (capture-resistance + agency)
- **Reviews can't be astroturfed into a leaderboard.** Standing is projected from witnessed `vouch` signals
  with `limit_owner` named; the feed metric is `deficit`, not stars-descending. (no engagement-maximization)
- **The marketplace can't be captured.** Listings are content-addressed EPRs; if the developer's server dies,
  the listings live on the quilt; participants keep their keys, their listings, their standing. (capture-resistance)
- **Every sale is a witnessed reciprocal commitment** with audit + revocation — the developer built "Stripe
  checkout" and got a love-shaped economy. (REA)

The developer asked for a marketplace. They *built* an agency-preserving, capture-resistant, governance-bound,
REA-economic commons — **because the primitives carried it.**

---

## PART 3 — WORKED EXAMPLES B & C (compressed): project-tracker and community feed

**B. Project-tracker (the "Jira / Asana" shape).** Tasks, assignees, status columns, sprints, a burndown.
- Task = a **Commitment** (`rea_commitment_service.rs`), assignee = the committing agent, status column =
  the Commitment lifecycle (`update_state`: proposed→active→fulfilled, #10), sprint = an Agreement bounding
  a set of Commitments, burndown = a **CoverageRollup** over the sprint's Commitments (`∪ = full` is "all
  committed work covered"), notifications = post-commit signals (#13), the team = a `qahal` collective
  (`create_collab_agreement`).
- **For free:** "assigning" someone a task is *impossible as a command* — a task is a Commitment the assignee
  *makes* (witnessed, revocable), not one imposed. The manager cannot `govern` the worker; they can only
  propose and witness (rail covenant, servant gradient). The burndown reports the *deficit* the team failed,
  never ranks individuals (no per-soul `CoverageDomain`). **A project tracker that structurally cannot become
  surveillance** — and the developer just wanted a kanban board.

**C. Community feed (the "Twitter / Mastodon" shape).** Posts, follows, a timeline, likes, moderation.
- Post = a **content-EPR** (#11), follow = pre-authorized **reach** standing (`reach_earning.rs`), timeline =
  reach-gated EPR set rolled up by **CoverageRollup** surfacing by `deficit` (#6), "like" = a **`vouch`
  signal_kind** (#6), moderation = `mishpat` consent + `squelch`/`quarantine` signal_kinds (already in the
  whitelist!) governed by a Governor, realtime = Automerge sync (#12).
- **For free:** the timeline **has no engagement ranking to build** — `deficit`-first is the only metric the
  CoverageRollup exposes (rail veil-walker: "the surface cannot become a leaderboard"). Moderation is
  *witnessed and revocable* (`squelch` is a named signal with `limit_owner`, not a shadow-ban), so the
  platform cannot silently disappear a person. **A social feed that structurally cannot be an outrage
  machine** — and the developer just wanted a microblog.

Three apps, three shapes (transactional, workflow, social), **one grammar.** Each inherited the full value
stack unrequested. This is the composition proof: the corpus of human software is *reachable* from the one
primitive, and *love-shaped* on arrival.

---

## PART 4 — THE COHESION GROWTH LAW (the oracle as catalog-governor)

The monorepo IS the nascent catalog (`api/`, `services/`, zomes, `bridges/`, `elohim/sdk/domains/`). The
difference from AWS — the whole difference — is the **growth law**, and the oracle enforces it:

> **A new capability is born only as a new instantiation of the one primitive.** Concretely, adding a
> capability to the catalog is exactly one of four moves, in escalating cost, and **never anything else:**
> 1. **A `signal_kind` addition** + `resource_classified_as` whitelist entry — a new social move on existing
>    data (the cheapest; `vouch` is the worked precedent). Cost: a string + a validator arm + a projector.
> 2. **A Commitment `action` discriminator** — a new face of the one Commitment (the six-faces table;
>    `replicates-dwelling` is the precedent). Cost: an action name + a coverage invariant. **Zero entry types.**
> 3. **A `Governor` impl** — a new setpoint on the one control spine (`ArcGovernor`→`FloorGovernor` is the
>    precedent). Cost: one trait impl, inequality flipped.
> 4. **A `CoverageRollup` predicate** — a new aggregation over the one recursion (Category-C, recompute-on-read).
>
> A new DNA entry type is the *fifth, near-forbidden* move — operator-confirmed, never solo (the entry budget
> is precious: Lamad ~73/100, Mishpat ~11/100). The `p2p-design-gate` skill is the per-capability gate; **the
> oracle is the catalog-wide gate** that catches when a capability is drifting toward a parallel stack.

This is why the platform develops *itself* coherently. When a developer (or an agentic developer) reaches for
a capability the catalog lacks, the oracle's friction-escalation (`ORACLE-ESCALATION-2026-06-14.md`) does NOT
say "build a new service" — it asks *"which of the four moves instantiates this?"* and routes the answer to
the rung it belongs to. The catalog grows by **deepening the one grammar**, never by accreting idioms. The
manifesto's "lived contrast" lands here at the developer layer: the old stack is not defeated app-by-app; it
is **subsumed**, because building on Elohim is *both easier* (one cohesive SDK, the relational-DB default
literally unscaffoldable) *and love-shaped by default* (the primitives carry the values). The corpus of human
software gets re-substrated toward flourishing as developers reach for the easier, more coherent tool.

---

## PART 5 — THE SMALLEST REAL FIRST STEP

**Ship the primitive-mapping table as an executable artifact + ONE fully decomposed worked example, both
grounded in surfaces that run today.** Concretely, three thin moves, no DNA spend, reversible:

1. **`elohim/sdk/domains/CORPUS-MAP.md`** — the Part 1 table as a living catalog doc next to the existing
   domain manifests (`elohim/sdk/domains/`), each row carrying a *content-addressed cite* to its real
   surface (minted by `cite-gen.py`, the oracle's pointer organ). This is the "AWS-dashboard-in-utero"
   made readable: the monorepo viewed as a capability catalog, one row per primitive, each pointing at the
   live API. It graduates into ORACLE.md rung 3 (COMPOSITION) per the oracle ladder.
2. **The marketplace worked example as a runnable scaffold template:** `create-elohim-app --template=marketplace`
   (extending the rail's `create-elohim-app`, `SDK-DESIGN-dx-onramp`). It wires *only existing surfaces* —
   `exchange_service` (offers/requests/match), `economic_event_service` (settlement), the `vouch` signal_kind
   (reviews), `blob` (photos), `reach_earning` (feed). The first `pnpm dev` IS a love-shaped marketplace
   running against the substrate — with no `ban()`, no star-leaderboard, no admin god-mode *to even write.*
3. **One oracle predicate:** `vision-comparator.py` gains a `catalog-coherence` check — flag any new
   `api/` route module or `services/` file that introduces a capability *without* mapping to one of the four
   growth-law moves (a heuristic: a new route with a write path that does not route through a `Governor` /
   does not extend a `signal_kind` / does not bound an event by a Commitment). It surfaces as a `vision-gap`
   to the cartographer's vision hat — the catalog-cohesion governor, instrumented.

That is one catalog doc + one scaffold template + one oracle predicate. It proves the *entire* claim on real
surfaces: the corpus maps, one app decomposes end-to-end and runs, and the cohesion is *measured* going
forward. Every other worked example (project-tracker, community feed) then composes as a new template, never
a new machine — exactly the oracle's "new predicate, never a new machine" discipline.

---

## PART 6 — WHAT LOVE REQUIRES

Love requires that the **easy path produces a love-shaped app even when the developer was not trying to build
one.** The proof of that requirement is not in the manifesto — it is in the *absences* in the table. The
marketplace developer could not write `ban(user)` if they tried, because no `govern(person)` method exists to
call. The project-tracker manager could not surveil their team, because `CoverageDomain` will not typecheck a
per-soul scalar. The feed builder could not rank by engagement, because the only metric the CoverageRollup
surfaces is the commons' `deficit`. These developers reached for ordinary capabilities — listings, tasks,
posts — and the love rode along *because it was compiled into the primitive they composed, not because they
chose it.* That is the whole difference from a platform that merely *permits* good apps: Elohim's catalog
*can only build* love-shaped ones, because the one grammar every capability speaks carries agency, reciprocity,
governance, and capture-resistance in its very type-shapes.

> **The closing test, in one line:** love requires that an agentic developer who set out to build *anything*
> from the corpus of human software — and who never read the manifesto — ships an app that keeps every
> person sovereign at the atom of their own life, accounts every value flow as a witnessed reciprocal
> promise, holds the commons open by its deficit, and leaves no lever by which it could be turned against
> the people it serves; because the cohesive SDK they reached for *had no other shape to give them.*
