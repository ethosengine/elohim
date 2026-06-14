// NOTE: The stub `<elohim-memory-safety>` maps to the convention-compliant
// `<elohim-qahal-memory-safety>` tag per the pillar naming convention.
//
// FeltStatusView is the ts-rs–generated type that will live at
// @elohim/storage-client when `cargo test export_bindings` re-runs with
// the additive `feltStatus` block on ResilienceSnapshotView.  Until that
// generation lands this import resolves to the local interim re-export
// defined in this file.  DO NOT redefine it locally as a permanent fixture —
// swap to `import type { FeltStatusView } from '@elohim/storage-client';`
// once the generated file exists.

import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

// ---------------------------------------------------------------------------
// Interim type — remove once @elohim/storage-client exports FeltStatusView
// ---------------------------------------------------------------------------

/** Holder entry in the feltStatus projection. */
export interface FeltStatusHolder {
  /** Internal id (content-addressed; never rendered). */
  id: string;
  /** Collective kind: "household" | "church" | "patron-circle" | … */
  kind: string;
  /** Human-readable display name.  Render this; fall back to "a household" when absent. */
  label?: string;
}

/** Floor coverage descriptor. */
export interface FeltStatusFloor {
  /** Tier name (e.g. "standard", "vault"). */
  tier: string;
  /** Whether the household explicitly chose this tier (true) or it is a system default (false). */
  tierDeclared: boolean;
  /** How many holders this content should have at this tier. */
  wantsHouseholds: number;
  /** How many holders this content currently has. */
  hasHouseholds: number;
}

/**
 * The household-facing felt projection block on ResilienceSnapshotView.
 * Source: elohim-storage household_resilience.rs::snapshot(); Cat-C operational view.
 *
 * @see elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json
 */
export interface FeltStatusView {
  /**
   * The canonical felt sentence.  Render this as the primary claim;
   * do NOT re-derive this message in the element.
   *
   * Examples:
   *   "Held by 3 households: the Dowells, Aunt Ruth, First Church"
   *   "We can't confirm these are backed up yet"
   */
  headline: string;
  /**
   * Reassurance level.
   *   protected   — all coverage requirements met; settled/safe
   *   watching     — coverage survives a lapse; attentive but NOT alarming
   *   needs-help   — below floor; pro-social invite action warranted
   *   not-yet-seen — unmeasured (distributionState=unmeasured); HONEST gap,
   *                  NEVER rendered as protected
   */
  reassurance: 'protected' | 'watching' | 'needs-help' | 'not-yet-seen';
  /**
   * The current holders.  Render label; fall back to "a household" when absent.
   * NEVER render the id field directly to the user.
   */
  heldBy: FeltStatusHolder[];
  /** The resilience floor relative to which the reassurance is evaluated. */
  floor: FeltStatusFloor;
  /**
   * Pro-social CTA copy.  Render as an action when present.
   * Example: "Invite a household to help hold these"
   */
  suggestedAction?: string;
}

// ---------------------------------------------------------------------------
// Element
// ---------------------------------------------------------------------------

