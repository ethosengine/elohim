---
title: Imagodei Profile-vs-Page — viewer-relative disclosure lens + addressing
id: imagodei-profile-page-viewer-lens-design
status: Draft
class: protocol-canonical
domain: D2
topic: [imagodei, profile, page, identity, addressing, viewer-lens, reach, relationship, intimacy-gradient, disclosure, sacred-floor, contributor-presence, who-is-who]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
refines:
  - genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
cites:
  - imagodei-surfaces | CANONICAL three-surface identity decomposition this refines; Surface 1 (social) IS the viewer-relative lens designed here | path: genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
  - epr-route-claims-link-conformance-design | the addressing + no-undesigned-wall mechanism the page landing + pretty handle compose (universal /epr/{id} floor, pretty mounts, doorway dispatch, validate_project_epr_commitment) | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md
  - contributor-presence-bootstrap-whoswho-design | the page substrate: ContributorPresence, the claim flow, recognition-before-registration | path: genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
  - resilience-facings-select-fold-aggregate-design | the pure DB-free fold framework the viewer-lens fold is a child of | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - contributor-visual-surfaces-plan | the frontend plan whose Sprint 2 (imagodei profile) this design reframes onto the viewer-lens architecture | path: genesis/docs/superpowers/plans/2026-06-22-contributor-visual-surfaces-plan.md
requires_env: [household-nodes]
---

> **Method / provenance.** Synthesized 2026-06-22 from a diverse-design workflow: 6 parallel substrate
> readers → 5 independent design approaches (minimal-compose · page-as-EPR · lens-primitive-first ·
> sacred-floor-first · disclosure-capability-first) → 3 diverse-lens judges (gate-compliance ·
> sacred-robustness · composes-implementable) → synthesis → **adversarial verification**. The adversarial
> pass found and source-confirmed two load-bearing safety bugs the first synthesis had *assumed away*
> (C1: a consent-blind relationship read that leaks intimate facets; H2: a standing "clamp" that does not
> exist in code and is inert at cold-start). The design below is the corrected version — it **builds** the
> consent gate it depends on rather than citing a non-existent one. See "Adversarial review" at the end.

## Decision log — operator-adjudicated (2026-06-22)

These resolve the "Open questions for the operator" (below); where they conflict with the body —
especially §4's "honest service-layer enforcement" framing and §7 Slice 6 — **this log governs.**

- **D1 — Enforcement posture: harden the sacred floor IN-ARC (resolves Open-Q1/Q2).** The disclosure
  floor is *notarized* in this arc, not left at service-layer trust. The hardening is **three legs, and
  only one is deferred:**
  - **Leg 1 — IN-ARC:** notarize `ConstitutionalFloor` (A) + `FacetDisclosureEvent` (A2) as Mishpat DNA
    entries (~11/100 headroom) — "an intimate facet never leaks" becomes **validation-enforced at the
    DNA layer**, not service-trust. (Was §7 Slice 6's first half; now core scope.)
  - **Leg 2 — IN-ARC:** un-stub the claim-verification validator (email / dns-txt control of a presence).
    Folds into the claim work (§7 Slice 4).
  - **Leg 3 — SEVERED (deferred):** cross-signing `AgentPeerBinding` (libp2p keypair ↔ Holochain agent
    key) is **NOT in this arc.** Per `2026-06-15-coherent-transport-identity-resolver-design.md` §0, the
    real fix is an *unbuilt* "step 0" cross-signed control proof — a live-security-issue project a prior
    red-team deliberately shelved. It is **off this feature's critical path**: the only storage consumers
    of `AgentPeerBinding` are economic-attribution/resilience joins (`cluster_view`, `reciprocity_view`,
    `resilience`) — none in the lens/profile/reflexive path; the viewer authenticates by **doorway
    session, not transport binding.** Leg 3 gates ONLY "cryptographically-proven economic attribution" —
    the reflexive feed renders honestly-caveated ("observed, not proven") without it. Severed to the
    filed security-backlog prerequisite (`genesis/data/timeline/backlog/`).
- **D2 — Re-plan around the lens (resolves the "next move").** The `contributor-visual-surfaces-plan`
  Sprint 2 is re-planned onto this viewer-lens architecture. **Execution order: 1 → 2′ → 4 → leg-1 → 3
  → 5** — *harden in-arc ≠ harden first.* §7's Slice 2 splits into **2′** (the service-layer risk-closer:
  consent-filtered read + counterparty-consent write + `profile_reach` default-private flip + raw-CRUD
  hardening — the **full C1 fix, NO DNA**) and **leg-1** (notarize `ConstitutionalFloor` +
  `FacetDisclosureEvent`), which is sequenced **later** so the visible page/profile surface (Slice 4)
  lands early on the service-layer floor and leg-1 DNA notarization upgrades enforcement *underneath an
  already-rendered surface*. C1 is closed by the service-layer consent fix, **not** by notarization, so
  moving leg-1 later does not reopen the leak. (§7's slice list is the design grouping; this is the
  execution sequence.)
- **Q3 / Q4 / Q5 — still open (interim assumptions hold):** the local `intimacy→tier` / `reach→label`
  mapping lands without blocking on the global reach reconciliation (Q3 = yes); the `profile_reach` flip
  posture (Q4) and claim-vs-administer-in-v1 (Q5) are resolved at plan time, not here.

# Imagodei Profile-vs-Page: Final Design

**Status legend used throughout:** `[BUILT]` = exists in substrate today · `[DESIGN]` = composed from built pieces, not yet wired · `[NET-NEW]` = new code/config this arc introduces · `[GATED]` = operator-gated hardening, not v1.

