# Doorway SSR Runtime Design

**Date:** 2026-05-07
**Status:** Draft — design approved through brainstorming, awaiting implementation plan
**Owners:** doorway / elohim-storage / elohim-app

## Why this exists

Anyone fetching an Elohim Protocol URL without running JavaScript receives an empty
`<app-root>` shell. AI design tools (claude.ai/design), social-card crawlers, search
engines, accessibility-without-JS clients, and AI agents observing the live system all
see the same blank shell. Five capabilities the protocol cares about are blocked by
this one gap: design-tool feedback loops, social link previews, search indexing,
graceful degradation, and AI observability of the running system.

Server-side rendering closes the gap. The architectural question is *where* it lives.
Bolting a Node SSR sidecar onto a mostly-Rust gateway introduces a Node ops surface
the protocol doesn't otherwise need. A headless-browser proxy doesn't fix social cards
or search. The remaining option — and the one this spec commits to — is a Rust
runtime that embeds a JavaScript engine, exposed as a library, consumed by the parts
of the protocol that already serve HTTP.

This design lands SSR as a *protocol primitive*: a library that any peer with the
hardware to support it can embed, used today for Angular rendering and shaped to
fulfil REA compute commitments later. It does not introduce a SaaS layer, a separate
deployment platform, or a Node runtime.

## P2P Design Gate disposition

This design introduces **no new DHT entry types**. The work is entirely in the
projection layer (web2 surface) and the operational layer (Rust runtime).
Disposition of every entity, schema, and route the design touches:

| Surface | Category | Source of truth | Why no new entry type |
|---|---|---|---|
| `ContentNode` (rendered by SSR) | **A — DHT-notarised** | Existing lamad DNA entry type | Already exists. SSR projects it. |
| Render output HTML | **C — operational projection** | None — derived from inputs | Pure function of (content, render-spec). Cacheable, regeneratable, never authoritative. |
| Render-result cache entry | **C — operational projection** | None — content-addressable cache | Keyed on input content hashes; invalidated when inputs change. |
| `Route.render` field | **Config schema, not entity** | Existing `crates/doorway-client/src/routes.rs` manifest | Extension to existing routing config. Not a DHT type. |
| `/lamad/concept/{id}`, `/lamad/path/*`, `/` (SSR-eligible) | **Routes serving existing entities** | `ContentNode`, `Path` — already in lamad DNA | These are existing logical routes; the SSR change adds rendering, not a new entity. |
| `/spa/*` (storage Path B) | **Route serving existing entities** | `ContentNode` | Same as above — peer-to-peer projection of content already on the DHT. |
| `RenderContext`, `RenderOutput`, `FetchRequest`, `FetchResponse`, `ContentRef`, `RenderLimits`, `RenderSpec`, `RenderError` | **Rust API types** | None — internal library contract | These are in-process API types, not wire-format types beyond a peer's own boundary. They never hit the DHT. |

**Lamad DNA entry-type headroom is not consumed.** Per the auto-memory entry on
DNA capacity: lamad is at ~73/~100, mishpat at 11/~100. This design adds zero.

**Future work that WILL go through the gate, separately:**

- *REA compute commitments for SSR rendering.* If the runtime later settles
  compute work via REA contracts (the audit-trail framing in the Audit trail
  section), that introduces a new commitment shape — and that spec must clear
  the design gate on its own.
- *Render-output as a content-addressable artifact stored on the DHT.* If
  rendering becomes a peer-shareable artifact (rather than a per-peer projection
  cache), that's a Category A or A2 entity — separate spec, separate gate.
- *Public TLS / DNS / auth surface on elohim-storage for external clients.*
  Already explicitly non-goal here. When that surface lands, it brings new
  routes that may or may not need new entity types depending on what they do
  beyond projection — separate spec, separate gate.

The design's `elohim-render` library is operational infrastructure. The
projection-layer changes are non-entity. Both are inside Category C and do not
require entity classification.

## Reading frame

This spec is load-bearing for several other commitments already in the auto-memory:

- **Three-layer truth model.** DHT is notarised state, libp2p is the data plane,
  doorway is web2 projection. SSR is *exactly* the doorway layer's job. The
  architecture preserves the boundary by making the rendering library callable from
  storage too — not by moving rendering responsibility into the libp2p data plane.
