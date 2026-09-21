---
id: "backlog-legacy-web-content-projected-inward"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Legacy web content projected INTO the network — external references as typed, witnessed, bounded things rather than markup"
slug: "legacy-web-content-projected-inward"
written: "2026-09-17"
author: "doorway-overnight-20260914 shift, from an operator design conversation prompted by the apex-transition sibling-browser failure"
status: "backlog"
priority: "medium"
tags: [open-question, legacy-web-projection, bridge, external-format, embed, privacy, reach, witnessed-interaction, chrome, a2o-oracle, design-pass]
cites:
  - bridges/CLAUDE.md
  - doorway/CLAUDE.md
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - app/elohim-app/src/app/components/hero/hero.component.html
  - genesis/a2o/steps/dataplane/apex-transition.steps.ts
  - genesis/a2o/src/framework/dataplane/real-app-network.ts
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/laneA-failure-analysis.md
---

# Legacy web content projected inward

**This entry is a collection point, not a design.** It names the question and gives every place
that touches it one tag to carry — `legacy-web-projection` — so a later deep design pass can
surface and reconcile them together. Tag a site when you meet it; do not fix it locally in a way
that pre-empts the design.

## What prompted it

The landing page is a notarized, content-addressed ContentNode served under a declared head. Inside
it, `hero.component.html` embeds a YouTube player — an iframe whose bytes, behaviour and telemetry
belong to a third party. On 2026-09-17 the a2o scenario "The apex name survives its doorway's shed"
failed at `apex-transition.steps.ts:1135` on four `net::ERR_ABORTED` requests to
`youtube.com/api/stats/atr` and `youtubei/v1/log_event`: a doorway-failover proof went red on a
third-party beacon. The scenario was right to *notice* those requests and wrong about what they
*meant* — because the protocol has no way to say what they are.

The page's integrity story stops at the iframe boundary, and nothing declares that it does.

## The question

How does legacy web content live inside a governed, content-addressed place — when online, and
when not — without either pretending it is native or pretending it is absent?

## Design directions to evaluate (none decided)

1. **The reference is first-class and typed.** `external` is already a core DNA-notarized format.
   An embed becomes an EPR whose payload is a *claim about an outside resource* (URL, kind, who
   cited it, when last witnessed). The reference is notarized; the bytes are not. The network can
   then answer "what outside resources does this household depend on?"
2. **Bridge seam, not renderer.** Translating an external protocol means adding a crate
   (`bridges/`); legacy web is the largest external protocol. A web bridge would own projection
   policy: reference-only · witnessed snapshot (title, thumbnail, transcript, hash of what was
   seen) · full capture where licensing allows.
3. **Online/offline is a gradient, stated honestly.** Online: the live resource inside a declared
   boundary. Offline or peer-only: the last witnessed projection, labelled as a snapshot with its
   date and witness. Same shape as verify-locally-then-serve; staleness graded by stakes.
4. **The boundary is a protection surface.** A raw third-party iframe lets an outside party observe
   a person inside a governed place, which undercuts what the chrome signals. Candidate default:
   click-to-load or a privacy-preserving facade; the chrome marks leaving governed ground. Reach
   applies — a commons page may cite the open web; an intimate-reach space may not phone out.
5. **Witnessing turns link rot into shared memory.** Peers attesting "this URL resolved to this
   hash at this time" is the witnessed-interaction primitive pointed outward: a plural,
   attributable record of what a link meant when cited. Thin — notarized observations, not a
   warehouse of other people's bytes.
6. **Test oracles gain a category.** With typed external references, "requests to declared
   external origins" is something a harness can reason about: a hidden request to an *undeclared*
   origin still fails; a declared embed's telemetry abort is not evidence about the system under
   test.

**Standing caution:** the bridge must not launder the legacy web into looking native. Its value is
that the seam stays visible — this part is governed, that part is theirs, here is what was witnessed.

## Open questions for the design pass

- P2P design gate: is an external reference Notarized (A), Linked (A2 — an attribute of the
  citing ContentNode), or does only the *witness observation* get notarized? What is the head-plane
  cost of witness records at one year?
- Which existing entry types and the `external` format already carry this, and what is missing?
- Who may witness, and does a witness observation earn or spend anything (REA)?
- What does the chrome show at the boundary, and what is the friend-voice wording for a person?
- Where does consent for a live third-party load get recorded, and at what reach is it refused?
- Licensing and takedown: what may a snapshot hold, and how is redress routed?

## Sites already known (tag these `legacy-web-projection` when touched)

| Site | Why it belongs here |
|---|---|
| `app/elohim-app/src/app/components/hero/hero.component.html` | FACADE as of slice 1 (below) — was the live YouTube embed inside the notarized landing node |
| `genesis/a2o/steps/dataplane/apex-transition.steps.ts` (~1135-1143) | Raw request-failure capture with no notion of a declared external origin |
| `genesis/a2o/src/framework/dataplane/real-app-network.ts` (`EXPECTED_NEGATIVE_HTTP`, origin-escape guard) | The harness's hand-kept list standing in for a protocol-level declaration |
| `bridges/` | Home for a web bridge if direction 2 holds |
| `doorway/doorway-service` web2 bridge consumption | Where an inward projection would be served and cached |

## Slice 1 (landed 2026-09-18, `feat(app): load outside video only when the person asks for it`)

