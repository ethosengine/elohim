/**
 * Perseus Custom Element - React-to-Angular bridge.
 *
 * This module creates a Web Component (Custom Element) that wraps the
 * Perseus ItemRenderer React component, allowing it to be used in Angular.
 *
 * The custom element provides:
 * - Setting the Perseus item via property
 * - Scoring the current answer
 * - Event callbacks for state changes
 *
 * @example
 * ```html
 * <perseus-question></perseus-question>
 * ```
 *
 * ```typescript
 * const element = document.querySelector('perseus-question');
 * element.item = perseusItemData;
 * element.onScore = (result) => console.log(result);
 * const score = element.score();
 * ```
 */

import React, { useRef, useCallback, useState, useEffect } from 'react';
import { createRoot, Root } from 'react-dom/client';
import type { PerseusItem, PerseusScoreResult } from './perseus-item.model';

// Perseus module loaded lazily to avoid initialization errors blocking custom element registration
let PerseusModule: typeof import('@khanacademy/perseus') | null = null;
let perseusLoadError: Error | null = null;
let perseusLoadPromise: Promise<void> | null = null;

/**
 * Initialize Perseus dependencies once when the module is first loaded.
 */
function initializePerseusDepedencies(module: typeof import('@khanacademy/perseus')) {
  const { Dependencies } = module;
  if (Dependencies && typeof Dependencies.setDependencies === 'function') {
    console.log('[Perseus] Setting up dependencies...');
    (Dependencies.setDependencies as (deps: unknown) => void)({
      // Minimal dependencies for standalone rendering
      TeX: ({ children }: { children: React.ReactNode }) =>
        React.createElement('span', { className: 'perseus-tex' }, children),
      // JIPT (Just-In-Place Translation) - disabled for standalone use
      JIPT: {
        useJIPT: false,
      },
      // Logging - use console
      Log: {
        log: console.log.bind(console),
        error: console.error.bind(console),
        warn: console.warn.bind(console),
      },
      // Analytics - no-op for standalone
      analytics: {
        onAnalyticsEvent: () => {},
      },
      // Static URL for assets (return as-is)
      staticUrl: (url: string) => url,
      // Other settings
      isDevServer: false,
      kaLocale: 'en',
      isMobile: false,
    });
    console.log('[Perseus] Dependencies configured');
  }
}

/**
 * Lazily load the Perseus module.
 * This happens on first render, not at bundle initialization time.
 */
async function ensurePerseusLoaded(): Promise<typeof import('@khanacademy/perseus') | null> {
  if (PerseusModule) return PerseusModule;
  if (perseusLoadError) return null;

  if (!perseusLoadPromise) {
    perseusLoadPromise = (async () => {
      try {
        console.log('[Perseus] Loading @khanacademy/perseus module...');
        // Dynamic import - happens at runtime, not bundle initialization
        const module = await import('@khanacademy/perseus');
        if (module && typeof module === 'object') {
          console.log('[Perseus] Module loaded:', Object.keys(module));
          PerseusModule = module;
          // Initialize dependencies immediately after loading
          initializePerseusDepedencies(module);
        } else {
          console.error('[Perseus] Module loaded but is null/undefined');
          perseusLoadError = new Error('Perseus module loaded but is null/undefined');
        }
      } catch (err) {
        console.error('[Perseus] Failed to load Perseus module:', err);
        perseusLoadError = err as Error;
      }
    })();
  }

  await perseusLoadPromise;
  return PerseusModule;
}

// Type Definitions for Perseus (dynamic import)

/**
 * Perseus score type (from @khanacademy/perseus).
 * This is what Renderer.score() returns.
 */
interface PerseusScore {
  type: 'points' | 'invalid';
  earned?: number;
  total?: number;
  message?: string | null;
}

/**
 * Perseus Renderer type (internal renderer with score method).
 */
interface PerseusRenderer {
  score(): PerseusScore;
  getUserInput(): unknown;
  getSerializedState(): unknown;
  restoreSerializedState?(state: unknown): void;
  focus(): boolean | null | undefined;
  blur(): void;
}

/**
 * Perseus renderer API reference type.
 * This is what we get from the ref on ServerItemRenderer.
 * Note: ServerItemRenderer exposes `questionRenderer` which is a Renderer instance.
 */
