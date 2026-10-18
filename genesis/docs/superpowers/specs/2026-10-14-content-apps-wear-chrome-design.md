---
title: Content apps wear chrome — a doorway never serves an html5-app bare to a visitor
id: content-apps-wear-chrome-design
status: Draft
class: architecture
serves: doorway-failover
date: 2026-10-14
context-tier: disclosed
steward: human:matthew
graduation-trigger: >
  On the household mesh, opening `/apps/<slug>/<entryPoint>` of an html5-app content node as a
  top-level page in Chromium shows the app itself under a thin protocol chrome whose resiliency
  badge links to `/epr/<id>`; the same address requested as an iframe (the lamad renderer, the EPR
  focal) returns the bundle's bytes unchanged, with no chrome added; and an app bundle that is
  itself a chromed EPR app (spa-bundle, e.g. lamad-spa, imagodei-portal) is served unchanged.
cites:
  - "elohim-seam-map-concern-routing | places app-serving and render runtimes peer-side and keeps the doorway to web2 delivery; this spec's navigation shell is delivery behaviour in that seam | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# Content apps wear chrome

## The principle

Nothing should be seen through a doorway on the Elohim Protocol without the minimal chrome that
qualifies it: what it is, who holds it, at what reach, who stewards it and how it is governed.
A reader who follows a link to a piece of content meets it inside that frame, never as a bare
page that could have come from anywhere.

The raw address still has a job. Embeddings inside the protocol's own surfaces (the lamad
iframe renderer, the EPR page's focal frame) load the bundle from `/apps/<slug>/<entryPoint>`
and rely on it being available. So the raw bytes stay reachable as an embedded resource; what
changes is what a visitor gets when they open that address directly as a page.

## What happens today

- `GET /apps/<slug>/<path>` (doorway `routes/apps.rs`) serves the file from the html5-app zip
  for every caller, after the reach check. Opened directly, an html5-app renders bare.
- `/epr/<id>` (elohim-app `app.routes.ts`, the universal route) renders the chrome and frames the
  bundle through the lamad iframe renderer.
- Found 2026-10-14 and fixed in the same change as this spec: the renderer treated the
  same-origin doorway base (the empty string) as "no doorway configured" and framed the node's
  `fallbackUrl` instead of the bundle. Every html5-app with a fallback framed an outside copy
  (`evolution-of-trust` showed ncase.me; the DTPR letter showed a claude.ai page that refuses
  framing, so the frame was blank). The fallback now only feeds the "open elsewhere" link.

## Design

The doorway decides by the browser's own declaration of intent, which it already receives on
every request (Fetch Metadata), and never changes the content's bytes:

1. A request to `/apps/<slug>/<entryPoint>` with `Sec-Fetch-Dest: document` and
   `Sec-Fetch-Mode: navigate` is a person opening the content as a page. For an **html5-app**
   content node, the doorway answers with a minimal chrome shell instead of the bare entry
   file: `<elohim-page-chrome>` with its omnibar, a **resiliency badge** for the node (how many
   households hold it, from the same resilience view the EPR page reads) that **links to
   `/epr/<id>`**, and the app itself filling the rest of the viewport in a frame whose source
   is the same `/apps/<slug>/<entryPoint>` address. The person stays on the app; the chrome is
   one thin bar; the full EPR page (reach, holders, steward, governance) is one click away.
2. Requests with `Sec-Fetch-Dest: iframe`, `frame`, `embed` or any subresource destination,
   and every request for a non-entry path (scripts, styles, fonts, images), are served as today,
   byte for byte. That includes the shell's own frame, so the app inside it loads raw. The
   doorway never adds chrome to an embedded response: a surface that embeds content (lamad, the
   EPR page) already implements the chrome's primitives and links itself, and a second frame of
   chrome inside it would be noise.
3. An app bundle that carries its own chrome (`spa-bundle` content such as lamad-spa and
   imagodei-portal, whose root renders `<elohim-page-chrome>`) is served unchanged: it is already
   a chromed surface.
4. A request without Fetch Metadata (older clients, tools) is served as today. The rule
   qualifies what people see; it is not an access control, and the reach check stays the gate.

The content id and the badge's figures come from the storage lookups the `/apps/` arm already
performs (slug index, reach check, distribution summary), so the doorway learns nothing new and
holds no policy of its own: whether an app is "content" or "chromed shell" is its declared
`contentFormat`. Because the shell frames the bundle rather than rewriting it, the bytes a
person runs are still exactly the bytes addressed by the node's `blobHash`.

## Decisions taken while building (2026-10-14)

Three points the design left open were settled when the shell was cut into work, each the
narrowest reading that keeps the doorway's existing contracts:

- **The shell's omnibar is the native EPR omni element the doorway already serves**
  (`GET /chrome/omni-element.js`, injected with the same `inject_element` helper the SSR shell
  uses), not the Lit `<elohim-page-chrome>`. The doorway serves no Lit bundle at runtime, and
  the omni element is the chrome every doorway-painted surface already wears. The resiliency
  badge in the shell is a server-rendered link; the reusable `<elohim-resiliency-badge>`
  element in `elohim-core` carries the same wording for the surfaces that bundle Lit (the EPR
  page), so the two never drift in what they say.
- **The doorway learns a slug's content id, format and entry point from its own content
  projection only** (the same Mongo documents its slug index is built from). When that
  projection is cold or absent, the request is served raw, as today. No storage endpoint
  changes, and `_capability` / `X-Deliverability` keep describing bundle readiness, never
  shell success.
- **The chrome branch fires only when the resolved file equals the node's declared
  `entryPoint`** (`index.html` when none is declared). A trailing slash still maps to
  `index.html`; a node whose entry is another file wears chrome at that file's address.

Two consequences that are part of the shell, not afterthoughts: the shell is `no-store` and
both representations of the entry address carry `Vary: Sec-Fetch-Dest, Sec-Fetch-Mode`, so no
cache on the path (the doorway's file cache, the warm shell, a CDN) can hand a shell where
bytes are expected or the reverse; and the client apps service worker, which intercepts every
`/apps/` request under its scope, lets a top-level document navigation reach the network first
and falls back to its cached raw entry only when the network is away, so a direct visit meets
the shell online and the app still opens offline.

## Links

Either address is a fine link once the shell exists: `/apps/<slug>/` opens the content itself
under thin chrome, `/epr/<id>` opens the full page about it. Until the shell is built, docs and
letters link to `/epr/<id>`, as the DTPR letter's README does.

## Debt

- The shell is not built. Homes: doorway `routes/apps.rs` (the navigation branch and the shell
  document), a resiliency-badge element in `app/elohim-elements/elohim-core` that renders a
  holders figure and an `href` (today the badge exists only inside the EPR page), and the a2o
  station named in the graduation trigger in `features/dataplane/epr-app-deliverability.feature`.
- `elohim-host-landing` is seeded as `html5-app` (genesis/data/lamad/content/elohim-host-landing.json;
  landing-page-dogfood.feature asserts it) although its root is a chromed shell. People meet it at
  `/`, which the doorway renders itself, never at its raw `/apps/` address, so the rule above only
  double-chromes a visit nobody is pointed to; still, the honest declaration is `spa-bundle`, and
  reseeding it with the dogfood story updated is open.
- `fallbackUrl` remains a field on html5-app content; whether a protocol-served node should
  carry an outside fallback at all is open.
