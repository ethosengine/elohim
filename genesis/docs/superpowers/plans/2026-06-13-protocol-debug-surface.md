# Protocol Debug Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a shell-level, hidden-but-accessible `/debug` surface to elohim-app (chrome://flags-faithful) that is reused across both deployment contexts (web-via-doorway and tauri-direct-to-storage), routes each diagnostic "lens" per context, degrades honestly, and exposes the native log-level range.

**Architecture:** elohim-app is ONE Angular build that runs in both web and tauri. A top-level `/debug` route (no redirecting guard — it always resolves; only its nav entry is gated) hosts a tab UI of standalone "lens" components. A `DebugContextService` tells each lens the active `ConnectionMode` and the storage base URL so it sources data per context and renders three honest states per block: real / pending-wire-up / N/A-in-this-context. Backend source is untouched: only a dev-proxy edit, a codegen registration, and a doorway schema-contract test touch the tree. Spec: `genesis/docs/superpowers/specs/2026-06-13-protocol-debug-surface-design.md`.

**Tech Stack:** Angular 19 (standalone components, signals, `inject()`), TypeScript strict, Vitest, JSON-Schema→TS codegen (`json-schema-to-typescript`), Rust (doorway-service contract test only).

---

## ⚠ Read before you touch anything (facts that break execution if missed)

1. **The generated TS interface is `StabilityStatusView`, NOT `SelfHealingView`.** The Rust struct is `SelfHealingView` (`doorway/doorway-service/src/routes/self_healing.rs`); the JSON-Schema `title` is `"StabilityStatusView"`, and codegen names the interface from the title. Use the schema-codegen path only; do NOT add `#[derive(TS)]` to the Rust struct or hand-write an interface.
2. **The `/debug` route has NO redirecting guard.** chrome://flags has no visible affordance — you reach it by URL. The route always resolves in all contexts; only the *nav entry* is gated on debug mode. Gating a route that reads an already-public endpoint protects nothing (see Security in the spec).
3. **`/admin/self-healing` exists only on doorway.** In web context the browser reaches it (same-origin in prod; via the Task 2 dev-proxy in dev). In tauri the app talks to elohim-storage `:8090` directly and `/admin/self-healing` 404s there — the StabilityLens must branch on context and compose from storage's raw status endpoints in tauri, NOT call the doorway endpoint.
4. **In dev, an unproxied request silently returns `index.html` (HTTP 200).** Without the Task 2 proxy edit, `fetch('/admin/self-healing')` from `:4200` returns the SPA shell; `JSON.parse` then throws on HTML. Task 2 fixes it for both `proxy.conf.mjs` and `proxy.conf.alpha.mjs`.
5. **In tauri, only the `projector` block is honestly reconstructable.** `peers` (doorway signal-peer health), `render` (SSR trace), `warmup` (projection warm-stream), `conductor` (doorway worker *pool*) are doorway-role state with no single-node analog. Render them "N/A on this node" — do NOT fabricate values. `autoPreset`/`admission`/`upstreams` are PENDING wire-up everywhere (null/`[]`) and render "pending wire-up", distinct from N/A.
6. **The worktree may have no `node_modules`.** Run `pnpm install` at repo root once (Task 1) — codegen imports `json-schema-to-typescript` and Vitest needs deps.
7. **Native Rust test needs `RUSTFLAGS=""` + a pool target dir.** Per root CLAUDE.md: native (doorway) builds break under the WASM getrandom flag. Set `CARGO_TARGET_DIR` to this worktree's pool slot (Task 12 gives the exact path; confirm with `bash genesis/agentic/bin/cargo-pool key` from the crate dir).

---

## What already exists (do NOT rebuild)

- **Backend (landed `82edc611e`, on this branch):** `GET /admin/self-healing` → `SelfHealingView`, composed in Rust by `compose_self_healing()` (`doorway/doorway-service/src/routes/self_healing.rs`). The Angular side must NOT re-aggregate.
- **Wire contract (authored, NOT codegen-registered):** `elohim/sdk/schemas/v1/views/stability-status-view.schema.json`. Task 1 registers it.
- **Storage raw status endpoints (for the tauri path):** `GET /api/v1/status/projector` → `ProjectorStatusView { cursors, lag[] }` with `lag[].lagSeconds: number|null` (`elohim/elohim-storage/src/projector/status.rs`); `GET /p2p/status` → `P2PStatusInfo` with `projectionReconcile?: { caughtUp: boolean; divergentAnchor: number; ... }` (`elohim/elohim-storage/src/p2p/mod.rs`, `p2p/projection_reconcile.rs`).
- **Per-context router:** `CONNECTION_STRATEGY` DI token + `detectConnectionMode()` (`@elohim/service/connection`, returns `'tauri' | 'direct' | 'doorway'`).
- **Runtime log knob:** `LoggerService` (`app/elohim-app/src/app/elohim/services/logger.service.ts`) — 4 levels `debug/info/warn/error`, `setMinLevel()`, `getRecentLogs()` (signal-backed, 100-entry buffer), `clearRecentLogs()`.
- **Built-but-dark health asset:** `HealthIndicatorComponent` + `HealthCheckService` (`app/elohim-app/src/app/elohim/components/health-indicator/`, `services/health-check.service.ts`).
- **HTTP idiom to mirror:** `DoorwayAdminService.getNodes()` (`app/elohim-app/src/app/doorway/services/doorway-admin.service.ts`) — `baseUrl = environment.doorwayUrl ?? ''`, `http.get<T>().pipe(timeout, retry, catchError(handleError))`.

---

## File Structure

**Create (all under `app/elohim-app/src/app/`):**
- `debug/debug.types.ts` — `DebugLens` interface, `BlockState<T>` helper, `DEBUG_LENSES` registry.
- `debug/debug-shell.component.{ts,html,scss}` — the `/debug` page: tab list + active lens.
- `debug/lenses/connection-lens.component.ts` — context/mode/base-URLs (inline template, cheapest).
- `debug/lenses/stability-lens.component.{ts,html,scss}` — `StabilityStatusView`, web fetch vs tauri compose.
- `debug/lenses/logging-lens.component.{ts,html,scss}` — native-level knob + live log viewer.
- `debug/lenses/health-lens.component.ts` — mounts `HealthIndicatorComponent`.
- `debug/lenses/flags-lens.component.ts` — read-only `FeatureFlags` display + drift note.
- `elohim/services/debug-context.service.ts` — active mode + storage base URL.
- `services/debug-mode.service.ts` — sticky (localStorage) nav-visibility flag.
- `doorway/doorway-service/tests/schema_contract.rs` **entries** (file exists; add test fns).

**Modify:**
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — register the view in `INTERFACE_FILES`.
- `app/elohim-app/proxy.conf.mjs` + `app/elohim-app/proxy.conf.alpha.mjs` — add `/admin`.
- `app/elohim-app/src/environments/environment.types.ts` — add `showDebug?: boolean` to `FeatureFlags`.
- `app/elohim-app/src/environments/environment.ts` (`showDebug: true`), `.prod.ts` (`false`), `.alpha.ts` (`false`).
- `app/elohim-app/src/app/app.routes.ts` — top-level `/debug` route.
- `app/elohim-app/src/app/doorway/services/doorway-admin.service.ts` — add `getSelfHealing()`.
- `app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts` — gated `/debug` nav entry.

**Generated (do NOT hand-edit; produced by Task 1):**
- `stability-status-view.ts` in all 6 `GENERATED_OUTPUT_DIRS` (the elohim-app one: `app/elohim-app/src/app/generated/stability-status-view.ts`).

---

