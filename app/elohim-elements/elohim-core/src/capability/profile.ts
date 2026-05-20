/**
 * Capability Profile — the viewer-side context object observed by every elohim-element.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §2
 */

export type Lens = 'minimal' | 'simple' | 'standard' | 'detail' | 'debug' | 'trace';

export const LENS_ORDER: readonly Lens[] = [
  'minimal',
  'simple',
  'standard',
  'detail',
  'debug',
  'trace',
] as const;

export type Theme = 'light' | 'dark' | 'auto';

export type Contrast = 'normal' | 'high' | 'auto';

/** BCP 47 language tag, or 'auto' to resolve from navigator.language. */
// eslint-disable-next-line sonarjs/redundant-type-aliases -- semantic alias; preserves intent at consumer sites
export type Locale = string;

export type Stimulus = 'still' | 'gentle' | 'lively' | 'auto';

export const STIMULUS_ORDER: readonly Stimulus[] = ['still', 'gentle', 'lively'] as const;

export type Textuality = 'symbolic' | 'textual' | 'auto';

export interface ProfileLock {
  kind: 'pilot' | 'steward' | 'elohim-support';
  pinnedLens?: Lens;
  maxLens?: Lens;
  pinnedTheme?: Theme;
  pinnedContrast?: Contrast;
  pinnedLocale?: Locale;
  pinnedStimulus?: Stimulus;
  maxStimulus?: Stimulus;
  pinnedTextuality?: Textuality;
  /** ms epoch — present for time-bounded elohim-support sessions */
  expiresAt?: number;
}

// eslint-disable-next-line sonarjs/redundant-type-aliases -- semantic alias; preserves intent at consumer sites
export type Standing = string;

export interface CapabilityProfile {
  lens: Lens;
  theme: Theme;
  contrast: Contrast;
  locale: Locale;
  stimulus: Stimulus;
  textuality: Textuality;
  standings: Standing[];
  lock: ProfileLock;
  origin: 'pilot' | 'steward' | 'elohim-support';
}

/**
 * The Sabbath default. Stillness in the type system; textual for the literate-adult baseline.
 * Stewards may pin pre-literate/symbolic, locked-lens, locked-locale, etc.
 */
export const DEFAULT_PROFILE: CapabilityProfile = {
  lens: 'standard',
  theme: 'auto',
  contrast: 'auto',
  locale: 'auto',
  stimulus: 'still',
  textuality: 'textual',
  standings: [],
  lock: { kind: 'pilot' },
  origin: 'pilot',
} as const;
