/**
 * Sophia Element Loader - Loads Sophia plugin from external bundle.
 *
 * This module provides lazy loading of the Sophia custom element from
 * an external UMD bundle, ensuring React and Sophia are only loaded when needed.
 */

// @coverage: 45.0% (2026-02-05)

// Import types for local use and re-export for consumers
import type { Moment, Recognition } from './sophia-moment.model';
export type { Moment, Recognition } from './sophia-moment.model';

/** User input map for answer persistence */
export type UserInputMap = Record<string, unknown>;

/**
 * Interface for the Sophia custom element.
 */
export interface SophiaQuestionElement extends HTMLElement {
  moment: Moment | null;
  mode: 'mastery' | 'discovery' | 'reflection';
  instrumentId: string | null;
  reviewMode: boolean;
  /** Set this BEFORE setting moment when navigating back to restore previous answer */
  initialUserInput: UserInputMap | null;
  onRecognition: ((result: Recognition) => void) | null;
  onAnswerChange: ((hasAnswer: boolean) => void) | null;
  getRecognition(): Recognition | null;
  focusInput(): void;
  getState(): unknown;
  restoreState(state: unknown): void;
}

// Track loading state
let loadPromise: Promise<void> | null = null;
let isRegistered = false;
let cssLoaded = false;

/** Log prefix for Sophia loader messages */
const _LOG_PREFIX = '[Sophia]';

/** Custom element tag name for Sophia questions */
const SOPHIA_ELEMENT_TAG = 'sophia-question';

// CSS URLs
const getSophiaStylesUrl = (): string => {
  return (
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ((globalThis as any)['__SOPHIA_STYLES_URL__'] as string) || '/assets/sophia-plugin/index.css'
  );
};

const getSophiaThemeOverridesUrl = (): string => {
  return (
    ((globalThis as any)['__SOPHIA_THEME_OVERRIDES_URL__'] as string) ||
    '/assets/sophia-plugin/sophia-theme-overrides.css'
  );
};

/**
 * Load Sophia CSS stylesheets lazily.
 */
function loadSophiaCSS(): void {
  if (cssLoaded) return;

  // Load base styles
  const baseStylesId = 'sophia-styles';
  if (!document.getElementById(baseStylesId)) {
    const baseLink = document.createElement('link');
    baseLink.id = baseStylesId;
    baseLink.rel = 'stylesheet';
    baseLink.href = getSophiaStylesUrl();
    document.head.appendChild(baseLink);
  }

  // Load theme overrides
  const themeOverridesId = 'sophia-theme-overrides';
  if (!document.getElementById(themeOverridesId)) {
    const themeLink = document.createElement('link');
    themeLink.id = themeOverridesId;
    themeLink.rel = 'stylesheet';
    themeLink.href = getSophiaThemeOverridesUrl();
    document.head.appendChild(themeLink);
  }

  cssLoaded = true;
}

// Cache-bust once per page load
const CACHE_BUST = Date.now();
const getSophiaPluginUrl = (): string => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const customUrl = (globalThis as any)['__SOPHIA_PLUGIN_URL__'] as string;
  if (customUrl) return customUrl;
  return `/assets/sophia-plugin/sophia-element.umd.js?v=${CACHE_BUST}`;
};

/**
 * Load an external script by URL.
 */
async function loadScript(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (document.querySelector(`script[src="${url}"]`)) {
      resolve();
      return;
    }

    const script = document.createElement('script');
    script.src = url;
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error(`Failed to load script: ${url}`));
    document.head.appendChild(script);
  });
}

// React URLs - local assets for offline support, CDN fallback for development
const getReactUrl = (): string => {
  return (
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ((globalThis as any)['__REACT_URL__'] as string) || '/assets/react/react.production.min.js'
  );
};

const getReactDomUrl = (): string => {
  return (
    ((globalThis as any)['__REACT_DOM_URL__'] as string) ||
    '/assets/react/react-dom.production.min.js'
  );
};

// CDN fallback URLs
const REACT_CDN_URL = 'https://unpkg.com/react@18/umd/react.production.min.js';
const REACT_DOM_CDN_URL = 'https://unpkg.com/react-dom@18/umd/react-dom.production.min.js';

/**
 * Load React from local assets, falling back to CDN if local fails.
 */