/**
 * Family Vault / Memory Safety card — household-facing resilience surface.
 *
 * The INVERSE of the operator debug Lens: renders NAMES, not nines.
 * When grandma's edge node dies, her family opens this card and SEES —
 * in plain, named language — that her photos are still held by people
 * who love her.
 *
 * Consumes the `feltStatus` block on ResilienceSnapshotView.
 * The element renders what the substrate computed; it never derives
 * its own copy of the protection sentence.
 *
 * @element elohim-qahal-memory-safety
 *
 * @prop {FeltStatusView | null} feltStatus - The felt-status projection block
 * @prop {string} contentLabel - Optional album/content name shown in the heading
 *
 * @fires memory-safety-action - Emitted when the user activates the suggestedAction CTA.
 *   Detail: `{ action: string }` matching `feltStatus.suggestedAction`.
 *
 * @cssprop --elohim-memory-safety-padding - Card container padding (default: 1.25rem)
 * @cssprop --elohim-memory-safety-bg - Card background (default: Canvas)
 * @cssprop --elohim-memory-safety-fg - Card foreground text (default: CanvasText)
 * @cssprop --elohim-memory-safety-border - Card border (default: 1px solid currentColor)
 * @cssprop --elohim-memory-safety-radius - Card border-radius (default: 0.5rem)
 * @cssprop --elohim-memory-safety-gap - Internal layout gap (default: 0.75rem)
 * @cssprop --elohim-memory-safety-headline-size - Headline font-size (default: 1rem)
 * @cssprop --elohim-memory-safety-holder-chip-bg - Holder chip background (default: ButtonFace)
 * @cssprop --elohim-memory-safety-holder-chip-fg - Holder chip foreground (default: ButtonText)
 * @cssprop --elohim-memory-safety-holder-chip-radius - Holder chip border-radius (default: 2rem)
 * @cssprop --elohim-memory-safety-status-protected-color - Accent color for "protected" state
 * @cssprop --elohim-memory-safety-status-watching-color - Accent color for "watching" state
 * @cssprop --elohim-memory-safety-status-needs-help-color - Accent color for "needs-help" state
 * @cssprop --elohim-memory-safety-status-not-yet-seen-color - Accent color for "not-yet-seen" state
 * @cssprop --elohim-memory-safety-cta-bg - CTA button background (default: ButtonFace)
 * @cssprop --elohim-memory-safety-cta-fg - CTA button foreground (default: ButtonText)
 * @cssprop --elohim-memory-safety-cta-border - CTA button border (default: 1px solid currentColor)
 * @cssprop --elohim-memory-safety-cta-radius - CTA button border-radius (default: 0.25rem)
 *
 * @csspart container - The outer card container
 * @csspart heading - The "Memory Safety" + content-label heading row
 * @csspart status-badge - The reassurance state badge (role=status)
 * @csspart headline - The canonical felt sentence paragraph
 * @csspart holder-list - The ordered list of named holders
 * @csspart holder-chip - Each individual holder chip (one per heldBy entry)
 * @csspart floor-line - The "held by M of N households" coverage line
 * @csspart cta - The suggestedAction call-to-action button
 * @csspart empty - The null/empty state placeholder
 *
 * @slot loading - Optional loading skeleton override
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityOptionalStandings pilot
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:designed, error:n/a, stale:designed, contested:n/a, offline:designed, unauthorized:n/a
 */
