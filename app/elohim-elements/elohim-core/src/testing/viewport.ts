/**
 * viewport — the viewport precondition gate (sibling of a11y.ts, i18n.ts,
 * ua-prefs.ts, theme-contrast.ts). Makes viewport behavior executable the
 * same way the theme cells make @capabilityThemes executable: a floating
 * surface (panel, menu, tooltip) must stay inside the visual viewport at
 * every archetype it can be anchored from.
 *
 * Cells: the named archetypes in `../viewport-archetypes.ts` (phone-small /
 * phone / tablet / desktop) × anchor edge (inline-start / inline-end). The
 * 2026-06-12 regression class this gate pins: an anchored fold-down surface
 * pinned `inset-inline-start: 0` to a tiny inline anchor in end-side chrome
 * projected 173px off a phone viewport — width caps (80vw) bound the box's
 * SIZE but never its POSITION, so only a position assertion catches it.
 *
 * Assertions:
 *  - assertInlineWithinViewport : the element's border box must sit fully
 *    inside the visual viewport on the inline axis (the regression anchor)
 *  - assertWithinViewport       : both axes (block overflow may be scrollable
 *    by design — use the inline assertion unless the surface claims both)
 *
 * `setViewportArchetype` drives the real browser window via
 * @web/test-runner-commands; tests MUST restore the default afterwards
 * (afterEach(resetViewport)) so later specs measure the runner default.
 */
/* eslint-disable import/no-extraneous-dependencies */
import { setViewport } from '@web/test-runner-commands';
/* eslint-enable import/no-extraneous-dependencies */

import { VIEWPORT_ARCHETYPES, type ViewportArchetypeName } from '../viewport-archetypes.js';

export {
  VIEWPORT_ARCHETYPES,
  type ViewportArchetype,
  type ViewportArchetypeName,
} from '../viewport-archetypes.js';

/** The @web/test-runner default window size — what resetViewport restores. */
const RUNNER_DEFAULT = { width: 800, height: 600 };

/** Resize the test browser window to a named archetype. */
export async function setViewportArchetype(name: ViewportArchetypeName): Promise<void> {
  const { width, height } = VIEWPORT_ARCHETYPES[name];
  await setViewport({ width, height });
}

/** Resize to a raw size (sub-archetype guards, e.g. below the 240px floor). */
export async function setViewportSize(size: { width: number; height: number }): Promise<void> {
  await setViewport(size);
}

/** Restore the runner default window size (call from afterEach). */
export async function resetViewport(): Promise<void> {
  await setViewport(RUNNER_DEFAULT);
}

/**
 * Assert the element's border box is fully inside the visual viewport on the
 * inline axis. `gutterPx` tolerates sub-pixel layout (default 0.5px).
 */
export function assertInlineWithinViewport(el: Element, gutterPx = 0.5): void {
  const rect = el.getBoundingClientRect();
  const viewportWidth = document.documentElement.clientWidth;
  const findings: string[] = [];
  if (rect.left < -gutterPx) {
    findings.push(`left edge at ${rect.left.toFixed(1)}px (< 0)`);
  }
  if (rect.right > viewportWidth + gutterPx) {
    findings.push(
      `right edge at ${rect.right.toFixed(1)}px (> viewport ${viewportWidth}px — ` +
        `${(rect.right - viewportWidth).toFixed(1)}px off-screen)`
    );
  }
  if (findings.length > 0) {
    throw new Error(
      `viewport gate: element overflows the inline axis: ${findings.join('; ')}. ` +
        `Floating surfaces must reposition (align="end") or clamp — width caps ` +
        `alone cannot keep a start-pinned box on-screen.`
    );
  }
}

/** Assert the element's border box is fully inside the viewport on both axes. */
export function assertWithinViewport(el: Element, gutterPx = 0.5): void {
  assertInlineWithinViewport(el, gutterPx);
  const rect = el.getBoundingClientRect();
  const viewportHeight = document.documentElement.clientHeight;
  if (rect.top < -gutterPx || rect.bottom > viewportHeight + gutterPx) {
    throw new Error(
      `viewport gate: element overflows the block axis: top=${rect.top.toFixed(1)}, ` +
        `bottom=${rect.bottom.toFixed(1)}, viewport height=${viewportHeight}px.`
    );
  }
}
