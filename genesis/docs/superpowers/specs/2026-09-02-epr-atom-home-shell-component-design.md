---
title: "EPR Atom Home — the shell-owned /epr/{id} surface: one frame for every atom, four legs in household words, and the commons around it"
id: epr-atom-home-shell-component
tier: spec
status: Draft
class: ui-truth-layer
context-tier: disclosed
steward: angular-architect
graduation-trigger: >
  Slice 1 lands on dev AND the a2o @concern:epr-atom-home browser scenarios run green against the
  local stack (arrival strip, identity header, focal render for an html5-app and a markdown atom,
  the four legs, the out-of-reach gate) AND one alpha render
  (`pnpm look https://alpha.elohim.host/epr/evolution-of-trust`) shows the shell frame with no
  pillar chrome — OR superseded by a fresh reader contesting §2's frame on evidence.
created: 2026-09-02
maintainers: Matthew Dowell + Claude Fable 5.1
habits: [epr-atom-home]
topic: [epr, universal-address, shell, atom-home, four-legs, commons, community-page, conversation, sensemaking, standing, tender, brand-tokens, arrival, unreachable-gate, bundle-seams]
informed-by:
  - genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md (§12 — the URL contract this surface is the human-facing twin of; §12.1 "reachable but unclaimed → shell resource viewer", "unreachable → designed gate, never a wall")
  - genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md (the atom's first-person fold over its legs — this page IS that fold rendered)
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (Part II earned reach, Part IV honest welcome · standing as a shape · carrot before stick · restitution as repair)
  - genesis/graphos/elohim-protocol-design-spec.md (palette, type, voice — the tokens §5 binds)
  - genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md (route claims — how "Open in Lamad" is minted without a pillar literal)
  - app/CLAUDE.md §"Bundle seams are not domain seams" (the import ratchet §4 honours)
  - https://claude.ai/code/artifact/50e6f942-e332-43c4-b619-66f0a5d2ccb0 (the design canvas, v2 community layer — four artboards this spec is the written form of)
cites:
  - "pillar-epr-decomposition-design | Pillar EPR Decomposition | sha256:3db7d2c205a0d7d6 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md"
  - "epr-content-perspective-facing-lens-design | EPR / Content-Perspective Facing | sha256:4a389fc4679c3c7e | path: genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md"
  - "epr-route-claims-link-conformance-design | EPR Route Claims, Redirect Governance & Link-Integrity Conformance | sha256:1d9969399472335d | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md"
  - genesis/docs/content/elohim-protocol/social_medium/epic.md
---

# EPR Atom Home — the shell-owned `/epr/{id}` surface

> **One-line:** `/epr/{id}` stops mounting lamad's learner viewer and becomes the atom's own home: a
> stable frame for every content type (arrival → identity → focal render shaped by format → the four
> legs in household words → the commons around it), owned by the shell, composed from services the
> shell already reaches, with the unreachable case rendered as a designed gate.

## 0. Provenance

Operator direction 2026-09-02 after an eyes-first pass on `alpha.elohim.host/epr/evolution-of-trust`:
*"a new shell-owned EPR home component. It still needs to be able to link over into lamad (the
learning context), but otherwise this is the EPR homepage. It's got to have the social context that
feels like a reddit/community page."* The design canvas (informed-by) carries four artboards —
desktop immersive, desktop reading, phone, out-of-reach — built from the live alpha payloads for
`evolution-of-trust` and `succession`. New_ Public's four blocks for a flourishing digital public
space (welcome · connect · understand · act) are the external bar; the epic's own mechanics
(earned reach, standing as a shape, the tender, correction and vouch, repair) are the internal one.

## 1. The problem: a misrouted surface

`app.routes.ts` mounts lamad's `ContentViewerComponent` at `epr/:resourceId`, so the universal
address wears a learner costume. Observed on alpha (render 2026-09-02):

- "← Back to Lamad" hardcoded to `/` regardless of arrival; the session nav stack records every
  cross-bundle handoff and nothing reads it.