export class ElohimQahalMemorySafety extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      box-sizing: border-box;
    }

    /* ------------------------------------------------------------------ */
    /* Container                                                            */
    /* ------------------------------------------------------------------ */

    .container {
      display: flex;
      flex-direction: column;
      gap: var(--elohim-memory-safety-gap, 0.75rem);
      padding: var(--elohim-memory-safety-padding, 1.25rem);
      background: var(--elohim-memory-safety-bg, Canvas);
      color: var(--elohim-memory-safety-fg, CanvasText);
      border: var(--elohim-memory-safety-border, 1px solid currentColor);
      border-radius: var(--elohim-memory-safety-radius, 0.5rem);
    }

    /* ------------------------------------------------------------------ */
    /* Heading row                                                          */
    /* ------------------------------------------------------------------ */

    .heading {
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
      gap: 0.375rem;
      margin-block: 0;
      margin-inline: 0;
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--elohim-memory-safety-fg, CanvasText);
      opacity: 0.8;
    }

    .heading .content-label {
      font-weight: 400;
      font-style: italic;
    }

    /* ------------------------------------------------------------------ */
    /* Status badge (reassurance indicator)                                 */
    /* ------------------------------------------------------------------ */

    .status-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.375rem;
      font-size: 0.75rem;
      font-weight: 600;
      padding-block: 0.2em;
      padding-inline: 0.6em;
      border-radius: 2rem;
      /* Backgrounds and foregrounds are entirely semantic CSSProps so
         graphos can theme without reaching inside the element.            */
      background: transparent;
      border: 1px solid currentColor;
      /* State-specific accent color via data-reassurance attribute */
    }

    .status-badge[data-reassurance='protected'] {
      color: var(--elohim-memory-safety-status-protected-color, CanvasText);
    }

    .status-badge[data-reassurance='watching'] {
      color: var(--elohim-memory-safety-status-watching-color, CanvasText);
    }

    .status-badge[data-reassurance='needs-help'] {
      color: var(--elohim-memory-safety-status-needs-help-color, CanvasText);
    }

    .status-badge[data-reassurance='not-yet-seen'] {
      color: var(--elohim-memory-safety-status-not-yet-seen-color, CanvasText);
      /* Visually distinguish from protected: dashed border (no color assumption) */
      border-style: dashed;
    }

    /* ------------------------------------------------------------------ */
    /* Headline sentence                                                    */
    /* ------------------------------------------------------------------ */

    .headline {
      margin-block: 0;
      margin-inline: 0;
      font-size: var(--elohim-memory-safety-headline-size, 1rem);
      line-height: 1.5;
    }

    /* ------------------------------------------------------------------ */
    /* Holder chip list                                                     */
    /* ------------------------------------------------------------------ */

    .holder-list {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    .holder-chip {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.25em;
      padding-inline: 0.75em;
      background: var(--elohim-memory-safety-holder-chip-bg, ButtonFace);
      color: var(--elohim-memory-safety-holder-chip-fg, ButtonText);
      border-radius: var(--elohim-memory-safety-holder-chip-radius, 2rem);
      font-size: 0.875rem;
      /* Minimum touch target for coarse pointers */
      min-block-size: 2.75rem;
    }

    @media (pointer: coarse) {
      .holder-chip {
        min-block-size: 2.75rem;
        padding-block: 0.5em;
        padding-inline: 1em;
      }
    }

    /* ------------------------------------------------------------------ */
    /* Floor line                                                           */
    /* ------------------------------------------------------------------ */

    .floor-line {
      font-size: 0.8125rem;
      opacity: 0.75;
      margin-block: 0;
      margin-inline: 0;
    }

    /* ------------------------------------------------------------------ */
    /* CTA button                                                           */
    /* ------------------------------------------------------------------ */

    .cta {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      background: var(--elohim-memory-safety-cta-bg, ButtonFace);
      color: var(--elohim-memory-safety-cta-fg, ButtonText);
      border: var(--elohim-memory-safety-cta-border, 1px solid currentColor);
      border-radius: var(--elohim-memory-safety-cta-radius, 0.25rem);
      padding-block: 0.625rem;
      padding-inline: 1.25rem;
      font: inherit;
      font-size: 0.875rem;
      align-self: flex-start;
      min-block-size: 2.75rem;
    }

    .cta:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    /* ------------------------------------------------------------------ */
    /* Empty / null state                                                   */
    /* ------------------------------------------------------------------ */

    .empty {
      font-size: 0.875rem;
      opacity: 0.6;
      font-style: italic;
    }

    /* ------------------------------------------------------------------ */
    /* Forced-colors (WCAG 2.1 §1.4.3 / Windows High Contrast)            */
    /* ------------------------------------------------------------------ */

    @media (forced-colors: active) {
      .container {
        background: Canvas;
        color: CanvasText;
        border-color: CanvasText;
      }

      .status-badge {
        color: CanvasText;
        border-color: CanvasText;
        background: Canvas;
      }

      .holder-chip {
        background: ButtonFace;
        color: ButtonText;
      }

      .cta {
        background: ButtonFace;
        color: ButtonText;
        border-color: ButtonText;
      }
    }

    /* ------------------------------------------------------------------ */
    /* Reduced-transparency: no translucency (we use opacity for floor-line)*/
    /* ------------------------------------------------------------------ */

    @media (prefers-reduced-transparency: reduce) {
      .floor-line {
        opacity: 1;
      }

      .heading {
        opacity: 1;
      }
    }
  `;

  /** The felt-status projection. Null renders the empty/loading placeholder. */
  @property({ type: Object })
  feltStatus: FeltStatusView | null = null;

  /** Optional album / content name rendered in the heading. */
  @property({ type: String, attribute: 'content-label' })
  contentLabel = '';

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  #handleCtaClick() {
    const action = this.feltStatus?.suggestedAction;
    if (!action) return;
    this.dispatchEvent(
      new CustomEvent('memory-safety-action', {
        detail: { action },
        bubbles: true,
        composed: true,
      })
    );
  }

  #handleCtaKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      this.#handleCtaClick();
    }
  }

  #badgeLabel(reassurance: FeltStatusView['reassurance']): string {
    switch (reassurance) {
      case 'protected':
        return 'Protected';
      case 'watching':
        return 'Watching';
      case 'needs-help':
        return 'Needs help';
      case 'not-yet-seen':
        return 'Not yet confirmed';
    }
  }

  #badgeIndicator(reassurance: FeltStatusView['reassurance']): string {
    switch (reassurance) {
      case 'protected':
        return '●';
      case 'watching':
        return '◐';
      case 'needs-help':
        return '○';
      case 'not-yet-seen':
        return '?';
    }
  }

  #holderLabel(holder: FeltStatusHolder): string {
    return holder.label?.trim() || 'a household';
  }

  #floorText(floor: FeltStatusFloor): string {
    const { wantsHouseholds, hasHouseholds, tier, tierDeclared } = floor;
    const tierPhrase = tierDeclared ? ` (${tier} tier)` : '';
    return `Held by ${hasHouseholds} of the ${wantsHouseholds} households this should live in${tierPhrase}`;
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  override render() {
    if (!this.feltStatus) {
      return html`
        <div class="container" part="container">
          <slot name="loading">
            <p class="empty" part="empty">Memory Safety information is loading…</p>
          </slot>
        </div>
      `;
    }

    const { headline, reassurance, heldBy, floor, suggestedAction } = this.feltStatus;

    const headingEl = html`
      <p class="heading" part="heading" aria-label="Memory Safety${this.contentLabel ? ` for ${this.contentLabel}` : ''}">
        <span>Memory Safety</span>
        ${this.contentLabel
          ? html`<span class="content-label">${this.contentLabel}</span>`
          : nothing}
      </p>
    `;

    // The status badge is announced to screen readers via role=status.
    // The data-reassurance attribute drives CSS — NOT color alone; the label
    // also changes, satisfying WCAG 1.4.1 (use of color).
    const badgeEl = html`
      <span
        class="status-badge"
        part="status-badge"
        role="status"
        aria-live="polite"
        data-reassurance=${reassurance}
        aria-label="Reassurance: ${this.#badgeLabel(reassurance)}"
      >
        <span aria-hidden="true">${this.#badgeIndicator(reassurance)}</span>
        ${this.#badgeLabel(reassurance)}
      </span>
    `;

    const headlineEl = html`
      <p class="headline" part="headline">${headline}</p>
    `;

    // Holder chips — render label only; fall back to "a household".
    // The id field is NEVER surfaced to the user.
    const holderListEl =
      heldBy.length > 0
        ? html`
            <ul class="holder-list" part="holder-list" aria-label="Held by">
              ${heldBy.map(
                h => html`
                  <li class="holder-chip" part="holder-chip">${this.#holderLabel(h)}</li>
                `
              )}
            </ul>
          `
        : nothing;

    // Floor line: "held by M of N households this should live in"
    const floorEl = html`
      <p class="floor-line" part="floor-line">${this.#floorText(floor)}</p>
    `;

    // CTA: only when suggestedAction is present.
    // Rendered as a button (keyboard-reachable, announced by screen reader).
    const ctaEl = suggestedAction
      ? html`
          <button
            class="cta"
            part="cta"
            type="button"
            @click=${this.#handleCtaClick}
            @keydown=${this.#handleCtaKeydown}
          >
            ${suggestedAction}
          </button>
        `
      : nothing;

    return html`
      <div class="container" part="container">
        ${headingEl} ${badgeEl} ${headlineEl} ${holderListEl} ${floorEl} ${ctaEl}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-memory-safety': ElohimQahalMemorySafety;
  }
}
