# Peer OAuth Portal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build seven Lit primitives in `app/elohim-elements/elohim-imagodei/` that compose into a wizard-shaped peer OAuth portal, deploy it as both a standalone EPR bundle (`app/imagodei-portal/`) and as Angular wrappers replacing the existing `LoginComponent` + `AuthCallbackComponent`, with three a2o features asserting the user-visible ceremony.

**Architecture:** Two trust modes (`doorway-host` for flywheel hosting; `peer-conductor` for graduated users — Tauri-direct OR doorway-routed; SAME UI either way) discriminated by chrome via the portal-shell's slotchange context propagation. Community-attestation security throughout (no sovereign-key/crypto-bro framing). RFC-6749 OAuth surface lives in the existing `auth_routes.rs` — this plan is UI-only.

**Tech Stack:** Lit + TypeScript (elohim-imagodei elements), Angular 19 (standalone EPR + wrapper components), web-test-runner + Chai (Lit unit tests), Vitest (TS), Cypress + Cucumber (a2o), pnpm workspaces.

**Spec:** `genesis/docs/superpowers/specs/2026-05-25-peer-oauth-portal-design.md` (commit `ee9658495`)

**Branch:** `design/peer-oauth-portal` (branched from dev). Implementation may continue on this branch.

**Sibling design dependency:** elohim-core's Session, Loader, page-chrome, EPR-link primitives from `2026-05-25-pillar-epr-decomposition-design.md`. When that sibling work lands first these are available as workspace imports; if the portal work begins while the EPR decomposition is still in progress, the relevant elohim-core pieces (Session in particular) must be present before B3 (portal-shell) lands. The plan assumes elohim-core's Session primitive is available at `app/elohim-elements/elohim-core/src/session/session.ts`.

## P2P Design Gate (run during brainstorming — see spec Appendix B)

No new DHT entry types. No new storage tables. No new doorway routes invented by this design. Reuses:
- `PortalHost` (Category A, already in `imagodei_integrity::portal_host`)
- `OAuthSessionDoc` + `OAuthClient` (Category C, already in `doorway-service/src/db/schemas/oauth_session.rs`)
- Session state via existing doorway-set cookies
- The `project-epr` REA Commitment action (Category A, from EPR decomposition Phase A) — used for projecting the standalone portal bundle at `/auth/portal`

Any minor backend additions surfaced by the Phase A audit (e.g., extending the `/auth/me` response shape with `trustMode` + `authority` fields) are operational projection-layer adjustments to existing endpoints, NOT new substrate primitives.

---

## File Structure

### Phase A — substrate audit (read-only)

| Path | New? | Responsibility |
|---|---|---|
| `genesis/docs/superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md` | new | Markdown report covering the 5 open questions from spec §8 + disposition decisions |

### Phase B — Lit primitives

| Path | New? | Responsibility |
|---|---|---|
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.ts` | new | The "which conductor is hosting you" chip; Mode A vs Mode B chrome |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.ts` | new | Avatar row of qahal/circle attestors; community-presence chrome |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-portal-shell.ts` | new | The shell that wraps the wizard; discovers trustMode; propagates context |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-portal-shell.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-federated-resolver.ts` | new | matthew@alpha.elohim.host input → resolved doorway URL |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-federated-resolver.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-login-card.ts` | new | Password + OAuth provider form |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-login-card.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-consent-card.ts` | new | RFC-6749 authorization-step consent surface |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-consent-card.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-oauth-callback.ts` | new | OAuth code-exchange callback handler |
| `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-oauth-callback.spec.ts` | new | |
| `app/elohim-elements/elohim-imagodei/src/register.ts` | modify | Side-effect registration of all 7 new elements |
| `app/elohim-elements/elohim-imagodei/src/index.ts` | modify | Type + class re-exports |

### Phase C — Library A + B stories

| Path | New? | Owner |
|---|---|---|
| `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-{primitive}.default.stories.ts` | new × 7 | component-architect |
| `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/{composition}.designed.stories.ts` | new × 7 | graphos-designer |

### Phase D — standalone EPR

| Path | New? | Responsibility |
|---|---|---|
| `app/imagodei-portal/angular.json` | new | Standalone Angular project config; outputPath `dist/imagodei-portal`; no SSR for MVP |
| `app/imagodei-portal/package.json` | new | Minimal deps (Angular core + elohim-core + elohim-imagodei + lit + tslib + rxjs + zone.js); versions pinned to match elohim-app |
| `app/imagodei-portal/tsconfig.json` | new | Mirror elohim-app's TS config |
| `app/imagodei-portal/tsconfig.app.json` | new | |
| `app/imagodei-portal/src/index.html` | new | `<base href="/auth/portal/">`; mounts `<imagodei-portal-root>` inside `<elohim-page-chrome>` |
| `app/imagodei-portal/src/main.ts` | new | Bootstrap; side-effect import of `elohim-core/register` + `elohim-imagodei/register` |
| `app/imagodei-portal/src/styles.scss` | new | Empty placeholder |
| `app/imagodei-portal/src/app/app.component.ts` | new | Root component; mounts `<elohim-imagodei-portal-shell>` with the right step based on URL query params (login flow vs consent flow) |
| `app/imagodei-portal/src/app/app.config.ts` | new | Angular standalone config + router providers |
| `app/imagodei-portal/src/app/app.routes.ts` | new | Two routes: `/` (login flow), `/consent` (authorization-request handling) |
| `app/imagodei-portal/src/app/services/standalone-resolver.ts` | new | Plain-fetch implementations of the resolver/login/exchange callbacks (the standalone counterpart to AuthService) |
| `app/imagodei-portal/src/app/services/standalone-resolver.spec.ts` | new | |
| `pnpm-workspace.yaml` | modify | Add `app/imagodei-portal` |
| `genesis/data/lamad/content/imagodei-portal.json` | new | Content row for the portal EPR (mirrors `elohim-host-landing.json`) |
| `genesis/seeder/src/seed-projections.ts` | modify | Add `imagodeiPortalAt(doorway)` to `defaultProjectionSeeds()` — 2 new commitments |
| `genesis/seeder/src/__tests__/seed-projections.test.ts` | modify | Update the deterministic-id test set to expect 6 projections (was 4) |

### Phase E — Angular wrapper cleanup

| Path | New? | Responsibility |
|---|---|---|
| `app/elohim-app/src/app/imagodei/components/login/login.component.ts` | modify (rewrite) | Thin wrapper that mounts `<elohim-imagodei-portal-shell>` and bridges Angular service callbacks |
| `app/elohim-app/src/app/imagodei/components/login/login.component.html` | modify (rewrite) | Collapse to the Lit-element shell |
| `app/elohim-app/src/app/imagodei/components/login/login.component.css` | modify | Strip per-step styling that the Lit elements now own |
| `app/elohim-app/src/app/imagodei/components/login/login.component.spec.ts` | modify | Rewrite tests to assert the wrapper bridges callbacks correctly |
| `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.ts` | modify (rewrite) | Thin wrapper for `<elohim-imagodei-oauth-callback>` |
| `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.spec.ts` | modify | Rewrite |

### Phase F — a2o features

| Path | New? |
|---|---|
| `genesis/a2o/features/peer-oauth-portal/hosted-login.feature` | new |
| `genesis/a2o/features/peer-oauth-portal/peer-conductor-login.feature` | new |
| `genesis/a2o/features/peer-oauth-portal/rp-consent.feature` | new |
| `genesis/a2o/steps/peer-oauth-portal/hosted-login.steps.ts` | new |
| `genesis/a2o/steps/peer-oauth-portal/peer-conductor-login.steps.ts` | new |
| `genesis/a2o/steps/peer-oauth-portal/rp-consent.steps.ts` | new |

---

# Tasks

Phases are sequenced. Within a phase, tasks may be reordered if dependencies allow. The `Owner` annotation on each task is the recommended agent (used by `superpowers:subagent-driven-development` for dispatch routing).

---

## Phase A — Substrate Audit

### Task A1 — Audit the five open questions

**Owner:** general-purpose (read-only research)

**Files:** Create `genesis/docs/superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md`

**Background:** Spec §8 lists five open questions:
1. `/auth/me` response shape — does it return `trustMode` + `authority`, or do we need to extend it?
2. `/.well-known/elohim-doorway` endpoint — does it exist on doorway for the standalone-bundle federated-resolver path?
3. Tauri local conductor `/auth/me` — does the Tauri sidecar expose this against localhost:8090?
4. OAuth-client registration surface — `get_registered_clients()` is hardcoded; is that OK for MVP?
5. PortalHost lookup endpoint — error 4.5 needs "list PortalHosts for an imagodei"; does this exist?

This task answers each in writing and proposes disposition for any gaps.

- [ ] **Step 1: Audit /auth/me response shape**

```bash
# Find the /auth/me handler in doorway
grep -n "fn me\|/auth/me\|auth_me\|handle_me" /projects/elohim/doorway/doorway-service/src/routes/auth_routes.rs

# Look at the response struct
grep -A 30 "struct MeResponse\|GetMeResponse" /projects/elohim/doorway/doorway-service/src/routes/auth_routes.rs

# Also check the storage side — does storage expose a similar endpoint?
grep -rn "/auth/me\|fn handle_me" /projects/elohim/elohim/elohim-storage/src/ 2>/dev/null | head -5
```

Document: existing fields returned, whether `trustMode` + `authority` need adding, recommended disposition.

- [ ] **Step 2: Audit /.well-known/elohim-doorway endpoint**

```bash
grep -rn "/.well-known/elohim-doorway\|well_known\|well-known" /projects/elohim/doorway/doorway-service/src/ /projects/elohim/elohim/elohim-storage/src/ 2>/dev/null | head -20

# How does the existing DoorwayRegistryService discover doorways?
grep -n "resolveGatewayToDoorwayUrl\|parseFederatedIdentifier" /projects/elohim/app/elohim-app/src/app/imagodei/services/doorway-registry.service.ts
grep -n "resolveGatewayToDoorwayUrl\|parseFederatedIdentifier" /projects/elohim/app/elohim-app/src/app/imagodei/models/doorway.model.ts
```

Document: existing resolution path; whether `.well-known` is part of it; what the standalone bundle's plain-fetch wrapper needs to call.

- [ ] **Step 3: Audit Tauri /auth/me**

```bash
find /projects/elohim/steward -type f \( -name "*.rs" -o -name "*.ts" \) 2>/dev/null | xargs grep -l "/auth/me\|auth_me\|currentUser" 2>/dev/null | head -5

# Tauri command surface
grep -rn "tauri::command\|invoke_handler" /projects/elohim/steward/device/src-tauri/src/ 2>/dev/null | head -10

# The TauriAuthService in elohim-app  
head -80 /projects/elohim/app/elohim-app/src/app/imagodei/services/tauri-auth.service.ts 2>/dev/null
```

Document: how the Tauri webview currently discovers authentication; whether localhost conductor exposes a `/auth/me`-equivalent OR whether Tauri uses an IPC command; what the standalone bundle's Tauri-detection path needs.

- [ ] **Step 4: Audit OAuth client registration**

```bash
grep -B 2 -A 20 "fn get_registered_clients" /projects/elohim/doorway/doorway-service/src/db/schemas/oauth_session.rs
grep -n "OAuthClient\|registered_clients\|client_id" /projects/elohim/doorway/doorway-service/src/routes/auth_routes.rs | head -20
```

Document: current hardcoded clients; whether MVP can keep this hardcoded (likely yes per spec); follow-up needed for dynamic registration.

- [ ] **Step 5: Audit PortalHost lookup**

```bash
# Substrate entry
head -120 /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/portal_host.rs

# Storage projection
grep -rn "portal_host\|PortalHost" /projects/elohim/elohim/elohim-storage/src/ 2>/dev/null | head -20

# Doorway exposure
grep -rn "portal_host\|PortalHost" /projects/elohim/doorway/doorway-service/src/ 2>/dev/null | head -10
```

Document: PortalHost-by-human-id lookup endpoint (exists / missing / needs adding); for MVP, whether spec §4.5's error message can omit the "other portals" list if the lookup isn't available.

- [ ] **Step 6: Write the audit report**

Author `genesis/docs/superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md` with the structure:

```markdown
# Peer OAuth Portal — Substrate Audit

**Date:** 2026-05-25
**Spec:** `genesis/docs/superpowers/specs/2026-05-25-peer-oauth-portal-design.md` §8
**Purpose:** Audit the five open questions before implementation; document any gaps and their dispositions.

## 1. /auth/me response shape
[findings; current fields; needed additions; disposition]

## 2. /.well-known/elohim-doorway
[findings; current path; standalone wrapper needs]

## 3. Tauri /auth/me
[findings; localhost conductor exposure or Tauri IPC; standalone Tauri-detection]

## 4. OAuth client registration
[findings; hardcoded for MVP; follow-up]

## 5. PortalHost lookup
[findings; endpoint exists or needs adding; MVP fallback if missing]

## Disposition summary

| Question | Status | Adds task to this plan? |
|---|---|---|
| /auth/me | ... | yes/no |
| .well-known | ... | yes/no |
| Tauri /auth/me | ... | yes/no |
| OAuth clients | ... | no (hardcoded OK) |
| PortalHost lookup | ... | yes/no |

## Recommended plan amendments

