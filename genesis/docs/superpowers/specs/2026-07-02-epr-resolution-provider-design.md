---
title: "EPR Resolution Provider — head-first previews, typed degradation, one ambient contract, manifest-declared route claims"
id: epr-resolution-provider-design
status: Draft
class: ui-truth-layer
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
domain: D-epr-apps
topic: [epr-link, resolution, epr-head, degradation, provider, injection-token, lit-context, route-claim, manifest-codegen, reach, hypercard, transport-abstraction, doorway-projection]
cites:
  - omnibar-consolidation-epr-native-links-design | Predecessor spec — it settled the two link classes, the /epr/{id} universal address, and the capture-phase interceptor (the navigation seam); THIS spec is its resolution-layer successor, completing the resolve/preview half it left as four independent body-endpoint wirings | sha256:3b018cf87bf8a809 | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
  - pillar-epr-decomposition-design | Parent canon for the HyperCard resolveInContext idiom, the /epr universal resolver, and the RouteClaimTemplate serializable claim shape (§3.1/§8.3) that I3 finally sources from the app manifest | sha256:3db7d2c205a0d7d6 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
  - elohim-seam-map-concern-routing | Seam placement — this design lives where the client surface (§3.8, element degradation + resolver) meets doorway projection (§3.9, the anonymous head endpoint + route-claim mount table); the head-vs-body split is the concern-routing line it honors | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - genesis/data/timeline/backlog/alpha-manifesto-content-403.md
---

# EPR Resolution Provider

The protocol's link/preview substrate resolves **four different ways** across four
call sites, resolves previews against the **reach-gated body** endpoint (so a
community-reach card dies where its head would have loaded), degrades every failure
into **one untyped `unreachable`** state that renders the raw `epr:` string, and
maintains route-claim vocabulary in **two independent homes**. The primitives are
right; the wiring is not. This spec is the resolution-layer successor to the
`omnibar-consolidation-epr-native-links` design — it consolidates the resolution
seam the way that spec consolidated the chrome seam.

- **Date:** 2026-07-02
- **Status:** Design (operator-approved direction, 2026-07-02; pre-implementation)
- **Seam:** Client surface (atlas §3.8 — the resolver + element degradation) meeting
  doorway projection (§3.9 — the head endpoint + route-claim mount table). Both
  home moves stay on the content-address plane; agent identity is untouched.
- **Predecessor:** `2026-06-05-omnibar-consolidation-epr-native-links-design.md`. It
  ratified the two link classes and the `/epr/{id}` universal address; this spec
  takes the *resolution* half of that world to completion.

## 1. Motivation — the weak spot

Link-and-preview handling is the protocol's most-touched read surface and its most
duplicated one. Four weaknesses, each evidence-backed.

