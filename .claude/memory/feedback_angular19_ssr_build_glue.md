---
id: feedback-angular19-ssr-build-glue
name: feedback_angular19_ssr_build_glue
description: "The 13-fix Angular-19-on-doorway SSR unblock cluster — the render unblock was wiring the fetch shim into the V8 isolate (with_full_shims, not with_shims); plus index.csr.html (not index.html), pnpm --filter doesn't walk tsconfig-path aliases, and shamefully-hoist is required."
metadata:
  node_type: memory
  type: feedback
  originSessionId: doorway-ssr-deliver-2026-05-07T23-37
cites:
  - elohim/elohim-render/src/angular.rs
  - doorway/doorway-service/Dockerfile
---

**Angular-19-on-doorway SSR build-glue gotchas (the 13-fix unblock cluster).** Four load-bearing ones:

1. **The render unblock was wiring the `fetch` shim into the V8 isolate.** `deno_core`'s `JsRuntime::with_shims()` does NOT include `fetch`; `with_full_shims(fetcher)` does. The Angular bundle awaits HTTP during bootstrap (ConfigService/AuthService); with no `fetch` global those hang forever past any timeout — the `elohim/elohim-render/src/angular.rs:99-102` "Task 14+" TODO was the actual blocker, not a render-pipeline bug.
2. **Angular 19's application builder with SSR emits `index.csr.html`, not `index.html`.** nginx-ingress base images carrying their own `index.html` silently serve the base Welcome page; the Docker build needs `rm -rf` of the nginx html dir before COPY.
3. **pnpm `--filter "elohim-app..."` does NOT walk tsconfig-path-aliased workspaces.** `@elohim/service` is referenced via `tsconfig.json` paths (not `package.json` deps), so its peerDeps weren't installed; needs an explicit `--filter "@elohim/service..."`.
4. **`shamefully-hoist=true` is required** for Angular pnpm monorepos. The docker build context doesn't COPY the repo-root `.npmrc` (it carries Nexus auth), so a stripped inline `.npmrc` write is needed inside the Dockerfile.

**How to apply:**
- SSR hang during bootstrap with no error → check `with_full_shims` vs `with_shims`; a missing `fetch` global hangs silently.
- SSR serving the framework's default Welcome page → the base image's `index.html` is shadowing `index.csr.html`; `rm -rf` the html dir before COPY.
- "module not found" for a tsconfig-aliased lib at SSR build → add an explicit `--filter "@<alias>..."`; pnpm filter doesn't follow path aliases.

Canonical watch-out also folded into the doorway-SSR runtime seed (B.2 build-glue subsection). The deploy is HELD: doorway SSR alpha pod is BLOCKED on Harbor registry storage EIO (`cf53a76c2`) — code + tests landed, do NOT assert in-cluster green. Related: [[project_ssr_is_compute_capability_claim]], [[project_ssr_anonymous_auth_context]].
