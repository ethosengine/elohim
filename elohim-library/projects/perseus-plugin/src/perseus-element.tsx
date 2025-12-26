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

// Type Definitions for Perseus (dynamic import)

/**
 * Perseus renderer API reference type.
 */
interface PerseusRendererAPI {
  scoreInput(): PerseusScoreResult;
  getSerializedState(): unknown;
  restoreSerializedState(state: unknown): void;
  focus(): void;
  blur(): void;
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
  const [PerseusModule, setPerseusModule] = useState<typeof import('@khanacademy/perseus') | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const rendererRef = useRef<PerseusRendererAPI | null>(null);

  // Dynamic import of Perseus to avoid SSR issues
  useEffect(() => {
    let mounted = true;

    async function loadPerseus() {
      try {
        const perseus = await import('@khanacademy/perseus');
        if (mounted) {
          setPerseusModule(perseus);
          setLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(`Failed to load Perseus: ${err instanceof Error ? err.message : 'Unknown error'}`);
          setLoading(false);
        }
      }
    }

    loadPerseus();

    return () => {
      mounted = false;
    };
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

  if (loading) {
    return (
      <div className="perseus-loading">
        <div className="perseus-loading-spinner" />
        <span>Loading question...</span>
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

  if (!item || !PerseusModule) {
    return (
      <div className="perseus-empty">
        <span>No question loaded</span>
      </div>
    );
  }

  // Perseus requires specific structure for rendering
  const { ServerItemRenderer, Dependencies } = PerseusModule;

  // Initialize Perseus dependencies if needed
  // Note: Cast to any to avoid strict type checking on partial dependencies
  if (Dependencies && typeof Dependencies.setDependencies === 'function') {
    (Dependencies.setDependencies as (deps: unknown) => void)({
      // Minimal dependencies for standalone rendering
      TeX: ({ children }: { children: React.ReactNode }) => <span className="perseus-tex">{children}</span>,
      // Add more dependencies as needed for specific widgets
    });
  }

  return (
    <div className="perseus-item-container">
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        dependencies={{} as any}
        apiOptions={{
          isMobile: false,
          customKeypad: false,
          readOnly: reviewMode,
          interactionCallback: () => handleChange({ hasBeenInteractedWith: true, hasContent: true }),
        }}
      />
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
      console.warn('Perseus renderer not initialized');
      return null;
    }

    try {
      const result = this.apiRef.current.scoreInput();

      if (this._onScore) {
        this._onScore(result);
      }

      return result;
    } catch (err) {
      console.error('Failed to score Perseus item:', err);
      return null;
    }
  }

  /**
   * Focus the question input.
   */
  focusInput(): void {
    if (this.apiRef.current) {
      this.apiRef.current.focus();
    }
  }

  /**
   * Get the serialized state for persistence.
   */
  getState(): unknown {
    if (this.apiRef.current) {
      return this.apiRef.current.getSerializedState();
    }
    return null;
  }

  /**
   * Restore from serialized state.
   */
  restoreState(state: unknown): void {
    if (this.apiRef.current && state) {
      this.apiRef.current.restoreSerializedState(state);
    }
  }

  // Private Methods

  private render(): void {
    if (!this.root) return;

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
  if (!customElements.get('perseus-question')) {
    customElements.define('perseus-question', PerseusQuestionElement);
    console.log('[Perseus Plugin] Custom element registered');
  }
}

/**
 * Type export for the custom element.
 */
export type { PerseusQuestionElement };