## Task 1: Register the wire contract for TS codegen

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (the `INTERFACE_FILES` array, in the `views/` block ~line 52-186)

- [ ] **Step 1: Install workspace deps** (one-time; codegen + Vitest need them)

Run: `pnpm install` (from repo root)
Expected: completes without error; `node_modules/` populated.

- [ ] **Step 2: Add the view to `INTERFACE_FILES`**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, find the `views/` entries in `INTERFACE_FILES` (e.g. the line `{ src: 'views/p2p-status-view.ts', dest: 'p2p-status-view.ts' },`) and add directly after it:

```javascript
  { src: 'views/stability-status-view.ts', dest: 'stability-status-view.ts' },
```

- [ ] **Step 3: Generate + distribute**

Run: `pnpm run schema:codegen:ts`
Expected: a new `stability-status-view.ts` exporting `interface StabilityStatusView` appears in all 6 `GENERATED_OUTPUT_DIRS`, including `app/elohim-app/src/app/generated/stability-status-view.ts`. Confirm the interface fields match the schema: `autoPreset`, `admission`, `upstreams`, `projector { lagSeconds, caughtUp, divergentAnchor }`, `peers`, `render { total, degenerateRate }`, `warmup`, `conductor`.

- [ ] **Step 4: Verify idempotent + fresh**

Run: `pnpm run schema:codegen:ts -- --verify`
Expected: exit 0, no diff. (The `--` passthrough is the form `.husky/pre-push` uses.) The `circuit` enum is an inline field, not a top-level union alias, so the known Reach/ContentFormat Prettier oscillation does not apply. If `--verify` flags a cosmetic re-wrap, re-run codegen once and commit the settled output.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/schemas/generated-ts/ \
        app/elohim-app/src/app/generated/stability-status-view.ts \
        app/elohim-library/projects/elohim-service/src/generated/stability-status-view.ts \
        app/elohim-library/projects/elohim-identity/src/generated/stability-status-view.ts \
        doorway/doorway-app/src/app/generated/stability-status-view.ts \
        app/lamad/src/generated/stability-status-view.ts \
        genesis/seeder/src/generated/stability-status-view.ts
git commit -m "feat(schema): register stability-status-view for TS codegen (debug surface)"
```

---

## Task 2: Make `/admin` reachable in dev (proxy both files)

**Files:**
- Modify: `app/elohim-app/proxy.conf.mjs` (context array, ~line 11)
- Modify: `app/elohim-app/proxy.conf.alpha.mjs` (context array, ~line 15)

- [ ] **Step 1: Add `/admin` to `proxy.conf.mjs`**

Change the context array from:

```javascript
    context: ['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p'],
```

to:

```javascript
    context: ['/api', '/db', '/blob', '/apps', '/epr-head', '/account', '/health', '/p2p', '/admin'],
```

- [ ] **Step 2: Add `/admin` to `proxy.conf.alpha.mjs`** — same edit to its context array (it currently lacks `/p2p`'s siblings' `/admin`; add `'/admin'` to the end of its context array).

- [ ] **Step 3: Commit**

```bash
git add app/elohim-app/proxy.conf.mjs app/elohim-app/proxy.conf.alpha.mjs
git commit -m "feat(elohim-app): dev-proxy /admin to doorway for debug surface"
```

> No manual stack run is required here; reachability is verified end-to-end in Task 8's manual check. This is a dev-proxy edit only — no doorway/storage source changes.

---

## Task 3: Add `getSelfHealing()` to `DoorwayAdminService`

**Files:**
- Modify: `app/elohim-app/src/app/doorway/services/doorway-admin.service.ts`
- Test: `app/elohim-app/src/app/doorway/services/doorway-admin.service.spec.ts` (create if absent)

- [ ] **Step 1: Write the failing test**

Create/append `doorway-admin.service.spec.ts`:

```typescript
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { firstValueFrom } from 'rxjs';
import { DoorwayAdminService } from './doorway-admin.service';

