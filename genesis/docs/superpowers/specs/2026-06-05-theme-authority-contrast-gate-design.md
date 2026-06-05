---
title: "Theme Authority + Theme-Contrast Gate — html-rooted mode propagation, color-scheme sync, and the executable fourth precondition gate"
id: theme-authority-contrast-gate-design
status: Draft
class: protocol-canonical
domain: D-epr-apps
topic: [theme, dark-mode, color-scheme, contrast, wcag, a11y-gate, tokens, chrome-binding, elohim-core, lamad, omnibar, navigator, theme-store, wtr, capability-contract]
refines: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
cites:
  - omnibar-consolidation-epr-native-links-design | the spec this refines — its B18 token wiring (§6 theme surfaces, §9.8 styling audit) is the regression trigger; its §9 follow-ups absorb the ThemeService-collapse + graphos-tokens seams | sha256:71ad45eb5993b56c | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
  - genesis/data/timeline/backlog/bundle-styling-token-contract.md
  - elohim-elements-ui-substrate-gospel | the gospel this amends — three precondition gates become four (theme-contrast), and §Layer rails store-contract line moves to html[data-theme] authority | sha256:84cff1a46650cf8f | path: app/elohim-elements/CLAUDE.md
  - elohim-library-pattern-gospel | Library A/B boundary the gate honors — Dark stories stay eyeball canaries; their assertion-backed twins are the gate cells, bindings live in test fixtures only | sha256:94b851810ce6cdc8 | path: app/elohim-library/CLAUDE.md
  - lamad-bundle-gospel | the consumer bundle whose rails change — styles import set gains base.scss; _chrome-binding.scss is the shipped binding the tokens-cells inject as fixture | sha256:5b547c63cc0c1a2c | path: app/lamad/CLAUDE.md
informed-by: [app/elohim-elements/CLAUDE.md]
---

# Theme Authority + Theme-Contrast Gate

Dark mode regressed on alpha `1.0.0-dev-60cc0846` after B18 token wiring: the lamad bundle began
importing `elohim-core/tokens.scss` + `app/lamad/src/_chrome-binding.scss`, moving the Lit chrome
from UA-correct system colors onto bound `--lamad-*` tokens. The operator reported unreadable text
on `<elohim-navigator part="identity-section">` and `<elohim-default-omnibar>`, the chrome **not
following the theme toggle** while content below it shifts, and a ~8px gutter wrapping the lamad
viewport. This spec root-causes all three (four distinct failure classes + one bundle gap), fixes
each at its §Layer-rails home, and makes the failure class **un-shippable** by promoting a fourth
precondition gate — `theme-contrast` — alongside a11y/i18n/ua-prefs at the element-spec layer.

## 1. Root cause — four failure classes (all verified)

### C1 — Frozen var-chain (chrome ignores the toggle)

CSS custom properties substitute `var()` references **at computed-value time on the element where
the declaration is specified** (CSS Variables L1 §3.1), not at the consuming element.

- `_chrome-binding.scss` declares `--elohim-nav-bg: var(--lamad-bg-secondary)` (and 16 siblings)
  **on `:root`** → substitution happens at `html`, against html's `--lamad-*` values (the dark
  defaults, or the `prefers-color-scheme: light` media block).
- The manual theme override redefines `--lamad-*` **on `body[data-theme]`** — a *descendant*.
  Descendants inherit the chrome props **already substituted**; the body-level override never
  reaches them.
- Angular components below consume `var(--lamad-*)` **directly**, so substitution happens at each
  consuming element inside body → the override applies → content flips while chrome stays frozen.

The binding file's own header comment ("Custom-property chains resolve at the consuming element,
so body[data-theme] overrides of --lamad-* flow through automatically") asserts the opposite of
the spec'd behavior. The wrong model is documented at the bug site.

### C2 — `color-scheme` desync (system colors anti-paired with tokens)

Nothing in the stack ever sets the CSS `color-scheme` property (repo-wide grep: zero hits in any
served stylesheet). The UA therefore treats every page as **light-scheme**: `CanvasText` resolves
to black, `Canvas` to white, `LinkText` to `#0000EE` — *regardless* of `body[data-theme]` or the
dark token defaults. Every element-internal system color and every unbound `var(--elohim-*, <system
color>)` fallback then pairs light-scheme values with dark token surfaces.

Computed (WCAG 2.1 relative luminance), current alpha state:

| Surface | fg | bg | Ratio | Threshold |
|---|---|---|---|---|
| omnibar brand/user text (inherits) | `CanvasText` #000 | `--lamad-bg-secondary` #1e293b frozen | **1.44:1** | 4.5 FAIL ← report #2 |
| omnibar sign-in link (`color: inherit`, .8 op) | CanvasText @0.8 | same | **1.37:1** | 4.5 FAIL |
| navigator `.context-switcher-btn` (hardcode) | `CanvasText` #000 | nav bg frozen #1e293b | **1.44:1** | 4.5 FAIL ← report #1 (identity-section) |
| navigator `.search-input` text (hardcode) | `CanvasText` #000 | `--lamad-bg-tertiary` #334155 | **2.03:1** | 4.5 FAIL |
| navigator `.tray-item` text (hardcode) | `CanvasText` #000 | tray bg frozen #1e293b | **1.44:1** | 4.5 FAIL |
| navigator `.tray-item.danger` (Sign out) | `LinkText` #0000EE | same | **1.56:1** | 4.5 FAIL |

### C3 — Element-internal hardcodes bypassing the cssprop surface

Within `elohim-navigator.ts`: `.context-switcher-btn`, `.search-input`, and `.tray-item` hardcode
`color: CanvasText` instead of routing through the element's published fg cssprops; the tray hover
mixes raw `Canvas`/`CanvasText` rather than the bound tray pair; `.banner-warning`/`.banner-error`
backgrounds are literally `color-mix(in oklch, Canvas 85%, Canvas)` — a no-op (warning and error
render identical to the page). `elohim-default-omnibar` publishes **no fg cssprop at all** (only
`-bg`/`-border`), so its text can never be paired by a binding layer. Opacity-on-text idioms
(identifier row inline `opacity: 0.6`, tagline `0.6`, omnibar `.user 0.8`) ride near the threshold.

### C4 — Palette pairing gaps (failures even with everything bound and reactive)

| Pair | dark | light | Threshold |
|---|---|---|---|
| `text-primary` on `accent-primary` (bubble initials 14px/600, toggle "A" badge) | **4.08:1** | **2.84:1** | 4.5 FAIL both |
| **pure white on dark `accent-primary` #6366f1** | **4.47:1** | — | 4.5 FAIL |
| `--lamad-border` (0.2α) vs `bg-tertiary` (search-input boundary) | **1.01:1** | 1.31:1 | 3.0 FAIL both |
| `text-muted` on `bg-secondary` (reference; non-chrome) | **3.07:1** | 7.47:1 | 4.5 FAIL dark |

The key discovery: the backlog's planned "mint `--lamad-on-accent` (near-white)" is
**insufficient** — *no* foreground passes 4.5:1 on `#6366f1` (white = 4.47, near-black = 4.0).
The accent surface itself must change for small text.

### D5 — Bundle baseline gap (the viewport gutter)

`app/lamad/src/styles.scss` imports `tokens.scss` + `_chrome-binding.scss` but **not
`elohim-core/base.scss`**, which carries the universal reset (`* { margin: 0; padding: 0 }`). The
UA default `body { margin: 8px }` is the reported ~8px frame around the lamad viewport. The
pillar-bundle-split runbook's bundle-creation checklist has no styling section (§4.X unwritten —
already captured in `bundle-styling-token-contract.md`), so the next bundle inherits the same hole.

## 2. Decisions (operator-approved 2026-06-05)

**D-1 · html is the theme authority; dual-write migration.** `documentElement[data-theme]` (+
`theme-*` classes) becomes the single mode authority at the outermost wrapper. Stores write the
attribute to **both** html (authority) and body (legacy compat — the shell's 30+
`body[data-theme] <descendant>` selectors keep working untouched). `tokens.scss` manual override
blocks move `body[data-theme='…']` → `:root[data-theme='…']`, which re-resolves the `:root`-scoped
binding chain correctly (declaration and override now cascade on the same element). Any EPR app or
wrapper watches the same `localStorage['elohim-theme']` + `elohim-theme-changed` contract as today.

**D-2 · `color-scheme` is owned by the token layer and asserted everywhere theme is.**
`tokens.scss` gains: `:root { color-scheme: dark }` (mirrors the dark-default palette),
`color-scheme: light` inside the existing `prefers-color-scheme: light` media block, and
`color-scheme: light|dark` inside each `:root[data-theme]` block. System colors and tokens can no
longer disagree. Blank-slate elements remain agnostic: they *honor* the inherited scheme; pages
that don't import tokens keep UA behavior.

