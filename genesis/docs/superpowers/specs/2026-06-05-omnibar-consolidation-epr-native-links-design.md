---
title: "Omnibar Consolidation + EPR-Native Links — trust surface, cross-bundle navigation, theme/lang controls"
id: omnibar-consolidation-epr-native-links-design
status: Draft
class: ui-truth-layer
domain: D-epr-apps
topic: [omnibar, protocol-omni, page-chrome, epr-link, navigation, debug-bar, serving-context, theme, locale, i18n, a11y, ua-prefs, elohim-core, lamad, resilience, hypercard, context-menu, tooltip]
cites:
  - pillar-epr-decomposition-design | parent canon — omnibar slotted contract, EPR-link HyperCard semantics (§7.3–7.4 context-menu derivation + card-flip-in-place this spec's §11 resilience hypercard speaks), and the §12 URL-routing amendment | sha256:8029079cea758380 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - elohim-sdk-epr-app-boundaries-sprint-kickoff | SDK-boundary canon whose app-bootstrap/app-manifest workstreams this chrome work composes with — Tier-5 delivery-seam vocabulary and the manifest-as-canonical constraint | sha256:9776d193efcabc84 | path: genesis/docs/superpowers/specs/2026-05-30-elohim-sdk-epr-app-boundaries-sprint-kickoff.md
  - sprint-1b-library-b-design | theme-binding discipline this spec honors — brand tokens bind at story-decorator level only, so the new toggle/picker elements stay blank-slate primitives | sha256:0d1ca1c9f6d09e92 | path: genesis/docs/superpowers/specs/2026-05-22-sprint-1b-library-b-design.md
  - session-bridge-design | its deferred ephemeral-UI-preferences question is where the person-level preference-sync follow-up (§9.1) eventually lands | sha256:1d52dbaa44affce5 | path: genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md
  - cradle-to-grave-capability-gradient | cradle-to-grave-capability-gradient | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
---

# Omnibar Consolidation + EPR-Native Links

EPR-app **loading** works between the elohim protocol landing and lamad; **navigation
between** EPR-apps does not. This spec heals the cross-bundle link regression and
consolidates the chrome layer around the omnibar as a **trust surface** — the
protocol's address-bar equivalent — absorbing the debug-bar's concerns, restoring the
theme toggle lamad lost in the Angular→Lit migration, and adding an opt-in language
picker. Accessibility preferences deliberately stay UA-native.

Design session: 2026-06-05. Parent canon: the pillar-EPR-decomposition design
(omnibar slotted contract, EPR-link HyperCard semantics, §12 URL-routing amendment —
"no bundle-absolute routes in shared code", `/epr/{id}` universal resolver).

## 1. Settled decisions (session record, 2026-06-05)

| Decision | Verdict |
|---|---|
| Cross-bundle link mechanism | **Sweep + interceptor.** All 21 first-party cross-bundle links rewritten to EPR-native forms AND a capture-phase document click interceptor in elohim-core catches leftover/content-authored bundle-absolute anchors. Markdown-authored content will always produce plain `<a href>` — the safety net keeps paying rent. |
| Cross-bundle anchor form | **Plain `href` (the projected EPR address) + interceptor handoff**, not `<elohim-epr-link>` everywhere. EPR-link's HyperCard card-flip is for content resolution; crossing a bundle boundary requires a bundle boot, and the doorway-projected URL *is* the EPR-native address. `routerLink` is reserved for same-bundle routes. |
| Debug-bar | **Deprecated and deleted.** Its concerns (env identity, log level, backend-config affordance) lift into `app-protocol-omni` as an **optional, trust-framed env-context segment** (see §5). |
| Env segment posture | **Opt-in (`showEnvContext`, default `false`), trust-framed.** Renders inside the EPR group, adjacent to the identifier, contextualizing *this EPR view* ("you're viewing this EPR through the alpha environment"). Prod or opt-out → renders nothing. The trust surface never cries wolf. |
| Serving-context dimension | **`ServingContext` — a dimension orthogonal to reach** capturing the system state an EPR is projected through: `{ tier, logLevel, buildId, variant? }`. The env segment renders this interface, not hardcoded fields. `buildId` = CI-substituted `gitHash` (already real, `Jenkinsfile:761-763`). `variant` is reserved — EPR-natively a variant is *which `project-epr` commitment / bundle CID served you* (blue/green & A/B legible without k8s vocabulary). Substrate home (doorway headers sibling to `X-Reach`; projection variant metadata) is captured follow-up §9.7. |
| Theme plumbing | **Shared theme store in elohim-core speaking the exact `ThemeService` contract** (`localStorage['elohim-theme']`, `body[data-theme]` + `theme-*` class, device→light→dark cycle). Any toggle anywhere stays in sync across bundles and tabs. |
| Theme toggle surfaces | **Omnibar opt-in + navigator restore.** `<elohim-default-omnibar show-theme-toggle>` (default off — landing keeps its floating toggle); `<elohim-navigator>` regains the toggle dropped by the Lit migration (profile tray + visitor inline); `app-protocol-omni [showThemeToggle]` opt-in, default off. |
| Language vs a11y split | **Language gets an omnibar opt-in picker; a11y stays UA-native.** The web platform has no per-site language switcher and `@lit/localize` is wired but unreachable; conversely a11y signals (reduced-motion, contrast, color-scheme, font scaling) are UA-owned AND already enforced by the ua-prefs element gate. Deep per-person a11y overrides are the settings-palette placeholder's job (follow-up). |
| Person-level preference sync | **Captured follow-up, not this design.** Theme/locale following the human across devices via imagodei would be Category B (private source-chain entry) and must re-run the p2p-design-gate. |

## 2. P2P design gate record

No DHT, diesel, or doorway-route changes anywhere in this design — UI truth-layer only.

| Entity | Class | Source of truth | Notes |
|---|---|---|---|
| ThemePreference | Operational (C) | `localStorage['elohim-theme']` (device-scoped) | Deliberately device-level: appearance follows the device/OS, matching existing `ThemeService`. Reconstruction = default `device` + UA signal. |
| LocalePreference | Operational (C) + existing field | `localStorage['elohim-locale']`; `SessionHuman.locale` when a session exists | No new entity — we *set* a `SessionHuman` field that exists today and is never written. BCP-47 codes; lit-localize registry is canonical. |
| CrossBundleNavHandoff | Operational (C) | sessionStorage (extends `elohim.session-nav-stack.v1`) | Back-affordance survives the bundle boundary. Reconstruction = empty stack (cosmetic loss). |

`ServingContext` (§5) is a read-only view-model over existing build/config values —
nothing persisted, no entity. Its eventual substrate home (doorway provenance
headers, projection variant metadata — §9.7) re-runs this gate when designed.

## 3. Architecture overview

```
                     ┌────────────────────────────────────────────────────┐
                     │                 elohim-core (Lit)                  │
                     │  NEW: epr-link-interceptor    theme/theme-store    │
                     │       <elohim-theme-toggle>   <elohim-lang-picker> │
                     └───────┬───────────────────────────────┬────────────┘
            auto-installed by│page-chrome         consumed by│omnibar/navigator
        ┌────────────────────┴────────────┐    ┌─────────────┴────────────────┐
        │ elohim-app (landing + pillars)  │    │ lamad bundle                 │
        │  protocol-omni: +env context    │    │  <elohim-page-chrome>        │
        │   (opt-in) +theme toggle opt-in │    │  <elohim-navigator>:         │
        │  debug-bar: DELETED             │    │   theme toggle RESTORED      │
        │  21-link sweep + EprNavService  │    │   +lang picker (tray)        │
        │  explicit interceptor install   │    │  default-omnibar:            │
        │   (router-aware ownsPath)       │    │   opt-in toggle attributes   │
        └─────────────────────────────────┘    └──────────────────────────────┘
```

One new shared seam (elohim-core primitives), two consumers, zero backend changes.
Everything rides the omnibar contract and vocabulary already ratified in the
pillar-EPR-decomposition design.

## 4. EPR-native cross-bundle links (sweep + interceptor)

### 4.1 The regression

The landing and lamad are separate Angular bundles (`<base href="/">` vs
`<base href="/lamad/">`) dispatched by doorway's `EprRouter`. A
`routerLink="/lamad"` inside elohim-app asks router tree A to match `lamad` — no
such route exists → `**` catch-all → 404 component. `app/app.routes.spec.ts:18`
documents this as the tracked regression ("lamad-bundle-only until Slice 2 /epr
resolver"). Inventory: **21 cross-bundle links across 14 files** (footer,
not-found ×3+1, profile ×3, community-home ×2, shefa-home, device-stewardship,
presence service ×2, tauri-auth, debug-bar, doorway-app ×2, navigator context
apps).

### 4.2 Interceptor — `elohim-core/src/navigation/epr-link-interceptor.ts` (new)

`installEprLinkInterceptor({ ownsPath?, beforeCrossBundle?, explicit?, assign? })`:

- **Capture-phase** `document` click listener — fires *before* Angular's
  `routerLink` target-phase handler, so it beats the 404 even on un-swept anchors.
- Walks `e.composedPath()` for the anchor (shadow-DOM-safe). Passes through
  untouched: modified clicks (ctrl/cmd/shift/alt/middle), `target="_blank"`,
  `download`, external origins, hash-only, `data-epr-bypass`.
- `ownsPath(path): boolean` supplied by the host. elohim-app: checks its actual
  route config (`'' | community | shefa | identity | account | doorway | avodah |
  resource | deliver | resolve | epr`). lamad: `/lamad/` base-href prefix check.
  Cross-bundle ⇔ `!ownsPath(path)`.
- Same-bundle anchors always pass through untouched — Angular `routerLink` (or
  default browser behavior) owns them. Implementation refinement 2026-06-05: the
  same-bundle upgrade hook was dropped because routerLink-managed anchors can't be
  reliably distinguished from plain anchors in prod builds; interception is
  cross-bundle-only, which is the regression being healed.
- Cross-bundle anchor → write nav-handoff record (source CID, scroll, timestamp
  into the session-nav-stack), then `window.location.assign(href)` — the full load
  through doorway **is** EPR-native navigation; the URL is the projected EPR
  address.
- Idempotent singleton (`window.__elohimEprLinkInterceptor` guard); returns an
  uninstall handle. **Fails open**: any internal error → default browser behavior.
- Auto-installed by `<elohim-page-chrome>` `connectedCallback` with the base-href
  prefix heuristic as default `ownsPath` (lamad gets it free); elohim-app installs
  explicitly in `app.component` with router-aware callbacks (page-chrome is not in
  its template; explicit install wins over the heuristic via the singleton guard).

### 4.3 Sweep — all 21 sites

| Form today | Becomes |
|---|---|
| Cross-bundle template `routerLink` (footer, not-found ×3, profile ×2, community-home ×2, shefa-home, device-stewardship) | Plain `href` — interceptor-handled; no router race; real URLs; middle-click/copy-link work |
| Programmatic `router.navigate(['/lamad'…])` (presence ×2, profile, tauri-auth, not-found) | `EprNavService.navigate(path)` (new, elohim-app): `ownsPath ? router.navigate : cross-bundle handoff` |
| `elohim-navigator` `navigate` event handlers in hosts | Route through the same `EprNavService` |
| doorway-app plain `href` ×2 | Already correct — verify only |
| debug-bar `routerLink="/doorway/elohim"` | Dies with debug-bar; the env segment uses `routerLink` (intra-bundle — `doorway/*` routes live inside elohim-app) |

`app.routes.spec.ts` tracked-regression assertions update to the new contract
(cross-bundle paths intentionally absent from the route tree AND covered by
`ownsPath` exclusion).

### 4.4 State handoff

Pre-navigation, the interceptor/EprNavService appends to the existing
`elohim.session-nav-stack.v1` sessionStorage stack (same-origin, survives the
bundle boot) so protocol-omni's back affordance works across the boundary.
Session identity already rides the `elohim_session` cookie — no work needed.

## 5. Serving context — optional, trust-framed segment of the EPR group

The omni-toolbar is the protocol's address bar. Reach answers *how far the content
extends* (audience dimension); resilience answers *how well it is stewarded*
(custody dimension, ◉ placeholder); this section adds a third, **orthogonal
dimension**: **serving context** — *through what system state is this EPR being
projected to you?* Environment tier, log level, build identity, and deployment
variant are facets of that one dimension, not separate ornaments.

### 5.1 The interface (view-model only)

```ts
export interface ServingContext {
  tier?: 'development' | 'alpha' | 'staging' | 'production'; // system level
  logLevel?: string;   // diagnostic verbosity
  buildId?: string;    // short gitHash today → bundle CID when the substrate header lands
  variant?: string;    // RESERVED — which projection served you (blue/green, A/B)
}
```

Sourced today from `AppConfig`: `ConfigService` gains a `gitHash` passthrough —
the value is already CI-substituted into the environment files
(`Jenkinsfile:761-763`); prod mode adds it to `/assets/config.json`. `variant`
has no source yet: at the protocol layer a variant is *which `project-epr`
commitment / bundle CID served you* — two live variants are two projection
commitments over different bundle addresses — so the facet fills from substrate
provenance when §9.7 lands, never from k8s deployment vocabulary.

### 5.2 The segment

- `app-protocol-omni` gains `showEnvContext = input<boolean>(false)` — **opt-in,
  default off**. The elohim-app shell template enables it (the surface where
  debug-bar lived). Other consumers never see it.
- When enabled AND `tier ∈ {development, alpha, staging}`: present facets render
  **inside the EPR group, adjacent to the identifier** —
  `EPR elohim-host-landing · alpha · 3cc9da6` — as a button
  (`data-testid="protocol-omni-env"`) whose `title`/`aria-label` enumerates every
  present facet in trust language: "You're viewing this EPR through the **alpha**
  environment · build 3cc9da6 · log: debug — backend details". Click →
  `/doorway/elohim` (intra-bundle `routerLink`).
- Collapsed chip gets an amber ring (`--omni-env-ring`) only when enabled +
  non-prod.
- Production, or opt-out: **nothing renders, no tint** — the trust surface never
  cries wolf. (Relaxing `buildId` to show on prod is deliberately deferred until
  the substrate header makes it doorway-attested rather than self-reported.)
- `debug-bar` component deleted: component files + spec, `home.component.html`
  usage + import, home's render test. Its three env-gating unit tests migrate to
  `protocol-omni.component.spec.ts`.

## 6. Theme — shared store, three surfaces

### 6.1 `elohim-core/src/theme/theme-store.ts` (new)

Singleton speaking *exactly* the Angular `ThemeService` contract so the two
implementations cannot disagree:

- States `device | light | dark`; cycle `device→light→dark→device`.
- Persists `localStorage['elohim-theme']` (same key); applies `body[data-theme]`
  AND `theme-light|theme-dark|theme-device` class (same attrs).
- Listens: `storage` event (cross-tab) + dispatches/listens
  `elohim-theme-changed` composed CustomEvent on `window` (cross-island, same
  page).
- Silent on storage failures (matches ThemeService posture).

`ThemeService` (Angular) gets a small patch: subscribe to `storage` +
`elohim-theme-changed` so Angular-side state follows Lit-side toggles. Captured
follow-up: collapse ThemeService onto the store entirely.

### 6.2 `<elohim-theme-toggle>` (new, elohim-core)

Blank-slate cycle button (sun/moon glyph + auto indicator), parts
(`button`, `icon`, `auto-indicator`), `--elohim-theme-toggle-*` cssprops,
capability tags, passes the three precondition gates (a11y/i18n/ua-prefs).
Library A default stories (Unstyled + CustomTheme + states); Library B designed
binding is graphos-designer's lane (follow-up story task).

### 6.3 Surfaces

| Surface | Change | Default |
|---|---|---|
| `<elohim-default-omnibar>` | `show-theme-toggle` boolean attribute | **off** — landing keeps its floating toggle; concern already handled there |
| `<elohim-navigator>` | Toggle **restored**: profile tray row (authed) + inline visitor toggle — heals the `8ce50c4e2` migration drop of `3a4b9613e` | on (it's a restore, matching pre-migration behavior) |
| `app-protocol-omni` | `showThemeToggle = input<boolean>(false)` rendering `<elohim-theme-toggle>` | **off** |

## 7. Language picker + a11y posture

### 7.1 `<elohim-lang-picker>` (new, elohim-core)

- Locale list from the lit-localize registry (`en` source; `es`, `he` targets) —
  never hardcoded elsewhere.
- On select: `setLocale()`, `document.documentElement.lang` + `dir` (he → rtl),
  persist `localStorage['elohim-locale']`, optional host callback
  (`locale-changed` event) for Angular to set `SessionHuman.locale`.
- First-run default: `navigator.language` matched against target locales, else
  `en`. Locale bundle load failure → fall back to `en`.
- Blank-slate, parts + cssprops, capability tags, three gates. Library A stories.
- Surfaces: `<elohim-default-omnibar show-lang-picker>` (default **off**),
  `<elohim-navigator>` profile tray.
- **Honest caveat**: Angular template strings are not localized yet — the picker
  initially governs Lit-element strings + document lang/dir. Angular surface
  localization is a captured follow-up.

### 7.2 A11y: no new toggles

UA signals stay authoritative — `prefers-reduced-motion`, `prefers-contrast`,
`prefers-color-scheme`, font scaling are browser/OS-owned and already enforced
across elements by the ua-prefs precondition gate. Tauri's webview exposes the
same signals; no Tauri-specific path needed. Deep per-person overrides land later
in the `elohim-imagodei-settings-palette` "Language / accessibility" placeholder
(#10), reachable from the profile tray — captured follow-up, person-level sync
gated by p2p-design-gate re-run.

## 8. Testing & error handling (story-first)

a2o scenarios committed with the implementation:

| Scenario | Feature file |
|---|---|
| Learner clicks "📚 Lamad" on the landing footer and arrives — no 404; back affordance returns across the boundary | `features/browser/navigation-browser.feature` (extend) |
| Serving context: default-off · alpha-enabled presentation (EPR-adjacent; tier + buildId + logLevel facets enumerated) · prod-silent | `features/protocol/protocol-omni.feature` (extend) |
| Theme choice persists across the app boundary (proves shared-key contract) | new `features/elohim-core/chrome-preferences.feature` |
| Switching to Hebrew flips chrome RTL and persists | same file |

Unit coverage: interceptor (modified-click passthrough, capture beats routerLink,
`ownsPath` branching, idempotent install, fails-open), ThemeStore
(cycle/persist/apply/cross-tab/cross-island), both new elements through the three
precondition gates (axe; RTL/logical properties; no-transition), protocol-omni
serving-context gating ×4 (development/alpha/staging/prod) + opt-out + facet
enumeration, EprNavService branching, navigator restored-toggle rendering (tray +
visitor).

Error posture: interceptor never blocks default navigation on error; storage
failures silent; locale bundle failure → `en`.

## 9. Out of scope / captured follow-ups

1. **Person-level preference sync via imagodei** (theme/locale/a11y follow the
   human) — Category B source-chain entry; re-run p2p-design-gate; settles where
   the session-bridge spec's "ephemeral UI preferences" question points.
2. **Settings-palette deep a11y overrides** (text size, contrast, motion
   overrides) — fills placeholder #10.
3. **Angular surface localization** (Angular i18n or runtime translation).
4. **ThemeService → ThemeStore collapse** (single implementation).
5. **Library B designed stories** for `<elohim-theme-toggle>` /
   `<elohim-lang-picker>` (graphos-designer lane).
6. **Resilience indicator wiring** (`elohim-resilience-snapshot` into
   protocol-omni) — ~~pre-existing placeholder, untouched here~~ **landed**
   (icon density, lazy household-snapshot fetch) and **extended by §11**
   (tooltip-direction fix + resilience hypercard).
7. **ServingContext substrate home** — doorway emits serving-context provenance
   headers sibling to `X-Content-Address`/`X-Reach` (e.g. `X-Build-Id`,
   `X-Bundle-Address`, `X-Variant` from projection-commitment metadata);
   `EprProjectionView` extension re-runs the p2p-design-gate. The omnibar
   `buildId` then upgrades from self-reported `gitHash` to doorway-attested
   bundle address (and may earn prod visibility); `variant` gains its source.
   rust-architect lane.
8. **Canonical bundle-styling token artifact** — the 2026-06-05 styling audit found
   the lamad split deferred token wiring (B18): tokens harvested into
   `elohim-core/tokens.scss` but imported nowhere, `--elohim-*` chrome bindings
   Storybook-only. Interim fix shipped with this work (lamad `styles.scss` imports
   tokens + binds the chrome surface). The canonical layer — shippable
   graphos-tokens artifact + runbook §4.X bundle-styling contract — is captured in
   `genesis/data/timeline/backlog/bundle-styling-token-contract.md`.

## 10. File inventory

**New** — `elohim-core/src/navigation/epr-link-interceptor.ts`,
`elohim-core/src/theme/theme-store.ts`, `elohim-core/src/elohim-theme-toggle.ts`,
`elohim-core/src/elohim-lang-picker.ts` (+ specs, + Library A stories, + registry
entries), `app/elohim-app/src/app/elohim/services/epr-nav.service.ts` (+ spec),
`app/elohim-app/src/app/elohim/models/serving-context.model.ts`,
`genesis/a2o/features/elohim-core/chrome-preferences.feature`.

**Modified** — `elohim-page-chrome.ts` (auto-install), `elohim-default-omnibar.ts`
(two opt-in attributes), `elohim-navigator.ts` (toggle restore + tray lang),
`protocol-omni.component.{ts,html,css,spec}` (serving-context segment + theme
opt-in), `config.service.ts` (+`gitHash` in `AppConfig`; prod `config.json`
shape), `theme.service.ts` (sync listeners), `app.component.ts` (interceptor
install), 14 sweep files, `app.routes.spec.ts`, `navigation-browser.feature`,
`protocol-omni.feature`.

**Deleted** — `components/debug-bar/*` (4 files) + its usage in
`home.component.{html,ts,spec}`.

## 11. Resilience hypercard — progressive disclosure from the omni resilience segment

Design session: 2026-06-06. Follow-up 6 landed after this spec was drafted
(protocol-omni renders `<elohim-resilience-snapshot density="icon">`, lazily fed
by `GET /api/v1/resilience/{id}/household`). Two defects/gaps in that landing:

1. **Tooltip clips above the viewport.** The icon-density hover tooltip is
   hard-coded `bottom: 125%` (always flips up); protocol-omni is fixed to the
   top viewport edge (`inset: 0 0 auto 0`), so on desktop the tooltip renders
   off-screen. Convention (matching `distribution-badge`'s `top: 100%; left: 0`):
   top-chrome affordances fold **down**, inline-start-aligned — down-right in
   LTR, down-left in RTL (logical properties; `he` is a claimed locale).
2. **The resilience unit has no progressive path.** `elohim-resilience-snapshot`
   already declares the density ladder (`icon → context → full`) but no
   interaction walks it. Meanwhile the pillar-EPR-decomposition design
   (§7.3–7.4) gives the protocol its HyperCard idiom — `<elohim-context-menu>`,
   menu items derive from the EPR being looked at, cards flip in place. The
   resilience icon should speak that idiom.

### 11.1 Decisions (session record, 2026-06-06)

| Decision | Verdict |
|---|---|
| Tooltip fate | **Keep** as the zero-click glance (L1), repositioned downward/inline-start. The existing a2o tooltip step stays green; coarse pointers rely on the click panel. |
| Click affordance | Icon click folds down a **hypercard panel**: context-density body + action row. Progressive: tooltip (L1) → context panel (L2) → full card (L3, in-place flip). |
| "View full resilience" | **In-place card flip** context→full inside the panel (HyperCard semantics; no full-resilience route exists and none is needed). Closing the panel resets to context density. |
| Primitive | New blank-slate **`<elohim-hypercard-panel>`** in elohim-core — a generic anchored fold-down surface (default slot for content + `ContextMenuItem[]` action row), sibling of `<elohim-context-menu>` and reusing its item type + fold-down motion. Component-architect lane: capability tags, four gates, Library A stories. |
| Boundary | Per pillar-EPR-decomposition §7.4 discipline: the Lit primitive renders slotted content + items and emits intent (`action-select`); *derivation* (which actions this EPR affords) is host-side; `resilience-snapshot` (Angular) owns the density flip; protocol-omni binds chrome theming via cssprops. |
| Epic E actions (Steward this content / View network) | **Not wired here** — destinations don't exist yet. The `actions` input + `actionSelect` output carry the contract (mirrors epr-link's `contextMenuItems` / `epr-menu-select`); wiring is captured follow-up §11.6. |

### 11.2 `<elohim-hypercard-panel>` (new, elohim-core)

- Props: `open` (boolean, reflected), `actions: ContextMenuItem[]` (type imported
  from `elohim-context-menu.ts`), `panelLabel: string` (the dialog's
  `aria-label`; host-supplied — the element ships no built-in strings).
- Default slot: arbitrary hypercard content.
- Action row: renders below the slot when `actions` is non-empty; each action
  button carries `part="action"` and `data-action-id="${item.id}"` (the
  shadow-piercing test seam).
- Events: `action-select` `{ detail: { id } }` (bubbles, composed), `close`.
- Positioning convention: `:host([open])` is `position: absolute;
  inset-block-start: calc(100% + 0.25rem); inset-inline-start: 0` — assumes a
  `position: relative` anchor wrapper, same `.menu-anchor` convention as
  epr-link. Folds **down + inline-start aligned** by construction.
  **End-edge amendment (2026-06-12):** the inline pin is the host's call —
  `align="end"` (reflected prop, default `start`) swaps to
  `inset-inline-end: 0` so a panel anchored in END-side chrome grows INTO the
  viewport (start-pinned from the omni resilience segment it projected ~173px
  off a 390px phone). Same fold-down convention, host-chosen inline side;
  min/max-width defaults clamp to `calc(100vw - 1rem)`. `<elohim-context-menu>`
  carries the same `align` prop, OPT-IN only (absent = static position for
  existing epr-link geometry).
- A11y: non-modal `role="dialog"` + `aria-label`; Escape closes; click-outside
  closes; focus moves into the panel on open and restores to the previously
  focused element on close (same discipline as `<elohim-context-menu>`).
- Motion: the `elohim-menu-fold-down` idiom (120ms ease-out), gated on
  `prefers-reduced-motion: no-preference` and `update: fast`.
- cssprops: `--elohim-hypercard-bg/border/radius/shadow/min-width/max-width/z`,
  system-color defaults (`Canvas`/`CanvasText`); `forced-colors` support.
- Four precondition gates + Library A default stories
  (`Default/Core/elohim-hypercard-panel`: Standard, WithActions, Unstyled,
  CustomTheme).

### 11.3 `elohim-resilience-snapshot` changes (elohim-library)

- **Tooltip fix**: `bottom: 125%` → below the icon (`top: calc(100% + 0.35rem)`);
  centered transform → `inset-inline-start: 0`; `white-space: nowrap` →
  wrapped at a sane `max-inline-size` so it cannot run off the viewport edge.
- Icon density becomes interactive: the icon `<span>` becomes a `<button>`
  (`aria-haspopup="dialog"`, `aria-expanded`, existing `aria-label` retained).
  Click toggles the hypercard panel. The **neutral glyph stays non-interactive**
  (no snapshot → nothing to show; the trust surface never fakes).
- Panel content: the context-density body (markup extracted to a shared
  `ng-template` so the `context` density and the panel render one source);
  built-in default action `{ id: 'view-full', label: 'View full resilience' }`
  flips the panel body to the full-density card in place.
- New `@Input() actions` (host-injected extras, appended after the built-in) and
  `@Output() actionSelect` (re-emits every selection id, built-in included —
  mirrors epr-link's re-emit contract).
- `CUSTOM_ELEMENTS_SCHEMA` + `import 'elohim-core/register'`
  (`elohim-core: workspace:*` is already a declared dependency).

### 11.4 protocol-omni binding

`protocol-omni.component.css` binds the panel's cssprops to the chrome vars
(`--elohim-hypercard-bg: var(--omni-bg)` etc. on `.omni-resilience`) so the
fold-down matches the toolbar in both schemes — custom properties cascade into
the Lit shadow root. No host actions injected yet (§11.6).

### 11.5 Testing (story-first)

a2o (extends `features/resilience/observable-distribution.feature`'s omni
section): tooltip folds down fully inside the viewport (`@regression` anchor for
the flip-up bug); icon click folds down the hypercard naming the stewarding
collective count; "View full resilience" flips the card in place with the URL
unchanged; Escape closes and restores focus to the icon.

Unit: hypercard panel mirrors the context-menu spec structure (open/close,
slot + action rendering, `action-select`, Escape/click-outside, focus restore,
axe strict, i18n logical properties, ua-prefs no-motion, theme-contrast system
cells). Angular: icon `aria-expanded` toggle, panel render on click, view-full
flip, `actionSelect` re-emit, neutral-glyph non-interactivity.

### 11.6 Captured follow-ups

1. **Epic E action wiring** — "Steward this content" / "View network" items for
   the omni resilience hypercard once their destinations exist (derivation
   host-side per §7.4; protocol-omni injects via `actions`).
2. **Lit migration of `elohim-resilience-snapshot`** — the Angular component is
   the last non-Lit piece of the omni's segments; migrating it would let the
   lamad bundle drop the Angular dependency for this surface
   (component-architect + angular-architect lanes).
3. **Tooltip → hypercard convergence** — once the panel has telemetry, consider
   whether the hover tooltip should render the same context body at a glance
   (one source, two stimuli) instead of the concatenated summary string.