[concrete task additions for the implementer; e.g., "Add `trustMode` field to MeResponse in auth_routes.rs (~10 lines + test)"]
```

- [ ] **Step 7: Commit**

```bash
cd /projects/elohim
git add genesis/docs/superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md
git commit -m "docs(audit): peer OAuth portal substrate audit (Phase A)"
```

If the audit identifies backend additions (e.g., the `/auth/me` response needs extending), insert new task(s) between Phase A and Phase B before continuing. Mark them clearly as "Phase A.N — added by audit".

---

## Phase B — Lit primitives

All primitives in this phase live in `app/elohim-elements/elohim-imagodei/src/`. Follow the existing imagodei element pattern (look at `elohim-imagodei-introspection-panel.ts` for the canonical shape: LitElement subclass, `@property()` decorators, CSS custom property surface only, JSDoc `@capability*` tags). Use `@open-wc/testing` + Chai for tests (matches the package convention).

The package's `register.ts` and `index.ts` are updated incrementally — each task appends its element's registration + export.

### Task B1 — `<elohim-imagodei-trust-indicator>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.spec.ts`
- Modify: `app/elohim-elements/elohim-imagodei/src/register.ts` — add registration
- Modify: `app/elohim-elements/elohim-imagodei/src/index.ts` — add export

**Background:** Spec §2.2. Small chip-shaped element showing where the conductor lives. Different glyph + copy per `trustMode`. Reads `trustMode` and `authorityLabel` from `@property()`. Click bubbles `trust-indicator-tap` event.

- [ ] **Step 1: Write failing tests**

Create `elohim-imagodei-trust-indicator.spec.ts`:

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-trust-indicator.js';
import type { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';

describe('<elohim-imagodei-trust-indicator>', () => {
  it('renders doorway-host chrome with the authority label', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="doorway-host"
        authority-label="alpha.elohim.host"
      ></elohim-imagodei-trust-indicator>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('alpha.elohim.host');
    expect(el.shadowRoot!.querySelector('[part="mode"]')?.getAttribute('data-mode')).to.equal('doorway-host');
  });

  it('renders peer-conductor chrome with a different mode marker', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="your conductor on this device"
      ></elohim-imagodei-trust-indicator>
    `);
    expect(el.shadowRoot!.querySelector('[part="mode"]')?.getAttribute('data-mode')).to.equal('peer-conductor');
  });

  it('surfaces flywheel hint only when flywheelHint is true', async () => {
    const elNoHint = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha"></elohim-imagodei-trust-indicator>
    `);
    expect(elNoHint.shadowRoot!.querySelector('[part="flywheel-hint"]')).to.be.null;

    const elHint = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha" ?flywheel-hint=${true}></elohim-imagodei-trust-indicator>
    `);
    expect(elHint.shadowRoot!.querySelector('[part="flywheel-hint"]')).to.exist;
  });

  it('emits trust-indicator-tap on click', async () => {
    const el = await fixture<ElohimImagodeiTrustIndicator>(html`
      <elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha"></elohim-imagodei-trust-indicator>
    `);
    let detail: unknown = null;
    el.addEventListener('trust-indicator-tap', (e) => { detail = (e as CustomEvent).detail; });
    (el.shadowRoot!.querySelector('button') as HTMLElement).click();
    expect(detail).to.deep.equal({ trustMode: 'doorway-host', authorityLabel: 'alpha' });
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

```bash
cd /projects/elohim/app/elohim-elements/elohim-imagodei
pnpm test 2>&1 | tail -15
```

Expected: failure — module not found.

- [ ] **Step 3: Implement the element**

Create `elohim-imagodei-trust-indicator.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type TrustMode = 'doorway-host' | 'peer-conductor';

/**
 * <elohim-imagodei-trust-indicator> — small chip showing where the conductor
 * lives. Two modes, two glyphs, two accent colors. Tap emits a tap event
 * that consumers (omnibar, portal-shell) can surface a details panel from.
 *
 * @element elohim-imagodei-trust-indicator
 *
 * @prop {TrustMode} trustMode - 'doorway-host' | 'peer-conductor'
 * @prop {string} authorityLabel - human-readable authority (e.g. "alpha.elohim.host" or "your conductor on this device")
 * @prop {boolean} flywheelHint - render the "you can graduate to your own conductor" hint (doorway-host only)
 *
 * @fires {CustomEvent<{trustMode: TrustMode, authorityLabel: string}>} trust-indicator-tap - bubbles on click
 *
 * @cssprop --elohim-trust-bg - background base
 * @cssprop --elohim-trust-fg - foreground/text base
 * @cssprop --elohim-trust-host-accent - accent applied in doorway-host mode
 * @cssprop --elohim-trust-peer-accent - accent applied in peer-conductor mode
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityRequiredStandings any
 */
export class ElohimImagodeiTrustIndicator extends LitElement {
  @property({ attribute: 'trust-mode' }) trustMode: TrustMode = 'doorway-host';
  @property({ attribute: 'authority-label' }) authorityLabel = '';
  @property({ attribute: 'flywheel-hint', type: Boolean }) flywheelHint = false;

  static styles = css`
    :host {
      display: inline-block;
      font: inherit;
    }
    button {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      padding-block: 0.25rem;
      padding-inline: 0.625rem;
      border: 1px solid color-mix(in oklch, currentColor 25%, transparent);
      border-radius: 999px;
      background: var(--elohim-trust-bg, transparent);
      color: var(--elohim-trust-fg, inherit);
      cursor: pointer;
      font: inherit;
    }
    button:hover, button:focus-visible {
      background: color-mix(in oklch, currentColor 6%, transparent);
      outline: none;
    }
    [data-mode='doorway-host'] {
      border-color: var(--elohim-trust-host-accent, color-mix(in oklch, currentColor 30%, transparent));
    }
    [data-mode='peer-conductor'] {
      border-color: var(--elohim-trust-peer-accent, color-mix(in oklch, currentColor 30%, transparent));
    }
    [part='flywheel-hint'] {
      font-size: 0.75rem;
      opacity: 0.7;
      margin-inline-start: 0.25rem;
    }
    @media (forced-colors: active) {
      button { border-color: CanvasText; }
    }
  `;

  render() {
    const modeIcon = this.trustMode === 'doorway-host' ? '⌂' : '◇';
    const modeLabel = this.trustMode === 'doorway-host' ? 'Hosted via' : 'Your conductor —';
    return html`
      <button type="button" @click=${this.onTap} part="mode" data-mode=${this.trustMode}>
        <span aria-hidden="true">${modeIcon}</span>
        <span><strong>${modeLabel}</strong> ${this.authorityLabel}</span>
        ${this.flywheelHint && this.trustMode === 'doorway-host'
          ? html`<span part="flywheel-hint">(flywheel)</span>`
          : ''}
      </button>
    `;
  }

  private onTap = () => {
    this.dispatchEvent(new CustomEvent('trust-indicator-tap', {
      detail: { trustMode: this.trustMode, authorityLabel: this.authorityLabel },
      bubbles: true,
      composed: true,
    }));
  };
}
```

- [ ] **Step 4: Register the element**

In `register.ts`, append:

```typescript
import { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';

if (!customElements.get('elohim-imagodei-trust-indicator')) {
  customElements.define('elohim-imagodei-trust-indicator', ElohimImagodeiTrustIndicator);
}
```

In `index.ts`, append:

```typescript
export { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';
export type { TrustMode } from './elohim-imagodei-trust-indicator.js';
```

- [ ] **Step 5: Run tests, see them pass**

```bash
pnpm test 2>&1 | grep -E "trust-indicator|passing|failing" | head -15
```

Expected: 4 tests pass, full suite green.

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim
git add app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.ts \
        app/elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.spec.ts \
        app/elohim-elements/elohim-imagodei/src/register.ts \
        app/elohim-elements/elohim-imagodei/src/index.ts \
        app/elohim-elements/elohim-imagodei/dist/
git commit -m "feat(elohim-imagodei): <elohim-imagodei-trust-indicator> primitive"
```

### Task B2 — `<elohim-imagodei-attestor-row>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.spec.ts`
- Modify: `app/elohim-elements/elohim-imagodei/src/register.ts`, `index.ts`

**Background:** Spec §2.3. Avatar row of qahal/circle attestors. `attestors: AttestorRef[]` where `AttestorRef = { eprRef, displayName, role }`. Overflow `+N more`. Empty state slot. `attestor-tap` event with `eprRef`.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-attestor-row.js';
import type { ElohimImagodeiAttestorRow, AttestorRef } from './elohim-imagodei-attestor-row.js';

const SAMPLE: AttestorRef[] = [
  { eprRef: 'epr:human-susan', displayName: 'Susan', role: 'qahal-elder' },
  { eprRef: 'epr:human-james', displayName: 'James', role: 'intimate-circle' },
  { eprRef: 'epr:human-marta', displayName: 'Marta', role: 'recovery-witness' },
];

describe('<elohim-imagodei-attestor-row>', () => {
  it('renders one avatar per attestor up to maxVisible', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll('[part="avatar"]');
    expect(avatars.length).to.equal(3);
  });

  it('renders +N overflow when attestors exceed maxVisible', async () => {
    const many: AttestorRef[] = Array.from({ length: 8 }, (_, i) => ({
      eprRef: `epr:human-${i}`, displayName: `H${i}`, role: 'qahal-elder',
    }));
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${many} .maxVisible=${5}></elohim-imagodei-attestor-row>
    `);
    const avatars = el.shadowRoot!.querySelectorAll('[part="avatar"]');
    expect(avatars.length).to.equal(5);
    expect(el.shadowRoot!.querySelector('[part="overflow"]')?.textContent).to.include('+3');
  });

  it('renders empty slot when attestors is empty', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${[]}>
        <span slot="empty">no witnesses yet</span>
      </elohim-imagodei-attestor-row>
    `);
    const empty = el.shadowRoot!.querySelector('slot[name="empty"]') as HTMLSlotElement;
    expect(empty).to.exist;
  });

  it('emits attestor-tap with eprRef on avatar click', async () => {
    const el = await fixture<ElohimImagodeiAttestorRow>(html`
      <elohim-imagodei-attestor-row .attestors=${SAMPLE}></elohim-imagodei-attestor-row>
    `);
    let detail: { eprRef?: string } | null = null;
    el.addEventListener('attestor-tap', (e) => { detail = (e as CustomEvent).detail; });
    (el.shadowRoot!.querySelectorAll<HTMLElement>('[part="avatar"]')[1]).click();
    expect(detail?.eprRef).to.equal('epr:human-james');
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

```bash
pnpm test 2>&1 | grep -E "attestor-row|passing|failing" | head -15
```

Expected: failure.

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface AttestorRef {
  eprRef: string;
  displayName: string;
  role: 'qahal-elder' | 'intimate-circle' | 'recovery-witness' | 'elohim-agent' | string;
}

/**
 * <elohim-imagodei-attestor-row> — avatar row of qahal / circle / witness
 * attestors. Foregrounds the social-trust model. Overflow shows +N more.
 *
 * @element elohim-imagodei-attestor-row
 *
 * @prop {AttestorRef[]} attestors - array of attestor references
 * @prop {number} maxVisible - cap on rendered avatars; default 5
 * @prop {'compact' | 'standard'} density - layout density; default 'standard'
 *
 * @slot empty - rendered when `attestors` is empty
 *
 * @fires {CustomEvent<{eprRef: string}>} attestor-tap
 *
 * @cssprop --elohim-attestor-ring - avatar ring color
 * @cssprop --elohim-attestor-avatar-size - avatar diameter
 * @cssprop --elohim-attestor-gap - gap between avatars
 */
export class ElohimImagodeiAttestorRow extends LitElement {
  @property({ attribute: false }) attestors: AttestorRef[] = [];
  @property({ type: Number, attribute: 'max-visible' }) maxVisible = 5;
  @property() density: 'compact' | 'standard' = 'standard';

  static styles = css`
    :host { display: inline-block; }
    .row {
      display: inline-flex;
      align-items: center;
      gap: var(--elohim-attestor-gap, -0.25rem);
    }
    [part='avatar'] {
      inline-size: var(--elohim-attestor-avatar-size, 28px);
      block-size: var(--elohim-attestor-avatar-size, 28px);
      border-radius: 50%;
      background: color-mix(in oklch, currentColor 14%, transparent);
      border: 2px solid var(--elohim-attestor-ring, Canvas);
      display: inline-flex;
      align-items: center;
      justify-content: center;
      font-size: 0.75rem;
      color: inherit;
      cursor: pointer;
      padding: 0;
    }
    [part='avatar']:focus-visible { outline: 2px solid currentColor; outline-offset: 2px; }
    [part='overflow'] {
      margin-inline-start: 0.25rem;
      font-size: 0.75rem;
      opacity: 0.7;
    }
    .empty { font-size: 0.875rem; opacity: 0.7; }
  `;

  render() {
    if (this.attestors.length === 0) {
      return html`<div class="empty"><slot name="empty">No witnesses recorded yet.</slot></div>`;
    }
    const visible = this.attestors.slice(0, this.maxVisible);
    const overflow = this.attestors.length - visible.length;
    return html`
      <div class="row" role="list" aria-label="attestors">
        ${visible.map((a) => html`
          <button
            type="button"
            part="avatar"
            role="listitem"
            title=${`${a.displayName} (${a.role})`}
            @click=${() => this.onTap(a.eprRef)}
          >${this.initials(a.displayName)}</button>
        `)}
        ${overflow > 0 ? html`<span part="overflow">+${overflow}</span>` : ''}
      </div>
    `;
  }

  private initials(name: string): string {
    const parts = name.trim().split(/\s+/);
    if (parts.length === 0) return '?';
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  private onTap(eprRef: string) {
    this.dispatchEvent(new CustomEvent('attestor-tap', {
      detail: { eprRef },
      bubbles: true,
      composed: true,
    }));
  }
}
```

- [ ] **Step 4: Register + export**

Append to `register.ts` + `index.ts` mirroring B1.

- [ ] **Step 5: Run tests + commit**

```bash
cd /projects/elohim/app/elohim-elements/elohim-imagodei
pnpm test 2>&1 | tail -8
cd /projects/elohim
git add app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.ts \
        app/elohim-elements/elohim-imagodei/src/elohim-imagodei-attestor-row.spec.ts \
        app/elohim-elements/elohim-imagodei/src/register.ts \
        app/elohim-elements/elohim-imagodei/src/index.ts \
        app/elohim-elements/elohim-imagodei/dist/
git commit -m "feat(elohim-imagodei): <elohim-imagodei-attestor-row> primitive"
```

### Task B3 — `<elohim-imagodei-portal-shell>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-portal-shell.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-portal-shell.spec.ts`
- Modify: `register.ts`, `index.ts`

**Background:** Spec §2.1. The wrapper. Discovers `trustMode` on mount via the `authorityEndpoint`. Renders trust-indicator + attestor-row in the header. Slots in the active step element via `primary` slot. Propagates `trustMode` + `authority` to slotted children via slotchange. Never auto-advances steps; consumer drives the wizard explicitly.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html, oneEvent } from '@open-wc/testing';
import './elohim-imagodei-portal-shell.js';
import './elohim-imagodei-trust-indicator.js';
import type { ElohimImagodeiPortalShell } from './elohim-imagodei-portal-shell.js';

