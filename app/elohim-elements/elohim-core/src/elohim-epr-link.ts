import { css, html, LitElement, nothing, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';
import './elohim-skeleton.js';
import './elohim-mention-base.js';
import './elohim-context-menu.js';
import type { ContextMenuItem } from './elohim-context-menu.js';

export type EprLinkDisplay = 'inline' | 'chip' | 'card' | 'popover';

export type EprLinkLoadLevel = 1 | 2 | 3 | 4;

export interface EprLinkResolution {
  title?: string;
  description?: string;
  pillar?: string;
  reach?: string;
  preview?: { title?: string; body?: string };
  unreachable?: boolean;
}

/**
 * <elohim-epr-link> — the protocol's HyperCard navigation primitive.
 *
 * Progressive loading (L1–L4): renders a sized skeleton instantly, fills
 * in title/metadata as Loader resolves, falls back to preview if the
 * target is unreachable.
 *
 * Default click: emits 'navigate' with the EPR ref. Parent decides what
 * to do (card-flip in place, or hard nav, depending on display variant).
 *
 * Right-click: opens <elohim-context-menu> with MVP items (Open / About
 * this EPR / Copy EPR link). Submenu items (View as..., Where this
 * leads...) deferred per spec §7.4.
 *
 * Resolution is decoupled from the loader: production wiring injects a
 * resolver function via the `.resolver` property. For tests / Storybook,
 * direct manipulation of internal state (via `setResolution()`) is the
 * supported test seam.
 *
 * @element elohim-epr-link
 *
 * @prop {string} epr - The epr:... reference
 * @prop {EprLinkDisplay} display - Visual variant (inline | chip | card | popover)
 * @prop {Function | null} resolver - Optional async resolver injected at bundle level
 *
 * @fires navigate - { detail: { epr: string } } on default-click activation
 * @fires about    - { detail: { epr: string } } when "About this EPR" is selected
 *
 * @cssprop --elohim-link-border   - Override chip border color
 * @cssprop --elohim-link-hover-bg - Override hover background
 *
 * @csspart skeleton  - The L1 skeleton placeholder
 * @csspart anchor    - The interactive chip/button rendered at L2/L3
 * @csspart fallback  - The <elohim-mention-base> rendered at L4 (unreachable)
 * @csspart menu      - The <elohim-context-menu>
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:designed, error:designed, stale:n/a, contested:n/a, offline:designed, unauthorized:n/a
 */
export class ElohimEprLink extends LitElement {
  @property() epr = '';
  @property() display: EprLinkDisplay = 'inline';

  /**
   * Optional async resolver. Receives the epr id; returns the resolution
   * object or null. Production: bundle wires this to the Loader + the
   * doorway's /api/v1/epr/{id} endpoint.
   */
  @property({ attribute: false })
  resolver: ((epr: string) => Promise<EprLinkResolution | null>) | null = null;

  @state() private loadLevel: EprLinkLoadLevel = 1;
  @state() private resolution: EprLinkResolution = {};
  @state() private menuOpen = false;

  private readonly menuItems: ContextMenuItem[] = [
    { id: 'open', label: 'Open' },
    { id: 'about', label: 'About this EPR' },
    { id: 'copy', label: 'Copy EPR link' },
  ];

  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    :host([hidden]) {
      display: none;
    }

    .anchor {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.125rem;
      padding-inline: 0.5rem;
      border: 1px solid
        var(--elohim-link-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: 999px;
      cursor: pointer;
      background: none;
      color: inherit;
      font: inherit;
    }

    .anchor:hover,
    .anchor:focus-visible {
      background: var(
        --elohim-link-hover-bg,
        color-mix(in oklch, currentColor 6%, transparent)
      );
      outline: none;
    }

    .menu-anchor {
      position: relative;
      display: inline-block;
    }

    @media (pointer: coarse) {
      .anchor {
        min-block-size: 44px;
        min-inline-size: 44px;
      }
    }

    @media (forced-colors: active) {
      .anchor {
        border-color: CanvasText;
        background: ButtonFace;
        color: ButtonText;
      }

      .anchor:hover,
      .anchor:focus-visible {
        background: Highlight;
        color: HighlightText;
      }
    }
  `;

  protected override updated(changed: PropertyValues): void {
    if (changed.has('epr') || changed.has('resolver')) {
      void this.resolve();
    }
  }

  /**
   * Test seam — directly set resolution state without going through the
   * async resolver. Tests and Storybook use this to exercise all load levels
   * without needing a real network layer.
   */
  setResolution(level: EprLinkLoadLevel, resolution: EprLinkResolution): void {
    this.loadLevel = level;
    this.resolution = resolution;
  }

  override render() {
    if (this.loadLevel === 1) {
      return html`<elohim-skeleton
        width="6rem"
        height="1rem"
        part="skeleton"
      ></elohim-skeleton>`;
    }

    if (this.loadLevel === 4 && this.resolution.unreachable) {
      return html`
        <span class="menu-anchor">
          <elohim-mention-base
            epr=${this.epr}
            label=${this.resolution.preview?.title ?? this.resolution.title ?? ''}
            part="fallback"
          ></elohim-mention-base>
          ${this.renderContextMenu()}
        </span>
      `;
    }

    const label = this.resolution.title ?? this.epr;
    return html`
      <span class="menu-anchor">
        <button
          type="button"
          class="anchor"
          part="anchor"
          @click=${this.handleClick}
          @contextmenu=${this.handleContextMenu}
          @keydown=${this.handleKeydown}
        >
          ${label}
        </button>
        ${this.renderContextMenu()}
      </span>
    `;
  }

  private renderContextMenu() {
    if (!this.menuOpen) return nothing;
    return html`
      <elohim-context-menu
        ?open=${this.menuOpen}
        .items=${this.menuItems}
        part="menu"
        @item-select=${this.handleMenuSelect}
        @close=${this.handleMenuClose}
      ></elohim-context-menu>
    `;
  }

  private readonly handleClick = (e: MouseEvent) => {
    e.preventDefault();
    this.dispatchNavigate();
  };

  private readonly handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    this.menuOpen = true;
  };

  private readonly handleKeydown = (e: KeyboardEvent) => {
    // Shift+F10 opens context menu per accessibility convention.
    if (e.shiftKey && e.key === 'F10') {
      e.preventDefault();
      this.menuOpen = true;
    }
  };

  private readonly handleMenuSelect = (e: Event) => {
    const id = (e as CustomEvent<{ id: string }>).detail.id;
    if (id === 'open') {
      this.dispatchNavigate();
    } else if (id === 'copy') {
      navigator.clipboard?.writeText(this.epr).catch(() => {});
    } else if (id === 'about') {
      this.dispatchEvent(
        new CustomEvent('about', {
          detail: { epr: this.epr },
          bubbles: true,
          composed: true,
        }),
      );
    }
  };

  private readonly handleMenuClose = () => {
    this.menuOpen = false;
  };

  private dispatchNavigate(): void {
    this.dispatchEvent(
      new CustomEvent('navigate', {
        detail: { epr: this.epr },
        bubbles: true,
        composed: true,
      }),
    );
  }

  /**
   * Default resolution flow:
   * - Without a `resolver`, we transition to L2 with empty resolution
   *   (just exposes the EPR id as-is). Tests use `setResolution()`
   *   to advance through levels.
   * - With a `resolver`, we call it and advance to L3 on success,
   *   or L4 with `unreachable: true` on null / rejection.
   */
  private async resolve(): Promise<void> {
    if (!this.epr) return;
    this.loadLevel = 1;
    this.resolution = {};
    if (!this.resolver) {
      this.loadLevel = 2;
      return;
    }
    try {
      const result = await this.resolver(this.epr);
      if (result) {
        this.loadLevel = 3;
        this.resolution = result;
      } else {
        this.loadLevel = 4;
        this.resolution = { unreachable: true };
      }
    } catch {
      this.loadLevel = 4;
      this.resolution = { unreachable: true };
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-epr-link': ElohimEprLink;
  }
}