- The most prominent element is a steward's concern ("Needs help — invite a household") shown to a
  first-time visitor above the content.
- A 220px affinity card alone under a full-width header; four tabs hiding the legs; a toolbar row
  holding one FOCUS button; two empty rail panels taking a third of the viewport; the 7 MB
  simulation squeezed to two-thirds width.
- Trust as a percentage, reach as unexplained Unicode circles, urgency as a red dot; slate/indigo
  tokens and a blue gradient the brand spec bans outright.
- A missing node (`concept-bidirectional-trust`) renders the full chrome: Edit, affinity,
  "Invite a household" — §12.1 says never a wall.
- Three holding instruments disagree (resilience: 5 peers healthy · household: 1 of 3 at-risk ·
  blob distribution: 0 of 7 critical, reach class private); the page shows the argument.

## 2. The frame (stable for every atom)

```
[arrival strip]   ← previous stop (nav stack) · else path context · else nothing
[identity]        eyebrow (type · time · author · licence) · title · lede · chips: reach · notarized · held-by
[focal slot]      shape by contentFormat (§2.1) · Focus · open source · "Open in <bundle>" (§4.3)
[commons + legs]  main: your mark · conversation · where people stand
                  rail: who holds it · who's here · where this lives · how we talk here · how it's governed · where it came from
[address line]    /epr/{id} works on any doorway that can reach it · Network detail · Raw node
```

### 2.1 The focal slot picks its shape from `contentFormat`

| Shape | Formats | Layout |
|---|---|---|
| immersive | `html5-app`, `video`, `audio`, `external`, `sophia-quiz-json` | full container width; rail below beside the commons |
| reading | `markdown`, `html`, `plaintext`, `gherkin` | 2/3 column at 66ch; rail beside from the top, sticky |

The frame never changes with content type; only the slot does. Rendering itself is delegated
(§4.2) — this spec owns the frame, not the renderers.

### 2.2 The four legs, in household words

| Leg | Protocol leg | Reads | Empty state (one line) |
|---|---|---|---|
| Who holds it | value | `ResilienceSnapshotView.feltStatus` (headline · heldBy · floor pips · suggestedAction), distribution peers, commons pledges | "We can't confirm this is backed up anywhere yet." |
| Where this lives | knowledge | `EprHead.relationships` (typed), nav-context `partOf` when it resolves; related ids named in metadata rendered as links, unreachable ones tagged | "Not on a learning path yet." |
| How it's governed | governance | challenges (`getChallengesForEntity`), labels | "No challenges, no labels. Nothing is in question." |
| Where it came from | process | stewardship allocation (steward · contribution type · since), source/licence from metadata, DHT anchor + `dhtAnchorState` in words, created/updated, link to `/epr/{id}/raw` | never empty — the atom always has provenance |

**One verdict, in words.** `feltStatus` is the only holding verdict on the page. Shard maps,
replica counts and the blob-distribution verdict live behind "Network detail" (the existing raw and
resilience surfaces). The instruments' disagreement is filed, not displayed (§8).

### 2.3 The commons around the atom

- **Your mark on it** — affinity as one row (track + "Practicing · 20%" + two quiet buttons),
  shown only when signed in; copy says it is kept on the device.
- **Conversation** — discussions for this entity (`DiscussionView`, `messages`, `parent_id`).
  Sort: most vouched · newest · bridging. Named-feeling reactions (the existing
  `elohim-reaction-bar`) sit at the top. Each message carries author, household, and a standing
  ring (`elohim-qahal-standing-ring`); **no score on people.** Reply kinds are the epic's moves:
  reply · add context · correct with evidence · vouch (v1 encodes kind in the message
  `category`; a first-class kind field is a backend follow-up).
