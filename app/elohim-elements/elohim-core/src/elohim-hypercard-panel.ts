import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

import type { ContextMenuItem } from './elohim-context-menu.js';

export type { ContextMenuItem } from './elohim-context-menu.js';

/**
 * <elohim-hypercard-panel> — anchored fold-down hypercard panel.
 *
 * A generic fold-down surface providing a default slot for arbitrary hypercard
 * content and an optional action row below the slot. Intended as the
 * progressive L2 disclosure surface from a position:relative anchor wrapper
 * (the same .menu-anchor convention as <elohim-epr-link>).
 *
 * Positioning: :host([open]) is position:absolute; inset-block-start:
 * calc(100% + 0.25rem); inset-inline-start:0 — folds DOWN + inline-start
 * aligned (down-right LTR, down-left RTL). Top-chrome affordances conventionally
 * fold downward so they render on-screen under fixed top-edge chrome.
 * The inline axis is the host's call (the host knows where its chrome sits;
 * the element can't): `align="end"` swaps the pin to inset-inline-end:0 so a
 * panel anchored in END-side chrome grows INTO the viewport instead of off
 * its edge — the 2026-06-12 phone-viewport regression class. Both inline-size
 * defaults clamp to calc(100vw - 1rem) so the box itself can never exceed
 * the screen.
 *
 * Accessibility: non-modal role="dialog" + aria-label from panelLabel; Escape
 * closes from anywhere inside; click-outside closes; on open focus moves into
 * the panel (first enabled action button, else the panel container with
 * tabindex="-1"); on close focus restores to the previously focused element.
 *
 * Motion: fold-down animation (120ms ease-out) gated on
 * (prefers-reduced-motion: no-preference) and (update: fast).
 *
 * @element elohim-hypercard-panel
 *
 * @prop {boolean} open       - Whether the panel is visible (reflected attribute)
 * @prop {ContextMenuItem[]} actions - Action row items; row is absent when empty
 * @prop {string} panelLabel  - aria-label for the dialog; host-supplied — the
 *   element ships NO built-in strings
 * @prop {'start'|'end'} align - Inline-axis pin (reflected). 'start' (default)
 *   pins inset-inline-start:0 to the anchor wrap; 'end' pins inset-inline-end:0
 *   — set by hosts whose anchor sits in end-side chrome so the fold-down grows
 *   into the viewport. Logical-property correct in RTL.
 *
 * @fires action-select - { detail: { id: string } } when an action is activated.
 *   The panel stays open — selection is intent, not dismissal; the HOST decides
 *   whether to close (set open=false), per the §11.1 boundary discipline. This
 *   is what makes in-place card flips possible (host swaps slotted content on
 *   selection without the panel closing underneath it).
 * @fires close         - When the panel closes (Escape / click-outside), bubbles + composed
 *
 * @slot - Arbitrary hypercard content rendered above the action row
 *
 * @cssprop --elohim-hypercard-bg         - Panel background (default: Canvas)
 * @cssprop --elohim-hypercard-border     - Panel border (default: 1px solid color-mix(in oklch, currentColor 15%, transparent))
 * @cssprop --elohim-hypercard-radius     - Panel border-radius (default: 6px)
 * @cssprop --elohim-hypercard-shadow     - Panel drop shadow (default: 0 4px 12px rgba(0,0,0,0.12))
 * @cssprop --elohim-hypercard-min-width  - Minimum inline size (default: min(240px, calc(100vw - 1rem)))
 * @cssprop --elohim-hypercard-max-width  - Maximum inline size (default: min(22rem, calc(100vw - 1rem)))
 * @cssprop --elohim-hypercard-z         - z-index when open (default: 1000)
 *
 * @csspart container   - The outer panel container (role="dialog")
 * @csspart content     - The slot wrapper div
 * @csspart action-row  - The action row container (only present when actions is non-empty)
 * @csspart action      - Each action button in the action row
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimHypercardPanel extends LitElement {
  @property({ type: Boolean, reflect: true }) open = false;
  @property({ type: Array }) actions: ContextMenuItem[] = [];
  @property({ type: String }) panelLabel = '';
  @property({ type: String, reflect: true }) align: 'start' | 'end' = 'start';

  @state() private focusedActionIndex = -1;

  private previouslyFocused: HTMLElement | null = null;
  private containerEl: HTMLElement | null = null;

  private readonly clickOutsideHandler = (e: MouseEvent) => {
    if (!this.open) return;
    const path = e.composedPath();
    if (!path.includes(this)) this.handleClose();
  };

  static override readonly styles = css`
    :host {
      display: none;
    }

    :host([open]) {
      display: block;
      position: absolute;
      inset-block-start: calc(100% + 0.25rem);
      inset-inline-start: 0;
      z-index: var(--elohim-hypercard-z, 1000);
      background: var(--elohim-hypercard-bg, Canvas);
      border: var(
        --elohim-hypercard-border,
        1px solid color-mix(in oklch, currentColor 15%, transparent)
      );
      border-radius: var(--elohim-hypercard-radius, 6px);
      box-shadow: var(--elohim-hypercard-shadow, 0 4px 12px rgb(0 0 0 / 12%));
      min-inline-size: var(--elohim-hypercard-min-width, min(240px, calc(100vw - 1rem)));
      max-inline-size: var(--elohim-hypercard-max-width, min(22rem, calc(100vw - 1rem)));
    }

    /* End-side chrome: the host pins the panel's inline-END edge to the
       anchor so the fold-down grows into the viewport, not off its edge. */
    :host([open][align='end']) {
      inset-inline-start: auto;
      inset-inline-end: 0;
    }

    :host([hidden]) {
      display: none;
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      :host([open]) {
        animation: elohim-hypercard-fold-down 120ms ease-out;
      }

      @keyframes elohim-hypercard-fold-down {
        from {
          opacity: 0;
          transform: translateY(-4px) scaleY(0.95);
          transform-origin: top;
        }

        to {
          opacity: 1;
          transform: translateY(0) scaleY(1);
        }
      }
    }

    @media (forced-colors: active) {
      :host([open]) {
        border-color: CanvasText;
        background: Canvas;
      }

      .action-btn:hover,
      .action-btn:focus {
        background: Highlight;
        color: HighlightText;
        outline: none;
      }

      .action-btn:disabled,
      .action-btn[aria-disabled='true'] {
        color: GrayText;
      }
    }

    [part='container'] {
      outline: none;
    }

    [part='content'] {
      padding-block: 0.75rem;
      padding-inline: 1rem;
    }

    [part='action-row'] {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border-block-start: 1px solid color-mix(in oklch, currentcolor 12%, transparent);
    }

    .action-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding-block: 0.5rem;
      padding-inline: 0.875rem;
      min-block-size: 44px;
      min-inline-size: 44px;
      cursor: pointer;
      user-select: none;
      color: inherit;
      font: inherit;
      background: transparent;
      border: 1px solid color-mix(in oklch, currentcolor 20%, transparent);
      border-radius: var(--elohim-hypercard-radius, 6px);
    }

    .action-btn:hover:not(:disabled, [aria-disabled='true']) {
      background: color-mix(in oklch, currentcolor 8%, transparent);
    }

    .action-btn:focus {
      outline: 2px solid currentcolor;
      outline-offset: 2px;
    }

    .action-btn:disabled,
    .action-btn[aria-disabled='true'] {
      opacity: 0.5;
      cursor: default;
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    document.addEventListener('mousedown', this.clickOutsideHandler);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    document.removeEventListener('mousedown', this.clickOutsideHandler);
  }

  protected override willUpdate(changed: PropertyValues): void {
    if (changed.has('open') && this.open) {
      this.previouslyFocused = (document.activeElement as HTMLElement) ?? null;
      this.focusedActionIndex = this.firstEnabledActionIndex();
    }
  }

  protected override updated(changed: PropertyValues): void {
    if (changed.has('open')) {
      if (this.open) {
        queueMicrotask(() => this.moveFocusIn());
      } else if (changed.get('open') === true) {
        this.previouslyFocused?.focus?.();
      }
    } else if (changed.has('actions') && this.open) {
      // The action set changed while open (e.g. an in-place flip consumed the
      // activating button). If that dropped focus to <body>, pull it back
      // inside so Escape keeps working; never steal focus the user moved
      // elsewhere deliberately.
      const active = document.activeElement;
      if (active === null || active === document.body) {
        this.focusedActionIndex = this.firstEnabledActionIndex();
        queueMicrotask(() => this.moveFocusIn());
      }
    }
  }

  override render() {
    const hasActions = this.actions.length > 0;
    return html`
      <div
        role="dialog"
        aria-label=${this.panelLabel}
        part="container"
        tabindex="-1"
        @keydown=${this.handleKeydown}
      >
        <div part="content">
          <slot></slot>
        </div>
        ${hasActions
          ? html`
              <div part="action-row" role="group">
                ${this.actions.map(
                  (item, i) => html`
                    <button
                      type="button"
                      class="action-btn"
                      part="action"
                      data-action-id=${item.id}
                      ?disabled=${item.disabled ?? false}
                      aria-disabled=${item.disabled ? 'true' : 'false'}
                      tabindex=${i === this.focusedActionIndex ? '0' : '-1'}
                      @click=${() => this.selectAction(item)}
                    >
                      ${item.label}
                    </button>
                  `
                )}
              </div>
            `
          : null}
      </div>
    `;
  }

  private firstEnabledActionIndex(): number {
    return this.actions.findIndex(it => !it.disabled);
  }

  private moveFocusIn(): void {
    const firstEnabled = this.firstEnabledActionIndex();
    if (firstEnabled >= 0) {
      const btn = this.shadowRoot?.querySelectorAll<HTMLButtonElement>('.action-btn')[firstEnabled];
      btn?.focus();
    } else {
      // No enabled action buttons — focus the container
      this.containerEl = this.shadowRoot?.querySelector('[part="container"]') as HTMLElement | null;
      this.containerEl?.focus();
    }
  }

  private readonly handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.handleClose();
    }
  };

  private selectAction(item: ContextMenuItem): void {
    if (item.disabled) return;
    // Selection is intent, not dismissal: the panel stays open and the host
    // decides whether to close (set open=false). Auto-closing here would race
    // the host's in-place card flip — the close-reset would always win.
    this.dispatchEvent(
      new CustomEvent('action-select', {
        detail: { id: item.id },
        bubbles: true,
        composed: true,
      })
    );
  }

  private handleClose(): void {
    this.open = false;
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }));
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-hypercard-panel': ElohimHypercardPanel;
  }
}
