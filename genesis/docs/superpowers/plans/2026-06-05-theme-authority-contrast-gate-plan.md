---
title: "Theme Authority + Theme-Contrast Gate — implementation plan"
id: theme-authority-contrast-gate-plan
status: Draft
class: protocol-canonical
domain: D-epr-apps
topic: [theme, dark-mode, color-scheme, contrast, wcag, a11y-gate, tokens, chrome-binding, elohim-core, lamad, theme-store, wtr]
cites:
  - theme-authority-contrast-gate-design | the spec this plan executes — 13 gap-items, four failure classes (C1 frozen chain, C2 color-scheme desync, C3 cssprop bypass, C4 pairing gaps) + D5 bundle gap | sha256:cbb30a3aa7f03d2d | path: genesis/docs/superpowers/specs/2026-06-05-theme-authority-contrast-gate-design.md
---

# Theme Authority + Theme-Contrast Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make dark/light mode propagate correctly from one html-rooted authority to all chrome and content, and make theme-claim contrast failures un-shippable via a fourth element-spec precondition gate (`theme-contrast`).

**Architecture:** TDD at system level — the gate helper is built first against synthetic fixtures, then RED gate specs on navigator/omnibar document the live failures (1.44:1 etc.), then fixes land layer-by-layer (tokens → stores → binding → elements → bundle) until green. Fixtures inject the REAL `tokens.scss`/`_chrome-binding.scss` (served by a wtr plugin), never copies.

**Tech Stack:** Lit 3 / @web/test-runner (playwright-chromium) / @open-wc/testing / axe-core 4.11 (existing ESM shim) / **colorjs.io (new devDep)** / SCSS-as-plain-CSS token files / Angular 19 (ThemeService twin, vitest).

**Worktree caveats (read first):**
- Shared worktree: 5 in-scope files (`elohim-default-omnibar.ts`, `elohim-theme-toggle.ts`, `elohim-lang-picker.ts`, `elohim-page-chrome.ts`, `index.ts` + several `*.spec.ts` and `dist/custom-elements.json`, `genesis/docs/architecture/pillar-bundle-split-runbook.md`) carry **another session's uncommitted cosmetic hunks** (Prettier wrapping + `readonly`). Policy: NEVER revert them. The first commit touching such a file includes its cosmetic hunks with a `(includes pending lint-pass hunks)` note in the body. `git add` whole files only after confirming the only foreign diff is cosmetic (`git diff HEAD -- <file>`).
- Baselines: `cd app/elohim-elements/elohim-core && pnpm test` = **447 pass / 0 fail** before this work. `pnpm lint` / `lint:css` have pre-existing failures — zero NEW only.
- All test commands run from `app/elohim-elements/elohim-core/` unless stated.

---

### Task 1: Test infrastructure — colorjs.io + theme-fixture file server

**Files:**
- Modify: `app/elohim-elements/elohim-core/package.json` (devDependency)
- Modify: `app/elohim-elements/elohim-core/web-test-runner.config.mjs`

- [ ] **Step 1.1: Add colorjs.io devDependency**

```bash
cd /projects/elohim/app/elohim-elements/elohim-core && pnpm add -D colorjs.io@^0.5.2
```
Expected: lockfile + package.json updated; `node_modules/colorjs.io` resolvable.

- [ ] **Step 1.2: Add the theme-fixture serve plugin to wtr config**

In `web-test-runner.config.mjs`, add imports at the top (after the existing two imports):

```js
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
```

After the `axeCoreShim()` function, add:

```js
// The theme-contrast gate injects the REAL token + binding sources as its
// fixture (spec §4.1: never a copied fixture that can drift). Both files are
// plain CSS in .scss clothing (no SCSS syntax). _chrome-binding.scss lives
// outside the wtr rootDir (app/lamad/src/), so serve both at virtual URLs —
// same precedent as axeCoreShim's SHIM_URL.
const PKG_DIR = dirname(fileURLToPath(import.meta.url));
const THEME_FIXTURE_FILES = {
  '/__elohim__/tokens.css': resolve(PKG_DIR, 'tokens.scss'),
  '/__elohim__/chrome-binding.css': resolve(PKG_DIR, '../../lamad/src/_chrome-binding.scss'),
};

function themeFixtureFiles() {
  return {
    name: 'theme-fixture-files',
    serve(context) {
      const src = THEME_FIXTURE_FILES[context.path];
      if (src) {
        return { body: readFileSync(src, 'utf8'), type: 'css' };
      }
      return undefined;
    },
  };
}
```

And register it in `plugins:` (before `esbuildPlugin`): `plugins: [axeCoreShim(), themeFixtureFiles(), esbuildPlugin({ ... })]`.

- [ ] **Step 1.3: Verify the suite still passes and the virtual files serve**

```bash
pnpm test 2>&1 | tail -5
```
Expected: 447 passing (no behavior change).

- [ ] **Step 1.4: Commit**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/package.json app/elohim-elements/elohim-core/web-test-runner.config.mjs pnpm-lock.yaml
git commit -m "test(elohim-core): colorjs.io devDep + wtr theme-fixture file server (real tokens/binding served at virtual URLs)"
```

---

### Task 2: `testing/theme-contrast.ts` — the gate helper (TDD)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/testing/theme-contrast.ts`
- Create: `app/elohim-elements/elohim-core/src/testing/theme-contrast.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/testing/index.ts` (export)

- [ ] **Step 2.1: Write the failing helper spec**

`src/testing/theme-contrast.spec.ts`:

