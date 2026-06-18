# Self-Healing Debug View — Plan C Frontend Handoff

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the already-built doorway `GET /admin/self-healing` read model as a debug-mode-gated view in the main Angular app (`elohim-app`), so a human, a future controls UI, or an on-device AI agent can see which self-healing mechanism is exhausted — without scraping logs.

**Architecture:** The backend is **done and committed** (Plan C, branch `shift/self-healing-control-plane`). This handoff is the frontend/TS sibling that was deliberately deferred. It builds on infrastructure `elohim-app` **already has**: a `doorway` pillar with a `DoorwayAdminService` and a `doorway-dashboard` component that already read `/admin/*`. The work collapses to: register the wire contract for codegen → make the endpoint reachable in dev → add one service method → gate it behind a debug flag → render it with a component that mirrors the existing dashboard.

**Tech Stack:** Angular 19 (standalone components, signals, functional guards), TypeScript strict, JSON-Schema→TS codegen (`json-schema-to-typescript`), Rust (doorway-service, for the contract test only).

---

## ⚠ Read this before you touch anything (five facts that break execution if missed)

1. **The generated TypeScript type is named `StabilityStatusView`, NOT `SelfHealingView`.** The Rust struct is `SelfHealingView` (`doorway/doorway-service/src/routes/self_healing.rs:32`); the JSON-Schema `title` is `"StabilityStatusView"` (`elohim/sdk/schemas/v1/views/stability-status-view.schema.json:4`), and codegen names the interface from the title. **Use the schema-codegen path only. Do NOT add `#[derive(TS)]` to the Rust struct and do NOT hand-write an interface** — both are wasted work and create a second drifting home. The schema is already authored; you just register it (Task 1).

2. **The debug-mode gate controls UI VISIBILITY ONLY — it is not access control.** `GET /admin/self-healing` is unauthenticated by design ("operator-only is an ingress property, not enforced here", `doorway/doorway-service/src/server/http.rs:2436`) and — per grounding — **no ingress rule currently enforces that property**, so the endpoint is publicly reachable on the prod/alpha doorway. Your debug flag decides whether a menu item appears; it protects nothing. See the **Security consideration** section — flag it for the operator, do not turn this into a hardening project.