interface PerseusRendererAPI {
  questionRenderer: PerseusRenderer;
  getSerializedState(): unknown;
  restoreSerializedState?(state: unknown): void;
  focus?(): boolean | null | undefined;
  blur?(): void;
}

/**
 * Transform Perseus score format to our expected format.
 */
function transformPerseusScore(score: PerseusScore, userInput: unknown): PerseusScoreResult {
  if (score.type === 'invalid') {
    return {
      correct: false,
      score: 0,
      guess: userInput,
      empty: true,
      message: score.message ?? undefined,
    };
  }

  const earned = score.earned ?? 0;
  const total = score.total ?? 1;

  return {
    correct: earned >= total,
    score: total > 0 ? earned / total : 0,
    guess: userInput,
    empty: false,
    message: score.message ?? undefined,
  };
}

/**
 * Props for the internal React wrapper component.
 */
interface PerseusItemWrapperProps {
  item: PerseusItem | null;
  onScore?: (result: PerseusScoreResult) => void;
  onAnswerChange?: (hasAnswer: boolean) => void;
  apiRef?: React.MutableRefObject<PerseusRendererAPI | null>;
  reviewMode?: boolean;
}

// React Wrapper Component

/**
 * Internal React component that renders Perseus ItemRenderer.
 *
 * This component handles:
 * - Dynamic loading of Perseus modules
 * - API reference for scoring
 * - Event propagation for answer changes
 */