async function ensureReactLoaded(): Promise<void> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const win = globalThis as any;

  if (win.React && win.ReactDOM) {
    return;
  }

  // Try local assets first (for offline support)
  const localReactUrl = getReactUrl();
  const localReactDomUrl = getReactDomUrl();

  try {
    await loadScript(localReactUrl);
    await loadScript(localReactDomUrl);

    if (win.React && win.ReactDOM) {
      return;
    }
  } catch {
    // Local React load failed - will fallback to CDN
  }

  // Fallback to CDN
  await loadScript(REACT_CDN_URL);
  await loadScript(REACT_DOM_CDN_URL);

  if (!win.React || !win.ReactDOM) {
    throw new Error('Failed to load React from both local assets and CDN');
  }
}

/**
 * Lazily register the Sophia custom element by loading the external bundle.
 */
export async function registerSophiaElement(): Promise<void> {
  // Already registered
  if (isRegistered || customElements.get(SOPHIA_ELEMENT_TAG)) {
    isRegistered = true;
    return;
  }

  // Loading in progress
  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Load CSS
      loadSophiaCSS();

      // Load React first
      await ensureReactLoaded();

      const pluginUrl = getSophiaPluginUrl();

      // Load the UMD bundle
      await loadScript(pluginUrl);

      // Element should be registered synchronously
      const elementDef = customElements.get(SOPHIA_ELEMENT_TAG);

      if (!elementDef) {
        throw new Error('Sophia custom element not registered after bundle load');
      }

      // Configure Sophia with debug logging for development
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const SophiaElement = (globalThis as any).SophiaElement;
      if (SophiaElement?.Sophia?.configure) {
        // Use 'attribute' mode because elohim-app sets data-theme="light|dark|device"
        // Class mode wouldn't work because our classes are 'theme-light' not 'light'
        // When data-theme="device", Sophia falls back to system preference (correct behavior)
        SophiaElement.Sophia.configure({
          theme: 'auto',
          detectThemeFrom: 'attribute',
          logLevel: 'debug',
        });
      } else {
        // Sophia.configure not available - using default configuration
        console.debug('[SophiaLoader] Sophia.configure not available, using defaults');
      }

      isRegistered = true;
    } catch (error) {
      // Sophia element registration failed - re-throw to caller
      loadPromise = null;
      throw error;
    }
  })();

  return loadPromise;
}

/**
 * Check if the Sophia element is registered.
 */
export function isSophiaElementRegistered(): boolean {
  return isRegistered || !!customElements.get(SOPHIA_ELEMENT_TAG);
}

/**
 * Get the Sophia element by querying the DOM.
 */
