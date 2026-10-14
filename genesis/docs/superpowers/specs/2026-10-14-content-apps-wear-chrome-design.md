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
  itself a chromed EPR app (spa-bundle, e.g. elohim-host-landing, lamad-spa) is served unchanged.
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
3. An app bundle that carries its own chrome (`spa-bundle` content such as elohim-host-landing
   and lamad-spa, whose root renders `<elohim-page-chrome>`) is served unchanged: it is already
   a chromed surface.
4. A request without Fetch Metadata (older clients, tools) is served as today. The rule
   qualifies what people see; it is not an access control, and the reach check stays the gate.

The content id and the badge's figures come from the storage lookups the `/apps/` arm already
performs (slug index, reach check, distribution summary), so the doorway learns nothing new and
holds no policy of its own: whether an app is "content" or "chromed shell" is its declared
`contentFormat`. Because the shell frames the bundle rather than rewriting it, the bytes a
person runs are still exactly the bytes addressed by the node's `blobHash`.

## Links

Either address is a fine link once the shell exists: `/apps/<slug>/` opens the content itself
under thin chrome, `/epr/<id>` opens the full page about it. Until the shell is built, docs and
letters link to `/epr/<id>`, as the DTPR letter's README does.

## Debt

- The shell is not built. Homes: doorway `routes/apps.rs` (the navigation branch and the shell
  document), a resiliency-badge element in `app/elohim-elements/elohim-core` that renders a
  holders figure and an `href` (today the badge exists only inside the EPR page), and the a2o
  station named in the graduation trigger in `features/dataplane/epr-app-deliverability.feature`.
- `fallbackUrl` remains a field on html5-app content; whether a protocol-served node should
  carry an outside fallback at all is open.
