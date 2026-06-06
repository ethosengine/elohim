---
id: epr-slice2-universal-address-plan
status: active
cites:
  - pillar-epr-decomposition-design | parent canon — §12 URL & Routing Contract; §12.3 mount-agnostic link minting and §12.6 slice table this plan implements (Slice 2) | sha256:f14c5ebe1fc086d8 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - omnibar-consolidation-epr-native-links-design | the landed interceptor + EprNavService substrate this plan distributes to every bundle; settled cross-bundle anchor form (plain href + handoff) | sha256:71ad45eb5993b56c | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
---

# §12.6 Slice 2 — Universal `/epr/{id}` Address + Distributed EPR-Link Routing Sweep — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the universal `/epr/{id}` address (doorway resolver + shell route + `eprToRoute`/`BundleRouteContext` claims rewrite in `@elohim/service`), then sweep every cross-bundle link-minting site in every bundle onto EPR-native navigation — per spec `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` §12 (esp. §12.3: *in-mount targets → relative routes; everything else → `/epr/{id}`; no literal pillar prefix in shared code*).

**Architecture:** Doorway gains a `/epr/*` service-path arm that serves the ROOT projection's bundle (the shell), whose new `epr/:resourceId` route renders the cross-pillar resource viewer. `eprToRoute()` is rewritten around a `BundleRouteContext` injection token — each bundle's composition root declares which EPR `contentType`s it renders natively (lamad claims `path`; the shell owns the universal route); unclaimed targets resolve to `commands: null` + `href: '/epr/{id}'`, navigated via plain anchors (epr-link interceptor) or `EprNavService.navigate()` (full-load handoff). ~20 broken lamad sites, the self-loop redirect, the Lit navigator's baked literals, the SEO canonical bug, the doorway journal stub literals, and the portal's missing interceptor all ride the same wave, with a2o render-verified scenarios committed alongside.

**Tech Stack:** Rust (hyper, doorway-service), Angular 19 (standalone, signals, Vitest), Lit 3 (@open-wc/testing, web-test-runner), Cucumber a2o (Playwright).

**Branch + hygiene:** Create `epr/slice2-universal-address` off `dev`. Commit per task with SELECTIVE `git add` of exactly the listed files (concurrent sessions may share the worktree — never `git add -A`). **Never push** — the integrator owns push/merge (`dev` lands via local ff merge).

**Key commands (per area):**

| Area | cwd | Command |
|---|---|---|
| doorway build/test | `doorway/doorway-service` | `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins` |
| doorway gates | `doorway/doorway-service` | `RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check` |
| storage fixture test | `elohim/elohim-storage` | `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo nextest run is_spa_route_subpath` (plain cargo target dir — WASM workspace) |
| shared lib | `app/elohim-library/projects/elohim-service` | `pnpm exec vitest run angular/utils/epr-ref` (full: `pnpm test`) |
| shell | `app/elohim-app` | `pnpm exec vitest run --config vite.config.ts <pattern>` |
| lamad | `app/lamad` | `pnpm exec vitest run --config vite.config.ts <pattern>` |
| elements | `app/elohim-elements/elohim-core` | `pnpm test` (wtr) · gates: `pnpm lint && pnpm lint:css && pnpm typecheck && pnpm build` (build regenerates custom-elements.json) |
| portal | `app/imagodei-portal` | `pnpm test && pnpm typecheck` |
| a2o gate | `genesis/a2o` | `npx cucumber-js --dry-run` (undefined-step gate) |

**RUSTFLAGS discipline:** doorway is native (`RUSTFLAGS=""` + pooled `CARGO_TARGET_DIR`); elohim-storage is a WASM workspace (keep `--cfg getrandom_backend="custom"`, plain cargo, NO `CARGO_TARGET_DIR`). Never green both with one env in one command.

---

## P2P design gate record

Per spec §12.8 (the parent design already ran the gate — this plan adds NO new entities):
the `/epr/{id}` doorway arm is **operational convention (C)** — a read-side resolution
policy over the **existing notarized `project-epr` Commitment (A)** (it dispatches the
already-registered root projection; same classification as the storage SPA-fallback).
`BundleRouteContext`/route claims are client-side composition-root config (C) — Slice 3
later moves claims INTO the bundle EPR's manifest content (still C, riding the existing
content row). No new DHT entry types, no new identity schemes, no new tables or sync
messages. The route follows from the DHT design (projection commitments), not before it.

## Design decisions locked by this plan

1. **`eprToRoute(ref, ctx, contentType?)` returns a discriminated `EprRouteResolution`** — `{ commands: string[] | null, href: string, claimed: boolean }`. `commands` is non-null iff this bundle claims the target (or owns the universal route); `href` is ALWAYS the origin-absolute universal address `/epr/{id}` (cross-bundle-safe everywhere). Unclaimed targets must NEVER become router commands inside a bundle — that is how `/lamad/epr/…` doubled prefixes would get minted (the §12.0 class Slice 1 killed).
2. **`ctx` is REQUIRED** (no default param) — forces every call site to be considered during the sweep. The injection-token default factory (`{ claims: [] }`) is the safe runtime default for bundles that never provide one.
3. **The type-vs-slug heuristic dies.** Route shape comes from the caller-supplied `contentType` (callers that fetched the head have it) or the single sanctioned structural inference: a `step` fragment implies `path`. Unknown type → `/epr/{id}` (always safe; the resolver figures out rendering).
4. **Doorway `/epr/{id}` = serve the root projection's bundle** (Slice 2 semantics). The 302-to-pretty-mount upgrade is Slice 3 (routeClaims in manifests) and is NOT built here. Reservation is enforced structurally: `/epr` joins `is_service_path`, so no projection mount can ever capture it (the EPR-router gate runs only on non-service paths), plus a warn-and-skip guard at projection ingestion.
5. **Step fragments degrade gracefully in the shell.** `/epr/{pathId}` (no step) is what the shell mints for an in-path target it doesn't claim; fragment-preserving redirect is Slice 3. The universal `href` carries `#step/n` for cold loads (harmless today, Slice-3-ready).
6. **Legacy `/lamad/resource/{id}` URLs get a one-route in-bundle bridge** (tiny redirect component → `/epr/{id}`) replacing the self-loop `redirectTo`. These were canonical monolith-era URLs with real shares in the wild; this is a deliberate legacy bridge at the route that minted them, not the §12.4 "redirect-heal hack" (which concerns doubled-URL bug minting; mount-level moves stay `redirects_from` on the commitment).
7. **The omnibar's `/auth/signin` href stays.** `/auth/*` is doorway-owned service vocabulary (same class as `/epr`), not a pillar bundle route — uniform across deployments. A JSDoc note documents this. The Lit navigator's `/identity/*` + pillar context-app routes ARE app routing and move to host-supplied properties.
8. **Journal derivative cards stop minting pillar mount URLs.** `suggested_path` for derivatives becomes `""` — `destination_type` is the routing vocabulary; clients mint routes via their claims context. (Filing cards keep their bare folder slug — it is journal-folder metadata, not a URL.)

---

## File structure

```
NEW
  app/elohim-library/projects/elohim-service/src/angular/utils/bundle-route-context.ts   (BUNDLE_ROUTE_CONTEXT token)
  app/lamad/src/app/components/legacy-resource-redirect/legacy-resource-redirect.component.ts (+ .spec.ts)
  app/lamad/src/app/shared/services/seo.service.spec.ts                                  (lamad canonical regression)
  genesis/a2o — new steps in steps/lamad/deep-link-delivery.steps.ts (no new step file)

MODIFIED — substrate
  doorway/doorway-service/src/server/http.rs              (/epr service path + universal dispatch arm + reservation guard + new test mod)
  doorway/doorway-service/src/projection/epr_router.rs    (warn-and-skip reserved mounts in replace_all)
  doorway/doorway-service/src/routes/journal.rs           (derivative suggested_path de-literalization + tests)
  elohim/sdk/fixtures/spa-route-discrimination.vectors.json (epr/{id} vector)

MODIFIED — shared lib
  app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts       (eprToRoute rewrite + types + eprToUniversalHref)
  app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.spec.ts  (rewritten eprToRoute block)
  app/elohim-library/projects/elohim-service/src/index.ts                        (export bundle-route-context)

MODIFIED — shell (app/elohim-app/src/app)
  app.routes.ts (+epr/:resourceId)  ·  app.routes.spec.ts (canaries 13→14, TODO refresh)
  app.config.ts (BUNDLE_ROUTE_CONTEXT provider)
  elohim/services/epr-nav.service.ts (+layout-root descent) + .spec.ts
  elohim/services/epr-resolver.service.ts (claims-aware minting, href fields) + .spec.ts
  elohim/components/epr-resolve-redirect/epr-resolve-redirect.component.ts
  elohim/components/epr-link/epr-link.component.ts (null-guard + href fallback)
  elohim/components/epr-popover/epr-popover.component.ts (+href input)
  services/seo.service.ts (base-aware canonical + /epr content canonical) + .spec.ts

MODIFIED — lamad (app/lamad/src/app)
  app.config.ts (LAMAD_EPR_NAV + BUNDLE_ROUTE_CONTEXT providers)
  interfaces/cross-pillar.interface.ts (+LAMAD_EPR_NAV token + ILamadEprNav)
  lamad.routes.ts (self-loop → legacy bridge) · lamad.routes.spec.ts
  components/{path-navigator,path-overview,attention-flow,learner-dashboard/refresh-queue,search,
              content-editor-page,content-viewer,graph-explorer,meaning-map,profile-page}/* (sweep + specs)
  services/path-context.service.ts (+ spec)
  guards/lamad-identity.guard.ts (+ spec)
  renderers/markdown-renderer/markdown-renderer.component.ts
  quiz-engine/components/recommendation-list/recommendation-list.component.ts
  components/lesson-view/lesson-view.component.ts (resolved.route null-guard; verify)
  components/lamad-layout/lamad-layout.component.{ts,html} (navigator host config) + spec
  shared/services/seo.service.ts (twin of shell edit)

MODIFIED — elements (app/elohim-elements/elohim-core/src)
  elohim-navigator.ts (DEFAULT_CONTEXT_APPS removed; identityRoutes host-supplied) + elohim-navigator.spec.ts
  elohim-default-omnibar.ts (JSDoc note only)
  app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-navigator.default.stories.ts (fixture contextApps)

MODIFIED — portal
  app/imagodei-portal/src/main.ts (interceptor install)

MODIFIED — a2o / fixtures
  genesis/a2o/features/lamad/deep-link-delivery.feature (+2 scenarios)
  genesis/a2o/steps/lamad/deep-link-delivery.steps.ts (+2 steps)
  genesis/a2o/src/framework/pages/selectors.ts (+CONTENT_VIEWER.ROOT)
  app/lamad/src/app/components/content-viewer/content-viewer.component.html (root testid if missing)

MODIFIED — docs (managed surfaces — use cite tooling discipline; see Task 16)
  genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md (§12.6 slice status note)
  app/lamad/CLAUDE.md + app/elohim-app/CLAUDE.md (cross-bundle rail: mint /epr/{id})
```

---

### Task 0: Branch setup

- [ ] **Step 0.1: Create the feature branch**

```bash
cd /projects/elohim
git checkout dev && git pull --ff-only 2>/dev/null || true
git checkout -b epr/slice2-universal-address
```

Expected: on branch `epr/slice2-universal-address`, clean tree (`git status` shows nothing staged).