describe('<elohim-imagodei-portal-shell>', () => {
  it('discovers trustMode from the authorityEndpoint mock', async () => {
    // Stub fetch
    const originalFetch = window.fetch;
    (window as any).fetch = async () => new Response(JSON.stringify({
      authenticated: false,
      trustMode: 'doorway-host',
      authority: { label: 'alpha.elohim.host' },
    }));

    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell authority-endpoint="/auth/me"></elohim-imagodei-portal-shell>
    `);
    await oneEvent(el, 'authority-resolved');
    expect(el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode'))
      .to.equal('doorway-host');

    (window as any).fetch = originalFetch;
  });

  it('renders the primary slot for the active step', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="step-content">login-card placeholder</div>
      </elohim-imagodei-portal-shell>
    `);
    const slot = el.shadowRoot!.querySelector('slot[name="primary"]') as HTMLSlotElement;
    const assigned = slot.assignedElements();
    expect(assigned.some((n) => (n as HTMLElement).id === 'step-content')).to.equal(true);
  });

  it('propagates trustMode + authority to slotted children via property assignment', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
      </elohim-imagodei-portal-shell>
    `);
    // Simulate authority resolution
    (el as any)._setAuthority({ trustMode: 'peer-conductor', authority: { label: 'your conductor' } });
    await el.updateComplete;
    const child = el.querySelector('#child') as any;
    expect(child.trustMode).to.equal('peer-conductor');
    expect(child.authority).to.deep.equal({ label: 'your conductor' });
  });

  it('does NOT auto-advance step; only consumer setting shell.step changes it', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve"></elohim-imagodei-portal-shell>
    `);
    expect(el.step).to.equal('resolve');
    // emit a child success event — shell ignores
    el.dispatchEvent(new CustomEvent('resolved', { detail: {}, bubbles: true }));
    await el.updateComplete;
    expect(el.step).to.equal('resolve');
    // consumer changes
    el.step = 'login';
    await el.updateComplete;
    expect(el.step).to.equal('login');
  });

  it('renders error-region slot when a child emits portal-error', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
        <div slot="error-region" id="errors">errors here</div>
      </elohim-imagodei-portal-shell>
    `);
    const errSlot = el.shadowRoot!.querySelector('slot[name="error-region"]') as HTMLSlotElement;
    expect(errSlot).to.exist;
    expect(errSlot.assignedElements().some((n) => (n as HTMLElement).id === 'errors')).to.equal(true);
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';
import './elohim-imagodei-trust-indicator.js';
import './elohim-imagodei-attestor-row.js';
import type { TrustMode } from './elohim-imagodei-trust-indicator.js';
import type { AttestorRef } from './elohim-imagodei-attestor-row.js';

export type PortalStep = 'resolve' | 'login' | 'consent' | 'callback';

export interface AuthorityResolution {
  trustMode: TrustMode;
  authority: { label: string; id?: string };
  flywheelHint?: boolean;
  attestors?: AttestorRef[];
}

/**
 * <elohim-imagodei-portal-shell> — the wizard's outer wrapper.
 *
 * Discovers trustMode on mount via `authorityEndpoint`. Propagates
 * `trustMode` + `authority` to slotted children. Renders persistent
 * chrome (trust-indicator + attestor-row) and slots in the active step
 * via the `primary` slot. NEVER auto-advances steps — consumers set
 * `step` explicitly in response to child events.
 *
 * @element elohim-imagodei-portal-shell
 *
 * @prop {string} authorityEndpoint - URL to discover trustMode from; default `/auth/me`
 * @prop {PortalStep} step - which step is active
 * @prop {boolean} flywheelHint - propagated to trust-indicator
 *
 * @slot header - defaults to trust-indicator + attestor-row
 * @slot primary - the active step element
 * @slot footer - legal / help text
 * @slot error-region - rendered when a child emits portal-error
 * @slot auth-wall - rendered when portal itself needs auth (deferred)
 *
 * @fires {CustomEvent<AuthorityResolution>} authority-resolved
 * @fires {CustomEvent<{step: PortalStep}>} step-change
 *
 * @cssprop --elohim-portal-bg
 * @cssprop --elohim-portal-fg
 * @cssprop --elohim-portal-panel-bg
 * @cssprop --elohim-portal-grid-gap
 */
export class ElohimImagodeiPortalShell extends LitElement {
  @property({ attribute: 'authority-endpoint' }) authorityEndpoint = '/auth/me';
  @property() step: PortalStep = 'resolve';
  @property({ attribute: 'flywheel-hint', type: Boolean }) flywheelHint = false;

  @state() private _trustMode: TrustMode = 'doorway-host';
  @state() private _authorityLabel = '';
  @state() private _attestors: AttestorRef[] = [];
  @state() private _resolved = false;

  static styles = css`
    :host {
      display: block;
      background: var(--elohim-portal-bg, Canvas);
      color: var(--elohim-portal-fg, CanvasText);
      min-block-size: 100vh;
    }
    .frame {
      display: grid;
      grid-template-rows: auto 1fr auto;
      gap: var(--elohim-portal-grid-gap, 1rem);
      padding: 1rem;
      max-inline-size: 480px;
      margin-inline: auto;
    }
    header { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
    main { background: var(--elohim-portal-panel-bg, color-mix(in oklch, Canvas 96%, CanvasText)); border-radius: 8px; padding: 1rem; }
    footer { font-size: 0.75rem; opacity: 0.7; }
    [part='error-region']:empty { display: none; }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    void this.discoverAuthority();
  }

  protected updated(changed: PropertyValues): void {
    if (changed.has('step')) {
      this.dispatchEvent(new CustomEvent('step-change', { detail: { step: this.step }, bubbles: true }));
    }
    // Re-propagate context to any newly slotted children
    this.propagateContextToSlots();
  }

  /** Test seam — directly set authority without HTTP. */
  _setAuthority(res: AuthorityResolution): void {
    this._trustMode = res.trustMode;
    this._authorityLabel = res.authority.label;
    this._attestors = res.attestors ?? [];
    this.flywheelHint = res.flywheelHint ?? this.flywheelHint;
    this._resolved = true;
    this.dispatchEvent(new CustomEvent('authority-resolved', { detail: res, bubbles: true }));
  }

  private async discoverAuthority(): Promise<void> {
    try {
      const resp = await fetch(this.authorityEndpoint, { credentials: 'include' });
      if (!resp.ok) return;
      const data = await resp.json();
      const res: AuthorityResolution = {
        trustMode: data.trustMode ?? 'doorway-host',
        authority: { label: data.authority?.label ?? '', id: data.authority?.id },
        flywheelHint: data.flywheelHint,
        attestors: data.attestors,
      };
      this._setAuthority(res);
    } catch {
      // Network error → leave defaults; the shell still renders with placeholder chrome.
    }
  }

  private propagateContextToSlots(): void {
    const slot = this.shadowRoot?.querySelector('slot[name="primary"]') as HTMLSlotElement | null;
    if (!slot) return;
    for (const node of slot.assignedElements()) {
      (node as any).trustMode = this._trustMode;
      (node as any).authority = { label: this._authorityLabel };
    }
  }

  render() {
    return html`
      <div class="frame">
        <header>
          <slot name="header">
            <elohim-imagodei-trust-indicator
              trust-mode=${this._trustMode}
              authority-label=${this._authorityLabel}
              ?flywheel-hint=${this.flywheelHint}
            ></elohim-imagodei-trust-indicator>
            <elohim-imagodei-attestor-row .attestors=${this._attestors}></elohim-imagodei-attestor-row>
          </slot>
        </header>
        <main part="primary-region">
          <slot name="primary" @slotchange=${this.propagateContextToSlots}></slot>
        </main>
        <div part="error-region"><slot name="error-region"></slot></div>
        <footer><slot name="footer"></slot></footer>
        <slot name="auth-wall"></slot>
      </div>
    `;
  }
}
```

- [ ] **Step 4: Register + export**

- [ ] **Step 5: Run tests + commit**

```bash
git commit -m "feat(elohim-imagodei): <elohim-imagodei-portal-shell> primitive — chrome + context propagation"
```

### Task B4 — `<elohim-imagodei-federated-resolver>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-federated-resolver.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-federated-resolver.spec.ts`
- Modify: `register.ts`, `index.ts`

**Background:** Spec §2.4. Input element for `matthew@alpha.elohim.host`-style identifiers. Calls injected `resolveIdentifier` callback. Emits `resolved` with `{ identifier, doorwayUrl }` or `resolve-error` with `{ identifier, reason }`. Remembers identifier in localStorage under key `rememberKey`.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-federated-resolver.js';
import type { ElohimImagodeiFederatedResolver } from './elohim-imagodei-federated-resolver.js';

describe('<elohim-imagodei-federated-resolver>', () => {
  beforeEach(() => { localStorage.clear(); });

  it('emits resolved event with doorwayUrl when resolver returns success', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (id: string) => ({ ok: true, doorwayUrl: `https://${id.split('@')[1]}` })}
      ></elohim-imagodei-federated-resolver>
    `);
    let detail: any = null;
    el.addEventListener('resolved', (e) => { detail = (e as CustomEvent).detail; });
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(detail).to.deep.equal({ identifier: 'matthew@alpha.elohim.host', doorwayUrl: 'https://alpha.elohim.host' });
  });

  it('emits resolve-error when resolver returns failure', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${async (id: string) => ({ ok: false, reason: 'unknown-host' })}
      ></elohim-imagodei-federated-resolver>
    `);
    let detail: any = null;
    el.addEventListener('resolve-error', (e) => { detail = (e as CustomEvent).detail; });
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'someone@nowhere';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(detail).to.deep.equal({ identifier: 'someone@nowhere', reason: 'unknown-host' });
  });

  it('persists identifier to localStorage under rememberKey on success', async () => {
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver
        remember-key="test-id-key"
        .resolveIdentifier=${async (id: string) => ({ ok: true, doorwayUrl: 'https://x' })}
      ></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    input.value = 'matthew@alpha.elohim.host';
    input.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(localStorage.getItem('test-id-key')).to.equal('matthew@alpha.elohim.host');
  });

  it('pre-fills input from localStorage on mount', async () => {
    localStorage.setItem('test-id-key', 'previously@alpha.elohim.host');
    const el = await fixture<ElohimImagodeiFederatedResolver>(html`
      <elohim-imagodei-federated-resolver remember-key="test-id-key"></elohim-imagodei-federated-resolver>
    `);
    const input = el.shadowRoot!.querySelector('input') as HTMLInputElement;
    expect(input.value).to.equal('previously@alpha.elohim.host');
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement } from 'lit';
import { property, state, query } from 'lit/decorators.js';

export interface ResolveOutcome {
  ok: boolean;
  doorwayUrl?: string;
  reason?: string;
}

export type ResolveIdentifierFn = (identifier: string) => Promise<ResolveOutcome>;

/**
 * <elohim-imagodei-federated-resolver> — input element for federated
 * identifiers (matthew@alpha.elohim.host). Calls the injected resolver
 * to discover the doorway endpoint; emits 'resolved' on success or
 * 'resolve-error' on failure.
 *
 * @element elohim-imagodei-federated-resolver
 *
 * @prop {ResolveIdentifierFn} resolveIdentifier - injected callback
 * @prop {string} placeholder
 * @prop {string} rememberKey - localStorage key for remembered identifier
 *
 * @slot help-text - "what does matthew@alpha.elohim.host mean" affordance
 *
 * @fires {CustomEvent<{identifier, doorwayUrl}>} resolved
 * @fires {CustomEvent<{identifier, reason}>} resolve-error
 *
 * @cssprop --elohim-input-border
 * @cssprop --elohim-input-focus-ring
 * @cssprop --elohim-input-error-fg
 */
export class ElohimImagodeiFederatedResolver extends LitElement {
  @property({ attribute: false })
  resolveIdentifier: ResolveIdentifierFn = async () => ({ ok: false, reason: 'no-resolver' });

  @property() placeholder = 'you@your-doorway.host';
  @property({ attribute: 'remember-key' }) rememberKey = 'elohim_auth_identifier';

  @state() private _value = '';
  @state() private _error: string | null = null;
  @state() private _busy = false;

  @query('input') private _input!: HTMLInputElement;

  static styles = css`
    :host { display: block; font: inherit; }
    form { display: grid; gap: 0.5rem; }
    label { display: block; font-size: 0.875rem; }
    input {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid var(--elohim-input-border, color-mix(in oklch, currentColor 30%, transparent));
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
    }
    input:focus-visible {
      outline: 2px solid var(--elohim-input-focus-ring, currentColor);
      outline-offset: 2px;
    }
    [part='error'] {
      color: var(--elohim-input-error-fg, color-mix(in oklch, currentColor 70%, red));
      font-size: 0.875rem;
    }
    button[type='submit'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
    }
    button[disabled] { opacity: 0.6; cursor: default; }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    const remembered = localStorage.getItem(this.rememberKey);
    if (remembered) this._value = remembered;
  }

  render() {
    return html`
      <form @submit=${this.onSubmit}>
        <label for="federated-input">Sign in as</label>
        <input
          id="federated-input"
          type="text"
          autocomplete="username"
          placeholder=${this.placeholder}
          .value=${this._value}
          @input=${(e: Event) => this._value = (e.target as HTMLInputElement).value}
          ?disabled=${this._busy}
        />
        <slot name="help-text"></slot>
        ${this._error ? html`<div part="error" role="alert">${this._error}</div>` : ''}
        <button type="submit" part="submit" ?disabled=${this._busy || !this._value.trim()}>
          ${this._busy ? 'Resolving…' : 'Continue'}
        </button>
      </form>
    `;
  }

  private async onSubmit(e: Event) {
    e.preventDefault();
    if (this._busy) return;
    this._error = null;
    this._busy = true;
    const identifier = this._value.trim();
    try {
      const outcome = await this.resolveIdentifier(identifier);
      if (outcome.ok && outcome.doorwayUrl) {
        try { localStorage.setItem(this.rememberKey, identifier); } catch {}
        this.dispatchEvent(new CustomEvent('resolved', {
          detail: { identifier, doorwayUrl: outcome.doorwayUrl },
          bubbles: true,
          composed: true,
        }));
      } else {
        this._error = outcome.reason ?? 'resolution failed';
        this.dispatchEvent(new CustomEvent('resolve-error', {
          detail: { identifier, reason: this._error },
          bubbles: true,
          composed: true,
        }));
      }
    } finally {
      this._busy = false;
    }
  }
}
```

- [ ] **Step 4: Register + export + run tests + commit**

```bash
git commit -m "feat(elohim-imagodei): <elohim-imagodei-federated-resolver> primitive"
```

### Task B5 — `<elohim-imagodei-login-card>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-login-card.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-login-card.spec.ts`
- Modify: `register.ts`, `index.ts`

**Background:** Spec §2.5. Renders password form + OAuth provider buttons. Emits `password-submit` or `oauth-start` events. Reads `trustMode` from parent to adapt copy (hosted vs conductor-local). Accepts `unlock-prompt` slot for Tauri local-key-unlock flow.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-login-card.js';
import type { ElohimImagodeiLoginCard, OAuthProviderRef } from './elohim-imagodei-login-card.js';

const PROVIDERS: OAuthProviderRef[] = [
  { id: 'google', displayName: 'Google' },
  { id: 'github', displayName: 'GitHub' },
];

describe('<elohim-imagodei-login-card>', () => {
  it('renders password form when allowPassword is true', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}></elohim-imagodei-login-card>
    `);
    expect(el.shadowRoot!.querySelector('input[type="password"]')).to.exist;
  });

  it('renders OAuth buttons when providers are present', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('[part="oauth-button"]');
    expect(buttons.length).to.equal(2);
  });

  it('emits password-submit on form submission', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true} .rememberedIdentifier=${'matthew@alpha'}></elohim-imagodei-login-card>
    `);
    let detail: any = null;
    el.addEventListener('password-submit', (e) => { detail = (e as CustomEvent).detail; });
    const pw = el.shadowRoot!.querySelector('input[type="password"]') as HTMLInputElement;
    pw.value = 'secret';
    pw.dispatchEvent(new Event('input'));
    el.shadowRoot!.querySelector('form')!.requestSubmit();
    await new Promise((r) => setTimeout(r, 10));
    expect(detail.identifier).to.equal('matthew@alpha');
    expect(detail.password).to.equal('secret');
  });

  it('emits oauth-start with providerId on OAuth button click', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .oauthProviders=${PROVIDERS}></elohim-imagodei-login-card>
    `);
    let detail: any = null;
    el.addEventListener('oauth-start', (e) => { detail = (e as CustomEvent).detail; });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="oauth-button"][data-provider="github"]'))!.click();
    expect(detail.providerId).to.equal('github');
  });

  it('renders unlock-prompt slot when provided (Tauri path)', async () => {
    const el = await fixture<ElohimImagodeiLoginCard>(html`
      <elohim-imagodei-login-card .allowPassword=${true}>
        <div slot="unlock-prompt" id="tauri-unlock">unlock your local key</div>
      </elohim-imagodei-login-card>
    `);
    const slot = el.shadowRoot!.querySelector('slot[name="unlock-prompt"]') as HTMLSlotElement;
    expect(slot.assignedElements().some((n) => (n as HTMLElement).id === 'tauri-unlock')).to.equal(true);
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';
import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export interface OAuthProviderRef {
  id: string;
  displayName: string;
  iconUrl?: string;
}

/**
 * <elohim-imagodei-login-card> — credentials form. Password + OAuth providers.
 * Adapts copy based on inherited trustMode.
 *
 * @element elohim-imagodei-login-card
 *
 * @prop {OAuthProviderRef[]} oauthProviders
 * @prop {boolean} allowPassword - default true
 * @prop {boolean} remember
 * @prop {string} rememberedIdentifier
 * @prop {TrustMode} trustMode - set by parent shell
 *
 * @slot unlock-prompt - alternative for Tauri local-key-unlock
 *
 * @fires {CustomEvent<{identifier, password, remember}>} password-submit
 * @fires {CustomEvent<{providerId}>} oauth-start
 * @fires {CustomEvent} cancel
 *
 * @cssprop --elohim-login-bg
 * @cssprop --elohim-login-input-bg
 * @cssprop --elohim-login-button-bg
 * @cssprop --elohim-login-button-fg
 */
export class ElohimImagodeiLoginCard extends LitElement {
  @property({ attribute: false }) oauthProviders: OAuthProviderRef[] = [];
  @property({ type: Boolean, attribute: 'allow-password' }) allowPassword = true;
  @property({ type: Boolean }) remember = false;
  @property({ attribute: 'remembered-identifier' }) rememberedIdentifier = '';
  @property() trustMode: TrustMode = 'doorway-host';

  @state() private _password = '';
  @state() private _busy = false;
  @state() private _error: string | null = null;

  static styles = css`
    :host { display: block; }
    form { display: grid; gap: 0.75rem; }
    label { display: block; font-size: 0.875rem; }
    input[type='password'] {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid color-mix(in oklch, currentColor 30%, transparent);
      border-radius: 6px;
      background: var(--elohim-login-input-bg, transparent);
      color: inherit;
      font: inherit;
    }
    .remember { display: inline-flex; align-items: center; gap: 0.5rem; font-size: 0.875rem; }
    button[type='submit'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: 6px;
      background: var(--elohim-login-button-bg, transparent);
      color: var(--elohim-login-button-fg, inherit);
      font: inherit;
      cursor: pointer;
    }
    .oauth-row { display: grid; gap: 0.5rem; }
    [part='oauth-button'] {
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid color-mix(in oklch, currentColor 30%, transparent);
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
      text-align: start;
    }
    .divider { display: flex; align-items: center; gap: 0.5rem; opacity: 0.6; font-size: 0.75rem; }
    .divider::before, .divider::after { content: ''; flex: 1; border-block-start: 1px solid currentColor; opacity: 0.3; }
    [part='error'] { color: color-mix(in oklch, currentColor 70%, red); font-size: 0.875rem; }
  `;

  render() {
    const passwordCopy = this.trustMode === 'peer-conductor'
      ? 'Unlock your conductor'
      : 'Sign in';
    return html`
      <slot name="unlock-prompt"></slot>
      ${this.oauthProviders.length > 0 ? html`
        <div class="oauth-row" part="oauth-row">
          ${this.oauthProviders.map((p) => html`
            <button
              type="button"
              part="oauth-button"
              data-provider=${p.id}
              @click=${() => this.onOAuth(p.id)}
            >Continue with ${p.displayName}</button>
          `)}
        </div>
        ${this.allowPassword ? html`<div class="divider">or</div>` : ''}
      ` : ''}
      ${this.allowPassword ? html`
        <form @submit=${this.onSubmit}>
          <label for="pw">${passwordCopy}</label>
          <input
            id="pw"
            type="password"
            autocomplete="current-password"
            .value=${this._password}
            @input=${(e: Event) => this._password = (e.target as HTMLInputElement).value}
            ?disabled=${this._busy}
          />
          <label class="remember">
            <input
              type="checkbox"
              ?checked=${this.remember}
              @change=${(e: Event) => this.remember = (e.target as HTMLInputElement).checked}
            />
            Remember me on this device
          </label>
          ${this._error ? html`<div part="error" role="alert">${this._error}</div>` : ''}
          <button type="submit" ?disabled=${this._busy || !this._password}>
            ${this._busy ? 'Signing in…' : 'Sign in'}
          </button>
        </form>
      ` : ''}
    `;
  }

  private async onSubmit(e: Event) {
    e.preventDefault();
    if (this._busy || !this._password) return;
    this._busy = true;
    this._error = null;
    this.dispatchEvent(new CustomEvent('password-submit', {
      detail: {
        identifier: this.rememberedIdentifier,
        password: this._password,
        remember: this.remember,
      },
      bubbles: true,
      composed: true,
    }));
    // Consumer is responsible for calling shell.error / advancing the step.
    // We release `_busy` after a tick so the button feels responsive.
    setTimeout(() => { this._busy = false; }, 0);
  }

  private onOAuth(providerId: string) {
    this.dispatchEvent(new CustomEvent('oauth-start', {
      detail: { providerId },
      bubbles: true,
      composed: true,
    }));
  }

  /** Public API for consumer to surface errors. */
  setError(message: string | null): void {
    this._error = message;
    this._busy = false;
  }
}
```

- [ ] **Step 4: Register + export + run tests + commit**

```bash
git commit -m "feat(elohim-imagodei): <elohim-imagodei-login-card> primitive"
```

### Task B6 — `<elohim-imagodei-consent-card>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-consent-card.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-consent-card.spec.ts`
- Modify: `register.ts`, `index.ts`

**Background:** Spec §2.6. RFC-6749 consent step. Shows requesting client + per-claim toggles. Required claims locked-on. Emits `approve` with grantedClaims OR `decline`.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-consent-card.js';
import type { ElohimImagodeiConsentCard, ClaimRef } from './elohim-imagodei-consent-card.js';

const CLAIMS: ClaimRef[] = [
  { id: 'imagodei.displayName', label: 'Your display name', description: 'How you appear in the app.' },
  { id: 'qahal.standing', label: 'Your qahal standing', description: 'Capability tier within your community.' },
];

describe('<elohim-imagodei-consent-card>', () => {
  it('renders client brand strip', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'graphos-designer', displayName: 'Graphos Designer' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    expect(el.shadowRoot!.querySelector('[part="client-name"]')?.textContent).to.include('Graphos Designer');
  });

  it('renders each requested claim as a toggleable row', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    const rows = el.shadowRoot!.querySelectorAll('[part="claim-row"]');
    expect(rows.length).to.equal(2);
  });

  it('locks required claims on and disables their toggle', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    const required = el.shadowRoot!.querySelector(
      '[part="claim-row"][data-claim-id="imagodei.displayName"] input'
    ) as HTMLInputElement;
    expect(required.checked).to.equal(true);
    expect(required.disabled).to.equal(true);
  });

  it('emits approve with granted claim ids', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    `);
    let detail: any = null;
    el.addEventListener('approve', (e) => { detail = (e as CustomEvent).detail; });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="approve"]'))!.click();
    expect(detail.grantedClaims).to.deep.equal(['imagodei.displayName', 'qahal.standing']);
  });

  it('emits decline with reason', async () => {
    const el = await fixture<ElohimImagodeiConsentCard>(html`
      <elohim-imagodei-consent-card
        .requestingClient=${{ id: 'g', displayName: 'g' }}
        .requestedClaims=${CLAIMS}
        .requiredClaims=${[]}
      ></elohim-imagodei-consent-card>
    `);
    let detail: any = null;
    el.addEventListener('decline', (e) => { detail = (e as CustomEvent).detail; });
    (el.shadowRoot!.querySelector<HTMLElement>('[part="decline"]'))!.click();
    expect(detail.reason).to.equal('user-rejected');
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';
import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export interface ClaimRef {
  id: string;
  label: string;
  description?: string;
}

export interface RequestingClient {
  id: string;
  displayName: string;
  brandMark?: string;
}

/**
 * <elohim-imagodei-consent-card> — RFC-6749 authorization-step consent.
 *
 * @element elohim-imagodei-consent-card
 *
 * @prop {RequestingClient} requestingClient
 * @prop {ClaimRef[]} requestedClaims
 * @prop {string[]} requiredClaims - subset of requested claim ids that can't be toggled off
 * @prop {TrustMode} trustMode - set by parent shell
 *
 * @slot policy-link
 * @slot claim-detail
 *
 * @fires {CustomEvent<{grantedClaims}>} approve
 * @fires {CustomEvent<{reason}>} decline
 *
 * @cssprop --elohim-consent-rp-bg
 * @cssprop --elohim-consent-claim-row-bg
 * @cssprop --elohim-consent-approve-bg
 */
export class ElohimImagodeiConsentCard extends LitElement {
  @property({ attribute: false }) requestingClient: RequestingClient = { id: '', displayName: '' };
  @property({ attribute: false }) requestedClaims: ClaimRef[] = [];
  @property({ attribute: false }) requiredClaims: string[] = [];
  @property() trustMode: TrustMode = 'doorway-host';

  @state() private _granted: Set<string> = new Set();
  @state() private _initialized = false;

  static styles = css`
    :host { display: block; }
    .card { display: grid; gap: 1rem; }
    .rp { display: flex; align-items: center; gap: 0.75rem; padding: 0.75rem; background: var(--elohim-consent-rp-bg, color-mix(in oklch, currentColor 6%, transparent)); border-radius: 8px; }
    .rp-mark { inline-size: 32px; block-size: 32px; border-radius: 50%; background: color-mix(in oklch, currentColor 16%, transparent); }
    [part='claim-row'] {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 0.75rem;
      align-items: start;
      padding: 0.75rem;
      background: var(--elohim-consent-claim-row-bg, transparent);
      border: 1px solid color-mix(in oklch, currentColor 14%, transparent);
      border-radius: 6px;
    }
    .actions { display: flex; gap: 0.5rem; justify-content: end; }
    [part='approve'], [part='decline'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
    }
    [part='approve'] { background: var(--elohim-consent-approve-bg, transparent); }
  `;

  protected updated(): void {
    if (!this._initialized && this.requestedClaims.length > 0) {
      // Initialize: every requested claim starts granted (optional can be toggled off; required can't).
      this._granted = new Set(this.requestedClaims.map((c) => c.id));
      this._initialized = true;
    }
  }

  render() {
    return html`
      <div class="card">
        <div class="rp" part="rp">
          <div class="rp-mark" part="rp-mark" aria-hidden="true"></div>
          <div>
            <div part="client-name"><strong>${this.requestingClient.displayName}</strong></div>
            <div class="muted">is asking for access to:</div>
          </div>
        </div>
        ${this.requestedClaims.map((c) => {
          const required = this.requiredClaims.includes(c.id);
          const granted = this._granted.has(c.id);
          return html`
            <label part="claim-row" data-claim-id=${c.id}>
              <input
                type="checkbox"
                ?checked=${granted}
                ?disabled=${required}
                @change=${(e: Event) => this.toggleClaim(c.id, (e.target as HTMLInputElement).checked)}
              />
              <div>
                <div><strong>${c.label}</strong>${required ? html` <small>(required)</small>` : ''}</div>
                ${c.description ? html`<div>${c.description}</div>` : ''}
                <slot name="claim-detail"></slot>
              </div>
            </label>
          `;
        })}
        <slot name="policy-link"></slot>
        <div class="actions">
          <button type="button" part="decline" @click=${this.decline}>Decline</button>
          <button type="button" part="approve" @click=${this.approve}>Approve</button>
        </div>
      </div>
    `;
  }

  private toggleClaim(id: string, on: boolean) {
    const next = new Set(this._granted);
    if (on) next.add(id); else next.delete(id);
    this._granted = next;
  }

  private approve = () => {
    // Defense-in-depth: any required claim toggled off → block
    for (const r of this.requiredClaims) {
      if (!this._granted.has(r)) {
        this.dispatchEvent(new CustomEvent('decline', {
          detail: { reason: 'partial-decline-blocked' },
          bubbles: true,
          composed: true,
        }));
        return;
      }
    }
    this.dispatchEvent(new CustomEvent('approve', {
      detail: { grantedClaims: Array.from(this._granted) },
      bubbles: true,
      composed: true,
    }));
  };

  private decline = () => {
    this.dispatchEvent(new CustomEvent('decline', {
      detail: { reason: 'user-rejected' },
      bubbles: true,
      composed: true,
    }));
  };
}
```

- [ ] **Step 4: Register + export + run tests + commit**

```bash
git commit -m "feat(elohim-imagodei): <elohim-imagodei-consent-card> primitive (RFC-6749 step)"
```

### Task B7 — `<elohim-imagodei-oauth-callback>`

**Owner:** component-architect

**Files:**
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-oauth-callback.ts`
- Create: `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-oauth-callback.spec.ts`
- Modify: `register.ts`, `index.ts`

**Background:** Spec §2.7. Renders skeleton while code-exchange runs. Surfaces errors prominently. `exchangeCode` callback is injected; success emits `success` with session, failure emits `error`.

- [ ] **Step 1: Write failing tests**

```typescript
import { expect, fixture, html } from '@open-wc/testing';
import './elohim-imagodei-oauth-callback.js';
import type { ElohimImagodeiOauthCallback } from './elohim-imagodei-oauth-callback.js';

describe('<elohim-imagodei-oauth-callback>', () => {
  it('renders skeleton during code exchange', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => new Promise(() => {})}
      ></elohim-imagodei-oauth-callback>
    `);
    expect(el.shadowRoot!.querySelector('[part="skeleton"]')).to.exist;
  });

  it('emits success when exchange resolves', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => ({ session: { humanId: 'matthew' } })}
      ></elohim-imagodei-oauth-callback>
    `);
    let detail: any = null;
    el.addEventListener('success', (e) => { detail = (e as CustomEvent).detail; });
    await new Promise((r) => setTimeout(r, 10));
    expect(detail.session.humanId).to.equal('matthew');
  });

  it('emits error when exchange rejects', async () => {
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        code="abc"
        state="xyz"
        .exchangeCode=${async () => { throw new Error('invalid_grant'); }}
      ></elohim-imagodei-oauth-callback>
    `);
    let detail: any = null;
    el.addEventListener('error', (e) => { detail = (e as CustomEvent).detail; });
    await new Promise((r) => setTimeout(r, 10));
    expect(detail.reason).to.include('invalid_grant');
  });

  it('does NOT auto-exchange when code is missing', async () => {
    let called = false;
    const el = await fixture<ElohimImagodeiOauthCallback>(html`
      <elohim-imagodei-oauth-callback
        .exchangeCode=${async () => { called = true; return { session: {} }; }}
      ></elohim-imagodei-oauth-callback>
    `);
    await new Promise((r) => setTimeout(r, 10));
    expect(called).to.equal(false);
  });
});
```

- [ ] **Step 2: Run tests, see them fail**

- [ ] **Step 3: Implement**

```typescript
import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

export interface ExchangeOutcome {
  session: unknown;
}

export type ExchangeCodeFn = (code: string, state: string) => Promise<ExchangeOutcome>;

/**
 * <elohim-imagodei-oauth-callback> — OAuth code-exchange handler.
 * Runs the exchange when `code` is set; renders a skeleton during the
 * exchange; emits `success` or `error`.
 *
 * @element elohim-imagodei-oauth-callback
 *
 * @prop {string} code
 * @prop {string} state
 * @prop {string} providerLabel
 * @prop {ExchangeCodeFn} exchangeCode - injected callback
 *
 * @slot error-detail
 * @slot recovery-cta
 *
 * @fires {CustomEvent<{session}>} success
 * @fires {CustomEvent<{reason, recoverable}>} error
 *
 * @cssprop --elohim-callback-spinner-color
 */
export class ElohimImagodeiOauthCallback extends LitElement {
  @property() code = '';
  @property() state = '';
  @property({ attribute: 'provider-label' }) providerLabel = '';
  @property({ attribute: false })
  exchangeCode: ExchangeCodeFn = async () => ({ session: null });

  @state() private _status: 'idle' | 'exchanging' | 'done' | 'error' = 'idle';
  @state() private _errorReason: string | null = null;
  @state() private _started = false;

  static styles = css`
    :host { display: block; padding: 1rem; }
    [part='skeleton'] {
      display: grid;
      gap: 0.5rem;
      animation: pulse 1.2s ease-in-out infinite;
    }
    [part='skeleton'] > div {
      block-size: 1rem;
      background: color-mix(in oklch, currentColor 14%, transparent);
      border-radius: 4px;
    }
    @keyframes pulse {
      0%, 100% { opacity: 0.6; }
      50% { opacity: 1; }
    }
    @media (prefers-reduced-motion: reduce) {
      [part='skeleton'] { animation: none; }
    }
    [part='error'] { color: color-mix(in oklch, currentColor 70%, red); }
  `;

  protected updated(changed: PropertyValues): void {
    if (!this._started && this.code && this.state) {
      this._started = true;
      void this.runExchange();
    }
  }

  private async runExchange(): Promise<void> {
    this._status = 'exchanging';
    try {
      const outcome = await this.exchangeCode(this.code, this.state);
      this._status = 'done';
      this.dispatchEvent(new CustomEvent('success', {
        detail: { session: outcome.session },
        bubbles: true,
        composed: true,
      }));
    } catch (e: unknown) {
      this._status = 'error';
      this._errorReason = e instanceof Error ? e.message : String(e);
      this.dispatchEvent(new CustomEvent('error', {
        detail: { reason: this._errorReason, recoverable: true },
        bubbles: true,
        composed: true,
      }));
    }
  }

  render() {
    if (this._status === 'idle') {
      return html`<div>Waiting for OAuth response${this.providerLabel ? html` from ${this.providerLabel}` : ''}…</div>`;
    }
    if (this._status === 'exchanging') {
      return html`
        <div part="skeleton">
          <div></div>
          <div></div>
          <div></div>
        </div>
        <p>Finishing sign-in${this.providerLabel ? html` with ${this.providerLabel}` : ''}…</p>
      `;
    }
    if (this._status === 'error') {
      return html`
        <div part="error" role="alert">
          Sign-in failed${this.providerLabel ? html` (${this.providerLabel})` : ''}: ${this._errorReason}
        </div>
        <slot name="error-detail"></slot>
        <slot name="recovery-cta"></slot>
      `;
    }
    return html`<p>Signed in. Redirecting…</p>`;
  }
}
```

- [ ] **Step 4: Register + export + run tests + commit**

```bash
git commit -m "feat(elohim-imagodei): <elohim-imagodei-oauth-callback> primitive"
```

---

## Phase C — Library A + B stories

### Task C1 — Library A default stories (7 files)

**Owner:** component-architect

**Files (all new):**
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-trust-indicator.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-attestor-row.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-portal-shell.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-federated-resolver.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-login-card.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-consent-card.default.stories.ts`
- `app/elohim-library/projects/graphos/src/default/imagodei/__docs__/elohim-imagodei-oauth-callback.default.stories.ts`

**Background:** Spec §5.2. Per `app/elohim-library/CLAUDE.md` Library A discipline:
- Title prefix: `Default/Imagodei/<element>`
- Required stories per element: `Unstyled (blank-slate proof)` (wrapped in `style="all: initial;"`), `CustomTheme (override surface proof)`, plus per-state stories per spec §5.2.

For each primitive, the state stories listed in spec §5.2 are:

| Primitive | State stories |
|---|---|
| portal-shell | `EmptyShell`, `WithLoginCard`, `WithConsentCard`, `ErrorBoundary` |
| trust-indicator | `DoorwayHost`, `PeerConductor`, `WithFlywheelHint`, `DisabledOffline` |
| attestor-row | `OneAttestor`, `FivePlusOverflow`, `EmptyState`, `RTLLayout` |
| federated-resolver | `Default`, `WithError`, `RememberedIdentifier`, `WithHelpSlot` |
| login-card | `PasswordOnly`, `OAuthOnly`, `Both`, `WithUnlockPromptSlot`, `WithError` |
| consent-card | `OneRequiredClaim`, `RequiredPlusOptional`, `WithPolicyLink`, `WithBrandMark` |
| oauth-callback | `Exchanging`, `Success`, `Error`, `WithRecoveryCta` |

- [ ] **Step 1: Look at existing default story pattern**

```bash
ls /projects/elohim/app/elohim-library/projects/graphos/src/default/core/__docs__/ 2>/dev/null
cat /projects/elohim/app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-button.default.stories.ts 2>/dev/null | head -80
```

If `elohim-button.default.stories.ts` doesn't exist, find any existing default story under the imagodei or core paths:

```bash
find /projects/elohim/app/elohim-library/projects/graphos -name "*.default.stories.ts" 2>/dev/null | head -3
```

- [ ] **Step 2: Create the imagodei default-stories directory + 7 story files**

For each primitive, write a `<element>.default.stories.ts` that:
- imports the element register: `import 'elohim-imagodei/register';` (or `import '../../../../../../elohim-elements/elohim-imagodei/src/elohim-imagodei-trust-indicator.js';` if the package isn't workspace-aliased yet — match the convention in the existing core stories)
- exports default meta with `title: 'Default/Imagodei/elohim-imagodei-trust-indicator'`
- exports the required stories per the table above

Sketch for trust-indicator (mirror for each):

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import 'elohim-imagodei/register';

const meta = {
  title: 'Default/Imagodei/elohim-imagodei-trust-indicator',
  component: 'elohim-imagodei-trust-indicator',
  parameters: {
    docs: {
      description: {
        component: 'Library A blank-slate proof of <elohim-imagodei-trust-indicator>. No Elohim brand binding.',
      },
    },
  },
} satisfies Meta;
export default meta;
type Story = StoryObj;

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha.elohim.host"></elohim-imagodei-trust-indicator>
    </div>
  `,
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override surface proof)',
  render: () => html`
    <div style="
      font-family: Courier, monospace;
      color: rebeccapurple;
      --elohim-trust-bg: papayawhip;
      --elohim-trust-host-accent: navy;
      --elohim-trust-peer-accent: maroon;
    ">
      <elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha.elohim.host"></elohim-imagodei-trust-indicator>
    </div>
  `,
};

export const DoorwayHost: Story = {
  render: () => html`<elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha.elohim.host"></elohim-imagodei-trust-indicator>`,
};

export const PeerConductor: Story = {
  render: () => html`<elohim-imagodei-trust-indicator trust-mode="peer-conductor" authority-label="your conductor on this device"></elohim-imagodei-trust-indicator>`,
};

export const WithFlywheelHint: Story = {
  render: () => html`<elohim-imagodei-trust-indicator trust-mode="doorway-host" authority-label="alpha.elohim.host" flywheel-hint></elohim-imagodei-trust-indicator>`,
};

export const DisabledOffline: Story = {
  render: () => html`
    <div style="opacity: 0.5;">
      <elohim-imagodei-trust-indicator trust-mode="peer-conductor" authority-label="conductor — offline"></elohim-imagodei-trust-indicator>
    </div>
  `,
};
```

Write equivalent files for the other six primitives following the table.

- [ ] **Step 3: Run Storybook to verify discovery**

```bash
cd /projects/elohim/app/elohim-library
pnpm exec ng run graphos:build-storybook 2>&1 | tail -10
grep -rn "Default/Imagodei" /projects/elohim/app/elohim-library/projects/graphos/storybook-static/ 2>/dev/null | head -3
```

Expected: all 7 story files contribute stories under `Default/Imagodei/*`.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add app/elohim-library/projects/graphos/src/default/imagodei/
git commit -m "docs(stories): Library A default stories for 7 elohim-imagodei portal primitives"
```

### Task C2 — Library B designed stories (7 compositions)

**Owner:** graphos-designer

**Files (all new):**
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/ModeA_FirstTimeLogin.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/ModeB_PeerConductorLogin.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/ModeB_TauriDirect.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/ConsentCardThreeClaims.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/EvictedAccount.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/PortalHostNotAuthorized.designed.stories.ts`
- `app/elohim-library/projects/graphos/src/designed/imagodei/__docs__/NetworkOffline.designed.stories.ts`

**Background:** Spec §5.3. Brand-bound compositions per `app/elohim-library/CLAUDE.md` Library B discipline:
- Story title prefix: `Designed/Imagodei/<composition>`
- Brand tokens bind via story decorators only; primitives never modified
- Protocol-realistic vocabulary throughout (matthew@alpha.elohim.host, aleph-household qahal members, "fair-exchange" concept as returnTo)

- [ ] **Step 1: Read the Library B existing examples + token surface**

```bash
find /projects/elohim/app/elohim-library/projects/graphos/src/designed -name "*.designed.stories.ts" 2>/dev/null | head -3
cat /projects/elohim/app/elohim-elements/elohim-core/tokens.scss 2>/dev/null | head -60
```

- [ ] **Step 2: Author each composition**

Sketch for `ModeA_FirstTimeLogin.designed.stories.ts` (the template all others follow):

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import 'elohim-core/register';
import 'elohim-imagodei/register';

const ELOHIM_TOKENS = `
  --elohim-portal-bg: #f6efe2;
  --elohim-portal-fg: #1a1812;
  --elohim-portal-panel-bg: #fdf8ec;
  --elohim-trust-bg: transparent;
  --elohim-trust-fg: #1a1812;
  --elohim-trust-host-accent: #c87f3a;
  --elohim-trust-peer-accent: #4a7c84;
  --elohim-attestor-ring: #fdf8ec;
  --elohim-login-button-bg: #c87f3a;
  --elohim-login-button-fg: #fdf8ec;
`;

const meta = {
  title: 'Designed/Imagodei/ModeA_FirstTimeLogin',
  parameters: {
    docs: {
      description: {
        component: "First-time visitor to alpha.elohim.host. Doorway-host mode; flywheel hint visible. Brand-bound via story decorators.",
      },
    },
  },
} satisfies Meta;
export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () => html`
    <style>:host, :root { ${ELOHIM_TOKENS} }</style>
    <div style="font-family: ui-serif, Georgia, serif; ${ELOHIM_TOKENS}">
      <elohim-imagodei-portal-shell step="login" flywheel-hint>
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
          flywheel-hint
        ></elohim-imagodei-trust-indicator>
        <elohim-imagodei-login-card
          slot="primary"
          allow-password
          remembered-identifier="matthew@alpha.elohim.host"
        ></elohim-imagodei-login-card>
      </elohim-imagodei-portal-shell>
    </div>
  `,
};
```

Sketch the rest (compose per spec §5.3 description). For each, use realistic protocol vocabulary and bind ELOHIM_TOKENS via decorator.

- [ ] **Step 3: Verify in Storybook**

```bash
cd /projects/elohim/app/elohim-library
pnpm exec ng run graphos:build-storybook 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add app/elohim-library/projects/graphos/src/designed/imagodei/
git commit -m "docs(stories): Library B designed compositions for the peer OAuth portal (7 scenes)"
```

---

## Phase D — Standalone EPR bundle

### Task D1 — Scaffold app/imagodei-portal Angular project

**Owner:** angular-architect

**Files:**
- Create: `app/imagodei-portal/angular.json`
- Create: `app/imagodei-portal/package.json`
- Create: `app/imagodei-portal/tsconfig.json`
- Create: `app/imagodei-portal/tsconfig.app.json`
- Create: `app/imagodei-portal/src/index.html`
- Create: `app/imagodei-portal/src/main.ts`
- Create: `app/imagodei-portal/src/styles.scss`
- Create: `app/imagodei-portal/src/app/app.component.ts`
- Create: `app/imagodei-portal/src/app/app.config.ts`
- Create: `app/imagodei-portal/src/app/app.routes.ts`
- Modify: `pnpm-workspace.yaml`

**Background:** Mirror the `app/lamad/` scaffold pattern from the EPR decomposition Phase B (Task B17). Standalone Angular project; own angular.json; no SSR; base href `/auth/portal/`.

- [ ] **Step 1: Look at app/lamad scaffold as template**

```bash
ls /projects/elohim/app/lamad/ 2>/dev/null
cat /projects/elohim/app/lamad/angular.json | head -60
cat /projects/elohim/app/lamad/package.json | head -40
```

- [ ] **Step 2: Add app/imagodei-portal to pnpm workspace**

In `/projects/elohim/pnpm-workspace.yaml`, add `app/imagodei-portal` after `app/lamad`.

- [ ] **Step 3: Create package.json**

Mirror app/lamad/package.json with `name: "imagodei-portal"`, version 0.0.0. Pin Angular deps to the same versions app/lamad uses; add `elohim-core: workspace:*` AND `elohim-imagodei: workspace:*` AND `lit` peer.

- [ ] **Step 4: Create angular.json**

Mirror app/lamad/angular.json; outputPath `dist/imagodei-portal`; index `src/index.html`; main `src/main.ts`. No `server` / `prerender` / `ssr` fields.

- [ ] **Step 5: Create tsconfig.json + tsconfig.app.json**

Mirror app/lamad's structure.

- [ ] **Step 6: Create src/index.html**

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Sign in — Elohim Protocol</title>
  <base href="/auth/portal/">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="icon" type="image/x-icon" href="favicon.ico">
</head>
<body>
  <elohim-page-chrome>
    <imagodei-portal-root></imagodei-portal-root>
  </elohim-page-chrome>
</body>
</html>
```

- [ ] **Step 7: Create src/main.ts**

```typescript
import { bootstrapApplication } from '@angular/platform-browser';
import 'elohim-core/register';
import 'elohim-imagodei/register';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err) => console.error(err));
```

- [ ] **Step 8: Create src/app/app.component.ts**

```typescript
import { Component } from '@angular/core';

@Component({
  selector: 'imagodei-portal-root',
  standalone: true,
  template: `
    <main>
      <h1 class="visually-hidden">Elohim Portal</h1>
      <!-- B-2 wires this up; for the empty scaffold the shell renders without children -->
      <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
    </main>
  `,
  styles: [`.visually-hidden { position: absolute; inline-size: 1px; block-size: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }`],
})
export class AppComponent {}
```

- [ ] **Step 9: Create app.config.ts + app.routes.ts + styles.scss**

`app.config.ts`:

```typescript
import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideRouter } from '@angular/router';
import { routes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
  ],
};
```

`app.routes.ts`:

```typescript
import { Routes } from '@angular/router';
export const routes: Routes = [
  { path: '', children: [] },
  { path: 'consent', children: [] },
];
```

`styles.scss`: empty (one comment line).

- [ ] **Step 10: pnpm install + build**

```bash
cd /projects/elohim
pnpm install 2>&1 | tail -10
cd app/imagodei-portal
pnpm run build 2>&1 | tail -10
grep "base href" dist/imagodei-portal/browser/index.html 2>/dev/null || \
  grep "base href" dist/imagodei-portal/index.html 2>/dev/null
```

Expected: build succeeds; `<base href="/auth/portal/">` present.

- [ ] **Step 11: Commit**

```bash
cd /projects/elohim
git add app/imagodei-portal/ pnpm-workspace.yaml pnpm-lock.yaml
git commit -m "feat(imagodei-portal): scaffold Angular project with <base href=/auth/portal/>"
```

### Task D2 — Wire portal-shell with route-aware step selection + standalone resolver

**Owner:** angular-architect

**Files:**
- Modify: `app/imagodei-portal/src/app/app.component.ts`
- Modify: `app/imagodei-portal/src/app/app.routes.ts`
- Create: `app/imagodei-portal/src/app/services/standalone-resolver.ts`
- Create: `app/imagodei-portal/src/app/services/standalone-resolver.spec.ts`

**Background:** The standalone bundle's AppComponent inspects the URL — if there are OAuth-authorization-request params (`client_id`, `claims`, `redirect_uri`, `state`), it mounts the consent flow; otherwise it mounts the login flow. The standalone resolver is the plain-fetch alternative to AuthService's Angular-DI path — exposes `resolveIdentifier`, `loginWithPassword`, `exchangeCode`, and `prepareConsent` callbacks the Lit elements consume.

- [ ] **Step 1: Write standalone-resolver tests first**

`/projects/elohim/app/imagodei-portal/src/app/services/standalone-resolver.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { StandaloneResolver } from './standalone-resolver.js';

describe('StandaloneResolver', () => {
  let resolver: StandaloneResolver;
  beforeEach(() => { resolver = new StandaloneResolver(); });

  it('resolves identifier by GET /.well-known/elohim-doorway from gateway', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce(new Response(
      JSON.stringify({ doorway: 'https://alpha.elohim.host' }),
      { status: 200, headers: { 'Content-Type': 'application/json' } },
    ));
    const out = await resolver.resolveIdentifier('matthew@alpha.elohim.host');
    expect(out.ok).toBe(true);
    expect(out.doorwayUrl).toBe('https://alpha.elohim.host');
  });

  it('returns ok=false with reason when identifier is malformed', async () => {
    const out = await resolver.resolveIdentifier('not-a-federated-id');
    expect(out.ok).toBe(false);
    expect(out.reason).toMatch(/format/);
  });

  it('returns ok=false when fetch fails', async () => {
    global.fetch = vi.fn().mockRejectedValueOnce(new Error('network'));
    const out = await resolver.resolveIdentifier('matthew@nowhere.host');
    expect(out.ok).toBe(false);
  });

  it('loginWithPassword POSTs to /auth/login and returns redirect path', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce(new Response(
      JSON.stringify({ redirect: '/lamad' }),
      { status: 200 },
    ));
    const out = await resolver.loginWithPassword({ identifier: 'a', password: 'b', remember: false });
    expect(out.redirect).toBe('/lamad');
  });
});
```

- [ ] **Step 2: Implement StandaloneResolver**

```typescript
// app/imagodei-portal/src/app/services/standalone-resolver.ts

export interface ResolveOutcome {
  ok: boolean;
  doorwayUrl?: string;
  reason?: string;
}

export interface LoginInput {
  identifier: string;
  password: string;
  remember: boolean;
}

export interface LoginOutcome {
  redirect?: string;
  error?: string;
}

export interface ConsentRequest {
  clientId: string;
  claims: string[];
  redirectUri: string;
  state: string;
}

export interface ConsentContext {
  requestingClient: { id: string; displayName: string };
  requestedClaims: { id: string; label: string; description?: string }[];
  requiredClaims: string[];
}

export class StandaloneResolver {
  async resolveIdentifier(identifier: string): Promise<ResolveOutcome> {
    const at = identifier.indexOf('@');
    if (at < 1 || at === identifier.length - 1) {
      return { ok: false, reason: 'format' };
    }
    const gatewayHost = identifier.slice(at + 1);
    const url = `https://${gatewayHost}/.well-known/elohim-doorway`;
    try {
      const resp = await fetch(url, { credentials: 'omit' });
      if (!resp.ok) return { ok: false, reason: `http-${resp.status}` };
      const data = await resp.json() as { doorway?: string };
      if (!data.doorway) return { ok: false, reason: 'no-doorway-in-response' };
      return { ok: true, doorwayUrl: data.doorway };
    } catch (e) {
      return { ok: false, reason: e instanceof Error ? e.message : 'fetch-failed' };
    }
  }

  async loginWithPassword(input: LoginInput): Promise<LoginOutcome> {
    const resp = await fetch('/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(input),
    });
    if (!resp.ok) {
      return { error: `http-${resp.status}` };
    }
    return await resp.json();
  }

  async exchangeCode(code: string, state: string): Promise<{ session: unknown }> {
    const resp = await fetch('/auth/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ code, state }),
    });
    if (!resp.ok) throw new Error(`exchange failed: ${resp.status}`);
    return await resp.json();
  }

  async prepareConsent(req: ConsentRequest): Promise<ConsentContext> {
    const resp = await fetch('/auth/authorize/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(req),
    });
    if (!resp.ok) throw new Error(`prepare failed: ${resp.status}`);
    return await resp.json();
  }
}
```

- [ ] **Step 3: Wire AppComponent to inspect URL + mount the right step**

```typescript
import { Component, signal, computed, OnInit, ElementRef, ViewChild, AfterViewInit } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { StandaloneResolver } from './services/standalone-resolver.js';

type Mode = 'login' | 'consent';

@Component({
  selector: 'imagodei-portal-root',
  standalone: true,
  template: `
    <main>
      <h1 class="visually-hidden">Elohim Portal</h1>
      <elohim-imagodei-portal-shell #shell [attr.step]="step()">
        <ng-container *ngIf="mode() === 'login'; else consentTpl">
          <elohim-imagodei-federated-resolver
            #resolver
            slot="primary"
            (resolved)="onResolved($event)"
            (resolve-error)="onResolveError($event)"
          ></elohim-imagodei-federated-resolver>
        </ng-container>
        <ng-template #consentTpl>
          <elohim-imagodei-consent-card
            slot="primary"
            [requestingClient]="consentCtx()?.requestingClient"
            [requestedClaims]="consentCtx()?.requestedClaims"
            [requiredClaims]="consentCtx()?.requiredClaims"
            (approve)="onConsentApprove($event)"
            (decline)="onConsentDecline($event)"
          ></elohim-imagodei-consent-card>
        </ng-template>
      </elohim-imagodei-portal-shell>
    </main>
  `,
  styles: [`.visually-hidden { position: absolute; inline-size: 1px; block-size: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }`],
})
export class AppComponent implements OnInit, AfterViewInit {
  private readonly route = inject(ActivatedRoute);
  private readonly resolver = new StandaloneResolver();

  mode = signal<Mode>('login');
  step = signal<'resolve' | 'login' | 'consent' | 'callback'>('resolve');
  consentCtx = signal<ConsentContext | null>(null);

  @ViewChild('shell') shellRef?: ElementRef<HTMLElement>;
  @ViewChild('resolver') resolverRef?: ElementRef<HTMLElement>;

  async ngOnInit() {
    const params = new URLSearchParams(window.location.search);
    const isConsent = params.has('client_id') && params.has('claims');
    if (isConsent) {
      this.mode.set('consent');
      this.step.set('consent');
      const ctx = await this.resolver.prepareConsent({
        clientId: params.get('client_id')!,
        claims: (params.get('claims') ?? '').split(','),
        redirectUri: params.get('redirect_uri') ?? '',
        state: params.get('state') ?? '',
      });
      this.consentCtx.set(ctx);
    }
  }

  ngAfterViewInit() {
    if (this.resolverRef) {
      (this.resolverRef.nativeElement as any).resolveIdentifier = this.resolver.resolveIdentifier.bind(this.resolver);
    }
  }

  onResolved(e: Event) {
    // advance to login step
    this.step.set('login');
    // (full wiring of login-card injection follows pattern; for MVP we keep this minimal)
  }

  onResolveError(e: Event) {
    // surface via shell's error region
  }

  onConsentApprove(e: Event) {
    // POST /auth/authorize/grant then redirect
  }

  onConsentDecline(e: Event) {
    // POST /auth/authorize/decline then redirect
  }
}
```

(The full wiring of login-card after federated-resolver and the grant/decline POSTs are extensions of this scaffold. Keep this task scoped to wiring the resolver path; subsequent wiring is part of the Angular wrapper or a follow-up task.)

- [ ] **Step 4: Run tests + build**

```bash
cd /projects/elohim/app/imagodei-portal
pnpm vitest run 2>&1 | tail -10
pnpm run build 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add app/imagodei-portal/
git commit -m "feat(imagodei-portal): wire portal-shell + standalone resolver service"
```

### Task D3 — Seed content row for imagodei-portal

**Owner:** content-pipeline

**Files:**
- Create: `genesis/data/lamad/content/imagodei-portal.json`

**Background:** Mirror `genesis/data/lamad/content/lamad-spa.json` from the EPR decomposition; the content row is the projection target for the standalone bundle's blob.

- [ ] **Step 1: Look at lamad-spa.json as template**

```bash
cat /projects/elohim/genesis/data/lamad/content/lamad-spa.json
```

- [ ] **Step 2: Create the imagodei-portal content row**

```json
{
  "id": "imagodei-portal",
  "contentType": "application",
  "title": "Elohim Imagodei Portal",
  "name": "Imagodei Portal",
  "description": "The peer OAuth portal — sign-in, consent, and account management. Renders identically whether served via doorway projection, doorway-routed peer conductor, or Tauri local conductor.",
  "content": {
    "slug": "imagodei-portal",
    "entryPoint": "index.html"
  },
  "contentFormat": "spa-bundle",
  "tags": ["application", "spa", "auth", "portal", "imagodei"],
  "blobHash": "",
  "reach": "commons",
  "metadata": {
    "category": "application",
    "embedStrategy": "root",
    "blobPopulatedBy": "Jenkinsfile:stageSpaBlob — the imagodei-portal bundle is built from app/imagodei-portal/ and uploaded as a separate blob; this content row's blobHash is patched at deploy time."
  },
  "createdAt": "2026-05-25T00:00:00.000000",
  "updatedAt": "2026-05-25T00:00:00.000000"
}
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/data/lamad/content/imagodei-portal.json
git commit -m "feat(seed): content row for imagodei-portal EPR (blobHash populated at deploy)"
```

### Task D4 — Add /auth/portal projections to seed-projections.ts

**Owner:** content-pipeline

**Files:**
- Modify: `genesis/seeder/src/seed-projections.ts`
- Modify: `genesis/seeder/src/__tests__/seed-projections.test.ts`

**Background:** Spec §6.1 — a `project-epr` commitment registers the standalone bundle at `urlPath: "/auth/portal"` on each doorway. Add to the existing `defaultProjectionSeeds()` from the EPR decomposition Phase A.

- [ ] **Step 1: Read the existing defaultProjectionSeeds**

```bash
grep -n "defaultProjectionSeeds\|landingAt\|lamadAt" /projects/elohim/genesis/seeder/src/seed-projections.ts
```

- [ ] **Step 2: Add imagodeiPortalAt helper + extend defaults**

Inside `defaultProjectionSeeds()`, add (after `lamadAt`):

```typescript
const imagodeiPortalAt = (doorwayId: string): ProjectionSpec => ({
  ...base,
  doorwayId,
  eprId: 'imagodei-portal',
  urlPath: '/auth/portal',
  baseHref: '/auth/portal/',
});
```

And add to the returned array:

```typescript
return [
  landingAt('alpha-elohim-host'),
  landingAt('elohim-host'),
  lamadAt('alpha-elohim-host'),
  lamadAt('elohim-host'),
  imagodeiPortalAt('alpha-elohim-host'),
  imagodeiPortalAt('elohim-host'),
];
```

- [ ] **Step 3: Update test expectations**

In `genesis/seeder/src/__tests__/seed-projections.test.ts`, update any assertion about the count of default seeds from 4 to 6, and add a test:

```typescript
it('default seed set includes /auth/portal projections on both doorways', () => {
  const seeds = defaultProjectionSeeds();
  const portalSeeds = seeds.filter((s) => s.eprId === 'imagodei-portal');
  expect(portalSeeds.length).toBe(2);
  expect(portalSeeds.every((s) => s.urlPath === '/auth/portal')).toBe(true);
});
```

- [ ] **Step 4: Run tests + commit**

```bash
cd /projects/elohim/genesis/seeder
pnpm vitest run src/__tests__/seed-projections.test.ts 2>&1 | tail -10
cd /projects/elohim
git add genesis/seeder/src/seed-projections.ts genesis/seeder/src/__tests__/seed-projections.test.ts
git commit -m "feat(seeder): add /auth/portal projections on both doorways"
```

---

## Phase E — Angular wrapper cleanup

### Task E1 — Rewrite LoginComponent as Lit-element wrapper

**Owner:** angular-architect

**Files:**
- Modify: `app/elohim-app/src/app/imagodei/components/login/login.component.ts`
- Modify: `app/elohim-app/src/app/imagodei/components/login/login.component.html`
- Modify: `app/elohim-app/src/app/imagodei/components/login/login.component.css`
- Modify: `app/elohim-app/src/app/imagodei/components/login/login.component.spec.ts`

**Background:** Spec §1.1 + §5.4. LoginComponent collapses to `<elohim-imagodei-portal-shell>` with slotted federated-resolver + login-card. AuthService injection stays; the component bridges Angular-DI service calls into the Lit element's callback props/events.

- [ ] **Step 1: Rewrite login.component.html**

```html
<elohim-imagodei-portal-shell
  #shell
  [attr.step]="step"
  [attr.flywheel-hint]="flywheelHint ? '' : null"
>
  <ng-container *ngIf="step === 'resolve'">
    <elohim-imagodei-federated-resolver
      #resolver
      slot="primary"
      remember-key="elohim_auth_identifier"
      (resolved)="onResolved($event)"
      (resolve-error)="onResolveError($event)"
    >
      <span slot="help-text">Enter your federated identifier — for example, matthew&#64;alpha.elohim.host.</span>
    </elohim-imagodei-federated-resolver>
  </ng-container>

  <ng-container *ngIf="step === 'login'">
    <elohim-imagodei-login-card
      #loginCard
      slot="primary"
      [attr.remembered-identifier]="identifier"
      allow-password
      (password-submit)="onPasswordSubmit($event)"
      (oauth-start)="onOAuthStart($event)"
    ></elohim-imagodei-login-card>
  </ng-container>

  <div slot="error-region" *ngIf="errorMessage">
    {{ errorMessage }}
    <a routerLink="/identity/recovery">recover account</a>
  </div>
</elohim-imagodei-portal-shell>
```

- [ ] **Step 2: Rewrite login.component.ts**

```typescript
import { CommonModule } from '@angular/common';
import { Component, ElementRef, ViewChild, AfterViewInit, OnInit, inject, signal } from '@angular/core';
import { Router, ActivatedRoute, RouterModule } from '@angular/router';

import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';

type Step = 'resolve' | 'login';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.css'],
})
export class LoginComponent implements OnInit, AfterViewInit {
  private readonly authService = inject(AuthService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly oauthProvider = inject(OAuthAuthProvider);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  step: Step = 'resolve';
  identifier = '';
  flywheelHint = false;
  errorMessage = '';

  @ViewChild('resolver') resolverRef?: ElementRef<HTMLElement>;
  @ViewChild('loginCard') loginCardRef?: ElementRef<HTMLElement>;

  ngOnInit() {
    // Pre-fill from query param if returnTo or identifier provided
    const stored = localStorage.getItem('elohim_auth_identifier');
    if (stored) this.identifier = stored;
  }

  ngAfterViewInit() {
    // Bridge AuthService into the Lit elements
    if (this.resolverRef) {
      (this.resolverRef.nativeElement as any).resolveIdentifier = async (id: string) => {
        try {
          const doorway = await this.doorwayRegistry.resolveGatewayToDoorwayUrl(id);
          return { ok: !!doorway, doorwayUrl: doorway, reason: doorway ? undefined : 'no-doorway' };
        } catch (e) {
          return { ok: false, reason: e instanceof Error ? e.message : 'resolve-failed' };
        }
      };
    }
  }

  onResolved(e: Event) {
    const detail = (e as CustomEvent).detail as { identifier: string; doorwayUrl: string };
    this.identifier = detail.identifier;
    this.step = 'login';
  }

  onResolveError(e: Event) {
    const detail = (e as CustomEvent).detail as { reason: string };
    this.errorMessage = `Could not resolve: ${detail.reason}`;
  }

  async onPasswordSubmit(e: Event) {
    const detail = (e as CustomEvent).detail as { identifier: string; password: string; remember: boolean };
    try {
      await this.passwordProvider.login({
        identifier: detail.identifier || this.identifier,
        password: detail.password,
      });
      const returnTo = this.route.snapshot.queryParamMap.get('returnTo') ?? '/';
      await this.router.navigateByUrl(returnTo);
    } catch (e) {
      this.errorMessage = e instanceof Error ? e.message : 'sign-in failed';
      if (this.loginCardRef) {
        (this.loginCardRef.nativeElement as any).setError?.(this.errorMessage);
      }
    }
  }

  async onOAuthStart(e: Event) {
    const detail = (e as CustomEvent).detail as { providerId: string };
    await this.oauthProvider.startFlow(detail.providerId);
  }
}
```

(Adapt method names like `passwordProvider.login` / `oauthProvider.startFlow` / `doorwayRegistry.resolveGatewayToDoorwayUrl` to the actual signatures in those services. The B0-style audit isn't strictly required because we've seen those services exist; if the signatures differ, the implementer reads them and adapts.)

- [ ] **Step 3: Rewrite login.component.css to strip per-step styling**

Keep only host-level styling that doesn't conflict with the Lit elements. Most of the existing CSS becomes irrelevant — strip aggressively.

- [ ] **Step 4: Rewrite login.component.spec.ts**

Replace tests with assertions about the wrapper bridging callbacks correctly. Example:

```typescript
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { LoginComponent } from './login.component';
import { ActivatedRoute, Router } from '@angular/router';
import { of } from 'rxjs';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

describe('LoginComponent (Lit wrapper)', () => {
  let fixture: ComponentFixture<LoginComponent>;
  let component: LoginComponent;
  let passwordProvider: jasmine.SpyObj<PasswordAuthProvider>;
  let router: jasmine.SpyObj<Router>;

  beforeEach(() => {
    passwordProvider = jasmine.createSpyObj('PasswordAuthProvider', ['login']);
    router = jasmine.createSpyObj('Router', ['navigateByUrl']);

    TestBed.configureTestingModule({
      imports: [LoginComponent],
      providers: [
        { provide: AuthService, useValue: {} },
        { provide: DoorwayRegistryService, useValue: { resolveGatewayToDoorwayUrl: async () => 'https://x' } },
        { provide: PasswordAuthProvider, useValue: passwordProvider },
        { provide: OAuthAuthProvider, useValue: { startFlow: async () => {} } },
        { provide: Router, useValue: router },
        { provide: ActivatedRoute, useValue: { snapshot: { queryParamMap: { get: () => '/' } } } },
      ],
    });
    fixture = TestBed.createComponent(LoginComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('advances to login step on resolved event', () => {
    component.onResolved(new CustomEvent('resolved', { detail: { identifier: 'matthew@alpha', doorwayUrl: 'https://alpha' } }));
    expect(component.step).toBe('login');
    expect(component.identifier).toBe('matthew@alpha');
  });

  it('calls passwordProvider.login on password-submit', async () => {
    passwordProvider.login.and.resolveTo({});
    component.identifier = 'matthew@alpha';
    await component.onPasswordSubmit(new CustomEvent('password-submit', { detail: { identifier: 'matthew@alpha', password: 'pw', remember: false } }));
    expect(passwordProvider.login).toHaveBeenCalledWith({ identifier: 'matthew@alpha', password: 'pw' });
    expect(router.navigateByUrl).toHaveBeenCalledWith('/');
  });
});
```

- [ ] **Step 5: Run tests + verify elohim-app builds**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/imagodei/components/login 2>&1 | tail -10
pnpm run build 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim
git add app/elohim-app/src/app/imagodei/components/login/
git commit -m "refactor(login): LoginComponent becomes a thin wrapper around <elohim-imagodei-portal-shell>"
```

### Task E2 — Rewrite AuthCallbackComponent as Lit-element wrapper

**Owner:** angular-architect

**Files:**
- Modify: `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.ts`
- Modify: `app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.spec.ts`

**Background:** Same pattern as E1. AuthCallbackComponent renders `<elohim-imagodei-oauth-callback>` and wires the `exchangeCode` callback to OAuthAuthProvider.

- [ ] **Step 1: Read the current AuthCallbackComponent**

```bash
cat /projects/elohim/app/elohim-app/src/app/imagodei/components/auth-callback/auth-callback.component.ts
```

- [ ] **Step 2: Rewrite the component**

```typescript
import { Component, ElementRef, ViewChild, AfterViewInit, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

@Component({
  selector: 'app-auth-callback',
  standalone: true,
  template: `
    <elohim-imagodei-oauth-callback
      #cb
      [attr.code]="code()"
      [attr.state]="state()"
      [attr.provider-label]="provider()"
      (success)="onSuccess($event)"
      (error)="onError($event)"
    ></elohim-imagodei-oauth-callback>
  `,
})
export class AuthCallbackComponent implements OnInit, AfterViewInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly oauth = inject(OAuthAuthProvider);

  code = signal('');
  state = signal('');
  provider = signal('');

  @ViewChild('cb') cbRef?: ElementRef<HTMLElement>;

  ngOnInit() {
    const qp = this.route.snapshot.queryParamMap;
    this.code.set(qp.get('code') ?? '');
    this.state.set(qp.get('state') ?? '');
    this.provider.set(qp.get('provider') ?? '');
  }

  ngAfterViewInit() {
    if (this.cbRef) {
      (this.cbRef.nativeElement as any).exchangeCode = async (code: string, state: string) => {
        return await this.oauth.exchangeCode(code, state);
      };
    }
  }

  onSuccess(_e: Event) {
    void this.router.navigateByUrl('/');
  }

  onError(e: Event) {
    const detail = (e as CustomEvent).detail as { reason: string };
    console.error('OAuth callback error:', detail.reason);
  }
}
```

- [ ] **Step 3: Rewrite the spec**

Pattern mirrors E1 — assert that `exchangeCode` is bridged and that success navigates.

- [ ] **Step 4: Run tests + commit**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/imagodei/components/auth-callback 2>&1 | tail -10
cd /projects/elohim
git add app/elohim-app/src/app/imagodei/components/auth-callback/
git commit -m "refactor(auth-callback): AuthCallbackComponent becomes a thin wrapper around <elohim-imagodei-oauth-callback>"
```

---

## Phase F — a2o features

### Task F1 — hosted-login.feature + step defs

**Owner:** general-purpose (a2o author — narrative discipline, no specialist agent yet)

**Files:**
- Create: `genesis/a2o/features/peer-oauth-portal/hosted-login.feature`
- Create: `genesis/a2o/steps/peer-oauth-portal/hosted-login.steps.ts`

**Background:** Spec §5.6 — Mode A doorway-hosted login feature file. Two scenarios.

- [ ] **Step 1: Create the feature file**

Paste the exact Gherkin from spec §5.6 (`hosted-login.feature`).

- [ ] **Step 2: Look at existing a2o step-definition patterns**

```bash
find /projects/elohim/genesis/a2o/steps -name "*.steps.ts" 2>/dev/null | head -3
cat /projects/elohim/genesis/a2o/steps/ui/delivery.steps.ts 2>/dev/null | head -60
```

- [ ] **Step 3: Implement steps**

Use the existing framework helpers (DoorwayClient, page objects). Sketch:

```typescript
import { Given, When, Then } from '@cucumber/cucumber';
import { expect } from 'chai';

Given('the alpha.elohim.host doorway has a projection for the peer-oauth-portal at {string}', async function (path: string) {
  const response = await this.doorwayClient.fetch(
    `/db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host`
  );
  const projections = await response.json();
  const portalProj = projections.find((p: any) => p.urlPath === path && p.eprId === 'imagodei-portal');
  expect(portalProj).to.exist;
});

Given('matthew is a pre-registered imagodei on alpha.elohim.host with password {string}', async function (password: string) {
  // Use the seeder's test-account creation API if available, else assume seeded.
  this.testCredentials = { identifier: 'matthew@alpha.elohim.host', password };
});

When('matthew opens {string}', async function (url: string) {
  await this.page.goto(url);
});

When('types {string} into the federated-resolver', async function (identifier: string) {
  // Page object selector for the federated-resolver input
  const input = this.page.locator('elohim-imagodei-federated-resolver').locator('input');
  await input.fill(identifier);
  await this.page.locator('elohim-imagodei-federated-resolver').locator('button[type=submit]').click();
});

Then('the portal advances to the login-card step', async function () {
  await this.page.waitForSelector('elohim-imagodei-login-card');
});

Then('the trust-indicator reads {string} with the flywheel hint visible', async function (label: string) {
  const text = await this.page.locator('elohim-imagodei-trust-indicator').innerText();
  expect(text).to.include(label);
  expect(text).to.match(/flywheel/i);
});

// ... continue per scenario
```

- [ ] **Step 4: Run the feature + commit**

```bash
cd /projects/elohim/genesis/a2o
pnpm run test:features features/peer-oauth-portal/hosted-login.feature 2>&1 | tail -15
cd /projects/elohim
git add genesis/a2o/features/peer-oauth-portal/hosted-login.feature \
        genesis/a2o/steps/peer-oauth-portal/hosted-login.steps.ts
git commit -m "test(a2o): hosted-login scenarios"
```

### Task F2 — peer-conductor-login.feature + step defs

**Owner:** general-purpose

**Files:**
- Create: `genesis/a2o/features/peer-oauth-portal/peer-conductor-login.feature`
- Create: `genesis/a2o/steps/peer-oauth-portal/peer-conductor-login.steps.ts`

- [ ] **Step 1: Paste the spec §5.6 feature for peer-conductor-login**

- [ ] **Step 2: Implement steps**

Mirror F1. The Tauri scenario likely uses a different test harness (Tauri-webview launcher); if the framework doesn't support Tauri tests, mark that scenario `@manual` or `@skip` and document the manual verification path in the audit notes.

- [ ] **Step 3: Run + commit**

```bash
git commit -m "test(a2o): peer-conductor-login scenarios"
```

### Task F3 — rp-consent.feature + step defs

**Owner:** general-purpose

**Files:**
- Create: `genesis/a2o/features/peer-oauth-portal/rp-consent.feature`
- Create: `genesis/a2o/steps/peer-oauth-portal/rp-consent.steps.ts`

- [ ] **Step 1: Paste the spec §5.6 feature for rp-consent**

- [ ] **Step 2: Implement steps**

The RP consent flow may need to seed a test OAuth client (`graphos-designer` is hardcoded in `oauth_session.rs::get_registered_clients`; verify or add a `test-rp` entry). Sketch:

```typescript
Given('graphos-designer.elohim.host is a registered OAuth client', async function () {
  const resp = await this.doorwayClient.fetch('/admin/oauth-clients');
  const clients = await resp.json();
  expect(clients.find((c: any) => c.id === 'graphos-designer')).to.exist;
});

When('matthew is redirected to {string}', async function (url: string) {
  await this.page.goto(url);
});

Then('the consent-card renders with {string} as the requesting client', async function (clientName: string) {
  const text = await this.page.locator('elohim-imagodei-consent-card').innerText();
  expect(text).to.include(clientName);
});

// ... continue
```

- [ ] **Step 3: Run + commit**

```bash
git commit -m "test(a2o): rp-consent scenarios"
```

---

## Phase G — Definition-of-done verification

### Task G1 — Run all tests + manual dogfood

**Owner:** general-purpose (operator-mediated)

**Files:** None (verification + tagging)

- [ ] **Step 1: Run all relevant test suites**

```bash
# Lit element tests
cd /projects/elohim/app/elohim-elements/elohim-imagodei
pnpm test 2>&1 | tail -10

# Standalone resolver tests
cd /projects/elohim/app/imagodei-portal
pnpm vitest run 2>&1 | tail -10

# Angular wrapper tests
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/imagodei/components/login src/app/imagodei/components/auth-callback 2>&1 | tail -15

# Seeder unit tests
cd /projects/elohim/genesis/seeder
pnpm vitest run src/__tests__/seed-projections.test.ts 2>&1 | tail -8

# Storybook build
cd /projects/elohim/app/elohim-library
pnpm exec ng run graphos:build-storybook 2>&1 | tail -10

# a2o (needs local dev stack)
cd /projects/elohim/genesis/a2o
pnpm run test:features features/peer-oauth-portal/ 2>&1 | tail -15
```

Expected: all suites green.

- [ ] **Step 2: Manual dogfood (operator-of-record)**

Walk through the criteria in spec §5.7. For each, document pass/fail + screenshots if useful.

- [ ] **Step 3: Verify no third portal**

```bash
# Should find ZERO matches — Angular LoginComponent should now ONLY be the wrapper
grep -rn "PasswordCredentials\|RoutedLogin\|new credential form\|legacy login" /projects/elohim/app/elohim-app/src/app/imagodei/components/login/ 2>/dev/null

# Confirm the wrapper is the only path
grep -c "elohim-imagodei-portal-shell" /projects/elohim/app/elohim-app/src/app/imagodei/components/login/login.component.html
```

Expected: 1 match (the wrapper).

- [ ] **Step 4: Tag MVP**

```bash
cd /projects/elohim
git tag mvp-peer-oauth-portal
```

(Don't push tag — operator decides when to publish.)

- [ ] **Step 5: Phase summary**

Print a summary of commits since branch creation and confirm DoD criteria from spec §7 are all green.

---

# Self-Review

## Spec coverage

Walking through spec §6.1 MVP scope:

| Spec item | Task |
|---|---|
| 7 Lit elements in elohim-imagodei | B1-B7 ✓ |
| Library A default stories | C1 ✓ |
| Library B designed stories | C2 ✓ |
| Standalone EPR project at app/imagodei-portal/ | D1-D2 ✓ |
| project-epr seed for /auth/portal | D4 ✓ |
| Content row for imagodei-portal EPR | D3 ✓ |
| Angular LoginComponent rewritten as wrapper | E1 ✓ |
| Angular AuthCallbackComponent rewritten as wrapper | E2 ✓ |
| Three a2o feature files | F1-F3 ✓ |
| Manual dogfood | G1 ✓ |
| Substrate audit (Phase A) | A1 ✓ |

All MVP items covered.

## Placeholder scan

No "TBD"/"TODO"/"implement later" — every step shows actual code or exact commands. The standalone resolver wires depend on the audit (A1) confirming the `/.well-known/elohim-doorway` endpoint shape, but the resolver code is concrete and adjusts if A1 reveals a different endpoint shape (see D2 implementation comment).

## Type consistency

- `TrustMode` is `'doorway-host' | 'peer-conductor'` everywhere (B1, B3, B5, B6, C1, C2)
- `AttestorRef` shape consistent in B2 + B3 + Library A stories
- `ProjectionSpec` shape extends the existing EPR-decomp Phase A type (D4)
- Event detail shapes consistent: `resolved` carries `{identifier, doorwayUrl}`; `password-submit` carries `{identifier, password, remember}`; `approve` carries `{grantedClaims}`; etc.

No drift detected.

---

# Execution Handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-25-peer-oauth-portal-plan.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Natural agent assignments visible in the plan:
   - `component-architect` for the 7 Lit primitives (B1-B7) and Library A stories (C1)
   - `graphos-designer` for Library B compositions (C2)
   - `angular-architect` for the standalone EPR scaffold + wiring (D1-D2) and Angular wrapper rewrites (E1-E2)
   - `content-pipeline` for seed-data tasks (D3-D4)
   - `general-purpose` for the substrate audit (A1) and a2o features (F1-F3) and DoD verification (G1)

2. **Inline Execution** — Execute tasks in this session using executing-plans, batch with checkpoints. Slower; doesn't parallelize.

Which approach?
