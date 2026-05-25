# Peer OAuth Portal — Design

**Status:** Design (pending implementation plan)
**Date:** 2026-05-25
**Sibling design:** `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` (elohim-core primitives this portal consumes — Session, Loader, page-chrome, EPR-link)
**Substrate predecessors (existing, NOT re-invented by this design):**
- `doorway/doorway-service/src/routes/auth_routes.rs` — full RFC-6749 OAuth surface (authorize, token, refresh, register, /auth/me, native-handoff, key-bundle export, stewardship migration)
- `doorway/doorway-service/src/db/schemas/oauth_session.rs` — 5-min single-use authorization codes + registered-client table
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/portal_host.rs` — Category A notarized PortalHost entry; substrate-attested "this imagodei authorized this doorway to render their auth portal" (accumulates during agency graduation)
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/recovery_v2.rs` — KeyRotation + RecoveryAuthority substrate (referenced but NOT exercised in MVP)
- Existing Angular: `LoginComponent`, `AuthCallbackComponent`, `AuthService`, `OAuthAuthProvider`, `PasswordAuthProvider`, `TauriAuthService`, `IdentityService`, `DoorwayRegistryService` — Angular components become thin Lit-element wrappers IN THIS SHIFT (no parallel UI implementations after MVP)

**P2P-design-gate:** Run inline (see Appendix B). No new DHT entry types; no new storage tables; no new doorway routes. This design is **purely the UI layer** over existing substrate.

**Operator of record (MVP):** Matthew (alpha.elohim.host + elohim.host doorway pair).

---

## 0. The picture in one paragraph