- **The tender** — the composer carries one plain line naming where this will reach (from the
  atom's `reach`) and that the author's standing rides with it, with "just my household" one tap
  away. A newcomer (no prior message from this identity on this doorway) sees the honest welcome
  and acknowledges before the first word.
- **Where people stand** — sensemaking statements (`StatementView`: agree · disagree · pass,
  `is_bridging` surfaced with a plum "bridging" tag). "Offer a statement" creates one.
- **Who's here** — distinct authors across discussions + statements, stewards, contributor
  presences; "Follow this page" (the existing subscribe signal).
- **How we talk here** — three covenant lines (reach is earned, never bought · a correction
  travels with the claim it corrects · repair is always open; exile is not a tool) and a link to
  the governing collective's rules when the atom's reach is a collective, else the commons covenant.

**Empty states collapse to a line.** No blank canvases, no "0 signals from 0 participants".

### 2.4 Arrival, not "Back to Lamad"

The arrival chip reads `SessionNavStackService.previous()` (label + url) first, then the path
context if the shell has one, else renders nothing. A cold share link has no chip; "Where this
lives" carries the place instead. "Open in Lamad" (or whichever bundle claims the type) is the one
primary action and is minted from route claims, never a literal (§4.3).

### 2.5 Unreachable is a designed boundary

`getContent` → null renders the gate: "We can't reach this one from here", the id in mono, what is
known (the referring atom from the nav stack, who holds that atom), and where to go (back to the
referrer; the same address on another doorway). No affinity, no Edit, no invite, no chrome.

### 2.6 Phone order

Header → focal → Open in bundle → Who holds it → your mark → conversation → where people stand →
the other legs as accordion rows. The rail never lands below the feedback block.

## 3. What the page reads (all live on alpha unless marked)

| Need | Source | Shell reach |
|---|---|---|
| atom | `StorageClientService.getContent` (raw `ContentView`) | shell |
| holding verdict | `ResilienceService.getSnapshot` → `feltStatus` | `@elohim/service` |
| distribution peers | `DistributionService.getDetails(blobHash)` | `@elohim/service` |
| stewardship | `StorageApiService.listStewardshipAllocations` | shell |
| relationships | `EprResolverService.resolveEprHead` | shell |
| nav context | `EprNavContextService` (`/api/v1/epr/{cid}/nav-context`) — **404 by slug on alpha today**; used when it answers | shell |
| discussions / statements / signals / challenges | `GovernanceApiService.queryDiscussions · getStatements · getSignals · getChallengesForEntity` | `@elohim/service` |
| affinity | `AffinityTrackingService` | shell |
| arrival | `SessionNavStackService.previous` | shell |
| session | `AuthService` | shell |
| containing paths | lamad-only computation today (loads every path) — **deferred**; §8 names the storage-side reverse route | — |

## 4. Ownership and seams

### 4.1 Shell-owned

`app/elohim-app/src/app/elohim/components/epr-home/` — `EprHomeComponent` (frame), with child
presentational components per region (`epr-home-identity`, `epr-home-legs`, `epr-home-conversation`,
`epr-home-stances`, `epr-home-gate`). `app.routes.ts` points `epr/:resourceId` at it. Lamad keeps
`resource/:resourceId` (legacy monolith URLs) and its own viewer for in-path reading; nothing in
lamad changes in Slice 1.

### 4.2 Zero new cross-workspace edges

The renderers are lamad-owned and the ratchet (`app/scripts/lint-workspace-imports.mjs`) refuses a
new specifier or a deeper count. The shell already carries the renderer-registry references in
`content-delivery.component.ts`. Decision: **extract that renderer host into a shell-owned
`EprFocalComponent`** that takes a slug, loads its node the way content-delivery does today, and
hosts the registered renderer; content-delivery composes it. The lamad references *move*, the
counts do not rise, and `EprHomeComponent` itself imports nothing from `@app/lamad/*` (it reads the
raw `ContentView` for the frame). The double fetch (raw view for the frame, adapted node for the
renderer) is accepted in v1 and disappears when the content substrate moves
(`arch-frontend-bundle-seams-backlog` row 1).

### 4.3 "Open in Lamad" without a literal

`codegen-route-claims.mjs` today emits each bundle's own claims. Extend it so the universal-route
owner's generated file also carries every declaring bundle's claims
(`BUNDLE_ROUTE_CLAIMS: {bundle, claims}[]`). The shell mints `mount(bundle) + claim.template`, with
the mount table living in the shell's composition root (defaulting to `/<bundle>`), until the
doorway's pretty-mount 302 (§12.6 Slice 3) supersedes the client-side link.

### 4.4 Lit elements

`import 'elohim-core/register'` and `'elohim-qahal/register'` in the component (per-component
opt-in, as the shell does today; qahal is not yet registered anywhere in the shell). Composed:
`elohim-reaction-bar`, `elohim-graduated-feedback`, `elohim-qahal-standing-ring`,
`elohim-qahal-memory-safety` (the felt-status card, in the rail, not above the content),
`app-epr-relationships-panel` (shell). The `viewer-relationships-panel` wrapper test id is kept so
the epr-link-navigation scenarios keep passing.

## 5. Brand tokens enter the shell, scoped

`app/elohim-app/src/styles/brand.css` declares the `--el-*` palette from the design spec (Linen,
Starlight, Hearthstone, Vineyard, New Growth, Harvest Gold, Terracotta, Morning, Sabbath, Deep Sky,
Indigo Night) and the four faces via `@fontsource-variable/{fraunces,source-serif-4,dm-sans,
jetbrains-mono}` (already resolved in the workspace lockfile for the library). The EPR home paints
its own linen ground on its host and binds only `--el-*`; the shell's `--lamad-*` tokens stay for
every other page until a separate brand pass. Rules honoured: no pure black or white; one Harvest
Gold element per composition (the holding pips / your mark); Terracotta for "needs help", never
red; serif for reading, sans for doing; no all-caps.

## 6. Test ids and scenarios

Region ids: `epr-home`, `epr-home-arrival`, `epr-home-title`, `epr-home-chip-reach`,
`epr-home-chip-notarized`, `epr-home-chip-held`, `epr-home-open-in-bundle`, `epr-home-focal`,
`epr-home-leg-holds`, `epr-home-leg-here`, `epr-home-leg-lives`, `epr-home-leg-talk`,
`epr-home-leg-governed`, `epr-home-leg-from`, `epr-home-conversation`, `epr-home-composer`,
`epr-home-tender`, `epr-home-stances`, `epr-home-gate`, `epr-home-address`. Retained from the
viewer for existing steps: `viewer-relationships-panel`, and the focal keeps lamad's
`.markdown-content` for the direct-view step.

Scenarios: `genesis/a2o/features/content/epr-atom-home.feature` (`@concern:epr-atom-home`,
`@browser-only`, `@act:i`). Unit: vitest per component; OnPush + signals; an Eager host spec for
the frame (the implicit-OnPush harness blindness).

## 7. Slices

| Slice | Contents | Proof |
|---|---|---|
| **0 — brand foundation** | `brand.css` tokens + fonts; no page changes | build green; tokens visible in devtools on any page |
| **1 — the frame** | `EprFocalComponent` extraction (count-neutral) · `EprHomeComponent` with arrival, identity, chips, focal, legs holds/lives/governed/from, address line, gate · route switch | scenarios 1–5 green locally; `pnpm look` on the local stack and on alpha after deploy |
| **2 — the commons** | conversation (discussions + messages, reply kinds, sort), tender + newcomer welcome, where people stand, who's here, how we talk here | scenarios 6–8 |
| **3 — bundle lens + phone** | generated all-bundle claims + mount table → "Open in Lamad"; phone order; lamad-side "open in EPR home" back-link audit | scenario 9; phone `pnpm look --viewport 390x844` |

## 8. Filed, not built here

- **Containing paths** need a storage-side reverse projection (`partOf` on nav-context resolvable
  by slug, or a `/db/content/{id}/part-of` view). Category C projection over existing rows; no new
  DHT entries. Until then "Where this lives" shows relationships only.
- **Three holding instruments disagree** for the same atom (resilience · household · blob
  distribution). The page shows one; the contradiction goes to the dataplane backlog.
- **Governance state / mechanism / accumulation** return 404 on every load; the home does not call
  them until the endpoints exist.
- **`relatedNodeIds`** live only in metadata; all four for `evolution-of-trust` are unseeded on
  alpha. The rail names them and tags them unreachable rather than hiding a "(4)".
- **Message kind** (reply · context · correction · vouch) as a first-class discussion field, and
  "most vouched" as a real sort, need the qahal discussion model to carry them.

## 9. P2P design-gate record

Zero new DHT entry types, tables, routes or identity schemes. Every leg is a read over notarized or
operational views that already exist; writes (discussion, message, statement, vote, reaction,
follow, affinity) go through the existing coordinator-backed routes. The one codegen extension
(§4.3) changes a generated TypeScript consumer, not the manifest schema. The only new data need is
the containing-paths reverse projection in §8, explicitly a projection over existing rows.

## 10. Non-goals

Doorway-side pretty-mount 302s; moving the content substrate out of lamad; restyling the rest of
the shell; the elohim "tender" as a live agent (v1 renders the reach truth as copy); comments as a
separate model (discussions are the conversation).

## 11. Slice 4 — the atom home subsumes the resource viewer (2026-10-14)

Operator direction 2026-10-14 after comparing `alpha.elohim.host/epr/evolution-of-trust` with
`alpha.elohim.host/resource/evolution-of-trust#network`: *"this page needs to subsume ALL the
designed concerns that you see on [the resource page]… both views are in-essence trying to do the
same thing: provide the commons surface / epr meta, lenses, and tooling of this epr."* Two surfaces
for one atom is the misrouting §1 named, half-cured: the home sends its own visitors back to the
viewer for "Network detail" and "Raise a concern" (`epr-home.component.html:210`,
`epr-home-legs.component.html:75`), and the epr-link context menu's "network" action lands on the
viewer's tab (`epr-link.component.ts:308`).

### 11.1 Principle

One frame, the legs, no tabs. Every concern the viewer carried lands in the leg whose protocol leg
it belongs to, as a **fold** under that leg's one-line verdict — closed by default, opened by its
toggle or by arriving with the leg's fragment (`#holds · #lives · #governed · #from · #here`).
Nothing on `/epr/{id}` links to `/resource/{id}`; `/resource/{id}` becomes a redirect to the
universal address (fragment map: `network→holds`, `governance→governed`, `trust→from`,
`content→` none; `holds|lives|governed|from|here` pass through). The viewer stays lamad's in-path
lesson component only.

### 11.2 Where each viewer concern lands

| Viewer concern (`content-viewer.component.html`) | Atom home | Reads (shell reach, no new `@app/lamad/*` edge) |
|---|---|---|
| Network tab → resilience grid, health badge | **Who holds it → fold "Network detail"**: health in words first (`Healthy · can lose N peers and still be whole` / `At risk · …`), then rows encoding · pieces distributed (`shardsWithLocations/totalShards`) · peers holding pieces · storage committed | new shell `ResilienceDetailService` (`elohim/services/`) — `GET /api/v1/resilience/{id}` → `ResilienceView` from `@elohim/storage-client/generated` (lamad's `services/resilience.service.ts` is the duplicate to shrink later) |
| Shard map | same fold, "Where the pieces are": one cell per shard, data vs parity, a missing shard Terracotta | same |
| Verify Resilience + result | same fold, button "Check it's still whole" → one line: `Whole · 1 of 1 pieces found · 12 ms` or `Not whole · 2 pieces missing` | `POST /api/v1/resilience/{id}/verify` → `VerificationResultView` |
| Distribution badge (replica count/target) | "Who holds it" row already reads `replicaCount`; the fold adds `copies: N of M wanted · {replicaHealth}` | `DistributionService.getDetails(blobHash)` (already fetched) |
| Memory-safety card | the leg's headline (unchanged) | — |
| Attention metrics | **Who's here** (new leg, §2.3): `Viewed N times by M people · K completions`; empty → `No one has been counted here yet.`; one quiet line: counted as protocol events, never analytics | `StorageClientService.getContentEngagement(id)` (JSON numbers; `Number()` the bigint-typed fields) |
| Attestations tab → trust %, badges, reach label | no percentage (§2.2). **Where it came from → fold "Attestations"**: one row per attestation — `title · by {issuer, short} · {proofClass} · {date}`; revoked rows struck with the reason; empty → `No one has attested to this yet.` | `AttestationApiService.listBySubject(id)` (shell, all kinds) |
| Stewardship allocations (ratio, recognition, dispute) | "Where it came from" steward rows gain `{displayName ?? presenceId} · {ratio}% share · {recognition} recognition`; disputed rows carry `Disputed: {reason}` in Terracotta | `StorageApiService.getContentStewardship(id)` replaces `getStewardshipAllocations` in `epr-home.component.ts:148` |
| Content warnings / flags | **How it's governed**: labels from `GovernanceStateView.labels` (when the endpoint answers) and the atom's own flags, as warm pills | `GovernanceApiService.getGovernanceState('content', id)` — 404 → null, silent (§8 stands: nothing renders until it answers) |
| Challenges + SLA | "How it's governed" rows gain the challenge's evidence line, `filed {date}`, and days remaining when a deadline exists | `getChallengesForEntity` (already read) |
| File a Challenge · gate feedback trigger (flag/challenge/feedback/report) | "How it's governed" → **Raise a concern** is `<elohim-gate-feedback-trigger>` on the page itself (`elohim-core/register`), its `feedback-submit` event handled by the home: `challenge` → `fileChallenge`; `feedback` → `recordSignal`; menu items restricted to the kinds that have a real route. Signed out → one line `Sign in to raise a concern.` | `GovernanceApiService.fileChallenge / recordSignal` (the viewer composed the trigger with no listener — decorative; here it acts) |
| Feedback-mechanism gateway | **not composed**: `getMechanismSelection`/`getAccumulationStatus` swallow a 404 into a default object (`governance-api.service.ts:291-350`), so "resolved" cannot be told from "absent"; §8 stands until the endpoints exist and the service surfaces absence | — |
| Discussions + Start Discussion | **Conversation** (§2.3, first cut): rows `title · N messages · status`, newest first; `Start a conversation` (one message — `CreateDiscussionInputView` carries no title; rows read `title ?? first line`, plus `category`) when signed in; empty → `No one has spoken about this here yet.` The tender and standing rings stay Slice 2 | `queryDiscussions / createDiscussion` |
| Reaction bar · graduated feedback · signal summary | **Your mark on it** grows "How it landed": `<elohim-reaction-bar>` with `.counts` adapted from the signal aggregate's bigint dictionaries (`epr-home-signals.ts`, pure, unit-tested) and `.onReactionSubmit` → `recordSignal` with every `RecordSignalInputView` field sourced from the session; `<elohim-graduated-feedback>` only with what the aggregate truly cross-tabulates, else omitted; one summary line in words (`N reactions · consensus {level}`), no percentages | `getSignalAggregate / recordSignal` |
| Appears in learning paths | still §8 (storage-side reverse projection); not built | — |
| Contributors / Inspired by | **Where it came from → "Inspired by / contributed to by"**: `<elohim-contributor-card compact>` per presence (`elohim-imagodei/register`, tag per `register.ts:81`), in the leg body — never inside a closed fold (`contributor-presences.steps.ts` asserts visibility on load); test ids `viewer-contributors-section / -list / viewer-contributor-card-{id}` kept for `contributor-presences.steps.ts` | `StorageApiService.getPresenceForContent(id)` |
| Exploration sidebar (authored + discovered see-also, explore in graph) | **Where this lives → fold "See also"**: authored rows from `getRelatedContent(id)` (`relationshipType`, target title) and discovered rows tagged `discovered` when `inferenceSource !== 'explicit'`; each row a minted `/epr/{id}` link. "Explore in full graph" is lamad's and is filed, not built | `StorageApiService.getRelatedContent(id)` |
| Download | "Where it came from": `Download ({format})` as `<a download>` to the blob when `blobHash` exists | `StorageClientService.getBlobUrl` |
| Edit | identity actions: `Edit in Lamad` when signed in, minted through `bundle-lens.ts` (`BUNDLE_EDITORS`, same `route-literal-ok` composition-root pragma as `BUNDLE_MOUNTS`); never for generated step ids | — |
| Focused view toggle | focal slot gains **Focus**: host class `epr-home--focused` hides everything but the focal; Escape exits | — |
| Tags · version | identity: tag chips under the lede; `v{version}` in the eyebrow when present | raw `ContentView.tags`, `metadata.version` |
| Affinity ± nudges | dropped by design (§2.3) | — |
| Witnessed-attention observation (`content-viewer.component.ts` ~395-460, ~640-660: scroll depth → `ObservationEmitterService`) | the atom home emits the same observation the same way (window scroll, or the focused container when Focus is on) — `attention-witnessed-privately.feature` proves it | `@elohim/rea-runtime` `ObservationEmitterService`, `EventService`, `scrollDepthPct` |
| Trust-badge actions (request review …), sentiment/effectiveness percentages, mini-graph | dropped: lamad-only UI with no backend route; the summary line in words and the See-also fold stand in | — |
| Back to Lamad · Return to path · Open in pillar | arrival chip and lens (unchanged) | — |

### 11.3 Test ids (contract for the a2o leg)

Folds: `epr-home-fold-toggle-{holds|lives|governed|from}`, `epr-home-fold-{holds|lives|governed|from}`.
Network detail: `epr-home-health`, `epr-home-pieces`, `epr-home-verify`, `epr-home-verify-result`.
Who's here: `epr-home-leg-here`, `epr-home-attention`. From: `epr-home-attestations`,
`epr-home-attestation`, `epr-home-steward`, `epr-home-download`. Governed: `epr-home-labels`,
`epr-home-challenge`, `epr-home-raise-concern` (the trigger's host). Lives: `epr-home-see-also`,
`epr-home-see-also-card[data-inference]`. Commons: `epr-home-conversation`, `epr-home-discussion`,
`epr-home-start-conversation`, `epr-home-reactions`. Identity: `epr-home-tags`, `epr-home-tag`,
`epr-home-focus`, `epr-home-edit`. The address footer keeps `epr-home-network-detail` as an in-page
`#holds` anchor that opens the fold. Retired with the viewer route: `viewer-tab-*`,
`viewer-affinity-*`, `viewer-file-challenge`, `viewer-start-discussion`, `viewer-verify-resilience`.

### 11.4 Seams and gate

The redirect is `pathMatch: 'full'`, preserves query params, and runs client-side with the fragment (a browser never sends the fragment to SSR; the hydrated router re-runs it). Zero new `@app/lamad/*` specifiers (the ratchet refuses). The redirect mints through
`eprToUniversalHref`, never a literal. The epr-link "network" action targets fragment `holds`.
The three pillar components that still `router.navigate(['/resource', id])`
(`shefa/components/resource-explorer/resource-explorer.component.ts:278`,
`imagodei/components/profile/profile.component.ts:369`,
`profile-discovery-section.component.ts:45`) mint the universal address instead. Gate:
`just gate elohim-app` (lint · lint-routes · lint-a11y · deps · build · test) plus a2o
`@concern:epr-atom-home` against the local shell on alpha data; render proof with `pnpm look` for
`#holds` open, the reading shape, the phone width. P2P design-gate record: unchanged from §9 —
every addition is a read over existing views or a write through an existing coordinator-backed route.

### 11.5 Filed, not built here

Explore-in-graph from the atom home (lamad route, no claim shape for it yet); appears-in-paths
(§8); lamad's duplicate `ResilienceService`/`resilience.service.ts` shrinking onto the shell's;
the `exploration-sidebar.feature` standalone scenario retargets to the atom home's See-also fold
(the lamad sidebar remains covered by its path-step scenario).
