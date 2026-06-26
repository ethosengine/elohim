---
title: Native Rust EPR-Agnostic SSR Engine — Omnibar as Runtime Chrome; SSR as One EPR's Nature × Peer Capability
id: native-rust-epr-shell-ssr-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
domain: D8
topic: [ssr, doorway, projection, render, elohim-render, omnibar, epr, theme, content-addressing, native-rust, chrome, v8, render-capability, dataplane-trajectory]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
cites:
  - doorway-ssr-runtime | SSR-as-compute-capability seed (D8); this spec KEEPS its V8 render core for the Angular body, wraps it in a native-Rust EPR-agnostic composition layer + runtime omnibar chrome, and routes the SSR-vs-static decision through the peer RenderCapabilityProfile it already defines | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - elohim-seam-map-concern-routing | Seam placement (D8 projection, Track 4 doorway); the omnibar moves from per-EPR app bundle (SDK seam) to runtime chrome (mod/plugin + projection seam); SSR-capability is EPR content (the "what"), serving is peer participation (the "how") | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - ssr-bundle-substrate-content-decouple-design | Foundation; this spec EXTENDS it — V8 materialize-at-boot is RETAINED — and CORRECTS its two-row model: the server bundle collapses from a sibling `elohim-host-landing-ssr` content row into a field on the one EPR node | path: genesis/docs/superpowers/specs/2026-06-24-ssr-bundle-substrate-content-decouple-design.md
  - tiered-quilt-stewardship-design | Trajectory target (substrate stewardship/replication, D8); the per-host blob-upload + per-host blobHash PATCH this spec relies on is MVP scaffold — gossip carries inventory not bytes, so content-addressed byte replication is what finally makes "alpha vs elohim.host" irrelevant | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
requires_env: [household-nodes]
---

# Native Rust EPR-Agnostic SSR Engine — Omnibar as Runtime Chrome; SSR as One EPR's Nature × Peer Capability

- **Date:** 2026-06-26
- **Status:** Design (approved direction; pre-implementation)
- **Seam:** Doorway projection (atlas §3.9, Track 4) + render runtime (`elohim-render`). The omnibar crosses a seam: per-EPR Angular app bundle (SDK seam) → runtime chrome (mod/plugin + projection seam). SSR-capability is **EPR content** ("what"); serving is **peer participation** ("how").
- **Extends + corrects:** `2026-06-24-ssr-bundle-substrate-content-decouple-design.md`. Its V8 materialize-at-boot is kept; its **two-row** model (a sibling `-ssr` content row) is collapsed.

## 1. Problem

Two symptoms, one root cause, plus a trajectory error.

**Symptom 1 — host divergence.** `elohim.host` served the `8a2c65e` landing bundle (theme-dead omnibar); `alpha.elohim.host` served `580b88d` (theme-responsive). Trigger (App #1560): the `elohim-host-landing-ssr` PATCH 404'd on alpha (that **second** content row was never seeded) and a single loop-wide `catchError` aborted the publish loop before the `elohim.host` iteration ran. Fixed tactically by per-`(host,slug)` isolation (`Jenkinsfile` `stageSpaBlobs`, `2a09234c7`).

**Symptom 2 — the omnibar is the wrong kind of thing.** `app-protocol-omni` ships **inside each EPR's Angular app bundle**, with hardcoded light/dark RGBA and a build marker baked from `environment.gitHash`. Coupled to the per-host bundle version ⇒ a stale bundle = a theme-dead omnibar. It also renders **client-side only** (needs router context + async config), so it paints after hydration, never in the SSR HTML. Nav-context carries **no theme declaration** — an EPR cannot express its own palette.

**Root cause.** The omnibar is **chrome that wraps every EPR** but is authored, versioned, and distributed as **per-EPR application code**. Chrome that frames all EPRs must be **runtime-owned and EPR-agnostic**, themed by the wrapped EPR's declared theme.

**Trajectory error (the deeper one).** SSR was modeled by *mirroring the browser bundle's per-host PATCH* into a **second content row** (`elohim-host-landing-ssr`) — the relational/per-row reflex: a facet of one EPR turned into a sibling row keyed per host. That fights the protocol's direction. SSR-capability is part of **one EPR's nature** (it has a server bundle, it can be SSR'd); whether SSR *happens* is the serving **peer's advertised capability** (RAM/V8/config). The runtime **already** models the peer half (`RenderCapabilityProfile` + CSR fallback); the second row is the redundant artifact fighting it.

## 2. Decision

