/**
 * UA-prefs precondition-gate helpers.
 *
 * Patches window.matchMedia to allow tests to force specific OS preference
 * states. Use clearMediaQueries() in afterEach to reset.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.3
 */

import type { Stimulus } from '../capability/profile.js';

type MediaPref =
  | 'prefers-reduced-motion'
  | 'prefers-color-scheme'
  | 'prefers-contrast'
  | 'prefers-reduced-transparency'
  | 'prefers-reduced-data'
  | 'forced-colors'
  | 'update'
  | 'pointer'
  | 'hover';

const overrides = new Map<string, string>();
let originalMatchMedia: typeof window.matchMedia | null = null;

function ensurePatched(): void {
  if (originalMatchMedia) return;
  originalMatchMedia = window.matchMedia.bind(window);
  window.matchMedia = (query: string): MediaQueryList => {
    const matches = matchesOverride(query);
    if (matches !== null) {
      return makeFakeMediaQueryList(query, matches);
    }
    return originalMatchMedia!(query);
  };
}

function matchesOverride(query: string): boolean | null {
  for (const [pref, value] of overrides) {
    const match = query.match(new RegExp(`\\(${pref}:\\s*([^)]+)\\)`));
    if (match) {
      const wanted = match[1]!.trim();
      return wanted === value;
    }
  }
  return null;
}

function makeFakeMediaQueryList(query: string, matches: boolean): MediaQueryList {
  return {
    matches,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  } as unknown as MediaQueryList;
}

/** Force a specific OS preference state for the next matchMedia query. */
export function setMediaQuery(pref: MediaPref, value: string): void {
  ensurePatched();
  overrides.set(pref, value);
}

/** Reset all overrides; restores original window.matchMedia. */
export function clearMediaQueries(): void {
  overrides.clear();
  if (originalMatchMedia) {
    window.matchMedia = originalMatchMedia;
    originalMatchMedia = null;
  }
}

/**
 * Computes the effective stimulus ceiling from OS preferences alone.
 * See spec §2.5: effectiveStimulus = min(profile.stimulus, osCeiling).
 */
export function effectiveStimulusCeiling(): Stimulus {
  const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const epaper = matchMedia('(update: slow)').matches;
  return reduceMotion || epaper ? 'still' : 'lively';
}

export interface LuminanceMeasurement {
  /** Number of luminance crossings per second (large-area). Above 3 Hz is unsafe per WCAG 2.3. */
  flashHz: number;
  /** Whether any 1-second window exceeded the WCAG flash threshold. */
  exceedsThreshold: boolean;
  /** Number of samples collected. */
  samples: number;
}

export interface LuminanceOptions {
  /** Duration of the sampling window in ms. */
  sampleMs?: number;
  /** Samples per second. */
  sampleHz?: number;
  /** Relative luminance delta that counts as a "flash" boundary. WCAG uses 0.1. */
  luminanceDelta?: number;
}

/**
 * Measures luminance changes in an element over time by sampling computed background.
 * Lightweight approximation: reads computed-style backgrounds for the element and its
 * first descendant; counts crossings of a brightness threshold. Intended as a
 * regression-detector, not a real visual analyzer.
 *
 * For accurate testing, use a tool like FlashTest or Visual A11y. This harness flags
 * the obvious cases (full-element strobing) and gives a starting point.
 */
export async function measureLuminanceChanges(
  el: Element,
  opts: LuminanceOptions = {}
): Promise<LuminanceMeasurement> {
  const sampleMs = opts.sampleMs ?? 1000;
  const sampleHz = opts.sampleHz ?? 30;
  const delta = opts.luminanceDelta ?? 0.1;
  const interval = 1000 / sampleHz;

  function getBrightness(target: Element): number {
    const style = getComputedStyle(target);
    const bg = style.backgroundColor;
    const rgb = bg.match(/\d+/g);
    if (!rgb || rgb.length < 3) return 0;
    const r = Number.parseInt(rgb[0]!, 10) / 255;
    const g = Number.parseInt(rgb[1]!, 10) / 255;
    const b = Number.parseInt(rgb[2]!, 10) / 255;
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }

  const target = (el.shadowRoot?.firstElementChild ?? el.firstElementChild ?? el) as Element;
  const samples: number[] = [];
  const end = performance.now() + sampleMs;
  await new Promise<void>(resolve => {
    function tick(): void {
      samples.push(getBrightness(target));
      if (performance.now() >= end) resolve();
      else setTimeout(tick, interval);
    }
    tick();
  });

  let crossings = 0;
  let prev = samples[0]!;
  for (const value of samples.slice(1)) {
    if (Math.abs(value - prev) > delta) crossings++;
    prev = value;
  }
  const flashHz = (crossings / sampleMs) * 1000 / 2; // each flash is a pair of crossings
  return {
    flashHz,
    exceedsThreshold: flashHz > 3,
    samples: samples.length,
  };
}