Direction 4 only — **the boundary as a protection surface**, at the one site that was actively
phoning out. `ExternalEmbedComponent`
(`app/elohim-app/src/app/elohim/components/external-embed/`) replaces both hero iframes with a
click-to-load facade. Before the person asks: no iframe, no remote thumbnail, no preconnect —
nothing third-party is fetched. The facade names the host (derived from `new URL(...).hostname`,
so the label cannot lie), says the content isn't part of this place and that playing it lets the
provider see they watched, and offers one button. On activation it embeds the privacy-enhanced
host (`www.youtube-nocookie.com`) under a restrictive `sandbox`, moves focus into the player, and
keeps a persistent caption so the seam stays visible after load. An unrecognised host is never
embedded — the facade says so honestly instead.

**What slice 1 deliberately does NOT decide** — these stay open above, and the component holds no
opinion on them:

- **Where consent is recorded.** Consent is per-embed and per-page-view; nothing is persisted (no
  localStorage, no cookie). The open question is which layer owns a durable consent and at what
  reach a live outside load is refused.
- **The typed external reference** (direction 1). The embed URL is still authored as markup in a
  template. Nothing here mints an EPR for the outside resource, and the provider allow-list is a
  hardcoded TS table, not a notarized declaration.
- **Witnessing** (direction 5). No observation of what the URL resolved to is recorded or shared.
- **The bridge seam** (direction 2). No crate, no projection policy, no snapshot; the online case
  only, and only for video.
- **The test-oracle category** (direction 6). The harness still has no notion of a *declared*
  external origin — slice 1 merely removes the undeclared requests from this page's load.

Evidence: `app/elohim-app` vitest (32 tests across the component + hero), `ng build --configuration
development` (AOT + strict templates), and a `pnpm look` render of the built bundle in both schemes
showing zero requests to any youtube/ytimg/google host on load.

## Regression observed 2026-09-20/21 — raw iframe requests recurred despite slice 1

The scenario "The apex name survives its doorway's shed" (`features/dataplane/doorway-apex-transition.feature:119`)
failed the same assertion class again on three consecutive household runs the night of 2026-09-20
(`F8-apex-transition-run2.log`, `F8-apex-transition-run3.log`, `G3-apex-transition-run.log`;
sprint reports `sprint-report-household-20260920T223619Z-3ec3614d.md` and
`…T232300Z-3ec3614d.md`) — this time on literal `net::ERR_ABORTED` requests to the raw embed URLs
themselves (`https://www.youtube.com/embed/6g6v7ZMEAxk`, `.../sVXwZ087ffA`), fired on page load with
no click, at `apex-transition.steps.ts:1391` (assertion at `:1403`).

**What is proven.** Current source (`hero.component.ts`/`.html`, read 2026-09-21) still uses
`ExternalEmbedComponent` with the click-to-load facade for both slots, same two video ids — the fix
was not reverted in-tree. A facade-rendered page issues zero network requests to any youtube host
before a click; the failure logs show real requests to the raw embed path at load time, so the SERVED
page — not the current source — lacked the facade. All three failing runs (22:35–23:27Z) predate the
household's next full `ng build --configuration development` (`J2-build.log`, timestamped 23:51Z, the
build accompanying commit `4dce6642f`'s server-bundle-feedback-loop fix) — so the dist these runs
served was staged before that rebuild.

**What is the strongest candidate cause, not fully proven.** `FINDING-server-bundle-feedback-loop.md`
(found ~23:30Z, i.e. between the F8/G3 failures and the J2 rebuild) documents the household's SSR
bundle path materializing INTO the source dist across every cast, so a prologue could package and
serve an old `.reconcile/<slug>/<hash16>/` generation's `main.server.mjs` instead of the freshest one —
five generations coexisted that day. An old generation predating this facade (or predating any build
with `ExternalEmbedComponent` wired into `hero.component`) being reconciler-selected is consistent with
every observed fact, but this pass did not read the served HTML byte-for-byte or diff the specific
`.reconcile` generation hash against the facade's landing commit, so the mechanism is corroborated by
timing and by the bundle-pollution finding, not directly confirmed.

**What is ruled out.** The `ng build --configuration development` flag itself is not implicated — that
configuration is the one the household prologue is required to stage for hosted-steward scenarios
generally (`project_mesh_browser_lane_needs_dev_config_dist`); nothing found ties it to stripping or
bypassing `ExternalEmbedComponent`.

**On scoping the assertion to first-party origins:** the step's own comment (`apex-transition.steps.ts:1394-1398`)
already records this as an open, deliberate non-decision — "the declaration belongs to the view and the
protocol, not the harness... Do not allowlist it here." This backlog entry does not reopen that call;
it reports a regression in what the harness observed, not a request to rescope the harness.

## Current decision

Captured, direction 4 partially landed (slice 1 above), and reconfirmed regressed on a stale/polluted
served bundle 2026-09-20/21 rather than a source reversion. No owner for the remaining directions;
blocks nothing beyond the one scenario. The narrow harness question (whether the sibling-browser
capture may be scoped to household origins) remains the operator's call, tracked in the shift journal,
not here — settle it without foreclosing direction 6. Next action before re-litigating the design: get
one apex-transition run against a freshly staged dist (post-`4dce6642f`) and confirm the assertion
passes clean, which would close the regression as bundle-staleness rather than a code regression.
