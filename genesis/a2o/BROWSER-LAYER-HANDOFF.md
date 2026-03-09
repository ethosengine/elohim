# Browser Layer Handoff: Doorway Agency Phase

Handoff document for implementing Playwright browser tests for the 27 doorway
agency scenarios. Best done on a machine with a display (native dev, not Che).

## Current State

The API layer is complete — 27 scenarios across 7 features verify endpoint
contracts via `undici` HTTP. The Playwright infrastructure exists and is mature:

- `PlaywrightDevice` wraps Browser+Context+Page with console/error/network capture
- `BasePage` provides `testId(id)` for `data-testid` queries
- `ThresholdLoginPage` and `AppShellPage` page objects exist
- 120+ `data-testid` selectors already mapped in `src/framework/pages/selectors.ts`
- Doorway-app templates already have testids on login, register, dashboard tabs,
  user management, toolbar, browser, federation, pipeline, and account
- Cucumber `browser` profile: `E2E_DEVICE_MODE=playwright`

What's missing: **page objects for doorway-app** and **browser step definitions**
that drive the UI instead of calling HTTP endpoints.

## Page Objects Needed

All go in `genesis/a2o/src/framework/pages/`. Selectors already exist in
`selectors.ts` — just wire them into page object methods.

### DoorwayDashboardPage
```
selectors: DOORWAY_DASHBOARD (dashboard-tab-{overview,nodes,users,...})
methods:
  switchTab(name)
  activeTab(): string
  waitForReady()
```

### UsersTabPage
```
selectors: DOORWAY_DASHBOARD users section (dashboard-users-search, etc.)
methods:
  searchUser(query)
  getUserRow(identifier): Locator
  openUserDetail(identifier)
  suspendUser(identifier)
  updateQuota(identifier, mb)
```

### UserDetailModalPage
```
selectors: user-detail-* (user-detail-permission, user-detail-toggle-status, etc.)
methods:
  getPermission(): string
  toggleStatus()
  close()
```

### NodesTabPage
```
selectors: DOORWAY_DASHBOARD nodes section (dashboard-nodes-filter, etc.)
methods:
  getNodeCount(): number
  getNodeRow(nodeId): Locator
  filterByStatus(status)
```

### FederationTabPage
```
selectors: FEDERATION (federation-refresh, federation-peer-url, federation-add-peer)
methods:
  addPeer(url)
  removePeer(url)
  refresh()
  getPeerList(): string[]
```

### PipelineTabPage
```
selectors: PIPELINE (pipeline-retry)
methods:
  getFunnelStage(name): number
  waitForReady()
```

### DoorwayLandingPage
```
selectors: LANDING (landing-sign-in, landing-create-account, landing-dashboard)
methods:
  clickSignIn()
  clickCreateAccount()
  clickDashboard()
```

### DoorwayToolbarPage
```
selectors: toolbar-{profile-bubble, logout, backdrop}
methods:
  logout()
  isAuthenticated(): boolean
```

## Which Scenarios Get Browser Upgrades

Not all 27 need Playwright. API-level assertions are fine for pure data
validation (permission checks, field mapping). Browser tests add value where
the *experience* matters.

### Must have (experience-critical)

| Feature | Scenario | Why |
|---------|----------|-----|
| session-handoff | Full handoff flow | Multi-tab: click Visit in elohim-app, land authenticated in doorway-app |
| session-handoff | Expired token redirects | Browser should show login page, not a blank screen |
| dashboard-health | Dashboard loads cleanly | The whole point — no console errors in a real browser |
| dashboard-health | Capabilities status | Disabled features show placeholders, not broken UI |
| dashboard-health | Missing orchestrator | "No orchestrator" message renders, not an error |
| operator-onboarding | Pipeline funnel | Visual funnel renders with real counts |

### Nice to have (confidence builders)

| Feature | Scenario | Why |
|---------|----------|-----|
| user-management | List/view/suspend | Admin UI works end-to-end through the dashboard |
| conductor-visibility | List conductors | Table renders with real pool data |
| operator-onboarding | Federation peers | Add/remove peers through the UI |

### Keep as API-only

| Feature | Scenario | Why |
|---------|----------|-----|
| session-handoff | Obtain/exchange/single-use tokens | Pure API contract |
| user-management | Non-admin forbidden | Permission check, no UI involved |
| conductor-visibility | Non-admin forbidden | Same |
| self-registration | All 3 | Backend behavior, no UI |
| auth-lifecycle | JWT identity plumbing | Token validation, no UI |