---

### Task 1: Doorway — `/epr` reservation + universal dispatch arm

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (is_service_path ~1096-1137; new fn near dispatch_to_projected_epr ~1452; new match arm near the `/epr-head/` arm ~2231; new test module AFTER `mod handoff_routing_tests`)
- Modify: `doorway/doorway-service/src/projection/epr_router.rs` (replace_all warn-and-skip)
- Modify: `elohim/sdk/fixtures/spa-route-discrimination.vectors.json`

**Context:** Request dispatch order in `handle_request`: auth gate → EPR-router gate (line ~1674, fires for `GET && !is_upgrade && !is_service_path`) → explicit match block. Without `/epr` in `is_service_path`, a root projection (`url_path="/"`) swallows `/epr/{id}` before any explicit arm runs. `mod shakeout_tests` (lines ~1236-1380) is a FROZEN oracle — add new tests in a NEW module, never edit it.

- [ ] **Step 1.1: Write the failing tests** (new module at the END of http.rs, after `mod handoff_routing_tests`)

```rust
#[cfg(test)]
mod epr_universal_tests {
    use super::*;

    // §12.1: /epr is reserved — a service path, never a projection mount.
    #[test]
    fn epr_prefix_is_a_service_path() {
        assert!(is_service_path("/epr"));
        assert!(is_service_path("/epr/manifesto-foundations"));
        assert!(is_service_path("/epr/foundations-christian-technology"));
    }

    #[test]
    fn epr_head_remains_a_service_path() {
        // /epr-head/ does NOT start with "/epr/" — the two prefixes are disjoint.
        assert!(is_service_path("/epr-head/manifesto"));
    }

    #[test]
    fn epr_reservation_rejects_projection_mounts() {
        assert!(is_reserved_url_path("/epr"));
        assert!(is_reserved_url_path("/epr/anything"));
        assert!(is_reserved_url_path("/db"));
        assert!(is_reserved_url_path("/api/v1"));
        // The root mount and the portal mount stay legal.
        assert!(!is_reserved_url_path("/"));
        assert!(!is_reserved_url_path("/auth/portal"));
        assert!(!is_reserved_url_path("/lamad"));
    }
}
```

- [ ] **Step 1.2: Run, verify failure** (compile error: `is_reserved_url_path` not found; the first test fails on `/epr`)

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins epr_universal
```

- [ ] **Step 1.3: Implement**

(a) In the `is_service_path` prefix array (lines ~1105-1127), insert `"/epr/",` immediately after `"/epr-head/",`; extend the final exact-match line to `matches!(path, "/admin" | "/status.json" | "/epr")`.

(b) Add below `is_service_path`:

```rust
/// §12.1 reserved-prefix guard: a projection's `url_path` may never collide
/// with a doorway service surface (including the universal `/epr` address).
/// `"/"` is the sanctioned root mount. Note `/auth/portal` stays legal:
/// `is_service_path` only owns the exact AUTH_OWNED_PATHS under /auth.
pub(crate) fn is_reserved_url_path(url_path: &str) -> bool {
    url_path != "/" && is_service_path(url_path)
}
```

(c) Add the universal dispatcher next to `dispatch_to_projected_epr` (~line 1452):

```rust
/// §12.1 universal EPR address: GET /epr/{id} serves the ROOT projection's
/// bundle (the shell), whose `epr/:resourceId` route resolves and renders the
/// EPR client-side. Doorway-side 302-to-pretty-mount is Slice 3 (routeClaims).
/// Passing "/" as the request path makes `derive_app_subpath` yield the
/// projection's entry_file — /epr/{id} is BY DEFINITION a page address.
async fn dispatch_epr_universal(state: &AppState, request_path: &str) -> Response<Full<Bytes>> {
    match state.epr_router.dispatch("/") {
        Some(root) => {
            tracing::debug!(path = %request_path, root_epr = %root.epr_id,
                "universal /epr address — serving root bundle");
            dispatch_to_projected_epr(state, "/", root).await
        }
        // No root projection registered — same posture as the bare "/" arm.
        None => Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/threshold")
            .body(Full::new(Bytes::new()))
            .unwrap(),
    }
}
```

(d) Add the match arm in the explicit block, directly ABOVE the `/epr-head/` arm (~line 2231) — they are disjoint prefixes but co-locating documents the family:

```rust
        // Universal EPR address (§12.1): /epr/{id} → root bundle (shell epr/:id route).
        (Method::GET, p) if p == "/epr" || p.starts_with("/epr/") => {
            return Ok(to_boxed(dispatch_epr_universal(&state, p).await));
        }
```

(e) In `epr_router.rs` `replace_all`, warn-and-skip reserved mounts (ingestion belt-and-suspenders; the gate order already makes capture impossible):

```rust
        // §12.1: reserved service prefixes can never be projection mounts.
        let (legal, reserved): (Vec<_>, Vec<_>) = views
            .into_iter()
            .partition(|v| !crate::server::http::is_reserved_url_path(&v.url_path));
        for v in &reserved {
            tracing::warn!(epr_id = %v.epr_id, url_path = %v.url_path,
                "skipping projection: url_path collides with a reserved service prefix (§12.1)");
        }
```

(adapt to `replace_all`'s actual local variable names; keep its existing table-rebuild logic operating on `legal`).

(f) Add the fixture vector to `elohim/sdk/fixtures/spa-route-discrimination.vectors.json` (append inside `vectors`):

```json
    { "subPath": "epr/manifesto-foundations", "kind": "route", "note": "universal EPR address under the root mount (§12.1 Slice 2)" }
```

- [ ] **Step 1.4: Run, verify pass** (same nextest command; then the full doorway suite, then the storage half of the drift guard)

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo clippy -- -D warnings
cargo fmt --check
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo nextest run is_spa_route_subpath
```

Expected: all green, including `shakeout_is_spa_route_agrees_with_shared_vectors` (doorway) and `is_spa_route_subpath_matches_shared_vectors` (storage) with the new vector.

- [ ] **Step 1.5: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/projection/epr_router.rs elohim/sdk/fixtures/spa-route-discrimination.vectors.json
git commit -m "feat(doorway): universal /epr/{id} address — reserved prefix + root-bundle dispatch (§12.6 Slice 2)"
```

---

### Task 2: Shell — `epr/:resourceId` route

**Files:**
- Modify: `app/elohim-app/src/app/app.routes.ts` (after the `resource/:resourceId` entry, lines ~56-64)
- Modify: `app/elohim-app/src/app/app.routes.spec.ts` (canaries: count 13→14; TODO block refresh)

**Param name MUST be `:resourceId`** — `ContentViewerComponent` reads `params['resourceId']` and `ProtocolRouteContextService` derives the omnibar `cid` from the same param; a bare `:id` leaves the protocol chrome blank.

- [ ] **Step 2.1: Update the canary spec first** (TDD on route shape). In `app.routes.spec.ts`: change line ~37 `expect(routes.length).toBe(13)` → `toBe(14)`; extend the enumerating comment (lines ~34-36) to `... deliver/:slug, resource/:resourceId, epr/:resourceId, map, resolve, and 404 catch-all`; add below the count test:

```typescript
  it('should have the universal epr/:resourceId route (§12.6 Slice 2)', () => {
    const eprRoute = routes.find(r => r.path === 'epr/:resourceId');
    expect(eprRoute).toBeDefined();
    expect(eprRoute?.data?.['protocolContent']).toBe(true);
  });
```

Replace the stale TODO block (lines 19-27) header sentence with:

```typescript
  // §12.6 Slice 2 (landed): the shell still has no top-level `path` route —
  // path-claimed EPRs render only inside the lamad bundle. Unclaimed refs now
  // mint the universal /epr/:resourceId route (BundleRouteContext, spec §12.3);
  // cross-bundle anchors ride the epr-link interceptor + EprNavService
  // (2026-06-05 omnibar-consolidation spec §4).
  // This canary pins the absence so a stray shell `path` route (or a regression
  // that silently makes these links "work" by accident) is caught.
```

- [ ] **Step 2.2: Run, verify the new assertions fail**

```bash
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/app.routes.spec.ts
```

Expected: FAIL — `routes.length` is 13; `epr/:resourceId` undefined.

- [ ] **Step 2.3: Add the route** in `app.routes.ts` directly after the `resource/:resourceId` entry:

```typescript
  // Universal EPR address (§12.1) — durable cross-bundle target. Renders the
  // cross-pillar resource viewer (reachable-but-unclaimed semantics); the
  // doorway serves this bundle for any /epr/* path (Slice 2). Param is named
  // resourceId so ContentViewerComponent + ProtocolRouteContextService work.
  {
    path: 'epr/:resourceId',
    loadComponent: async () =>
      import('@app/lamad/components/content-viewer/content-viewer.component').then(
        m => m.ContentViewerComponent
      ),
    data: { protocolContent: true },
  },
```

- [ ] **Step 2.4: Run, verify pass** (same command). Also run the route-context spec to confirm no cid regression:

```bash
pnpm exec vitest run --config vite.config.ts src/app/elohim/services/protocol-route-context.service.spec.ts
```

- [ ] **Step 2.5: Commit**

```bash
git add app/elohim-app/src/app/app.routes.ts app/elohim-app/src/app/app.routes.spec.ts
git commit -m "feat(app): universal epr/:resourceId shell route (§12.6 Slice 2)"
```

---

### Task 3: Shared lib — `eprToRoute` claims rewrite + `BUNDLE_ROUTE_CONTEXT`

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts` (replace eprToRoute, lines 151-166; add types + helper)
- Create: `app/elohim-library/projects/elohim-service/src/angular/utils/bundle-route-context.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/index.ts` (line ~168 — add export)
- Modify: `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.spec.ts` (rewrite `describe('eprToRoute')`, lines 132-150)

**Both `app/elohim-app` and `app/lamad` path-map `@elohim/service` to the lib SOURCE — a signature change surfaces immediately in both apps' compile.** That is intentional: Tasks 4-10 fix every consumer; the lib task only needs ITS OWN suite green.

- [ ] **Step 3.1: Rewrite the spec block** — replace `describe('eprToRoute', …)` (epr-ref.spec.ts:132-150) with:

```typescript
  describe('eprToRoute (claims-aware, §12.3)', () => {
    const LAMAD_CTX: BundleRouteContext = {
      claims: [
        {
          contentType: 'path',
          commands: ref =>
            ref.fragment?.type === 'step'
              ? ['/path', ref.id, 'step', ref.fragment.value]
              : ['/path', ref.id],
        },
      ],
    };
    const SHELL_CTX: BundleRouteContext = { claims: [], ownsUniversalRoute: true };
    const EMPTY_CTX: BundleRouteContext = { claims: [] };

    it('unclaimed in a pillar bundle → no commands, universal href', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, LAMAD_CTX);
      expect(res).toEqual({ commands: null, href: '/epr/manifesto', claimed: false });
    });

    it('claimed contentType → in-bundle commands', () => {
      const res = eprToRoute({ id: 'elohim-protocol', tier: 'head' }, LAMAD_CTX, 'path');
      expect(res).toEqual({
        commands: ['/path', 'elohim-protocol'],
        href: '/epr/elohim-protocol',
        claimed: true,
      });
    });

    it('step fragment structurally implies the path claim', () => {
      const res = eprToRoute(
        { id: 'elohim-protocol', tier: 'head', fragment: { type: 'step', value: '2' } },
        LAMAD_CTX
      );
      expect(res?.commands).toEqual(['/path', 'elohim-protocol', 'step', '2']);
      expect(res?.claimed).toBe(true);
      expect(res?.href).toBe('/epr/elohim-protocol#step/2');
    });

    it('shell owns the universal route → /epr commands for unclaimed refs', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, SHELL_CTX);
      expect(res).toEqual({ commands: ['/epr', 'manifesto'], href: '/epr/manifesto', claimed: false });
    });

    it('step fragment in the shell (no path claim) degrades to /epr commands', () => {
      const res = eprToRoute(
        { id: 'elohim-protocol', tier: 'head', fragment: { type: 'step', value: '2' } },
        SHELL_CTX
      );
      expect(res?.commands).toEqual(['/epr', 'elohim-protocol']);
      expect(res?.href).toBe('/epr/elohim-protocol#step/2');
    });

    it('empty context (no claims, no universal route) is cross-bundle for everything', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, EMPTY_CTX);
      expect(res).toEqual({ commands: null, href: '/epr/manifesto', claimed: false });
    });

    it('returns null for blob tier', () => {
      expect(eprToRoute({ id: 'manifesto', tier: 'blob' }, SHELL_CTX)).toBeNull();
    });
  });

  describe('eprToUniversalHref', () => {
    it('mints the universal address', () => {
      expect(eprToUniversalHref({ id: 'manifesto', tier: 'head' })).toBe('/epr/manifesto');
    });
    it('carries the fragment', () => {
      expect(
        eprToUniversalHref({ id: 'p', tier: 'head', fragment: { type: 'step', value: '3' } })
      ).toBe('/epr/p#step/3');
    });
    it('URI-encodes the id', () => {
      expect(eprToUniversalHref({ id: 'a b', tier: 'head' })).toBe('/epr/a%20b');
    });
  });
```

Update the spec's import line to include the new names:
`import { parseEpr, formatEpr, epr, eprToDid, eprToRoute, eprToUniversalHref, type BundleRouteContext } from './epr-ref';` (match the file's existing import style).

- [ ] **Step 3.2: Run, verify failure**

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm exec vitest run angular/utils/epr-ref
```

Expected: FAIL (compile — `eprToUniversalHref`/`BundleRouteContext` not exported; eprToRoute arity).

- [ ] **Step 3.3: Implement.** In `epr-ref.ts`, replace the whole `eprToRoute` function (lines 151-166) with:

```typescript
// ── §12.3 Mount-agnostic link minting ──────────────────────────────────────

/** A contentType this bundle renders natively, with its in-bundle route shape. */
export interface RouteClaim {
  contentType: string;
  /** Mint in-bundle router commands for a claimed ref. */
  commands(ref: EprRef): string[];
}

/**
 * Declared by each bundle's composition root (provide via BUNDLE_ROUTE_CONTEXT):
 * which EPR shapes this bundle renders natively. Unclaimed → /epr/{id}.
 */
export interface BundleRouteContext {
  claims: readonly RouteClaim[];
  /** The shell owns the universal epr/:resourceId route. */
  ownsUniversalRoute?: boolean;
}

/**
 * Result of claims-aware route minting. `commands` is non-null iff THIS bundle
 * claims the target (routerLink/router.navigate is safe); otherwise navigate
 * cross-bundle via `href` (plain anchor → epr-link interceptor, or
 * EprNavService.navigate → full doorway load).
 */
export interface EprRouteResolution {
  commands: string[] | null;
  /** Origin-absolute universal address — always present, safe in every bundle. */
  href: string;
  claimed: boolean;
}

/** Mint the universal EPR address (§12.1): /epr/{id}[#fragment]. */
export function eprToUniversalHref(ref: EprRef): string {
  const frag = ref.fragment ? `#${formatFragment(ref.fragment)}` : '';
  return `/epr/${encodeURIComponent(ref.id)}${frag}`;
}

/**
 * Convert an EprRef to a navigable resolution for THIS bundle (§12.3):
 * in-mount targets → relative commands; everything else → /epr/{id}.
 * Route shape comes from the EPR head's contentType (caller-supplied when
 * known) — never guessed from the id. The single sanctioned structural
 * inference: a `step` fragment implies contentType 'path'.
 * Returns null for blob tier (no page address).
 */
export function eprToRoute(
  ref: EprRef,
  ctx: BundleRouteContext,
  contentType?: string | null
): EprRouteResolution | null {
  if (ref.tier === 'blob') return null;

  const effectiveType = contentType ?? (ref.fragment?.type === 'step' ? 'path' : null);
  const href = eprToUniversalHref(ref);

  if (effectiveType) {
    const claim = ctx.claims.find(c => c.contentType === effectiveType);
    if (claim) {
      return { commands: claim.commands(ref), href, claimed: true };
    }
  }

  if (ctx.ownsUniversalRoute) {
    return { commands: ['/epr', ref.id], href, claimed: false };
  }

  return { commands: null, href, claimed: false };
}
```

(`formatFragment` already exists in this module's internal section — no change needed.)

Create `bundle-route-context.ts` (sibling file — keeps `epr-ref.ts` framework-free):

```typescript
import { InjectionToken } from '@angular/core';

import type { BundleRouteContext } from './epr-ref';

/**
 * Each bundle's composition root provides its route claims here (spec §12.3).
 * Default: nothing claimed, no universal route — every EPR target resolves
 * cross-bundle to /epr/{id}, which is always safe.
 */
export const BUNDLE_ROUTE_CONTEXT = new InjectionToken<BundleRouteContext>('BUNDLE_ROUTE_CONTEXT', {
  providedIn: 'root',
  factory: (): BundleRouteContext => ({ claims: [] }),
});
```

In `src/index.ts`, after line 168 (`export * from './angular/utils/epr-ref';`) add:

```typescript
export * from './angular/utils/bundle-route-context';
```

- [ ] **Step 3.4: Run, verify pass** — lib suite only (consumers are red until Tasks 4-10; that's expected):

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm test
```

Expected: PASS for the lib's own suite (epr-ref.spec green).

- [ ] **Step 3.5: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts \
        app/elohim-library/projects/elohim-service/src/angular/utils/bundle-route-context.ts \
        app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.spec.ts \
        app/elohim-library/projects/elohim-service/src/index.ts
git commit -m "feat(service)!: eprToRoute rewritten around BundleRouteContext claims — the type-vs-slug heuristic dies (§12.3)"
```

---

### Task 4: Shell adoption — context provider, resolver rewrite, component guards

**Files:**
- Modify: `app/elohim-app/src/app/app.config.ts` (provider)
- Modify: `app/elohim-app/src/app/elohim/services/epr-resolver.service.ts` (+ spec)
- Modify: `app/elohim-app/src/app/elohim/components/epr-resolve-redirect/epr-resolve-redirect.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/epr-popover/epr-popover.component.ts`

- [ ] **Step 4.1: Update epr-resolver.service.spec.ts first.** The `resolveInContext` describe (lines ~118-186) pins old shapes. Rewrite the standalone-branch assertions to the shell context and ADD a lamad-context block. Key changes:

```typescript
    it('resolves to standalone when no path context', () => {
      const result = service.resolveInContext('epr:rea-foundations', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'rea-foundations']); // shell owns /epr
      expect(result.href).toBe('/epr/rea-foundations');
    });

    it('resolves to standalone when target not in current path', () => {
      const result = service.resolveInContext('epr:unknown-content', 'my-path', steps);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'unknown-content']);
    });

    it('in-path target degrades to /epr in the shell (no path claim)', () => {
      const result = service.resolveInContext('epr:rea-foundations', 'my-path', steps);
      expect(result.resolution).toBe('in-path');
      expect(result.stepIndex).toBe(1);
      expect(result.route).toEqual(['/epr', 'my-path']); // shell mints universal
      expect(result.href).toBe('/epr/my-path#step/1');
    });

    it('uses eprToRoute for known path-type URIs', () => {
      const result = service.resolveInContext('epr:elohim-protocol#step/2', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'elohim-protocol']);
      expect(result.href).toBe('/epr/elohim-protocol#step/2');
    });
```

Mirror the same change in the cross-path test (`['/epr', 'other-path']` + `href '/epr/other-path#step/3'`). Then ADD a lamad-context block at the end of the describe — it proves the SAME service mints in-bundle path routes when the bundle claims `path`:

```typescript
    describe('with a path-claiming bundle context (lamad)', () => {
      let lamadService: EprResolverService;
      beforeEach(() => {
        TestBed.resetTestingModule();
        TestBed.configureTestingModule({
          providers: [
            // ...repeat the existing providers from the outer beforeEach...
            {
              provide: BUNDLE_ROUTE_CONTEXT,
              useValue: {
                claims: [
                  {
                    contentType: 'path',
                    commands: (ref: EprRef) =>
                      ref.fragment?.type === 'step'
                        ? ['/path', ref.id, 'step', ref.fragment.value]
                        : ['/path', ref.id],
                  },
                ],
              },
            },
          ],
        });
        lamadService = TestBed.inject(EprResolverService);
      });

      it('mints in-bundle step commands for in-path targets', () => {
        const result = lamadService.resolveInContext('epr:rea-foundations', 'my-path', steps);
        expect(result.route).toEqual(['/path', 'my-path', 'step', '1']);
      });

      it('mints no commands for unclaimed content (cross-bundle href)', () => {
        const result = lamadService.resolveInContext('epr:unknown-content', null, []);
        expect(result.route).toBeNull();
        expect(result.href).toBe('/epr/unknown-content');
      });
    });
```

(Adapt the inner `beforeEach` to replicate the file's existing TestBed providers; import `BUNDLE_ROUTE_CONTEXT`, `type EprRef` from `@elohim/service`.)

- [ ] **Step 4.2: Run, verify failure**

```bash
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/elohim/services/epr-resolver.service.spec.ts
```

- [ ] **Step 4.3: Implement the resolver rewrite.** In `epr-resolver.service.ts`:

(a) Imports: `import { type BundleRouteContext, BUNDLE_ROUTE_CONTEXT, type EprRef, eprToRoute, eprToUniversalHref, parseEpr } from '@elohim/service';` and inject: `private readonly routeCtx = inject(BUNDLE_ROUTE_CONTEXT);`

(b) Result types — `route` keeps its name; semantics become "in-bundle commands or null"; every shape gains `href`:

```typescript
export interface ResolvedEpr {
  ref: EprRef;
  url: string;
  /** In-bundle router commands (null = cross-bundle or blob tier) */
  route: string[] | null;
  /** Universal address — always navigable, in every bundle (§12.1) */
  href: string;
}

export interface ResolvedContent {
  ref: EprRef;
  content: StorageContentNode;
  blobUrl: string | null;
  route: string[] | null;
  href: string;
}

export interface ContextResolvedRoute {
  route: string[] | null;
  href: string;
  resolution: 'in-path' | 'cross-path' | 'standalone';
  stepIndex?: number;
  crossPath?: { pathId: string; stepIndex: number };
}
```

(c) `resolveUrl` (line ~113):

```typescript
  resolveUrl(input: string, blobHash?: string): ResolvedEpr {
    const ref = parseEpr(input);
    const res = eprToRoute(ref, this.routeCtx);
    return {
      ref,
      url: this.buildUrl(ref, blobHash),
      route: res?.commands ?? null,
      href: res?.href ?? eprToUniversalHref(ref),
    };
  }
