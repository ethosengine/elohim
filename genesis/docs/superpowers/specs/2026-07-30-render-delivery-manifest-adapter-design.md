---
title: Render Delivery as Manifest Contract — the Protocol-Level Client Adapter
id: render-delivery-manifest-adapter-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
domain: D8
topic: [ssr, render, delivery, app-manifest, sdk, doorway, elohim-render, render-capability, client-surface, adapter, v8]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
cites:
  - doorway-ssr-runtime | Peer-capability foundation this position keeps intact — RenderCapabilityProfile as Category-C derived min(probes/allocation/ceiling), CSR floor, x-ssr-skipped taxonomy | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - native-rust-epr-shell-ssr-design | Settles the content half (SSR-capability as one EPR nature, server-bundle field not sibling row); this spec adds the missing developer half — the manifest delivery contract | sha256:22d465f63c1fe668 | path: genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md
  - ssr-bundle-substrate-content-decouple-design | Bundle-as-substrate-content move this spec generalizes from per-host publish plumbing into a declarative manifest delivery section | sha256:8a2e71a5b235206e | path: genesis/docs/superpowers/specs/2026-06-24-ssr-bundle-substrate-content-decouple-design.md
  - elohim-seam-map-concern-routing | Routing authority for the verdict — the adapter is a MANIFEST addition (SDK seam 3.5/3.7), not more engine substrate (3.3) or doorway plumbing (3.9) | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - platform-one-sdk-many-apis-design | The five-verb grammar this extends sideways — delivery becomes one more declarative manifest surface, no new grammar verbs, no new DHT entry types | sha256:a15b10c68787a460 | path: genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md
---

# Render Delivery as Manifest Contract — the Protocol-Level Client Adapter

- **Date:** 2026-07-30
- **Status:** Design position (settled verdict; manifest-contract work pre-implementation)
- **Seam:** SDK grammar + app-manifest (atlas §3.5/§3.7) for the contract; doorway projection (§3.9) and runtime/footprint (§3.3) stay the serving/engine seams they already are.
- **Prompted by:** the Angular 22 landing — the operator's question whether SSR is "a square peg in a round hole" for a P2P architecture, and whether pushing a low-level V8 SSR substrate is the right investment versus a protocol-level adapter over the elohim-storage / app-manifest / doorway contracts.

## 1. The question, and the verdict

Is SSR fundamentally misaligned with P2P architecture?

**Verdict: no — but only because of how this corpus already models it, and the operator's
instinct correctly names the one place it is still misaligned.** SSR here is not a
server-owned rendering tier; it is modeled as **one EPR's nature × the serving peer's
advertised capability**, with CSR as the unconditional floor:

- The content node carries its own delivery nature (browser bundle ref, optional server
  bundle ref, theme) — settled in the 2026-06-26 native-rust-epr-shell design, which
  explicitly corrected the earlier per-host sibling-row reflex.
- Whether SSR *happens* is the serving peer's business: `RenderCapabilityProfile` is
  **Category-C operational state** derived at runtime as `min(probes, allocation, ceiling)`,
  advertised to peers, never DHT-notarized. A peer that cannot render says so and serves
  the CSR shell. No peer must render for the network to work — SSR is a capability capable
  peers *contribute*, exactly like blob stewardship. This satisfies the hub-optional floor:
  render capability graduates convenience, it never gates participation.
- The engine (`elohim-render`, deno_core/V8) is already consumed by BOTH doorway and an
  SSR-enabled elohim-storage — the a2o contract language is explicit: "SSR is p2p-native,
  not doorway-owned." Composition is selector-agnostic (`<app-root>`, `<lamad-root>` — the
  root tag is derived from the rendered document, no per-app code).

So the peer half and the engine half are aligned. What is NOT yet protocol-level is the
**developer half** — and that is the square peg the operator is feeling.

## 2. The actual gap — delivery is per-app build plumbing

The app-manifest schema today declares renderers only as **in-app vocabulary**
(`rendering` → Angular component names for content formats; `routeClaims`). It has **zero
fields for delivery**: no bundle refs, no render-spec requirement, no server-entry
declaration. How an app's client actually reaches a serving peer lives in per-app build
plumbing — angular.json, Dockerfile sed strips, image baking, per-host blob PATCH loops —
precisely the layer that produced the host-divergence and stale-bundle incident classes.