function PerseusItemWrapper({
  item,
  onScore,
  onAnswerChange,
  apiRef,
  reviewMode = false
}: PerseusItemWrapperProps): JSX.Element {
  const [initialized, setInitialized] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const rendererRef = useRef<PerseusRendererAPI | null>(null);

  // Debug: log render state
  console.log('[PerseusItemWrapper] Render:', {
    hasItem: !!item,
    itemId: item?.id || 'null',
    hasQuestion: !!item?.question,
    hasWidgets: !!item?.question?.widgets,
    initialized,
    loading,
    hasPerseusModule: !!PerseusModule,
    hasServerItemRenderer: !!PerseusModule?.ServerItemRenderer,
    error,
  });

  // Initialize Perseus on mount (lazy load the module)
  useEffect(() => {
    let cancelled = false;

    async function loadPerseus() {
      console.log('[PerseusItemWrapper] Starting lazy load of Perseus module...');
      setLoading(true);

      const module = await ensurePerseusLoaded();

      if (cancelled) return;

      console.log('[PerseusItemWrapper] PerseusModule loaded:', module ? Object.keys(module) : 'null');

      if (!module || !module.ServerItemRenderer) {
        console.error('[PerseusItemWrapper] Perseus module missing ServerItemRenderer');
        setError(perseusLoadError?.message || 'Perseus module not properly loaded');
      }

      setLoading(false);
      setInitialized(true);
    }

    loadPerseus();

    return () => { cancelled = true; };
  }, []);

  // Expose API ref for external scoring
  useEffect(() => {
    if (apiRef) {
      apiRef.current = rendererRef.current;
    }
  }, [apiRef, rendererRef.current]);

  // Handle answer changes
  const handleChange = useCallback(
    (state: { hasBeenInteractedWith: boolean; hasContent: boolean }) => {
      if (onAnswerChange) {
        onAnswerChange(state.hasContent);
      }
    },
    [onAnswerChange]
  );

  if (loading || !initialized) {
    return (
      <div className="perseus-loading">
        <div className="perseus-loading-spinner" />
        <span>Loading Perseus quiz engine...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="perseus-error">
        <span className="perseus-error-icon">!</span>
        <span>{error}</span>
      </div>
    );
  }

  if (!item) {
    return (
      <div className="perseus-empty">
        <span>No question loaded</span>
      </div>
    );
  }

  if (!PerseusModule?.ServerItemRenderer) {
    return (
      <div className="perseus-error">
        <span className="perseus-error-icon">!</span>
        <span>Perseus renderer not available</span>
      </div>
    );
  }

  // Perseus requires specific structure for rendering
  // Dependencies are initialized once in initializePerseusDepedencies() when module loads
  const { ServerItemRenderer, PerseusI18nContextProvider } = PerseusModule;

  // Dependencies for DependenciesContext (used by ErrorBoundary, widgets, etc.)
  // Note: This must match PerseusDependenciesV2 interface
  const dependenciesV2 = {
    analytics: {
      onAnalyticsEvent: (...args: unknown[]) => {
        console.log('[Perseus] analytics.onAnalyticsEvent called:', args);
      },
    },
    generateUrl: ({ path, query }: { path: string; query?: Record<string, string> }) => {
      // Simple URL generation - append query params if provided
      if (query) {
        const params = new URLSearchParams(query).toString();
        return params ? `${path}?${params}` : path;
      }
      return path;
    },
    useVideo: () => ({ status: 'success' as const, data: { video: null } }),
  };

  // Default i18n strings for Perseus - comprehensive set for quiz rendering
  const defaultStrings = {
    // Keypad
    closeKeypad: 'Close keypad',
    openKeypad: 'Open keypad',
    mathInputBox: 'Math input box',
    mathInputTitle: 'Math input',
    mathInputDescription: 'Use keyboard/mouse to interact with math-based input fields',

    // Highlights
    removeHighlight: 'Remove highlight',
    addHighlight: 'Add highlight',

    // Hints
    hintPos: ({ pos }: { pos: number }) => `Hint ${pos}`,
    hints: 'Hints',
    getAnotherHint: 'Get another hint',

    // Errors
    errorRendering: ({ error }: { error: string }) => `Error rendering: ${error}`,
    APPROXIMATED_PI_ERROR: 'Your answer is close, but you may have approximated pi. Enter your answer as a multiple of pi, like 12 pi or 2/3 pi',
    EXTRA_SYMBOLS_ERROR: 'We could not understand your answer. Please check your answer for extra text or symbols.',
    NEEDS_TO_BE_SIMPLFIED_ERROR: 'Your answer is almost correct, but it needs to be simplified.',
    MISSING_PERCENT_ERROR: 'Your answer is almost correct, but it is missing a percent sign.',
    MULTIPLICATION_SIGN_ERROR: 'I couldn\'t understand that. Please use \'*\' to multiply.',
    WRONG_CASE_ERROR: 'Your answer includes use of a variable with the wrong case.',
    WRONG_LETTER_ERROR: 'Your answer includes a wrong variable letter.',

    // Radio/Multiple Choice widget
    letters: 'A B C D E F G H I J K L M N O P Q R S T U V W X Y Z',
    chooseOneAnswer: 'Choose 1 answer:',
    chooseAllAnswers: 'Choose all answers that apply:',
    chooseNumAnswers: ({ numCorrect }: { numCorrect: string }) => `Choose ${numCorrect} answers:`,
    choice: ({ letter }: { letter: string }) => `(Choice ${letter})`,
    choiceChecked: ({ letter }: { letter: string }) => `(Choice ${letter}, Checked)`,
    choiceCorrect: ({ letter }: { letter: string }) => `(Choice ${letter}, Correct)`,
    choiceIncorrect: ({ letter }: { letter: string }) => `(Choice ${letter}, Incorrect)`,
    choiceCheckedCorrect: ({ letter }: { letter: string }) => `(Choice ${letter}, Checked, Correct)`,
    choiceCheckedIncorrect: ({ letter }: { letter: string }) => `(Choice ${letter}, Checked, Incorrect)`,
    noneOfTheAbove: 'None of the above',

    // Boolean choices
    false: 'False',
    true: 'True',
    no: 'No',
    yes: 'Yes',

    // Feedback
    correct: 'Correct',
    incorrect: 'Incorrect',
    correctExcited: 'Correct!',
    keepTrying: 'Keep trying',
    tryAgain: 'Try again',
    check: 'Check',

    // Loading
    loading: 'Loading...',
  };

  // ServerItemRenderer wraps its content in DependenciesContext.Provider using the
  // dependencies prop, so we pass our dependencies directly to it instead of wrapping
  const rendererElement = (
    <ServerItemRenderer
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ref={(ref: any) => {
        rendererRef.current = ref;
        if (apiRef) {
          apiRef.current = ref;
        }
      }}
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      item={item as any}
      problemNum={1}
      reviewMode={reviewMode}
      // Pass dependencies directly - ServerItemRenderer wraps content in DependenciesContext.Provider
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      dependencies={dependenciesV2 as any}
      apiOptions={{
        isMobile: false,
        customKeypad: false,
        readOnly: reviewMode,
        interactionCallback: () => handleChange({ hasBeenInteractedWith: true, hasContent: true }),
      }}
    />
  );

  // Wrap in i18n provider for localization
  return (
    <div className="perseus-item-container">
      {PerseusI18nContextProvider ? (
        <PerseusI18nContextProvider locale="en" strings={defaultStrings}>
          {rendererElement}
        </PerseusI18nContextProvider>
      ) : (
        rendererElement
      )}
    </div>
  );
}