**The spine.** A profile is *one* viewer-relative disclosure graph. The "page" is its commons band; the "sacred profile" is the same graph walked deeper. The lens that decides what a viewer sees is **computed, never stored** — a read-time fold keyed on the qualitative **viewer-to-subject relationship**, not on reach. Reach is demoted to a per-facet sensitivity *label*. The page-vs-profile seam rides the `commons` / non-`commons` line that `validate_project_epr_commitment` already enforces, so it needs **no new addressing scheme and no new DHT entry type**.

**What the adversarial pass changed (headline).** The first draft asserted a consent-gated floor ("a viewer cannot self-elevate") and a standing "betrayed-intimate clamp" — both resting on functions that do not do what the draft claimed. The verifier confirmed both end-to-end against source. This final design **builds** the consent gate it names (a new consent-filtered read + a counterparty-consent write check, both `[NET-NEW]`), and **demotes** standing from a load-bearing safety floor to an honestly-scoped, cold-start-inert *advisory demotion*. The enforceable floor in v1 is consent + default-private + raw-CRUD hardening — not standing.

---

## 1. The profile-vs-page model and how the two surfaces relate

There are two surfaces, and the dominant failure mode is collapsing them into one ontology. They are kept distinct by a single discriminator: **is this surface YOU, or an entity you run?**

### The unifying frame

Page and profile are **two ends of one viewer-relative gradient, not two artifacts.** The imagodei is a disclosure graph spanning commons → intimate. The *commons band* of that graph is public, brandable, cacheable, crawlable — it IS your public-figure face. The *interior bands* (familiar → trusted → intimate → self) are served live, per-viewer. The seam between them is exactly the `commons` / non-`commons` line that `validate_project_epr_commitment` (`rea_commitments.rs:1089`) `[BUILT]` already enforces at grant time. You never maintain two profiles; you tune one graph's gradient.

```
SACRED INTERIOR (live, per-viewer)          COMMONS BAND (cached, world-readable)
  self ─ intimate ─ trusted ─ familiar ──── │ ──── commons
  served live, relationship-gated           │   frozen → /epr/{id} + pretty mount
                                            └─ split at the line validate_project_epr_commitment enforces
```

### The two-entity reconciliation (the page-model collision, resolved)

The frame above describes **your own public face**. But a "public figure or organization" page is sometimes *not you* — it is an entity you operate. These are different ontologies, kept separate by the discriminator:

| Case | What it is | Substrate | Edge to the human |
|---|---|---|---|
| **Your public-figure face** | The commons band of *your own* imagodei | `Human` (agent-keyed) `[BUILT]` — the same graph, walked shallow | identity (it IS you) |
| **An org / band / persona / historical presence** | A *separate* entity with its own brandable face | `ContributorPresence` (`presenceType: person\|organization`, slug-addressed, recognition-accruing) `[BUILT]` | one of two edges below |

The two edges that bind a `ContributorPresence` to a human are **distinct operations** and must not collapse:

1. **CLAIM = "this presence IS me."** The built collapse flow (`POST /db/presences/{id}/claim` → `verify-claim`, `contributor_presences.rs:378–451`) `[BUILT, but verifier STUBBED — see §4 honesty constraint]`. Recognition transfers to the human; `claimed_agent_id` binds. One-shot, terminal. You only claim *your own* historical/seeded presence. Claiming **adds the intimate interior** on top of a previously-ownerless commons band (an unclaimed presence is commons-only precisely *because there is no agent to be intimate with*).

2. **ADMINISTER = "I run this page."** Reuse the built `steward_id` / `HumanRelationship(relationship_type="stewardship")` edge `[BUILT]`. The page stays a **separate** `ContributorPresence`; the human stays a separate `Human`. This is the Facebook-page case: one human administers many org pages, none collapsing into their personal identity. **Zero new entity.**

**Moving between surfaces** happens on the account-management surface (Surface 3 of `imagodei-surfaces-design.md`, reachable via doorway recovery web-path OR steward device — same route, different auth): one tab tunes your own imagodei gradient (which facets sit at which band); another creates/brands/administers the org presences you steward. The pretty handle (§2) is what the public sees; the agent key is what you hold.

---

## 2. Addressing — canonical identity, pretty handle, public page landing

Honoring established-fact #2 exactly. The mutable page body is **never** body-hashed.

| Layer | Form | Mutability | What it names | Resolver | Status |
|---|---|---|---|---|---|
| **Canonical identity** | agent key `uhCAk…` (claimed) or slug `human-…` / `presence-…` (unclaimed) | immutable anchor | the being | the key/slug itself — NOT a content-CID | `[BUILT]` |
| **Pretty handle** | `/in/{name}`, `@name@doorway` | mutable alias, per-doorway-unique | human-friendly indirection | doorway-scoped routeClaim → 302 → canonical | `[NET-NEW]` |
| **Profile/page body** (Cat C) | the rendered surface | editable | the composite face | resolved *through* the canonical id; never body-CID-addressed | `[DESIGN]` |
| **Cited immutable content** | attestations, established content, EPR heads | immutable | the atoms a face cites | `/epr/{cid}` | `[BUILT]` |

**Universal floor (cannot lie):** every identity and every facet has `/epr/{id}` as the backstop viewer `[BUILT]`. The pretty mount is the 302 fast-path when the walk passes.

**The handle resolves; it does not contain.** `@alice@doorway.elohim.host` is a doorway-scoped routeClaim (Mastodon `handle@instance` model, per-doorway uniqueness at grant time). It resolves to the *canonical identity*, after which the doorway walks the graph for the requesting viewer (§3). Resolving a handle does NOT dump a profile.

**Clean canonical/alias discipline.** ONE canonical identity per being; handles are edge-resolved aliases. A separate `ContributorPresence` (org page) gets its *own* slug and its *own* pretty mount — it is a different canonical, not a sub-route of yours.

