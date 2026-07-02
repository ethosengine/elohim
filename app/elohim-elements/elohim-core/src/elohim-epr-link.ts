import { ContextConsumer } from '@lit/context';
import { css, html, LitElement, nothing, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

import './elohim-skeleton.js';
import './elohim-mention-base.js';
import './elohim-context-menu.js';

import {
  eprResolutionContext,
  type EprHeadResolution,
} from './navigation/epr-resolution-provider.js';

import type { ContextMenuItem } from './elohim-context-menu.js';

export type EprLinkDisplay = 'inline' | 'chip' | 'card' | 'popover';

export type EprLinkLoadLevel = 1 | 2 | 3 | 4;

/**
 * Typed resolution outcome. The head resolving is `resolved`; the distinct
 * degraded states let the element render an honest face instead of one
 * `unreachable` collapse:
 * - `forbidden` — the head is gated at the reader's reach ("unavailable at your reach")
 * - `missing`   — the head is 404 (honest missing state)
 * - `error`     — network / decode failure
 */
export type EprLinkState = 'resolved' | 'forbidden' | 'missing' | 'error';

export interface EprLinkResolution {
  title?: string;
  description?: string;
  pillar?: string;
  reach?: string;
  preview?: { title?: string; body?: string };
  /**
   * Typed degradation outcome. When absent, the element infers it: a truthy
   * resolution reads as `resolved`, unless the legacy `unreachable` flag is set.
   */
  state?: EprLinkState;
  /**
   * Legacy degraded flag, superseded by `state`. Retained so existing hosts and
   * tests that set `unreachable: true` still collapse to the degraded fallback
   * face; new callers use `state` instead.
   */
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
 * Right-click: opens <elohim-context-menu>. The default menu is the MVP
 * three (Open / About this EPR / Copy EPR link); a host may inject a
 * fuller action set via the `.contextMenuItems` property (e.g. the Epic E
 * actions: View network & resilience, See relationships, Steward this
 * content, Flag / Challenge / Open feedback). The element stays
 * blank-slate: app-specific routing/governance/stewardship is NOT decided
 * here — every selection is re-emitted as `epr-menu-select` so the host
 * can act on ids it owns. Submenu items (View as..., Where this leads...)
 * deferred per spec §7.4.
 *
 * Resolution is decoupled from the loader two ways, prop over context:
 *   1. an explicit `.resolver` function property (production wiring / tests), or
 *   2. an AMBIENT {@link EprResolutionProvider} consumed via `@lit/context`,
 *      installed once at the surface root — every `<elohim-epr-link>` beneath it
 *      resolves through the provider's head plane with no per-element wiring.
 * The explicit prop always wins over context (backward compatible). For tests /
 * Storybook, direct manipulation of internal state (via `setResolution()`) is
 * the supported test seam.
 *
 * @element elohim-epr-link
 *
 * @prop {string} epr - The epr:... reference
 * @prop {EprLinkDisplay} display - Visual variant (inline | chip | card | popover)
 * @prop {Function | null} resolver - Optional async resolver override; wins over the ambient context provider
 * @prop {ContextMenuItem[] | null} contextMenuItems - Optional host-injected context-menu
 *   action set. When null, the default MVP three (Open / About this EPR / Copy EPR link)
 *   render — preserving standalone/Storybook usability.
 *
 * @fires navigate        - { detail: { epr: string } } on default-click activation
 * @fires about           - { detail: { epr: string } } when "About this EPR" is selected
 * @fires epr-menu-select - { detail: { id: string, epr: string } } for EVERY context-menu
 *   selection, so the host can handle app-specific ids (network / relationships / steward /
 *   flag / challenge / feedback) it injected via `contextMenuItems`
 *
 * @cssprop --elohim-link-border          - Override chip border color
 * @cssprop --elohim-link-hover-bg        - Override hover background
 * @cssprop --elohim-link-unavailable-fg  - Override the degraded "unavailable" note color
 *
 * @csspart skeleton      - The L1 skeleton placeholder
 * @csspart anchor        - The interactive chip/button rendered at L2/L3
 * @csspart fallback      - The <elohim-mention-base> rendered at L4 (degraded)
 * @csspart fallback-note - The honest "unavailable" note rendered beside the L4 fallback
 * @csspart menu          - The <elohim-context-menu>
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:designed, error:designed, stale:n/a, contested:n/a, offline:designed, unauthorized:designed
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

  /**
   * Ambient resolution provider, consumed via `@lit/context` from the surface
   * root. Used only when no explicit `.resolver` prop is set (the prop wins).
   * `subscribe: true` re-fires the callback when the provider arrives or
   * changes so a late-installed provider still resolves the element.
   */
  private readonly _resolutionProvider = new ContextConsumer(this, {
    context: eprResolutionContext,
    subscribe: true,
    callback: (): void => {
      // Re-resolve when the ambient provider arrives/changes — but only when no
      // explicit resolver prop overrides it and there is a ref to resolve.
      if (!this.resolver && this.epr) void this.resolve();
    },
  });

  /**
   * Optional host-injected context-menu action set. When null (default),
   * the element renders the MVP three (Open / About this EPR / Copy EPR
   * link) so it stays usable standalone. The Angular host injects the full
   * Epic E action set here and owns the app-specific handling via the
   * `epr-menu-select` event.
   */
  @property({ attribute: false })
  contextMenuItems: ContextMenuItem[] | null = null;

  @state() private loadLevel: EprLinkLoadLevel = 1;
  @state() private resolution: EprLinkResolution = {};
  @state() private menuOpen = false;

  private get defaultMenuItems(): ContextMenuItem[] {
    return [
      { id: 'open', label: 'Open' },
      { id: 'about', label: 'About this EPR' },
      { id: 'copy', label: 'Copy EPR link' },
    ];
  }

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
      background: var(--elohim-link-hover-bg, color-mix(in oklch, currentColor 6%, transparent));
      outline: none;
    }

    .menu-anchor {
      position: relative;
      display: inline-block;
    }

    .fallback {
      display: inline-flex;
      align-items: baseline;
      gap: 0.375rem;
    }

    /* Honest degraded affordance — full-contrast by default (never opacity-dimmed
       below the a11y floor); hosts may recolor via the cssprop. */
    .fallback-note {
      font-size: 0.75em;
      color: var(--elohim-link-unavailable-fg, inherit);
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
      return html`
        <elohim-skeleton width="6rem" height="1rem" part="skeleton"></elohim-skeleton>
      `;
    }

    // L4 is the degraded band: forbidden / missing / error (and the legacy
    // `unreachable` flag). Render an honest, legible face — never a bare
    // `epr:{id}` when head metadata is available — plus the context menu.
    if (this.loadLevel === 4) {
      return this.renderDegraded();
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

  /**
   * The degraded (L4) face. The label prefers any known title (head metadata
   * or a legacy preview) so the raw `epr:{id}` only surfaces when nothing else
   * is resolvable; the note states honestly WHY the target is unavailable.
   */
  private renderDegraded(): unknown {
    const label = this.resolution.preview?.title ?? this.resolution.title ?? '';
    const note = this.degradedNote();
    return html`
      <span class="menu-anchor">
        <span class="fallback" part="fallback" role="note" aria-label=${note}>
          <elohim-mention-base epr=${this.epr} label=${label}></elohim-mention-base>
          <span class="fallback-note" part="fallback-note">${note}</span>
        </span>
        ${this.renderContextMenu()}
      </span>
    `;
  }

  /** Honest, human-legible reason for the degraded face, keyed off the state. */
  private degradedNote(): string {
    switch (this.resolution.state) {
      case 'forbidden':
        return 'Unavailable at your reach';
      case 'missing':
        return 'No longer available';
      case 'error':
        return 'Could not load';
      default:
        return 'Unavailable';
    }
  }

  private renderContextMenu(): unknown {
    if (!this.menuOpen) return nothing;
    return html`
      <elohim-context-menu
        ?open=${this.menuOpen}
        .items=${this.contextMenuItems ?? this.defaultMenuItems}
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

    // Built-in conveniences fire only for the default (no-override) menu, so
    // the element stays useful standalone (Storybook, raw HTML). When a host
    // injects `contextMenuItems`, it owns ALL behavior via epr-menu-select.
    if (!this.contextMenuItems) {
      if (id === 'open') {
        this.dispatchNavigate();
      } else if (id === 'copy') {
        navigator.clipboard?.writeText(this.epr).catch(() => {
          // Clipboard write can reject (permissions / insecure context); no-op.
        });
      } else if (id === 'about') {
        this.dispatchEvent(
          new CustomEvent('about', {
            detail: { epr: this.epr },
            bubbles: true,
            composed: true,
          })
        );
      }
    }

    // Always re-emit the raw selection so the host can act on app-specific
    // ids (network / relationships / steward / flag / challenge / feedback).
    this.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id, epr: this.epr },
        bubbles: true,
        composed: true,
      })
    );
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
      })
    );
  }

  /**
   * Default resolution flow. The effective resolver is the explicit `.resolver`
   * prop if set, else the ambient {@link EprResolutionProvider} from context
   * (prop wins — backward compatible):
   * - With NEITHER, we transition to L2 with empty resolution (just exposes the
   *   EPR id as-is). Tests use `setResolution()` to advance through levels.
   * - Otherwise a `resolved` (or legacy plain) result advances to L3; a typed
   *   degraded result (forbidden / missing / error), the legacy `unreachable`
   *   flag, or a null return drops to the L4 degraded face. A null return maps
   *   to `missing` — the most honest reading of "the resolver found nothing".
   */
  private async resolve(): Promise<void> {
    if (!this.epr) return;
    this.loadLevel = 1;
    this.resolution = {};
    // Precedence: explicit `.resolver` prop over the ambient context provider.
    const explicit = this.resolver;
    const provider = this._resolutionProvider.value;
    if (!explicit && !provider) {
      this.loadLevel = 2;
      return;
    }
    try {
      const result = explicit
        ? await explicit(this.epr)
        : headToLinkResolution(await provider!.resolveHead(this.epr));
      if (result && isResolvedResolution(result)) {
        this.loadLevel = 3;
        this.resolution = result;
      } else {
        this.loadLevel = 4;
        this.resolution = result ?? { state: 'missing' };
      }
    } catch {
      this.loadLevel = 4;
      this.resolution = { state: 'error' };
    }
  }
}

/**
 * Adapt the ambient provider's typed head outcome to the element's resolution
 * shape. The head-plane states map 1:1 onto the element's degradation states;
 * `resolved` carries the head's title/description/reach so the chip is legible,
 * and `forbidden` keeps any known title for the honest fallback face.
 */
function headToLinkResolution(outcome: EprHeadResolution): EprLinkResolution {
  return {
    state: outcome.state,
    title: outcome.head?.title,
    description: outcome.head?.description ?? undefined,
    reach: outcome.head?.reach ?? undefined,
  };
}

/**
 * A resolution reads as fully `resolved` (interactive L3 chip) unless it
 * declares a degraded `state` or the legacy `unreachable` flag. A plain
 * `{ title }` result (no state, no flag) stays resolved — backward compatible.
 */
function isResolvedResolution(r: EprLinkResolution): boolean {
  if (r.unreachable) return false;
  return r.state === undefined || r.state === 'resolved';
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-epr-link': ElohimEprLink;
  }
}
