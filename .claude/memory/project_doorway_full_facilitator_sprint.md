---
name: Doorway as full-blown web2 facilitator (SPA host + projection + SSR + ingress concerns)
description: Future sprint candidate — extend doorway to absorb the SPA-host role currently held by elohim-site, so every doorway is a complete web2 projection of its substrate (not split across separate static-site pod + ingress rules + cache layer)
type: project
originSessionId: 2a998ad1-49e1-4f9d-a4ca-0cb796181cbf
---
Direction surfaced 2026-05-08 during /deliver of doorway-ssr-runtime: the alpha ingress had to gain explicit `/lamad/concept`, `/lamad/path` rules to keep them reaching doorway and not falling through to the static SPA pod. That's a routing concern that the source-of-truth (storage's `build_manifest()` with `render: "angular-ssr"` annotations) already declares — but the ingress can't introspect it.

**The sprint hypothesis:** retire elohim-site as a separate pod. Doorway absorbs SPA hosting. Single ingress rule per doorway host (`/` Prefix → doorway). Doorway internally dispatches:
- SSR-eligible manifest routes → elohim-render
- API/blob routes → storage proxy / blob cache
- Bare client-side SPA routes (`/lamad/concept/X` when client wants no SSR) → serve `/index.html` from baked-in SPA assets
- Static SPA assets (`/main.js`, `/styles.*.css`, `/assets/*`) → serve from baked-in browser bundle

**Why:**
- Pattern uniform across every peer that runs doorway (not just doorways with elohim-site sidecars)
- Ingress simplifies to one rule per host
- The "drift between ingress and storage manifest" class of bug disappears entirely (this is the bug the ingress comment in `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml:44-59` documents)
- SSR-eligibility, asset caching, blob caching, and SPA fallback all live in one process — single source of truth for "what does this doorway do for an HTTP client?"

**Aligns with existing direction:**
- `project_doorway_manifest_driven_routes` — "App manifests declare HTTP routes; doorway is registry-driven proxy"
- `project_doorway_single_target_no_fanout` — doorway is single-target dispatch
- `project_three_layer_truth_model` — doorway = web2 projection (not P2P participant)

**Design space for brainstorming:**
- Should doorway include the elohim-app browser bundle in its image (alongside the SSR server bundle), or pull both as separate Harbor artifacts?
- Should there be a `manifest endpoint` doorway publishes for ingress controllers to reflectively poll (so the rare cases where ingress *must* see paths — e.g., per-path TLS or rate limits — stay in sync)?
- How does this interact with multi-doorway federation? (Each doorway federates a different SPA build? Same browser bundle but different SSR-config?)
- What about Tauri / native clients that don't go through doorway at all?

**Status:** Not blocking the doorway-ssr-runtime delivery — that lands first. After SSR is observable on alpha (B2 + B3 + A all green), this is a clean candidate for `superpowers:brainstorming`. The ingress comment added at `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml:44-59` is the TODO marker for it.
