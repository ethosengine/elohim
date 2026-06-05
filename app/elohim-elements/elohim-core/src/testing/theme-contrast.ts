/**
 * theme-contrast — the FOURTH element precondition gate (sibling of a11y.ts,
 * i18n.ts, ua-prefs.ts). Makes @capabilityThemes / @capabilityContrast claims
 * executable: an element claiming `dark` must pass the dark cells.
 *
 * Cells (theme-authority spec 2026-06-05 §4.1):
 *  - system-light / system-dark : blank-slate render inside a color-scheme
 *    wrapper; system-color defaults must pair correctly per scheme.
 *  - tokens-light / tokens-dark : the REAL tokens.scss + _chrome-binding.scss
 *    are injected (served by the wtr theme-fixture plugin — never copies) and
 *    documentElement[data-theme] is set, reproducing the production cascade.
 *
 * Assertions:
 *  - assertThemeContrast  : computed-style WCAG 1.4.3/1.4.11 walk (load-bearing)
 *  - assertThemeReactivity: themed surfaces must CHANGE across data-theme
 *    (catches frozen var-chains — the C1 regression class)
 *  - axeScanStrict        : axe backstop; violations AND color-contrast
 *    incompletes must both be empty (an incomplete means contrast was never
 *    actually measured — shadow/slot/transparent-stack cases)
 *
 * Binding lives in the TEST fixture only — blank-slate discipline holds.
 */
import { fixture } from '@open-wc/testing';
import axe from 'axe-core';
import Color from 'colorjs.io';
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

function flushCleanups(): void {
  cleanups.forEach(fn => fn());
  cleanups.clear();
}