The cost of the missing contract is also visible at runtime: the elohim-app SSR stall
(v22 landing, resolved fourth-pass 2026-07-30) was the render host's **implicit
environment contract breaking silently** — Angular 22 began requiring
`URLSearchParams.sort()` on every HttpClient request (transfer-cache interceptor), the
elohim-render shim didn't provide it, and the resulting synchronous throw leaked an
unremovable PendingTask that permanently blocked `whenStable()`. Because the guaranteed
shim surface is declared nowhere, a framework-version bump turned a one-method
conformance gap into a silent total stall instead of a nameable contract violation. The
web-api shim surface, the fetch routing, the stability semantics — all real, all
load-bearing, none declared. (Shim conformance is an open class — `Headers`,
`AbortSignal`, `ReadableStream`, `FormData` carry the same "minimal until the framework
moves" risk — which is exactly what a versioned contract in §3b retires.)

## 3. Decision — the adapter is a manifest contract, not more V8

Per the atlas disambiguator: *what do you ADD?* A **manifest** → SDK seam. The
protocol-level adapter the operator is asking for is a **delivery section of the
app-manifest** plus a named **render-host environment contract**, both riding surfaces
that already exist. Not a new crate, not more engine substrate, not per-framework plumbing
in every app.

**3a. Manifest `delivery` contract (the "what an app ships").** The app-manifest schema
gains a delivery declaration: browser bundle ref (content-addressed, CID form), optional
server bundle ref (its presence IS the SSR-capable marker, per the one-EPR's-nature
model), and the render spec (e.g. `angular/22`) the server bundle targets. The render
spec is a **matching key, never an obligation on peers**: it names which renderer a peer
would need in order to *choose* SSR for this app. A peer that holds a matching renderer
and has capacity may render; every other peer — and any peer, any time it prefers — serves
the browser bundle as-is (CSR). The manifest describes what the app ships; it never
prescribes what a peer must run. This is the diversity-of-peers epic expressed at the
delivery layer: the same manifest is fully servable by a household node with no V8 and by
a render-capable doorway, with no app-side or manifest-side difference. Selector stays
derived from the rendered document — never declared, so it cannot drift. Publishing an app = publishing bundles as substrate content + declaring them
in the manifest; any capable peer can then serve and optionally render it. This is "one
SDK, many apps" extended to delivery: the five-verb grammar untouched, one declarative
surface added.

**3b. Render-host environment contract (the "what a serving peer guarantees").** The
implicit contract between `elohim-render` and an app bundle becomes explicit and
versioned: the guaranteed global surface (the web_api/node shim set), the fetch routing
rule (**server renders receive their data plane from the render host — single
server-appropriate base, no browser failover heuristics, no wall-clock browser
timeouts**), and the stability semantics (zoneless; PendingTasks gate the render). Apps
target this contract; the SSR-stall class becomes a contract violation you can name
instead of a silent hang. The existing shed taxonomy (`x-ssr-skipped: <reason>`) is the
runtime face of the same contract.

**3c. Peer side — unchanged by design.** `RenderCapabilityProfile` stays Category-C,
derived, reduce-only under operator override; renderer-vs-app identity is gated before a
render is spent; CSR shell remains the floor. Nothing in this design adds notarized state.

**3d. Framework pluggability rides the render spec.** `angular/22` is one renderer id, not
the contract. The native-Rust body render for content-shaped EPRs (2026-06-26 §9) slots in
as another renderer id against the same manifest declaration — which is exactly the
"adaptable clients from what emerges of our SDK" the operator asked for.

**3e. The renderer is an optional feature of the runtime; doorway is only a projector.**
Render capability composes into the **peer runtime** (the `ssr` cargo feature on
elohim-storage — the runtime/footprint seam's leanness lever: a household node builds
without V8 and loses nothing but the option to render). Doorway consumes the *identical*
engine and contract solely to project rendered HTML to the web2 world — its normal
projection job, nothing more. The operational test: **a mesh with zero doorways has full
SSR delivery** among its render-capable peers; doorway never appears in the render
dependency chain, only at the web2 boundary. Consequence for code placement: render-path
logic — the trust-scoped render cache, capability derivation, auth-mode gating, the shed
taxonomy — belongs in the shared engine/render-host layer, with doorway and storage as
symmetric thin hosts. Any render logic that exists only in doorway is a misplacement to
be migrated, not a pattern to extend.

## 4. P2P design gate

| Entity | Classification | Address | Source of truth |
|---|---|---|---|
| Server/browser bundle bytes | existing blob plane | CID (`bafkrei…` target; bare-sha wire is legacy) | substrate content |
| Bundle refs + render spec | manifest declaration (SDK seam — declarative data, integrity by construction) | fields on the app manifest / EPR node | schema-validated manifest |
| `RenderCapabilityProfile` | **C (operational)** — already so classified | derived per peer | live runtime; reconstructed, never notarized |
| Render delegation across peers (future) | Mishpat `delegates-compute` commitment | commitment CID = entry_hash | DHT |

No new DHT entry types. No new tables. The future cross-peer render-delegation row is
named so it lands on the existing compute-commitment primitive when it matters, not on an
API key.

## 5. Boundaries — what this deliberately does not do

- **No per-user credentialed SSR** until the isolate-reuse trust boundary is closed
  (realm-per-render or equivalent); the render host contract inherits that tripwire.
- **No SSR-as-requirement anywhere, in either direction.** A manifest with no server
  bundle ref is a complete, first-class app; a peer with no renderer (or one that sheds a
  render for any reason) is a complete, first-class serving peer — it serves the browser
  bundle and that is full participation, per the diversity-of-peers epic. Any design that
  makes rendering capability load-bearing for participation is a capture smell.
- **No engine expansion for its own sake.** V8 substrate work (shims, probes, realms) is
  justified only as far as the environment contract in 3b requires — the contract bounds
  the engine, not the other way around.

## 6. Sequencing

1. Land the Angular 22 follow-ups that harden the current engine seam: the SSR-stall fix
   (URLSearchParams shim conformance in `elohim-render` — the first concrete instance of
   the §3b contract), the isolate-reuse settlement, and the zone-polyfill finding (folded
   into the browser-zoneless migration as an acceptance criterion — the builder already
   splits server polyfills off `isZonelessApp(polyfills)`; there is no standalone build
   lever). These are in flight on `feat/angular22-node24`.
2. Consolidate render serving logic out of doorway (§3e): the trust-scoped cache,
   capability derivation, and gating move to the shared engine/render-host layer so
   doorway and storage-ssr are symmetric thin hosts and the zero-doorway mesh test holds.
3. Manifest `delivery` schema fields + codegen (SDK seam; schema → contract test → TS).
4. Environment-contract doc + version stamp surfaced beside `RenderSpec`, with the a2o
   compose/capability features extended to assert it.
5. Cross-peer render delegation waits for a real second-peer use case, then lands on
   `delegates-compute`.