**Surface-coherence scope (corrects M5).** The canonical+pretty addressing above is delivered **for Surface 1 (social) in v1, and that is an explicit scope choice, not a coherence claim across all three surfaces.** The other two surfaces are addressed honestly:
- **Surface 3 (account-management)** is reached by *capability*, not by a public address: the doorway recovery web-path (auth-gated, e.g. `/account` behind session) OR the steward device's peer-native route — same logical surface, two auth paths, deliberately **not** publicly addressable (publishing an edit surface at a guessable URL is a footgun). Its "address" is "authenticate as the subject, then `/account`."
- **Surface 2 (self-knowledge)** is **deferred by `imagodei-surfaces-design.md` itself** and gets no v1 address. We do not claim one.

So: **part-(e) human-friendly addressing is fully delivered for the social surface; the account-management surface is capability-addressed (auth-gated, by design); self-knowledge is out of scope.** This is stated rather than papered over.

**Hash-hiding (requirements A + E).** The doorway's `IdentityResolver` `[NET-NEW]` is the human-friendly backend. A human types "Alice Rivera, musician," picks a handle, writes a bio, drops links. The service mints — underneath — a `project-epr` commitment (page CID), a routeClaim (`/in/alice` or `/org/alice-music`), and the head-edge faces, surfacing none of the `bafyrei…`/`uhCAk…` complexity. Discovery search falls back to recognition-ranked `ContributorPresence` lookup (the Google-Maps crowdsourcing attractor; §4).

---

## 3. The viewer-relative lens — the primitive, with reach interrogated honestly

### Reach is the wrong primitive. The proof, not the assertion.

Reach as built (`epr/src/reach.rs`, `openness:41`) `[BUILT]` is:
- **Content-scoped, not viewer-scoped** — it sits on the EPR `Envelope`, declaring the visibility *of a content unit*: "at what level was THIS published?"
- **Author-earned + receiver-pre-authorized** (`reach_earning.rs`, `p2p/reach_authorization.rs`) — the author earns the right to publish at a level; a receiver qualifies by participation tier. **There is no `(viewer, subject)` term anywhere** in the reach computation. It is monotonic (Private=1 … Commons=8) and binary at the gate (authorized/refused), never graduated by closeness.
- **3-vocabulary-drifted** (schema 8 / storage 8-different-shape / resilience 5) — a fragile axis to hang identity privacy on.

Reach answers *"who may receive this content atom?"* The lens must answer *"which facets of this human does this viewer see?"* Different questions. Using reach for both is the **misrouting** the seam-map atlas warns about: a relationship fact in a content-reach costume. **Reject reach as the lens driver; relocate it to the per-facet sensitivity label.** (The verifier independently re-confirmed reach has no viewer term and withdrew its own draft worry about an ordinal inversion — the ladder is monotonic, no inversion.)

### The correct primitive: `(relationship_class × intimacy)`, with standing as an advisory demotion

The router and depth components **already exist** `[BUILT]`. Standing is reused only as a **soft, optional** demotion — explicitly NOT the floor.

| Component | Role | Where it lives | Status & honest scope |
|---|---|---|---|
| **`relationship_class`** | qualitative **router** — which facet family (colleague→work; sibling→family) | `human_relationships.relationship_type`; resolver `get_relationship_between` (bidirectional-aware) | `[BUILT]` — but consent-blind as shipped (C1); see new read below |
| **`intimacy`** | ordinal **depth** within each family | `human_relationships.intimacy_level` (4-rung: recognition / connection / trusted / intimate, `models.rs:623`) | `[BUILT]` |
| **`standing`** | per-evaluator **advisory demotion** + the no-edge **fallback** | `standing_view (evaluator, subject)` → `Standing::evaluate` | `[BUILT]` evaluator/score only — **no disclosure-ceiling API exists; cold-start = `Unknown`** |

### CRITICAL FIX (C1): the consent gate must be BUILT, not cited

The first draft said widening to trusted/intimate "requires a consented, bidirectional edge" and "a viewer cannot self-elevate" — then keyed the lens on `get_relationship_between`, which **ignores consent**, and on a write path that takes `consent_given_by_a/b`, `intimacy_level`, and both party ids **straight from caller input**. Confirmed against source: `get_relationship_between` (`:204`) has no consent filter; `get_trusted_contacts` (`:248–251`) is the only resolver that requires both consents; `create_human_relationship` (`:261`) writes consent flags from input with no counterparty authorization. The leak is real: Mallory inserts `{party_a: Mallory, party_b: Victim, family, intimate, consent_by_a: true, consent_by_b: false}`, the consent-blind read returns it, and Mallory is routed into Victim's family-intimate corridor. **The floor was asserted, not built.** Three changes make it real:

1. **`[NET-NEW]` Consent-filtered resolver `get_consented_relationship_between`.** Generalize the `get_trusted_contacts` predicate: return a relationship row to the lens **only if `consent_given_by_a == 1 AND consent_given_by_b == 1`** — the both-consents check is the security gate. (An additional blanket `is_bidirectional == 1` filter is **deferred** — confirmed at Slice-1 review 2026-06-22: `is_bidirectional` is a per-row annotation, not a type discriminator, and a blanket filter would wrongly hide legitimately one-way *consented* edges like mentor→mentee; gate on it only once relationship-type symmetry semantics are enumerated.) The lens calls THIS, never the raw `get_relationship_between`. A unilaterally-inserted, half-consented row is **invisible to the lens** — it cannot route a viewer into any corridor above the commons/recognition floor.
2. **`[NET-NEW]` Counterparty-consent write check in `create_human_relationship` / its service caller.** A caller may set `consent_given_by_<self>` only for the party they authenticate as; the counterparty's consent flag **cannot be set true by the initiator** — it flips true only via an explicit accept by the counterparty (a second authenticated call). This is the constitutional symmetry the draft claimed but never wired.
3. **`[DESIGN]` Lens consumes (1).** `effective_corridor/depth` derive only from consent-complete edges. No consent-complete edge → the viewer is a stranger → commons/recognition tier.