**W1 — Four resolution paths, zero providers.** The same semantics ("resolve an
`epr:` ref, render a preview, navigate on click") are wired four independent times:

| Path | Site | How it resolves |
|---|---|---|
| Angular epr-link wrapper | `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts` | injects `EprResolverService` per instance (`:106`) and sets `litEl.resolver = () => eprResolver.resolve(...)` (`:169`) |
| Lamad markdown renderer | `app/lamad/src/app/renderers/markdown-renderer/markdown-renderer.component.ts` | renders `<a data-epr>` anchors (`:200`), wires its own click handler → `resolveAndNavigate` (`:420`) → `resolveInContext` (`:434`) |
| Relationship card | `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.ts` | injects `EprResolverService` + `ResilienceService` and joins its own resolve + resilience fetches |
| Capture-phase interceptor | `app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.ts` | a fourth net — document-level click capture for leftover/content-authored anchors |

Four wirings, one meaning. Each re-derives the transport decision, the failure
handling, and the navigate rule independently. That is the drift surface.

**W2 — Previews resolve against the body, not the head.** `EprResolverService.resolve()`
(`app/elohim-app/src/app/elohim/services/epr-resolver.service.ts:152-171`) calls
`StorageClientService.getContent(ref.id)` → `/db/content/{id}`. That endpoint is
**reach-gated**: `handle_db_content_by_id` (`elohim/elohim-storage/src/http.rs:4649`,
gate at `:4684-4699`) serves anonymously **iff** `reach ∈ {commons, public}`, else
returns `403` unless an `Authorization`/`X-Agent-Id` header is present. Meanwhile
`/epr-head/{id}` (`handle_get_epr_head`, `http.rs:8167`) derives the head with **no
reach gate** — heads flow freely, bodies are gated. This is not aspirational: the
resolver **already has** the correct call — `resolveEprHead()` (`epr-resolver.service.ts:263-280`)
fetches `/epr-head/{id}` with DAG-CBOR negotiation — but the preview path
(`resolve()`, and therefore the `litEl.resolver` the Angular wrapper wires) never
uses it. The service **conflates resolve-for-preview (head) with fetch-for-render
(body)**. This is the root of the live landing's dead manifesto chip: anonymous
`/db/content/manifesto` `403`s while `/epr-head/manifesto` `200`s (tracked:
`genesis/data/timeline/backlog/alpha-manifesto-content-403.md`).

**W3 — The element API over-promises and degrades untyped.** `<elohim-epr-link>`
(`app/elohim-elements/elohim-core/src/elohim-epr-link.ts`) publishes
`display = 'inline' | 'chip' | 'card' | 'popover'`, but `render()` (`:187-223`)
branches **only on load level** — `display` is never read, so `display="card"` is
cosmetic. Worse, `resolve()` (`:313-334`) collapses **null (not-found), rejection
(network), and 403 (forbidden)** all into a single `loadLevel = 4,
{ unreachable: true }` state whose only rendering is `<elohim-mention-base>`
falling back to the raw `epr:` string. The `EprLinkResolution` interface carries one
boolean, `unreachable?`. A resolvable-head-but-forbidden-body — exactly the manifesto
case — is indistinguishable from a genuinely-missing ref, so a card that *could* show
its title and an honest affordance instead shows a raw `epr:` string.

**W4 — Route claiming lives in two homes.** Which bundle renders which `contentType`
natively is declared twice and maintained independently:

- **TS home:** each bundle's `BUNDLE_ROUTE_CONTEXT` — the shell provides
  `{ claims: [], ownsUniversalRoute: true }` (`app/elohim-app/src/app/app.config.ts:136-137`);
  lamad provides a `path` claim. `eprToRoute` consumes these
  (`app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts:240`).
- **Doorway home:** the `EprRouter` compiles a `claims` index of
  `contentType → mount` bindings and dispatches by longest-prefix match
  (`doorway/doorway-service/src/projection/epr_router.rs:184,300-328`).

Two homes for one fact drift apart — the same failure shape as the
cluster-state / deployments.json drift of 2026-06-03.

## 2. What is already right (credit before change)

The primitives ratified in the predecessor spec are sound and are **kept**:

- **Two link classes.** Content-resolution links via `<elohim-epr-link>` (HyperCard
  card-flip in place) vs cross-bundle navigation via plain `href` to the universal
  `/epr/{id}` address + interceptor handoff. This spec changes neither.
- **The injectable `.resolver` seam.** `<elohim-epr-link>` already decouples
  resolution from rendering: production wires `.resolver`; tests use `setResolution()`.
  The provider (I2) plugs into this exact seam — no element-contract widening for the
  wiring itself.
- **`resolveInContext` — the HyperCard rule.** The same `epr:` link resolves to
  in-path / cross-path / standalone depending on where it is clicked
  (`epr-resolver.service.ts:205-255`). A pure function, correctly kept lamad-agnostic.
- **The head endpoint and its client.** `/epr-head/{id}` is anonymous-safe by design
  and `resolveEprHead()` already speaks it. I1 routes onto existing capability.
- **The route-claim vocabulary.** `RouteClaim` (executable) and `RouteClaimTemplate`
  (the *serializable* shape "bundle manifests DECLARE and commitment grants carry")
  already exist (`epr-ref.ts:161-193`). I3 lands the declaration home the template
  type was designed for.

The fix is consolidation and correct routing, not replacement.

## 3. P2P design-gate record (walked 2026-07-02)

No new DHT entry types, no new HTTP routes, no wire-format change. Resolution is a
**projection concern**, not a truth concern.

| Entity / change | Class | Source of truth | Notes |
|---|---|---|---|
| `EprHead` | **A** (notarized, CID-addressed, DHT truth; SQLite projection) | existing — DHT / storage projection | I1 shifts preview consumers onto the existing anonymous-safe `/epr-head/{id}` read; it **removes** a REST-body-as-resolution anti-pattern. No new entity or route. |
| Route-claim declaration | schema **vocabulary** (not a runtime entity) | app manifest `elohim/sdk/domains/*/manifest.json` | Declared once; TS (`RouteClaimTemplate[]`) + Rust (doorway mount map) consumers **generated**. Single-homes W4. |
| `EprResolutionProvider` + degradation states | code abstraction only | n/a | No persistence, no wire change. A client-surface interface over existing reads. |

**Load-bearing constraints (gate output):**
1. The **forbidden-vs-missing** distinction is load-bearing for the acceptance
   scenario (§6) — the element MUST tell them apart.
2. The provider resolves **content addresses only**; `agent_cid` / canonical-identity
   rules (`elohim-storage` §Identity coherence) are untouched — a resolver never joins
   or mints an agent identity.

## 4. The design

### I1 — Head-first resolution + typed degradation

**Resolver.** Previews and links resolve via `/epr-head/{id}` (already implemented as
`resolveEprHead()`); the body (`/db/content/{id}`) is fetched **only** on click-through
or embed. `EprResolverService.resolve()` is split so the preview path returns a head
projection and never touches the reach-gated body.

**Element.** `<elohim-epr-link>` gains four typed states, replacing the single
`unreachable` boolean. `resolve()` maps its resolver's outcome onto them:

```ts
// elohim-epr-link.ts — replaces `unreachable?: boolean`
export type EprLinkState =
  | 'resolved'    // head + body reachable → full card face
  | 'head-only'   // head resolved, body not yet fetched (preview default)
  | 'forbidden'   // head resolved, body 403 at this reach →
                  //   render head TITLE + honest "content unavailable at your reach" affordance
  | 'missing';    // head 404 / unresolvable → the only state that may fall back to the raw ref

export interface EprLinkResolution {
  state: EprLinkState;
  title?: string;
  description?: string;
  pillar?: string;
  reach?: string;
  preview?: { title?: string; body?: string };
}
```

- `forbidden` ≠ `missing`: the former has a head (title + reach known) and renders a
  legible, honest affordance; only the latter may render the raw `epr:` string.
- `display="card"` becomes real: `render()` branches on `display` × `state` rather than
  load-level alone (closing the cosmetic-prop half of W3).
- The `.resolver` contract widens from `Promise<EprLinkResolution | null>` to a result
  that carries `state`, so the host distinguishes 403 from 404 — the element cannot
  invent the distinction the transport must report.

I1 is **standalone-valuable** and ships without I2/I3 (see §5).

### I2 — The ambient `EprResolutionProvider`

One contract, provided once per surface, consumed by all four W1 paths:

```ts
export interface EprResolutionProvider {
  /** Preview/link resolution — reach-safe, anonymous-OK; hits the head plane. */
  resolveHead(ref: string): Promise<EprHeadResolution>;      // → resolved | head-only | forbidden | missing
  /** In-bundle vs cross-bundle route minting for a claimed ref (wraps eprToRoute). */
  resolveRoute(ref: string, contentType?: string): EprRouteResolution | null;
  /** Body fetch — click-through/embed only; may 403; caller renders the forbidden affordance. */
  resolveBody(ref: string): Promise<EprBodyResolution>;
}
```

- **Angular:** an `InjectionToken<EprResolutionProvider>` provided in each bundle's
  `app.config`, with that bundle's `BUNDLE_ROUTE_CONTEXT` baked in — so `resolveRoute`
  is claims-correct per bundle without each call site re-injecting the context.
- **Lit:** a `@lit/context` `ContextProvider` installed at the shell root — the **same
  root** where `app.component` installs the click interceptor — so every
  `<elohim-epr-link>` in the tree consumes the provider through context instead of the
  host hand-setting `.resolver` per element.
- **Transport abstraction:** the provider is the single seam where transport is chosen
  — doorway HTTP in the browser, the local storage sidecar (`:8090`) in Tauri,
  head-gossip later — **content addresses only; agent identity untouched.** The four
  W1 paths become thin: they consume the provider and render; they no longer each own
  a transport decision.

The interceptor (W1 path 4) stays as the capture-phase safety net for
content-authored/legacy anchors — but its handoff routes through the provider too, so
"what does resolving mean" has exactly one answer.

### I3 — Manifest-declared route claims

Bundle `contentType` route claims are declared in the app manifests
(`elohim/sdk/domains/*/manifest.json`) as `RouteClaimTemplate` entries — the existing
manifest→codegen governance pattern (the same shape `schema:codegen:ts` /
`schema:codegen:rs` already drive):

```jsonc
// elohim/sdk/domains/lamad/manifest.json  (illustrative)
"routeClaims": [
  { "contentType": "path", "template": "/path/{id}", "fragments": { "step": "/path/{id}/step/{n}" } }
]
```

Both consumers are **generated**, never hand-maintained:

- **TS:** the `RouteClaimTemplate[]` fed to `claimsFromDeclaration` for each bundle's
  `BUNDLE_ROUTE_CONTEXT` (replacing the hand-written `useValue` in `app.config`).
- **Rust:** the doorway `EprRouter`'s `contentType → mount` binding map.

One home (the manifest), two generated consumers — W4's drift class is structurally
gone. The shell's "claims nothing, owns `/epr`" posture becomes a declared manifest
fact rather than an inline literal.

## 5. Sequencing — and why I1 stands alone

**I1 → I2 → I3**, and I1 is the standalone increment because:

- It heals the live symptom (dead manifesto chip → legible `forbidden` card) using
  **only** capability that already exists (`resolveEprHead`, the anonymous `/epr-head`
  endpoint) — no new provider plumbing, no manifest/codegen work.
- It is the a2o-acceptance-bearing increment (§6): the `@wip` scenario passes on I1
  alone.
- I2 then consolidates the four wirings onto the head-first contract I1 defined
  (moving the *how* into one provider without changing the *what*).
- I3 is orthogonal to both — it can land before or after I2 — and closes W4 on its
  own timeline.

Ship order is a value ranking, not a dependency chain past I1: I1 is worth doing even
if I2/I3 never follow.

## 6. Acceptance criteria

Two **independent** fixes converge on the live manifesto card; neither is the other.

1. **The element degradation is structurally correct (this spec, I1).** The `@wip`
   a2o scenario *"A reference card whose body cannot be reached still renders a legible
   fallback"* (`genesis/a2o/features/content/landing-discovery.feature`, currently
   `@wip @browser-only @requires:seeded-content`) passes: the manifesto reference card
   renders a legible fallback carrying **its title** and an honest **"content
   unavailable"** affordance, and **no reference card on the doorstep is left blank**.
   Hard invariant: **no raw `epr:` string ever renders as a card face** — `missing` is
   the only state permitted the raw ref, and a resolvable head forbids that path.