const mochaGlobal = globalThis as { afterEach?: (fn: () => void) => void };
if (typeof mochaGlobal.afterEach === 'function') {
  mochaGlobal.afterEach(() => {
    flushCleanups();
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
  // Tokens cells use the page-level pair the consuming app provides; system
  // cells use the honest UA pair under the requested scheme.
  const pageBg = tokens ? 'var(--lamad-bg-primary)' : 'Canvas';
  const pageFg = tokens ? 'var(--lamad-text-primary)' : 'CanvasText';
  const wrapper = await fixture<HTMLDivElement>(
    html`
      <div
        style="color-scheme: ${scheme}; background: ${pageBg}; color: ${pageFg}; padding: 8px;"
      >
        ${template}
      </div>
    `
  );
  const el = wrapper.firstElementChild as T;
  disableMotion(wrapper);
  return { el, wrapper, cell };
}

/**
 * Contrast is a STEADY-STATE property: an element animating open (opacity
 * transition) measures mid-flight otherwise — fg composited onto bg at
 * opacity≈0 reads as a bogus 1:1. Document styles don't pierce shadow roots,
 * so adopt a no-motion sheet into every shadow root in the subtree.
 */
const NO_MOTION_SHEET = new CSSStyleSheet();
// `:host` is required separately — `*` inside a shadow root never matches the
// host element, and host-level entry animations (e.g. :host([open]) fold-down)
// are exactly what leaves computed opacity at 0 mid-measurement.
NO_MOTION_SHEET.replaceSync(
  ':host, *, *::before, *::after { transition: none !important; animation: none !important; }'
);

function disableMotion(rootEl: Element): void {
  const queue: Element[] = [rootEl];
  while (queue.length > 0) {
    const el = queue.shift()!;
    if (el.shadowRoot) {
      if (!el.shadowRoot.adoptedStyleSheets.includes(NO_MOTION_SHEET)) {
        el.shadowRoot.adoptedStyleSheets = [...el.shadowRoot.adoptedStyleSheets, NO_MOTION_SHEET];
      }
      queue.push(...Array.from(el.shadowRoot.querySelectorAll('*')));
    }
    queue.push(...Array.from(el.children));
  }
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
  const slot = (el as HTMLElement).assignedSlot;
  if (slot) return slot;
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
  const base = layers.at(-1);
  let acc = base && (base.alpha ?? 1) >= 1 ? base : new Color('white');
  if (acc === base) layers.pop();
  for (let i = layers.length - 1; i >= 0; i--) {
    acc = compositeOver(layers[i] as Color, acc);
  }
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
const EMOJI_ONLY = /^[\s\p{Extended_Pictographic}\u{FE0F}\u{200D}]*$/u;

function isVisible(el: Element): boolean {
  const cs = getComputedStyle(el);
  if (cs.display === 'none' || cs.visibility === 'hidden') return false;
  // display:contents (e.g. <slot>) generates no box — children decide.
  // Skipping on zero-rect here silently dropped all slotted text (gap found
  // by the axe backstop on elohim-button's slotted labels).
  if (cs.display === 'contents') return true;
  const rect = el.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

function* walkTextNodes(rootEl: Element): Generator<{ text: string; el: Element }> {
  const queue: Element[] = [rootEl];
  while (queue.length > 0) {
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

/** WCAG 1.4.3 exempts "text that is part of an inactive user interface component". */
function isInactiveControl(el: Element): boolean {
  let cur: Element | null = el;
  while (cur) {
    if (
      cur.hasAttribute('disabled') ||
      cur.getAttribute('aria-disabled') === 'true'
    ) {
      return true;
    }
    cur = flattenedParent(cur);
  }
  return false;
}

function requiredRatio(el: Element, text: string): number {
  const cs = getComputedStyle(el);
  const size = parseFloat(cs.fontSize);
  const weight = parseInt(cs.fontWeight, 10) || 400;
  const large = size >= 24 || (size >= 18.667 && weight >= 700);
  if (!HAS_LETTERS_OR_DIGITS.test(text)) return 3.0; // symbol glyph in a control → 1.4.11
  return large ? 3.0 : 4.5;
}

function describePath(el: Element): string {
  const parts: string[] = [];
  let cur: Element | null = el;
  while (cur && parts.length < 5) {
    const part = cur.getAttribute?.('part');
    const cls = typeof cur.className === 'string' ? cur.className.split(/\s+/)[0] : '';
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
    if (isInactiveControl(textEl)) continue; // WCAG 1.4.3 inactive-component exemption
    const cs = getComputedStyle(textEl);
    let fg = new Color(cs.color);
    const bg = effectiveBackground(textEl);
    const opacity = cumulativeOpacity(textEl);
    const fgAlpha = (fg.alpha ?? 1) * opacity;
    if (fgAlpha < 1) {
      fg.alpha = fgAlpha;
      fg = compositeOver(fg, bg);
    }
    const finding: ContrastFinding = {
      path: describePath(textEl),
      text: text.slice(0, 40),
      fg: cs.color,
      bg: bg.toString({ format: 'srgb' }),
      ratio: Math.round(fg.contrast(bg, 'WCAG21') * 100) / 100,
      required: requiredRatio(textEl, text),
    };
    findings.push(finding);
    if (finding.ratio < finding.required && !opts.ignore?.(finding)) failures.push(finding);
  }
  if (failures.length > 0) {
    throw new Error(
      `theme-contrast: ${failures.length} failing pair(s):\n` +
        failures
          .map(
            f =>
              `  ${f.path} "${f.text}" — ${f.ratio}:1 < ${f.required}:1 (fg ${f.fg} on bg ${f.bg})`
          )
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
 * probed surface's computed colors differ. Catches frozen var-chains (C1):
 * a binding declared on :root substitutes against :root-level token values,
 * so overrides that cascade anywhere BELOW :root never reach it.
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
    flushCleanups(); // flush BETWEEN cells so themes don't stack
    return result;
  };
  const light = await snap('tokens-light');
  const dark = await snap('tokens-dark');
  if (light.color === dark.color && light.backgroundColor === dark.backgroundColor) {
    throw new Error(
      `theme-contrast: element is theme-FROZEN — identical computed colors in tokens-light and ` +
        `tokens-dark (color ${light.color}, background ${light.backgroundColor}). ` +
        `Check the var-chain: bindings declared on :root resolve against :root-level token values.`
    );
  }
}

/* ── axe backstop ── */

/**
 * axe with the silent-incomplete trap closed: violations AND color-contrast
 * incompletes must both be empty (an incomplete means contrast was never
 * actually measured — shadow/slot/transparent-stack cases).
 *
 * Excused incomplete classes — abstentions the load-bearing
 * assertThemeContrast walk already covers (it measures every visible text
 * node, symbols included, with colorjs.io parsing every CSS Color 4
 * serialization):
 *  - `colorParse`       : axe-core 4.11's parser rejects Chrome's
 *    achromatic-hue serialization (`oklch(L C none)`, produced by e.g.
 *    `color-mix(in oklch, Canvas 92%, currentColor)`)
 *  - `nonBmp` / `shortTextContent` : axe abstains on glyph-only content
 *    (▾, ×) — content classification, not an unmeasured pair
 * Geometry/stack incompletes (bgImage, gradient, obscured, …) remain hard
 * failures because nothing else measured them.
 */
const EXCUSED_INCOMPLETE_KEYS = new Set(['colorParse', 'nonBmp', 'shortTextContent']);

export async function axeScanStrict(el: Element): Promise<void> {
  const results = await axe.run(el as HTMLElement);
  const contrastIncomplete = results.incomplete
    .filter(r => r.id === 'color-contrast')
    .map(r => ({
      ...r,
      nodes: r.nodes.filter(
        n =>
          !n.any.every(check =>
            EXCUSED_INCOMPLETE_KEYS.has((check.data as { messageKey?: string })?.messageKey ?? '')
          )
      ),
    }))
    .filter(r => r.nodes.length > 0);
  if (results.violations.length > 0 || contrastIncomplete.length > 0) {
    throw new Error(
      `axeScanStrict: ${results.violations.length} violation(s), ` +
        `${contrastIncomplete.length} unmeasured color-contrast case(s):\n` +
        JSON.stringify({ violations: results.violations, contrastIncomplete }, null, 2)
    );
  }
}