// Custom Element Definition

/**
 * PerseusQuestionElement - Web Component wrapper for Perseus.
 *
 * This custom element bridges the React Perseus renderer to the DOM,
 * allowing usage in Angular or any other framework.
 */
class PerseusQuestionElement extends HTMLElement {
  private root: Root | null = null;
  private _item: PerseusItem | null = null;
  private _onScore: ((result: PerseusScoreResult) => void) | null = null;
  private _onAnswerChange: ((hasAnswer: boolean) => void) | null = null;
  private _reviewMode: boolean = false;
  private apiRef: React.MutableRefObject<PerseusRendererAPI | null> = { current: null };

  static get observedAttributes(): string[] {
    return ['review-mode'];
  }

  connectedCallback(): void {
    // Create shadow DOM for style isolation
    if (!this.shadowRoot) {
      this.attachShadow({ mode: 'open' });
    }

    // Add Perseus styles to shadow DOM
    const style = document.createElement('style');
    style.textContent = this.getStyles();
    this.shadowRoot!.appendChild(style);

    // Create mount point for React
    const container = document.createElement('div');
    container.className = 'perseus-mount';
    this.shadowRoot!.appendChild(container);

    // Initialize React root
    this.root = createRoot(container);
    this.render();
  }

  disconnectedCallback(): void {
    if (this.root) {
      this.root.unmount();
      this.root = null;
    }
  }

  attributeChangedCallback(name: string, _oldValue: string, newValue: string): void {
    if (name === 'review-mode') {
      this._reviewMode = newValue !== null && newValue !== 'false';
      this.render();
    }
  }

  // Public Properties

  /**
   * Set the Perseus item to render.
   */
  set item(value: PerseusItem | null) {
    console.log('[PerseusQuestionElement] set item:', {
      hasValue: !!value,
      valueId: value?.id || 'null',
      hasQuestion: !!value?.question,
      hasRoot: !!this.root
    });
    this._item = value;
    this.render();
  }

  get item(): PerseusItem | null {
    return this._item;
  }

  /**
   * Callback when scoring occurs.
   */
  set onScore(callback: ((result: PerseusScoreResult) => void) | null) {
    this._onScore = callback;
    this.render();
  }

  /**
   * Callback when answer state changes.
   */
  set onAnswerChange(callback: ((hasAnswer: boolean) => void) | null) {
    this._onAnswerChange = callback;
    this.render();
  }

  /**
   * Set review mode (read-only, shows correct answers).
   */
  set reviewMode(value: boolean) {
    this._reviewMode = value;
    if (value) {
      this.setAttribute('review-mode', '');
    } else {
      this.removeAttribute('review-mode');
    }
    this.render();
  }

  get reviewMode(): boolean {
    return this._reviewMode;
  }

  // Public Methods

  /**
   * Score the current answer and return the result.
   */
  score(): PerseusScoreResult | null {
    if (!this.apiRef.current) {
      console.warn('[Perseus] Renderer not initialized');
      return null;
    }

    try {
      // Access the questionRenderer property which has the score() method
      const renderer = this.apiRef.current.questionRenderer;
      if (!renderer) {
        console.warn('[Perseus] questionRenderer not available');
        return null;
      }

      // Get the raw Perseus score
      const perseusScore = renderer.score();
      console.log('[Perseus] Raw score:', perseusScore);

      // Get user input for the result
      const userInput = renderer.getUserInput?.() ?? null;

      // Transform to our expected format
      const result = transformPerseusScore(perseusScore, userInput);
      console.log('[Perseus] Transformed result:', result);

      if (this._onScore) {
        this._onScore(result);
      }

      return result;
    } catch (err) {
      console.error('[Perseus] Failed to score item:', err);
      return null;
    }
  }

  /**
   * Focus the question input.
   */
  focusInput(): void {
    if (this.apiRef.current?.questionRenderer) {
      this.apiRef.current.questionRenderer.focus();
    }
  }