This is the headline fix. **Slice 2 cannot be called "enforcement" until (1) and (2) ship** — and (1) is the consent-correct read that Slice 1 actually needs (corrects L6: Slice 1 is "compose three reads," but ONE of those reads is `[NET-NEW]`, not pre-existing).

### HIGH FIX (H2): standing is an advisory demotion, NOT a clamp floor

The draft wrote `effective_tier = min(class∧intimacy_tier, standing_ceiling)` and called standing "the betrayed-intimate defense." Confirmed: **there is no disclosure-ceiling API** in `standing.rs` (only `evaluate`, `with_lift` which "never demotes," `cache_priority_weight`, `schemaref_depth_limit`), and `evaluate` returns `Unknown` at cold-start (`cache_priority_weight(Unknown) == 50`, neutral). So the clamp was invented logic dressed as reuse, and it is inert exactly against a fresh attacker. Corrected design:

- The **enforceable floor is consent + default-private + raw-CRUD hardening** (above + §4) — all deterministic and live on day one, independent of standing.
- Standing is a **`[NET-NEW]` advisory demotion**, honestly scoped: when `standing_view` has accumulated enough `FeedbackSignal` to return `Computed { Low/Floor }` for `(subject's-graph, viewer)`, the lens MAY narrow an already-consented edge (a known-bad but consented contact gets less). It is **opt-in narrowing, never widening**, and it is **explicitly inert at cold-start** (`Unknown` → no demotion → consent + intimacy govern alone). We do NOT present it as protection against a fresh betrayer; the protection against a non-consented viewer is the consent gate, which is deterministic.
- Implementing it requires a small `[NET-NEW]` `Standing::disclosure_demotion(self) -> Option<TierDelta>` helper — declared as net-new, not as reuse of `with_lift`.

### The cut-line function (corrected)

```
edge     = get_consented_relationship_between(viewer, subject)   // [NET-NEW] consent+bidi filtered
class    = edge.map(relationship_type)                            // routes facet corridor; None ⇒ stranger
intimacy = edge.map(intimacy_level)                               // depth in corridor (4-rung)

base_tier = match edge {
  Some => corridor_depth(class, intimacy),
  None => stranger_floor(viewer),     // recognized→public(T1); anon→commons(T0)
}
// standing is OPTIONAL, demote-only, inert on Unknown:
effective_tier = base_tier.saturating_sub( standing_demotion(subject_graph, viewer) )  // 0 unless Computed-Low/Floor

visible_facets = { f : f.sensitivity_label ≤ effective_tier  AND  f.class ∈ corridor(class) }
```

Most-restrictive-wins on the per-facet label: a facet labeled `intimate` cannot reach a viewer whose `effective_tier` is below it, even if a rule tried.

### MEDIUM FIX (M3): the intimacy→sensitivity mapping is a Slice-1 prerequisite, not a deferred open question

The draft deferred the mapping to "Open Q3." Correct: **Slice 1 cannot compute `DisclosureTier` without it**, so it is a Slice-1 prerequisite and is defined here, locally, now:

- The lens uses a **local 6-rung tier ladder** `{T0 commons, T1 public, T2 familiar, T3 trusted, T4 intimate, T5 self}` — it does **NOT** reuse the 8-rung reach enum as the tier axis (that is exactly the drift trap). Reach is the per-facet *label* only.
- The 4-rung `intimacy_level` maps onto the corridor depth: `recognition→T1`, `connection→T2`, `trusted→T3`, `intimate→T4`. `T0` is the no-edge anon floor; `T5` is `viewer == subject`.
- The per-facet sensitivity label maps from the reach vocabulary onto the SAME 6 rungs via an explicit local table (e.g. `commons→T0/T1`, `community→T2`, `trusted→T3`, `private→T4`, `intimate→T4`, `self→T5`); the two reach rungs the draft "silently dropped" (`community`, `private`) **are** in this table. (Corrected at Slice-1 review 2026-06-22: `intimate`→**T4** — the ladder's T4 *is* the intimate tier; an earlier draft's `self/intimate→T5` contradicted the ladder. In v1 the reach labels `private` and `intimate` both gate at T4; distinguishing them into separate rungs is a **Slice-5 refinement** once domain facets actually carry the `intimate` label.) This local mapping is self-contained and does **not** block on the global 3-vocabulary reach reconciliation (which remains a separate backlog item; confirming that independence is Open Question 3).

### Where the computation lives

A **pure fold in `elohim-facings`** (the DB-free fold crate; the exact pattern of `folds/contributor_reflexive.rs` `[BUILT, verified present]`), with a thin impure adapter in `elohim-storage/src/services/viewer_lens_facing.rs` `[NET-NEW]` doing three reads — `get_consented_relationship_between` `[NET-NEW]`, `Standing::evaluate` `[BUILT]`, `load_subject_facets` `[DESIGN]` (joined on `agent_cid`, never raw transport id) — then calling the pure fold.

HTTP surface: `GET /api/v1/identity/{subjectId}/profile` where **the viewer is the authenticated caller (from session), NEVER a `?viewer=` query param** — a query param is a raw-identity spoof surface. Anonymous callers resolve to the commons tier. The fold is **pure, idempotent, recomputed-never-stored** (§4).

---

## 4. Sacred floor vs commons discovery — and how visibility is ENFORCED

### The gradient as ordered tiers (commons → intimate)