3. **The endpoint can return 503 and 500 — not just 200/JSON.** It is **not** admission-exempt, so under inbound saturation it returns `503` with body `{"status":"catching-up","retryAfter":N}` (`server/http.rs:2117-2137`, `:3746-3757`). The only 500 path returns a **plain-text** body `"Failed to serialize self-healing view"`, NOT JSON (`self_healing.rs:273-278`). The service must handle 503 (transient, show "catching up", retry later) and a non-JSON 5xx body (don't `JSON.parse` it) distinctly from 200.

4. **In dev, an unproxied request silently returns `index.html` (HTTP 200), not an error.** `/admin` is not in the Angular dev-server proxy context, so without the Task 2 edit a `fetch('/admin/self-healing')` is served the SPA shell — `JSON.parse` then throws on HTML, or you silently get an HTML body. This failure is mystifying if you don't expect it. Task 2 fixes it.

5. **Three of the eight blocks are PENDING wire-up (always `null`/`[]` today), and that is distinct from "no data yet."** `autoPreset` (null), `admission` (null), `upstreams` (`[]`) are stubbed pending sibling follow-ons. `projector.lagSeconds`, `projector.caughtUp`, `peers[].lastSeen`, `warmup.lastError` are wired to real sources but go `null` when cold. **Render these two null-meanings differently** ("pending wire-up" vs "no data yet") — for a debug view, honest signal is the feature. Mirror the `protocol-omni` neutral-glyph "never cries wolf" posture (`app/elohim-app/src/app/elohim/components/protocol-omni/protocol-omni.component.ts:38-45`).

---

## What already exists (do NOT rebuild)

- **The endpoint + composed view (committed, Plan C):** `GET /admin/self-healing` → `routes::handle_self_healing` (`doorway/doorway-service/src/server/http.rs:2439-2441`), composed purely in Rust by `compose_self_healing()` (`doorway/doorway-service/src/routes/self_healing.rs:139`). The Angular service must **not** re-aggregate — composition is done.
- **The wire contract schema (committed):** `elohim/sdk/schemas/v1/views/stability-status-view.schema.json`. Already conforms to the 10 view conventions (camelCase, `additionalProperties:false`, `required`, EPR `$id`, "Source of truth:" description). **It is NOT yet registered for codegen** — Task 1.
- **The `doorway` pillar in elohim-app:** `app/elohim-app/src/app/doorway/` — a `DoorwayAdminService` (`services/doorway-admin.service.ts`) that already calls `/admin/*`, a `doorway-dashboard` component, and `doorway.routes.ts` with a lazy shell route registered at `app/elohim-app/src/app/app.routes.ts:28-31`. This is your service + component + route pattern.
- **The serde camelCase contract:** `SelfHealingView` uses `#[serde(rename_all = "camelCase")]` with NO `skip_serializing_if` (`self_healing.rs:31`), so every key is always present and already camelCase — no `JSON.parse`, no case conversion, no `toWire/fromWire` (adapter rule, `app/elohim-app/src/app/elohim/CLAUDE.md`).

## The wire shape you are consuming (today's truth)

Top-level object, 8 keys, **all always present** (no `skip_serializing_if`):

| Key | Type | Status today | Source |
|---|---|---|---|
| `autoPreset` | `object \| null` | **PENDING → always `null`** | auto-config (arc-policy) sibling |
| `admission` | `object \| null` | **PENDING → always `null`** | inbound-semaphore accessor follow-on |
| `upstreams` | `array` | **PENDING → always `[]`** | upstream-breaker snapshot follow-on |
| `projector` | `{lagSeconds:int\|null, caughtUp:bool\|null, divergentAnchor:int\|null}` | REAL (cold-nullable) | storage `/api/v1/status/projector` + `p2p_health` cache |
| `peers` | `[{peer:string, status:string, lastSeen:string\|null}]` | REAL | `peer_health.snapshot()` |
| `render` | `{total:int, degenerateRate:number}` | REAL (2-field subset of render-stats) | `render_trace_stats.snapshot()` |
| `warmup` | `{inProgress:bool, attempts:int, completed:bool, lastError:string\|null}` | REAL | `warmup_state` atoms |
| `conductor` | `{connected:bool, connectedWorkers:int, totalWorkers:int}` | REAL | conductor `pool` |

- `peers[].status` is the Rust `Debug` form: `"Healthy" | "Degraded" | "Offline"`.
- `upstreams[].circuit` (when it lands) is the enum `"closed" | "half-open" | "open"`.
- `render` here is a deliberate 2-field subset of the 9-field `/admin/render-stats` (`RenderTraceSnapshot`: `total, rendered, renderedEmpty, stalled, timedOut, errored, avgWallMs, maxWallMs, degenerateRate`). If the debug view ever needs the full terminal-class breakdown, read `/admin/render-stats` too — but Plan C's `render` block is intentionally minimal.

---

## File Structure

**Create:**
- `app/elohim-app/src/app/doorway/components/self-healing/self-healing.component.ts` (+ `.html`, `.scss`) — the debug view, standalone, signal-based, mirrors `doorway-dashboard`.
- `app/elohim-app/src/app/elohim/guards/debug-mode.guard.ts` — a `CanActivateFn` gating the route on the debug flag.
- `doorway/doorway-service/tests/schema_contract.rs` **entry** (file exists; add a test fn) — locks Rust↔schema drift.

**Modify:**
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — register the view in `INTERFACE_FILES`.
- `app/elohim-app/proxy.conf.mjs` **and** `app/elohim-app/proxy.conf.alpha.mjs` — add `/admin` to the proxy context.
- `app/elohim-app/src/environments/environment.types.ts` — add `showSelfHealing?: boolean` to `FeatureFlags`.
- `app/elohim-app/src/environments/environment.ts` (+ `.prod.ts`, `.alpha.ts`) — set the flag per environment (default false).
- `app/elohim-app/src/app/doorway/services/doorway-admin.service.ts` — add `getSelfHealing()`.
- `app/elohim-app/src/app/doorway/doorway.routes.ts` — add the gated child route.
- (optional nav) wherever a debug menu entry belongs.

**Generated (do not hand-edit; produced by Task 1):**
- `app/elohim-app/src/app/generated/stability-status-view.ts` (+ the same file in 5 other consumer dirs).

---

## Task 1: Register the wire contract for TS codegen

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (the `INTERFACE_FILES` array, near line 61-66)
- Prereq: `pnpm install` at repo root (the worktree has **no** `node_modules`; codegen imports `json-schema-to-typescript` and shells out to `prettier`)

- [ ] **Step 1: Install workspace deps** (one-time; codegen will fail with a module-not-found otherwise)

```bash
pnpm install   # from repo root
```

- [ ] **Step 2: Add the view to `INTERFACE_FILES`**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, find the `INTERFACE_FILES` array (the `views/` block around line 61-66) and add one entry, matching the surrounding `{ src, dest }` format:

```js
  { src: 'views/stability-status-view.ts', dest: 'stability-status-view.ts' },
```

Place it among the other `views/` entries (e.g. after `peer-info-view`). The `src` is the **intermediate** generated path — `generateFromDir('views', ...)` (`codegen-ts.mjs:562`) already auto-walks every `views/*.schema.json` and emits `generated-ts/views/stability-status-view.ts`; the `INTERFACE_FILES` entry is what gets **distributed** to the 6 consumer dirs and what `--verify` checks. No other code change is needed.

- [ ] **Step 3: Generate + distribute**

Run: `pnpm run schema:codegen:ts`
Expected: a new `stability-status-view.ts` exporting `interface StabilityStatusView` appears in all 6 `GENERATED_OUTPUT_DIRS`, including `app/elohim-app/src/app/generated/stability-status-view.ts`. The barrel `elohim/sdk/schemas/generated-ts/index.ts` re-exports it.

- [ ] **Step 4: Verify codegen is idempotent + fresh**

Run: `pnpm run schema:codegen:ts -- --verify`
Expected: exit 0 (no diff). Note the `--` passthrough is required (this is the literal form `.husky/pre-push` uses). Idempotency risk is LOW here — the `circuit` enum is inline on a field, not a top-level union alias, so the known Reach/ContentFormat Prettier oscillation does not apply. If `--verify` flags a cosmetic re-wrap, re-run codegen once and commit the settled output.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/schemas/generated-ts/ \
        app/elohim-app/src/app/generated/stability-status-view.ts \
        app/elohim-library/projects/elohim-service/src/generated/stability-status-view.ts \
        doorway/doorway-app/src/app/generated/stability-status-view.ts \
        app/lamad/src/generated/stability-status-view.ts \
        app/elohim-library/projects/elohim-identity/src/generated/stability-status-view.ts \
        genesis/seeder/src/generated/stability-status-view.ts
git commit -m "feat(schema): register stability-status-view for TS codegen (Plan C)"
```

---

## Task 2: Make the endpoint reachable in dev (proxy both files)

**Files:**
- Modify: `app/elohim-app/proxy.conf.mjs` (context array, ~line 11)
- Modify: `app/elohim-app/proxy.conf.alpha.mjs` (context array, ~line 15)

The dev server (`pnpm start` / `pnpm start:alpha`, wired via `angular.json:146`) only proxies a fixed set of path prefixes to doorway `:8888`. `/admin` is absent, so the browser silently gets the SPA shell for `/admin/self-healing` (see Critical fact #4). Add `/admin` to **both** context arrays.

- [ ] **Step 1: Add `/admin` to `proxy.conf.mjs`**

Find the context array (currently `['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p']`) and add `'/admin'`:

```js
['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p', '/admin']
```

- [ ] **Step 2: Add `/admin` to `proxy.conf.alpha.mjs`** (the `start:alpha` twin — same edit to its context array).

- [ ] **Step 3: Manually verify reachability** (with a local stack or `start:alpha`)

Run: `pnpm start` (or `pnpm start:alpha`), then in the browser console: `await (await fetch('/admin/self-healing')).json()`
Expected: a JSON object with keys `autoPreset, admission, upstreams, projector, peers, render, warmup, conductor` — **not** an HTML string. If you get HTML, the proxy entry didn't take (restart the dev server — proxy config is read at startup).

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/proxy.conf.mjs app/elohim-app/proxy.conf.alpha.mjs
git commit -m "feat(elohim-app): proxy /admin to doorway for self-healing debug view (Plan C)"
```

---

## Task 3: Add `getSelfHealing()` to `DoorwayAdminService`

**Files:**
- Modify: `app/elohim-app/src/app/doorway/services/doorway-admin.service.ts`
- Test: co-located `.spec.ts` if the service has one; otherwise add a minimal Vitest spec.

Mirror the existing `getNodes()` pattern (`doorway-admin.service.ts:67-86`): `baseUrl = environment.doorwayUrl ?? ''`, `http.get<T>(...).pipe(timeout, retry(2), catchError(...))`. Consume the generated `StabilityStatusView` type from `@app/generated` (or the app's generated barrel).

- [ ] **Step 1: Write the failing test**

```typescript
// doorway-admin.service.spec.ts
it('GETs /admin/self-healing and returns the typed view', async () => {
  const view: StabilityStatusView = await firstValueFrom(service.getSelfHealing());
  expect(view).toHaveProperty('projector');
  expect(view).toHaveProperty('peers');
});
```

Run: `pnpm exec vitest run --config vite.config.ts doorway-admin.service` — Expected: FAIL ("getSelfHealing is not a function").

- [ ] **Step 2: Implement, mirroring `getNodes()`**

```typescript
import type { StabilityStatusView } from '../../generated/stability-status-view';

/** Doorway self-healing read model (Plan C). Node-local, unauthenticated by
 *  design — see the debug-mode gate (UI visibility only, not access control). */
getSelfHealing(): Observable<StabilityStatusView> {
  return this.http
    .get<StabilityStatusView>(`${this.baseUrl}/admin/self-healing`)
    .pipe(
      timeout(this.timeout),
      retry(1),
      catchError(this.handleError<StabilityStatusView>('getSelfHealing')),
    );
}
```

> **503/500 handling (Critical fact #3):** the endpoint can return `503 {status:"catching-up"}` under load and a **plain-text** 500. Ensure `handleError` (or the component) treats a 503 as transient (surface "node catching up", offer retry) and never `JSON.parse`s a non-JSON 5xx body. If the shared `handleError` swallows status codes, branch on `err.status === 503` in the component instead.

- [ ] **Step 3: Run the test** — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/doorway/services/doorway-admin.service.ts \
        app/elohim-app/src/app/doorway/services/doorway-admin.service.spec.ts
git commit -m "feat(elohim-app): DoorwayAdminService.getSelfHealing (Plan C)"
```

---

## Task 4: Add the debug-mode gate (FeatureFlag)

**Files:**
- Modify: `app/elohim-app/src/environments/environment.types.ts` (the `FeatureFlags` interface, ~line 90-99)
- Modify: `app/elohim-app/src/environments/environment.ts`, `environment.prod.ts`, `environment.alpha.ts`
- Create: `app/elohim-app/src/app/elohim/guards/debug-mode.guard.ts`

There is **no** existing DebugService/FeatureFlagService/runtime toggle. The established idiom is static `FeatureFlags` in the environment files (the lone existing flag is `useGraphqlTopology`). Use it — prod-safe by default (false).

- [ ] **Step 1: Add the flag to the interface**

In `environment.types.ts`, add to `FeatureFlags`:

```typescript
  /** Show the self-healing debug view (Plan C). UI visibility only — the
   *  endpoint is unauthenticated, so this is not access control. Default false. */
  showSelfHealing?: boolean;
```

- [ ] **Step 2: Set it per environment** — `environment.ts` (dev): `features: { ..., showSelfHealing: true }`. `environment.prod.ts`: omit or `false`. `environment.alpha.ts`: your call — `true` makes it visible on alpha (alpha is `production:false`); default to `false` unless the operator wants it on alpha.

- [ ] **Step 3: Write the guard**

```typescript
// app/elohim-app/src/app/elohim/guards/debug-mode.guard.ts
import { inject, isDevMode } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { environment } from '../../../environments/environment';

/** Gates debug-only routes on the showSelfHealing flag, falling back to
 *  isDevMode() so `ng serve` always exposes it. UI-visibility gate ONLY. */
export const debugModeGuard: CanActivateFn = () => {
  const enabled = environment.features?.showSelfHealing ?? isDevMode();
  if (enabled) return true;
  return inject(Router).createUrlTree(['/']);
};
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/environments/ app/elohim-app/src/app/elohim/guards/debug-mode.guard.ts
git commit -m "feat(elohim-app): showSelfHealing debug-mode gate (Plan C)"
```

> **Confirm-point for the user (do not build without confirming):** the literal phrasing "generally available if the runtime is in a debug mode" could mean a *runtime* toggle (no rebuild) rather than a build-time flag. That needs a small `ConfigService` addition reading a `config.json` field (the service exists: `app/elohim-app/src/app/services/config.service.ts` exposes an `environment` string). If the user wants runtime-toggle, swap the guard's `environment.features?.showSelfHealing` for a `ConfigService` read. Default to the build-time flag unless told otherwise.

---

## Task 5: Build the SelfHealing debug component

**Files:**
- Create: `app/elohim-app/src/app/doorway/components/self-healing/self-healing.component.ts` (+ `.html`, `.scss`)
- Template to mirror: `app/elohim-app/src/app/doorway/components/doorway-dashboard/doorway-dashboard.component.ts`

Standalone component, `inject(DoorwayAdminService)`, signal state (`loading`/`error`/`view`), `firstValueFrom` fetch in `ngOnInit`, sectioned render of the 8 blocks.

- [ ] **Step 1: Component skeleton** (mirror `doorway-dashboard`)

```typescript
@Component({
  selector: 'app-self-healing',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './self-healing.component.html',
  styleUrl: './self-healing.component.scss',
})
export class SelfHealingComponent implements OnInit {
  private readonly admin = inject(DoorwayAdminService);
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly view = signal<StabilityStatusView | null>(null);

  async ngOnInit(): Promise<void> {
    try {
      this.view.set(await firstValueFrom(this.admin.getSelfHealing()));
    } catch (e: unknown) {
      // 503 = node catching up (transient); other = real error. Never parse a
      // non-JSON 5xx body. See handoff Critical fact #3.
      this.error.set(this.describe(e));
    } finally {
      this.loading.set(false);
    }
  }
  private describe(e: unknown): string { /* branch on status 503 vs other */ return String(e); }
}
```

- [ ] **Step 2: Template — render the 8 blocks with honest null-semantics** (Critical fact #5)

For `autoPreset`/`admission`/`upstreams`: when `null`/empty, render a muted **"pending wire-up"** label (these will populate when their sibling follow-ons land) — visually distinct from a **"no data yet"** state on `projector`/`warmup` cold-null fields. Mirror the `protocol-omni` neutral-glyph posture (`protocol-omni.component.ts:38-45`) — a debug view that cries wolf on a stubbed field is worse than useless.

- [ ] **Step 3: Tauri / no-doorway degradation** (Critical: doorway-only endpoint)

`/admin/self-healing` exists on **doorway only**; elohim-storage has no such route. In Tauri desktop (direct to storage `:8090`, no doorway) the call will 404. Detect the direct-storage deployment (`storage-client.service.ts` `getStorageBaseUrl()` returns `http://localhost:8090`) and render a **"not available in desktop mode — this is a doorway-edge endpoint"** placeholder. Do **not** retry against `:8090`, do **not** surface the 404 as an error toast.

- [ ] **Step 4: Add a note that the read model is per-replica** (node-local): the view reflects whichever doorway replica answered, not a cluster-wide aggregate — relevant if alpha runs >1 doorway replica behind one ingress. A one-line caption suffices.

- [ ] **Step 5: Test + commit** — a render test asserting the pending-vs-cold distinction shows correctly for a mock view; then:

```bash
git add app/elohim-app/src/app/doorway/components/self-healing/
git commit -m "feat(elohim-app): self-healing debug view component (Plan C)"
```

---

## Task 6: Route it (gated)

**Files:**
- Modify: `app/elohim-app/src/app/doorway/doorway.routes.ts`

- [ ] **Step 1: Add the gated child route** (mirror the lazy `loadComponent` entries already in `doorway.routes.ts:15-64`, under the existing `doorway-layout` shell):

```typescript
{
  path: 'self-healing',
  canActivate: [debugModeGuard],
  loadComponent: () =>
    import('./components/self-healing/self-healing.component')
      .then((m) => m.SelfHealingComponent),
  title: 'Self-Healing (debug)',
},
```

Resulting path: `/doorway/self-healing`. (If the user prefers a top-level `/debug/self-healing`, add the lazy entry to `app.routes.ts` instead, mirroring its single-component entries at `:94-109` — same guard.)

- [ ] **Step 2: (optional) Add a nav entry** behind the same flag, so the view is discoverable when debug mode is on. The one existing "show extra in dev" precedent is `elohim-navigator.component.ts:172` (`isDevMode()` gate).

- [ ] **Step 3: Verify the gate** — with `showSelfHealing:false`, navigating to `/doorway/self-healing` redirects to `/`; with it true (or in `ng serve`), the view renders.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/doorway/doorway.routes.ts
git commit -m "feat(elohim-app): route the gated self-healing debug view (Plan C)"
```

---

## Task 7: Lock Rust↔schema drift with a contract test

**Files:**
- Modify: `doorway/doorway-service/tests/schema_contract.rs` (the file exists and already validates the `auth-response` family — the analogous doorway-local Category-C case). **NOT** `elohim/elohim-storage/tests/schema_contract.rs` — `SelfHealingView` lives in doorway-service.

- [ ] **Step 1: Add a test** that builds a sample `SelfHealingView` (or calls `compose_self_healing(...)` — module is public: `pub mod self_healing` in `routes/mod.rs:29`, same access pattern as the existing `doorway::routes::auth_routes` import), serializes it, and calls `validate_against_schema("views/stability-status-view.schema.json", &json)` (helper at `schema_contract.rs:125`) plus `assert_source_of_truth_declared(...)` (`:144`). Cover the PENDING null/empty fields (`autoPreset`/`admission` null, `upstreams` `[]`) so the forward-compat seam is locked.

- [ ] **Step 2: Run the gate**

Run (from `doorway/doorway-service/`): `RUSTFLAGS="" CARGO_TARGET_DIR=<pool slot> cargo test --test schema_contract`
Expected: PASS. (Per root CLAUDE.md: native build needs `RUSTFLAGS=""`; set `CARGO_TARGET_DIR` to the pool slot for this worktree; use plain `cargo test`, and never `&&`-pipe the gate exit code.)

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/tests/schema_contract.rs
git commit -m "test(doorway): schema_contract for stability-status-view (Plan C)"
```

---

## Task 8 (optional follow-on, NOT in scope unless asked): doorway-app operator tab

The deployed operator dashboard (`doorway/doorway-app/`, served at `/threshold/`, auth-gated) is the natural **operator** home for the same view — add a "Self-Healing" tab mirroring `topology-tab.component.ts` (which already consumes a composed `/admin/dashboard/topology` view). The codegen in Task 1 already distributes the type to `doorway/doorway-app/src/app/generated/`. This reaches the human-operator consumer; the elohim-app view (this plan) is the one that reaches the on-device/agent consumer. Build only if the user wants the operator surface too.

---

## Security consideration (surface to the operator — do NOT hardening-project this)

`GET /admin/self-healing` is **unauthenticated and, per grounding, currently publicly reachable** on the prod/alpha doorway (no ingress `/admin/*` rule was found; the "operator-only" property is aspirational). The block exposes, among low-sensitivity counters, **peer IDs plus per-peer health/topology** (`peers[].peer`, `peers[].status`). In a privacy-oriented P2P protocol that is a **deanonymization / targeting surface** — materially more sensitive than render counters, even though it carries no secrets or user PII.

This is **not a blocker** for the debug view, but the executor must:
1. Keep the debug flag default **false** in prod/alpha (Task 4) — UI visibility is not the leak; the open endpoint is.
2. **Flag for the operator** (who owns ingress/cluster, per CLAUDE.md) the decision to ingress-protect `/admin/self-healing` (and the sibling `/admin/*` read endpoints) — e.g. restrict `/admin/*` at the ingress to operator networks. This is an operator/cluster action, out of scope for this frontend plan.

Stop there. Do not build auth into the endpoint as part of this work.

---

## Open confirm-points for the user (decide before/while executing)

1. **Gate mechanism:** build-time `FeatureFlag` (this plan) vs runtime `ConfigService` toggle (Task 4 confirm-point). Default: build-time flag.
2. **Route home:** `/doorway/self-healing` (co-located with the existing `/admin/*` consumer) vs top-level `/debug/self-healing`. Default: `/doorway/self-healing`.
3. **Alpha visibility:** flag on or off on alpha (Task 4 Step 2). Default: off until requested.
4. **doorway-app operator tab:** build Task 8 too, or elohim-app only? Default: elohim-app only.

---

## Self-review checklist (run before handing back)

- [ ] Codegen `--verify` is green and the generated `StabilityStatusView` is committed in all 6 dirs.
- [ ] Both `proxy.conf.mjs` and `proxy.conf.alpha.mjs` got `/admin`.
- [ ] Service handles 200 / 503-catching-up / non-JSON-5xx distinctly.
- [ ] Component renders PENDING (`autoPreset`/`admission`/`upstreams`) distinctly from cold-null, and degrades gracefully in Tauri.
- [ ] Route is gated; flag defaults false in prod.
- [ ] Contract test is in **doorway-service**, green under `RUSTFLAGS=""`.
- [ ] Debug-flag visibility is documented as NOT access control; the open-endpoint security note is surfaced to the operator.

---

*Backend grounding for this handoff (file:line) is current as of branch `shift/self-healing-control-plane` HEAD. If you execute on a later branch, re-confirm the recall-sensitive anchors (`INTERFACE_FILES` format, the two `proxy.conf` context arrays, `DoorwayAdminService` GET pattern, the `StabilityStatusView` type name) — they are the ones that break execution if they've drifted.*