describe('DoorwayAdminService.getSelfHealing', () => {
  let service: DoorwayAdminService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [DoorwayAdminService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(DoorwayAdminService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('GETs /admin/self-healing and returns the typed view', async () => {
    const promise = firstValueFrom(service.getSelfHealing());
    const req = httpMock.expectOne((r) => r.url.endsWith('/admin/self-healing'));
    expect(req.request.method).toBe('GET');
    req.flush({
      autoPreset: null, admission: null, upstreams: [],
      projector: { lagSeconds: 0, caughtUp: true, divergentAnchor: 0 },
      peers: [], render: { total: 0, degenerateRate: 0 },
      warmup: { inProgress: false, attempts: 0, completed: false, lastError: null },
      conductor: { connected: true, connectedWorkers: 1, totalWorkers: 1 },
    });
    const view = await promise;
    expect(view.projector.caughtUp).toBe(true);
    expect(view).toHaveProperty('peers');
  });
});
```

Run: `pnpm exec vitest run --config vite.config.ts doorway-admin.service`
Expected: FAIL — `service.getSelfHealing is not a function`.

- [ ] **Step 2: Implement, mirroring `getNodes()`**

Add the import near the top of `doorway-admin.service.ts`:

```typescript
import type { StabilityStatusView } from '../../generated/stability-status-view';
```

Add the method to the class (after `getNodes()`):

```typescript
  /**
   * Doorway self-healing read model (debug surface). Node-local, unauthenticated
   * by design — see the /debug visibility gate (UI visibility only, not access
   * control). Web context only; in tauri the StabilityLens composes from storage
   * raw status endpoints instead (this endpoint 404s on elohim-storage).
   */
  getSelfHealing(): Observable<StabilityStatusView> {
    return this.http.get<StabilityStatusView>(`${this.baseUrl}/admin/self-healing`).pipe(
      timeout(this.timeout),
      retry(1),
      catchError((error: HttpErrorResponse) => {
        if (error.status === 503) {
          console.warn('[DoorwayAdmin] getSelfHealing 503: node catching up');
        }
        return throwError(() => error);
      })
    );
  }
```

Add `throwError` to the existing rxjs import line:

```typescript
import { Observable, Subject, catchError, of, retry, throwError, timeout } from 'rxjs';
```

> Note: unlike `getNodes()` (which swallows errors into a fallback via `handleError`), `getSelfHealing()` re-throws so the StabilityLens can distinguish 503 "catching up" / 404 / other and render honestly. Never `JSON.parse` a body here — `HttpClient` already parses JSON, and on a non-JSON 5xx body it surfaces the error to `catchError`.

- [ ] **Step 3: Run the test** — Expected: PASS.

Run: `pnpm exec vitest run --config vite.config.ts doorway-admin.service`

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/doorway/services/doorway-admin.service.ts \
        app/elohim-app/src/app/doorway/services/doorway-admin.service.spec.ts
git commit -m "feat(elohim-app): DoorwayAdminService.getSelfHealing (debug surface)"
```

---

## Task 4: `DebugContextService` (active mode + storage base URL)

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/debug-context.service.ts`
- Test: `app/elohim-app/src/app/elohim/services/debug-context.service.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
import { TestBed } from '@angular/core/testing';
import { DebugContextService } from './debug-context.service';

describe('DebugContextService', () => {
  beforeEach(() => TestBed.configureTestingModule({ providers: [DebugContextService] }));

  it('reports doorway mode + empty storage base in a browser test env', () => {
    const svc = TestBed.inject(DebugContextService);
    // jsdom test env: no __TAURI__, no process.versions.node guaranteed → doorway.
    expect(['doorway', 'direct', 'tauri']).toContain(svc.mode());
  });

  it('routes the storage base URL by mode', () => {
    const svc = TestBed.inject(DebugContextService);
    const base = svc.storageBaseUrl();
    // doorway → '' (same-origin) | tauri/direct → http://localhost:8090
    expect(base === '' || base === 'http://localhost:8090').toBe(true);
  });
});
```

Run: `pnpm exec vitest run --config vite.config.ts debug-context.service`
Expected: FAIL — cannot find `./debug-context.service`.

- [ ] **Step 2: Implement**

```typescript
import { Injectable, computed, signal } from '@angular/core';
import { detectConnectionMode } from '@elohim/service/connection';
import { environment } from '../../../environments/environment';

/** Active deployment-context descriptor for the debug surface. The single place
 *  lenses consult to source data per context and degrade honestly. */
@Injectable({ providedIn: 'root' })
export class DebugContextService {
  /** 'doorway' (web→doorway) | 'tauri' (native) | 'direct' (CLI/node). */
  readonly mode = signal(detectConnectionMode());

  /** True in the native desktop shell. */
  readonly isTauri = computed(() => this.mode() === 'tauri');

  /** True when this context talks to elohim-storage directly (no doorway). */
  readonly isDirectStorage = computed(() => this.mode() !== 'doorway');

  /** Environment name (development / alpha / production). */
  readonly environmentName = environment.environment;

  /**
   * Base URL for HTTP reads:
   *  - doorway: '' (same-origin; in dev the proxy forwards to :8888) — used for /admin/*.
   *  - tauri/direct: the elohim-storage sidecar at :8090 (per elohim-storage/CLAUDE.md
   *    "Tauri/Direct … localhost:8090 … same HTTP routes as the proxied path") — used for
   *    /api/v1/status/* and /p2p/status.
   */
  readonly storageBaseUrl = computed(() =>
    this.mode() === 'doorway' ? (environment.doorwayUrl ?? '') : 'http://localhost:8090'
  );
}
```

- [ ] **Step 3: Run the test** — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/debug-context.service.ts \
        app/elohim-app/src/app/elohim/services/debug-context.service.spec.ts
git commit -m "feat(elohim-app): DebugContextService (per-context debug routing)"
```

---

## Task 5: `DebugModeService` (sticky nav-visibility flag)

**Files:**
- Create: `app/elohim-app/src/app/services/debug-mode.service.ts`
- Test: `app/elohim-app/src/app/services/debug-mode.service.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
import { TestBed } from '@angular/core/testing';
import { DebugModeService } from './debug-mode.service';

describe('DebugModeService', () => {
  beforeEach(() => {
    localStorage.removeItem('elohim-debug');
    TestBed.configureTestingModule({ providers: [DebugModeService] });
  });

  it('enable() persists and disable() clears the sticky flag', () => {
    const svc = TestBed.inject(DebugModeService);
    svc.enable();
    expect(localStorage.getItem('elohim-debug')).toBe('on');
    expect(svc.navVisible()).toBe(true);
    svc.disable();
    expect(localStorage.getItem('elohim-debug')).toBeNull();
  });
});
```

Run: `pnpm exec vitest run --config vite.config.ts debug-mode.service`
Expected: FAIL — cannot find `./debug-mode.service`.

- [ ] **Step 2: Implement**

```typescript
import { Injectable, isDevMode, signal } from '@angular/core';

const KEY = 'elohim-debug';

/** Controls whether the /debug NAV entry is shown. The /debug route itself always
 *  resolves by URL (chrome://flags model) — this gates discoverability only, not
 *  access. Sticky via localStorage so a flip survives reload. */
@Injectable({ providedIn: 'root' })
export class DebugModeService {
  private readonly sticky = signal(this.readSticky());

  /** Nav entry visible when dev-mode OR the user flipped the sticky flag. */
  readonly navVisible = () => isDevMode() || this.sticky();

  enable(): void {
    try { localStorage.setItem(KEY, 'on'); } catch { /* storage unavailable */ }
    this.sticky.set(true);
  }

  disable(): void {
    try { localStorage.removeItem(KEY); } catch { /* storage unavailable */ }
    this.sticky.set(false);
  }

  private readSticky(): boolean {
    try { return localStorage.getItem(KEY) === 'on'; } catch { return false; }
  }
}
```

- [ ] **Step 3: Run the test** — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/services/debug-mode.service.ts \
        app/elohim-app/src/app/services/debug-mode.service.spec.ts
git commit -m "feat(elohim-app): DebugModeService (sticky debug-nav flag)"
```

---

## Task 6: `showDebug` feature flag

**Files:**
- Modify: `app/elohim-app/src/environments/environment.types.ts` (the `FeatureFlags` interface, ~line 90-99)
- Modify: `environment.ts`, `environment.prod.ts`, `environment.alpha.ts` (the `features` blocks)

- [ ] **Step 1: Add the flag to the interface**

In `environment.types.ts`, inside `interface FeatureFlags`, add after `useGraphqlTopology?: boolean;`:

```typescript
  /**
   * Seed the /debug nav entry visible by default in this environment build.
   * UI visibility only — the /debug route always resolves by URL and the
   * endpoints it reads are public, so this is NOT access control. Default false;
   * DebugModeService also honors isDevMode() and a sticky localStorage flag.
   */
  showDebug?: boolean;
```

- [ ] **Step 2: Set it per environment**

`environment.ts` (dev) `features` block — add `showDebug: true,`:

```typescript
  features: {
    useGraphqlTopology: true,
    showDebug: true,
  },
```

`environment.prod.ts` and `environment.alpha.ts` `features` blocks — add `showDebug: false,` (explicit, default-off in deployed builds):

```typescript
  features: {
    useGraphqlTopology: true,
    showDebug: false,
  },
```

> The flag is a *seed* for the nav entry; `DebugModeService` already ORs `isDevMode()`. It is wired into `DebugModeService.readSticky()` only if you want env-seeding — for MVP, `DebugModeService` reads localStorage + isDevMode; the env flag is consumed by the navigator gate in Task 7 (`environment.features?.showDebug`). Keep it simple: the navigator shows the entry when `debugMode.navVisible() || environment.features?.showDebug`.

- [ ] **Step 3: Verify the app still type-checks**

Run: `pnpm exec vitest run --config vite.config.ts debug-mode.service` (sanity; environments compile)
Expected: PASS (no type errors from the new optional flag).

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/environments/
git commit -m "feat(elohim-app): showDebug feature flag (default false in deployed builds)"
```

---

## Task 7: Debug shell + lens registry + ConnectionLens + route + nav entry

This task ships a working `/debug` page with its first (cheapest) lens, proving the shell. Tasks 8-11 add more lenses.

**Files:**
- Create: `app/elohim-app/src/app/debug/debug.types.ts`
- Create: `app/elohim-app/src/app/debug/lenses/connection-lens.component.ts`
- Create: `app/elohim-app/src/app/debug/debug-shell.component.ts` (+ `.html`, `.scss`)
- Modify: `app/elohim-app/src/app/app.routes.ts`
- Modify: `app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts`

- [ ] **Step 1: Define the lens contract + registry**

Create `debug/debug.types.ts`:

```typescript
import { Type } from '@angular/core';

/** Per-block availability for honest rendering across contexts. */
export type BlockAvailability = 'real' | 'pending' | 'na' | 'loading' | 'error';

/** A debug block's value plus how to render its availability. */
export interface BlockState<T> {
  state: BlockAvailability;
  value?: T;
  /** Why a non-'real' state (e.g. "doorway-role — N/A on this node"). */
  note?: string;
}

/** A registered debug lens (one tab in the shell). */
export interface DebugLens {
  id: string;
  title: string;
  icon: string;
  component: Type<unknown>;
}
```

- [ ] **Step 2: Build the ConnectionLens (first lens — no external deps)**

Create `debug/lenses/connection-lens.component.ts`:

```typescript
import { CommonModule } from '@angular/common';
import { Component, inject } from '@angular/core';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { environment } from '../../../environments/environment';

/** Answers "what context am I in?" — the cheapest, always-available lens. */
@Component({
  selector: 'app-connection-lens',
  standalone: true,
  imports: [CommonModule],
  template: `
    <dl class="debug-kv">
      <dt>Connection mode</dt><dd>{{ ctx.mode() }}</dd>
      <dt>Tauri (native)</dt><dd>{{ ctx.isTauri() ? 'yes' : 'no' }}</dd>
      <dt>Direct-to-storage</dt><dd>{{ ctx.isDirectStorage() ? 'yes' : 'no' }}</dd>
      <dt>Storage base URL</dt><dd>{{ ctx.storageBaseUrl() || '(same-origin)' }}</dd>
      <dt>Doorway URL</dt><dd>{{ doorwayUrl || '(same-origin)' }}</dd>
      <dt>Environment</dt><dd>{{ ctx.environmentName }}</dd>
      <dt>Production build</dt><dd>{{ production ? 'yes' : 'no' }}</dd>
      <dt>Git hash</dt><dd>{{ gitHash }}</dd>
    </dl>
  `,
  styles: [
    `.debug-kv { display: grid; grid-template-columns: max-content 1fr; gap: 0.25rem 1rem; }
     dt { font-weight: 600; opacity: 0.8; } dd { margin: 0; font-family: monospace; }`,
  ],
})
export class ConnectionLensComponent {
  readonly ctx = inject(DebugContextService);
  readonly doorwayUrl = environment.doorwayUrl ?? '';
  readonly production = environment.production;
  readonly gitHash = environment.gitHash;
}
```

- [ ] **Step 3: Build the shell**

Create `debug/debug-shell.component.ts`:

```typescript
import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import { DebugLens } from './debug.types';
import { ConnectionLensComponent } from './lenses/connection-lens.component';

/** Hidden-but-accessible /debug surface (chrome://flags model). Always resolves
 *  by URL; the nav entry is gated separately (DebugModeService). Read-only +
 *  client-local toggles only — no backend actuation. */
@Component({
  selector: 'app-debug-shell',
  standalone: true,
  imports: [CommonModule, ConnectionLensComponent],
  templateUrl: './debug-shell.component.html',
  styleUrl: './debug-shell.component.scss',
})
export class DebugShellComponent {
  // Registry — Tasks 8-11 append Stability, Logging, Health, Flags here.
  readonly lenses: DebugLens[] = [
    { id: 'connection', title: 'Connection', icon: '🔌', component: ConnectionLensComponent },
  ];
  readonly activeId = signal(this.lenses[0].id);
  select(id: string): void { this.activeId.set(id); }
}
```

Create `debug-shell.component.html`:

```html
<section class="debug-shell">
  <header class="debug-header">
    <h1>Protocol Debug</h1>
    <p class="debug-subtitle">
      Node-local diagnostics for this {{ '' }}device. Read-only. Reflects whichever
      replica/node answered — not a cluster-wide aggregate.
    </p>
  </header>

  <nav class="debug-tabs" role="tablist">
    <button
      type="button"
      *ngFor="let lens of lenses"
      role="tab"
      class="debug-tab"
      [class.active]="lens.id === activeId()"
      [attr.aria-selected]="lens.id === activeId()"
      (click)="select(lens.id)"
    >
      <span class="tab-icon">{{ lens.icon }}</span> {{ lens.title }}
    </button>
  </nav>

  <div class="debug-panel" role="tabpanel">
    <ng-container *ngFor="let lens of lenses">
      <ng-container *ngIf="lens.id === activeId()">
        <ng-container *ngComponentOutlet="lens.component" />
      </ng-container>
    </ng-container>
  </div>
</section>
```

Create `debug-shell.component.scss`:

```scss
.debug-shell { max-width: 900px; margin: 0 auto; padding: 1.5rem; }
.debug-header h1 { margin: 0 0 0.25rem; }
.debug-subtitle { opacity: 0.7; font-size: 0.9rem; margin: 0 0 1rem; }
.debug-tabs { display: flex; flex-wrap: wrap; gap: 0.25rem; border-bottom: 1px solid var(--color-border, #ccc); margin-bottom: 1rem; }
.debug-tab { background: none; border: none; padding: 0.5rem 0.75rem; cursor: pointer; opacity: 0.7; border-bottom: 2px solid transparent; }
.debug-tab.active { opacity: 1; border-bottom-color: var(--color-accent, #6a8); font-weight: 600; }
.debug-panel { padding: 0.5rem 0; }
```

> `*ngComponentOutlet` is used so Tasks 8-11 only append to the `lenses` array — no template churn. `CommonModule` provides `*ngComponentOutlet`, `*ngFor`, `*ngIf`.

- [ ] **Step 4: Add the top-level `/debug` route (NO guard)**

In `app/elohim-app/src/app/app.routes.ts`, add this entry among the top-level single-component routes (e.g. directly before the `map` route), mirroring the existing `loadComponent` pattern:

```typescript
  // Hidden-but-accessible debug surface (chrome://flags model). Always resolves
  // by URL; the nav entry is gated by DebugModeService. No guard — it reads only
  // already-public endpoints, so gating the route would protect nothing.
  {
    path: 'debug',
    loadComponent: async () =>
      import('./debug/debug-shell.component').then(m => m.DebugShellComponent),
    title: 'Protocol Debug',
  },
```

- [ ] **Step 5: Add the gated nav entry**

In `elohim-navigator.component.ts`, inject `DebugModeService` and `environment`, then extend the `contextApps` computed. Add the import + field:

```typescript
import { DebugModeService } from '../../../services/debug-mode.service';
import { environment } from '../../../../environments/environment';
// ...
private readonly debugMode = inject(DebugModeService);

private readonly debugApp: ContextAppConfig = {
  id: 'debug',
  name: 'Debug',
  icon: '🛠️',
  route: '/debug',
  tagline: 'Node Diagnostics',
  available: true,
};
```

Extend the existing `contextApps` computed (it already pushes `doorwayApp` conditionally) to also push `debugApp`:

```typescript
readonly contextApps = computed(() => {
  const apps = [...this.baseContextApps];
  if (this.runningContext.hasDoorwayCapableNode() || isDevMode()) {
    apps.push(this.doorwayApp);
  }
  if (this.debugMode.navVisible() || environment.features?.showDebug) {
    apps.push(this.debugApp);
  }
  return apps;
});
```

> `inject` and `isDevMode`/`computed` are already imported in this component (the existing gate uses them). Add only `DebugModeService` and `environment` imports.

- [ ] **Step 6: Build to verify it compiles + renders**

Run: `pnpm exec vitest run --config vite.config.ts debug-context.service debug-mode.service`
Expected: PASS (smoke). Then a production build sanity:
Run: `pnpm run build`
Expected: build succeeds (the `/debug` lazy chunk emits; no template/type errors).

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/debug/ \
        app/elohim-app/src/app/app.routes.ts \
        app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts
git commit -m "feat(elohim-app): /debug shell + lens registry + ConnectionLens + gated nav"
```

---

## Task 8: StabilityLens (web fetch vs tauri compose, honest per-block)

**Files:**
- Create: `app/elohim-app/src/app/debug/lenses/stability-lens.component.ts` (+ `.html`, `.scss`)
- Test: `app/elohim-app/src/app/debug/lenses/stability-lens.component.spec.ts`
- Modify: `app/elohim-app/src/app/debug/debug-shell.component.ts` (register the lens)

- [ ] **Step 1: Write the failing test** (covers both context branches + honest states)

```typescript
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { signal } from '@angular/core';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { StabilityLensComponent } from './stability-lens.component';

describe('StabilityLensComponent', () => {
  function setup(mode: 'doorway' | 'tauri') {
    const ctx = {
      mode: signal(mode),
      isTauri: signal(mode === 'tauri'),
      isDirectStorage: signal(mode !== 'doorway'),
      storageBaseUrl: signal(mode === 'doorway' ? '' : 'http://localhost:8090'),
      environmentName: 'test',
    };
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(), provideHttpClientTesting(),
        { provide: DebugContextService, useValue: ctx },
      ],
    });
    const fixture = TestBed.createComponent(StabilityLensComponent);
    const httpMock = TestBed.inject(HttpTestingController);
    return { fixture, httpMock };
  }

  it('doorway: marks autoPreset PENDING and projector REAL', async () => {
    const { fixture, httpMock } = setup('doorway');
    fixture.detectChanges(); // ngOnInit fires the fetch
    httpMock.expectOne((r) => r.url.endsWith('/admin/self-healing')).flush({
      autoPreset: null, admission: null, upstreams: [],
      projector: { lagSeconds: 3, caughtUp: true, divergentAnchor: 0 },
      peers: [], render: { total: 5, degenerateRate: 0 },
      warmup: { inProgress: false, attempts: 0, completed: true, lastError: null },
      conductor: { connected: true, connectedWorkers: 1, totalWorkers: 1 },
    });
    await fixture.whenStable();
    expect(fixture.componentInstance.blocks().autoPreset.state).toBe('pending');
    expect(fixture.componentInstance.blocks().projector.state).toBe('real');
    expect(fixture.componentInstance.blocks().render.state).toBe('real');
  });

  it('tauri: projector REAL from storage, doorway-role blocks N/A', async () => {
    const { fixture, httpMock } = setup('tauri');
    fixture.detectChanges();
    httpMock.expectOne((r) => r.url.endsWith('/api/v1/status/projector')).flush({
      cursors: [], lag: [{ pillar: 'lamad', kind: 'content', lagSeconds: 7 }],
    });
    httpMock.expectOne((r) => r.url.endsWith('/p2p/status')).flush({
      projectionReconcile: { caughtUp: false, divergentAnchor: 2 },
    });
    await fixture.whenStable();
    const b = fixture.componentInstance.blocks();
    expect(b.projector.state).toBe('real');
    expect(b.projector.value?.lagSeconds).toBe(7);
    expect(b.projector.value?.caughtUp).toBe(false);
    expect(b.render.state).toBe('na');
    expect(b.peers.state).toBe('na');
  });
});
```

Run: `pnpm exec vitest run --config vite.config.ts stability-lens`
Expected: FAIL — cannot find `./stability-lens.component`.

- [ ] **Step 2: Implement the component**

Create `debug/lenses/stability-lens.component.ts`:

```typescript
import { CommonModule } from '@angular/common';
import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { StabilityStatusView } from '../../generated/stability-status-view';
import { DoorwayAdminService } from '../../doorway/services/doorway-admin.service';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { BlockState } from '../debug.types';