```
COMMONS ◄──────────────────────────────────────────────► INTIMATE
  T0          T1          T2          T3          T4         T5
commons    public    familiar    trusted    intimate     self
(anon)   (recognized)(known)    (close)    (inner)   (the human)
  T0  name, brandable headline, public attestations, commons-reach contributions
  T1  + presence links, established-content citations, public affinities
  T2  + class-appropriate domain facets (lamad mastery summary, qahal memberships)
  T3  + recent activity, fuller domain composites, contact affordances
  T4  + class-GATED interior (work-T4 ≠ family-T4: same depth, disjoint corridors)
  T5  + full self-knowledge surface, audit trail, account-management (viewer == subject only)
```

### Enforcement is FOUR layered mechanisms, honestly tiered

**(a) Computed-never-stored fold.** The lens is a read-time fold. A stored `(viewer, subject) → facets` row would be the **O(viewers²) ACL table the p2p-design-gate exists to catch.** Materializing the lens is **forbidden.** One fold, one chokepoint.

**(b) Read-path = static envelope, per-viewer WALK.** Each facet's head-edge envelope (`Projection.preview_epr_ref`, `gate_hints`, `dead_end` `[BUILT]`) is **static-per-facet and cacheable**; cost paid at authorship/grant time. The **per-viewer part is the WALK** — one consent-filtered relationship lookup + an ordinal compare per facet — then full bodies for passing facets, cached head-edges for the rest. **A commons viewer walks only commons edges → zero governance compute → commons fast-path preserved.**

**(c) The raw-CRUD read is the actual leak path (`[NET-NEW]`).** A read-time fold is **theater** if `GET /db/humans/{id}` still serves gated fields. For the sacred tier, the raw row's reach-gated fields are served *only* behind the authenticated fold — raw `/db/humans/{id}` drops gated fields or requires viewer context. Plus the **default flip:** `humans.profile_reach DEFAULT 'public'` (`up.sql:252`) → `self`/`private` (grounded in the protocol's own `human_relationships.reach DEFAULT 'private'`). Identity is born private and promoted, never born public and walled.

**(d) The write-time gate (`[NET-NEW]`, service layer in v1).** Read-path filtering is one DB-direct write away from a leak. So two write-time guards, both service-layer in v1:
- The **counterparty-consent check** from §3 fix (1)+(2) — a viewer cannot self-elevate intimacy, because they cannot set the counterparty's consent flag, and the lens ignores any edge that isn't both-consented.
- A **`validate_facet_disclosure`** guard mirroring `validate_project_epr_commitment` Rule 1 — REJECTS any operation that widens a sacred-floor facet past its constitutional ceiling and writes an attributed disclosure record.

These together convert default-private from aspiration into an enforced floor — at **service-layer trust**, not cryptographic proof (see honesty constraint).

### Commons discovery preserved (requirement D)

Default-private costs the discovery attractor **nothing**, because the attractor lives on a *different address*: the commons band / unclaimed `ContributorPresence`, which accrues `recognition_score` / `unique_engagers` *while unclaimed* (recognition-before-registration) `[BUILT]`. A contributor is discovered as a public figure long before any sacred imagodei exists; claim later attaches a default-private interior behind that already-public face. The constitutional "no undesigned wall" (`validate_project_epr_commitment`) guarantees a gated facet always presents a designed commons-band face — there is **no private profile that 404s a stranger**; a stranger always gets the commons lens, the convergence point between page and profile.

### Honesty constraint (load-bearing)

`AgentPeerBinding` is **unsigned** `[BUILT-but-unsigned]`; the claim-verification validator is **stubbed** `[STUBBED]`; the tending-immune floor is **spec-asserted, not mishpat-notarized**. So v1 enforcement is **consent-filtered read + counterparty-consent write + default-flip + fail-closed projection + service-layer `validate_facet_disclosure`** — real and deterministic, but **doorway/storage-trust-rooted, not subject-cryptographically-proven.** We do **NOT** claim cryptographic proof that "this human runs this page" anywhere, and the slice demos (§7) are explicit about exercising the **consent gate's deny path**, not just the claim happy-path (corrects M4). The path to cryptographic enforcement is a named hardening wave (§6, Open Q1/Q2), not asserted as shipped.

---

## 5. P2P design-gate classification of every entity touched

*Identity is not the UUID; the UUID is just the edge handle between two keyed parties.*