- **Doorway as web2 projection, not a P2P participant.** SSR is the canonical web2
  projection: produce HTML for clients that don't speak the substrate. The library
  is shared because peers running their own storage can also project to web2 directly,
  bypassing the doorway federation hop when they have the capacity for it.
- **Diversity of peers.** SSR is opt-in everywhere except the doorway, which has
  it built-in. Storage peers gain it behind a `--feature ssr` flag because most
  peer hardware (phones, laptops, household devices) can't sustain a V8 isolate.
- **Doorway routes are manifest-driven.** SSR-eligibility lives in the storage
  manifest, not in doorway code. Adding a new SSR-rendered route is a storage
  manifest edit, never a doorway code change.
- **Single-target dispatch.** Doorway forwards each render to a single target
  (in-process renderer, or a single peer's storage). No fan-out. No per-peer
  iteration. SSR follows the same rule as blob serving.

## Architecture

### Crate layout

```
elohim/
  elohim-render/         (NEW LIBRARY — V8 + adapter framework + AngularRenderer)
  elohim-storage/        (existing — gains --feature ssr; embeds elohim-render)

doorway/
  doorway-service/       (existing — depends on elohim-render unconditionally)
```

`elohim-render` is the unit of reuse. Both `doorway-service` and `elohim-storage`
link it directly. There is no separate render-server binary in the MVP. Crash
isolation by process boundary can be added later if it proves necessary; the alpha
cluster does not need it.

### `elohim-render` responsibilities

The library owns three concerns and nothing else:

1. **V8 lifecycle.** Boot, snapshot, teardown of isolates. Memory ceilings, time
   budgets, isolate pool management. Implemented on top of `deno_core`.
2. **Module loader and minimal stdlib shim.** Loads the framework SSR bundle. Provides
   `URL`, `TextEncoder`, `console`, and a `fetch` shim that dispatches to a
   `DataFetcher` trait the embedder supplies. The renderer has no direct network
   access — embedders mediate every fetch.
3. **Adapter framework.** A `Renderer` trait with one MVP impl, `AngularRenderer`.
   The trait is what makes other frameworks (Svelte, Vue, doorway-app's own SSR)
   plug in later without library churn.

The library exposes no HTTP server, no service shape. It is a Rust API.

```rust
pub trait Renderer: Send + Sync {
    fn render(&self, ctx: RenderContext) -> Result<RenderOutput>;
}

pub struct RenderContext {
    pub spec: RenderSpec,                  // "angular-ssr" today
    pub url: String,                       // route being rendered
    pub data_fetcher: Arc<dyn DataFetcher>,
    pub limits: RenderLimits,
}

pub struct RenderOutput {
    pub html: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub fetched_inputs: Vec<ContentRef>,   // for cache invalidation
}

pub trait DataFetcher: Send + Sync {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse>;
}
```

### Engine choice

`deno_core` (V8 + minimal runtime). Snapshot support means cold-start after the first
boot is sub-10ms per render. Statically links V8 (~200MB binary growth on
doorway-service). Largest community of patterns to crib from. Cloudflare's workerd,
Deno itself, Vercel Edge — all V8. The protocol gravitates here for the same reason.

### Deployment shapes

Operator chooses, not architecture:

| Peer profile                       | elohim-storage `ssr` feature | V8 instances per peer |
|------------------------------------|------------------------------|-----------------------|
| Doorway operator (any)             | n/a                          | 1 (in doorway)        |
| Capable storage-only peer          | on                           | 1 (in storage)        |
| Default storage-only peer          | off                          | 0                     |

Doorway operators are server-class hardware by definition. Storage peers run on
phones, laptops, household devices — most can't sustain V8. The flag respects that.

## Data flow

Two render paths live side-by-side.

### Path A — external WebFetch via doorway

External clients (browser, AI design tool, social-card crawler, search engine, AI
agent) hit `https://doorway.host/lamad/concept/X`. Doorway's existing route
registry matches the request. Routes flagged `ssr-eligible` in the storage manifest
dispatch to the in-process `Renderer` instead of forwarding as a JSON proxy. The
renderer's `DataFetcher` calls back through doorway's existing projection-cache
resolver, so SSR data fetches hit the warm cache when it is warm.

### Path B — peer-to-peer direct fetch to a capable storage peer

A peer (or local Tauri, or an internal AI agent that knows the peer's URL) hits
`https://matthew-peer.local:8090/spa/lamad/concept/X`. Storage's HTTP layer (with
`--feature ssr` on) calls its embedded `Renderer`. The `DataFetcher` resolves
locally — storage already has the content. No doorway hop, no inter-process call.

This path is libp2p-routed and internal-facing on the public internet at MVP.
Storage does not gain a public TLS / DNS / auth surface for external clients in
this design. That is a separate spec.

### Route eligibility (manifest-driven)

```
manifest.routes = [
  { path: "/db/content/*",     target: storage,         render: null },
  { path: "/lamad/concept/*",  target: storage,         render: "angular-ssr" },
  { path: "/lamad/path/*",     target: storage,         render: "angular-ssr" },
  { path: "/blob/*",           target: storage,         render: null },
]
```

A new SSR-eligible route is added by editing `build_manifest()` in storage's
`http.rs` to declare `render: "angular-ssr"` on a route group. Doorway picks it up
at next boot. No doorway code change. This follows the existing rule from
`doorway/CLAUDE.md`: registry-driven routes, never per-domain proxy files.

### MVP eligible-routes set

Three route groups gain SSR for MVP:

- `/lamad/concept/{id}` — single content node
- `/lamad/path/{slug}` and `/lamad/path/{slug}/step/{n}` — learning path views
- `/` — landing page

Everything else continues serving the SPA shell with client-side hydration. Auth is
out of scope: only `commons` reach content is rendered. The `RenderContext` carries
a session token slot for future use, but it is `None` for MVP and the renderer
rejects any data-fetch attempt that requires it.

### Cache key and invalidation

`(route_url, fetched_input_hashes, render_spec_version)`. Render results land in
doorway's existing projection cache as a new content kind. When any fetched input's
content hash changes, all cached render outputs that included that input are
evicted. Reuses doorway's existing DHT-signal-driven cache invalidation path —
this is exactly the controller pattern from the auto-memory entry on
elohim-storage as reconciliation controller.

## Angular SSR specifics

### Build-side change to elohim-app

Today: `bootstrapApplication` from `main.ts`, no SSR build target. Adding SSR is a
single Angular CLI affordance: `ng add @angular/ssr`. From its output we keep:

- `src/main.server.ts` — server bootstrap, identical app config plus
  `provideServerRendering()` and `provideClientHydration()`
- `angular.json` `server` build target

We discard `src/server.ts` (the Express harness Angular generates by default).
Express is bound to Node's `http`, and elohim-render is the runtime — we don't
need both. The compiled artifact `dist/elohim-app/server/main.server.mjs` is what
the `AngularRenderer` adapter loads.

### Hydration

Angular 19's standard hydration via `provideClientHydration()` works without custom
work. Angular emits `ngh="..."` markers on rendered nodes; the client picks them up
on bootstrap. The browser bundle is unchanged. Rolling our own hydration protocol
would be a multi-quarter project; using Angular's is a config flag.

### sophia-element — the SSR-unsafe surface

`sophia-element` registers `<sophia-question>` as a custom element via
`customElements.define()` at module load. The UMD bundle assumes browser globals
(`window`, `document`, `customElements`). Executing it during SSR would either
crash or pollute the renderer process across requests.

Strategy:

1. **Schema declaration.** elohim-app declares `<sophia-question>` via
   `CUSTOM_ELEMENTS_SCHEMA` so Angular's renderer treats it as an unknown element
   with attributes preserved — emits the placeholder element literally without
   evaluating it.
2. **Snapshot-time isolation.** elohim-render's snapshot loader does not evaluate
   UMD bundles. Only the Angular SSR bundle loads. Sophia is a client-side asset
   shipped via `<script>` tag; the tag is in the rendered HTML but elohim-render
   does not execute `<script>` content server-side.
3. **Hydration takes over.** When the browser loads the rendered HTML, the
   `<script src="sophia-element.umd.js">` runs, registers the custom element, and
   Angular's hydration upgrades the placeholder.

External clients see the placeholder plus surrounding context, all `<meta>` tags,
the page title, and any rendered text outside the quiz. They do not see the
interactive quiz — quizzes need a runtime, and these clients don't have one.

### Streaming SSR

Out of scope for MVP. Full-render-then-send. Angular 19 supports streaming and
`@defer` blocks; the latency gain matters only for large pages, and we don't have
any yet. Documented as a post-MVP optimisation.

### Render limits per request (configurable)

- Wall time: 2 seconds
- Memory: 128 MB per isolate
- Data fetch fan-out: max 32 calls per render
- Max output size: 2 MB

Time or memory exceeded → 504 with the SPA shell as fallback.

## Error handling

| Mode                       | Cause                          | Response                                 |
|----------------------------|--------------------------------|------------------------------------------|
| Render times out           | Slow data fetch, V8 hot loop   | 504, CSR shell fallback served           |
| V8 OOM                     | Memory budget exceeded         | Isolate killed, 504, CSR shell           |
| Module load failure (boot) | `main.server.mjs` missing      | Boot fails fast — operator must fix     |
| DataFetcher returns 404/5xx| Content gone, storage down     | Render proceeds with placeholder; status reflects upstream |
| Render panics in adapter   | Bug in `AngularRenderer` impl  | Caught, logged, 502, CSR shell           |
| Unknown route              | Not in eligibility set         | Pass through to existing JSON proxy / SPA shell |

The CSR shell stays a first-class fallback. SSR is additive. A peer that loses its
render capability never goes dark — it stops being crawler-friendly until the
issue is fixed. SSR failure must never block the SPA from loading.

## Observability

Native to doorway / storage's existing `tracing` + `tracing-subscriber` stack.
No new exporter, no new dashboard primitive.

- `render.duration_ms` — histogram per (route, status)
- `render.cache.hit` / `render.cache.miss` — counters
- `render.bytes_out` — histogram
- `render.error{kind=timeout|oom|module_load|panic|fetch_failed}` — counter
- `render.isolate_pool.size` — gauge

## Isolate pool and memory ceiling

`elohim-render` maintains a small pool of pre-snapshotted V8 isolates ready for
render requests. Cold-start cost (~100–300ms for an Angular SSR bundle) is paid
once at startup; per-render cost is the snapshot restore (<10ms).

Pool size defaults:

- Doorway (server profile): 4 isolates
- Storage with `--feature ssr` (capacity-constrained profile): 1 isolate

Both are configurable. The library exposes a global `total_memory_budget` for all
isolates combined. When exceeded, new render requests block briefly waiting for an
isolate to free, then fall through to CSR shell. The runtime's failure mode is "no
SSR for this request," not "the gateway dies."

## Audit trail and forward-compatibility with REA compute

Every render's tuple — `(route, content_hashes_fetched, render_spec_version,
output_hash)` — is recorded in storage as a derived artifact. This means:

- **Cache invalidation is content-driven.** When any input hash changes, dependent
  renders evict.
- **Renders are reproducible.** Same inputs + same spec → same output hash.
- **Future REA compute commitments can be settled against this artifact.** A peer
  earned a compute credit by producing this render — auditable, attestable, in the
  shape the substrate already understands.

The MVP does not deliver REA compute settlement. The spec records the audit trail
in a shape that supports it later — future-compatible, never future-foreclosing.
This is the architectural answer to the question of whether the runtime could
become a doorway service contract that funds the substrate from the web2 surface:
yes, eventually, and the design preserves the option.

## Testing strategy

Three layers, mapped to existing project conventions.

### `elohim-render` unit tests (Rust)

Renderer trait conformance: a fake `DataFetcher` returns canned content, the
renderer produces deterministic HTML for a fixed input. Snapshot tests on rendered
HTML. Adapter trait tested with a no-op `EchoRenderer` that returns its input as
HTML — proves the framework-agnostic shape works without Angular in the loop.

### doorway integration tests (Rust + reqwest)

Spin up doorway with the SSR feature, mock storage manifest declares
`/lamad/concept/test-fixture` as ssr-eligible, mock storage returns canned content,
hit doorway with curl, assert the response body contains rendered text and
`ngh="..."` hydration markers.

### a2o scenarios (Gherkin)

The human-experience layer per the project's story-first default. Scenarios live
in `genesis/a2o/features/`:

- "AI design tool fetches a learning concept page and reads its content"
- "Social-card crawler fetches a learning path and gets a rich link preview"
- "Browser fetches a concept page; SSR delivers HTML, hydration upgrades to
  interactive SPA without flashing"

Implementation is done when these scenarios pass against a live cluster. Gherkin
files land in the same commit as the implementation slice that makes them pass.

## MVP — first observable outcome

> `curl https://doorway.host/lamad/concept/elohim-protocol-overview` returns
> fully-rendered HTML with the concept title in the `<title>` tag, the concept
> body in `<article>`, OpenGraph `<meta>` tags populated, and Angular hydration
> markers intact — and the same URL in a real browser hydrates to the live SPA
> without a visible re-render flash.

Two observable artifacts prove the architecture:

- A `curl` body that a human reading the source — or an AI tool, crawler, search
  engine — can understand without running JavaScript.
- A browser session where the SSR'd HTML and the hydrated SPA are visually
  indistinguishable across the boundary.

## Smallest first slice that proves the architecture

A "hello world" SSR slice rendered through the full pipeline:

- Single hardcoded route `/render-test`
- Single hardcoded content fixture
- `deno_core` boots an Angular SSR bundle once
- `AngularRenderer` adapter produces HTML
- doorway dispatches to it
- `curl` sees the rendered body

No caching, no manifest-driven eligibility, no hydration verification, no real
content routes. Just proof the pipe works end-to-end.

Once that slice is green, every subsequent slice is additive:

1. Manifest-driven eligibility (move route declarations from hardcoded to
   manifest-driven)
2. Real content routes (`/lamad/concept/*`, `/lamad/path/*`, `/`)
3. Render-result cache with content-hash invalidation
4. Hydration verification via Playwright (no re-render flash)
5. elohim-storage `--feature ssr` embed for path B
6. a2o scenarios green against a live cluster

Each slice is its own PR onto `dev`. The /shift agentic developer can take this
slice list and grind it.

## Risks

- **deno_core / V8 build cost.** ~200MB binary growth on doorway-service is real.
  CI build time on the doorway pipeline grows materially. Mitigation: `sccache` and
  pre-built V8 binaries; budget an extra 5–10 minutes on first build of the
  pipeline.
- **Angular SSR ↔ deno_core API mismatch.** Angular 19's SSR assumes Node-shaped
  globals (`process`, `Buffer`, parts of `node:fs`). The shim we ship has to cover
  exactly the surface Angular touches and no more. If Angular reaches for something
  unexpected, the renderer fails at module load. Mitigation: the smallest first
  slice exercises the full Angular bootstrap path — failure here surfaces the
  shim gap immediately.
- **sophia-element drift.** If sophia-element's UMD ever gains side-effects beyond
  custom-element registration (telemetry, cross-window communication, etc.), the
  SSR-skip strategy breaks. Mitigation: a snapshot test on the rendered HTML
  asserts the placeholder shape; a regression in sophia-element that affects SSR
  trips that test.
- **Hydration mismatch.** SSR-rendered HTML must match the client-rendered tree
  exactly, or Angular discards it and re-renders (the "flash"). Diverging
  environment values (dates, locales, environment configs) are the usual culprit.
  Mitigation: the AngularRenderer pins locale and timestamp inputs explicitly;
  Playwright a2o scenarios catch the flash.
- **Memory ceiling under burst.** A burst of SSR requests can saturate the isolate
  pool and force CSR fallback. Acceptable degradation, but operator visibility
  matters. Mitigation: the `render.isolate_pool.size` gauge and a documented runbook
  for "we're SSR-saturated."
- **REA compute scope creep.** The audit-trail framing makes it tempting to fold
  compute settlement into the MVP. Don't. The MVP delivers SSR. Compute settlement
  is a separate spec that consumes this artifact later.

## Non-goals

- Replacing Cloudflare Workers as a general PaaS. The runtime *could* graduate
  toward that — the audit trail is shaped for it — but this spec does not deliver
  it.
- Reimplementing Node from scratch. The shim covers exactly what Angular SSR
  reaches for, and refuses everything else.
- SSR for user-personalized routes. MVP renders only `commons` reach content.
- Public TLS / DNS / auth surface on elohim-storage for external clients. Storage
  stays internal-facing on the public internet for MVP.
- Streaming SSR.
- Server-side execution of `sophia-element` or any other UMD-shaped client asset.
- Wiring SSR into the design-asset pull workflow.

## Vocabulary check

This spec stays inside the protocol's vocabulary discipline:

- "doorway" not "edge service"
- "peer" not "node" (when describing autonomous participants)
- "rendering" / "render" not "compute" (compute is the broader REA category)
- "audit trail" / "derived artifact" not "telemetry" / "metrics" (those
  observability hooks are stewardship-internal, not user-facing)
- No "scale," "platform," "users," "engagement," "optimize"

The runtime is infrastructure for the table — a shared kitchen tool, not a
SaaS layer.