/** Minimal read-subset of storage's ProjectorStatusView / P2PStatusInfo (canonical
 *  types: ts-rs ProjectorStatusView + P2PStatusInfo in @elohim/storage-client). */
interface ProjectorStatusReadModel { lag: Array<{ lagSeconds: number | null }>; }
interface P2pStatusReadModel {
  projectionReconcile?: { caughtUp: boolean; divergentAnchor: number } | null;
}

type ProjectorBlock = StabilityStatusView['projector'];

interface StabilityBlocks {
  autoPreset: BlockState<unknown>;
  admission: BlockState<unknown>;
  upstreams: BlockState<unknown>;
  projector: BlockState<ProjectorBlock>;
  peers: BlockState<StabilityStatusView['peers']>;
  render: BlockState<StabilityStatusView['render']>;
  warmup: BlockState<StabilityStatusView['warmup']>;
  conductor: BlockState<StabilityStatusView['conductor']>;
}

const NA_NODE = 'doorway-role — not applicable on this single node';
const PENDING = 'pending wire-up (sibling follow-on)';

@Component({
  selector: 'app-stability-lens',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './stability-lens.component.html',
  styleUrl: './stability-lens.component.scss',
})
export class StabilityLensComponent implements OnInit {
  private readonly admin = inject(DoorwayAdminService);
  private readonly http = inject(HttpClient);
  private readonly ctx = inject(DebugContextService);