| Entity | Class | Address strategy | Source of truth | Coordinator fn | Projection / signal | Route | Anti-pattern check |
|---|---|---|---|---|---|---|---|
| **`Human`** (sacred profile) `[BUILT]` | **B** (agent-scoped) — reused | agent key `uhCAk…` / `human-…` slug | DHT; `humans` table | existing `create_human` | `HumanView` | `/api/v1/identity/{id}` | ✓ identity = the key, not a body-CID; **CHANGE: default `profile_reach → self` `[NET-NEW]`** |
| **`ContributorPresence`** (org/persona page) `[BUILT]` | **B2** (agent-scoped + attestation; unsigned today) | slug `presence-…` | DHT; `contributor_presences` table | existing presence create + claim | `ContributorPresenceView` | `/db/presences/{id}`, `/org/{slug}` | ✓ slug justified (unclaimed = no key yet); ⚠ AgentPeerBinding UNSIGNED, claim validator STUBBED |
| **`HumanRelationship`** (viewer↔subject edge — the gate key) `[BUILT]` | **B2** (agent-scoped + attestation, `dht_anchor_hash`, consent flags) — reused | UUID row keyed by `(party_a, party_b)` | DHT-anchored; `human_relationships` table | `create_human_relationship` **+ counterparty-consent check `[NET-NEW]`** | `HumanRelationshipView` / `relationship-formed` | `/db/human-relationships` | ✓ identity is the keyed parties; UUID is only the edge handle. ⚠ consent enforced ONLY after the §3 fixes |
| **`get_consented_relationship_between`** (consent-filtered read) `[NET-NEW]` | (read predicate, no storage) | n/a | n/a | generalizes `get_trusted_contacts` predicate | n/a | internal (consumed by lens) | ✓ the fix for C1 — lens reads ONLY both-consented edges |
| **`standing_view`** (per-evaluator standing — advisory demotion + fallback) `[BUILT]` | **C** (operational projection) — reused | PK `(evaluator_pubkey, subject_pubkey)` | recomputed from FeedbackSignal subgraph (NOT truth) | `standing_projector` | folds FeedbackSignal | internal | ✓ NOT authoritative; pluralist; cold-start `Unknown`→neutral; demote-only, inert when Unknown |
| **`Standing::disclosure_demotion`** (advisory narrow) `[NET-NEW]` | (pure helper) | n/a | n/a | net-new (NOT `with_lift`) | n/a | internal | ✓ declared net-new; never widens; H2 fix |
| **ViewerLens fold** (the per-viewer aperture) `[NET-NEW]` | **C** (ephemeral, **recomputed-never-stored**) | none — computed per request | derived from the B2 + C inputs above | pure fold (read-only) | request-scoped, not gossiped | `GET /api/v1/identity/{id}/profile` (viewer = session) | ✓ NOT a stored per-viewer permission row (= O(viewers²) ACL anti-pattern); NOT content-addressed |
| **Facet sensitivity label + class** `[NET-NEW field, no new enum]` | **C** (manifest metadata; reuses reach vocabulary as label) | rides the facet | the subject (declared) | facet authoring | manifest codegen | within profile composite | ✓ NOT a new enum; local tier ladder distinct from reach axis; domains attach via app-manifest |
| **`validate_facet_disclosure`** (write gate) `[NET-NEW]` | **C** (service-layer validator in v1) | n/a | n/a | blocking validator on disclosure-widening | n/a | internal | ✓ mirrors `validate_project_epr_commitment` Rule 1; no new entry type in v1 (notarized = Open Q1) |
| **Page projection** (brandable landing) `[DESIGN]` | **C** (mutable body, NOT body-hashed) — reused pattern | `/epr/{id}` floor + routeClaim pretty mount | `project-epr` commitment; body in projection table | existing `project-epr` grant | `validate_project_epr_commitment` (the no-undesigned-wall face) | `/in/{name}`, `/org/{slug}` (302) | ✓ Cat-C mutable body cites immutable CIDs; passes no-undesigned-wall |
| **Route-claim / pretty handle** `[NET-NEW]` | **C** (doorway-scoped alias) — reused mechanism | `@name@doorway`, `/in/{name}` → canonical | doorway routeClaim grant table | existing route-claims grant | doorway 302 dispatch | resolver only | ✓ alias → ONE canonical; per-doorway uniqueness; not body-hashed, not global, not cryptographic |
| **qahal `PARTICIPATES_IN` HTTP projection** `[DESIGN]` | **A2** (derived via link; declared, not yet HTTP-projected) | link human→collective | DHT (qahal) | needs HTTP projection arm | new view over existing edge | `/db/relationships?type=PARTICIPATES_IN` | ⚠ sizing item — collective facets need this surfaced |
| *(Open Q1)* **`ConstitutionalFloor` + `FacetDisclosureEvent`** `[GATED]` | **A / A2** (notarized) | content-derived; append-only audit links from Human | notarized DNA entries | constitutional declaration; `facet-disclosure-widened` signal | observation projection | internal audit | ✓ check Mishpat headroom (~11/~100); the only gate-justifiable new entries |

**No new DHT entry type in v1.** Every notarized input (`Human`, `ContributorPresence`, `HumanRelationship`) already exists; all v1 net-new is Category C (the fold, facet metadata, the two service-layer write guards, the consent-filtered read, the standing helper). The notarized floor/ledger is a deferred, operator-gated hardening (§6, Open Q1).

---

## 6. Net-new vs reused

**Reused (compose, do NOT fork — the spine is built):**
- `Human` — change ONE default.
- `ContributorPresence` + claim flow + recognition accrual + `presenceType: organization` — page substrate AND recognition-before-registration.
- `steward_id` / stewardship relationship — the **administer** edge.
- `human_relationships` + `intimacy_levels` (4-rung) — the viewer→subject **gate key** (read+write fixed below).
- `get_trusted_contacts` predicate — generalized into the consent-filtered read.
- `standing_view` + `Standing::evaluate` — per-evaluator **advisory demotion + fallback** (NOT a clamp floor).
- `reach` vocabulary + `openness()` — per-facet **sensitivity label** only.
- `Projection` head-edge envelope + `validate_project_epr_commitment` — static cacheable face + no-undesigned-wall floor + write-validator pattern to mirror.
- Route-claims + `/epr/{id}` floor + commons fast-path.
- `elohim-facings` + `folds/contributor_reflexive.rs` — the fold home and pattern (verified present).