2. **The live manifesto body is served (separate, alpha reach-drift fix).** The
   `403` on anonymous `/db/content/manifesto` is a seed/reach verdict on the row
   (`alpha-manifesto-content-403.md`), fixed on the substrate/seed side — **not** by
   this spec. When both land, the card resolves fully (`resolved`); with only (1), it
   degrades honestly (`forbidden`); with only (2), the untyped collapse is still latent
   for the next community-reach ref. They compose; they are not substitutes.

I2 acceptance: all four W1 paths consume the single provider; removing any one call
site's bespoke resolution wiring leaves no behavior gap. I3 acceptance: the TS claim
tables and the doorway mount map are byte-generated from the manifests, and a manifest
route-claim edit reflects in both consumers with no hand edit (verify generated-output
freshness the way the schema codegen gate already does).

## 7. Non-goals

- **No new DHT entry types.** `EprHead` is existing Category A truth; this is a
  consumer re-route.
- **No blob-plane changes.** Body/blob fetch, content addressing, and the `/blob`
  path are untouched; only *when* the body is fetched moves (click-through, not
  preview).
- **No identity or agent routing.** The provider resolves content addresses only;
  `agent_cid` canonical-identity, transport-identity coherence, and reach *earning*
  are out of scope. This spec consumes the reach verdict; it never computes or mints
  one.
- **No reach-enforcement change.** The `403` semantics of `/db/content/{id}` stay
  exactly as they are — I1 stops *asking the wrong endpoint*, it does not relax the
  gate.
- **No new omnibar/chrome work.** The predecessor spec owns chrome; this is strictly
  the resolution layer under it.

## 8. Relationship to prior art

This spec is the **resolution-layer successor** to
`2026-06-05-omnibar-consolidation-epr-native-links-design.md`. That design settled the
two link classes, the `/epr/{id}` universal address, and the capture-phase interceptor
— the *navigation* seam. It left resolution itself as four independent wirings against
the body endpoint with untyped degradation. This spec completes its other half: one
ambient resolution contract, head-first previews, typed degradation, and a single
manifest home for the route claims that design's `BUNDLE_ROUTE_CONTEXT` introduced.

It also inherits the **trust-tier legibility** posture of
`2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md`: "heads
flow freely, bodies are gated" is the read-plane echo of "the notary adds trust as a
late overlay, never a precondition." The four element states (`resolved` / `head-only`
/ `forbidden` / `missing`) are the person-facing projection of that same
freely-witnessed-metadata / gated-payload split — honesty at the card face, the way
that spec asks for honesty at the trust tier.