  readonly error = signal<string | null>(null);
  readonly blocks = signal<StabilityBlocks>(this.loadingBlocks());
  readonly contextNote = computed(() =>
    this.ctx.mode() === 'doorway'
      ? 'Composed by the doorway edge (full self-healing view).'
      : 'Composed on-device from storage status endpoints (node-local).'
  );

  async ngOnInit(): Promise<void> {
    try {
      if (this.ctx.mode() === 'doorway') {
        this.blocks.set(this.fromDoorway(await firstValueFrom(this.admin.getSelfHealing())));
      } else {
        this.blocks.set(await this.fromStorage());
      }
    } catch (e: unknown) {
      this.error.set(this.describe(e));
      this.blocks.set(this.errorBlocks());
    }
  }

  private fromDoorway(v: StabilityStatusView): StabilityBlocks {
    return {
      autoPreset: v.autoPreset == null ? { state: 'pending', note: PENDING } : { state: 'real', value: v.autoPreset },
      admission: v.admission == null ? { state: 'pending', note: PENDING } : { state: 'real', value: v.admission },
      upstreams: !v.upstreams?.length ? { state: 'pending', note: PENDING } : { state: 'real', value: v.upstreams },
      projector: { state: 'real', value: v.projector },
      peers: { state: 'real', value: v.peers },
      render: { state: 'real', value: v.render },
      warmup: { state: 'real', value: v.warmup },
      conductor: { state: 'real', value: v.conductor },
    };
  }

  private async fromStorage(): Promise<StabilityBlocks> {
    const base = this.ctx.storageBaseUrl();
    const [proj, p2p] = await Promise.all([
      firstValueFrom(this.http.get<ProjectorStatusReadModel>(`${base}/api/v1/status/projector`)),
      firstValueFrom(this.http.get<P2pStatusReadModel>(`${base}/p2p/status`)),
    ]);
    const lags = (proj.lag ?? []).map((l) => l.lagSeconds).filter((n): n is number => n != null);
    const projector: ProjectorBlock = {
      lagSeconds: lags.length ? Math.max(...lags) : null,
      caughtUp: p2p.projectionReconcile?.caughtUp ?? null,
      divergentAnchor: p2p.projectionReconcile?.divergentAnchor ?? null,
    };
    return {
      autoPreset: { state: 'pending', note: PENDING },
      admission: { state: 'pending', note: PENDING },
      upstreams: { state: 'pending', note: PENDING },
      projector: { state: 'real', value: projector },
      peers: { state: 'na', note: NA_NODE },
      render: { state: 'na', note: NA_NODE },
      warmup: { state: 'na', note: NA_NODE },
      conductor: { state: 'na', note: NA_NODE },
    };
  }

