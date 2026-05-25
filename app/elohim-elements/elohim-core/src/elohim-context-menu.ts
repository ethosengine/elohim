import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, queryAll, state } from 'lit/decorators.js';

export interface ContextMenuItem {
  id: string;
  label: string;
  disabled?: boolean;
}

/**
 * <elohim-context-menu> — Google Drive-style fold-down menu.
 *
 * MVP: flat list of items. Submenu support (View as..., Where this
 * leads...) deferred to a fast-follow shift per design spec §7.4.
 *
 * Accessibility: arrow-key navigation, Enter to select, Escape to close,
 * focus trap while open, ARIA menu role, focus restoration on close.
 *
 * @element elohim-context-menu
 *
 * @prop {boolean} open  - Whether the menu is visible
 * @prop {ContextMenuItem[]} items - The menu items
 *
 * @fires item-select - { detail: { id: string } } when an item is selected
 * @fires close       - When the menu closes (any reason)
 *
 * @cssprop --elohim-menu-bg     - Override background
 * @cssprop --elohim-menu-border - Override border color
 * @cssprop --elohim-menu-radius - Override border-radius
 * @cssprop --elohim-menu-shadow - Override drop shadow
 *
 * @csspart menu - The <ul> element
 * @csspart item - Each <li> menu item
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
export class ElohimContextMenu extends LitElement {
  @property({ type: Boolean, reflect: true }) open = false;
  @property({ type: Array }) items: ContextMenuItem[] = [];

  @state() private focusedIndex = -1;
  @queryAll('[role="menuitem"]') private menuItems!: NodeListOf<HTMLElement>;

  private previouslyFocused: HTMLElement | null = null;

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
      background: var(--elohim-menu-bg, Canvas);
      border: 1px solid var(--elohim-menu-border, color-mix(in oklch, currentColor 15%, transparent));
      border-radius: var(--elohim-menu-radius, 6px);
      box-shadow: var(--elohim-menu-shadow, 0 4px 12px rgba(0, 0, 0, 0.12));
      min-inline-size: 180px;
      padding-block: 0.25rem;
      padding-inline: 0;
    }

    :host([hidden]) {
      display: none;
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      :host([open]) {
        animation: elohim-menu-fold-down 120ms ease-out;
      }

      @keyframes elohim-menu-fold-down {
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

      [role='menuitem']:hover,
      [role='menuitem']:focus {
        background: Highlight;
        color: HighlightText;
        outline: none;
      }
    }

    [role='menu'] {
      list-style: none;
      margin: 0;
      padding: 0;
    }

    [role='menuitem'] {
      display: block;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      cursor: pointer;
      user-select: none;
      color: inherit;
      font: inherit;
      min-block-size: 44px;
      display: flex;
      align-items: center;
    }

    [role='menuitem']:hover,
    [role='menuitem']:focus {
      background: color-mix(in oklch, currentColor 8%, transparent);
      outline: none;
    }

    [role='menuitem'][aria-disabled='true'] {
      opacity: 0.5;
      cursor: default;
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    document.addEventListener('mousedown', this.clickOutsideHandler);
    this.setAttribute('role', 'presentation');
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    document.removeEventListener('mousedown', this.clickOutsideHandler);
  }

  protected override willUpdate(changed: PropertyValues): void {
    if (changed.has('open') && this.open) {
      // Capture focus and set initial index before render, so the first render
      // already has the correct tabindex — no second update cycle.
      this.previouslyFocused = (document.activeElement as HTMLElement) ?? null;
      this.focusedIndex = this.firstEnabledIndex();
    }
  }

  protected override updated(changed: PropertyValues): void {
    if (changed.has('open')) {
      if (this.open) {
        // Actually move focus to the first enabled item after the DOM is ready.
        queueMicrotask(() => this.focusItem(this.focusedIndex));
      } else if (changed.get('open') === true) {
        this.previouslyFocused?.focus?.();
      }
    }
  }

  override render() {
    return html`
      <ul role="menu" part="menu" @keydown=${this.handleKeydown}>
        ${this.items.map(
          (item, i) => html`
            <li
              role="menuitem"
              part="item"
              tabindex=${i === this.focusedIndex ? '0' : '-1'}
              aria-disabled=${item.disabled ? 'true' : 'false'}
              @click=${() => this.select(item)}
            >
              ${item.label}
            </li>
          `,
        )}
      </ul>
    `;
  }

  private firstEnabledIndex(): number {
    return this.items.findIndex((it) => !it.disabled);
  }

  private focusItem(i: number): void {
    if (i < 0) return;
    const items = Array.from(this.menuItems);
    items[i]?.focus();
  }

  private readonly handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.handleClose();
      return;
    }
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const item = this.items[this.focusedIndex];
      if (item) this.select(item);
      return;
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      this.moveFocus(e.key === 'ArrowDown' ? 1 : -1);
    }
  };

  private moveFocus(delta: number): void {
    const n = this.items.length;
    if (n === 0) return;
    let next = this.focusedIndex;
    for (let step = 0; step < n; step += 1) {
      next = (next + delta + n) % n;
      if (!this.items[next]!.disabled) break;
    }
    this.focusedIndex = next;
    this.focusItem(next);
  }

  private select(item: ContextMenuItem): void {
    if (item.disabled) return;
    this.dispatchEvent(
      new CustomEvent('item-select', {
        detail: { id: item.id },
        bubbles: true,
        composed: true,
      }),
    );
    this.handleClose();
  }

  private handleClose(): void {
    this.open = false;
    this.dispatchEvent(new CustomEvent('close', { bubbles: true, composed: true }));
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-context-menu': ElohimContextMenu;
  }
}
