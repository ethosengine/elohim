# HANDOFF — Brainstorm: canonical EPR-app routing (Slice 3 routeClaims + the bridge-vs-debt question)

_Last updated: 2026-06-06 · Author: Claude Opus · Branch: `dev` (clean tree) · Session mode: **orchestrating** — this hands off into a `/brainstorm` session, not an implementation checklist._

_The previous handoff (2026-06-05, shift/a2o-greenup) is RESOLVED: both deliverables (household-formation stage 1 `89d86805c..`, omnibar-consolidation `..f0b134a89`) verified merged into dev's **pushed** history. Still-pending carry-overs from it are listed at the bottom._

**Branch state (verified 2026-06-06):** `dev` is **31 commits ahead of `origin/dev`, all local-only** — 25 from the §12.6 Slice 2 branch (`dd98da925..174c98a8e`, ff-merged, feature branch deleted) + 6 prior local commits (`1fbd130fa..2dfeb7d3a`). **Integrator owns the push** (it triggers the orchestrator; the changeset spans app + doorway + sdk fixtures + genesis). Post-push, the three new `@browser-only` a2o routing scenarios get their first real run on alpha — the one verification the dev container can't perform.

---

## Goal

Run a **brainstorming session (with the mandatory `p2p-design-gate`)** that elevates EPR-app routing from "Slice 2 landed + Slice 3 sketched in one paragraph" into a **canonical spec**, answering three entangled questions:

1. **Canonical bridge-work vs tech debt.** Slice 2 shipped a "transitional kindness": `LegacyResourceRedirectComponent` bridges monolith-era `/lamad/resource/{id}` shares to `/epr/{id}` — a client-side, un-notarized, per-route redirect. The spec's only sanctioned migration tool is `redirects_from` on the `project-epr` Commitment (mount-level moves, DHT-notarized; §12.4 explicitly rejects "redirect-heal hacks"). The open question: **is route-level redirect/aliasing real doorway bridge-work that deserves substrate backing** — read through the EPR story + value + governance pillars (who committed to keep an old address alive, for whom, at what cost, revocable how, visible to whom?) — **or debt with a retirement date?** Today it's neither: it works, but it has no story.

2. **routeClaims as a steward-governed routing substrate.** §12.3 sketches claims in ONE paragraph (`[{contentType: 'path', template: 'path/{id}', fragments: {step: 'path/{id}/step/{n}'}}]` riding the bundle EPR "like §3 element manifests"). Un-designed: the manifest schema, versioning, **claim-conflict resolution between bundles** (two bundles claim `contentType: 'path'` on one doorway — first-wins like §2 path conflicts? operator-curated? commitment-ranked?), **claim trust/authority** (who MAY claim a contentType — is a claim itself a commitment? an attestation?), and **how the doorway consumes claims at dispatch time** (it loads projections from `/db/rea_commitments?action=project-epr` at boot + SSE refresh; claims ride bundle-EPR manifest *content* — what projection surface delivers them?). §12.8 already says routeClaims re-enters the p2p-design-gate as manifest content.

3. **Provability — "the `<a>` tag of the elohim protocol, applied everywhere."** The operator's bar: ISO-9000 / W3C-conformance-level confidence that **when we write a link, we KNOW it works** — no dead `<a>` tags, no dead `<elohim-epr-link>`s, anywhere, ever. The docs corpus already achieves this with the semantic-links system (content-addressed cites: slug + fingerprint + `path:` locator, `DEAD-CITE`/`HELD-CITE`/`stale` statuses, `cite-gen --verify` as a dissolution gate, `cite-propagate` as the corpus sweep). **The runtime web surface deserves the isomorphic discipline.** The proof has layers to design:
   - **By construction** (the deepest layer): `/epr/{id}` + the §6 gate experience means a well-formed link CANNOT dead-end — unreachable resolves to a designed boundary, never a wall or raw 404. "No dead links" becomes a property of the resolver, not a property of authors' diligence.
   - **Static** (authoring/CI): links are MINTED, never hand-written — `eprToRoute`/claims is the only legal source; a lint/CI gate enforces "no raw route literals" (the Slice-2 grep audits — `'/resource'`, `'/content'`, `TODO(#12-6)` — done by hand this branch, should become a permanent gate, like the runbook's router-literal canary). Referenced EPR ids checkable against seed/content at build time (cite-gen's fingerprint discipline, applied to code-minted links).
   - **Contract** (drift guards): claims vocabulary shared between client and doorway with a two-layer fixture (the spa-route-discrimination pattern); testid-sync for link elements.
   - **Conformance** (continuous, W3C-style): a normative spec with MUST/SHOULD conformance classes + a crawler/a2o suite that walks every rendered anchor and asserts it lands on a rendered surface or a designed boundary (render-verified, never HTML-shell). ServingContext provenance makes "which projection served this link" auditable.