```ts
import { expect } from '@open-wc/testing';
import { html, css, LitElement } from 'lit';
import { customElement } from 'lit/decorators.js';

import {
  themeFixture,
  assertThemeContrast,
  assertThemeReactivity,
  contrastRatio,
  type ThemeCell,
} from './theme-contrast.js';

/** Deliberately failing element: light-gray text hardcoded on white. */
@customElement('tc-bad-contrast')
class TcBadContrast extends LitElement {
  static override readonly styles = css`
    :host { display: block; background: #ffffff; }
    p { color: #cccccc; } /* 1.6:1 on white — must be caught */
  `;
  override render() {
    return html`<p>barely visible text</p>`;
  }
}

/** Honest blank-slate element: system pair, follows color-scheme. */
@customElement('tc-good-contrast')
class TcGoodContrast extends LitElement {
  static override readonly styles = css`
    :host { display: block; background: Canvas; color: CanvasText; }
  `;
  override render() {
    return html`<p>readable text</p>`;
  }
}

/** Theme-frozen element: same colors regardless of data-theme. */
@customElement('tc-frozen')
class TcFrozen extends LitElement {
  static override readonly styles = css`
    :host { display: block; background: #1e293b; color: #f1f5f9; }
  `;
  override render() {
    return html`<p>always dark</p>`;
  }
}

/** Token-reactive element: surface bound to the lamad palette. */
@customElement('tc-reactive')
class TcReactive extends LitElement {
  static override readonly styles = css`
    :host {
      display: block;
      background: var(--lamad-bg-secondary, Canvas);
      color: var(--lamad-text-primary, CanvasText);
    }
  `;
  override render() {
    return html`<p>themed text</p>`;
  }
}

describe('theme-contrast helper', () => {
  it('computes WCAG21 ratios from CSS color strings', () => {
    expect(contrastRatio('rgb(0, 0, 0)', 'rgb(255, 255, 255)')).to.be.closeTo(21, 0.1);
    expect(contrastRatio('rgb(241, 245, 249)', 'rgb(99, 102, 241)')).to.be.closeTo(4.08, 0.05);
  });

  it('flags a failing fg/bg pair with the offending ratio', async () => {
    const { el } = await themeFixture<TcBadContrast>(
      html`<tc-bad-contrast></tc-bad-contrast>`,
      'system-light'
    );
    let thrown: Error | null = null;
    try {
      assertThemeContrast(el);
    } catch (e) {
      thrown = e as Error;
    }
    expect(thrown, 'expected contrast failure').to.not.be.null;
    expect(thrown!.message).to.contain('barely visible');
    expect(thrown!.message).to.match(/1\.6\d?:1/);
  });

  it('passes an honest system-color element in BOTH schemes', async () => {
    for (const cell of ['system-light', 'system-dark'] as ThemeCell[]) {
      const { el } = await themeFixture<TcGoodContrast>(
        html`<tc-good-contrast></tc-good-contrast>`,
        cell
      );
      expect(() => assertThemeContrast(el)).to.not.throw();
    }
  });

  it('tokens cells apply the real palette via documentElement[data-theme]', async () => {
    const { el } = await themeFixture<TcReactive>(html`<tc-reactive></tc-reactive>`, 'tokens-dark');
    const bg = getComputedStyle(el).backgroundColor;
    expect(bg).to.equal('rgb(30, 41, 59)'); // --lamad-bg-secondary dark
  });

  it('assertThemeReactivity catches a frozen element', async () => {
    let thrown = false;
    try {
      await assertThemeReactivity(() => html`<tc-frozen></tc-frozen>`);
    } catch {
      thrown = true;
    }
    expect(thrown, 'frozen element must fail reactivity').to.be.true;
  });

  it('assertThemeReactivity passes a token-bound element', async () => {
    await assertThemeReactivity(() => html`<tc-reactive></tc-reactive>`);
  });
});
```

- [ ] **Step 2.2: Run to verify it fails**

```bash
pnpm exec wtr src/testing/theme-contrast.spec.ts 2>&1 | tail -10
```
Expected: FAIL — `theme-contrast.js` module not found.

- [ ] **Step 2.3: Implement `src/testing/theme-contrast.ts`**

```ts
/**
 * theme-contrast — the FOURTH element precondition gate (sibling of a11y.ts,
 * i18n.ts, ua-prefs.ts). Makes @capabilityThemes / @capabilityContrast claims
 * executable: an element claiming `dark` must pass the dark cells.
 *
 * Cells (spec 2026-06-05-theme-authority-contrast-gate-design §4.1):
 *  - system-light / system-dark : blank-slate render inside a color-scheme
 *    wrapper; system-color defaults must pair correctly per scheme.
 *  - tokens-light / tokens-dark : the REAL tokens.scss + _chrome-binding.scss
 *    are injected (served by the wtr theme-fixture plugin — never copies) and
 *    documentElement[data-theme] is set, reproducing the production cascade.
 *
 * Assertions:
 *  - assertThemeContrast  : computed-style WCAG 1.4.3/1.4.11 walk (load-bearing)
 *  - assertThemeReactivity: themed surfaces must CHANGE across data-theme
 *    (catches frozen var-chains — the C1 class)
 *  - axeScanStrict        : axe backstop; violations AND color-contrast
 *    incompletes must both be empty (incomplete = silently unmeasured)
 *
 * Binding lives in the TEST fixture only — blank-slate discipline holds.
 */
import { fixture } from '@open-wc/testing';
import Color from 'colorjs.io';
import axe from 'axe-core';
import { html, type TemplateResult } from 'lit';

export type ThemeCell = 'system-light' | 'system-dark' | 'tokens-light' | 'tokens-dark';

export interface ThemeFixtureResult<T extends Element> {
  el: T;
  wrapper: HTMLDivElement;
  cell: ThemeCell;
}

export interface ContrastFinding {
  path: string;
  text: string;
  fg: string;
  bg: string;
  ratio: number;
  required: number;
}

const TOKEN_URLS = ['/__elohim__/tokens.css', '/__elohim__/chrome-binding.css'];
let tokenCssCache: string | null = null;

async function loadTokenCss(): Promise<string> {
  if (tokenCssCache === null) {
    const parts = await Promise.all(
      TOKEN_URLS.map(async u => {
        const r = await fetch(u);
        if (!r.ok) throw new Error(`theme-contrast: failed to fetch ${u}: ${r.status}`);
        return r.text();
      })
    );
    tokenCssCache = parts.join('\n');
  }
  return tokenCssCache;
}

/* ── fixture lifecycle (module-level afterEach, same pattern as i18n.ts) ── */

const cleanups = new Set<() => void>();
if (typeof globalThis.afterEach === 'function') {
  globalThis.afterEach(() => {
    cleanups.forEach(fn => fn());
    cleanups.clear();
  });
}

export async function themeFixture<T extends Element>(
  template: TemplateResult,
  cell: ThemeCell
): Promise<ThemeFixtureResult<T>> {
  const scheme = cell.endsWith('dark') ? 'dark' : 'light';
  const tokens = cell.startsWith('tokens');
  const root = document.documentElement;

  if (tokens) {
    const styleEl = document.createElement('style');
    styleEl.setAttribute('data-theme-fixture', '');
    styleEl.textContent = await loadTokenCss();
    document.head.append(styleEl);
    // Mirror the store's dual-write contract exactly (html authority + body compat).
    const prevHtml = root.getAttribute('data-theme');
    const prevBody = document.body.getAttribute('data-theme');
    root.setAttribute('data-theme', scheme);
    document.body.setAttribute('data-theme', scheme);
    cleanups.add(() => {
      styleEl.remove();
      if (prevHtml === null) root.removeAttribute('data-theme');
      else root.setAttribute('data-theme', prevHtml);
      if (prevBody === null) document.body.removeAttribute('data-theme');
      else document.body.setAttribute('data-theme', prevBody);
    });
  }

  // Opaque page context (axe + the bg walk both need an opaque base).
  // Tokens cells use the page-level pair the lamad app provides; system
  // cells use the honest UA pair under the requested scheme.
  const pageBg = tokens ? 'var(--lamad-bg-primary)' : 'Canvas';
  const pageFg = tokens ? 'var(--lamad-text-primary)' : 'CanvasText';
  const wrapper = await fixture<HTMLDivElement>(
    html`<div
      style="color-scheme: ${scheme}; background: ${pageBg}; color: ${pageFg}; padding: 8px;"
    >
      ${template}
    </div>`
  );
  const el = wrapper.firstElementChild as T;
  return { el, wrapper, cell };
}

/* ── color math ── */

export function contrastRatio(fgCss: string, bgCss: string): number {
  const fg = new Color(fgCss);
  const bg = new Color(bgCss);
  return fg.contrast(bg, 'WCAG21');
}

/** Porter-Duff `over`: composite src (with alpha) onto an opaque dst. */
function compositeOver(src: Color, dst: Color): Color {
  const a = src.alpha ?? 1;
  if (a >= 1) return src;
  const s = src.to('srgb');
  const d = dst.to('srgb');
  return new Color('srgb', [
    s.coords[0] * a + d.coords[0] * (1 - a),
    s.coords[1] * a + d.coords[1] * (1 - a),
    s.coords[2] * a + d.coords[2] * (1 - a),
  ]);
}

/** Flattened-tree parent: slot assignment > parentElement > shadow host. */
function flattenedParent(el: Element): Element | null {
  if ((el as HTMLElement).assignedSlot) return (el as HTMLElement).assignedSlot;
  if (el.parentElement) return el.parentElement;
  const rootNode = el.getRootNode();
  return rootNode instanceof ShadowRoot ? rootNode.host : null;
}

/** Effective opaque background behind `el`: walk flattened ancestors compositing alpha layers. */
function effectiveBackground(el: Element): Color {
  const layers: Color[] = [];
  let cur: Element | null = el;
  while (cur) {
    const bg = getComputedStyle(cur).backgroundColor;
    if (bg && bg !== 'transparent') {
      const c = new Color(bg);
      if ((c.alpha ?? 1) > 0) {
        layers.push(c);
        if ((c.alpha ?? 1) >= 1) break; // opaque base found
      }
    }
    cur = flattenedParent(cur);
  }
  // Composite bottom-up; fall back to white if nothing opaque was found
  // (fixture wrappers are opaque, so this is a guard, not a normal path).
  let acc = layers.length && (layers[layers.length - 1].alpha ?? 1) >= 1
    ? layers.pop()!
    : new Color('white');
  while (layers.length) acc = compositeOver(layers.pop()!, acc);
  return acc;
}

/** Cumulative CSS `opacity` product up the flattened chain (whole-subtree dimming). */
function cumulativeOpacity(el: Element): number {
  let o = 1;
  let cur: Element | null = el;
  while (cur) {
    o *= parseFloat(getComputedStyle(cur).opacity || '1');
    cur = flattenedParent(cur);
  }
  return o;
}

/* ── the contrast walk ── */

const HAS_LETTERS_OR_DIGITS = /[\p{L}\p{N}]/u;
/** Emoji / pictographs render as multicolor glyphs — ratio math doesn't apply. */
const EMOJI_ONLY =
  /^[\s\p{Extended_Pictographic}\u{FE0F}\u{200D}]*$/u;

function isVisible(el: Element): boolean {
  const cs = getComputedStyle(el);
  if (cs.display === 'none' || cs.visibility === 'hidden') return false;
  const rect = el.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

function* walkTextNodes(rootEl: Element): Generator<{ text: string; el: Element }> {
  const queue: Element[] = [rootEl];
  while (queue.length) {
    const el = queue.shift()!;
    if (!isVisible(el)) continue;
    const treeChildren: Node[] =
      el instanceof HTMLSlotElement
        ? el.assignedNodes({ flatten: true })
        : Array.from((el.shadowRoot ?? el).childNodes);
    for (const node of treeChildren) {
      if (node.nodeType === Node.TEXT_NODE) {
        const text = node.textContent?.trim() ?? '';
        if (text) yield { text, el };
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        queue.push(node as Element);
      }
    }
  }
}

function requiredRatio(el: Element, text: string): number {
  const cs = getComputedStyle(el);
  const size = parseFloat(cs.fontSize);
  const weight = parseInt(cs.fontWeight, 10) || 400;
  const large = size >= 24 || (size >= 18.66 && weight >= 700);
  if (!HAS_LETTERS_OR_DIGITS.test(text)) return 3.0; // symbol glyph in a control → 1.4.11
  return large ? 3.0 : 4.5;
}

function describePath(el: Element): string {
  const parts: string[] = [];
  let cur: Element | null = el;
  while (cur && parts.length < 5) {
    const part = cur.getAttribute?.('part');
    const cls = (cur as HTMLElement).className?.toString?.().split(/\s+/)[0];
    parts.unshift(part ? `[part=${part}]` : cls ? `.${cls}` : cur.localName);
    cur = flattenedParent(cur);
  }
  return parts.join(' > ');
}

/**
 * Walk every visible text node in the flattened tree; assert WCAG contrast.
 * Throws with a full findings report on any failure; returns the audited
 * samples (pass + fail) for optional further assertions.
 */
export function assertThemeContrast(
  el: Element,
  opts: { ignore?: (finding: ContrastFinding) => boolean } = {}
): ContrastFinding[] {
  const findings: ContrastFinding[] = [];
  const failures: ContrastFinding[] = [];
  for (const { text, el: textEl } of walkTextNodes(el)) {
    if (EMOJI_ONLY.test(text)) continue;
    const cs = getComputedStyle(textEl);
    let fg = new Color(cs.color);
    const bg = effectiveBackground(textEl);
    const opacity = cumulativeOpacity(textEl);
    const fgAlpha = (fg.alpha ?? 1) * opacity;
    if (fgAlpha < 1) fg = compositeOver(new Color(fg.to('srgb'), ), bg) /* placeholder */;
    const ratio = fg.contrast(bg, 'WCAG21');
    const finding: ContrastFinding = {
      path: describePath(textEl),
      text: text.slice(0, 40),
      fg: cs.color,
      bg: bg.toString({ format: 'srgb' }),
      ratio: Math.round(ratio * 100) / 100,
      required: requiredRatio(textEl, text),
    };
    findings.push(finding);
    if (finding.ratio < finding.required && !opts.ignore?.(finding)) failures.push(finding);
  }
  if (failures.length) {
    throw new Error(
      `theme-contrast: ${failures.length} failing pair(s):\n` +
        failures
          .map(f => `  ${f.path} "${f.text}" — ${f.ratio}:1 < ${f.required}:1 (fg ${f.fg} on bg ${f.bg})`)
          .join('\n')
    );
  }
  return findings;
}

/* ── reactivity ── */

interface ColorSnapshot {
  color: string;
  backgroundColor: string;
}

/**
 * Render the SAME template under tokens-light and tokens-dark; assert the
 * probed surface's computed colors differ. Catches frozen var-chains (C1).
 */
export async function assertThemeReactivity<T extends Element>(
  templateFactory: () => TemplateResult,
  probe: (el: T) => Element | null | undefined = el => el as unknown as Element
): Promise<void> {
  const snap = async (cell: ThemeCell): Promise<ColorSnapshot> => {
    const { el } = await themeFixture<T>(templateFactory(), cell);
    await (el as unknown as { updateComplete?: Promise<unknown> }).updateComplete;
    const target = probe(el) ?? (el as unknown as Element);
    const cs = getComputedStyle(target);
    const result = { color: cs.color, backgroundColor: cs.backgroundColor };
    cleanups.forEach(fn => fn()); // flush BETWEEN cells so themes don't stack
    cleanups.clear();
    return result;
  };
  const light = await snap('tokens-light');
  const dark = await snap('tokens-dark');
  if (light.color === dark.color && light.backgroundColor === dark.backgroundColor) {
    throw new Error(
      `theme-contrast: element is theme-FROZEN — identical computed colors in tokens-light and tokens-dark ` +
        `(color ${light.color}, background ${light.backgroundColor}). ` +
        `Check the var-chain: bindings declared on :root resolve against :root-level token values.`
    );
  }
}

/* ── axe backstop ── */

/**
 * axe with the silent-incomplete trap closed: violations AND color-contrast
 * incompletes must both be empty (an incomplete means contrast was never
 * actually measured — shadow/slot/transparent-stack cases).
 */
export async function axeScanStrict(el: Element): Promise<void> {
  const results = await axe.run(el as HTMLElement);
  const contrastIncomplete = results.incomplete.filter(r => r.id === 'color-contrast');
  if (results.violations.length > 0 || contrastIncomplete.length > 0) {
    throw new Error(
      `axeScanStrict: ${results.violations.length} violation(s), ` +
        `${contrastIncomplete.length} unmeasured color-contrast case(s):\n` +
        JSON.stringify({ violations: results.violations, contrastIncomplete }, null, 2)
    );
  }
}
```

**Fix the placeholder line** in `assertThemeContrast` (the fg alpha compositing) — the real implementation is:

```ts
    let fg = new Color(cs.color);
    const bg = effectiveBackground(textEl);
    const opacity = cumulativeOpacity(textEl);
    const fgAlpha = (fg.alpha ?? 1) * opacity;
    if (fgAlpha < 1) {
      fg.alpha = fgAlpha;
      fg = compositeOver(fg, bg);
    }
```

- [ ] **Step 2.4: Export from `src/testing/index.ts`**

Append (matching the existing export style):

```ts
export {
  themeFixture,
  assertThemeContrast,
  assertThemeReactivity,
  axeScanStrict,
  contrastRatio,
} from './theme-contrast.js';
export type { ThemeCell, ThemeFixtureResult, ContrastFinding } from './theme-contrast.js';
```

- [ ] **Step 2.5: Run the helper spec to green**

```bash
pnpm exec wtr src/testing/theme-contrast.spec.ts 2>&1 | tail -10
```
Expected: 6 passing. NOTE: `tokens cells apply the real palette` and `assertThemeReactivity passes a token-bound element` exercise the CURRENT tokens.scss — `tc-reactive` consumes `--lamad-*` directly (not via a :root-declared chain), and the fixture dual-writes body too, so the current `body[data-theme]` blocks satisfy them. They stay green across Task 4's authority move (the fixture mirrors the store's dual-write).

- [ ] **Step 2.6: Full suite + commit**

```bash
pnpm test 2>&1 | tail -3
cd /projects/elohim && git add app/elohim-elements/elohim-core/src/testing/theme-contrast.ts app/elohim-elements/elohim-core/src/testing/theme-contrast.spec.ts app/elohim-elements/elohim-core/src/testing/index.ts
git commit -m "feat(elohim-core): theme-contrast gate helper — 4-cell fixtures, computed-style WCAG walk, reactivity + axe-strict assertions"
```

---

### Task 3: RED gate specs on navigator + omnibar (document the live failures)

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts` (append describe)
- Modify: `app/elohim-elements/elohim-core/src/elohim-default-omnibar.spec.ts` — NOTE: omnibar has **no spec file yet**; Create: `app/elohim-elements/elohim-core/src/elohim-default-omnibar.spec.ts` if absent (check first: `ls src/elohim-default-omnibar.spec.ts`)

- [ ] **Step 3.1: Append the fourth-gate describe to `elohim-navigator.spec.ts`**

```ts
import {
  themeFixture,
  assertThemeContrast,
  assertThemeReactivity,
  axeScanStrict,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-navigator> — theme-contrast gate', () => {
  const CELLS: ThemeCell[] = ['system-light', 'system-dark', 'tokens-light', 'tokens-dark'];

  for (const cell of CELLS) {
    it(`visitor chrome passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimNavigator>(
        html`<elohim-navigator
          .banners=${[{ id: 'b1', message: 'Maintenance notice', severity: 'info' as const }]}
        ></elohim-navigator>`,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });

    it(`authenticated profile tray passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimNavigator>(
        html`<elohim-navigator
          .isAuthenticated=${true}
          .displayName=${'Matthew'}
          .identifier=${'matthew@elohim.host'}
        ></elohim-navigator>`,
        cell
      );
      await el.updateComplete;
      el.shadowRoot!.querySelector<HTMLButtonElement>('.profile-bubble')!.click();
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });

    it(`context tray passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimNavigator>(
        html`<elohim-navigator></elohim-navigator>`,
        cell
      );
      await el.updateComplete;
      el.shadowRoot!.querySelector<HTMLButtonElement>('.context-switcher-btn')!.click();
      await el.updateComplete;
      assertThemeContrast(el);
    });
  }

  it('chrome reacts to the theme (frozen-chain canary)', async () => {
    await assertThemeReactivity<ElohimNavigator>(
      () => html`<elohim-navigator></elohim-navigator>`,
      el => el.shadowRoot!.querySelector('.nav')
    );
  });
});
```

(Use the spec file's existing import style for `ElohimNavigator` / `html` — they are already imported at the top.)

- [ ] **Step 3.2: Create/append the omnibar gate spec**

If `src/elohim-default-omnibar.spec.ts` doesn't exist, create it with register import per sibling specs (`import './register.js';` or direct class import — copy the import pattern from `elohim-theme-toggle.spec.ts`); then:

```ts
describe('<elohim-default-omnibar> — theme-contrast gate', () => {
  const CELLS: ThemeCell[] = ['system-light', 'system-dark', 'tokens-light', 'tokens-dark'];

  for (const cell of CELLS) {
    it(`anonymous omnibar (sign-in link) passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimDefaultOmnibar>(
        html`<elohim-default-omnibar></elohim-default-omnibar>`,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }

  it('omnibar reacts to the theme (frozen-chain canary)', async () => {
    await assertThemeReactivity<ElohimDefaultOmnibar>(
      () => html`<elohim-default-omnibar></elohim-default-omnibar>`
    );
  });
});
```

- [ ] **Step 3.3: Run and CONFIRM RED for the documented reasons**

```bash
pnpm exec wtr src/elohim-navigator.spec.ts src/elohim-default-omnibar.spec.ts 2>&1 | tail -40
```
Expected failures (this is the point — they reproduce production):
- `tokens-dark` cells: ctx-switcher/tray/search text ~1.4–2.0:1 (CanvasText on dark tokens)
- `tokens-light`/`tokens-dark` bubble: 4.08 / 2.84:1
- reactivity canaries: **theme-FROZEN** error (the :root chain)
- system cells: should PASS (blank slate was always correct) — if a system cell fails, that's a real pre-existing blank-slate bug: record it, it must be fixed in Task 6 too.

**Do NOT commit yet** — red specs commit together with the green fixes (Tasks 4–6) so the suite never lands broken.

---

### Task 4: tokens.scss — html authority, color-scheme, emphasis pair

**Files:**
- Modify: `app/elohim-elements/elohim-core/tokens.scss`

- [ ] **Step 4.1: Apply the four edits**

1. In the top `:root {` block, FIRST line after the opening brace:

```css
:root {
  /* The UA must agree with the token palette (spec D-2): dark is the default
     scheme, exactly as the dark palette is the default below. System colors
     (Canvas, CanvasText, LinkText, …) follow color-scheme, never data-theme. */
  color-scheme: dark;
```

and at the end of the same `:root` block (after `--lamad-info`), add the emphasis pair:

```css
  /* On-accent pair (spec D-3): accent-primary #6366f1 cannot carry ANY small
     text at 4.5:1 (white = 4.47:1). Small-text-on-accent surfaces (profile
     bubble, toggle badge) bind to the emphasis pair instead. */
  --lamad-accent-emphasis: #4f46e5;
  --lamad-on-accent: #ffffff;
```

2. In the `@media (prefers-color-scheme: light)` → `:root` block: add `color-scheme: light;` as the first declaration, and the same two emphasis tokens at the end (`--lamad-accent-emphasis: #4f46e5; --lamad-on-accent: #ffffff;`).

3. Change `body[data-theme='light'] {` → `:root[data-theme='light'] {` and add as first declaration `color-scheme: light;`; append the two emphasis tokens. Update the preceding comment block:

```css
/* Manual Theme Overrides - These take precedence over media queries.
   Applied by ThemeStore/ThemeService setting data-theme on documentElement
   (the authority — chrome var-chains are declared on :root and substitute at
   :root, so the override MUST cascade on the same element) and on body
   (legacy compat for the shell's body[data-theme] descendant selectors). */
```

4. Change `body[data-theme='dark'] {` → `:root[data-theme='dark'] {`, add `color-scheme: dark;` first, append the emphasis tokens.

- [ ] **Step 4.2: Re-run the helper spec (must stay green) + navigator gate (expect PARTIAL improvement)**

```bash
pnpm exec wtr src/testing/theme-contrast.spec.ts 2>&1 | tail -5
pnpm exec wtr src/elohim-navigator.spec.ts 2>&1 | tail -20
```
Expected: helper spec 6 passing. Navigator reactivity canary now GREEN (chain unfrozen). Contrast cells still RED (hardcoded CanvasText now follows color-scheme — wait: with color-scheme dark + dark tokens, CanvasText≈white on #1e293b actually PASSES; the remaining reds are bubble 4.08/2.84 and any pair the run surfaces — record actuals).

- [ ] **Step 4.3: Commit (tokens layer alone is consistent)**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/tokens.scss
git commit -m "fix(tokens): :root[data-theme] authority + color-scheme sync + on-accent/accent-emphasis pair

body[data-theme] overrides could never reach the :root-declared chrome
var-chains (substitution happens at the declaring element). color-scheme
now always agrees with the palette. Spec D-1/D-2/D-3."
```

---

### Task 5: Store dual-write — ThemeStore + Angular ThemeService twins

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/theme/theme-store.ts:115-121`
- Modify: `app/elohim-elements/elohim-core/src/theme/theme-store.spec.ts`
- Modify: `app/elohim-app/src/app/services/theme.service.ts:75-88`
- Modify: `app/elohim-app/src/app/services/theme.service.spec.ts`

- [ ] **Step 5.1: Failing test first — theme-store.spec.ts**

Append to the existing applyToDocument coverage (match existing test style):

```ts
it('dual-writes data-theme + theme class to documentElement (authority) and body (compat)', () => {
  const store = new ThemeStore();
  store.set('dark');
  expect(document.documentElement.getAttribute('data-theme')).to.equal('dark');
  expect(document.documentElement.classList.contains('theme-dark')).to.be.true;
  expect(document.body.getAttribute('data-theme')).to.equal('dark');
  expect(document.body.classList.contains('theme-dark')).to.be.true;
  store.set('light');
  expect(document.documentElement.getAttribute('data-theme')).to.equal('light');
  expect(document.documentElement.classList.contains('theme-light')).to.be.true;
  expect(document.documentElement.classList.contains('theme-dark')).to.be.false;
  store.destroy();
});
```

Run: `pnpm exec wtr src/theme/theme-store.spec.ts 2>&1 | tail -8` — expected: the new test FAILS (documentElement untouched).

- [ ] **Step 5.2: Implement in theme-store.ts**

Replace `applyToDocument` (lines 115–121):

```ts
  private applyToDocument(theme: ElohimTheme): void {
    if (typeof document === 'undefined') return;
    // html is the AUTHORITY: tokens.scss :root[data-theme] + color-scheme key
    // off it (chrome var-chains substitute at :root). body keeps the attribute
    // for the shell's legacy body[data-theme] descendant selectors.
    for (const target of [document.documentElement, document.body]) {
      target.classList.remove('theme-light', 'theme-dark', 'theme-device');
      target.classList.add(`theme-${theme}`);
      target.setAttribute('data-theme', theme);
    }
  }
```

Also update the class JSDoc contract line (lines 6–8) to:

```
 *   - html[data-theme="device|light|dark"] (authority) + body[data-theme] (compat)
 *     each with the matching theme-{device|light|dark} class
```

Run: theme-store spec green.

- [ ] **Step 5.3: Failing test — theme.service.spec.ts (vitest)**

Append (match existing spec style — TestBed instantiation already exists in the file):

```ts
it('dual-writes data-theme to documentElement (authority) and body (compat)', () => {
  service.setTheme('dark');
  expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  expect(document.documentElement.classList.contains('theme-dark')).toBe(true);
  expect(document.body.getAttribute('data-theme')).toBe('dark');
});
```

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/services/theme.service.spec.ts 2>&1 | tail -8` — expected FAIL.

- [ ] **Step 5.4: Implement in theme.service.ts**

Replace `applyTheme` (lines 75–88):

```ts
  /**
   * Apply the theme to the document.
   * html is the AUTHORITY (tokens.scss :root[data-theme] + color-scheme);
   * body keeps the attribute for legacy body[data-theme] descendant selectors.
   * Twin contract with elohim-core ThemeStore — change BOTH or NEITHER.
   */
  private applyTheme(theme: Theme): void {
    for (const target of [document.documentElement, document.body]) {
      this.renderer.removeClass(target, 'theme-light');
      this.renderer.removeClass(target, 'theme-dark');
      this.renderer.removeClass(target, 'theme-device');
      this.renderer.addClass(target, `theme-${theme}`);
      this.renderer.setAttribute(target, 'data-theme', theme);
    }
  }
```

Run vitest again — expected PASS.

- [ ] **Step 5.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/src/theme/theme-store.ts app/elohim-elements/elohim-core/src/theme/theme-store.spec.ts app/elohim-app/src/app/services/theme.service.ts app/elohim-app/src/app/services/theme.service.spec.ts
git commit -m "fix(theme): twins dual-write data-theme to documentElement (authority) + body (compat)

(includes pending lint-pass hunks in theme-store.spec.ts)"
```

---

### Task 6: Binding fg pairings + chrome element cssprop routing → navigator/omnibar gates GREEN

**Files:**
- Modify: `app/lamad/src/_chrome-binding.scss` (full rewrite below)
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.ts`
- Modify: `app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts`

- [ ] **Step 6.1: Rewrite `_chrome-binding.scss`**

```scss
/* Chrome binding layer (interim home).
 *
 * ONE concern: map the blank-slate Lit chrome's published @cssprop surface
 * onto the lamad token palette. Nothing else belongs in this file.
 *
 * RESOLUTION MODEL (the 2026-06-05 dark-mode regression root cause): a custom
 * property's var() references substitute at computed-value time ON THE
 * ELEMENT WHERE THE DECLARATION IS SPECIFIED — :root here. Theme overrides
 * therefore MUST also cascade on :root (tokens.scss :root[data-theme],
 * written to documentElement by ThemeStore/ThemeService). A body-level
 * override can never reach these chains.
 *
 * Every *-bg here is paired with a *-fg from the same palette row — never
 * leave a bound surface with an unbound foreground (that pairing gap is what
 * put light-scheme system colors on dark token surfaces).
 *
 * This file migrates WHOLESALE to the shippable graphos-tokens artifact
 * when it ships (see genesis/data/timeline/backlog/
 * bundle-styling-token-contract.md). Do not add bundle-local styles here.
 */
:root {
  /* <elohim-navigator> */
  --elohim-nav-bg: var(--lamad-bg-secondary);
  --elohim-nav-fg: var(--lamad-text-primary);
  --elohim-nav-border: 1px solid var(--lamad-border);
  --elohim-nav-bubble-bg: var(--lamad-accent-emphasis);
  --elohim-nav-bubble-fg: var(--lamad-on-accent);
  --elohim-nav-tray-bg: var(--lamad-bg-secondary);
  --elohim-nav-tray-fg: var(--lamad-text-primary);
  --elohim-nav-tray-border: 1px solid var(--lamad-border);
  --elohim-nav-search-bg: var(--lamad-bg-tertiary);
  --elohim-nav-search-fg: var(--lamad-text-primary);
  --elohim-nav-search-border: 1px solid var(--lamad-border-hover);
  --elohim-nav-banner-info-bg: var(--lamad-bg-tertiary);
  --elohim-nav-banner-info-fg: var(--lamad-text-primary);
  --elohim-nav-banner-warning-bg: var(--lamad-warning);
  --elohim-nav-banner-warning-fg: #1c1300;
  --elohim-nav-banner-error-bg: #7f1d1d;
  --elohim-nav-banner-error-fg: #fef2f2;

  /* <elohim-page-chrome> + <elohim-default-omnibar> */
  --elohim-omnibar-bg: var(--lamad-bg-secondary);
  --elohim-omnibar-fg: var(--lamad-text-primary);
  --elohim-omnibar-border: var(--lamad-border);

  /* <elohim-theme-toggle> */
  --elohim-theme-toggle-fg: var(--lamad-text-primary);
  --elohim-theme-toggle-badge-bg: var(--lamad-accent-emphasis);
  --elohim-theme-toggle-badge-fg: var(--lamad-on-accent);

  /* <elohim-lang-picker> */
  --elohim-lang-picker-fg: var(--lamad-text-primary);
  --elohim-lang-picker-border: var(--lamad-border);
}
```

NOTE the `--elohim-nav-border` / `--elohim-nav-tray-border` values change from bare colors to full `1px solid <color>` shorthands — the element consumes them as `border: var(--elohim-nav-border, 1px solid …)` (whole-border custom props). Verify against the element source while editing; the OLD binding bound bare colors into shorthand slots, which silently invalidated the border (a latent bonus bug — confirm in the gate run).
Banner warning/error literals are palette-row additions pending the graphos-tokens artifact; the gate verifies the pairs (warning `#1c1300` on `--lamad-warning` #f59e0b ≈ 10+:1; error `#fef2f2` on `#7f1d1d` ≈ 8+:1).

- [ ] **Step 6.2: Navigator element edits (`elohim-navigator.ts`)**

In the JSDoc `@cssprop` block, add:

```
 * @cssprop --elohim-nav-tray-fg - Dropdown tray foreground
 * @cssprop --elohim-nav-search-fg - Search input foreground
 * @cssprop --elohim-nav-banner-info-bg - Info banner background
 * @cssprop --elohim-nav-banner-info-fg - Info banner foreground
 * @cssprop --elohim-nav-banner-warning-bg - Warning banner background
 * @cssprop --elohim-nav-banner-warning-fg - Warning banner foreground
 * @cssprop --elohim-nav-banner-error-bg - Error banner background
 * @cssprop --elohim-nav-banner-error-fg - Error banner foreground
 * @cssprop --elohim-nav-danger-fg - Destructive tray-item foreground
 ```

CSS edits in `static styles`:

```css
    .search-input {
      /* … keep existing … */
      color: var(--elohim-nav-search-fg, CanvasText);   /* was: color: CanvasText */
    }

    .context-switcher-btn {
      /* … keep existing … */
      color: var(--elohim-nav-fg, CanvasText);          /* was: color: CanvasText */
    }

    .tray-item {
      /* … keep existing … */
      color: var(--elohim-nav-tray-fg, CanvasText);     /* was: color: CanvasText */
    }

    .tray-item:hover,
    .tray-item:focus-visible {
      /* theme-correct hover: derive from the BOUND tray pair, not raw system colors */
      background: color-mix(
        in oklch,
        var(--elohim-nav-tray-bg, Canvas) 88%,
        var(--elohim-nav-tray-fg, CanvasText)
      );
      outline: none;
    }

    .tray-item.danger {
      color: var(--elohim-nav-danger-fg, LinkText);
    }

    .context-app-tagline {
      font-size: 0.75rem;
      opacity: 0.75;            /* was 0.6 — light-mode headroom (4.63→~6.5:1) */
    }

    .banner-info {
      background: var(--elohim-nav-banner-info-bg, color-mix(in oklch, Canvas 90%, LinkText));
      color: var(--elohim-nav-banner-info-fg, CanvasText);
    }

    .banner-warning {
      /* was color-mix(in oklch, Canvas 85%, Canvas) — a no-op (bug) */
      background: var(--elohim-nav-banner-warning-bg, Mark);
      color: var(--elohim-nav-banner-warning-fg, MarkText);
    }

    .banner-error {
      /* was the same no-op mix as warning */
      background: var(--elohim-nav-banner-error-bg, Mark);
      color: var(--elohim-nav-banner-error-fg, MarkText);
    }
```

Template edit — the identifier row (`_renderIdentifierRow`):

```ts
  private _renderIdentifierRow() {
    if (!this.identifier) return nothing;
    return html`
      <div style="opacity: 0.75; font-size: 0.8em;">${this.identifier}</div>
    `;
  }
```

- [ ] **Step 6.3: Omnibar element edits (`elohim-default-omnibar.ts`)**

JSDoc: add `@cssprop --elohim-omnibar-fg - Override foreground (default: inherit)`.

`:host` styles gain one line:

```css
      color: var(--elohim-omnibar-fg, inherit);
```

- [ ] **Step 6.4: Run the gate specs to GREEN**

```bash
pnpm exec wtr src/elohim-navigator.spec.ts src/elohim-default-omnibar.spec.ts 2>&1 | tail -15
```
Expected: ALL cells + reactivity canaries pass. If a cell still fails, the error message names the exact part/pair/ratio — fix at the right layer (element default vs binding pair vs token) before proceeding. Then full suite:

```bash
pnpm test 2>&1 | tail -3
```
Expected: 447 + new tests, 0 fail.

- [ ] **Step 6.5: Commit (red specs + fixes land together, suite green)**

```bash
cd /projects/elohim && git add app/lamad/src/_chrome-binding.scss app/elohim-elements/elohim-core/src/elohim-navigator.ts app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts app/elohim-elements/elohim-core/src/elohim-default-omnibar.spec.ts
git commit -m "fix(chrome): pair every bound surface with a bound fg; route internals through cssprops; theme-contrast gates green on navigator+omnibar

Binding gains fg rows (omnibar-fg, tray-fg, search-fg, banner pairs) +
emphasis-accent pairing for bubble/badge. Navigator stops hardcoding
CanvasText; banner warning/error color-mix no-op fixed; opacity 0.6->0.75.
(includes pending lint-pass hunks in elohim-default-omnibar.ts + navigator spec)"
```

---

### Task 7: Gate retrofit — theme-toggle, lang-picker, page-chrome

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-theme-toggle.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/elohim-lang-picker.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts`

- [ ] **Step 7.1: Append the gate describe to each spec** — same pattern as Task 3; all three get the four cells + reactivity. Fixtures:
  - theme-toggle: `html`<elohim-theme-toggle></elohim-theme-toggle>`` — its host is transparent; the wrapper provides the page bg. Reactivity probe: `el => el.shadowRoot!.querySelector('button')` (fg flips via `--elohim-theme-toggle-fg` binding). To exercise the "A" badge, the store must be in device mode (default) — assert the badge node is present before walking: `el.shadowRoot!.querySelector('[part=auto-indicator]')`.
  - lang-picker: `html`<elohim-lang-picker></elohim-lang-picker>`` ; probe `el => el.shadowRoot!.querySelector('select')`.
  - page-chrome: pure layout wrapper (no own text) — contrast cells run with slotted content: `html`<elohim-page-chrome><p>page body text</p></elohim-page-chrome>`` (the walk covers slotted light-DOM); SKIP reactivity (no themed surface of its own — document why in a comment).

- [ ] **Step 7.2: Run; triage**

```bash
pnpm exec wtr src/elohim-theme-toggle.spec.ts src/elohim-lang-picker.spec.ts src/elohim-page-chrome.spec.ts 2>&1 | tail -20
```
Expected mostly green after Task 4/6 (badge now on emphasis pair via binding). Known risk: lang-picker `<select>`/`<option>` — native select text color in tokens cells (if the element doesn't route select color through `--elohim-lang-picker-fg`, fix in the element exactly as Task 6 did). Fix-or-file per spec §4.3: trivial → fix now; structural → backlog note (Task 12) + claim shrink.

- [ ] **Step 7.3: Commit**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/src/elohim-theme-toggle.spec.ts app/elohim-elements/elohim-core/src/elohim-lang-picker.spec.ts app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts $(git diff --name-only -- app/elohim-elements/elohim-core/src/elohim-theme-toggle.ts app/elohim-elements/elohim-core/src/elohim-lang-picker.ts app/elohim-elements/elohim-core/src/elohim-page-chrome.ts)
git commit -m "test(elohim-core): theme-contrast gate on toggle/picker/page-chrome (+surfaced fixes)

(includes pending lint-pass hunks in touched files)"
```

---

### Task 8: lamad bundle — base.scss import (the 8px gutter) + runbook row

**Files:**
- Modify: `app/lamad/src/styles.scss`
- Modify: `genesis/docs/architecture/pillar-bundle-split-runbook.md` (CAUTION: carries another session's uncommitted hunks — `git diff HEAD -- <file>` first; edit only your section)

- [ ] **Step 8.1: styles.scss**

```scss
/* Lamad bundle styles (B18).
 *
 * Layer imports — order matters and each layer has ONE home:
 *   base   : universal reset + a11y floor (kills the UA 8px body margin)
 *   tokens : --lamad-* palette, color-scheme, :root[data-theme] reactivity
 *   binding: chrome @cssprop -> palette mapping (interim home)
 * Never define or duplicate tokens here. Source of truth: operational
 * styling, no substrate state (omnibar spec §2; theme-authority spec §3.5).
 */
@use '../../elohim-elements/elohim-core/base';
@use '../../elohim-elements/elohim-core/tokens';
@use './chrome-binding';
```

- [ ] **Step 8.2: Verify the bundle builds**

```bash
cd /projects/elohim/app/lamad && pnpm run build 2>&1 | tail -5
```
Expected: build success. (If `pnpm run build` is not the lamad script name, check `package.json` scripts — use the build script present.)

- [ ] **Step 8.3: Runbook §4.X** — locate the bundle-creation checklist section (`grep -n "checklist\|4\." genesis/docs/architecture/pillar-bundle-split-runbook.md | head`) and append a styling row to the bundle-creation checklist:

```markdown
- [ ] **Styling/token contract**: `src/styles.scss` imports, in order: `elohim-core/base`
  (reset + a11y floor — without it the UA's 8px body margin frames the viewport),
  `elohim-core/tokens` (palette + `color-scheme` + `:root[data-theme]` reactivity), and the
  chrome binding layer (interim `_chrome-binding.scss`, wholesale-migrates to graphos-tokens).
  Never define or duplicate `--lamad-*` tokens in the bundle. (2026-06-05 theme-authority spec §3.5.)
```

- [ ] **Step 8.4: Commit (selective — runbook may carry foreign hunks)**

```bash
cd /projects/elohim && git diff HEAD -- genesis/docs/architecture/pillar-bundle-split-runbook.md | head -50   # confirm what's foreign
git add app/lamad/src/styles.scss
git add -p genesis/docs/architecture/pillar-bundle-split-runbook.md   # stage ONLY the §4.X styling row hunk
git commit -m "fix(lamad): import base.scss — kills UA 8px viewport gutter; runbook bundle-styling checklist row"
```

---

### Task 9: Long-tail retrofit — gate on the remaining 13 theme-claimants

**Files (append a gate describe to each existing spec):**
`elohim-button.spec.ts`, `elohim-epr-link.spec.ts`, `elohim-context-menu.spec.ts`, `elohim-skeleton.spec.ts`, `elohim-mention-base.spec.ts`, `elohim-compute-tile.spec.ts`, `elohim-content-analytics.spec.ts`, `elohim-epr-popover.spec.ts`, `elohim-epr-relationships-panel.spec.ts`, `elohim-feedback-mechanism-gateway.spec.ts`, `elohim-gate-feedback-trigger.spec.ts`, `elohim-graduated-feedback.spec.ts`, `elohim-reaction-bar.spec.ts`

These have **no shipped binding** → run the **system cells only** (`system-light`, `system-dark`); skip tokens cells + reactivity with a one-line comment (`// tokens cells: no shipped binding for this element yet — see theme-authority spec §4.1`). Render each element with a representative fixture (reuse the spec file's existing fixture data/mocks — every spec already renders the element; copy its richest fixture).

Gate describe template per element (substitute tag + fixture):

```ts
describe('<TAG> — theme-contrast gate', () => {
  for (const cell of ['system-light', 'system-dark'] as ThemeCell[]) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ELEMENT_CLASS>(FIXTURE_TEMPLATE, cell);
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
```

- [ ] **Step 9.1: Add gates to all 13 specs; run the full suite; record every failure** (`pnpm test 2>&1 | tee /tmp/retrofit-run.txt`)

- [ ] **Step 9.2: Pre-registered fixes (apply where the gate confirms):**

1. **elohim-button ghost variant** (`elohim-button.ts:111`): `color: var(--elohim-button-fg, var(--text-light, #f3f4f6))` → `color: var(--elohim-button-fg, CanvasText)`. Also the brand-bake fallbacks (gospel cardinal sin): `:100` `var(--elohim-button-bg, var(--primary, #6b46c1))` → `var(--elohim-button-bg, ButtonFace)` + `:101` fg `#fff` → `ButtonText`; `:105/:106` secondary likewise → `ButtonFace`/`ButtonText`; `:82` focus outline `var(--tech-glow, #7fcbee)` → `Highlight`. THEN add button rows to `_chrome-binding.scss` so token-importing bundles keep an accent button: `--elohim-button-bg: var(--lamad-accent-emphasis); --elohim-button-fg: var(--lamad-on-accent);` — note in the commit that variant-specific brand styling moves to Library B/graphos-tokens (file the variant-granularity gap in Task 12 if one cssprop pair proves too coarse: the gate only enforces contrast, not brand).
2. **elohim-graduated-feedback selected position-btn** (`:402-406` inline style): replace `color: CanvasText` with a luminance-picked literal:

```ts
// in the class, above render():
private static readableOn(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  const L = 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
  // white passes 4.5:1 below L≈0.1833; black above
  return L > 0.1833 ? '#000000' : '#ffffff';
}
```

and in the inline style: `color: ${selected ? ElohimGraduatedFeedback.readableOn(position.color) : 'CanvasText'};` (unselected stays transparent-bg + CanvasText). NOTE: verify scale hexes against the helper — `#f1c40f` (L≈0.66) → black ✓; `#e74c3c` (L≈0.22) → black at 4.6:1… if the gate reports a borderline pair, darken the scale hex in `DEFAULT_SCALES` instead (these are element-owned defaults, not brand).
3. **elohim-reaction-bar warning color** (`--elohim-reaction-warning-color` default `Canvas` used as FG on Canvas dialog): default → `Mark`-on-`MarkText`? No — set fg default `CanvasText` and give the indicator a `Mark` background chip, OR simplest honest default: `var(--elohim-reaction-warning-color, LinkText)`. Pick by gate output; the requirement is visible-by-default.
4. **GrayText as active affordance** (gate-feedback-trigger `.trigger-btn`/`.close-btn`, reaction-bar `.reaction-count`): keep the cssprop, change default `GrayText` → `CanvasText` (these are ACTIVE controls; GrayText is the disabled color and fails 4.5:1 in most schemes).
5. **gateway invisible badges** (`badge-controversy`/`badge-settled` bg=Canvas fg=CanvasText): text passes contrast (the gate won't fail it) — the invisible-shape issue is 1.4.11-adjacent; FILE to backlog (Task 12), don't fix here.

- [ ] **Step 9.3: For any OTHER failure the run surfaces**: trivial (default-value or pairing) → fix inline at the right layer; structural → `it.skip` is FORBIDDEN — instead shrink the element's `@capabilityThemes`/`@capabilityContrast` claim to what passes, add the backlog line (Task 12), and leave the gate asserting the shrunk claim. Re-run `pnpm test` to full green.

- [ ] **Step 9.4: Commit (one commit per element-fix cluster is fine; minimum two commits: gate-specs + fixes)**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/src/*.spec.ts
git commit -m "test(elohim-core): theme-contrast system-cells gate retrofitted onto all theme-claiming elements"
git add app/elohim-elements/elohim-core/src/elohim-button.ts app/elohim-elements/elohim-core/src/elohim-graduated-feedback.ts app/elohim-elements/elohim-core/src/elohim-reaction-bar.ts app/elohim-elements/elohim-core/src/elohim-gate-feedback-trigger.ts app/lamad/src/_chrome-binding.scss
git commit -m "fix(elohim-core): retrofit triage — de-brand button fallbacks (cardinal sin), luminance-picked scale fg, honest GrayText/warning defaults"
```

---

### Task 10: a2o scenarios — pin the regression classes

**Files:**
- Modify: `genesis/a2o/features/elohim-core/chrome-preferences.feature`

- [ ] **Step 10.1: Append two scenarios** (match the file's existing Background/persona/tag conventions exactly — read the file first):

```gherkin
  @wip @browser-only
  Scenario: Chrome follows the theme toggle
    # Pins the frozen var-chain class (theme-authority spec C1): the omnibar
    # and navigator must repaint when the person toggles theme — not just the
    # content below them.
    When Matthew navigates to "/lamad"
    And Matthew notes the computed background of the navigator chrome
    And Matthew clicks the element with testid "nav-theme-inline"
    Then the computed background of the navigator chrome changes
    And the document root carries the chosen theme in both data-theme and color-scheme

  @wip @browser-only
  Scenario: Dark-mode chrome is readable
    # Pins the color-scheme desync + pairing classes (spec C2/C3/C4): every
    # visible chrome text node meets WCAG 1.4.3 in dark mode.
    When Matthew navigates to "/lamad"
    And Matthew sets the theme to "dark"
    Then every visible text node in the navigator chrome meets contrast 4.5:1
    And every visible text node in the omnibar meets contrast 4.5:1
```

- [ ] **Step 10.2: Commit**

```bash
cd /projects/elohim && git add genesis/a2o/features/elohim-core/chrome-preferences.feature
git commit -m "test(a2o): pin chrome theme-reactivity + dark-readability scenarios (@wip, step defs follow)"
```

---

### Task 11: Gospel amendments (cite-disciplined)

**Files:**
- Modify: `app/elohim-elements/CLAUDE.md`
- Modify: `app/lamad/CLAUDE.md`
- Modify: `app/elohim-app/CLAUDE.md`

All three are managed surfaces — after editing, run the cite tooling; never hand-write slugs/fingerprints.

- [ ] **Step 11.1: `app/elohim-elements/CLAUDE.md`**

(a) Section "The three precondition gates" → retitle "**The four precondition gates**", renumber, and append:

```markdown
4. **Theme-contrast (theme-contrast)** — the `@capabilityThemes`/`@capabilityContrast` claims are
   EXECUTABLE: every claimed theme has a passing gate cell
   (`src/testing/theme-contrast.ts`). All claimants run the system cells
   (blank-slate under `color-scheme: light|dark` — system-color defaults must
   pair correctly per scheme); elements with a shipped binding also run the
   tokens cells (the REAL `tokens.scss` + binding injected, `documentElement
   [data-theme]` set) plus the reactivity canary (themed surfaces must CHANGE
   across themes — frozen var-chains are the 2026-06-05 dark-mode regression
   class). Contrast walk is computed-style WCAG 1.4.3/1.4.11; axe runs strict
   (violations AND color-contrast incompletes empty). An element that cannot
   pass a cell shrinks its claim — never skips the assertion.
```

Update the intro line "every element passes three gates" accordingly.

(b) §Layer rails — the preference-store rail bullet changes to:

```markdown
- Preference state crosses the chain only via the shared store contracts
  (`theme/theme-store.ts`: `localStorage['elohim-theme']` + `html[data-theme]`
  (authority — token overrides + `color-scheme` live at `:root[data-theme]`,
  where the :root-declared binding chains actually re-resolve) +
  `body[data-theme]` (legacy compat) + `elohim-theme-changed`;
  `localize/locale-store.ts` likewise) — never element-private theme/locale state.
```

(c) §Layer rails table, Token layer row: append to the concern cell: "Owns `color-scheme` — system colors must always agree with the palette."

- [ ] **Step 11.2: `app/lamad/CLAUDE.md`** — in the EPR-app bundle rails Styling bullet, change the import list to "imports the base layer (`elohim-core/base.scss` — reset + a11y floor), the token layer (`elohim-core/tokens.scss`) and the chrome-binding layer (…)".

- [ ] **Step 11.3: `app/elohim-app/CLAUDE.md`** — Chrome rails Theme bullet: replace `body[data-theme]` with `html[data-theme] (authority) + body[data-theme] (compat)` in the twin-contract sentence.

- [ ] **Step 11.4: Cite refresh + commit**

```bash
cd /projects/elohim
python3 .claude/scripts/memory-kit/cite-gen.py --refresh app/elohim-elements/CLAUDE.md app/lamad/CLAUDE.md app/elohim-app/CLAUDE.md 2>&1 | tail -5
# if --refresh isn't the right verb, the PreToolUse hook injection on edit will have named the exact command — follow it; cite-propagate --apply updates inbound fingerprints
python3 .claude/scripts/memory-kit/cite-propagate.py --apply 2>&1 | tail -5
git add app/elohim-elements/CLAUDE.md app/lamad/CLAUDE.md app/elohim-app/CLAUDE.md .claude/memory-kit/cites-index.json
git commit -m "docs(gospel): fourth precondition gate (theme-contrast) + html[data-theme] authority contract + bundle base.scss rail"
```

(If `cites-index.json` carries foreign hunks, `git add -p` only the entries the propagate run touched.)

---

### Task 12: Backlog + follow-up filing

**Files:**
- Modify: `genesis/data/timeline/backlog/bundle-styling-token-contract.md`

- [ ] **Step 12.1: Update item 4 and append findings**

Replace item 4's text with:

```markdown
4. **On-accent pair — PARTIALLY RESOLVED (2026-06-05 theme-authority spec):** minted
   `--lamad-on-accent: #ffffff` + `--lamad-accent-emphasis: #4f46e5` (white on the dark-mode
   accent-primary #6366f1 computes 4.47:1 — NO foreground passes 4.5:1 on it, so small-text
   surfaces bind the emphasis pair instead). Remaining for the artifact: carry both tokens into
   graphos-tokens and re-audit every accent consumer.
```

Append new items:

```markdown
5. **`--lamad-text-muted` fails dark (3.07:1 on bg-secondary)** — palette-wide consumer audit
   needed before any small-text use; chrome avoids it. (theme-authority spec §1 C4)
6. **capabilityContract gate write-back unwired** — `cem-plugins/capability-contract.mjs` stubs
   a11y/i18n/uaPrefs as "unknown"; the theme-contrast + sibling gates now produce real grades; wire
   the test-runner write-back into `dist/custom-elements.json`. (spec §8.3)
7. **Gateway badges invisible by default** (`badge-controversy`/`badge-settled` bg=Canvas on
   Canvas) — shape differentiation, 1.4.11-adjacent; needs a designed treatment, not a default
   tweak. (spec §4.4)
8. **SSR early-theme inline script** — doorway-served pages flash default-dark before
   ThemeStore applies a persisted light preference; needs a doorway-ssr-context design pass.
   (spec §8.6)
9. **Button variant brand styling granularity** — de-branding the primitive (cardinal-sin fix,
   2026-06-05) leaves one binding pair for all variants; per-variant brand treatment belongs to
   graphos-tokens / Library B. (spec §4.3 triage)
```

- [ ] **Step 12.2: Commit**

```bash
cd /projects/elohim && git add genesis/data/timeline/backlog/bundle-styling-token-contract.md
git commit -m "docs(backlog): on-accent resolved-with-evidence + theme-gate follow-ups (muted-dark, write-back, badges, SSR flash, button variants)"
```

---

### Task 13: Final verification + manifest regeneration

- [ ] **Step 13.1: Regenerate custom-elements manifest** (new @cssprop entries):

```bash
cd /projects/elohim/app/elohim-elements/elohim-core && pnpm run analyze 2>&1 | tail -3
```
(`dist/custom-elements.json` is generated AND carries foreign hunks — staging the regenerated whole file is correct.)

- [ ] **Step 13.2: Full quality pass (baseline-aware)**

```bash
cd /projects/elohim/app/elohim-elements/elohim-core
pnpm test 2>&1 | tail -3            # expected: all green (447 baseline + ~60 new gate tests)
pnpm typecheck 2>&1 | tail -3       # expected: clean
pnpm lint 2>&1 | tail -5            # expected: NO NEW failures vs baseline
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/services/theme.service.spec.ts 2>&1 | tail -3
cd /projects/elohim/app/lamad && pnpm run build 2>&1 | tail -3
```

- [ ] **Step 13.3: Live re-probe (the operator's three reports, verified fixed locally)** — re-run the Playwright probe from the spec's evidence section against a local lamad build (or assert via the gate cells if no local stack): body margin `0px`, `colorScheme` follows theme, chrome computed bg CHANGES across toggle, ctx-switcher fg is light-on-dark in dark.

- [ ] **Step 13.4: Final commit + story-harvest**

```bash
cd /projects/elohim && git add app/elohim-elements/elohim-core/dist/custom-elements.json
git commit -m "chore(elohim-core): regenerate custom-elements manifest (new chrome @cssprop surface)"
```

Then invoke the `story-harvest` skill (per CLAUDE.md: after debugging with root cause fixed) — the parameter-bearing discoveries: var()-substitution-at-declaring-element, color-scheme as the system-color authority, the 4.47:1 white-on-#6366f1 bound.

---

## Self-review notes

- Spec coverage: §3.1→T4, §3.2→T5, §3.3/§3.4→T6, §3.5→T8, §4.1/4.2→T1+T2, §4.3/4.4→T3+T7+T9, §6→T10, §7→T11, §8→T12. Gate-cell policy (system-only for unbound elements) matches spec §4.1.
- Types consistent: `ThemeCell`, `themeFixture`, `assertThemeContrast`, `assertThemeReactivity`, `axeScanStrict` used identically in T2/T3/T7/T9.
- Known judgment points left to the executor WITH decision rules: banner literal values (gate arbitrates), reaction-bar warning default (gate output picks), runbook hunk isolation (`git add -p`).