> **A native-Rust, EPR-agnostic SSR engine, with SSR modeled as `EPR-nature × peer-capability`.**
> 1. The render runtime becomes a Rust host, generic over EPRs, that renders the **shell + omnibar chrome natively** (no V8, themed from the EPR's declared tokens) and **drives V8 to SSR the Angular body**, composing the two.
> 2. The **one** `elohim-host-landing` EPR node carries its **full nature** as content — browser bundle ref, **server bundle ref**, an **SSR-capable** marker, and the **theme**. The sibling `-ssr` row, `SSR_BUNDLE_SLUG`, and the `kind:server` PATCH collapse away.
> 3. **Serving stays peer-capability-gated** by the existing `RenderCapabilityProfile` + render semaphore + CSR-shell fallback: SSR iff the EPR is SSR-capable **and** the peer can + is configured to; else serve the browser bundle.

This is the **"keep V8" fork** (server-rendering Angular needs a JS engine) **plus** the **collapse-the-row correction** (operator direction, 2026-06-26).

Consequences (all intended):
1. **Omnibar → EPR-agnostic runtime chrome** — server-rendered, themed by the wrapped EPR, identical across hosts/bundles, independent of any app-bundle version (and gains server-side first-paint).
2. **The two-row drift class is gone** — one node, no sibling row that can be missing ⇒ Symptom 1's 404 trigger cannot recur.
3. **V8 + the server bundle are retained** — full body SSR (first-paint + SEO) preserved.

## 3. Goals / Non-goals

**Goals**
- Native-Rust EPR-agnostic composition engine: native chrome + V8 body, composed.
- Omnibar leaves the Angular app bundle; server-rendered; an app-only change can never again theme-kill it.
- One EPR node declares its full renderable nature (browser ref + server ref + SSR-capable + theme); the `-ssr` row is removed.
- Serving gated by the **existing** peer `RenderCapabilityProfile` (stop fighting it with content rows).
- Full body SSR preserved (V8 retained).

**Non-goals**
- Retiring V8 / Angular SSR (the body needs a JS engine).
- Native-Rust rendering of the Angular **body** (Rust can't execute Angular; content-EPR native render is the §9 extension).
- **Byte replication / true peer-agnosticism.** Blobs still don't auto-replicate (§4.7); per-host publish remains MVP scaffold here. That is the predecessor-trajectory spec `2026-05-11-tiered-quilt-stewardship-design.md`, explicitly out of scope.
- Any new DHT entry type, table, or migration (§6).

## 4. Architecture

### 4.1 The composition engine (`elohim-render`)

`elohim-render` keeps its V8 `AngularRenderer` and gains a **native composition layer**. The doorway's existing per-route dispatch (`AppState.renderer: Option<Arc<dyn Renderer>>`, `http.rs:209`; `classify_dispatch` `http.rs:76-113`; SSR arm `http.rs:3459-3600`) is refactored to flow through a composing renderer that, per request (EPR-agnostic):

1. **Fetches EPR data** (already reachable): the EPR content node (slug, browser `blobHash`, **server bundle ref**, SSR-capable, `metadata.theme`) + the `EprNavContextView` projection (resilience tier, prev/next).
2. **Renders chrome natively** (Rust, no V8): `<head>` (title/description/OG/canonical), a scoped `<style>` binding the EPR's theme tokens to CSS custom properties, and the omnibar markup.
3. **Drives V8 to SSR the Angular body** (unchanged path): `AngularRenderer` runs the server bundle → `<app-root>` inner HTML + hydration state.
4. **Composes:** native shell with the V8 body in `<app-root>`, `with-omni` padding on the shell, the browser-bundle `<script>` (hydration), and `omni-enhance.js` (§4.5).

```html
<!DOCTYPE html>
<html lang="en" data-color-scheme="{colorScheme}">
  <head> <title>{epr.title}</title> {meta/og/canonical}
    <style>:root{ --epr-omni-bg:{t.bg}; --epr-omni-fg:{t.fg}; --epr-omni-accent:{t.accent}; ... }</style>
  </head>
  <body class="with-omni">
    {NATIVE Rust omnibar chrome — themed via the vars above}
    <app-root>{V8-SSR'd Angular body}</app-root>
    <script src="/blob/{browser-bundle-hash}/main.mjs" type="module"></script>
    <script src="/chrome/omni-enhance.{hash}.js" defer></script>
  </body>
</html>
```
The omnibar is outside `<app-root>`, so Angular hydration (scoped to `<app-root>`) is unaffected.

### 4.2 SSR-vs-static is already peer-gated — stop fighting it

The decision the runtime already makes (`http.rs:3459-3600`): the route is `render:"angular-ssr"`-eligible **and** a renderer is loaded **and** the `render_semaphore` (sized from `RenderCapabilityProfile.max_concurrent_renders`) has a permit **and** the peer supports the auth mode → SSR; otherwise serve the browser bundle (CSR shell, `x-ssr-skipped`). That **is** `EPR-capable × peer-capable`. This spec stops opposing it with a second content row; the EPR's SSR-capability becomes a field the manifest's `render` eligibility derives from, and the peer `RenderCapabilityProfile` (`render/types.rs`, `render/capability.rs`) remains the serving authority.

### 4.3 One EPR node carries its full nature

The `elohim-host-landing` content node (`db/content/{slug}`) gains, alongside its existing `blobHash` (browser) and `metadata`:
- a **server bundle reference** (`metadata.serverBlobHash`, written by the deploy PATCH like `blobHash`);
- an **SSR-capable** marker (presence of a server bundle ⇒ capable; or `metadata.ssr.capable`);
- the **theme** (`metadata.theme`: `colorScheme` + tokens; absent ⇒ base palette — the current omnibar default RGBA lifted verbatim).

The sibling `elohim-host-landing-ssr` content node, the `SSR_BUNDLE_SLUG` env, and the `kind:server` second PATCH are **removed**. `init_renderer` resolves the server bundle from the one node's `serverBlobHash` (not a separate slug). `nav-context` stays nav-only (+ `resilience_tier`).

### 4.4 (covered by 4.2) — serving authority is the peer profile

### 4.5 Behavior via progressive enhancement

The omnibar's interactivity (expand/collapse resilience panel, theme toggle + persist, account href, prev/next nav) ships as one small **static, content-addressed** script (`/chrome/omni-enhance.{hash}.js`) served by the runtime — **not** in the Angular bundle. Server renders the collapsed bar as functional markup (links work with zero JS); the script enhances on load.

### 4.6 Migration of the Angular app

- Delete `app/elohim-app/src/app/elohim/components/protocol-omni/` (+ the `/deliver` `protocol-omnibar` sibling) and its use in `app.component.{ts,html}`.
- `with-omni` padding moves to the runtime shell; drop the `environment.gitHash` omnibar coupling (marker is runtime-sourced from the served `blobHash`).
- Net: both the browser **and** server bundles shrink; omnibar changes stop re-triggering app builds and stop riding either bundle.

### 4.7 Trajectory note — per-host publish is MVP scaffold

Today blobs do **not** auto-replicate (`Jenkinsfile:328-331`: "the blob does not auto-replicate P2P and the blobHash PATCH is a per-storage write"; gossip carries inventory, not bytes). So the slug→blobHash pointer is **per-host mutable state**, and the pipeline publishes to both hosts. **Collapsing the row does not, by itself, make the EPR identical on every peer** — it removes the two-row drift + the 404 class, but per-host publish remains. That per-host model is **MVP convenience on the way to the synchronized P2P dataplane** (`2026-05-11-tiered-quilt-stewardship-design.md`), which is what finally makes "alpha vs elohim.host" irrelevant. Until then, Track A's per-`(host,slug)` isolation (`2a09234c7`) keeps one host's failure from stranding the other. **Do not model new per-host rows; aim at the single-node, content-addressed target and let the dataplane layer close the replication gap.**

## 5. Data flow

```
author EPR content (browser ref + serverBlobHash + ssrCapable + theme)  ──seed/PATCH (per host, MVP)──▶ db/content/{slug}
                                                   │   (ONE node — no sibling -ssr row)
request /  ──▶ doorway EprRouter ──angular-ssr──▶ Composition engine (Rust host)
                       ├─ native: shell + themed omnibar chrome                    (no V8)
                       └─ if (EPR ssrCapable) AND (peer RenderCapabilityProfile ok + semaphore free):
                            V8 AngularRenderer runs server bundle -> body          (V8)
                          else: serve browser bundle (CSR shell, x-ssr-skipped)
                                                   ▼
            <native shell> <omnibar/> <app-root>{V8 body | CSR}</app-root> <browser script> <omni-enhance>
```

## 6. P2P design gate record

The data-bearing change adds three fields (`serverBlobHash`, SSR-capable marker, `theme`) to the **existing** EPR content node and **removes** a sibling row. Gate:
1. **Entity class?** Operational metadata on an existing content-addressed EPR node — Category C (mirrors `EprNavContextView`). The change *reduces* entity count (two nodes → one).
2. **DHT entry type exists?** Yes — the EPR content node already exists; these are fields on it. **No new entry type.**
3. **Identity?** The EPR's identity is its content-derived CID/slug; the server bundle + capability + theme are facets of its content. **No new identity** (and the collapse *removes* the redundant `-ssr` slug-identity).
4. **Coordinator fn / signal?** Authored at content-seed time; the server `blobHash` written by the deploy PATCH (as today, just onto the one node); read at render time. **No new coordinator function, signal, table, or migration** (`metadata` is JSON).

No `GET /api/v1/thing` is added: the engine consumes the existing content-row fetch + nav-context projection.

## 7. Tradeoffs

- **V8 stays** — full body SSR preserved (the "keep V8" fork). Cost: V8 cold-start, the server bundle, the isolate budget remain. Accepted.
- **Two render paths in one engine** — native chrome (Rust) + body (V8) must compose; omnibar outside `<app-root>` keeps hydration clean. Golden-HTML tests cover composition (§10).
- **Per-host publish remains** (§4.7) — collapsing the row kills the drift + 404 class but not the replication gap; the dataplane spec is the completion. Honest scaffold, named.
- **Reverses a prior choice** — the predecessor chose two rows deliberately; the reversal is justified (upsert covers the PATCH-target reason; a field-not-served-slug covers isolation; one row still yields per-host signals).

## 8. Rollout sequencing (each step independently shippable)

1. **Immediate sync is already handled, independent of this arc:** Track A (`2a09234c7`) hardens the per-host publish, so elohim.host's **browser** bundle (the client-rendered omnibar) re-syncs on the next `dev` merge — no need to wait for the rest.
2. **Collapse the rows:** add `serverBlobHash` + SSR-capable to the `elohim-host-landing` node; PATCH one node with both hashes; remove the `-ssr` seed, `SSR_BUNDLE_SLUG`, and `kind:server`; `init_renderer` reads `serverBlobHash` from the node. (This alone deletes the perpetual `-ssr` 404.)
3. Add `metadata.theme` + base-palette fallback; seed `theme` for `elohim-host-landing`.
4. Build the native chrome renderer + the composition engine wrapping `AngularRenderer`; golden-HTML tests (chrome: theme × nav × colorScheme; composition: chrome + stub body).
5. Route the landing `angular-ssr` dispatch through the composing engine; confirm serving stays gated by `RenderCapabilityProfile`.
6. Remove `app-protocol-omni` from the Angular bundle; ship `omni-enhance.js`.
7. Verify on alpha **and** elohim.host in **both** themes (the originating defect) via `pnpm look` + the theme probe.

## 9. Future extension (named, out of scope)

- **Native content-EPR rendering.** For EPRs whose content is renderable substrate (markdown/html, not an Angular app), the same EPR-agnostic engine can render the **body** natively in Rust (no V8 for those) — a per-EPR choice the dispatch already supports.
- **Synchronized P2P dataplane** (`2026-05-11-tiered-quilt-stewardship-design.md`) — byte replication that makes the per-host publish (§4.7) obsolete and "alpha vs elohim.host" truly irrelevant. The completion of this spec's trajectory.

## 10. Testing

- **Unit (Rust):** golden HTML for native chrome (theme present/absent, colorScheme light/dark/auto, nav present/absent, resilience tiers); golden HTML for the composed document (chrome + stub body); the `EPR-capable × peer-capable` branch (capable+capable → SSR; capable+incapable-peer → CSR fallback).
- **Contract:** the `metadata.theme` + `serverBlobHash` shapes (schema + fixture validity); base-palette fallback when theme absent; one-node resolution (no `-ssr` slug).
- **a2o visual regression:** omnibar themed correctly across light/dark on **both** hosts (the originating defect), via the render + theme-probe rails.
- **Integration:** `angular-ssr` dispatch flows through the composing engine; the V8 body renders when the peer is capable, CSR shell when not; `omni-enhance.js` wires expand/toggle; hydration of `<app-root>` is unaffected by the surrounding chrome.

## 11. Relationship to the predecessor spec

`2026-06-24-ssr-bundle-substrate-content-decouple-design.md` made both bundles content-addressed substrate artifacts materialized at boot. **V8 materialize-at-boot is retained.** This spec (a) adds the native, EPR-agnostic composition layer, (b) lifts the omnibar out of the app bundle into runtime chrome, (c) introduces the EPR theme declaration, and (d) **corrects the predecessor's two-row model** — the server bundle collapses from a sibling `-ssr` content row into a field on the one EPR node, which dissolves the per-host `-ssr` 404 that this incident exposed. The remaining per-host blob publish is named MVP scaffold toward `2026-05-11-tiered-quilt-stewardship-design.md`.