  private describe(e: unknown): string {
    if (e instanceof HttpErrorResponse) {
      if (e.status === 503) return 'Node catching up (503) — retry shortly.';
      if (e.status === 404) return 'Endpoint not present in this context (404).';
      return `Request failed (${e.status}).`;
    }
    return String(e);
  }

  private base(state: BlockState<unknown>['state']): StabilityBlocks {
    const b: BlockState<never> = { state };
    return { autoPreset: b, admission: b, upstreams: b, projector: b, peers: b, render: b, warmup: b, conductor: b } as unknown as StabilityBlocks;
  }
  private loadingBlocks(): StabilityBlocks { return this.base('loading'); }
  private errorBlocks(): StabilityBlocks { return this.base('error'); }
}
```

Create `debug/lenses/stability-lens.component.html`:

```html
<p class="lens-note">{{ contextNote() }}</p>
<p class="lens-error" *ngIf="error()">{{ error() }}</p>

<div class="block" *ngFor="let row of [
  { key: 'projector', label: 'Projector (lag / caught-up / divergent-anchor)' },
  { key: 'peers', label: 'Peers (signal-peer health)' },
  { key: 'render', label: 'Render (SSR trace)' },
  { key: 'warmup', label: 'Warmup (projection warm-stream)' },
  { key: 'conductor', label: 'Conductor (worker pool)' },
  { key: 'admission', label: 'Admission (inbound semaphore)' },
  { key: 'upstreams', label: 'Upstreams (circuit breakers)' },
  { key: 'autoPreset', label: 'Auto preset (resource policy)' }
]">
  <div class="block-head">
    <span class="block-label">{{ row.label }}</span>
    <span class="block-badge" [attr.data-state]="blocks()[row.key].state">
      {{ blocks()[row.key].state }}
    </span>
  </div>
  <pre class="block-value" *ngIf="blocks()[row.key].state === 'real'">{{ blocks()[row.key].value | json }}</pre>
  <p class="block-note" *ngIf="blocks()[row.key].note">{{ blocks()[row.key].note }}</p>
