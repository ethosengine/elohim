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