**Net-new in v1 (all Category C — derived/config/validator, no new notarized substrate):**
1. **`get_consented_relationship_between`** — consent+bidirectional-filtered read (C1 fix; what the lens actually consumes).
2. **Counterparty-consent write check** in `create_human_relationship` / its caller (C1 fix — a viewer cannot set the other party's consent flag).
3. **ViewerLens fold** (`elohim-facings/src/viewer_lens.rs` + `services/viewer_lens_facing.rs` adapter) — computed-never-stored.
4. **`DisclosureTier::compute`** — relationship-class router + intimacy depth, with the local 6-rung ladder and intimacy→tier + reach→label mappings (M3 fix).
5. **`Standing::disclosure_demotion`** — net-new advisory demote-only helper (H2 fix), inert at cold-start.
6. **Facet sensitivity label + class** on imagodei core's sections + domain manifests (reuse reach vocabulary; no new enum).
7. **`GET /api/v1/identity/{id}/profile`** — viewer = authenticated session.
8. **Service-layer `validate_facet_disclosure`** write-gate.
9. **Raw-CRUD hardening:** `/db/humans/{id}` drops gated fields without viewer context.
10. **`profile_reach` default flip** migration.
11. **Pretty-handle minting UX** over the existing routeClaim mechanism.
12. **`PARTICIPATES_IN` HTTP projection.**

**Net-new but operator-gated (deferred hardening wave — Open Q1/Q2):**
- Promote `validate_facet_disclosure`'s floor to a **notarized `ConstitutionalFloor` (A)** + **`FacetDisclosureEvent` (A2)** audit ledger.
- **Subject-signed B2 facet gate-rule** — converts doorway-enforced to subject-cryptographically-enforced.
- Sign the relationship edge / the `AgentPeerBinding` (closes the unsigned gap; makes claim-attribution cryptographic).

---

## 7. Sizing into bounded slices

Constraint: the cheapest slice mints zero DNA entries and is frontend-startable. Do not front-load the notarized floor. **Slices 1+2 ship together** — Slice 1 is theater without Slice 2, and Slice 1's relationship read is the `[NET-NEW]` consent-filtered one, not a pre-existing read (L6 correction).

- **Slice 1 — The fold on the commons floor (no entries).** `ViewerLens::resolve` in `elohim-facings`; `DisclosureTier::compute` with the local ladder + intimacy→tier + reach→label mappings (M3 prerequisite resolved); standing stubbed `Unknown`→no-demotion; commons-only walk for anon/stranger. Includes the `[NET-NEW]` `get_consented_relationship_between` read (Slice 1 is NOT purely pre-existing reads). Lights page = commons subgraph; recognition-before-registration works; commons fast-path verified. a2o: *"a sibling and a colleague see different facets of the same human; an anon sees only the commons face."* (~M)
- **Slice 2 — Sacred-tier enforcement + the consent gate (the required risk-closer).** Flip `profile_reach` default; raw `/db/humans/{id}` drops gated fields without viewer context; fold becomes the only authenticated path to gated facets; **counterparty-consent write check** + service-layer `validate_facet_disclosure`. a2o (deny-path explicit): *"Mallory inserts a half-consented family-intimate edge naming the victim → the lens shows Mallory ONLY the commons face (the consent gate denies); a viewer cannot self-elevate intimacy."* (~M)
- **Slice 3 — Standing advisory demotion.** Wire `Standing::disclosure_demotion` as demote-only on already-consented edges; honestly inert on `Unknown`. a2o: *"a consented contact whose per-evaluator standing is Floor is narrowed below their intimacy tier; a cold-start contact is NOT narrowed."* (~M, gated on Phase 3.5 `standing_view` population)
- **Slice 4 — Page surface + pretty handle + claim-vs-administer.** `presence`/`page` renderer mapping (fills the documented renderer gap); `/in/{name}` + `@name@doorway` routeClaim resolver; "claim this presence" affordance; administer-an-org-page via stewardship. a2o: *"create a branded public page without seeing a CID; claiming a page does not expose the claimant's private profile; one human administers two org pages without identity collapse."* (~M)
- **Slice 5 — Domain composites + qahal projection.** Manifest-declared profile surfaces (lamad/shefa/qahal); project `PARTICIPATES_IN` to HTTP. (~M, the only DHT-adjacent work)
- **Slice 6 (operator-gated hardening) — Notarized floor + signed gate-rule.** Promote `validate_facet_disclosure`'s floor to notarized `ConstitutionalFloor` + `FacetDisclosureEvent`; sign the relationship edge / facet gate-rule / `AgentPeerBinding`. Converts service-layer-enforced to subject-cryptographically-enforced. (~L, **only if Open Q1/Q2 say go**)

Each slice is independently demonstrable via `pnpm look` against a claimed presence on alpha — and the slice that exercises the gate (Slice 2) demos the **deny path**, not the claim happy-path.

---

## Open questions for the operator

1. **Constitutional floor: service-layer (v1) vs notarized DNA entries.** The synthesis ships the write-time guards (consent check + `validate_facet_disclosure`) at the service layer in v1 and presents the notarized `ConstitutionalFloor` (A) + `FacetDisclosureEvent` (A2) as Slice 6. **Decision: how constitutional must the floor be in v1, and is the Mishpat-headroom spend (~11/~100) justified now?**

2. **Cryptographic anchoring now or later (collapsible with Q1).** Subject-signed facet gate-rule + signing the relationship edge / `AgentPeerBinding` would convert doorway/storage-trust enforcement into subject-cryptographic enforcement — but `AgentPeerBinding` is unsigned and the claim validator is stubbed today. **Harden the unsigned substrate in this arc, or accept honest service-layer enforcement for v1 and harden later?** (Synthesis assumes the latter and says so plainly in §4.)

3. **Local `intimacy→tier` / `reach→label` mapping vs the 3-vocabulary reach drift.** The fold defines a *local* mapping (§3, M3 fix) and does not block on the global reach-vocabulary reconciliation. **Confirm the local mapping may land without blocking on the global reach-drift reconciliation** (synthesis assumes yes; this is now a Slice-1 prerequisite, not a deferred nicety).

4. **`profile_reach` default flip (`public` → `self`/`private`) — migration posture.** The protocol's own `human_relationships.reach DEFAULT 'private'` supports the flip, but it has live-data implications (existing default-public humans become default-private). **Confirm the flip and the posture for existing rows (flip-all vs grandfather-existing).**

5. **Claim-vs-administer surfacing in the UI.** Claim (identity-merge, terminal) and administer (stewardship, separable) are kept distinct. **Confirm both belong in v1 account-management, or whether administer-an-org-page is a follow-on after the personal-imagodei lens lands.**

---

## Adversarial review — resolved / rebutted

**C1 (CRITICAL) — consent gate cited but not enforced; concrete intimate-facet leak. → FIXED (design changed).** Verified against source: `get_relationship_between:204` returns all matching rows with no consent filter; `get_trusted_contacts:248–251` is the only resolver requiring both consents; `create_human_relationship:261` writes consent flags + party ids + intimacy straight from input. The leak (Mallory's half-consented family-intimate row) is real. **Fix:** the lens now consumes a new `get_consented_relationship_between` (both-consents-required, generalizing the `get_trusted_contacts` predicate), and `create_human_relationship` gains a counterparty-consent write check (a caller can set only their own consent flag; the counterparty's flips true only via a second authenticated accept). The floor is now built, not asserted. Slice 2 explicitly demos the deny path. §3 fix (1)+(2), §4(c)(d), §5, §6, §7.

**H2 (HIGH) — standing "betrayed-intimate clamp" invented (no ceiling API) and cold-start-inert. → FIXED (design changed; claim demoted).** Verified: `standing.rs` has no disclosure-ceiling API (`with_lift` "never demotes," `evaluate`→`Unknown` at cold-start, `cache_priority_weight(Unknown)==50`). **Fix:** standing is demoted from a load-bearing "clamp floor" to a `[NET-NEW]`, opt-in, **demote-only** advisory (`Standing::disclosure_demotion`, declared net-new — NOT reuse of `with_lift`), **explicitly inert at cold-start**. The enforceable floor is now consent + default-private + raw-CRUD hardening (deterministic, day-one), not standing. We no longer claim it defends against a fresh betrayer. §3 H2 fix, §5, §7 Slice 3.

**M3 (MEDIUM) — intimacy→reach-label mapping undefined; 2 reach levels dropped; mislabeled as a later open question. → FIXED (promoted to Slice-1 prerequisite).** **Fix:** §3 now defines a self-contained **6-rung local tier ladder** (distinct from the 8-rung reach axis — using reach as the tier axis was the drift trap), an explicit `intimacy_level(4-rung)→tier` mapping, and a `reach-label→tier` table that **includes** `community` and `private` (the two the draft dropped). Resolved locally without blocking on the global reach reconciliation (now Open Q3, reframed as confirm-independence, not defer-the-mapping).

**M4 (MEDIUM) — claim attribution uncryptographic (acknowledged), but slice demo leans on the stubbed validator. → FIXED (honesty + demo scope).** **Fix:** §4 honesty constraint now states claim verification is `[STUBBED]` and `AgentPeerBinding` `[unsigned]`; §7 makes the gate-bearing slice (Slice 2) demo the **consent deny path**, not the claim happy-path. Cryptographic claim-attribution is Slice 6 / Open Q2, not asserted as shipped.

**M5 (MEDIUM) — Surfaces 2 and 3 lack canonical + human-friendly addresses; part-(e) coherence holds only for the social surface. → FIXED (scope stated honestly).** **Fix:** §2 now states explicitly that v1 delivers human-friendly addressing for **Surface 1 (social)** only; **Surface 3 (account-management)** is capability-addressed (auth-gated `/account` via doorway recovery OR steward device — deliberately not publicly addressable); **Surface 2 (self-knowledge)** is deferred by the canonical surfaces spec and gets no v1 address. No coherence claim beyond the social surface.

**L6 (LOW) — "first slice over existing reads" oversells; the consent-correct relationship read is net-new. → FIXED.** **Fix:** §3 and §7 Slice 1 now state plainly that one of the three reads (`get_consented_relationship_between`) is `[NET-NEW]`; Slice 1 is "two existing reads + one net-new consent-filtered read + a pure fold," not "purely existing reads."

**Verifier items that were already sound (no change, acknowledged):** (a) p2p-design-gate PASSES — no content-address forced on the mutable page, O(viewers²) ACL explicitly forbidden, `agent_cid` not raw id, lens correctly Category C, no new DHT entry type in v1; (c) composing-vs-forking — genuinely reuses the spine (the one fork-as-reuse, the standing clamp, is corrected under H2); (d) reach-vs-relationship — the proof that reach has no `(viewer,subject)` term is correct and adopted; (f) bounded slices — Slice 2 correctly the non-optional risk-closer. **Self-correction acknowledged:** the verifier's withdrawn first-pass reach-ordinal-inversion flag was correctly withdrawn (the ladder is monotonic); the residual M3 (dropped levels / undefined mapping) is what stood and is now fixed.

**Key files:** `/projects/elohim/elohim/elohim-storage/src/db/human_relationships.rs` (`get_relationship_between:204` consent-blind read; `get_trusted_contacts:248–251` the both-consents predicate to generalize; `create_human_relationship:261` consent-from-input write — the C1 fix sites), `/projects/elohim/elohim/elohim-storage/src/services/standing.rs` (`evaluate:87` cold-start `Unknown`; `with_lift:56` "never demotes"; no disclosure-ceiling API — the H2 fix site), `/projects/elohim/elohim/epr/src/reach.rs` (`openness:41` monotonic 1–8, no viewer term — reach is the sensitivity label only), `/projects/elohim/elohim/elohim-storage/src/db/models.rs:623` (`intimacy_levels` 4-rung), `/projects/elohim/elohim/elohim-facings/src/folds/contributor_reflexive.rs` (the pure-fold pattern the lens copies — confirmed present), `/projects/elohim/elohim/elohim-storage/src/db/rea_commitments.rs:1089` (`validate_project_epr_commitment` — no-undesigned-wall floor + write-validator pattern to mirror), `/projects/elohim/elohim/elohim-storage/migrations/2026-01-08-000000_initial/up.sql:252` (`profile_reach DEFAULT 'public'` to flip), `/projects/elohim/elohim/elohim-views/src/imagodei.rs` (Human/ContributorPresence views), `/projects/elohim/genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md` (three-surface canon), `/projects/elohim/genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md` (route-claims / pretty-mount / no-undesigned-wall mechanism).
