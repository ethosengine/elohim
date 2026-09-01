---
title: Contributor-Visual Surfaces — presences-on-EPR (done) + imagodei profile/page via the viewer-lens arc
id: contributor-visual-surfaces-plan
status: Draft
class: protocol-canonical
domain: D2
topic: [contributor-presence, imagodei, profile, epr, frontend, attribution, who-is-who, facings]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
cites:
  - "contributor-presence-bootstrap-whoswho-design | the Wave 1-2 spec these two surfaces consume + render; this plan refines its visual layer | sha256:08d11210fd816f68 | path: genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md"
  - "resilience-facings-select-fold-aggregate-design | the §11 facings framework the reflexive aggregator (surface B feed) is a child of | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md"
  - "imagodei-surfaces | CANONICAL three-surface identity decomposition (social/self-knowledge/account-mgmt); Sprint 2 builds Surface 1 + a thin Surface 3 touch | path: genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md"
  - "epr-route-claims-link-conformance-design | the addressing mechanism the pretty-handle reuses (universal /epr/{id} floor + steward-granted routeClaims + doorway dispatch + claimed/unclaimed classifier) | sha256:1d9969399472335d | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md"
  - "imagodei-profile-page-viewer-lens-design | the design + slices Sprint 2 now executes (viewer-relative lens, profile/page split, in-arc legs 1+2 hardening) | path: genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md"
refines:
  - genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
  - genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md
requires_env: [household-nodes]
---

# Contributor-Visual Surfaces — presences-on-EPR + imagodei profile

The two visual surfaces that consume the contributor-presence substrate (Waves 1–2 of
`2026-06-21-contributor-presence-bootstrap-whoswho-design.md`).

- **Sprint 1 (presences-on-EPR) — DONE.** Pure frontend rendering + wiring on data that was ~90%
  ready: no new substrate, no new DHT entry type. Shipped on `feat/frontend-eyes-sprint`.
- **Sprint 2 (imagodei profile/page) — RE-PLANNED onto the viewer-lens architecture** (operator
  decision 2026-06-22). This is **no longer a pure-frontend sprint**: it introduces the viewer-relative
  disclosure lens, a consent gate, a default-private flip, and (legs 1+2) a notarized disclosure floor.
  The design + slices live in `2026-06-22-imagodei-profile-page-viewer-lens-design.md`; this plan is the
  execution view. Grounded against the live frontend 2026-06-22 (eyes-first; the deployed-alpha read
  plane was 404-ing all reads at grounding time — see Env caveats).

## Surfaces
- **A — Contributor presences ON an EPR**: a "Contributors / Inspired by" panel on the content/EPR
  viewer — who inspired/contributed to this artifact.
- **B — Imagodei profile** (by id), two cases: **(b1)** a network-CLAIMED human/agent's profile;
  **(b2)** an UNCLAIMED contributor presence's profile (accrued recognition via the reflexive feed +
  a claim affordance). The claimed/unclaimed *standing gradient* maps onto these two states.

## What's already wired (reuse, don't rebuild)
- Content viewer (`app/lamad/src/app/components/content-viewer/`) — tabbed shell + EPR-relationships
  panel + stewardship cards. Presence LIST (`app/elohim-app/src/app/imagodei/.../presence-list`) with
  recognition stats + state badges + "Become Steward".
- Self-profile (`/identity/profile`, `ProfileComponent`) — 9 reusable `profile/sections/*` components.
- `PresenceApiService` (`initiateClaim`/`verifyClaim`/`getPresenceById` already written, **unused**),
  `ContributorApiService` (`dashboard`/`impact`/`recognition`, **unused**), `ContributorPresenceView`
  (consumed), `getPresenceForContent` (`storage-api.service.ts:419`, client-side establishingContentIds filter).