export function getSophiaElement(container: HTMLElement): SophiaQuestionElement | null {
  return container.querySelector(SOPHIA_ELEMENT_TAG) as SophiaQuestionElement | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Psyche API Access
// ─────────────────────────────────────────────────────────────────────────────
//
// Architecture note: psyche-core is bundled within sophia-plugin and exposed
// through window.SophiaPlugin. This loader provides typed access to the
// psyche-core functions for Angular consumers.
//
// The types below mirror psyche-core's exports. Consumers should use the
// getPsycheAPI() and requirePsycheAPI() accessors rather than accessing
// window.SophiaPlugin directly to ensure proper typing.
//
// For the rendering layer, use the sophia-question element directly.
// For psychometric processing (aggregation, interpretation), use this API.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Interface for the Psyche API exported by the Sophia plugin.
 * These functions provide psychometric instrument processing.
 */
export interface PsycheAPI {
  // Instrument registry
  registerInstrument: (instrument: PsychometricInstrument) => void;
  updateInstrument: (instrument: PsychometricInstrument) => void;
  unregisterInstrument: (instrumentId: string) => boolean;
  getInstrument: (instrumentId: string) => PsychometricInstrument | undefined;
  getAllInstruments: () => PsychometricInstrument[];
  hasInstrument: (instrumentId: string) => boolean;
  clearInstruments: () => void;
  createInstrument: (options: CreateInstrumentOptions) => PsychometricInstrument;

  // Reflection processing
  recognizeReflection: (
    moment: ReflectionMoment,
    userInput: Record<string, unknown>
  ) => ReflectionRecognition;
  aggregateReflections: (
    recognitions: ReflectionRecognition[],
    options?: AggregateReflectionOptions
  ) => AggregatedReflection;
  mergeAggregatedReflections: (
    a: AggregatedReflection,
    b: AggregatedReflection
  ) => AggregatedReflection;
  getTopSubscales: (aggregated: AggregatedReflection, n?: number) => [string, number][];
  getPrimarySubscale: (aggregated: AggregatedReflection) => string | undefined;
  hasSufficientData: (
    aggregated: AggregatedReflection,
    minMoments?: number,
    minSubscales?: number
  ) => boolean;
  createEmptyAggregation: (subscales?: SubscaleDefinition[]) => AggregatedReflection;

  // Interpretation
  interpretReflection: (
    instrumentId: string,
    aggregated: AggregatedReflection,
    options?: InterpretReflectionOptions
  ) => PsychometricInterpretation;
  interpretWithInstrument: (
    instrument: PsychometricInstrument,
    aggregated: AggregatedReflection,
    options?: InterpretReflectionOptions
  ) => PsychometricInterpretation;
}

/**
 * Psychometric instrument definition.
 */
export interface PsychometricInstrument {
  id: string;
  name: string;
  category:
    | 'personality'
    | 'values'
    | 'interests'
    | 'skills'
    | 'vocational'
    | 'wellbeing'
    | 'learning'
    | 'custom';
  subscales: SubscaleDefinition[];
  scoringConfig: ScoringConfig;
  interpret: (aggregated: AggregatedReflection) => unknown;
  resultTypes?: ResultTypeDefinition[];
  description?: string;
  version?: string;
}

export interface SubscaleDefinition {
  id: string;
  name: string;
  description?: string;
  dimension?: string;
  color?: string;
  icon?: string;
}

export interface ResultTypeDefinition {
  id: string;
  name: string;
  description?: string;
  subscaleProfile?: Record<string, number>;
}

export interface ScoringConfig {
  method: 'highest-subscale' | 'threshold-based' | 'profile-matching' | 'dimensional' | 'custom';
  normalize?: boolean;
  thresholds?: Record<string, number>;
}

export interface CreateInstrumentOptions {
  id: string;
  name: string;
  category: PsychometricInstrument['category'];
  subscales: SubscaleDefinition[];
  scoringConfig: ScoringConfig;
  interpret?: (aggregated: AggregatedReflection) => unknown;
  resultTypes?: ResultTypeDefinition[];
  description?: string;
  version?: string;
}

export interface ReflectionMoment {
  id: string;
  purpose: 'reflection' | 'invitation';
  subscaleContributions?: Record<string, Record<string, Record<string, number>>>;
  questionType?: string;
  isReversed?: boolean;
  weight?: number;
}

export interface ReflectionRecognition {
  momentId: string;
  purpose: 'reflection';
  userInput: Record<string, unknown>;
  reflection?: {
    subscaleContributions: Record<string, number>;
    selectedChoiceIds?: string[];
    rawValue?: number;
    openResponse?: string;
  };
  timestamp: number;
}

export interface AggregatedReflection {
  subscaleTotals: Record<string, number>;
  subscaleCounts: Record<string, number>;
  normalizedScores: Record<string, number>;
  momentCount: number;
  momentIds: string[];
  aggregatedAt: number;
}

export interface AggregateReflectionOptions {
  normalization?: 'none' | 'max' | 'sum' | 'count';
  includeEmpty?: boolean;
  subscales?: SubscaleDefinition[];
}

export interface InterpretReflectionOptions {
  minConfidence?: number;
  includeInterpretation?: boolean;
  includeGrowthAreas?: boolean;
  metadata?: Record<string, unknown>;
}

export interface PsychometricInterpretation {
  id: string;
  instrumentId: string;
  category: string;
  primaryType?: TypeResult;
  secondaryTypes?: TypeResult[];
  profile?: DimensionalProfile;
  summary: string;
  interpretation?: string;
  aggregatedReflection: AggregatedReflection;
  interpretedAt: number;
  confidence: number;
}

export interface TypeResult {
  typeId: string;
  typeName: string;
  confidence: number;
  description?: string;
}

export interface DimensionalProfile {
  dimensions: Record<string, number>;
  descriptions?: Record<string, string>;
}

/**
 * Get the Psyche API from the loaded Sophia element.
 *
 * The sophia-element UMD bundle exports to window.SophiaElement.
 * This function provides a facade that combines available SophiaElement
 * exports with local implementations for missing aggregation functions.
 *
 * Returns null if the sophia element hasn't been loaded yet.
 */
export function getPsycheAPI(): PsycheAPI | null {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const sophiaElement = (globalThis as any).SophiaElement;

  if (!sophiaElement) {
    return null;
  }

  // Get getPrimarySubscale from SophiaElement (re-exported from sophia-core)
  const getPrimarySubscaleFn = sophiaElement.getPrimarySubscale;
  if (!getPrimarySubscaleFn) {
    // getPrimarySubscale not available in this version of SophiaElement
    console.debug('[SophiaLoader] getPrimarySubscale not available in SophiaElement');
  }

  // Create a facade using available SophiaElement exports + local implementations
  return {
    // Local implementation for reflection aggregation
    aggregateReflections: (
      recognitions: ReflectionRecognition[],
      _options?: AggregateReflectionOptions
    ): AggregatedReflection => {
      const subscaleTotals: Record<string, number> = {};
      const subscaleCounts: Record<string, number> = {};
      const momentIds: string[] = [];

      for (const recognition of recognitions) {
        momentIds.push(recognition.momentId);
        const contributions = recognition.reflection?.subscaleContributions ?? {};

        for (const [subscale, value] of Object.entries(contributions)) {
          subscaleTotals[subscale] = (subscaleTotals[subscale] ?? 0) + value;
          subscaleCounts[subscale] = (subscaleCounts[subscale] ?? 0) + 1;
        }
      }

      // Normalize scores (sum normalization)
      const total = Object.values(subscaleTotals).reduce((sum, v) => sum + v, 0) || 1;
      const normalizedScores: Record<string, number> = {};
      for (const [subscale, value] of Object.entries(subscaleTotals)) {
        normalizedScores[subscale] = value / total;
      }

      return {
        subscaleTotals,
        subscaleCounts,
        normalizedScores,
        momentCount: recognitions.length,
        momentIds,
        aggregatedAt: Date.now(),
      };
    },

    // Use SophiaElement export if available, otherwise local implementation
    getPrimarySubscale: (aggregated: AggregatedReflection): string | undefined => {
      if (getPrimarySubscaleFn) {
        // SophiaElement's getPrimarySubscale works with subscaleTotals
        return getPrimarySubscaleFn(aggregated.subscaleTotals);
      }
      // Fallback: find highest scoring subscale
      const entries = Object.entries(aggregated.subscaleTotals);
      if (entries.length === 0) return undefined;
      const [firstEntry, ...restEntries] = entries;
      return restEntries.reduce((max, curr) => (curr[1] > max[1] ? curr : max), firstEntry)[0];
    },

    // Local implementation for data sufficiency check
    hasSufficientData: (
      aggregated: AggregatedReflection,
      minMoments = 3,
      minSubscales = 1
    ): boolean => {
      return (
        aggregated.momentCount >= minMoments &&
        Object.keys(aggregated.subscaleTotals).length >= minSubscales
      );
    },

    // Stub implementations for interface completeness
    // These are not needed for basic discovery/reflection flow
    registerInstrument: () => undefined,
    updateInstrument: () => undefined,
    unregisterInstrument: () => false,
    getInstrument: () => undefined,
    getAllInstruments: () => [],
    hasInstrument: () => false,
    clearInstruments: () => undefined,
    createInstrument: () => {
      throw new Error('Sophia adapter: createInstrument not implemented');
    },
    recognizeReflection: () => {
      throw new Error('Sophia adapter: recognizeReflection not implemented');
    },
    mergeAggregatedReflections: (a, b) => ({
      subscaleTotals: { ...a.subscaleTotals, ...b.subscaleTotals },
      subscaleCounts: { ...a.subscaleCounts, ...b.subscaleCounts },
      normalizedScores: { ...a.normalizedScores, ...b.normalizedScores },
      momentCount: a.momentCount + b.momentCount,
      momentIds: [...a.momentIds, ...b.momentIds],
      aggregatedAt: Date.now(),
    }),
    getTopSubscales: (aggregated, n = 3) => {
      return Object.entries(aggregated.subscaleTotals)
        .sort(([, a], [, b]) => b - a)
        .slice(0, n);
    },
    createEmptyAggregation: () => ({
      subscaleTotals: {},
      subscaleCounts: {},
      normalizedScores: {},
      momentCount: 0,
      momentIds: [],
      aggregatedAt: Date.now(),
    }),
    interpretReflection: () => {
      throw new Error('Not implemented');
    },
    interpretWithInstrument: () => {
      throw new Error('Not implemented');
    },
  } as PsycheAPI;
}

/**
 * Get the Psyche API, throwing if not available.
 * Use this when you know the sophia element should be loaded.
 */
export function requirePsycheAPI(): PsycheAPI {
  const api = getPsycheAPI();
  if (!api) {
    throw new Error('Sophia element not loaded - call registerSophiaElement() first');
  }
  return api;
}