</div>
```

Create `debug/lenses/stability-lens.component.scss`:

```scss
.lens-note { opacity: 0.7; font-size: 0.85rem; }
.lens-error { color: var(--color-warn, #b85); font-weight: 600; }
.block { border: 1px solid var(--color-border, #ddd); border-radius: 6px; padding: 0.5rem 0.75rem; margin-bottom: 0.5rem; }
.block-head { display: flex; justify-content: space-between; align-items: center; }
.block-label { font-weight: 600; }
.block-badge { font-size: 0.7rem; text-transform: uppercase; padding: 0.1rem 0.4rem; border-radius: 4px; background: var(--color-surface-2, #eee); }
.block-badge[data-state='real'] { background: #d6efd6; }
.block-badge[data-state='pending'] { background: #fff0c8; }
.block-badge[data-state='na'] { background: #e8e8e8; opacity: 0.8; }
.block-badge[data-state='error'] { background: #f3d2d2; }
.block-value { font-family: monospace; font-size: 0.8rem; overflow-x: auto; margin: 0.4rem 0 0; }
.block-note { opacity: 0.65; font-size: 0.8rem; margin: 0.3rem 0 0; }
```

- [ ] **Step 3: Register the lens in the shell**

In `debug-shell.component.ts`, import and append to `lenses`:

```typescript
import { StabilityLensComponent } from './lenses/stability-lens.component';
// ... inside lenses array, after connection:
{ id: 'stability', title: 'Stability', icon: '🩺', component: StabilityLensComponent },
```

- [ ] **Step 4: Run the test** — Expected: PASS.

Run: `pnpm exec vitest run --config vite.config.ts stability-lens`

- [ ] **Step 5: Manual reachability check** (proves Task 2's proxy + the web path)

Run: `pnpm start:alpha` (local UI against live alpha doorway). In the browser console: `await (await fetch('/admin/self-healing')).json()`
Expected: a JSON object with keys `autoPreset, admission, upstreams, projector, peers, render, warmup, conductor` — NOT an HTML string. Then navigate to `http://localhost:4200/debug`, open the Stability tab: projector/peers/render show REAL; autoPreset/admission/upstreams show PENDING. (If you get HTML, the proxy entry didn't take — restart the dev server.)

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/debug/lenses/stability-lens.component.ts \
        app/elohim-app/src/app/debug/lenses/stability-lens.component.html \
        app/elohim-app/src/app/debug/lenses/stability-lens.component.scss \
        app/elohim-app/src/app/debug/lenses/stability-lens.component.spec.ts \
        app/elohim-app/src/app/debug/debug-shell.component.ts
git commit -m "feat(elohim-app): StabilityLens (web fetch / tauri compose, honest per-block)"
```

---

## Task 9: LoggingLens (native-level knob + live log viewer)

**Files:**
- Create: `app/elohim-app/src/app/debug/lenses/logging-lens.component.ts` (+ `.html`, `.scss`)
- Test: `app/elohim-app/src/app/debug/lenses/logging-lens.component.spec.ts`
- Modify: `debug-shell.component.ts`

- [ ] **Step 1: Write the failing test**

```typescript
import { TestBed } from '@angular/core/testing';
import { LoggerService } from '../../elohim/services/logger.service';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { signal } from '@angular/core';
import { LoggingLensComponent } from './logging-lens.component';

describe('LoggingLensComponent', () => {
  it('setLevel() calls LoggerService.setMinLevel and persists', () => {
    const logger = TestBed.configureTestingModule({
      providers: [
        LoggerService,
        { provide: DebugContextService, useValue: { isTauri: signal(false) } },
      ],
    }).inject(LoggerService);
    const spy = vi.spyOn(logger, 'setMinLevel');
    const fixture = TestBed.createComponent(LoggingLensComponent);
    fixture.componentInstance.setLevel('warn');
    expect(spy).toHaveBeenCalledWith('warn');
    expect(localStorage.getItem('elohim-log-level')).toBe('warn');
  });
});
```

> Uses `vi` (Vitest global). Run: `pnpm exec vitest run --config vite.config.ts logging-lens` — Expected: FAIL (no component).

- [ ] **Step 2: Implement**

Create `debug/lenses/logging-lens.component.ts`:

```typescript
import { CommonModule } from '@angular/common';
import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { LoggerService, LogLevel } from '../../elohim/services/logger.service';
import { DebugContextService } from '../../elohim/services/debug-context.service';

const LEVEL_KEY = 'elohim-log-level';
const ANGULAR_LEVELS: LogLevel[] = ['debug', 'info', 'warn', 'error'];
// Rust tracing range — displayed (read-only) in tauri; live-adjust is a follow-on
// (needs a set_log_level IPC + reloadable EnvFilter, neither exists today).
const RUST_LEVELS = ['off', 'error', 'warn', 'info', 'debug', 'trace'];

@Component({
  selector: 'app-logging-lens',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './logging-lens.component.html',
  styleUrl: './logging-lens.component.scss',
})
export class LoggingLensComponent implements OnInit {
  private readonly logger = inject(LoggerService);
  private readonly ctx = inject(DebugContextService);

  readonly angularLevels = ANGULAR_LEVELS;
  readonly rustLevels = RUST_LEVELS;
  readonly current = signal<LogLevel>('debug');
  readonly levelFilter = signal<LogLevel | 'all'>('all');
  readonly isTauri = this.ctx.isTauri;

  readonly logs = computed(() => {
    const f = this.levelFilter();
    const all = this.logger.getRecentLogs();
    return f === 'all' ? all : all.filter((e) => e.level === f);
  });

  ngOnInit(): void {
    const saved = this.readSaved();
    if (saved) { this.logger.setMinLevel(saved); this.current.set(saved); }
  }

  setLevel(level: LogLevel): void {
    this.logger.setMinLevel(level);
    this.current.set(level);
    try { localStorage.setItem(LEVEL_KEY, level); } catch { /* unavailable */ }
  }

  setFilter(f: LogLevel | 'all'): void { this.levelFilter.set(f); }
  clear(): void { this.logger.clearRecentLogs(); }

  private readSaved(): LogLevel | null {
    try {
      const v = localStorage.getItem(LEVEL_KEY);
      return (ANGULAR_LEVELS as string[]).includes(v ?? '') ? (v as LogLevel) : null;
    } catch { return null; }
  }
}
```

Create `debug/lenses/logging-lens.component.html`:

```html
<div class="level-control">
  <span class="control-label">App log level (live):</span>
  <button
    type="button"
    *ngFor="let lvl of angularLevels"
    class="level-btn"
    [class.active]="lvl === current()"
    (click)="setLevel(lvl)"
  >{{ lvl }}</button>
</div>

<p class="rust-levels" *ngIf="isTauri()">
  Native (Rust) range: <code>{{ rustLevels.join(' · ') }}</code> — current set via
  <code>RUST_LOG</code> at launch; live adjust is a follow-on.
</p>

<div class="log-toolbar">
  <span>Recent logs ({{ logs().length }}):</span>
  <select (change)="setFilter($any($event.target).value)">
    <option value="all">all</option>
    <option *ngFor="let lvl of angularLevels" [value]="lvl">{{ lvl }}</option>
  </select>
  <button type="button" (click)="clear()">clear</button>
</div>

<ul class="log-list">
  <li class="log-line" *ngFor="let e of logs()" [attr.data-level]="e.level">
    <span class="log-ts">{{ e.timestamp.split('T')[1].split('.')[0] }}</span>
    <span class="log-level">{{ e.level }}</span>
    <span class="log-msg">{{ e.message }}</span>
  </li>
</ul>
```

Create `debug/lenses/logging-lens.component.scss`:

```scss
.level-control, .log-toolbar { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.6rem; }
.control-label { font-weight: 600; }
.level-btn { border: 1px solid var(--color-border, #ccc); background: none; padding: 0.2rem 0.6rem; cursor: pointer; border-radius: 4px; }
.level-btn.active { background: var(--color-accent, #6a8); color: #fff; }
.rust-levels { opacity: 0.7; font-size: 0.85rem; }
.log-list { list-style: none; padding: 0; margin: 0; max-height: 360px; overflow-y: auto; font-family: monospace; font-size: 0.8rem; }
.log-line { display: grid; grid-template-columns: 5rem 3.5rem 1fr; gap: 0.5rem; padding: 0.1rem 0; }
.log-line[data-level='warn'] { color: #a76b00; }
.log-line[data-level='error'] { color: #b23; }
.log-ts { opacity: 0.6; }
```

- [ ] **Step 3: Register in the shell** (`debug-shell.component.ts`):

```typescript
import { LoggingLensComponent } from './lenses/logging-lens.component';
// ... append:
{ id: 'logging', title: 'Logging', icon: '📜', component: LoggingLensComponent },
```

- [ ] **Step 4: Run the test** — Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/debug/lenses/logging-lens.component.* \
        app/elohim-app/src/app/debug/debug-shell.component.ts
git commit -m "feat(elohim-app): LoggingLens (native level knob + live log viewer)"
```

---

## Task 10: HealthLens (mount the dark HealthIndicatorComponent)

**Files:**
- Create: `app/elohim-app/src/app/debug/lenses/health-lens.component.ts`
- Modify: `debug-shell.component.ts`

- [ ] **Step 1: Implement (wrapper that mounts the existing component)**

```typescript
import { Component } from '@angular/core';
import { HealthIndicatorComponent } from '../../elohim/components/health-indicator/health-indicator.component';

/** Surfaces the (previously unmounted) HealthIndicatorComponent — holochain /
 *  indexedDb / blobCache / network checks. Works in both contexts. */
@Component({
  selector: 'app-health-lens',
  standalone: true,
  imports: [HealthIndicatorComponent],
  template: `<app-health-indicator />`,
})
export class HealthLensComponent {}
```

- [ ] **Step 2: Register in the shell**:

```typescript
import { HealthLensComponent } from './lenses/health-lens.component';
// ... append:
{ id: 'health', title: 'Health', icon: '💓', component: HealthLensComponent },
```

- [ ] **Step 3: Verify build** — Run: `pnpm run build` — Expected: succeeds (HealthIndicatorComponent imports resolve).

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/debug/lenses/health-lens.component.ts \
        app/elohim-app/src/app/debug/debug-shell.component.ts
git commit -m "feat(elohim-app): HealthLens (mounts the dark HealthIndicatorComponent)"
```

---

## Task 11: FlagsLens (read-only FeatureFlags + drift note)

**Files:**
- Create: `app/elohim-app/src/app/debug/lenses/flags-lens.component.ts`
- Modify: `debug-shell.component.ts`

- [ ] **Step 1: Implement**

```typescript
import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { environment } from '../../../environments/environment';

/** Read-only view of build-time FeatureFlags. chrome://flags-style transparency,
 *  not a runtime override framework (YAGNI). Notes the useGraphqlTopology drift. */
@Component({
  selector: 'app-flags-lens',
  standalone: true,
  imports: [CommonModule],
  template: `
    <dl class="debug-kv">
      <ng-container *ngFor="let f of flags">
        <dt>{{ f.key }}</dt><dd>{{ f.value }}</dd>
      </ng-container>
    </dl>
    <p class="flags-note">
      Build-time flags (no runtime override). Note: <code>useGraphqlTopology</code> is
      documented "default false" but set <code>true</code> in every environment build.
    </p>
  `,
  styles: [
    `.debug-kv { display: grid; grid-template-columns: max-content 1fr; gap: 0.25rem 1rem; }
     dt { font-weight: 600; } dd { margin: 0; font-family: monospace; }
     .flags-note { opacity: 0.7; font-size: 0.85rem; margin-top: 0.75rem; }`,
  ],
})
export class FlagsLensComponent {
  readonly flags = Object.entries(environment.features ?? {}).map(([key, value]) => ({
    key,
    value: String(value),
  }));
}
```

- [ ] **Step 2: Register in the shell**:

```typescript
import { FlagsLensComponent } from './lenses/flags-lens.component';
// ... append:
{ id: 'flags', title: 'Flags', icon: '🚩', component: FlagsLensComponent },
```

- [ ] **Step 3: Verify build** — Run: `pnpm run build` — Expected: succeeds.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/debug/lenses/flags-lens.component.ts \
        app/elohim-app/src/app/debug/debug-shell.component.ts
git commit -m "feat(elohim-app): FlagsLens (read-only FeatureFlags + drift note)"
```

---

## Task 12: Lock Rust↔schema drift with a contract test

**Files:**
- Modify: `doorway/doorway-service/tests/schema_contract.rs` (file exists; add a `use` line + two test fns)

- [ ] **Step 1: Add the imports + tests**

Add to the `use` block at the top of `schema_contract.rs`:

```rust
use doorway::routes::self_healing::{
    AdmissionView, ConductorView, PeerView, ProjectorView, RenderView, SelfHealingView,
    UpstreamView, WarmupView,
};
```

Add these two test functions (the file already defines `validate_against_schema(&str, &Value)`, `assert_source_of_truth_declared(&Value, &str)`, and `load_schema(&str) -> Value` — used by the existing auth-response tests):

```rust
#[test]
fn stability_status_view_pending_matches_schema() {
    // PENDING wire-up state: autoPreset/admission null, upstreams empty.
    let view = SelfHealingView {
        auto_preset: None,
        admission: None,
        upstreams: Vec::new(),
        projector: ProjectorView { lag_seconds: None, caught_up: None, divergent_anchor: None },
        peers: Vec::new(),
        render: RenderView { total: 0, degenerate_rate: 0.0 },
        warmup: WarmupView { in_progress: false, attempts: 0, completed: false, last_error: None },
        conductor: ConductorView { connected: false, connected_workers: 0, total_workers: 0 },
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/stability-status-view.schema.json", &json);

    let schema = load_schema("views/stability-status-view.schema.json");
    assert_source_of_truth_declared(&schema, "stability-status-view");
}

#[test]
fn stability_status_view_populated_matches_schema() {
    // Forward-compat: the PENDING fields populated once their siblings wire up.
    let view = SelfHealingView {
        auto_preset: Some(serde_json::json!({ "maxInflight": 64 })),
        admission: Some(AdmissionView { max_inflight: 64, available: 60, shed_total: 3 }),
        upstreams: vec![UpstreamView {
            endpoint: "https://upstream.example".to_string(),
            circuit: "open".to_string(),
            error_streak: 5,
            last_good: Some("2026-06-13T00:00:00Z".to_string()),
            skipped: true,
        }],
        projector: ProjectorView { lag_seconds: Some(7), caught_up: Some(false), divergent_anchor: Some(2) },
        peers: vec![PeerView {
            peer: "uhCAk...".to_string(),
            status: "Degraded".to_string(),
            last_seen: Some("2026-06-13T00:00:00Z".to_string()),
        }],
        render: RenderView { total: 42, degenerate_rate: 0.1 },
        warmup: WarmupView { in_progress: true, attempts: 2, completed: false, last_error: Some("timeout".to_string()) },
        conductor: ConductorView { connected: true, connected_workers: 3, total_workers: 4 },
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/stability-status-view.schema.json", &json);
}
```

> If `load_schema` is private or named differently in this file, mirror exactly how the existing `*_source_of_truth*` test obtains the schema `Value` (grep the file for `assert_source_of_truth_declared` usage). The struct field names/types above are verbatim from `self_healing.rs` (`u64`/`f64`/`usize`/`u32`/`Option<String>`).

- [ ] **Step 2: Run the gate**

Determine this worktree's pool slot:
Run (from `doorway/doorway-service/`): `bash ../../genesis/agentic/bin/cargo-pool key`
Expected: prints a path like `/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev`.

Run (from `doorway/doorway-service/`, substituting that path):
```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev cargo test --test schema_contract stability_status_view
```
Expected: both `stability_status_view_pending_matches_schema` and `stability_status_view_populated_matches_schema` PASS. (Per root CLAUDE.md: native build needs `RUSTFLAGS=""`; never `&&`-pipe the gate exit code — run it as a single command.)

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/tests/schema_contract.rs
git commit -m "test(doorway): schema_contract for stability-status-view (pending + populated)"
```

---

## Task 13 (stretch — build only if explicitly wanted): NativeLens (tauri IPC)

**Files:**
- Create: `app/elohim-app/src/app/debug/lenses/native-lens.component.ts`
- Modify: `debug-shell.component.ts`

The deepest "hop into native territory": the tauri-only `doorway_status` IPC command (`steward/device/src-tauri/src/lib.rs`). In web it renders "native-only — not available in web". This depends on `@tauri-apps/api`'s `invoke`; guard it behind `DebugContextService.isTauri()` and dynamic-import `invoke` so the web bundle never hard-imports tauri APIs.

- [ ] **Step 1: Implement**

```typescript
import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal } from '@angular/core';
import { DebugContextService } from '../../elohim/services/debug-context.service';

/** Tauri-native diagnostics via IPC. Web context shows N/A. */
@Component({
  selector: 'app-native-lens',
  standalone: true,
  imports: [CommonModule],
  template: `
    <p *ngIf="!ctx.isTauri()" class="na">native-only — not available in web</p>
    <pre *ngIf="ctx.isTauri() && status()" class="block-value">{{ status() | json }}</pre>
    <p *ngIf="ctx.isTauri() && error()" class="na">{{ error() }}</p>
  `,
  styles: [`.na { opacity: 0.7; } .block-value { font-family: monospace; font-size: 0.8rem; }`],
})
export class NativeLensComponent implements OnInit {
  readonly ctx = inject(DebugContextService);
  readonly status = signal<unknown>(null);
  readonly error = signal<string | null>(null);

  async ngOnInit(): Promise<void> {
    if (!this.ctx.isTauri()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      this.status.set(await invoke('doorway_status'));
    } catch (e: unknown) {
      this.error.set(`IPC failed: ${String(e)}`);
    }
  }
}
```

> Confirm `@tauri-apps/api` is a dependency of elohim-app (`grep '@tauri-apps/api' app/elohim-app/package.json`); if absent, this lens belongs in the steward build, not elohim-app — leave it out and note the constraint. Confirm the exact IPC command name in `steward/device/src-tauri/src/lib.rs` (`doorway_status`) before shipping.

- [ ] **Step 2: Register + build + commit** (mirror prior lens-registration + `pnpm run build` + commit steps).

---

## Final verification (run before handing back)

- [ ] **Codegen fresh:** `pnpm run schema:codegen:ts -- --verify` → exit 0.
- [ ] **Unit tests:** `pnpm exec vitest run --config vite.config.ts debug` → all debug-surface specs pass.
- [ ] **Build:** `pnpm run build` → succeeds.
- [ ] **Rust contract:** the Task 12 `cargo test` command → both stability tests pass.
- [ ] **Lint/format (touched tree):** `pnpm run lint` + `pnpm run format:check` in `app/elohim-app` → clean (per the sprint-DoD-includes-pre-push-gates lesson — run the touched tree's gates, not just unit tests).
- [ ] **Route gate behavior:** `/debug` resolves by URL in dev; nav entry shows (dev-mode on). Confirm prod-build env has `showDebug: false` and the nav entry is hidden unless the sticky flag is set.
- [ ] **Honest states:** Stability tab shows projector REAL + autoPreset/admission/upstreams PENDING in web; (if a tauri/direct stack is available) projector REAL + peers/render/warmup/conductor N/A.
- [ ] **Security note surfaced:** confirm the spec's operator-owned ingress note is unchanged; do NOT add endpoint auth here.

---

## Self-review checklist (plan author)

- **Spec coverage:** home=shell `/debug` (Task 7) ✓; reuse via CONNECTION_STRATEGY/DebugContextService (Task 4) ✓; lens registry + honest degradation (Tasks 7-11) ✓; native level range (Task 9) ✓; read-only + client-local toggles ✓; backend untouched except dev-proxy/codegen/contract-test (Tasks 1,2,12) ✓; security note carried in spec ✓; doorway-app out of scope ✓; tauri 6-level adjust deferred (Task 9 display-only) ✓.
- **Type consistency:** `StabilityStatusView` (generated, Task 1) consumed in Tasks 3 & 8; `BlockState`/`DebugLens` (Task 7) consumed in Task 8; `LogLevel` from `LoggerService` (Task 9); Rust sub-view struct names match `self_healing.rs` (Task 12).
- **No placeholders:** every code step shows complete code; commands have expected output. The one soft spot (Task 12 `load_schema` access; Task 13 `@tauri-apps/api` presence) carries an explicit verify-and-adapt instruction rather than a vague TODO.
