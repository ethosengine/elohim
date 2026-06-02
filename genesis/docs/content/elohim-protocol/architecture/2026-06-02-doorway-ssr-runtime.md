---
title: Doorway SSR Runtime — server-render as an honest compute capability
tier: architecture
status: Design + code landed; in-cluster deploy BLOCKED (alpha SSR pod on Harbor registry storage EIO, cf53a76c2) — do NOT assert in-cluster green
created: 2026-06-02
pillar coupling: doorway (web2 projection), elohim (capability advertisement substrate)
# Born-linked: this seed compacts a settled three-thread design cluster. Raw bodies retire to git history.
compacted_from:
  - genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md
  - genesis/docs/superpowers/plans/2026-05-07-doorway-ssr-runtime.md
  - genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (the access-tier model SSR serves into)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (doorway as Track-4 web2 projection)
informs:
  - All future doorway render-path specs (Renderer trait, manifest render: directive)
  - All future capability-advertisement work (renderCapability profile, PeerStatusView projection)
memory_anchors:
  - project_ssr_anonymous_auth_context
  - project_ssr_is_compute_capability_claim
  - project_doorway_single_target_no_fanout
  - project_compute_and_model_independent_diversity_surfaces
  - project_storage_as_pod_operator_sets_virtual_limits
  - feedback_angular19_ssr_build_glue
defers:
  - REA settlement of SSR compute (audit tuple shaped for it; not wired)
  - sophia-element server-side rendering (placeholder via CUSTOM_ELEMENTS_SCHEMA)
---

# Doorway SSR Runtime — server-render as an honest compute capability

> **Canon status:** Architecture seed for the doorway server-render path.
> **Held:** code + tests landed; the alpha SSR pod deploy is BLOCKED on a Harbor registry storage EIO
> (`cf53a76c2`). Do not read this seed as "in-cluster verified" — it describes the landed shape, not a live pod.

---

## What this is

The doorway can **server-render content routes** instead of always shipping the CSR shell. The render runtime is the `elohim-render` crate embedding **V8 via `deno_core`** behind a `Renderer` trait; the manifest opts a route in with `render: "angular-ssr"`. The CSR shell is **always the fallback floor** — if SSR is unavailable, mis-configured, or the render fails, the route degrades to client-side rendering, never to a blank page.

SSR here is not a performance trick bolted onto the web tier. It is framed as an **honest, advertised compute capability**: a doorway that can server-render *says so* (a `renderCapability` profile, surfaced via `GET /admin/capability`) and that claim is **projected into `PeerStatusView` as Category-C operational state** — reconstructed from the live runtime, never DHT-notarized. A peer that cannot render does not pretend it can. This is the same discipline as the rest of the substrate's capability advertisement: compute and model are independent diversity axes, and a capability claim is checkable, not asserted.

## The auth-context invariant

Auth context is threaded through the **V8 fetch shim** so a server-render runs *as the requesting identity*. The hard rule: **an authenticated request must NEVER silently downgrade to an anonymous render.** If the auth context cannot be threaded, the route falls back to **CSR with an explicit `x-ssr-skipped` marker** — the browser then renders client-side under the real session, rather than the server emitting an anonymous page that looks authenticated. (Memory: `project_ssr_anonymous_auth_context`.)

Capability claims are **stratified** and conservative by default: a doorway stays Tier-2 (CSR-only projection) unless it has earned and advertised the render capability. The audit tuple emitted per render is **shaped for future REA settlement** of SSR compute — so the compute↔settlement loop can close later without a wire break — but settlement itself is deferred.

---

## Watch-outs (carry these forward)

### Angular-19-on-doorway SSR build-glue (the 13-fix unblock cluster)

Standing up SSR against an Angular 19 bundle is a build-glue problem before it is a runtime problem. Four glue gotchas, each of which silently produces a wrong-but-green build:

1. **The render unblock is the `fetch` shim in the V8 isolate.** `deno_core`'s `JsRuntime::with_shims()` does NOT include `fetch`; `with_full_shims(fetcher)` does. The Angular bundle awaits HTTP during bootstrap (ConfigService/AuthService) — with no `fetch` global those awaits hang **forever past any timeout** (this was the actual `elohim-render/src/angular.rs` "Task 14+" blocker, not a render bug).
2. **Angular 19's application builder with SSR emits `index.csr.html`, not `index.html`.** An nginx-ingress base image carrying its own `index.html` then silently serves the base Welcome page. The Docker build must `rm -rf` the nginx html dir before COPY.
3. **pnpm `--filter "elohim-app..."` does NOT walk tsconfig-path-aliased workspaces.** `@elohim/service` is referenced via `tsconfig.json` paths (not `package.json` deps), so its peerDeps go uninstalled; needs an explicit `--filter "@elohim/service..."`.
4. **`shamefully-hoist=true` is required** for Angular pnpm monorepos, and the Docker build context does NOT COPY the repo-root `.npmrc` (it carries Nexus auth) — so a stripped inline `.npmrc` write is needed in the build stage.

(Full cluster: memory `feedback_angular19_ssr_build_glue`.)

### Runtime + cold-start

- The Angular-19 **global-shape shim must cover exactly what Angular touches** — no more (over-shimming masks real bootstrap errors).
- **`sophia-element` / the UMD bundle must NOT run server-side** — placeholder it via `CUSTOM_ELEMENTS_SCHEMA`; the web component renders client-side only.
- **V8 cold-start is real:** observed wall-time 2s → 15s → 60s as the isolate warms. The pod needs a **memory bump + a `startupProbe`**, or the orchestrator kills it mid-warm. (Operator note: staging/prod pod floors carried 256Mi and will OOM on an SSR roll — that bump is a tracked backlog item, not done here.)

---

## Excluded (genuinely live — left in the pile)

- `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` — DISTINCT cluster (viewer-side CapabilityProfile, folded into the cradle-to-grave gradient canon's rendering-realization section). The doorway-SSR `renderCapability` and the viewer-side CapabilityProfile are different observables; do not merge them.

---

## Status / hold

Code + unit/integration tests landed. The **alpha SSR pod deploy is BLOCKED** on a Harbor registry storage EIO (`cf53a76c2`); this seed therefore documents the *landed shape*, not an in-cluster-verified runtime. When the registry heals and the pod rolls, update this status line — do not retroactively claim green from the code-landed state.