**The vision target (operator's words):** EPR-app routing becomes *"a steward-governed, multi-federated, DHT-notarized, sensemaking system over nodes in the peer-native data layer"* — graph-aware (route shape from the EPR head's place in the graph), reach-aware (resolution composes with reach gates — never a wall, always a designed boundary), p2p-aware (any doorway that knows the contract can resolve; §12.7's federated anycast + toll access via `delegates-compute` commitments). Routing decisions become legible, governed artifacts — not config. And every link the protocol mints is *provably* alive — the docs corpus's content-addressed link integrity, extended to the running web.

## Current Progress (verified in-repo)

**Landed (Slice 1, 2026-06-04, `edc907c43`):** doorway `spa_fallback` + storage safety-net fallback + lamad de-literalization.

**Landed (Slice 2, 2026-06-06, `bcf1b7a28..174c98a8e` on dev):**
- `/epr/{id}` universal address: doorway reserves `/epr` (`is_service_path` + `is_reserved_url_path` + ingestion warn-skip in `epr_router.rs::replace_all`); `dispatch_epr_universal` serves the root projection's bundle; shell route `epr/:resourceId` renders the cross-pillar viewer.
- `eprToRoute(ref, ctx, contentType?)` rewritten around **client-side** `BundleRouteContext` claims (`RouteClaim { contentType, commands(ref) }`; shell provides `{claims: [], ownsUniversalRoute: true}`; lamad claims only `'path'`). The type-vs-slug heuristic is dead; the contract is `commands: string[] | null` + an always-present universal `href`.
- Full distributed link sweep (lamad templates/programmatic/guard/resolver-consumers; Lit navigator de-literalized to host-supplied routes; Angular navigator straggler; SEO canonicals base-aware + `/epr` content canonicals; journal stub de-literalized; portal interceptor) + `EprNavService` hardening (pathless-layout-root `ownsPath` descent; origin-relative sink guard).
- The bridge under adjudication: `app/lamad/src/app/components/legacy-resource-redirect/legacy-resource-redirect.component.ts` (replaced the self-looping absolute `redirectTo`).
- Coverage: `genesis/a2o/features/lamad/deep-link-delivery.feature` now 7 scenarios (universal address renders · View-Resource-Details crosses the boundary · monolith-era share traverses the bridge `@regression`).

**NOT built (Slice 3, deliberately):** routeClaims in manifests, doorway-side type-driven 302-to-pretty-mount, fragment-preserving redirects, the designed gate experience at `/epr/{id}` for unreachable EPRs, card-flip + pushState mechanics (history stack / scroll / session continuity — principle-level only, §7 + §12.3 last bullet).

## The reading list (by role)

**Design canon (read first):**
- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` (id: `pillar-epr-decomposition-design`) — THE parent. §2 project-epr commitments + §2.4 validation rules; §3 element-registry manifests (the "claims ride like §3" referent); §6 outward face (`previewEprRef`/`gateHints`/`deadEnd` — the gate experience); §7 EPR-link HyperCard semantics + §7.2 `resolveInContext`; **§12 URL & Routing Contract**: §12.1 three surfaces + `/epr/{id}` resolver semantics (claimed→302, unclaimed→shell viewer, unreachable→gate), §12.3 mount-agnostic minting + the ONE-PARAGRAPH routeClaims sketch, §12.4 data flows + the `redirects_from` sanction, §12.6 slice table (Slice 2 marked landed, with implementation notes), **§12.7 vision tier** (toll access via `delegates-compute`, federated anycast doorways, compute-capability load balancing), §12.8 p2p-gate addendum (routeClaims = manifest content, class C *as sketched* — the brainstorm may revise), Appendix A glossary (projection, designed boundary, gate hint, steward-direct), Appendix B gate decisions.
- `genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md` (id: `omnibar-consolidation-epr-native-links-design`) — settled cross-bundle mechanics: §1 decisions (plain href + capture-phase interceptor; routerLink same-bundle only), §4.2 interceptor contract, §4.4 nav-stack handoff, §5 ServingContext ("which projection served you" — routing provenance), §9.7 substrate-provenance follow-up.
- `genesis/docs/superpowers/plans/2026-06-06-epr-slice2-universal-address-plan.md` (id: `epr-slice2-universal-address-plan`) — the landed plan; its "Design decisions locked" section records WHY (esp. decision 4: Slice-2 resolver semantics without claims; decision 6: the bridge rationale vs §12.4; decision 7: `/auth`-class protocol vocabulary vs app routing).
- `genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md` — Custodian→Steward→Operator chain via REA Commitments + Attestations: the governance substrate any "steward-governed routing" must back onto.
- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` §1 — the REA compute-commitment primitive (`Mishpat::Commitment`, `delegates-compute`): bounded reciprocity with on-chain standing + revocation + audit trail — §12.7's toll-access mechanism and the template for "a routing claim is a commitment."

**Principles / gospel rails:**
- `genesis/docs/architecture/stewardship-over-sovereignty.md` §3 — substrate-as-steward: why authority lives substrate-side, not client-side.
- `genesis/docs/architecture/elohim-sdk.md` — category C (operational) library boundary.
- `genesis/docs/architecture/pillar-bundle-split-runbook.md` — bundle-split canaries (incl. the app-absolute router-literal canary).
- `app/elohim-app/CLAUDE.md` (id: `elohim-app-frontend-gospel`) + `app/lamad/CLAUDE.md` (id: `lamad-bundle-gospel`) — the freshly-updated cross-bundle + universal-address rails (twins).
- `.claude/skills/p2p-design-gate/SKILL.md` — MANDATORY; the brainstorm's first structural move.
- Memory: `project_rea_compute_commitment_primitive` (gospel-tier — ONE substrate primitive instantiated across deploy/hosting/moderation/recovery; "displaces X-API-Key admin grants"), `feedback_k8s_is_not_the_architecture` (peer-native home is where design lands).

**Link-integrity precedent (the provability question's prior art, in-repo):**
- `genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md` + `.claude/skills/semantic-links/SKILL.md` — the docs corpus's content-addressed citation system: slug = identity, fingerprint = drift-truth, `path:` = tool-managed locator; `DEAD-CITE`/`HELD-CITE`/`stale` statuses; `cite-gen --verify` gate + `cite-propagate` sweep. The model to extend to runtime links.
- `app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.ts` + `elohim-epr-link.ts` — the runtime link primitives ("the `<a>` tag of the protocol"): capture-phase safety net (fails open) + blank-slate element (emits intent; host resolves).
- `genesis/docs/architecture/pillar-bundle-split-runbook.md` §4.4 — the router-literal canary: the existing static-audit precedent that the "no raw route literals" CI gate would generalize.
- `.claude/skills/page-model/SKILL.md` (testid-sync) + the a2o `look` tool (`pnpm look <url>`) — the render-verification machinery a link-conformance crawler would build on.

**Backlog (sweep + absorb or re-capture during the brainstorm):**
- `genesis/data/timeline/backlog/epr-routing-complementary-captures.md` — captures from the ORIGINAL §12 brainstorm (2026-06-04), several now in scope: `LamadNotFoundComponent → §6 gate experience` (explicitly gated on Slice 3 — this brainstorm's output unblocks it), alpha-ingress `/lamad/path` SSR-intent dead rules (§12.2 seam), doorway proxy dropping `X-Cache` (observability for link-serving provenance), tauri-direct deep-link verification, `/db/paths` list-route absence.
- `genesis/data/timeline/backlog/bundle-styling-token-contract.md` — adjacent (bundle delivery contract), not routing; leave.
- Slice-2 review-debt (small, recorded in this branch's commits): step-def cold/warm consolidation in `deep-link-delivery.steps.ts`; `EprNavService` → shared-lib collapse (currently shell-owned, lamad binds via token); `updateForProfile` canonical oddity; shared `eprUniversalHref` helper for the five raw `'/epr/' + id` template concats (encoding consistency — `search.component.ts` encodes, templates don't).

**Code anchors (ground truth for dispatch today):**
- `doorway/doorway-service/src/server/http.rs` — `is_service_path`/`is_reserved_url_path` (~1096-1150), EPR-router gate "B13" (~1674), `dispatch_to_projected_epr` (~1560: reach≠commons→401, mode≠Cached→501 — MVP gates, not design), `dispatch_epr_universal` (~1614), `derive_app_subpath` + `is_spa_route_subpath` (the ROUTE/ASSET rule).
- `doorway/doorway-service/src/projection/epr_router.rs` — longest-prefix dispatch over `EprProjectionView.url_path`; reserved-mount skip.
- `elohim/elohim-views/src/projection.rs` — `EprProjectionView`: `epr_id, url_path, mode, reach, base_href, entry_file, spa_fallback, redirects_from, preview_epr_ref, gate_hints, dead_end, steward_direct_endpoint` — the wire shape claims must extend or ride beside.
- `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts` + `bundle-route-context.ts` — the client-side claims contract Slice 3 must stay isomorphic with.
- `app/lamad/src/app/components/legacy-resource-redirect/legacy-resource-redirect.component.ts` — the bridge under adjudication.
- `genesis/seeder/src/seed-projections.ts` — projection seeding today (mounts: lamad→`/lamad`, imagodei-portal→`/auth/portal`).
- `elohim/sdk/domains/lamad/manifest.json` + `elohim/sdk/schemas/v1/` — manifest/schema homes if claims become declared vocabulary.
- `elohim/sdk/fixtures/spa-route-discrimination.vectors.json` — the two-layer drift-guard pattern to replicate for any new shared rule.
- `genesis/a2o/features/lamad/deep-link-delivery.feature` — the 7 routing scenarios incl. the bridge `@regression` anchor (the scenario that prompted this handoff).

## What Worked (patterns to carry forward)

- **Claims-as-injected-context** (Slice 2): one `EprResolverService` minting differently per bundle purely via composition-root `BUNDLE_ROUTE_CONTEXT` — Slice 3's doorway-side claims should be the substrate-anchored twin of this model, not a different model.
- **Two-layer drift guards**: one shared fixture consumed by both crates — replicate for whatever claim-resolution rule emerges.
- **Safe-by-default universal address**: unknown contentType → `/epr/{id}` always works. Any claims design must preserve this floor.
- **p2p-design-gate run EARLY** (§12.8 / Appendix B kept Slice 2 entity-free) — run the gate first, not as a post-hoc addendum.

## What Didn't Work (constraints discovered, now test-pinned)

- **Absolute `redirectTo` in a based bundle's router re-enters ITSELF** (self-loop; can never escape) — cross-bundle escapes need a component/full-load. Pinned by `lamad.routes.spec` + the bridge a2o scenario.
- **`ownsPath` over top-level segments only** broke pathless layout roots (pillar-bundle shape) — pinned by `epr-nav.service.spec`. A doorway-side claims matcher must not repeat this class (route SHAPE ≠ route TABLE).
- **Client-only redirect knowledge is invisible to governance** — the bridge works, but no commitment records it; exactly the gap this brainstorm adjudicates.

## Next Steps (ordered)

1. **Integrator: push `dev`** (31 local commits) — triggers the orchestrator; then watch the a2o `@browser-only` routing scenarios on alpha (first real render of the Slice-2 surfaces).
2. **Start the brainstorm in a fresh conversation**: `/brainstorm` opening with this HANDOFF.md. Scope: *"Canonical EPR-app routing — routeClaims substrate + route-level redirect governance."*
3. **Run `p2p-design-gate` as the first structural move.** Load-bearing gate questions:
   - Is a **routeClaim** notarized (A — a new `action` discriminator on the existing `Commitment`, like `project-epr` itself?), derived-via-link (A2 — content in the bundle EPR's manifest, addressed by CID, surfaced through the projection the doorway already loads), or operational (C — §12.8's current sketch)? *"Steward-governed routing" pulls toward commitment-anchored; weigh against entry-type headroom (Lamad ~73/100, Mishpat ~11/100) and §12.8's no-new-entry-types precedent.*
   - Is a **route-level redirect/alias** (the bridge's job) a commitment field (extend `redirects_from` from mount-level to route-template-level?), claim-content (legacy templates inside a routeClaim?), or explicitly debt-with-a-retirement-date? Each option needs its story/value/governance reading: who commits, who benefits, what does revocation mean for links already shared in the wild?
   - **Conflict resolution**: two bundles claim one contentType per doorway — operator curation (first-wins, like §2 path conflicts)? commitment standing? Does reach/graph context break ties (the "sensemaking" dimension)?
   - **Reach × claims composition**: a claimed-but-gated EPR → 302 to the mount's gate experience, or resolve at `/epr/{id}` with the §6 outward face? (Today's 401/501 in `dispatch_to_projected_epr` are MVP placeholders.)
4. **Design the dispatch-time consumption path**: how claims reach `EprRouter` (projections already flow boot-fetch + SSE refresh — do claims ride that flow or the bundle manifest fetch?); fragment-mapping ownership ("exactly one place per side", §12.3); and the client/doorway isomorphism guarantee (one claims vocabulary, two consumers → drift guard required).
5. **Design the link-integrity conformance system** (Goal question 3): the conformance classes (what MUST a minted link guarantee at author time / build time / serve time?); the static gate (lint rule generalizing the router-literal canary: links are minted, never literal); the build-time id-existence check (cite-gen's fingerprint discipline applied to code-minted EPR refs); the crawler/a2o conformance suite (every rendered anchor → rendered surface or designed boundary); and whether link-integrity attestation itself becomes substrate-visible (a doorway attests "all links in projection X resolved at audit time T" — the ISO-9000 audit record, possibly an Attestation on the projection commitment). Decide which layers are THIS spec vs captured follow-ups.
6. **Then card-flip + pushState** (§7 + §12.3 last bullet) — only after claims, since flips push "mount URL if claimed, else `/epr/{id}`": history stack, scroll restoration, session continuity.
7. **Sweep the backlog** (`epr-routing-complementary-captures.md` first) — absorb what this design unblocks (gate-experience upgrade, ingress SSR-rule reconcile), re-capture the rest with fresh pointers.
8. **Output**: a new spec (born-linked via `cite-gen --seal`, class `protocol-canonical`, citing `pillar-epr-decomposition-design` + `epr-slice2-universal-address-plan` + `semantic-computable-links-design`), amending or superseding §12.3/§12.8's sketch — then its own implementation plan. The §12.7 vision tier (federated anycast / toll access / capability load-balancing) stays designed-for unless the gate work naturally settles its substrate.

## Carried over from the 2026-06-05 handoff (still pending, not brainstorm-blocking)

- **Household-formation Task 10 fixture retirement** (precondition-gated): when a CI run shows `"partial": false` AND `GET /api/v1/commitments?action=custody-blob&state=active` returns triad rows with `metadata.seedGeneration == "ceremony"` → execute plan Task 10 (`2026-06-04-household-formation-ceremony-stage1.md`, ~30 min).
- **Post-deploy a2o verification** (omnibar + household): `features/browser/navigation-browser.feature`, `features/protocol/protocol-omni.feature`, `features/qahal/household-formation.feature` — fold into the same alpha watch as the new routing scenarios after the dev push.
- Backlog: `genesis/data/timeline/backlog/bundle-styling-token-contract.md` (graphos-tokens artifact); de-@wip `features/elohim-core/chrome-preferences.feature`.

---

_Open this file in a fresh conversation to begin: the brainstorm needs no other context beyond the reading list above._