A peer-OAuth portal expressed as seven Lit primitives in `app/elohim-elements/elohim-imagodei/`, composed into a wizard-shaped shell that renders the same way regardless of consumption path (standalone EPR bundle, Angular pillar wrapper, or Tauri webview) and regardless of network transport (doorway-routed or direct-from-local). The portal supports two trust modes — `doorway-host` (the doorway runs your conductor; you're on the flywheel; you can be evicted) and `peer-conductor` (your conductor runs on your storage instance or device; doorway is at most transparent ingress) — and discriminates them via the chrome (trust-indicator + attestor-row) without ever leaning on a sovereign-key/crypto-bro framing. Security in BOTH modes comes from community attestation (qahal + intimate circle + global witnesses + your elohim agent); cryptographic hardening (Shamir, hardware tokens, 2FA) is optional for high-risk individuals. The Angular `LoginComponent` and `AuthCallbackComponent` are converted to thin wrappers around the new Lit elements in the same shift, so there is exactly ONE visible portal UI at completion. Recovery launcher, account migration, and PortalHost authorize UI are explicitly deferred; the substrate gates that exist (PortalHost-based pre-registration via agency graduation) are surfaced honestly in error states without requiring a new authoring UI.

---

## 1. Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  CONSUMERS                                                       │
│                                                                  │
│  (a) Standalone EPR Bundle               (b) Imagodei pillar     │
│      app/imagodei-portal/                    in elohim-app /     │
│      (new Angular project; minimal)          eventually own      │
│      Projected by doorway at /auth/portal    bundle. Angular     │
│      Tauri serves locally too                wrappers consume    │
│                                              the same Lit        │
│                                              elements.           │
└──────────────────────────────────────────────────────────────────┘
                       │ both consume the same primitives │
                       ▼                                  ▼
┌──────────────────────────────────────────────────────────────────┐
│  NEW LIT ELEMENTS — app/elohim-elements/elohim-imagodei/src/     │
│                                                                  │
│  <elohim-imagodei-portal-shell>     ← chrome + trust-mode ctx    │
│   ├── <elohim-imagodei-trust-indicator>                          │
│   ├── <elohim-imagodei-attestor-row>   ← qahal/circle witnesses  │
│   └── slot: one of                                               │
│       │   <elohim-imagodei-federated-resolver>  (matthew@host)   │
│       │   <elohim-imagodei-login-card>          (credentials)    │
│       │   <elohim-imagodei-consent-card>        (RP claim grant) │
│       │   <elohim-imagodei-oauth-callback>      (redirect step)  │
│                                                                  │
│  trustMode: 'doorway-host' | 'peer-conductor'                    │
│  Authority indicator: which doorway / which device               │
└──────────────────────────────────────────────────────────────────┘
                       │ consume Session + Loader │
                       ▼                          ▼
┌──────────────────────────────────────────────────────────────────┐
│  ELOHIM-CORE (from EPR decomposition work)                       │
│  - Session primitive (current-user, capabilities, reach)         │
│  - Loader (CID resolution; pulls in mention-base etc. if needed) │
│  - elohim-page-chrome (standalone EPR mounts inside one of these)│
│  - elohim-default-omnibar (anonymous mode shows the portal CTA)  │
└──────────────────────────────────────────────────────────────────┘
                       │ talk HTTP to                              │
                       ▼                                           ▼
┌──────────────────────────────────────────────────────────────────┐
│  BACKEND SUBSTRATE — already exists                              │
│                                                                  │
│  doorway/doorway-service/src/                                    │
│    routes/auth_routes.rs        ← OAuth + login + handoff (full) │
│    db/schemas/oauth_session.rs  ← 5-min single-use codes         │
│  elohim/holochain/dna/imagodei/zomes/imagodei_integrity/         │
│    portal_host.rs               ← Category A: doorway authorized │
│                                    by this imagodei (substrate-  │
│                                    attested via graduation)      │
│    recovery_v2.rs               ← KeyRotation + Recovery sub.    │
│                                    (NOT exercised by MVP portal) │
└──────────────────────────────────────────────────────────────────┘
                       │ ALSO consumes                             │
                       ▼                                           ▼
┌──────────────────────────────────────────────────────────────────┐
│  EXISTING ANGULAR SERVICES — stay, get wrapped                   │
│                                                                  │
│  AuthService           ← Lit elements call via Angular DI when   │
│  OAuthAuthProvider       running inside the elohim-app shell     │
│  PasswordAuthProvider    (the elohim-imagodei-portal-shell       │
│  TauriAuthService        receives them as injected callbacks).   │
│  IdentityService         For the standalone EPR + Tauri-direct   │
│  DoorwayRegistryService  paths, the elements use plain fetch()   │
│                          against the doorway HTTP API.           │
└──────────────────────────────────────────────────────────────────┘
```

### 1.1 Key architectural properties

- **Lit elements are framework-agnostic.** They render inside Angular templates, standalone HTML, and Tauri webviews equivalently. Hosting environments provide the runtime callbacks: Angular DI for elohim-app; plain `fetch()` for the standalone EPR; Tauri IPC + plain fetch for Tauri-direct.

- **trustMode is discovered, not configured.** On mount, `<elohim-imagodei-portal-shell>` queries the configured `authorityEndpoint` (default `/auth/me`). The response declares which mode is in force and who the authority is. Consumers don't pass `trustMode` as configuration — it emerges from the actual runtime situation. This is what makes the same bundle render identically in doorway-routed and Tauri-direct paths.

- **The standalone EPR (a) deploys via the EPR-decomposition substrate.** A new Angular project at `app/imagodei-portal/` produces a bundle with `<base href="/auth/portal/">` (mirrors lamad-spa). A `project-epr` REA Commitment registers it at `urlPath: "/auth/portal"` on each doorway. Tauri loads the same bundle locally (no doorway in path).

- **The pillar integration (b)** preserves the existing Angular routes (`/identity/login`, `/identity/callback`) but the route components become thin wrappers — `LoginComponent`'s template collapses to `<elohim-imagodei-portal-shell>` with a slotted login-card, plus the existing service-layer wiring as injected callbacks.

- **No third portal at completion.** Doorway's HTTP API (`auth_routes.rs`) is substrate, not a portal — it stays untouched. The Angular UI components become wrappers around the canonical Lit elements. The Lit elements are the SOLE visible UI for both deployment paths.

---

## 2. Components

Seven new Lit elements in `app/elohim-elements/elohim-imagodei/src/`. All theme-agnostic (CSS custom property surface only). All carry `@capability*` JSDoc contracts per the elohim-imagodei package conventions. **All authored by component-architect.**

### 2.1 `<elohim-imagodei-portal-shell>`

The wrapper that all other portal primitives sit inside. Discovers `trustMode` on mount; propagates it (via slotchange property assignment) to slotted children. Renders persistent chrome (trust-indicator + attestor-row); slots the active step into the primary region.

| Surface | Shape |
|---|---|
| Props | `authorityEndpoint: string` (default `/auth/me`); `step: 'resolve' \| 'login' \| 'consent' \| 'callback'`; `flywheelHint?: boolean` |
| Events | `step-change`, `authority-resolved` (`{ trustMode, authority }`) |
| Slots | `header`, `primary`, `footer`, `error-region`, `auth-wall` |
| @cssprop | `--elohim-portal-bg`, `--elohim-portal-fg`, `--elohim-portal-panel-bg`, `--elohim-portal-grid-gap` |
| Depends on | Loader (elohim-core), Session (elohim-core), `<elohim-imagodei-trust-indicator>`, `<elohim-imagodei-attestor-row>` |

### 2.2 `<elohim-imagodei-trust-indicator>`

Small chip-shaped element showing where the conductor lives. Different glyph + copy per mode. Reads `trustMode` from parent shell; also usable standalone in an omnibar.

| Surface | Shape |
|---|---|
| Props | `trustMode: 'doorway-host' \| 'peer-conductor'`; `authorityLabel: string`; `flywheelHint?: boolean` |
| Events | `trust-indicator-tap` |
| Slots | none |
| @cssprop | `--elohim-trust-bg`, `--elohim-trust-fg`, `--elohim-trust-host-accent`, `--elohim-trust-peer-accent` |
| Depends on | none |

### 2.3 `<elohim-imagodei-attestor-row>`

Small avatar row showing your qahal / circle witnesses. Foregrounds the social-trust model; explicitly contradicts crypto-bro framing.

| Surface | Shape |
|---|---|
| Props | `attestors: AttestorRef[]` (`{eprRef, displayName, role}`); `maxVisible?: number` (default 5); `density?: 'compact' \| 'standard'` |
| Events | `attestor-tap` (`{ eprRef }`) |
| Slots | `empty` (defaults to "your community will witness you when you finish enrollment") |
| @cssprop | `--elohim-attestor-ring`, `--elohim-attestor-avatar-size`, `--elohim-attestor-gap` |
| Depends on | `<elohim-mention-base>` (elohim-core) |

### 2.4 `<elohim-imagodei-federated-resolver>`

User enters federated identifier (`matthew@alpha.elohim.host`); element resolves to a doorway endpoint. Emits `resolved` with the resolved URL; shell advances to login.

| Surface | Shape |
|---|---|
| Props | `placeholder?: string`; `rememberKey?: string` (defaults to existing `AUTH_IDENTIFIER_KEY`); `resolveIdentifier: (id: string) => Promise<ResolveOutcome>` |
| Events | `resolved` (`{ identifier, doorwayUrl }`); `resolve-error` (`{ identifier, reason }`) |
| Slots | `help-text` |
| @cssprop | `--elohim-input-border`, `--elohim-input-focus-ring`, `--elohim-input-error-fg` |
| Depends on | external resolver callback (Angular DI: `DoorwayRegistryService`; standalone: plain fetch wrapper) |

### 2.5 `<elohim-imagodei-login-card>`

Credentials surface. Renders OAuth provider buttons + password form. Inherits `trustMode` from shell so copy adapts (hosted vs conductor-local).

| Surface | Shape |
|---|---|
| Props | `oauthProviders: OAuthProviderRef[]`; `allowPassword: boolean` (default true); `remember: boolean`; `rememberedIdentifier?: string` |
| Events | `password-submit` (`{ identifier, password, remember }`); `oauth-start` (`{ providerId }`); `cancel` |
| Slots | `unlock-prompt` (alternative for Tauri local-key-unlock flow) |
| @cssprop | `--elohim-login-bg`, `--elohim-login-input-bg`, `--elohim-login-button-bg`, `--elohim-login-button-fg` |
| Depends on | none internal — emits events; consumer wires services |

### 2.6 `<elohim-imagodei-consent-card>`

Renders the RFC-6749 authorization-step consent surface. Shows requesting client + per-claim toggles + inherited trust chrome.

| Surface | Shape |
|---|---|
| Props | `requestingClient: { id, displayName, brandMark? }`; `requestedClaims: ClaimRef[]`; `requiredClaims: string[]` |
| Events | `approve` (`{ grantedClaims: string[] }`); `decline` (`{ reason?: 'user-rejected' \| 'partial-decline-blocked' }`) |
| Slots | `policy-link`, `claim-detail` |
| @cssprop | `--elohim-consent-rp-bg`, `--elohim-consent-claim-row-bg`, `--elohim-consent-approve-bg` |
| Depends on | `<elohim-epr-link>` (elohim-core) for RP brand mention rendering |

### 2.7 `<elohim-imagodei-oauth-callback>`

Mostly invisible — handles redirect-back from external OAuth provider. Renders skeleton during code-exchange; surfaces errors prominently.

| Surface | Shape |
|---|---|
| Props | `code?: string`; `state?: string`; `providerLabel?: string`; `exchangeCode: (code, state) => Promise<ExchangeOutcome>` |
| Events | `success` (`{ session }`); `error` (`{ reason, recoverable }`) |
| Slots | `error-detail`, `recovery-cta` |
| @cssprop | `--elohim-callback-spinner-color` |
| Depends on | `<elohim-skeleton>` (elohim-core); consumer-provided `exchangeCode` callback |

### 2.8 Cross-cutting: the `trustMode` context

The shell discovers `trustMode` on mount and propagates it to slotted children via DOM property assignment in a `slotchange` handler. Same pattern as `<elohim-page-chrome>` from the EPR decomposition design — no DI framework needed.

```ts
private onPrimarySlotChange(e: Event): void {
  const slot = e.target as HTMLSlotElement;
  for (const node of slot.assignedElements()) {
    (node as any).trustMode = this._trustMode;
    (node as any).authority = this._authority;
  }
}
```

Sub-primitives that consume `trustMode` (login-card, consent-card, trust-indicator) accept it via `@property()` and re-render on change. Primitives that don't (federated-resolver, oauth-callback) ignore the assignment.

### 2.9 Ownership

| Primitive | Author | Library A story | Library B story |
|---|---|---|---|
| portal-shell | component-architect | yes | graphos-designer |
| trust-indicator | component-architect | yes | graphos-designer |
| attestor-row | component-architect | yes | graphos-designer |
| federated-resolver | component-architect | yes | graphos-designer |
| login-card | component-architect | yes | graphos-designer |
| consent-card | component-architect | yes | graphos-designer |
| oauth-callback | component-architect | yes | graphos-designer |

---

## 3. Data Flow

### 3.1 Scenario — Mode A first-time login (doorway-host)

```
Browser → alpha.elohim.host doorway (epr_router from EPR decomp)
   ├─ projection at urlPath="/auth/portal" → serves the imagodei-portal bundle
   └─ bundle index.html mounts <elohim-imagodei-portal-shell step="resolve">

<elohim-imagodei-portal-shell> on mount:
   ├─ Loader.fetch authorityEndpoint = /auth/me
   ├─ Response: { authenticated: false } → no current session
   ├─ Default trustMode held in 'unknown' until step='login' resolves
   └─ Renders header chrome (trust-indicator placeholder + empty attestor-row)

<elohim-imagodei-federated-resolver> in primary slot:
   ├─ User types "matthew@alpha.elohim.host"
   ├─ Element invokes resolver callback
   │   (Angular DI: DoorwayRegistryService; standalone: fetch
   │   /.well-known/elohim-doorway from gateway host)
   ├─ Returns { doorwayUrl: 'https://alpha.elohim.host' }
   └─ emit('resolved', { identifier, doorwayUrl })

shell handles 'resolved' →
   ├─ stores identifier + doorwayUrl
   ├─ advances step="login"
   └─ slots in <elohim-imagodei-login-card>

<elohim-imagodei-login-card>:
   ├─ Renders password field + remember toggle + OAuth provider buttons
   ├─ User submits password
   └─ emit('password-submit', { identifier, password, remember })

shell handles 'password-submit' →
   ├─ Angular path: AuthService.loginWithPassword(...)
   ├─ Standalone path: fetch POST /auth/login {identifier, password}
   ├─ Doorway side (auth_routes.rs):
   │    ├─ verifies credentials
   │    ├─ issues JWT (RFC-6749 access_token)
   │    ├─ Set-Cookie: elohim_session=<JSON: humanId, capabilities, reach>
   │    │  + httpOnly auth cookie elohim_auth=<JWT>
   │    └─ returns { profile, redirect: returnTo }
   ├─ shell's onSuccess:
   │    ├─ session.refreshFromCookies() (elohim-core Session primitive)
   │    ├─ resolves trustMode='doorway-host', authority='alpha.elohim.host'
   │    ├─ flywheelHint=true (first-time doorway-host login)
   │    ├─ renders <elohim-imagodei-trust-indicator> with the host chrome
   │    └─ optionally renders <elohim-imagodei-attestor-row> if profile
   │       carries initial qahal/circle memberships
   └─ browser navigates to returnTo (or shell stays on /auth/portal/welcome)
```

### 3.2 Scenario — Mode B login (peer-conductor)

Two transports for the same ceremony:

**Transport α: browser → doorway-routed → peer-conductor**
```
Same projection at /auth/portal serves the same bundle.
On mount, /auth/me hits doorway, doorway proxies to user's registered
conductor (via route registry from EPR decomp).
Conductor responds with { authenticated: false, authority: 'peer-conductor',
                          conductorEndpoint: '<user's conductor peer-id/url>' }
shell trustMode='peer-conductor', authority='your conductor at <peer-id>'

User enters federated identifier → resolver knows this peer is conductor-mode
shell advances step="login" with <elohim-imagodei-login-card>
User submits password → doorway proxies to conductor's local auth endpoint
Conductor verifies (using LOCAL stored credential material), issues session
Doorway transparently forwards Set-Cookie back to browser
Session primitive picks it up; flywheelHint=false (already graduated)
trust-indicator chrome reads "Your conductor at <peer-id>;
                              alpha.elohim.host is helping with ingress"
```

**Transport β: Tauri → localhost conductor (no doorway)**
```
Tauri webview loads the same bundle from local storage (Tauri sidecar
serves it). On mount, /auth/me hits localhost conductor directly.
Conductor responds same as Transport α.
shell renders the SAME UI as Transport α with one chrome difference:
trust-indicator authorityLabel reads "Your conductor on this device" —
                  no doorway-ingress addendum needed.
Otherwise identical. Same primitives. Same ceremony.
```

**Invariant:** the bundle does not know which transport is in play. It only knows the authority the `/auth/me` lookup returns. Doorway-as-ingress is indistinguishable from no-doorway from the bundle's perspective. This is what makes the standalone-EPR + Tauri share one implementation.

### 3.3 Scenario — RP consent (external app requests claims)

```
External app graphos-designer.elohim.host redirects user to
https://alpha.elohim.host/auth/portal?client_id=graphos-designer&claims=...
                                       &redirect_uri=...&state=...

Browser → doorway → epr_router → /auth/portal bundle
Bundle reads URL params, detects OAuth authorization-request shape
   ├─ Bundle posts /auth/authorize/prepare with the params
   ├─ Doorway (auth_routes.rs) validates client_id + redirect_uri
   │   against oauth_session.rs's registered-clients table
   ├─ Returns { requestingClient: {...}, requestedClaims: [...],
   │            requiredClaims: [...], policy: {...} }
   └─ Bundle mounts <elohim-imagodei-portal-shell step="consent">

If user is already logged in (Session primitive returns currentUser):
   └─ shell slots in <elohim-imagodei-consent-card>

If user is NOT logged in:
   └─ shell first runs the resolve→login flow (3.1 or 3.2),
      then transitions to consent. Redirect chain preserved in state.

<elohim-imagodei-consent-card>:
   ├─ Renders RP brand strip (graphos-designer's display + reach badge)
   ├─ Per-claim row with toggle (required claims locked on,
   │   optional toggleable)
   ├─ Inherited trust-indicator from shell — user can see "you are
   │   signing this consent as <doorway-host or peer-conductor>"
   ├─ User clicks Approve
   └─ emit('approve', { grantedClaims })

shell handles 'approve' →
   ├─ POST /auth/authorize/grant { grantedClaims, state }
   ├─ Doorway:
   │    ├─ writes OAuth authorization code to oauth_session.rs
   │    │   (5-min single-use)
   │    └─ returns { redirect: '<RP redirect_uri>?code=...&state=...' }
   └─ Browser → RP redirect_uri
       └─ RP exchanges code at POST /auth/token → JWT with granted claims
```

**Decline path:** POST /auth/authorize/decline { state } → doorway 302 to RP redirect_uri with `error=access_denied&state=...` (RFC-6749 conformant).

### 3.4 Cross-scenario: what the shell does for free

- **Authority refresh:** polls `/auth/me` on `visibilitychange` (mirrors `<elohim-default-omnibar>`'s pattern from EPR decomp) so cross-tab logouts surface within a tab switch.
- **Recovery launch hooks** (deferred for primitives, but the LINK surfaces are present): every error region carries a `/identity/recovery` link.
- **Error envelope:** any sub-primitive can emit `portal-error` which the shell renders into the `error-region` slot, with the trust-indicator chrome preserved.
- **Step transitions are explicit, not implicit:** the shell never advances on its own. Sub-primitives emit success → consumer (Angular wrapper, standalone EPR controller) sets `shell.step` explicitly. Keeps the wizard predictable + externally testable.

---

## 4. Edge Behavior — Designed Boundaries

Same principle as the EPR decomposition spec: errors are first-class designed experiences. The portal never shows a bare 500. Every failure foregrounds what the user can do next, with the trust-indicator chrome preserved so they know which mode they were in.

### 4.1 Auth failures

| Case | Surface | Recovery affordance |
|---|---|---|
| Wrong password | `<elohim-imagodei-login-card>` inline error; field shake (reduced-motion compliant); copy "credentials didn't match. Try again, or use OAuth, or ask for help from your circle." | `/identity/recovery` link |
| Password rotation required | Same card; "your sign-in needs a refresh. We'll walk you through it." | Auto-redirects to rotation flow |
| OAuth provider failure | `<elohim-imagodei-oauth-callback>` error state; provider's response in `error-detail` slot | "Try a different sign-in method" → back to login-card |
| Account locked (rate-limit) | Full-card error in `error-region`. Trust-indicator preserved. Copy: "Your account is paused. Usually rate-limiting — wait 15 min, or ask a witness in your circle to vouch right now." | "Vouch from circle" → recovery launcher (deferred for MVP) |
| Doorway-side eviction (Mode A) | Full-card; explicit copy: "alpha.elohim.host has stopped hosting your account. Reasons: terms-of-stewardship violation, compute-budget exceeded, doorway operator decision. You can recover and graduate to your own conductor — your community + recovery substrate are still intact." | "Recover + migrate" → graduation flow (deferred); "Talk to operator" → operator contact card |

### 4.2 Network failures

```
Loader fetch chain returns 'unresolved' for /auth/me:
   ├─ Browser-via-doorway: doorway up, can't reach storage. Shell renders:
   │     trust-indicator dimmed; "your doorway is having trouble.
   │     We can't reach your account right now. This is on our side,
   │     not yours."  Retry button (debounced 5s). Status link.
   │
   ├─ Tauri-direct: localhost conductor not started. Shell renders:
   │     trust-indicator reads "Your conductor on this device — not
   │     running". "Your conductor hasn't started yet. The desktop app
   │     starts it automatically; if you got here too quickly, give it
   │     a few seconds and try again."
   │
   └─ Browser-via-doorway-routed-to-peer-conductor: doorway up, target
      conductor offline. Shell renders:
        trust-indicator reads "Your conductor — offline". "Your conductor
        isn't reachable right now. Your account is still safe — your
        community still has your back. You can sign in via your other
        devices or ask a witness in your circle to help."
        "Sign in elsewhere" → list registered PortalHosts.
        "Recovery" → recovery launcher (deferred).
```

### 4.3 OAuth protocol failures (RFC-6749 §4.1.2.1)

| OAuth error | Surface |
|---|---|
| `invalid_request` (malformed params) | "The app that sent you here didn't ask correctly. We've reported this to its developer — try again later, or contact the app." |
| `unauthorized_client` (RP not registered) | "alpha.elohim.host doesn't recognize this app. If you're the developer, register it at `/admin/oauth-clients`." |
| `access_denied` (user rejected) | Brief acknowledgement: "You declined. Returning you to <RP>." |
| `unsupported_response_type` | Same as `invalid_request`. |
| `invalid_scope` | "This app asked for something you can't grant. Contact the app's developer if this is wrong." |
| `server_error` | Generic full-card with retry + status link. |

All error redirects back to RP carry `state` per RFC-6749 §10.12.

### 4.4 Trust-mode discovery edge cases

| Case | Behavior |
|---|---|
| `/auth/me` 503 (doorway up, conductor down) | Held trust-mode in `'unknown'`. Trust-indicator shows shimmer. After 10s, fallback to 4.2 doorway-unreachable. |
| Conflicting info (cookie says doorway-host but conductor claims peer-conductor) | Conductor wins; doorway cookie discarded. Trust-indicator briefly reads "(reconciled)" to acknowledge swap. Substrate-coherence event. |
| Anonymous user (no session) | trust-mode defaults to `'doorway-host'` for login flow (user is being hosted during auth even if they'll graduate later). |

### 4.5 PortalHost-substrate failures

Even though PortalHost authorize UI is deferred, the SUBSTRATE gates auth — agency-graduation pre-registration determines which doorways an imagodei can sign in through.

```
User enters matthew@beta.elohim.host on alpha.elohim.host's portal
Federated-resolver hits beta to look up matthew → 404 / "not registered here"

shell receives 'resolve-error' reason='not-portal-host-for-this-imagodei':
   ├─ Error region rendered (trust-indicator hidden — no authority yet)
   ├─ Copy: "matthew@beta.elohim.host hasn't authorized alpha.elohim.host
   │         to render their sign-in. Try signing in at:
   │           - beta.elohim.host (your registered portal)
   │           - <list other portals you've authorized, if any>"
   ├─ List comes from best-effort lookup at user's primary doorway
   └─ "If you've lost access to all your portals → start recovery"
       (deferred; for MVP this is a static link)
```

### 4.6 Recovery hand-off (deferred but entry points work)

Every error surface that mentions recovery links to `/identity/recovery` (existing Angular route — Lit version is fast-follow). MVP doesn't ship recovery primitives as Lit elements, but the portal-shell's `error-region` slot accepts the link rendered consistently across all error surfaces.

### 4.7 Defaults / fallback shapes

When any child primitive throws an uncaught render exception, `<elohim-imagodei-portal-shell>` wraps the slotted region with an error boundary rendering:

```html
<div role="alert" part="boundary-fallback">
  <elohim-imagodei-trust-indicator ...></elohim-imagodei-trust-indicator>
  <p>Something went sideways during sign-in. The trust-indicator above
     shows where you were. Try refresh, or sign in from a different device.</p>
  <button @click="refresh">Refresh</button>
  <a href="/identity/recovery">Recover account</a>
</div>
```

Lit has no native error-boundary API; shell wraps slotted children in `try/catch` around first render + listens for `errorEvent` on the slot container. Pragmatic enough for MVP.

---

## 5. Testing Surfaces

### 5.1 Unit tests — per Lit primitive

`<element>.spec.ts` in `app/elohim-elements/elohim-imagodei/src/`. Pattern: web-test-runner + Chai, per elohim-core conventions (B4-B8 in the EPR decomposition plan).

| Primitive | Focus | Tests est. |
|---|---|---|
| portal-shell | trustMode discovery (mocked); step transitions; slot context propagation; error boundary; reduced-motion | ~10 |
| trust-indicator | chrome per trustMode; flywheelHint; forced-colors; click event | ~6 |
| attestor-row | avatars + overflow `+N more`; empty slot fallback; click with eprRef; RTL | ~7 |
| federated-resolver | identifier parse; `resolved` event; `resolve-error`; localStorage round-trip; keyboard submit | ~7 |
| login-card | password submit event; OAuth provider event; remember toggle; tab order; aria-invalid | ~9 |
| consent-card | RP brand + claim list; required-locked; toggle disclosure; approve/decline events | ~8 |
| oauth-callback | skeleton during exchange; success/error events; error-detail slot; recovery-cta slot | ~6 |

Plus axe-core a11y scans on each in default + custom-theme states.

### 5.2 Library A default stories (component-architect)

Per `app/elohim-library/CLAUDE.md`, every primitive ships with `<element>.default.stories.ts` containing at minimum:
- `Unstyled (blank-slate proof)` wrapped in `style="all: initial;"`
- `CustomTheme (override surface proof)` with a deliberately non-Elohim binding
- Per-capability state stories

Required state stories beyond Unstyled + CustomTheme:

| Primitive | Stories |
|---|---|
| portal-shell | EmptyShell, WithLoginCard, WithConsentCard, ErrorBoundary |
| trust-indicator | DoorwayHost, PeerConductor, WithFlywheelHint, DisabledOffline |
| attestor-row | OneAttestor, FivePlusOverflow, EmptyState, RTLLayout |
| federated-resolver | Default, WithError, RememberedIdentifier, WithHelpSlot |
| login-card | PasswordOnly, OAuthOnly, Both, WithUnlockPromptSlot, WithError |
| consent-card | OneRequiredClaim, RequiredPlusOptional, WithPolicyLink, WithBrandMark |
| oauth-callback | Exchanging, Success, Error, WithRecoveryCta |

### 5.3 Library B designed stories (graphos-designer)

Pattern stories composing primitives into recognizable portal scenes. Brand tokens bind via story decorators only; primitives never modified.

| Story | Composes |
|---|---|
| `ModeA_FirstTimeLogin.designed.stories.ts` | shell + trust-indicator (doorway-host) + attestor-row (empty) + federated-resolver → login-card |
| `ModeB_PeerConductorLogin.designed.stories.ts` | shell + trust-indicator (peer-conductor) + attestor-row (populated) + login-card |
| `ModeB_TauriDirect.designed.stories.ts` | shell + trust-indicator (peer-conductor, no doorway label) + login-card |
| `ConsentCardThreeClaims.designed.stories.ts` | shell + trust-indicator + consent-card with 3 claims (1 required, 2 optional) |
| `EvictedAccount.designed.stories.ts` | shell + trust-indicator + error region (the 4.1 doorway-eviction surface) |
| `PortalHostNotAuthorized.designed.stories.ts` | shell + error region (4.5 substrate-gate surface) |
| `NetworkOffline.designed.stories.ts` | shell + trust-indicator dimmed + error region (4.2) |

Realistic protocol vocabulary throughout (matthew@alpha.elohim.host, aleph-household qahal members, "fair-exchange" concept as returnTo).

### 5.4 Angular wrapper integration tests (angular-architect)

| Test | Path |
|---|---|
| LoginComponent renders `<elohim-imagodei-portal-shell>` and wires AuthService callbacks | `app/elohim-app/src/app/imagodei/components/login/login.component.spec.ts` (existing — rewrite test bodies) |
| AuthCallbackComponent renders `<elohim-imagodei-oauth-callback>` and wires the exchange callback | `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.spec.ts` (existing — rewrite) |
| Federated identifier flow works through the new resolver primitive | Cypress E2E (new or existing) |

### 5.5 Standalone EPR bundle integration tests

| Test | Path |
|---|---|
| Bundle index.html has `<base href="/auth/portal/">` | `app/imagodei-portal/test/base-href.spec.ts` |
| `main.ts` registers all elohim-imagodei + elohim-core elements before app mount | `app/imagodei-portal/src/main.spec.ts` |
| Standalone HTTP fallback resolver returns correct doorwayUrl | `app/imagodei-portal/src/app/services/standalone-resolver.spec.ts` |

### 5.6 a2o scenarios (executable specifications)

Three new feature files in `genesis/a2o/features/peer-oauth-portal/`:

#### `hosted-login.feature`

```gherkin
Feature: Mode A — Doorway-hosted login
  As a new visitor to alpha.elohim.host, I sign in via the federated portal
  and the doorway hosts my conductor while I settle in.

  Scenario: First-time sign-in surfaces the flywheel chrome
    Given the alpha.elohim.host doorway has a projection for the peer-oauth-portal at "/auth/portal"
    And matthew is a pre-registered imagodei on alpha.elohim.host with password "shibboleth"
    When matthew opens "https://alpha.elohim.host/auth/portal?returnTo=/lamad"
    And types "matthew@alpha.elohim.host" into the federated-resolver
    Then the portal advances to the login-card step
    And the trust-indicator reads "Hosted via alpha.elohim.host" with the flywheel hint visible

    When matthew submits the password "shibboleth"
    Then the doorway sets the elohim_session cookie
    And the trust-indicator updates to show matthew's humanId
    And the browser navigates to "/lamad"

  Scenario: Wrong password preserves trust-indicator chrome
    Given matthew is on the login-card step at alpha.elohim.host
    When matthew submits an incorrect password
    Then the login-card shows the inline credentials error
    And the trust-indicator remains visible at the top of the shell
    And an "ask for help" link points to /identity/recovery
```

#### `peer-conductor-login.feature`

```gherkin
Feature: Mode B — Peer-conductor login
  My conductor on my own storage instance is the auth authority.
  Doorway is at most a transparent ingress, or absent entirely (Tauri).

  Scenario: Sign-in via doorway-routed peer-conductor
    Given matthew has graduated to running his own conductor
    And alpha.elohim.host is configured to route auth requests for matthew to his conductor
    When matthew opens "https://alpha.elohim.host/auth/portal"
    And the portal queries /auth/me
    Then the response reports trustMode "peer-conductor" and authority "matthew's conductor"
    And the trust-indicator reads "Your conductor — alpha.elohim.host is helping with ingress"

  Scenario: Sign-in via Tauri direct (no doorway)
    Given matthew has the Tauri app installed with his conductor running locally
    When matthew opens the portal inside the Tauri webview
    And the portal queries /auth/me against localhost:8090
    Then the trust-indicator reads "Your conductor on this device"
    And no doorway-ingress label appears
```

#### `rp-consent.feature`

```gherkin
Feature: RP consent — external app requests user claims
  Per RFC-6749, when an external relying party redirects the user to the
  authorization endpoint, the portal renders a consent surface that names
  the requesting app and lists the claims it wants.

  Scenario: User approves a per-claim consent
    Given matthew is signed in to alpha.elohim.host (Mode A)
    And graphos-designer.elohim.host is a registered OAuth client
    When matthew is redirected to "/auth/portal?client_id=graphos-designer&claims=imagodei.displayName,qahal.standing&redirect_uri=...&state=abc"
    Then the consent-card renders with graphos-designer as the requesting client
    And both claims are listed with toggles
    And "imagodei.displayName" is required (locked on)

    When matthew approves
    Then a 5-min single-use OAuth code is issued
    And matthew is redirected to graphos-designer's redirect_uri with code + state preserved

  Scenario: User declines consent
    Given matthew is on the consent-card for graphos-designer
    When matthew clicks Decline
    Then matthew is redirected to graphos-designer's redirect_uri with error=access_denied + state preserved
```

### 5.7 Manual dogfood criteria (operator signoff)

1. `alpha.elohim.host/auth/portal` loads the standalone EPR bundle with `<base href="/auth/portal/">`.
2. From the bundle: federated-resolver → login-card → success cookie set → navigate to returnTo works for the operator's pre-registered account.
3. The Angular `/identity/login` route renders identically to the standalone EPR (same visible result).
4. Trust-indicator chrome is visibly different (but not alarming) between Mode A and Mode B sign-ins. Both modes show the attestor row when populated.
5. Wrong password → inline error with trust-indicator preserved.
6. RP consent flow: any test RP redirected through `/auth/portal?client_id=...&claims=...` renders the consent card; approve → token exchange works; decline → RP gets `access_denied`.
7. Tauri webview loading the same bundle from local storage renders the SAME UI as the doorway-routed peer-conductor path. The trust-indicator differs only in the authority label.
8. **There is NO REMAINING legacy LoginComponent** — all `/identity/login` renders are the Lit-element-based path. (The cleanup-immediately criterion.)

---

## 6. MVP Scope

### 6.1 What ships

| Layer | Item |
|---|---|
| Primitives | 7 Lit elements in `app/elohim-elements/elohim-imagodei/src/` |
| Stories | Library A default (per §5.2) + Library B designed (per §5.3) |
| Standalone EPR | `app/imagodei-portal/` Angular project with `<base href="/auth/portal/">`; project-epr commitment seed creates a projection at `urlPath: "/auth/portal"` on each doorway |
| Angular wrappers | `LoginComponent` + `AuthCallbackComponent` rewritten as thin wrappers around the Lit elements; service-layer wiring preserved |
| a2o | 3 feature files: hosted-login, peer-conductor-login, rp-consent |
| Dogfood | `/auth/portal` works on alpha.elohim.host + Angular `/identity/login` renders identically + Tauri path matches |

### 6.2 What's deferred (with rationale)

| Item | Rationale | Likely shift |
|---|---|---|
| Recovery launcher (as Lit primitives) | Heavy enough for its own brainstorm + spec; the substrate (`recovery_v2.rs`) deserves first-class UI design attention | Fast-follow shift |
| Account migration (hosted → native) as Lit flow | Substrate exists in `auth_routes.rs` (native-handoff + key-bundle export); UI port is a focused shift | Fast-follow shift |
| PortalHost authorize UI | Substrate handles registration via agency-graduation; UI for managing portal-host grants is a steward self-service surface — separate scope | When stewards need it |
| Peer-direct trust ceremonies (no doorway involvement) | Out of MVP scope per operator direction | Later |
| Cross-bundle element loading for portal primitives | Element-registry pattern from EPR decomposition exists; portal primitives are imported at build-time in MVP | Same time as second pillar splits |

---

## 7. Definition of Done

MVP complete when:

- All 7 Lit primitives ship with passing unit tests (web-test-runner + Chai).
- Library A default stories (7 primitives × Unstyled + CustomTheme + state cells per §5.2) all render in Storybook.
- Library B designed stories (7 named compositions per §5.3) all render with brand tokens bound via story decorators.
- Angular `LoginComponent` + `AuthCallbackComponent` are wrappers around the Lit elements (no parallel UI code remaining).
- Standalone EPR project (`app/imagodei-portal/`) builds; bundle has `<base href="/auth/portal/">`; the `project-epr` commitment seed creates a projection at `urlPath: "/auth/portal"` on each doorway.
- Three a2o feature files pass via Cypress + Cucumber in CI.
- Operator-of-record manually verifies the §5.7 criteria on alpha.elohim.host.
- **ZERO third portal** — doorway HTTP API (`auth_routes.rs`) stays as substrate; only ONE visible portal UI exists, expressed through the Lit elements.

---

## 8. Open Questions for the Implementation Plan

1. **Shift breakdown** — likely two shifts: (a) primitives + Library A/B stories + a2o test scaffold; (b) standalone EPR bundle + Angular wrapper cleanup + Mode B/Tauri integration. Writing-plans evaluates final boundaries.

2. **`/.well-known/elohim-doorway` endpoint** — the federated-resolver's standalone HTTP path needs this surface from doorway. Existing `DoorwayRegistryService` likely already calls something equivalent; the implementation plan audits the actual endpoint.

3. **Tauri local conductor `/auth/me` endpoint** — does the existing Tauri sidecar already expose `/auth/me` against localhost? If not, that's a small implementation task for tauri-architect.

4. **OAuth-client registration surface for third-party RPs** — the current registered-clients table in `oauth_session.rs` is hardcoded (`get_registered_clients()`). MVP can keep this for the consent flow; a dynamic registration UI is a follow-up.

5. **PortalHost lookup endpoint** — error 4.5 requires querying "which doorways has this imagodei authorized?" Does this surface exist in `auth_routes.rs` or does it need to be added? Implementation plan checks.

---

## Appendix A: Glossary

- **trustMode** — the operational mode the portal is running under: `doorway-host` (doorway runs your conductor; you're a guest on the flywheel) or `peer-conductor` (your own conductor is the auth authority; doorway at most ingress).
- **Authority** — the actor that signs your auth tokens: a specific doorway (in Mode A) or a specific peer-conductor (in Mode B). Reported by the `/auth/me` endpoint.
- **Flywheel** — the protocol-built-in expectation that Mode A users graduate to Mode B over time. Doorway-hosted accounts are temporary by design.
- **PortalHost** — a notarized DHT entry (Category A) declaring that an imagodei has authorized a specific doorway to render its auth portal. Accumulated during agency graduation.
- **Attestor** — a member of your qahal, intimate circle, recovery witness committee, or other community body that vouches for your humanity / identity. The social-trust substrate.
- **Federated identifier** — the `user@gateway-host` form (e.g., `matthew@alpha.elohim.host`) used to discover which doorway handles a user.
- **RP (Relying Party)** — an external app requesting user claims via OAuth.

---

## Appendix B: P2P-Design-Gate Decisions

Per `.claude/skills/p2p-design-gate/SKILL.md`:

1. **Entity classification:** No new entities. This design is purely the UI layer over existing substrate:
   - PortalHost (Category A — notarized) already exists in `imagodei_integrity::portal_host`
   - OAuthSessionDoc + OAuthClient (Category C — operational) already exist in `doorway-service/src/db/schemas/oauth_session.rs`
   - Session state (Category B — agent-scoped) already exists as cookies the doorway sets
   - No new DHT entry types proposed by this design.

2. **Existing entry types reused:** Yes — PortalHost (read-only in MVP) and the OAuth-session/client tables.

3. **Identity:** Existing — the imagodei's humanId is the identity; the PortalHost entry's `human_action_hash` is the substrate-attestable identity.

4. **Coordinator functions:** No new ones. Existing doorway HTTP handlers in `auth_routes.rs` are the coordinator surface. The UI primitives are pure-frontend consumers.

The post-write `p2p-design-gate` heuristic audit may flag new schema/route mentions in this spec; this appendix is the authoritative answer that all referenced schemas/routes are EXISTING substrate, not new entities introduced by this design.

---

## Appendix C: Cross-References

- Sibling design: `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — provides elohim-core's Session, Loader, page-chrome, EPR-link primitives this portal consumes.
- Predecessor substrate spec: `2026-04-25-recovery-protocol-phase-2-m5-...` (referenced by `portal_host.rs`) — defines the PortalHost reach model + agency-graduation flow.
- Substrate code:
  - `doorway/doorway-service/src/routes/auth_routes.rs`
  - `doorway/doorway-service/src/db/schemas/oauth_session.rs`
  - `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/portal_host.rs`
  - `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/recovery_v2.rs`
- Existing Angular components to be wrapped:
  - `app/elohim-app/src/app/imagodei/components/login/login.component.ts`
  - `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.ts`
  - `app/elohim-app/src/app/imagodei/services/auth.service.ts`
  - `app/elohim-app/src/app/imagodei/services/providers/oauth-auth.provider.ts`
  - `app/elohim-app/src/app/imagodei/services/providers/password-auth.provider.ts`
  - `app/elohim-app/src/app/imagodei/services/tauri-auth.service.ts`
  - `app/elohim-app/src/app/imagodei/services/doorway-registry.service.ts`

---

*End of design.*