## Session Handoff: Multi-Tab Testing

This is the hardest scenario. The flow crosses two Angular apps:

1. Matthew is logged in on elohim-app (tab 1)
2. Clicks "Visit" on his doorway from imagodei profile
3. New tab opens doorway-app with `?session_token=xxx`
4. doorway-app exchanges token, shows dashboard

Playwright approach:
```typescript
// Tab 1: elohim-app
const elohimPage = await context.newPage();
await elohimPage.goto(elohimAppUrl);
// ... login, navigate to profile

// Click Visit — intercept popup
const [doorwayPage] = await Promise.all([
  context.waitForEvent('page'),  // captures new tab
  elohimPage.click('[data-testid="profile-doorway-visit"]'),
]);

// Tab 2: doorway-app
await doorwayPage.waitForURL(/\/dashboard/);
await expect(doorwayPage.locator('[data-testid="dashboard-tab-overview"]')).toBeVisible();
```

Both apps must be running or both must point to deployed alpha.

## Cleanup Strategy

### Ephemeral humans (created during tests)

Current state: `randomUUID()` in identifier, never cleaned up, accumulates.

Recommended approach — **scenario-scoped cleanup via After hook**:

```typescript
// In world.ts or common.steps.ts After hook
After(async function (this: E2EWorld) {
  for (const [name, human] of this.humans) {
    // Only clean up ephemeral humans (not fixtures like Matthew)
    if (human.credentials.identifier.startsWith('e2e-')) {
      try {
        // Admin device deletes the user via API
        await adminDevice.client.adminDeleteUser(human.humanId);
      } catch {
        // Best effort — don't fail the scenario over cleanup
      }
    }
  }
});
```

This requires a `DELETE /admin/users/{id}` endpoint (or reuse suspend + purge).

Alternatively, add a **run-scoped namespace** prefix and a bulk cleanup script:

```bash
# After test suite completes
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  npx tsx scripts/cleanup-e2e-users.ts --prefix "e2e-" --older-than 1h
```

### Federation peers (created during tests)

The operator-onboarding scenario adds then removes a peer in the same scenario.
Self-cleaning. No additional cleanup needed.

## Tauri Desktop Context

The Tauri context (steward app) has a different auth flow — identity comes from
the local Holochain conductor, not JWT. Browser tests for Tauri would need:

- `TauriDevice` class that wraps Playwright connecting to `tauri://localhost`
- Auth injection via IPC commands instead of localStorage
- Test against `steward/` app build

This is a separate effort from the doorway Playwright work. Recommend deferring
until the doorway browser layer is solid.

## Running the Tests

```bash
cd genesis/a2o

# API-only (works in Che, CI, anywhere)
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm test:auth

# Browser — headless (needs Chromium)
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm test:browser

# Browser — headed (see it run, useful for debugging)
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm test:browser:headed

# Browser — with trace recording
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm test:browser:trace

# View trace after
npx playwright show-trace reports/traces/session-handoff.zip
```

## File Map

```
genesis/a2o/
├── features/
│   ├── auth/
│   │   ├── session-handoff.feature      ← needs browser scenarios added
│   │   ├── user-management.feature      ← partial browser upgrade
│   │   └── operator-onboarding.feature  ← partial browser upgrade
│   ├── browser/
│   │   └── doorway-dashboard-health.feature  ← already @browser-only
│   └── deployment/
│       ├── conductor-visibility.feature  ← keep API-only
│       └── doorway-self-registration.feature  ← keep API-only
├── steps/
│   ├── ui/
│   │   └── doorway-dashboard.steps.ts   ← CREATE: browser steps
│   ├── session-handoff.steps.ts         ← extend with browser variants
│   └── ...
└── src/framework/
    └── pages/
        ├── selectors.ts                 ← selectors already defined
        ├── base.page.ts                 ← base class exists
        ├── threshold-login.page.ts      ← exists
        ├── app-shell.page.ts            ← exists
        ├── doorway-dashboard.page.ts    ← CREATE
        ├── users-tab.page.ts            ← CREATE
        ├── nodes-tab.page.ts            ← CREATE
        ├── federation-tab.page.ts       ← CREATE
        ├── pipeline-tab.page.ts         ← CREATE
        └── doorway-toolbar.page.ts      ← CREATE
```