  /**
   * Get the serialized state for persistence.
   */
  getState(): unknown {
    if (this.apiRef.current?.questionRenderer) {
      return this.apiRef.current.questionRenderer.getSerializedState();
    }
    return null;
  }

  /**
   * Restore from serialized state.
   */
  restoreState(state: unknown): void {
    if (this.apiRef.current?.questionRenderer?.restoreSerializedState && state) {
      this.apiRef.current.questionRenderer.restoreSerializedState(state);
    }
  }

  // Private Methods

  private render(): void {
    if (!this.root) {
      console.log('[PerseusQuestionElement] render: no root, skipping');
      return;
    }

    console.log('[PerseusQuestionElement] render: calling root.render with item:', this._item?.id || 'null');
    this.root.render(
      <PerseusItemWrapper
        item={this._item}
        onScore={this._onScore ?? undefined}
        onAnswerChange={this._onAnswerChange ?? undefined}
        apiRef={this.apiRef}
        reviewMode={this._reviewMode}
      />
    );
  }

  private getStyles(): string {
    return `
      :host {
        display: block;
        font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
      }

      .perseus-mount {
        padding: 1rem;
      }

      .perseus-loading {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 2rem;
        color: var(--text-secondary, #666);
      }

      .perseus-loading-spinner {
        width: 20px;
        height: 20px;
        border: 2px solid var(--border-color, #e0e0e0);
        border-top-color: var(--primary-color, #1976d2);
        border-radius: 50%;
        animation: spin 1s linear infinite;
      }

      @keyframes spin {
        to { transform: rotate(360deg); }
      }

      .perseus-error {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 1rem;
        background: var(--error-bg, #ffebee);
        color: var(--error-color, #c62828);
        border-radius: 4px;
      }

      .perseus-error-icon {
        font-size: 1.25rem;
      }

      .perseus-empty {
        padding: 2rem;
        text-align: center;
        color: var(--text-secondary, #666);
      }

      .perseus-item-container {
        /* Perseus-specific styling overrides */
      }

      .perseus-tex {
        font-family: 'KaTeX_Main', 'Times New Roman', serif;
      }

      /* Radio widget styling */
      .perseus-item-container :global(.radio-option) {
        padding: 0.75rem 1rem;
        margin: 0.5rem 0;
        border: 1px solid var(--border-color, #e0e0e0);
        border-radius: 8px;
        cursor: pointer;
        transition: border-color 0.2s, background-color 0.2s;
      }

      .perseus-item-container :global(.radio-option:hover) {
        border-color: var(--primary-color, #1976d2);
        background-color: var(--hover-bg, #f5f5f5);
      }

      .perseus-item-container :global(.radio-option.selected) {
        border-color: var(--primary-color, #1976d2);
        background-color: var(--selected-bg, #e3f2fd);
      }

      /* Correct/incorrect feedback */
      .perseus-item-container :global(.correct) {
        border-color: var(--success-color, #4caf50) !important;
        background-color: var(--success-bg, #e8f5e9) !important;
      }

      .perseus-item-container :global(.incorrect) {
        border-color: var(--error-color, #f44336) !important;
        background-color: var(--error-bg, #ffebee) !important;
      }
    `;
  }
}

// Registration

/**
 * Check if the Perseus element is registered.
 */
export function isPerseusElementRegistered(): boolean {
  return !!customElements.get('perseus-question');
}

/**
 * Register the custom element if not already registered.
 */
export function registerPerseusElement(): void {
  console.log('[Perseus Plugin] registerPerseusElement called');
  console.log('[Perseus Plugin] PerseusQuestionElement class:', PerseusQuestionElement);
  console.log('[Perseus Plugin] customElements available:', typeof customElements !== 'undefined');

  if (!customElements.get('perseus-question')) {
    try {
      console.log('[Perseus Plugin] Attempting to define custom element...');
      customElements.define('perseus-question', PerseusQuestionElement);
      console.log('[Perseus Plugin] Custom element registered successfully');
    } catch (err) {
      console.error('[Perseus Plugin] Failed to define custom element:', err);
      throw err;
    }
  } else {
    console.log('[Perseus Plugin] Custom element already registered');
  }
}

/**
 * Type export for the custom element.
 */
export type { PerseusQuestionElement };
