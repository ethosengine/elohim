import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

/** Lamad pillar — knowledge context embedded in the EPR Head. */
export interface EprLamadContext {
  title: string;
  contentType: string;
  description?: string;
  contentFormat?: string;
  tags?: string[];
}

/** Shefa pillar — value context embedded in the EPR Head. */
export interface EprShefaContext {
  stewards?: string[];
  allocations?: number[];
}

/** Qahal pillar — governance context embedded in the EPR Head. */
export interface EprQahalContext {
  reach?: string;
  layer?: string;
}

/**
 * The EPR Head — the ~500B gossip-friendly metadata envelope.
 * Structural type definition matching app/elohim-app models/epr-head.model.ts.
 * When @elohim/storage-client exports EprHead (Slice 2.5), import from there instead.
 */
export interface EprHead {
  version: number;
  id: string;
  content: string;
  lamad: EprLamadContext;
  shefa: EprShefaContext;
  qahal: EprQahalContext;
  relationships: { type: string; target: string; targetCid?: string }[];
  author?: string;
  updated?: string;
}

/**
 * <elohim-epr-popover> — three-pillar metadata popover primitive.
 *
 * Shows a positioned popover with EPR Head metadata. Displays pillar-specific
 * context from the three protocol pillars:
 * - Lamad: title, content type, format, tags, description
 * - Shefa: steward count
 * - Qahal: reach level, constitutional layer
 *
 * Designed to be used programmatically (positioned over anchors) by the
 * markdown-renderer follow-up (Wave 3 / Slice 2.6 continuation) to render
 * `<a data-epr=...>` anchor metadata popovers.
 *
 * Does NOT modify `<elohim-epr-link>` — this is a separate, distinct primitive.
 *
 * Migrated/added from EprPopoverComponent (app/elohim-app/elohim/components/epr-popover)
 * as the Slice 2.6 canary addition.
 *
 * @element elohim-epr-popover
 *
 * @prop {EprHead | null} head - EPR Head metadata to display
 * @prop {{ top: number; left: number }} position - Position relative to the viewport (px)
 * @prop {string | null} route - Route href for the "Open resource" link (routerLink, in-bundle)
 * @prop {string | null} href - Universal href for cross-bundle navigation (plain anchor fallback)
 * @prop {boolean} visible - Whether the popover is visible
 *
 * @fires epr-popover-enter - when mouse enters the popover
 * @fires epr-popover-dismiss - when mouse leaves or escape is pressed
 * @fires epr-popover-navigate - { detail: { route: string } } when link is clicked
 *
 * @cssprop --elohim-epr-popover-width - Popover width
 * @cssprop --elohim-epr-popover-bg - Popover background
 * @cssprop --elohim-epr-popover-fg - Popover foreground text color
 * @cssprop --elohim-epr-popover-border - Popover border
 * @cssprop --elohim-epr-popover-radius - Popover border radius
 * @cssprop --elohim-epr-popover-shadow - Popover box shadow
 * @cssprop --elohim-epr-type-bg - Content type badge background
 * @cssprop --elohim-epr-type-fg - Content type badge foreground
 * @cssprop --elohim-epr-tag-bg - Tag chip background
 * @cssprop --elohim-epr-tag-fg - Tag chip foreground
 * @cssprop --elohim-epr-reach-bg - Reach badge background
 * @cssprop --elohim-epr-reach-fg - Reach badge foreground
 * @cssprop --elohim-epr-layer-bg - Layer badge background
 * @cssprop --elohim-epr-layer-fg - Layer badge foreground
 * @cssprop --elohim-epr-link-color - "Open resource" link color
 * @cssprop --elohim-epr-divider-color - Section divider color
 *
 * @csspart popover - The popover container
 * @csspart header - The type + format header row
 * @csspart type-badge - The content type badge
 * @csspart format - The content format label
 * @csspart title - The content title heading
 * @csspart description - The description paragraph
 * @csspart tags - The tags container
 * @csspart tag - Each tag chip
 * @csspart shefa-section - The shefa (stewards) section
 * @csspart qahal-section - The qahal (reach + layer) section
 * @csspart reach-badge - The reach badge
 * @csspart layer-badge - The layer badge
 * @csspart link - The "Open resource" link
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimEprPopover extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      position: fixed;
      z-index: 1000;
      pointer-events: auto;
    }

    :host([hidden]),
    :host(:not([visible])) {
      display: none;
    }

    .popover {
      inline-size: var(--elohim-epr-popover-width, 280px);
      padding: 0.75rem;
      background: var(--elohim-epr-popover-bg, Canvas);
      color: var(--elohim-epr-popover-fg, CanvasText);
      border: var(
        --elohim-epr-popover-border,
        1px solid color-mix(in oklch, currentColor 20%, transparent)
      );
      border-radius: var(--elohim-epr-popover-radius, 0.5rem);
      box-shadow: var(--elohim-epr-popover-shadow, 0 4px 16px rgb(0 0 0 / 12%));
      font-size: 0.85rem;
      line-height: 1.4;
    }

    .header {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      margin-block-end: 0.375rem;
    }

    .type-badge {
      display: inline-block;
      padding-block: 1px;
      padding-inline: 0.375rem;
      border-radius: 0.25rem;
      background: var(--elohim-epr-type-bg, ButtonFace);
      color: var(--elohim-epr-type-fg, ButtonText);
      font-size: 0.7em;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      font-weight: 600;
    }

    .format {
      font-size: 0.75em;
    }

    .title {
      margin: 0 0 0.25rem;
      font-size: 0.95em;
      font-weight: 600;
    }

    .description {
      margin: 0 0 0.5rem;
      font-size: 0.8em;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }

    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: 0.25rem;
      margin-block-end: 0.5rem;
    }

    .tag {
      padding-block: 1px;
      padding-inline: 0.375rem;
      border-radius: 0.625rem;
      background: var(--elohim-epr-tag-bg, Canvas);
      color: var(--elohim-epr-tag-fg, CanvasText);
      font-size: 0.75em;
      border: 1px solid color-mix(in oklch, currentcolor 15%, transparent);
    }

    .section {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      margin-block-end: 0.25rem;
      font-size: 0.8em;
    }

    .section-value {
      font-weight: 500;
    }

    .reach-badge {
      padding-block: 1px;
      padding-inline: 0.375rem;
      border-radius: 0.25rem;
      font-size: 0.75em;
      background: var(--elohim-epr-reach-bg, Canvas);
      color: var(--elohim-epr-reach-fg, CanvasText);
      border: 1px solid color-mix(in oklch, currentcolor 15%, transparent);
    }

    .layer-badge {
      padding-block: 1px;
      padding-inline: 0.375rem;
      border-radius: 0.25rem;
      font-size: 0.75em;
      background: var(--elohim-epr-layer-bg, Canvas);
      color: var(--elohim-epr-layer-fg, CanvasText);
      border: 1px solid color-mix(in oklch, currentcolor 15%, transparent);
    }

    .link {
      display: block;
      margin-block-start: 0.5rem;
      padding-block-start: 0.5rem;
      border-block-start: 1px solid
        var(--elohim-epr-divider-color, color-mix(in oklch, currentColor 15%, transparent));
      color: var(--elohim-epr-link-color, LinkText);
      text-decoration: none;
      cursor: pointer;
      background: none;
      border-inline: none;
      border-block-end: none;
      padding-inline: 0;
      font: inherit;
      font-size: 0.8em;
    }

    .link:hover {
      text-decoration: underline;
    }

    .link:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }

    @media (forced-colors: active) {
      .popover {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      .type-badge {
        background: ButtonFace;
        color: ButtonText;
        border: 1px solid ButtonText;
      }

      .tag {
        background: ButtonFace;
        color: ButtonText;
        border-color: ButtonText;
      }

      .reach-badge,
      .layer-badge {
        background: ButtonFace;
        color: ButtonText;
        border-color: ButtonText;
      }

      .link {
        color: LinkText;
      }
    }
  `;

  /** EPR Head metadata to display */
  @property({ attribute: false })
  head: EprHead | null = null;

  /** Position relative to the viewport (pixels) */
  @property({ attribute: false })
  position: { top: number; left: number } = { top: 0, left: 0 };

  /** Route href for the "Open resource" link (routerLink, in-bundle) */
  @property()
  route: string | null = null;

  /** Universal href for cross-bundle navigation (plain anchor fallback) */
  @property()
  href: string | null = null;

  /** Whether the popover is currently visible */
  @property({ type: Boolean, reflect: true })
  visible = false;

  override updated(changed: Map<string | symbol, unknown>): void {
    super.updated(changed);
    if (changed.has('position') || changed.has('visible')) {
      this.applyClampedPosition();
    }
  }

  /**
   * Apply the consumer-supplied coords, clamped to the visual viewport. Any
   * naive anchor-rect caller (rect.bottom/rect.left) reproduces the
   * 2026-06-12 fold-down overflow class near viewport edges, so the element
   * owns the final guard. Fixed-position coords are physical by nature —
   * matching the {top,left} API shape. Re-runs on visible because the box
   * has no measurable size until it renders.
   */
  private applyClampedPosition(): void {
    let { top, left } = this.position;
    if (this.visible) {
      const GUTTER = 8;
      const rect = this.getBoundingClientRect();
      const viewportWidth = document.documentElement.clientWidth;
      const viewportHeight = document.documentElement.clientHeight;
      left = Math.max(GUTTER, Math.min(left, viewportWidth - rect.width - GUTTER));
      top = Math.max(GUTTER, Math.min(top, viewportHeight - rect.height - GUTTER));
    }
    this.style.top = `${top}px`;
    this.style.left = `${left}px`;
  }

  override render(): unknown {
    if (!this.head || !this.visible) return nothing;

    const { lamad, shefa, qahal } = this.head;

    return html`
      <div
        class="popover"
        part="popover"
        role="tooltip"
        aria-labelledby="epr-popover-title"
        data-testid="epr-popover"
        @mouseenter=${this.handleMouseEnter}
        @mouseleave=${this.handleMouseLeave}
        @keydown=${(e: KeyboardEvent) => {
          if (e.key === 'Escape') this.dispatchDismiss();
        }}
      >
        <!-- Header: content type badge + format -->
        <div class="header" part="header">
          <span class="type-badge" part="type-badge" data-testid="epr-popover-type">
            ${lamad.contentType}
          </span>
          ${lamad.contentFormat
            ? html`
                <span class="format" part="format" data-testid="epr-popover-format">
                  ${lamad.contentFormat}
                </span>
              `
            : nothing}
        </div>

        <h4 id="epr-popover-title" class="title" part="title" data-testid="epr-popover-title">
          ${lamad.title}
        </h4>

        ${lamad.description
          ? html`
              <p class="description" part="description" data-testid="epr-popover-desc">
                ${lamad.description}
              </p>
            `
          : nothing}

        <!-- Lamad: tags -->
        ${lamad.tags?.length
          ? html`
              <div class="tags" part="tags" data-testid="epr-popover-tags">
                ${lamad.tags.map(
                  tag => html`
                    <span class="tag" part="tag">${tag}</span>
                  `
                )}
              </div>
            `
          : nothing}

        <!-- Shefa: steward count -->
        ${shefa?.stewards?.length
          ? html`
              <div class="section" part="shefa-section" data-testid="epr-popover-shefa">
                <span class="section-label">Stewards</span>
                <span class="section-value">${shefa.stewards.length}</span>
              </div>
            `
          : nothing}

        <!-- Qahal: reach + layer -->
        ${this.renderQahalSection(qahal)}

        <!-- Footer: link to full resource — router event (in-bundle) or plain anchor (cross-bundle) -->
        ${this.renderFooter()}
      </div>
    `;
  }

  private renderQahalSection(qahal: EprQahalContext): unknown {
    if (!qahal?.reach && !qahal?.layer) return nothing;
    return html`
      <div class="section" part="qahal-section" data-testid="epr-popover-qahal">
        ${qahal.reach
          ? html`
              <span class="reach-badge" part="reach-badge">${qahal.reach}</span>
            `
          : nothing}
        ${qahal.layer
          ? html`
              <span class="layer-badge" part="layer-badge">${qahal.layer}</span>
            `
          : nothing}
      </div>
    `;
  }

  private renderFooter(): unknown {
    if (this.route) {
      return html`
        <button
          class="link"
          part="link"
          type="button"
          data-testid="epr-popover-link"
          @click=${this.handleLinkClick}
        >
          Open resource
        </button>
      `;
    }
    if (this.href) {
      return html`
        <a class="link" part="link" href=${this.href} data-testid="epr-popover-link">
          Open resource
        </a>
      `;
    }
    return nothing;
  }

  private handleMouseEnter(): void {
    this.dispatchEvent(new CustomEvent('epr-popover-enter', { bubbles: true, composed: true }));
  }

  private handleMouseLeave(): void {
    this.dispatchDismiss();
  }

  private dispatchDismiss(): void {
    this.dispatchEvent(new CustomEvent('epr-popover-dismiss', { bubbles: true, composed: true }));
  }

  private handleLinkClick(): void {
    if (this.route) {
      this.dispatchEvent(
        new CustomEvent('epr-popover-navigate', {
          detail: { route: this.route },
          bubbles: true,
          composed: true,
        })
      );
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-epr-popover': ElohimEprPopover;
  }
}