**D-3 · Emphasis-accent pair, scoped.** Mint `--lamad-on-accent: #ffffff` and
`--lamad-accent-emphasis: #4f46e5` (same value both themes; white on it = **6.29:1**). The chrome
binding pairs bubble/badge surfaces with the emphasis pair. `--lamad-accent-primary` (#6366f1
dark) is untouched for non-text accent uses. The bubble-vs-navbar boundary drops to ~2.3:1 in
dark, acceptable per WCAG 1.4.11 — the initials text (≥4.5:1) is the control's visual identifier,
not the boundary.

**D-4 · Gate mechanics: DIY computed-style contrast is load-bearing; axe is backstop.** (Research-
verified, sources in §5.) In real Chromium, `getComputedStyle` returns var()-chains and
`color-mix()` fully resolved — but serialized as `oklch(…)`/`color(srgb …)` for non-legacy spaces,
so parsing uses **colorjs.io** (new devDependency), and alpha compositing over the ancestor stack
is hand-walked (Porter-Duff over). axe-core resolves the same values but its color-contrast rule
degrades to **silent `incomplete`** on shadow-DOM/slot/transparent-stack cases (axe #687, #4468) —
a violations-only assertion can pass while contrast was never measured. The gate therefore asserts
exact computed ratios itself and runs axe strictly (violations **and** color-contrast incompletes
both empty) as breadth backstop.

## 3. The fix, by layer (§Layer rails — one concern per home)

### 3.1 Token layer — `app/elohim-elements/elohim-core/tokens.scss`

1. Move `body[data-theme='light']` → `:root[data-theme='light']`; same for `dark`. (Values
   unchanged.)
2. Add `color-scheme` to all four theme-asserting scopes (default dark, media light, manual
   light/dark).
3. Mint `--lamad-on-accent: #ffffff` + `--lamad-accent-emphasis: #4f46e5` in all palette blocks.
4. `base.scss`: move the two `body[data-theme]` scrollbar overrides to `:root[data-theme] body`
   form? **No** — they are descendant selectors off body and keep working under dual-write; leave
   them (legacy-compat surface, reconciles in the graphos-tokens artifact per backlog item 3).

### 3.2 Stores — `theme-store.ts` + Angular `ThemeService` (twin contract)

`applyToDocument()` writes `data-theme` + `theme-*` class to `document.documentElement` **and**
`document.body`. The documented contract becomes: *localStorage `elohim-theme` + html[data-theme]
(authority) + body[data-theme] (compat) + `elohim-theme-changed`*. Both twins change in the same
commit; twin parity asserted in both spec suites.

### 3.3 Binding layer — `app/lamad/src/_chrome-binding.scss`

1. Replace the false resolution-model comment with the correct one (declared-element
   substitution; authority at `:root[data-theme]`).
2. Add missing fg pairings: `--elohim-omnibar-fg: var(--lamad-text-primary)`;
   `--elohim-nav-tray-fg`, `--elohim-nav-search-fg`, banner severity pairs (info/warning/error
   bg+fg from palette).
3. Re-pair accent surfaces: `--elohim-nav-bubble-bg: var(--lamad-accent-emphasis)`,
   `--elohim-nav-bubble-fg: var(--lamad-on-accent)`; same for
   `--elohim-theme-toggle-badge-bg/-fg`.
4. `--elohim-nav-search-border: var(--lamad-border-hover)` (0.4α — the 0.2α token computes 1.01:1,
   an invisible control boundary).

### 3.4 Elements (chrome five) — `app/elohim-elements/elohim-core/src/`

Blank-slate defaults stay system colors; every internal color routes through a *published*
cssprop. `elohim-navigator.ts`: `.context-switcher-btn { color: var(--elohim-nav-fg, CanvasText) }`;
`.search-input { color: var(--elohim-nav-search-fg, CanvasText) }` (+ `@cssprop`);
`.tray-item { color: var(--elohim-nav-tray-fg, CanvasText) }` (+ `@cssprop`);
tray hover → `color-mix(in oklch, var(--elohim-nav-tray-bg, Canvas) 90%, var(--elohim-nav-tray-fg,
CanvasText))`; banner severity backgrounds fixed (info `Canvas 90%/LinkText` as today; warning/
error gain honest mixes + `@cssprop` overrides — `color-mix(in oklch, Canvas 80%, Mark)` class of
fix, exact values verified by the gate); identifier row inline `opacity: 0.6` → `0.75` (computed
headroom: light 4.63→~6.5); tagline likewise. `elohim-default-omnibar.ts`: mint
`@cssprop --elohim-omnibar-fg` (`color: var(--elohim-omnibar-fg, inherit)` on `:host`) so the
binding can pair text with surface. `elohim-theme-toggle` / `elohim-lang-picker` / `elohim-page-
chrome`: audit-only changes surfaced by the gate (their cssprop surfaces are already routed).

### 3.5 Bundle — `app/lamad/src/styles.scss` + runbook

`@use '../../elohim-elements/elohim-core/base'` ahead of tokens (kills the 8px gutter; brings the
focus-visible/reduced-motion/sr-only floor every bundle needs). Pillar-bundle-split runbook gains
the §4.X bundle-styling checklist row: *import base + tokens + binding artifact; never define or
duplicate tokens* (coordinating with the backlog's canonical-artifact item, which this does not
replace).

## 4. The fourth precondition gate — `theme-contrast`

New helper `src/testing/theme-contrast.ts`, wired into the existing per-element gate pattern
(named `describe` blocks in each `*.spec.ts`, like a11y/i18n/ua-prefs). Gate cells derive from the
same `@capability*` claims the `*.manifest.spec.ts` files already assert, making
`@capabilityThemes light, dark` + `@capabilityContrast normal, high` **executable**.

### 4.1 Fixtures (binding lives in the TEST; blank-slate discipline holds)

`themeFixture(template, cell)` renders the element inside an opaque-background fixture:

- **`system-light` / `system-dark`** — no tokens; wrapper sets `color-scheme: light|dark`. The
  blank-slate contract per scheme: system-color defaults must pair correctly when the UA flips.
  Runs for **all** theme-claiming elements (18/18 currently claim `light, dark`).
- **`tokens-light` / `tokens-dark`** — injects the **real** `tokens.scss` + `_chrome-binding.scss`
  text (fetched from source at test time — never a copied fixture that can drift) and sets
  `documentElement[data-theme]`, reproducing the production cascade end-to-end. Runs for elements
  with a shipped binding (the chrome five: navigator, default-omnibar, page-chrome, theme-toggle,
  lang-picker). When the graphos-tokens artifact ships, it becomes the injected source.

### 4.2 Assertions

1. **`assertThemeContrast(el)`** — walk the flattened shadow tree (incl. slotted content); for
   every visible text node compute effective fg (inline + inherited color, opacity stack
   composited) over effective bg (ancestor walk with Porter-Duff compositing until opaque); parse
   all serializations via colorjs.io; assert WCAG 1.4.3 (4.5:1, or 3:1 at ≥24px / ≥18.66px-bold
   from computed font) and 1.4.11 (3:1) for declared control boundaries.
2. **`assertThemeReactivity(el)`** — render the tokens cells under both `data-theme` values;
   assert at least one themed surface/fg computed value **differs**. This assertion alone would
   have caught C1 (the frozen chain) before ship.
3. **`axeScanStrict(el)`** — axe backstop asserting `violations.length === 0` **and**
   `incomplete` (filtered to color-contrast) empty.

### 4.3 Retrofit + fix-or-file policy

The gate lands on all theme-claiming elements. Chrome five fixed in this branch. Long-tail
failures triage: trivial/inline (luminance-picked fg over graduated-feedback's fixed scale hexes;
GrayText-as-active-affordance → cssprop with honest default; reaction-bar's
`--elohim-reaction-warning-color: Canvas`-on-Canvas invisibility) vs design-level/filed (button
brand-bake removal — `#6b46c1`/`#ec4899`/`#f3f4f6`/`#7fcbee` fallbacks are the gospel's cardinal
sin, predating the split; gateway invisible badges). **No element ships with a green gate cell it
doesn't pass; no claimed theme without a passing cell.** An element that cannot yet pass a cell
must shrink its claim (and the manifest-spec enforces claim/tag parity) — never skip the assertion.

### 4.4 Known survey findings the retrofit will hit (pre-registered)

button ghost-variant fg `#f3f4f6` on light Canvas (<1.3:1); graduated-feedback selected
position-btn forces `CanvasText` over fixed hexes (theme-blind); reaction-bar warning-color
default invisible; gate-feedback-trigger ⋮/× default `GrayText`; navigator CustomTheme story binds
cssprops the element doesn't declare (`--elohim-nav-switcher-*`, `-profile-*`, `-banner-*`) —
story/element drift the banner cssprop work (§3.4) partially heals; remainder fix-or-file.

## 5. Evidence base

- Contrast matrix: WCAG 2.1 relative-luminance computation over
  `tokens.scss` palettes × `_chrome-binding.scss` pairs × element var-chains (script-verified,
  2026-06-05; reproduced in §1).
- Live alpha probe (Playwright vs `alpha.elohim.host/lamad/`, 2026-06-05, Chrome 131 headless,
  dark colorScheme): `getComputedStyle(body).margin = "8px"` (D5 confirmed);
  `getComputedStyle(html).colorScheme = "normal"` (C2 confirmed — never set); after toggling
  `body[data-theme=light]` the navigator `.nav` stays `rgb(30,41,59)`/`rgb(241,245,249)` (C1
  frozen chain confirmed); `.context-switcher-btn` computes `color: rgb(0,0,0)` on the dark nav
  (1.44:1 — operator report #1, C3 confirmed); `.search-input` black-on-`rgb(51,65,85)` (2.03:1);
  `.profile-bubble` `rgb(241,245,249)`-on-`rgb(99,102,241)` (4.08:1, C4 confirmed).
- axe-core capability research: getComputedStyle returns var()/color-mix resolved
  (oklch/color(srgb) serializations — colorjs.io parses natively); axe color-contrast
  shadow/slot incompletes (dequelabs/axe-core#687, #4468); compositing not provided by
  getComputedStyle (hand-walk required). Sources: axe-core
  `lib/commons/color/get-background-color.js`, MDN CSSOM resolved-value serialization,
  colorjs.io docs.
- Test-infra survey: wtr + playwrightLauncher(chromium) + esbuild TS; axe-core 4.11.1 via ESM
  shim; ua-prefs helper monkey-patches `matchMedia` only (cannot drive CSS `@media`) — which is
  why the gate uses the `color-scheme` *property* (per-element, no media emulation needed) for
  system cells and `documentElement[data-theme]` for token cells.

## 6. Stories + a2o

- `genesis/a2o/features/elohim-core/chrome-preferences.feature` gains two scenarios
  (`@wip @browser-only`, conventions as sibling scenarios): *"Chrome follows the theme toggle"*
  (omnibar/navigator computed background changes across the toggle — pins C1) and *"Dark-mode
  chrome is readable"* (computed contrast of chrome text ≥ 4.5:1 in dark — pins C2/C3/C4).
- Library A Dark stories remain eyeball canaries (Library A must not bind brand tokens); their
  **assertion-backed twins are the gate's tokens-dark cells**, recorded as such in the story
  descriptions.

## 7. Gospel amendment (cite-disciplined)

`app/elohim-elements/CLAUDE.md`: "The three precondition gates" → **four** (theme-contrast
contract as §4.1–4.2 above); §Layer rails store-contract line updates to *html[data-theme]
authority + body compat*. Edits go through the managed-surface flow (`cite-gen.py --seal`, never
hand-written slugs/fingerprints). `app/lamad/CLAUDE.md` bundle rails: styles import set gains
`base.scss`.

## 8. Out of scope / follow-ups (captured, not absorbed)

1. **graphos-tokens canonical artifact** — unchanged home (`bundle-styling-token-contract.md`);
   this spec updates the backlog item: on-accent minted *with the white-fails-on-#6366f1 finding*,
   emphasis-pair decision recorded; interim binding file remains until the artifact ships.
2. **`text-muted` dark-mode failure (3.07:1)** — palette-wide consumer audit (non-chrome); filed
   to the same backlog.
3. **capabilityContract gate write-back** — `capability-contract.mjs` stubs a11y/i18n/uaPrefs as
   `"unknown"` awaiting an unwired test-runner write-back; the gate produces real grades; wiring
   the manifest write-back is a follow-up (filed).
4. **ThemeService → ThemeStore collapse** — omnibar spec §9.4, unchanged; dual-write makes the
   eventual collapse strictly simpler.
5. **Button brand-bake removal + long-tail element fixes** the retrofit files (per §4.3 triage).
6. **SSR/doorway early-theme inline script** (FOUC avoidance when localStorage theme ≠ default) —
   note only; needs doorway-ssr context.