```

(d) `resolve` (line ~136) — the fetched head's contentType drives the claim:

```typescript
        const blobHash = this.extractBlobHash(content);
        const blobUrl = blobHash ? this.storage.getBlobUrl(blobHash) : null;
        const res = eprToRoute(ref, this.routeCtx, content.contentType);
        return of({
          ref,
          content,
          blobUrl,
          route: res?.commands ?? null,
          href: res?.href ?? eprToUniversalHref(ref),
        });
```

(e) `resolveInContext` (lines ~178-216) — the in-path/cross-path branches mint through the claims context (the inline `['/path', …]` literals and their `TODO(#12-6 Slice 2)` comments die):

```typescript
  resolveInContext(
    input: string,
    currentPathId: string | null,
    currentSteps: StepRef[],
    crossPathMatches?: CrossPathMatch[]
  ): ContextResolvedRoute {
    const ref = parseEpr(input);
    const targetId = ref.id;

    const stepResolution = (pathId: string, stepIndex: number): Omit<ContextResolvedRoute, 'resolution'> => {
      const stepRef: EprRef = {
        id: pathId,
        tier: 'head',
        fragment: { type: 'step', value: String(stepIndex) },
      };
      const res = eprToRoute(stepRef, this.routeCtx, 'path');
      return {
        route: res?.commands ?? null,
        href: res?.href ?? eprToUniversalHref(stepRef),
        stepIndex,
      };
    };

    // 1. Check current path for the target content
    if (currentPathId) {
      const stepIndex = currentSteps.findIndex(s => s.resourceId === targetId);
      if (stepIndex >= 0) {
        const { stepIndex: _ignored, ...rest } = stepResolution(currentPathId, stepIndex);
        return { ...rest, resolution: 'in-path', stepIndex };
      }
    }

    // 2. Check cross-path matches (if provided by caller)
    if (crossPathMatches && crossPathMatches.length > 0) {
      const match = crossPathMatches[0];
      const { stepIndex: _ignored, ...rest } = stepResolution(match.pathId, match.stepIndex);
      return { ...rest, resolution: 'cross-path', crossPath: match };
    }

    // 3. Standalone resource view (fallback)
    const res = eprToRoute(ref, this.routeCtx);
    return {
      route: res?.commands ?? null,
      href: res?.href ?? eprToUniversalHref(ref),
      resolution: 'standalone',
    };
  }
```

(f) `app.config.ts` providers array — add (import `BUNDLE_ROUTE_CONTEXT, type BundleRouteContext` from `@elohim/service`):

```typescript
    // §12.3: the shell claims nothing by type — it OWNS the universal /epr
    // route, so every unclaimed EPR resolves in-shell to ['/epr', id].
    {
      provide: BUNDLE_ROUTE_CONTEXT,
      useValue: { claims: [], ownsUniversalRoute: true } satisfies BundleRouteContext,
    },
```

(g) `epr-resolve-redirect.component.ts` (line ~32-35):

```typescript
    const ref = parseEpr(cleaned);
    const res = eprToRoute(ref, this.routeCtx);
    void this.router.navigate(res?.commands ?? ['/epr', ref.id]);
```

(inject `routeCtx` via `inject(BUNDLE_ROUTE_CONTEXT)`; update imports accordingly.)

(h) `epr-link.component.ts` — all three `resolve(...).subscribe` navigation sites (navigateListener ~116, the `network` case ~185, `navigateToResource` ~211) get the null-guard. In the shell `route` is always non-null (ownsUniversalRoute), but the component must stay correct under any context:

```typescript
    this.eprResolver.resolve(epr).subscribe(resolved => {
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      } else if (resolved) {
        this.eprNav.navigate(resolved.href);
      }
    });
```

(`network` case keeps its `{ fragment: 'network' }` option on the navigate call; inject `EprNavService` as `private readonly eprNav = inject(EprNavService);`.)

(i) `epr-popover.component.ts` — add an `href` escape hatch beside the commands input (line ~230 + template ~94):

```typescript
  /** In-bundle router commands; null when the target is cross-bundle. */
  @Input() route: string[] | null = null;
  /** Universal address fallback — used when route is null (cross-bundle). */
  @Input() href: string | null = null;
```

```html
      <!-- Footer: link to full resource -->
      <a *ngIf="route" [routerLink]="route" class="epr-popover-link" data-testid="epr-popover-link">
        Open resource
      </a>
      <a *ngIf="!route && href" [href]="href" class="epr-popover-link" data-testid="epr-popover-link">
        Open resource
      </a>
```

- [ ] **Step 4.4: Run, verify pass**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/elohim/services/epr-resolver.service.spec.ts
pnpm exec vitest run --config vite.config.ts src/app/elohim/components/epr-link/epr-link.component.spec.ts
pnpm exec vitest run --config vite.config.ts src/app/elohim/components/epr-popover/epr-popover.component.spec.ts
pnpm exec vitest run --config vite.config.ts src/app/elohim/components/epr-resolve-redirect
```

Fix any spec mocks in these files that pin `route: ['/resource', …]` shapes (update to `route: ['/epr', …], href: '/epr/…'`).

- [ ] **Step 4.5: Commit**

```bash
git add app/elohim-app/src/app/app.config.ts \
        app/elohim-app/src/app/elohim/services/epr-resolver.service.ts \
        app/elohim-app/src/app/elohim/services/epr-resolver.service.spec.ts \
        app/elohim-app/src/app/elohim/components/epr-resolve-redirect/ \
        app/elohim-app/src/app/elohim/components/epr-link/ \
        app/elohim-app/src/app/elohim/components/epr-popover/
git commit -m "feat(app): claims-aware EPR resolution — shell mints /epr universal routes (§12.3)"
```

---

### Task 5: `EprNavService.ownsPath` — pathless layout-root descent

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/epr-nav.service.ts` (ownsPath, lines 25-32)
- Modify: `app/elohim-app/src/app/elohim/services/epr-nav.service.spec.ts`

**Why:** lamad's router config is `[{ path: '', children: [path/:pathId, explore, …] }]` — a pathless layout root. The current `ownsPath` checks only top-level `r.path` first segments, so inside lamad it would return `false` for `/path/x` (a route lamad OWNS) and full-load every in-bundle nav. The shell's flat config is unaffected.

- [ ] **Step 5.1: Write the failing test** (append to epr-nav.service.spec.ts, matching its existing TestBed style):

```typescript
  describe('ownsPath with a pathless layout root (pillar-bundle shape)', () => {
    beforeEach(() => {
      // lamad-shaped config: everything hangs off a path:'' layout root.
      router.resetConfig([
        {
          path: '',
          children: [
            { path: 'path/:pathId', children: [] },
            { path: 'explore', children: [] },
            { path: 'resource/:resourceId/edit', children: [] },
            { path: '**', children: [] },
          ],
        },
      ]);
    });

    it('owns routes declared under the layout root', () => {
      expect(service.ownsPath('/path/foundations/step/0')).toBe(true);
      expect(service.ownsPath('/explore')).toBe(true);
      expect(service.ownsPath('/resource/abc/edit')).toBe(true);
    });

    it('does not own foreign top segments', () => {
      expect(service.ownsPath('/epr/abc')).toBe(false);
      expect(service.ownsPath('/identity/login')).toBe(false);
    });
  });
```

(Use the spec file's existing `router` handle; if it uses a mock Router, switch these tests to `TestBed.inject(Router)` with `provideRouter([])` + `resetConfig` — follow whichever harness the file already uses.)

- [ ] **Step 5.2: Run, verify failure**

```bash
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/elohim/services/epr-nav.service.spec.ts
```

- [ ] **Step 5.3: Implement** — replace `ownsPath` (epr-nav.service.ts:25-32):

```typescript
  ownsPath(path: string): boolean {
    const top = path.replace(/^\//, '').split(/[/?#]/)[0] ?? '';
    if (top === '') return true; // root landing is bundle-owned
    const matches = (routes: readonly Route[] | undefined): boolean =>
      !!routes?.some(r => {
        // Pathless layout roots (pillar-bundle shape) — descend into children.
        if (r.path === '' && r.children) return matches(r.children);
        if (!r.path || r.path === '**') return false;
        return r.path.split('/')[0] === top;
      });
    return matches(this.router.config);
  }
```

(add `import type { Route } from '@angular/router';`)

- [ ] **Step 5.4: Run, verify pass** (same command — ALL epr-nav tests, old and new).

- [ ] **Step 5.5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/epr-nav.service.ts app/elohim-app/src/app/elohim/services/epr-nav.service.spec.ts
git commit -m "fix(app): EprNavService.ownsPath descends pathless layout roots — correct in pillar bundles"
```

---

### Task 6: Lamad wiring — `LAMAD_EPR_NAV` token + route claims

**Files:**
- Modify: `app/lamad/src/app/interfaces/cross-pillar.interface.ts` (token, near LAMAD_EPR_RESOLVER at line ~375)
- Modify: `app/lamad/src/app/app.config.ts` (two providers)

- [ ] **Step 6.1: Add the token + interface** in `cross-pillar.interface.ts` next to `LAMAD_EPR_RESOLVER`:

```typescript
/** EPR-aware navigation (§12.3): same-bundle → router; cross-bundle → handoff + full load. */
export interface ILamadEprNav {
  ownsPath(path: string): boolean;
  navigate(pathOrCommands: string | readonly unknown[]): void;
  recordHandoff(): void;
}

export const LAMAD_EPR_NAV = new InjectionToken<ILamadEprNav>('LamadEprNav');
```

- [ ] **Step 6.2: Bind in `app.config.ts`** (composition root — the ONE sanctioned place for `@app/elohim` imports). Add to the imports: `EprNavService` from `@app/elohim/services/epr-nav.service`, `LAMAD_EPR_NAV` from `./interfaces/cross-pillar.interface`, and `BUNDLE_ROUTE_CONTEXT, type BundleRouteContext, type EprRef` from `@elohim/service`. Add to providers (next to the LAMAD_EPR_RESOLVER binding at line ~165):

```typescript
    { provide: LAMAD_EPR_NAV, useExisting: EprNavService },
    // §12.3: lamad claims contentType 'path' — everything else is cross-bundle.
    {
      provide: BUNDLE_ROUTE_CONTEXT,
      useValue: {
        claims: [
          {
            contentType: 'path',
            commands: (ref: EprRef) =>
              ref.fragment?.type === 'step'
                ? ['/path', ref.id, 'step', ref.fragment.value]
                : ['/path', ref.id],
          },
        ],
      } satisfies BundleRouteContext,
    },
```

- [ ] **Step 6.3: Verify compile + existing suites still green**

```bash
cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts src/app/components/lamad-layout
```

- [ ] **Step 6.4: Commit**

```bash
git add app/lamad/src/app/interfaces/cross-pillar.interface.ts app/lamad/src/app/app.config.ts
git commit -m "feat(lamad): LAMAD_EPR_NAV token + path route-claim — composition-root wiring (§12.3)"
```

---

### Task 7: Lamad sweep A — template links

**Files (each with its spec):**
- `components/path-navigator/path-navigator.component.html:173-183`
- `components/path-overview/path-overview.component.html:416-423`
- `components/attention-flow/attention-flow.component.html:17-23`
- `components/learner-dashboard/refresh-queue/refresh-queue.component.ts:45-52` (inline template)
- `components/search/search.component.ts:34 + 174-182` (+ spec line 163)
- `components/profile-page/profile-page.component.html:22-30 + 479-491`

Cross-bundle template anchors become **plain `href` to the universal address** — the capture-phase epr-link interceptor (auto-installed via lamad's `<elohim-page-chrome>`) records the handoff and full-loads through the doorway. `data-testid`s are preserved (testid-sync).

- [ ] **Step 7.1: path-navigator.component.html** — replace lines 175-179:

```html
              <a
                [attr.href]="'/epr/' + stepView.content.id"
                class="view-resource-link"
                data-testid="path-nav-view-resource"
              >
```

- [ ] **Step 7.2: path-overview.component.html** — replace lines 416-419:

```html
        <a
          [attr.href]="'/epr/path-' + pathId"
          class="view-resource-link"
          data-testid="overview-view-as-content"
        >
```

- [ ] **Step 7.3: attention-flow.component.html** — replace the routerLink line:

```html
        [attr.href]="'/epr/' + event.contentId"
```

- [ ] **Step 7.4: refresh-queue.component.ts** inline template — replace the routerLink line:

```html
                <a
                  [attr.href]="'/epr/' + item.contentId"
                  class="practice-link"
                  title="Practice this content"
                >
```

(Remove `RouterLink`/`RouterModule` from each component's `imports` array IF this was its only router-directive usage — check per component; leave it when other routerLinks remain.)

- [ ] **Step 7.5: search.component.ts** — split path (claimed, routerLink) from content (cross-bundle, href). Replace `getNodeRoute` (lines 174-182):

```typescript
  /** In-bundle commands for path results; null for cross-bundle content (§12.3). */
  getNodeCommands(result: SearchResult): string[] | null {
    return result.contentType === 'path' ? ['/path', result.id] : null;
  }

  /** Universal address for cross-bundle content results. */
  getNodeHref(result: SearchResult): string {
    return `/epr/${encodeURIComponent(result.id)}`;
  }
```

Replace the inline-template anchor (line ~34):

```html
          @for (result of results; track result.id) {
            @if (getNodeCommands(result); as commands) {
              <a [routerLink]="commands" class="result-card">
```
…(existing card body unchanged)…
```html
              </a>
            } @else {
              <a [attr.href]="getNodeHref(result)" class="result-card">
```
…(same card body — duplicate the existing inner markup verbatim)…
```html
              </a>
            }
          }
```

(If the file still uses `*ngFor`, convert just this block to `@for/@if` or use two `*ngIf`'d anchors — match the file's prevailing syntax. Keep the card body identical in both branches.)

Update `search.component.spec.ts:163`: `expect(route).toEqual(['/resource', 'test-id'])` becomes:

```typescript
      expect(component.getNodeCommands(contentResult)).toBeNull();
      expect(component.getNodeHref(contentResult)).toBe('/epr/test-id');
```

(and keep/adjust the path-branch assertion `getNodeCommands(pathResult) → ['/path', 'path-id']`.)

- [ ] **Step 7.6: profile-page.component.html** — line 24: `routerLink="/identity/profile"` → `href="/identity/profile"` (shell mount URL; interceptor handles). Lines 479-491 ternary → split:

```html
                  @if (event.resourceId) {
                    @if (event.resourceType === 'path') {
                      <a [routerLink]="['/path', event.resourceId]" class="timeline-event-link">
                        View {{ event.resourceType }}
                        <span class="material-icons">open_in_new</span>
                      </a>
                    } @else {
                      <a [attr.href]="'/epr/' + event.resourceId" class="timeline-event-link">
                        View {{ event.resourceType }}
                        <span class="material-icons">open_in_new</span>
                      </a>
                    }
                  }
```

- [ ] **Step 7.7: Run the touched specs; fix canaries in the same commit**

```bash
cd /projects/elohim/app/lamad
pnpm exec vitest run --config vite.config.ts src/app/components/search src/app/components/path-navigator src/app/components/path-overview src/app/components/attention-flow src/app/components/learner-dashboard src/app/components/profile-page
```

Expected canary updates: `search.component.spec.ts:163` (Step 7.5); any template-binding snapshot failures in the five swept components.

- [ ] **Step 7.8: Commit**

```bash
git add app/lamad/src/app/components/path-navigator/path-navigator.component.html \
        app/lamad/src/app/components/path-overview/path-overview.component.html \
        app/lamad/src/app/components/attention-flow/attention-flow.component.html \
        app/lamad/src/app/components/learner-dashboard/refresh-queue/refresh-queue.component.ts \
        app/lamad/src/app/components/search/search.component.ts app/lamad/src/app/components/search/search.component.spec.ts \
        app/lamad/src/app/components/profile-page/profile-page.component.html
git commit -m "fix(lamad): template sweep — cross-bundle anchors mint the universal /epr address (§12.3)"
```

---

### Task 8: Lamad sweep B — programmatic navigation + guard + path-context

**Files (each with its spec):**
- `components/content-editor-page/content-editor-page.component.ts:135-137`
- `components/content-viewer/content-viewer.component.ts:968-979 (handleAction), 1039-1048 (viewRelatedContent), 1233-1245 (onGraphNodeSelected)`
- `components/path-navigator/path-navigator.component.ts:860-873 (onExploreContent)`
- `components/path-overview/path-overview.component.ts:735-743 (goToConcept)`
- `components/graph-explorer/graph-explorer.component.ts:827-829`
- `components/meaning-map/meaning-map.component.ts:186-188`
- `components/profile-page/profile-page.component.ts:284-291`
- `services/path-context.service.ts:163-180, 244-263`
- `guards/lamad-identity.guard.ts`

**Pattern:** every component injects the nav seam once — `private readonly eprNav: ILamadEprNav = inject(LAMAD_EPR_NAV);` (imports from `'../../interfaces/cross-pillar.interface'` — adjust relative depth per file) — and cross-bundle navigations become `this.eprNav.navigate(...)`. In-bundle navigations (`['/path', …]`, `['/']`, `['/explore']`) are NOT touched.

- [ ] **Step 8.1: content-editor-page** `navigateBack()`:

```typescript
  navigateBack(): void {
    this.eprNav.navigate(`/epr/${encodeURIComponent(this.resourceId)}`);
  }
```

- [ ] **Step 8.2: content-viewer** — three sites:

```typescript
  handleAction(action: { route?: string }): void {
    if (action.route) {
      // Dynamic target (trust-badge config) — eprNav decides in-bundle vs handoff.
      this.eprNav.navigate(action.route);
    }
  }
```

```typescript
  viewRelatedContent(node: ContentNode): void {
    this.eprNav.navigate(`/epr/${encodeURIComponent(node.id)}`);
  }
```

`onGraphNodeSelected` (line ~1245) — keep the `startDetour` tracking call as-is, replace only the navigation line:

```typescript
    // Navigate to the selected content (cross-bundle: shell resource viewer).
    this.eprNav.navigate(`/epr/${encodeURIComponent(nodeId)}`);
```

- [ ] **Step 8.3: path-navigator** `onExploreContent` — keep `startDetour`, replace the navigate+catch:

```typescript
    // Navigate to the content (cross-bundle handoff — full doorway load).
    this.eprNav.navigate(`/epr/${encodeURIComponent(contentId)}`);
```

- [ ] **Step 8.4: path-overview** `goToConcept` fallback branch:

```typescript
    } else {
      // Fallback to the universal resource view if no matching step found
      this.eprNav.navigate(`/epr/${encodeURIComponent(conceptId)}`);
    }
```

- [ ] **Step 8.5: graph-explorer** `navigateToContent` and **meaning-map** `viewContent`:

```typescript
  navigateToContent(nodeId: string): void {
    this.eprNav.navigate(`/epr/${encodeURIComponent(nodeId)}`);
  }
```

```typescript
  viewContent(node: ContentNodeWithAffinity): void {
    this.eprNav.navigate(`/epr/${encodeURIComponent(node.id)}`);
  }
```

- [ ] **Step 8.6: profile-page** programmatic identity navs:

```typescript
  goToIdentityProfile(): void {
    this.eprNav.navigate('/identity/profile');
  }

  /** Navigate to registration for network upgrade */
  onJoinNetwork(): void {
    this.eprNav.navigate('/identity/register');
  }
```

- [ ] **Step 8.7: path-context.service.ts** — the detour-return/breadcrumb mint sites switch to universal commands (consumed by callers that route through eprNav or router; `['/epr', id]` via `EprNavService.navigate` full-loads correctly from lamad):

Line ~175: `return ['/resource', previousDetour.toContentId];` → `return ['/epr', previousDetour.toContentId];`
Line ~256: `route: ['/resource', detour.toContentId],` → `route: ['/epr', detour.toContentId],`

Find the consumers of `returnFromDetour()`/breadcrumb `route` arrays (grep `returnFromDetour\(|\.route` in components) and ensure each navigates via `this.eprNav.navigate(route)` rather than `router.navigate(route)` — `eprNav` handles both in-bundle `['/path',…]` returns and cross-bundle `['/epr',…]` detour returns through one seam. Update `path-context.service.spec.ts:247` (`['/resource', 'related-concept']` → `['/epr', 'related-concept']`).

**Behavioral note (document in the commit body):** cross-bundle detours full-load into the shell, so lamad's in-memory detour stack does not survive them; the back-affordance is the session-nav-stack handoff. This was ALREADY broken (self-loop redirect) — the sweep makes the behavior honest.

- [ ] **Step 8.8: lamad-identity.guard.ts** — a UrlTree cannot escape the bundle. Replace the guard body:

```typescript
/** Login route for unauthenticated users (shell mount — cross-bundle from lamad). */
const LOGIN_ROUTE = '/identity/login';

export const lamadIdentityGuard: CanActivateFn = (route, state): boolean => {
  const identityService = inject(LAMAD_IDENTITY);
  const eprNav = inject(LAMAD_EPR_NAV);

  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  // /identity lives in the shell: full-load handoff with the PUBLIC return URL
  // (state.url is base-stripped — re-prefix the /lamad mount).
  const returnUrl = encodeURIComponent(`/lamad${state.url}`);
  eprNav.navigate(`${LOGIN_ROUTE}?returnUrl=${returnUrl}`);
  return false;
};
```

(Remove the now-unused `Router`/`UrlTree` imports; update the guard's spec to assert `eprNav.navigate` was called with the prefixed returnUrl and the guard returned `false`.)

- [ ] **Step 8.9: Update the spec canaries in the same commit** (from the inventory; exact assertions):
- `content-viewer.component.spec.ts:762, :804` — `['/resource', 'related-node']` → assert the eprNav spy: `expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/related-node')`
- `content-viewer.component.spec.ts:398` — `['/content', 'related-1']` → `'/epr/related-1'`
- `path-navigator.component.spec.ts:593` — → `'/epr/related-node-1'`
- `graph-explorer.component.spec.ts:263, :763` — → `'/epr/concept-1'` / `'/epr/node-123'`
- `meaning-map.component.spec.ts:196` — → `'/epr/node-1'`
- `profile-page.component.spec.ts:182, :187` — `router.navigate(['/identity/profile'])` → `eprNavSpy.navigate('/identity/profile')` / `'/identity/register'`
- `path-context.service.spec.ts:247` — Step 8.7

Each touched component spec needs a `LAMAD_EPR_NAV` provider: `{ provide: LAMAD_EPR_NAV, useValue: { navigate: vi.fn(), ownsPath: vi.fn(() => true), recordHandoff: vi.fn() } }`. The `/path/...` assertions (content-viewer:403/:730, path-navigator:246/260/274/573, graph-explorer:488) are in-bundle and MUST NOT change — if one fails, the sweep over-reached.

- [ ] **Step 8.10: Run, verify pass**

```bash
cd /projects/elohim/app/lamad
pnpm exec vitest run --config vite.config.ts src/app/components src/app/services/path-context.service.spec.ts src/app/guards
```

- [ ] **Step 8.11: Commit**

```bash
git add app/lamad/src/app/components app/lamad/src/app/services/path-context.service.ts \
        app/lamad/src/app/services/path-context.service.spec.ts app/lamad/src/app/guards
git commit -m "fix(lamad): programmatic sweep — cross-bundle navs ride LAMAD_EPR_NAV; identity guard full-load handoff (§12.3)

Cross-bundle detours full-load into the shell; the in-memory detour stack
does not survive the boundary (back-affordance = session-nav-stack). This
was already broken via the resource self-loop — the sweep makes it honest."
```

---

### Task 9: Lamad legacy bridge — kill the self-loop redirect

**Files:**
- Create: `app/lamad/src/app/components/legacy-resource-redirect/legacy-resource-redirect.component.ts` (+ `.spec.ts`)
- Modify: `app/lamad/src/app/lamad.routes.ts:64-69`
- Modify: `app/lamad/src/app/lamad.routes.spec.ts:36-37`

- [ ] **Step 9.1: Write the failing component spec**

```typescript
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { LAMAD_EPR_NAV } from '../../interfaces/cross-pillar.interface';
import { LegacyResourceRedirectComponent } from './legacy-resource-redirect.component';

describe('LegacyResourceRedirectComponent', () => {
  const eprNav = { navigate: vi.fn(), ownsPath: vi.fn(() => false), recordHandoff: vi.fn() };

  beforeEach(() => {
    eprNav.navigate.mockClear();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([
          { path: 'resource/:resourceId', component: LegacyResourceRedirectComponent },
        ]),
        { provide: LAMAD_EPR_NAV, useValue: eprNav },
      ],
    });
  });

  it('bridges the legacy /lamad/resource URL to the universal address', async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl('/resource/fct-module-01-church-dilemma');
    expect(eprNav.navigate).toHaveBeenCalledWith('/epr/fct-module-01-church-dilemma');
  });
});
```

- [ ] **Step 9.2: Run, verify failure** (`pnpm exec vitest run --config vite.config.ts src/app/components/legacy-resource-redirect` — module not found)

- [ ] **Step 9.3: Implement the component**

```typescript
import { Component, OnInit, inject } from '@angular/core';
import { ActivatedRoute } from '@angular/router';

import { ILamadEprNav, LAMAD_EPR_NAV } from '../../interfaces/cross-pillar.interface';

/**
 * Legacy URL bridge (§12.6 Slice 2): /lamad/resource/{id} was the monolith-era
 * canonical content URL — real shares exist. The viewer is now shell-owned at
 * the universal /epr/{id} address; this route hands off across the bundle
 * boundary. (Replaces the absolute redirectTo that could never escape this
 * router and self-looped.)
 */
@Component({
  selector: 'app-legacy-resource-redirect',
  standalone: true,
  template: '',
})
export class LegacyResourceRedirectComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly eprNav: ILamadEprNav = inject(LAMAD_EPR_NAV);

  ngOnInit(): void {
    const id = this.route.snapshot.params['resourceId'] as string;
    this.eprNav.navigate(`/epr/${encodeURIComponent(id)}`);
  }
}
```

- [ ] **Step 9.4: Swap the route** — `lamad.routes.ts:65-69` becomes:

```typescript
      // Legacy bridge: /lamad/resource/{id} (monolith-era canonical) hands off
      // to the universal /epr/{id} address. NOTE: an absolute redirectTo here
      // re-enters THIS router and self-loops — it can never escape the bundle.
      {
        path: 'resource/:resourceId',
        loadComponent: async () =>
          import('./components/legacy-resource-redirect/legacy-resource-redirect.component').then(
            m => m.LegacyResourceRedirectComponent
          ),
      },
```

Update `lamad.routes.spec.ts:36-37` to pin the new shape:

```typescript
    const resourceRoute = children?.find(r => r.path === 'resource/:resourceId');
    expect(resourceRoute).toBeDefined();
    expect(resourceRoute?.redirectTo).toBeUndefined(); // self-loop killed (§12.6 Slice 2)
    expect(resourceRoute?.loadComponent).toBeDefined();
```

- [ ] **Step 9.5: Run, verify pass**

```bash
cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts src/app/components/legacy-resource-redirect src/app/lamad.routes.spec.ts
```

- [ ] **Step 9.6: Commit**

```bash
git add app/lamad/src/app/components/legacy-resource-redirect app/lamad/src/app/lamad.routes.ts app/lamad/src/app/lamad.routes.spec.ts
git commit -m "fix(lamad): legacy /lamad/resource bridge — kill the self-loop redirectTo (§12.6 Slice 2)"
```

---

### Task 10: Lamad resolver-driven consumers — null-guard + href

**Files:**
- `app/lamad/src/app/renderers/markdown-renderer/markdown-renderer.component.ts:431-440, 530-534, 577-582`
- `app/lamad/src/app/quiz-engine/components/recommendation-list/recommendation-list.component.ts:134-141`
- `app/lamad/src/app/components/lesson-view/lesson-view.component.ts` (locate its `resolved.route` nav site near the resolver injection at line ~691)
- Spec mocks: `content-viewer.component.spec.ts:252`, `lesson-view.component.spec.ts:103`

These already navigate resolver OUTPUT — with the Task 4 rewrite, lamad's resolver returns `route: ['/path', …]` for claimed targets and `route: null + href` for cross-bundle. Consumers need the null branch.

- [ ] **Step 10.1: markdown-renderer** — inject `private readonly eprNav: ILamadEprNav = inject(LAMAD_EPR_NAV);`. Site 1 (line ~439):

```typescript
    this.destroyPopover();
    if (resolved.route) {
      void this.router.navigate(resolved.route);
    } else {
      this.eprNav.navigate(resolved.href);
    }
```

Site 2 (lines ~530-534) — the popover link derivation simplifies to the resolver's own fields:

```typescript
    // Route (in-bundle) or universal href (cross-bundle) for the "Open resource" link
    const { route, href } = this.eprResolver.resolveUrl(eprUri);
    const routeHref = route ? '/' + route.filter(Boolean).join('/').replace(/^\/+/, '') : href;
```

(then pass `route` and `href` through to wherever the popover inputs are set — popover gets `[route]` when non-null, `[href]` otherwise, matching the Task 4 popover API.)

Site 3 (lines ~577-582, `onNavigate`):

```typescript
      const onNavigate = (): void => {
        this.destroyPopover();
        if (route) {
          void this.router.navigate(route);
        } else if (href) {
          this.eprNav.navigate(href);
        }
      };
```

- [ ] **Step 10.2: recommendation-list** `navigateListener` (line ~134):

```typescript
  private readonly navigateListener = (e: Event): void => {
    const epr = (e as CustomEvent<{ epr: string }>).detail.epr;
    this.eprResolver.resolve(epr).subscribe(resolved => {
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      } else if (resolved) {
        this.eprNav.navigate(resolved.href);
      }
    });
  };
```

(inject `LAMAD_EPR_NAV` as in Task 8.)

- [ ] **Step 10.3: lesson-view** — locate the `resolved.route` consumer (grep `resolve(` / `.route` near line 691's resolver) and apply the same null-guard pattern. Update its spec mock (`lesson-view.component.spec.ts:103`): `route: ['/resource', 'bar']` → `route: null, href: '/epr/bar'` and assert `eprNav.navigate('/epr/bar')`.

- [ ] **Step 10.4: content-viewer spec mock** (`content-viewer.component.spec.ts:252`): `route: ['/resource', 'foo']` → `route: null, href: '/epr/foo'` (adjust the downstream assertion to the eprNav spy if that test asserts navigation).

- [ ] **Step 10.5: Run, verify pass**

```bash
cd /projects/elohim/app/lamad
pnpm exec vitest run --config vite.config.ts src/app/renderers src/app/quiz-engine src/app/components/lesson-view src/app/components/content-viewer
```

- [ ] **Step 10.6: Run BOTH full suites** (shared-lib signature change ripples everywhere — this is the checkpoint):

```bash
cd /projects/elohim/app/lamad && pnpm test
cd /projects/elohim/app/elohim-app && pnpm test
```

Expected: green (pre-existing failures only if any — record them). Watch for the zone.js native-await phantom-uncaught under load: if an innocent test fails with an unhandled-rejection flag, fix component-side with sync `.then/.catch` attach (never fakeAsync-swap).

- [ ] **Step 10.7: Commit**

```bash
git add app/lamad/src/app/renderers app/lamad/src/app/quiz-engine app/lamad/src/app/components/lesson-view app/lamad/src/app/components/content-viewer
git commit -m "fix(lamad): resolver-driven consumers honor claims — route null-guard + universal href fallback (§12.3)"
```

---

### Task 11: Lit navigator blank-slate — host-supplied routes

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.ts` (DEFAULT_CONTEXT_APPS:39-73; contextApps default:438; tray literals:500-540)
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts` (fixtures + :88/:112)
- Modify: `app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts` (JSDoc note at ~139)
- Modify: `app/lamad/src/app/components/lamad-layout/lamad-layout.component.{ts,html}` (+ spec)
- Modify: `app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-navigator.default.stories.ts`

**Scope guard:** this targets ONLY the Lit `<elohim-navigator>`. The Angular `<app-elohim-navigator>` (elohim-app legacy, used by qahal/shefa/avodah layouts) is a separate codepath — Step 11.7 is a verify-only check there.

- [ ] **Step 11.1: Update the element specs first.** In `elohim-navigator.spec.ts`, define a shared fixture config near the top:

```typescript
const FIXTURE_APPS = [
  { id: 'lamad', name: 'Lamad', icon: '📚', route: '/lamad', tagline: 'Learning & Content', available: true },
  { id: 'shefa', name: 'Shefa', icon: '✨', route: '/shefa', tagline: 'Economics of Flourishing', available: true },
];
const FIXTURE_IDENTITY = { profile: '/identity/profile', login: '/identity/login', register: '/identity/register' };
```

Update the two context-switch tests (:67-113) to render `<elohim-navigator .contextApps=${FIXTURE_APPS}></elohim-navigator>` (assertions on `/shefa` unchanged). Update the tray tests (nav-login :157, nav-logout :177 fixtures) to pass `.identityRoutes=${FIXTURE_IDENTITY}`. ADD two blank-slate proofs:

```typescript
  it('renders no context switcher without host-supplied apps (blank-slate)', async () => {
    const el = await fixture<ElohimNavigator>(html`<elohim-navigator></elohim-navigator>`);
    expect(el.shadowRoot?.querySelector('[part="context-switcher-btn"]')).to.equal(null);
  });

  it('renders no identity tray items without host-supplied routes (blank-slate)', async () => {
    const el = await fixture<ElohimNavigator>(html`<elohim-navigator></elohim-navigator>`);
    const trayBtn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="nav-login"]');
    expect(trayBtn).to.equal(null);
  });
```

- [ ] **Step 11.2: Run, verify failure** (`cd /projects/elohim/app/elohim-elements/elohim-core && pnpm test`)

- [ ] **Step 11.3: Implement the element changes.**

(a) Delete the `DEFAULT_CONTEXT_APPS` const (lines 39-73). Change the property default (line 438):

```typescript
  /** Host-supplied context apps (§12.3: no app routing baked into primitives). Empty → switcher hidden. */
  @property({ attribute: false }) contextApps: ElohimContextAppConfig[] = [];
```

(b) Add the identity-routes property beside it:

```typescript
  /** Host-supplied identity surface routes; absent entries render no tray item. */
  @property({ attribute: false }) identityRoutes: { profile?: string; login?: string; register?: string } = {};
```

(c) Guard the context-switcher render: wrap the switcher button/panel render with `${this.contextApps.length > 0 ? html`…existing markup…` : nothing}` (import `nothing` from `lit`).

(d) In `_renderProfileTrayContent` (lines ~492-541) replace the three literals; render each item only when its route is supplied:

```typescript
        ${this.identityRoutes.profile
          ? html`
              <button
                class="tray-item"
                type="button"
                role="menuitem"
                data-testid="nav-identity-profile"
                @click=${() => this.navigate(this.identityRoutes.profile!)}
              >
                Identity Profile
              </button>
              <div class="tray-divider"></div>
            `
          : nothing}
```

…and the anonymous branch:

```typescript
    return html`
      ${this.identityRoutes.login
        ? html`
            <button class="tray-item" type="button" role="menuitem" data-testid="nav-login"
              @click=${() => this.navigate(this.identityRoutes.login!)}>
              Sign in
            </button>
          `
        : nothing}
      ${this.identityRoutes.register
        ? html`
            <button class="tray-item" type="button" role="menuitem" data-testid="nav-register"
              @click=${() => this.navigate(this.identityRoutes.register!)}>
              Register
            </button>
          `
        : nothing}
    `;
```

(Keep the existing Sign-out button untouched — it calls `handleLogout`, not a route.)

(e) `elohim-default-omnibar.ts` (~line 140) — JSDoc note above the anchor, no behavior change:

```typescript
                <!-- /auth/* is doorway-owned service vocabulary (same class as
                     /epr) — uniform across deployments, NOT app routing. -->
                <a href="/auth/signin">sign in</a>
```

(Lit comment syntax inside html`` — use `<!-- -->`; verify the existing template style.)

- [ ] **Step 11.4: Host supply in lamad-layout.** In `lamad-layout.component.ts` add fields (import `type ElohimContextAppConfig` from `'elohim-core'`):

```typescript
  /** Host-supplied navigator config (§12.3: routing lives in the host, not the primitive). */
  readonly navigatorApps: ElohimContextAppConfig[] = [
    { id: 'lamad', name: 'Lamad', icon: '📚', route: '/lamad', tagline: 'Learning & Content', available: true },
    { id: 'community', name: 'Qahal', icon: '👥', route: '/community', tagline: 'Community & Governance', available: true },
    { id: 'shefa', name: 'Shefa', icon: '✨', route: '/shefa', tagline: 'Economics of Flourishing', available: true },
    { id: 'avodah', name: 'Avodah', icon: '🔨', route: '/avodah', tagline: 'Work & Stewardship', available: true },
    { id: 'map', name: 'Map', icon: '🌍', route: '/map', tagline: 'Living Places', available: true },
  ];

  readonly navigatorIdentityRoutes = {
    profile: '/identity/profile',
    login: '/identity/login',
    register: '/identity/register',
  } as const;
```

In `lamad-layout.component.html:3` add the property bindings (PROPERTY binding — `contextApps` is `attribute:false`):

```html
  <elohim-navigator
    [attr.context]="'lamad'"
    [attr.show-search]="true"
    [contextApps]="navigatorApps"
    [identityRoutes]="navigatorIdentityRoutes"
    (navigate)="onNavigatorNavigate($event)"
  >
```

`onNavigatorNavigate` keeps handling the emitted strings (in-lamad strip vs `location.assign`) — unchanged; the existing spec canaries at lamad-layout.component.spec.ts:77-107 stay green.

- [ ] **Step 11.5: Update the graphos default story** — `elohim-navigator.default.stories.ts`: add a `contextApps` fixture (same five entries) bound via `.contextApps=${...}` in the render functions, and update the docs prose (lines ~47-48) to say the switcher entries are host-supplied (the story demonstrates a host binding). If the designed story renders the navigator, mirror the fixture there.

- [ ] **Step 11.6: Run, verify pass + regen manifest**

```bash
cd /projects/elohim/app/elohim-elements/elohim-core
pnpm test && pnpm lint && pnpm typecheck && pnpm build   # build regenerates dist/custom-elements.json
cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts src/app/components/lamad-layout
```

- [ ] **Step 11.7: Verify-only — the Angular navigator.** `grep -n "router.navigate\|EprNavService" app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts` — confirm its `/lamad` entry navigates via EprNavService or plain href (the omnibar-consolidation sweep should have covered it). If it does `router.navigate(['/lamad'…])`, fix it the Task 8 way and note it in the commit.

- [ ] **Step 11.8: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-navigator.ts app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts \
        app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts app/elohim-elements/elohim-core/dist/custom-elements.json \
        app/lamad/src/app/components/lamad-layout/ \
        app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-navigator.default.stories.ts
git commit -m "fix(elohim-core)!: navigator goes blank-slate — context apps + identity routes are host-supplied (§12.3)"
```

---

### Task 12: SEO twins — base-aware canonical + universal content canonical

**Files:**
- Modify: `app/elohim-app/src/app/services/seo.service.ts` (generateCanonicalUrl ~314-320; updateForContent ~393-405)
- Modify: `app/lamad/src/app/shared/services/seo.service.ts` (IDENTICAL edit, +2 line offset — keep `diff <(tail -n +3 lamad) elohim-app` EMPTY)
- Create: `app/lamad/src/app/shared/services/seo.service.spec.ts`
- Modify: `app/elohim-app/src/app/services/seo.service.spec.ts` (add generateCanonicalUrl coverage)

- [ ] **Step 12.1: Write the failing lamad regression spec** (new file — minimal, targets exactly the bug):

```typescript
import { TestBed } from '@angular/core/testing';
import { Subject } from 'rxjs';
import { Router } from '@angular/router';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { SeoService } from './seo.service';

describe('SeoService (lamad bundle — base href /lamad/)', () => {
  let service: SeoService;
  let baseEl: HTMLBaseElement;

  beforeEach(() => {
    // jsdom derives document.baseURI from a <base> element — simulate the bundle mount.
    baseEl = document.createElement('base');
    baseEl.href = '/lamad/';
    document.head.prepend(baseEl);

    TestBed.configureTestingModule({
      providers: [
        SeoService,
        {
          provide: Router,
          useValue: { events: new Subject().asObservable(), url: '/path/foundations/step/0' },
        },
      ],
    });
    service = TestBed.inject(SeoService);
  });

  afterEach(() => {
    baseEl.remove();
    document.querySelector('link[rel="canonical"]')?.remove();
  });

  it('re-prefixes the bundle mount onto generated canonical URLs (§12 keeper)', () => {
    service.updateSeo({ title: 'Step', description: 'A step' });
    const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    expect(canonical?.href).toBe('https://elohim.host/lamad/path/foundations/step/0');
  });

  it('mints the universal address for content canonicals', () => {
    service.updateForContent({ id: 'fct-module-01', title: 'T', contentType: 'concept' });
    const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    expect(canonical?.href).toBe('https://elohim.host/epr/fct-module-01');
  });
});
```

(Adapt provider list to whatever the elohim-app spec's TestBed needs — copy its Router/ActivatedRoute mocks; the service also injects ActivatedRoute/Title/Meta/DOCUMENT which TestBed provides by default in the browser-like env.)

- [ ] **Step 12.2: Run, verify failure**

```bash
cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts src/app/shared/services/seo.service.spec.ts
```

Expected: FAIL — canonical is `https://elohim.host/path/foundations/step/0` (mount dropped) and `…/resource/fct-module-01`.

- [ ] **Step 12.3: Implement — identical edit in BOTH copies.**

`generateCanonicalUrl` becomes (replace in both files):

```typescript
  /**
   * Generate canonical URL from current route. Bundles are served under a
   * base href (lamad: /lamad/) and router.url is base-stripped — re-prefix so
   * the canonical names the PUBLIC URL (§12.1; SEO absolute URLs are keepers).
   */
  private generateCanonicalUrl(): string {
    const path = this.router.url.split('?')[0]; // Remove query params
    const base = new URL(this.document.baseURI).pathname.replace(/\/$/, '');
    const publicPath = `${base}${path}`.replace(/\/{2,}/g, '/');
    return `${DEFAULTS.siteUrl}${publicPath}`;
  }
```

`updateForContent` canonical line becomes (both files):

```typescript
    // Universal EPR address (§12.1) — the durable, bundle-agnostic canonical.
    const canonicalUrl = `${DEFAULTS.siteUrl}/epr/${content.id}`;
```

(`updateForPath`'s `/lamad/path/{id}` stays — it IS the public pretty-mount URL, a documented SEO keeper. `updateForProfile`'s `/lamad/human` oddity is out of scope — leave.)

- [ ] **Step 12.4: Add the shell-side coverage** — in `app/elohim-app/src/app/services/seo.service.spec.ts`, add to the existing canonical describe:

```typescript
    it('generates canonical from router.url at the root base href', () => {
      service.updateSeo({ title: 'T', description: 'D' });
      const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
      expect(canonical?.href).toBe('https://elohim.host/test-path');
    });
```

(The spec's Router mock url is `/test-path`; elohim-app test DOM has no `<base>` override → base path `/` → unchanged behavior proven.)

- [ ] **Step 12.5: Run both, verify pass + twin invariant**

```bash
cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts src/app/shared/services/seo.service.spec.ts
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/services/seo.service.spec.ts
diff <(tail -n +3 /projects/elohim/app/lamad/src/app/shared/services/seo.service.ts) /projects/elohim/app/elohim-app/src/app/services/seo.service.ts && echo TWIN-OK
```

Expected: both suites green; `TWIN-OK`.

- [ ] **Step 12.6: Commit**

```bash
git add app/elohim-app/src/app/services/seo.service.ts app/elohim-app/src/app/services/seo.service.spec.ts \
        app/lamad/src/app/shared/services/seo.service.ts app/lamad/src/app/shared/services/seo.service.spec.ts
git commit -m "fix(seo): base-aware canonical generation (lamad mount no longer dropped) + universal /epr content canonicals"
```

---

### Task 13: Doorway journal stub — de-literalize derivative paths

**Files:**
- Modify: `doorway/doorway-service/src/routes/journal.rs` (templates ~154-176 + tests)
- Modify: `app/elohim-app/src/app/shefa/services/journal-routing.service.spec.ts` (mock at line ~46)
- Modify: `app/elohim-app/src/app/shefa/components/.../journal-routing-cards.component.spec.ts` (MOCK_SUGGESTIONS line ~26)

- [ ] **Step 13.1: Update the Rust tests first** — in `test_suggestions_include_derivative_for_each_type` (and any sibling asserting derivative fields), add:

```rust
        // §12.3: the server never mints pillar mount URLs. destination_type is
        // the routing vocabulary; clients mint routes via their claims context.
        assert_eq!(suggestions[1].suggested_path, "");
        assert_eq!(suggestions[2].suggested_path, "");
```

(Filing-card assertions — `suggestions[0].suggested_path == "learning"` etc. — stay: a journal folder slug is metadata, not a URL.)

- [ ] **Step 13.2: Run, verify failure**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins journal
```

- [ ] **Step 13.3: Implement** — in `generate_suggestions_stub`, change the three template tuples' path elements to `""` and add the rationale comment:

```rust
    // Derivative cards. suggested_path is deliberately EMPTY (§12.3): the
    // server never mints pillar mount URLs — destination_type is the routing
    // vocabulary; the client mints routes via its BundleRouteContext claims.
    let templates: &[(&str, &str, &str, &str, &str)] = &[
        (
            "exchange-request",
            "Post to exchange",
            "Post a request or offer on the community exchange.",
            "",
            "community",
        ),
        (
            "governance-proposal",
            "Draft governance proposal",
            "Draft a governance proposal for community review.",
            "",
            "community",
        ),
        (
            "content",
            "Share as learning content",
            "Share this as learning content for others.",
            "",
            "network",
        ),
    ];
```

- [ ] **Step 13.4: Update the frontend spec mocks** — `journal-routing.service.spec.ts` (~line 46): `suggestedPath: '/shefa/exchange/'` → `suggestedPath: ''`; `journal-routing-cards.component.spec.ts` MOCK_SUGGESTIONS (~line 26): same. (The filing-card mocks with bare folder slugs stay.)

- [ ] **Step 13.5: Run, verify pass**

```bash
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins journal
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/shefa
```

- [ ] **Step 13.6: Commit**

```bash
git add doorway/doorway-service/src/routes/journal.rs app/elohim-app/src/app/shefa
git commit -m "fix(doorway): journal derivative cards stop minting pillar mount URLs (§12.3)"
```

---

### Task 14: imagodei-portal — interceptor safety net

**Files:**
- Modify: `app/imagodei-portal/src/main.ts`

The portal's base href `/auth/portal/` makes the DEFAULT `baseHrefOwnsPath` heuristic correct — do NOT pass `explicit: true` (no router-aware ownsPath exists; default semantics are right). `main.ts` is coverage-excluded; this mirrors page-chrome's `connectedCallback` model (no teardown needed for a top-level bundle).

- [ ] **Step 14.1: Implement**

```typescript
import { bootstrapApplication } from '@angular/platform-browser';
import 'elohim-core/register';
import 'elohim-imagodei/register';
import { installEprLinkInterceptor } from 'elohim-core';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

// Cross-bundle safety net (§12.3): content-authored/legacy anchors get the
// EPR-native handoff. Base href /auth/portal/ makes the default ownsPath
// heuristic correct — default (non-explicit) install is the right semantics.
installEprLinkInterceptor();

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);
```

- [ ] **Step 14.2: Verify** — `cd /projects/elohim/app/imagodei-portal && pnpm typecheck && pnpm test` (suite green; no unit test possible for main.ts — node env, coverage-excluded).

- [ ] **Step 14.3: Commit**

```bash
git add app/imagodei-portal/src/main.ts
git commit -m "feat(imagodei-portal): install the epr-link interceptor safety net (§12.3)"
```

---

### Task 15: a2o — render-verified scenarios for the universal address + cross-bundle handoff

**Files:**
- Modify: `app/lamad/src/app/components/content-viewer/content-viewer.component.html` (root testid, IF missing)
- Modify: `genesis/a2o/src/framework/pages/selectors.ts` (+CONTENT_VIEWER)
- Modify: `genesis/a2o/features/lamad/deep-link-delivery.feature` (+2 scenarios)
- Modify: `genesis/a2o/steps/lamad/deep-link-delivery.steps.ts` (+2 steps)

- [ ] **Step 15.1: Ensure the content-viewer root testid.** `grep -n 'data-testid' app/lamad/src/app/components/content-viewer/content-viewer.component.html | head -3` — if no root-container `data-testid="content-viewer"` exists, add it to the outermost container element. Register in `selectors.ts` (follow the PATH_NAV/PATH_OVERVIEW pattern):

```typescript
/** Cross-pillar resource viewer (shell-rendered at /resource/:id and /epr/:id). */
export const CONTENT_VIEWER = {
  ROOT: 'content-viewer',
} as const;
```

- [ ] **Step 15.2: Extend the feature** — append to `deep-link-delivery.feature`:

```gherkin
  @browser-only
  Scenario: Universal EPR address resolves to a rendered surface
    # §12.1 Slice 2 — /epr/{id}: the doorway serves the root bundle; the
    # shell's epr/:resourceId route resolves the EPR and renders the
    # cross-pillar resource viewer. The durable, bundle-agnostic address.
    Given a learner opens the deep link "/epr/foundations-christian-technology"
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: View Resource Details crosses the bundle boundary
    # §12.3 sweep — the lamad step navigator's resource link is a plain href
    # to the universal address; the epr-link interceptor records the handoff
    # and the full doorway load renders the shell viewer. Regression anchor
    # for the resource self-loop redirect killed in Slice 2.
    Given a learner opens the deep link "/lamad/path/foundations-christian-technology/step/0"
    Then the lamad step navigator renders
    When the learner follows the View Resource Details link
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response
```

- [ ] **Step 15.3: Implement the two new steps** (append to `deep-link-delivery.steps.ts`; import `CONTENT_VIEWER` and `PATH_NAV` from the selectors module — PATH_NAV.VIEW_RESOURCE is already registered):

```typescript
Then('the cross-pillar resource viewer renders', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const rendered = await rootRendered(device, CONTENT_VIEWER.ROOT);
  assert.ok(
    rendered,
    `Expected the cross-pillar resource viewer to render (data-testid="${CONTENT_VIEWER.ROOT}"); ` +
      `URL is "${device.page.url()}"`
  );
});

When('the learner follows the View Resource Details link', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  await device.page.locator(`[data-testid="${PATH_NAV.VIEW_RESOURCE}"]`).first().click();
  await device.page.waitForLoadState('domcontentloaded');
});
```

- [ ] **Step 15.4: Audit legacy-URL steps.** `grep -rn "/lamad/resource/" genesis/a2o/steps genesis/a2o/features` — files known: `steps/ui/epr-content.steps.ts`, `steps/ui/feedback-gate.steps.ts`. These now traverse the legacy bridge (lamad boot → handoff → shell viewer) — slower but still landing on rendered content. Update their navigation URLs to `/epr/{id}` where the step merely needs the content rendered (keep ONE legacy-form usage if a scenario explicitly guards the bridge). Record any scenario whose semantics change in the commit body.

- [ ] **Step 15.5: Dry-run gate**

```bash
cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run
```

Expected: 0 undefined steps. (Full e2e runs on alpha post-merge via CI; do not attempt locally.)

- [ ] **Step 15.6: Commit**

```bash
git add genesis/a2o/features/lamad/deep-link-delivery.feature genesis/a2o/steps/lamad/deep-link-delivery.steps.ts \
        genesis/a2o/src/framework/pages/selectors.ts genesis/a2o/steps/ui \
        app/lamad/src/app/components/content-viewer/content-viewer.component.html
git commit -m "test(a2o): universal /epr address + cross-bundle resource handoff — render-verified (§12.5)"
```

---

### Task 16: Full gates, doc currency, finish

- [ ] **Step 16.1: Full verification ladder** (every project touched):

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm test
cd /projects/elohim/app/elohim-app && pnpm test && pnpm run lint
cd /projects/elohim/app/lamad && pnpm test && pnpm run lint
cd /projects/elohim/app/elohim-elements/elohim-core && pnpm test && pnpm lint && pnpm typecheck && pnpm build
cd /projects/elohim/app/imagodei-portal && pnpm test && pnpm typecheck
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo nextest run --lib --bins && RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo nextest run is_spa_route_subpath
cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run
```

All green, or each failure traced to a pre-existing issue (record verbatim in the sprint notes).

- [ ] **Step 16.2: Grep-audit — no stray literals survive the sweep**

```bash
cd /projects/elohim
grep -rn "'/resource'" app/lamad/src --include="*.ts" --include="*.html" | grep -v "resource/:resourceId/edit" | grep -v spec
grep -rn "'/content'" app/lamad/src --include="*.ts" --include="*.html" | grep -v spec
grep -rn "TODO(#12-6" app/ | grep -v node_modules
```

Expected: zero production hits (the `TODO(#12-6 Slice 2)` markers died with the rewrite; doc-comment keepers OK). Investigate anything that prints.

- [ ] **Step 16.3: Spec + gospel currency (managed surfaces — cite tooling discipline).** These files are managed surfaces: edit through the cite tooling flow (seal/describe/propagate/refresh per `_lib/managed_surfaces.py` registry; the PreToolUse injection will guide):
  - `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — §12.6 table: mark Slice 2 contents landed (date), note Slice 3 unchanged.
  - `app/elohim-app/CLAUDE.md` "Cross-bundle navigation" rail — append: *unclaimed EPR targets mint the universal `/epr/{id}` address (`BundleRouteContext`, §12.3); the shell owns `epr/:resourceId`.*
  - `app/lamad/CLAUDE.md` EPR-app bundle rails — append to cross-bundle links: *cross-bundle content links mint `/epr/{id}` (plain href or `LAMAD_EPR_NAV`); lamad claims only `contentType: 'path'`.*

- [ ] **Step 16.4: Commit docs**

```bash
git add genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md app/elohim-app/CLAUDE.md app/lamad/CLAUDE.md
git commit -m "docs(spec+gospel): §12.6 Slice 2 landed — universal /epr address + claims rewrite rails"
```

- [ ] **Step 16.5: Story harvest + finish.** Invoke the `story-harvest` skill (failure-then-fix cycle: the ownsPath layout-root discovery and the self-loop redirect are parameter-bearing constraints worth scenario coverage — the a2o scenarios from Task 15 may already satisfy it; let the skill judge). Then invoke `superpowers:finishing-a-development-branch` — present merge options to the operator. **Do NOT push** (integrator owns push/merge; CI validation happens on the operator's dev merge).

---

## Self-review notes (spec coverage)

| Spec §12 requirement | Task |
|---|---|
| `/epr/{id}` universal address, doorway-served | 1 |
| `/epr` reserved-prefix enforcement | 1 |
| Shell `epr/:id` route (reachable-but-unclaimed → shell viewer) | 2 |
| `eprToRoute` rewritten around BundleRouteContext; heuristic dies | 3 |
| In-path/cross-path `resolveInContext` arms consult injected context | 4 |
| No literal pillar prefix in shared code (`@elohim/service`, elohim-core) | 3, 11 |
| Lamad mints `['/path', …]` only via its claim; everything else `/epr/{id}` | 6-10 |
| Doubled URLs stop being minted; self-loop killed; legacy bridge | 9 |
| elohim-elements stay blank-slate | 11 |
| §12.5 a2o render-verified scenarios + shared-fixture vector | 1, 15 |
| Slice 3 (routeClaims manifests, 302, gate experience, card-flip pushState) | OUT OF SCOPE — unchanged |

Known consciously-deferred items: card-flip + pushState (§12.3 last bullet — Slice 3 with the claims machinery); `updateForProfile` canonical oddity; ThemeService/SeoService elohim-core extraction (B18c follow-up); the Angular `<app-elohim-navigator>` literals (verify-only, Step 11.7).