- The reflexive aggregator `GET /api/v1/contributors/{id}/reflexive` → `ContributorReflexiveView` (the
  b2 "how the network sees them" feed) — endpoint committed (`1e7bc2d89`), SDK type generated +
  **now barrel-exported** (this session). ⚠ committed on `feat/frontend-eyes-sprint` + locally-merged,
  **NOT on `origin/dev` / not deployed** — gated remote integration is operator-owned.

## Addressing + viewer-lens model — see the design spec

The foundational addressing question ("how is the imagodei profile addressed — CID-addressable
public/private, pretty handles routing through doorways") and the viewer-relative disclosure lens are
**designed in `2026-06-22-imagodei-profile-page-viewer-lens-design.md`** (this plan executes it). The
load-bearing decisions, in one breath:

- **A profile is one viewer-relative disclosure graph.** The "page" is its commons band (public,
  brandable); the "sacred profile" is the same graph walked deeper. Seam = the `commons`/non-`commons`
  line `validate_project_epr_commitment` already enforces — no new addressing scheme.
- **"CID-addressable" disentangled**: canonical identity = agent key (`uhCAk…`) / slug — **not** a content
  CID; pretty handle = doorway-scoped routeClaim alias (Mastodon `@name@doorway` model); the **mutable
  profile page is a Category-C projection** (never body-hashed); only the immutable content it *cites* is
  `/epr/{cid}`.
- **The lens primitive is `relationship_class × intimacy`, NOT reach** (reach is content-scoped, no
  viewer term; demoted to a per-facet sensitivity label). Standing is an advisory, demote-only narrowing.
- **Profile vs page = "is this YOU or an entity you run?"**: your public face = the commons band of your
  own `Human`; an org/persona = a separate `ContributorPresence`, bound by CLAIM ("it IS me") or
  ADMINISTER ("I run it" — stewardship). Zero new entity.
- **Sacred floor hardened in-arc** (decision 2026-06-22): consent gate + default-private + leg-1 notarized
  `ConstitutionalFloor`/`FacetDisclosureEvent` + leg-2 un-stubbed claim verification. Leg 3 (cross-signed
  `AgentPeerBinding`) is **severed** to the security-backlog prerequisite — off the critical path (it gates
  only proven economic attribution; the reflexive feed renders honestly-caveated meanwhile).

The full P2P design-gate classification of every entity is in the spec's §5.

## Sprint 1 — Contributor presences on an EPR
- [x] Expose the content→presences edge over HTTP: wire the dead DB reverse-query
      `get_presences_for_content` (`elohim/elohim-storage/src/db/contributor_presences.rs:183`) to a
      route (`GET /api/v1/content/{id}/presences` or `/db/presences?establishingContent={id}`) +
      doorway passthrough. (MVP fallback: the client-side `getPresenceForContent` filter.)
- [x] graphos contributor/credit card primitive (Library A default + Library B designed) — presence
      display-name, image, recognition, claimed/unclaimed badge; links to the profile (Surface B).
      None exists today.
- [x] Inject the presence fetch into `ContentViewerComponent` (injects no presence service today) +
      render a "Contributors / Inspired by" section.
- [x] Label `DERIVED_FROM` / `source_of` in the EPR-relationships panel (defined `RelationshipType`
      but **unlabeled** — `epr-relationships-panel.component.ts:18`).
- [x] a2o scenario: "see who inspired/contributed to this content."
- [ ] Verify rendered (local stack or healthy alpha).

## Sprint 2 — Imagodei profile/page via the viewer-lens arc

Re-planned 2026-06-22 onto `2026-06-22-imagodei-profile-page-viewer-lens-design.md` (the design home;
its §7 slices + 14 gap-items are the fine grain — what follows is the sprint-level sequence). The arc
renders Surface 1 (social) in both states (claimed human / unclaimed presence) as the *commons band* of
one viewer-relative disclosure graph, hardens the sacred interior in-arc (legs 1+2), and renders the
profile/page on top. Rust/storage-heavy — **no longer a pure-frontend sprint.**

**Sequencing principle: harden in-arc ≠ harden first.** The C1 leak is closed by the *service-layer*
consent fix (consent-filtered read + counterparty-consent write), **not** by DNA notarization; and §4's
enforceable floor is consent + default-private + raw-CRUD, which leg-1 *upgrades* but does not gate. So
the **visible landing lands early on the service-layer floor**, and leg-1 DNA notarization — the heaviest,
most operator-gated class of change in the repo (DNA-reinstall gating, Mishpat pipeline, sweettests) —
upgrades enforcement *underneath an already-rendered surface*. **Order: 1 → 2′ → 4 → leg-1 → 3 → 5.**

**First SDD-runnable chunk = Slices 1 + 2′** (the lens fold + the *service-layer* risk-closer — NO DNA).
The visible profile/page surface (Slice 4) lands immediately after.

- [ ] **Slice 1 — Lens fold + read route.** `ViewerLens::resolve` + `DisclosureTier::compute` (local
      6-rung ladder; intimacy→tier + reach→label maps) in `elohim-facings`; the **net-new**
      `get_consented_relationship_between` read; the profile read route (viewer = authenticated session);
      standing stubbed `Unknown`→no-demotion; commons-only walk for anon/stranger. Lights page = commons
      subgraph; commons fast-path verified. (spec §7 Slice 1; gap-items #1, #3, #4, #7)
- [ ] **Slice 2′ — Service-layer sacred enforcement + consent gate (risk-closer, NO DNA; ships with Slice 1).**
      The full C1 fix: the **net-new** counterparty-consent write check; `profile_reach` default flip
      (public→self/private); raw `/db/humans/{id}` drops gated fields without viewer context; the fold
      becomes the only authed path to gated facets; the service-layer `validate_facet_disclosure` write-gate.
      a2o (deny-path): "Mallory's half-consented intimate edge → the lens shows her only the commons face."
      (spec §7 Slice 2 [service-layer part]; gap-items #2, #8, #9, #10)
- [ ] **Slice 4 — Page surface + pretty handle + claim-vs-administer (THE VISIBLE LANDING; pulled forward).**
      `presence`/`page` renderer mapping; the `/in/{name}` + `@name@doorway` routeClaim resolver
      (`IdentityResolver` hides the CID complexity — the human-friendly frontend); "claim this presence"
      affordance + **leg 2 — un-stub claim verification** (email / dns-txt control); administer-an-org-page
      via stewardship; the pillar-internal SPA routes (`/identity/profile/:id`, `presences/:id`) + the
      reflexive feed render (honestly-caveated per leg-3 severance — "observed, not proven"). a2o: "create a
      branded page without seeing a CID; claiming a page doesn't expose the claimant's private profile; one
      human administers two org pages without identity collapse." (spec §7 Slice 4; gap-items #6, #11, #14)
- [ ] **Leg-1 slice — Notarize the disclosure floor (in-arc DNA; upgrades underneath the visible surface).**
      Notarize `ConstitutionalFloor` (A) + `FacetDisclosureEvent` (A2) as Mishpat DNA entries (~11/100
      headroom) so the floor is DNA-validation-enforced, not service-trust. Heaviest/most operator-gated
      class — sequenced AFTER the surface renders, so a DNA-reinstall stall never blocks the landing.
      (spec §7 Slice 6 leg-1; gap-item #13)
- [ ] **Slice 3 — Standing advisory demotion.** Wire `Standing::disclosure_demotion` (demote-only, inert
      at cold-start) on already-consented edges. (spec §7 Slice 3; gap-item #5)
- [ ] **Slice 5 — Domain composites + qahal projection.** Manifest-declared profile surfaces
      (lamad/shefa/qahal); project `PARTICIPATES_IN` to HTTP. (spec §7 Slice 5; gap-item #12)
- [ ] Verify each slice rendered (`pnpm look` against a claimed presence — local stack or healthy alpha;
      deployed alpha was 404-ing all reads at grounding; the gate-bearing Slice 2′ demos the **deny path**,
      not the claim happy-path).

**Forks to resolve at slice start** (spec Open Q3/Q4/Q5): the local reach→tier mapping lands without
blocking on the global reach reconciliation (assume yes); the `profile_reach` flip posture (flip-all vs
grandfather-existing); whether administer-an-org-page is v1 or a follow-on.

## Sizing
- **Sprint 1 (presence-on-EPR)** — DONE. Reused the viewer shell, relationships panel,
  `ContributorPresenceView`, `get_presences_for_content` (DB); net-new = the HTTP route, the graphos
  credit card, the viewer section, `DERIVED_FROM` labeling. (Medium.)
- **Sprint 2 (viewer-lens arc)** — the full reuse-vs-net-new breakdown is the spec's §6 (reuses `Human`,
  `ContributorPresence` + claim flow, `human_relationships` + `intimacy_levels`, `standing_view`, the
  `reach` vocabulary as a label, the route-claims/`/epr/{id}` floor, `elohim-facings`; net-new = the 12
  Category-C items — the fold, the consent-filtered read + counterparty-consent write, the disclosure
  tier, the standing demotion, facet labels, the profile route, the write-gate, raw-CRUD hardening, the
  `profile_reach` flip, the handle-minting UX, the `PARTICIPATES_IN` projection — plus legs 1+2). Larger
  than Sprint 1 and Rust/DNA-heavy; sequence Slices 1+2 first, protect Slices 4–5 if it overflows.

## Pre-reqs / caveats
- **SDK barrel** — `ContributorReflexiveView` + `RecognitionByAction` are now barrel-exported (done
  this session). ⚠ The `src/generated/index.ts` auto-generator is **drifted**: `export_bindings`
  rewrites type files but does NOT regenerate the barrel, and `generate-types.sh` emits the wrong
  format (single-quote / `models.rs` header vs the live double-quote / `views.rs` header). The barrel
  was synced by minimal additive insertion; **the generator drift is a separate fix** (find/repair
  the real index.ts generator).
- **App-dir codegen deferred** — `INTERFACE_FILES` (`codegen-ts.mjs`) + `pnpm schema:codegen:ts` to
  push the view's TS to the 5 app dirs (the SDK type + endpoint are the verified Wave-2 deliverable).
- **Env (verification)** — deployed `doorway-alpha.elohim.host` was returning 404 for ALL data reads
  at grounding time (conductor "use WebSocket /admin" 404; "0 content available") — alpha's read
  plane regressed since the Wave-1 seed verified 200. Verify these surfaces on a **local stack or a
  healthy alpha**, not this host as-is.
- **Doorway proxy** — `/api/v1` reads proxy with `X-Agent-Cid: matthew` (no presence allowlist);
  `/db/presences` is the presence front door. Confirm the new content→presences route is proxied.

## Out of scope (named, deferred)
- **Leg 3 — cross-signed `AgentPeerBinding`** (libp2p keypair ↔ Holochain agent key). SEVERED from this
  arc (operator decision 2026-06-22): the real fix is an *unbuilt* cross-signed control proof a prior
  red-team shelved (`2026-06-15-coherent-transport-identity-resolver-design.md` §0). Off the critical
  path — gates only *cryptographically-proven* economic attribution; the reflexive feed renders
  honestly-caveated ("observed, not proven") without it. Captured as a security-backlog prerequisite.
- Wave 3 (opt-out→commons) — DEFERRED in the bootstrap spec (DNA-notarized/heavy).
- The partial-commons + fractal-steward reflexive folds (no data substrate yet — `CoverageRollup` is
  the future recursion path).
- The manifesto-as-ContentNode establishing edge (the on-grain allocation link).

> Note: the pretty-handle-through-doorway arc, *deferred* in the pre-re-plan version, is now **in scope
> as Slice 4** (the `IdentityResolver` + routeClaim resolver) — it is the human-friendly addressing the
> operator asked for.
