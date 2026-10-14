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
  top-level page in Chromium lands on `/epr/<id>`, which shows the node's chrome (reach, holders,
  steward, governance) with the bundle framed inside it; the same address requested as an iframe
  (the lamad renderer, the EPR focal) still returns the bundle's bytes; and an app bundle that is
  itself a chromed EPR app (spa-bundle, e.g. elohim-host-landing, lamad-spa) is served unchanged.
cites:
  - "elohim-seam-map-concern-routing | places app-serving and render runtimes peer-side and keeps the doorway to web2 delivery; this spec's navigation redirect is delivery behaviour in that seam | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
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
every request (Fetch Metadata):

1. A request to `/apps/<slug>/<entryPoint>` with `Sec-Fetch-Dest: document` and
   `Sec-Fetch-Mode: navigate` is a person opening the content as a page. For an **html5-app**
   content node, the doorway answers `303 See Other` to `/epr/<content id>`, preserving the
   query string as a hint the EPR page may use. This is delivery behaviour (which document a
   navigation receives), so it sits in the doorway's web2 seam, beside the reach check the
   `/apps/` arm already runs.
2. Requests with `Sec-Fetch-Dest: iframe`, `frame`, `embed` or any subresource destination,
   and every request for a non-entry path (scripts, styles, fonts, images), are served as today,
   byte for byte. The doorway never adds chrome to an embedded response: a surface that embeds
   content (lamad, the EPR page) already implements the chrome's primitives and links itself,
   and a second frame of chrome inside it would be noise. Only a bare top-level visit is sent to
   the chromed view.
3. An app bundle that carries its own chrome (`spa-bundle` content such as elohim-host-landing
   and lamad-spa, whose root renders `<elohim-page-chrome>`) is served unchanged: it is already
   a chromed surface.
4. A request without Fetch Metadata (older clients, tools) is served as today. The rule
   qualifies what people see; it is not an access control, and the reach check stays the gate.

The content id comes from the same storage lookup the slug route already performs, so the
doorway learns nothing new and holds no policy of its own: whether an app is "content" or
"chromed shell" is its declared `contentFormat`.

## Links

Repository docs, READMEs and outward letters link to `/epr/<id>`, never to `/apps/…` directly.
The DTPR letter's README is the first to follow this.

## Debt

- The redirect itself is not built. Home: doorway `routes/apps.rs`, with the a2o station named
  in the graduation trigger added to `features/dataplane/epr-app-deliverability.feature`.
- `fallbackUrl` remains a field on html5-app content; whether a protocol-served node should
  carry an outside fallback at all is open.
