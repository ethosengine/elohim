---
title: Contributor-Visual Surfaces — presences-on-EPR + imagodei profile (2 sprints)
id: contributor-visual-surfaces-plan
status: Draft
class: protocol-canonical
domain: D2
topic: [contributor-presence, imagodei, profile, epr, frontend, attribution, who-is-who, facings]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
cites:
  - contributor-presence-bootstrap-whoswho-design | the Wave 1-2 spec these two surfaces consume + render; this plan refines its visual layer | sha256:08d11210fd816f68 | path: genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
  - resilience-facings-select-fold-aggregate-design | the §11 facings framework the reflexive aggregator (surface B feed) is a child of | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
refines:
  - genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
requires_env: [household-nodes]
---

# Contributor-Visual Surfaces — presences-on-EPR + imagodei profile

The two visual surfaces that consume the contributor-presence substrate (Waves 1–2 of
`2026-06-21-contributor-presence-bootstrap-whoswho-design.md`). **Both are frontend rendering +
wiring on data that is ~90% ready — no new substrate, no new DHT entry type.** Grounded against the
live frontend 2026-06-22 (eyes-first; the deployed-alpha read plane was 404-ing all reads at
grounding time — see Env caveats).

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

## P2P design gate
Both routes are **read projections over already-notarized entities** — they follow from the existing
DHT design (gated in the bootstrap spec), they do not precede it. `GET /content/{id}/presences` reads
the existing `ContributorPresence` entry via its `establishing_content_ids` link (Category A2/C, no
new entry type); `GET /contributors/{id}/reflexive` is the Wave-2 Operational Category C facing over
`economic_events` + `stewardship_allocations` (already notarized). No new entry type, no new write
path, no new sync message — purely read surfaces.

## Sprint 1 — Contributor presences on an EPR
- [ ] Expose the content→presences edge over HTTP: wire the dead DB reverse-query
      `get_presences_for_content` (`elohim/elohim-storage/src/db/contributor_presences.rs:183`) to a
      route (`GET /api/v1/content/{id}/presences` or `/db/presences?establishingContent={id}`) +
      doorway passthrough. (MVP fallback: the client-side `getPresenceForContent` filter.)
- [ ] graphos contributor/credit card primitive (Library A default + Library B designed) — presence
      display-name, image, recognition, claimed/unclaimed badge; links to the profile (Surface B).
      None exists today.
- [ ] Inject the presence fetch into `ContentViewerComponent` (injects no presence service today) +
      render a "Contributors / Inspired by" section.
- [ ] Label `DERIVED_FROM` / `source_of` in the EPR-relationships panel (defined `RelationshipType`
      but **unlabeled** — `epr-relationships-panel.component.ts:18`).
- [ ] a2o scenario: "see who inspired/contributed to this content."
- [ ] Verify rendered (local stack or healthy alpha).

## Sprint 2 — Imagodei profile (claimed identity + unclaimed presence)
- [ ] **b1**: `/identity/profile/:id` route + by-id fetch (`IdentityService` / `HumanView`) +
      read-only (non-self) variant; refactor the 9 `profile/sections/*` to accept an input profile.
- [ ] **b2**: `presences/:id` route + presence-profile component; make presence-list cards clickable →
      detail.
- [ ] **b2** feed: add `getReflexive(id)` to `ContributorApiService`; render `ContributorReflexiveView`
      (total routed recognition, by-action, distinct content, commons flow [PARTIAL], steward
      allocations) as "how the network sees them".
- [ ] **b2** claim affordance: wire the already-written `initiateClaim`/`verifyClaim`/`getPresenceById`
      into the unclaimed-presence view ("Claim this presence", alongside "Become Steward"), gated by
      `presence_state`.
- [ ] `establishingContentIds → content links` on the presence profile (cross-link back to Surface A).
- [ ] a2o scenarios: view a network profile · view an unclaimed presence · initiate a claim.
- [ ] Verify rendered (local stack or healthy alpha).

## Reusable-vs-net-new (the 2-sprint sizing)
| Piece | Reuse | Net-new | Size |
|---|---|---|---|
| Sprint 1 (presence-on-EPR) | viewer shell, relationships panel, `ContributorPresenceView`, `getPresenceForContent`, `get_presences_for_content` (DB) | the HTTP route, the graphos credit card, the viewer section, `DERIVED_FROM` labeling | Medium |
| Sprint 2 b1 (claimed by-id) | 9 `profile/sections/*`, `IdentityService`, `HumanView` | `/profile/:id` route + by-id fetch + read-only variant | Medium |
| Sprint 2 b2 (unclaimed presence + claim) | `PresenceApiService` (claim methods exist), `ContributorReflexiveView` feed, `presence.model.ts` helpers | `presences/:id` route + component, `getReflexive()`, claim CTA wiring, recognition render | Larger (protect if Sprint 2 overflows) |

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
- Wave 3 (opt-out→commons) — DEFERRED in the bootstrap spec (DNA-notarized/heavy).
- The partial-commons + fractal-steward reflexive folds (no data substrate yet — `CoverageRollup` is
  the future recursion path).
- The manifesto-as-ContentNode establishing edge (the on-grain allocation link).
