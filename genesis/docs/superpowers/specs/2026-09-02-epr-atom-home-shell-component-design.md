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
