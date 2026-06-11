import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface EprRelationship {
  /** Relationship type: PREREQUISITE, TEACHES, CONTAINS, REFERENCES, etc. */
  type: string;
  /** Target content ID (resolves to another EPR Head) */
  target: string;
  /** Optional target content address (CID link in IPLD) */
  targetCid?: string;
}

interface RelationshipGroup {
  type: string;
  label: string;
  items: EprRelationship[];
}

// ── Ordering & labels ─────────────────────────────────────────────────────────

const KNOWN_ORDER = ['PREREQUISITE', 'TEACHES', 'CONTAINS', 'REFERENCES'];

const GROUP_LABELS: Record<string, string> = {
  PREREQUISITE: 'Prerequisites',
  TEACHES: 'This teaches',
  CONTAINS: 'Contains',
  REFERENCES: 'References',
};

function groupLabel(type: string): string {
  return GROUP_LABELS[type] ?? type.charAt(0).toUpperCase() + type.slice(1).toLowerCase();
}

function buildGroups(relationships: EprRelationship[]): RelationshipGroup[] {
  const map = new Map<string, EprRelationship[]>();
  for (const rel of relationships) {
    const bucket = map.get(rel.type);
    if (bucket) {
      bucket.push(rel);
    } else {
      map.set(rel.type, [rel]);
    }
  }
  const knownPresent = KNOWN_ORDER.filter(t => map.has(t));
  const unknownPresent = [...map.keys()].filter(t => !KNOWN_ORDER.includes(t));
  const orderedTypes = [...knownPresent, ...unknownPresent];
  return orderedTypes.map(type => ({
    type,
    label: groupLabel(type),
    items: map.get(type)!,
  }));
}

/**
 * <elohim-epr-relationships-panel> — cross-pillar relationship visualization primitive.
 *
 * Groups an array of EPR relationships by type and renders them in a
 * responsive grid. Knows no brand; all visual properties are override-able
 * via CSS custom properties. Accepts service deps as properties — no Angular DI.
 *
 * Migrated from EprRelationshipsPanelComponent (app/elohim-app).
 *
 * @element elohim-epr-relationships-panel
 *
 * @prop {EprRelationship[]} relationships - The EPR relationships to render
 * @prop {Function | null} navigator - Optional callback to navigate to a target EPR
 *
 * @fires epr-navigate - { detail: { target: string } } when a relationship card is activated
 *
 * @cssprop --elohim-epr-panel-heading-color - Group heading color
 * @cssprop --elohim-epr-panel-card-bg - Relationship card background
 * @cssprop --elohim-epr-panel-card-border - Relationship card border
 * @cssprop --elohim-epr-panel-card-fg - Relationship card foreground text color
 * @cssprop --elohim-epr-panel-card-radius - Relationship card border radius
 * @cssprop --elohim-epr-panel-gap - Vertical rhythm between panel sections/groups
 * @cssprop --elohim-epr-panel-card-gap - Gap between relationship cards inside the grid
 *
 * @csspart panel - The outer section element
 * @csspart group - Each relationship group container
 * @csspart group-heading - The group heading element
 * @csspart group-grid - The card grid container
 * @csspart card - Each individual relationship card
 * @csspart card-type - The type badge on each card
 * @csspart card-target - The target ID text on each card
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimEprRelationshipsPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    :host([hidden]) {
      display: none;
    }

    .panel {
      margin-block-start: var(--elohim-epr-panel-gap, 1.5rem);
    }

    .group {
      margin-block-end: var(--elohim-epr-panel-gap, 1.25rem);
    }

    .group-heading {
      font-size: 0.72rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--elohim-epr-panel-heading-color, CanvasText);
      margin: 0 0 0.5rem;
    }

    .group-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
      gap: var(--elohim-epr-panel-card-gap, 0.625rem);
    }

    .card {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      background: var(--elohim-epr-panel-card-bg, Canvas);
      color: var(--elohim-epr-panel-card-fg, CanvasText);
      border: var(
        --elohim-epr-panel-card-border,
        1px solid color-mix(in oklch, currentColor 20%, transparent)
      );
      border-radius: var(--elohim-epr-panel-card-radius, 0.375rem);
      cursor: pointer;
      font: inherit;
      text-align: start;
    }

    .card:hover,
    .card:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }

    .card-type {
      font-size: 0.7rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      opacity: 0.6;
    }

    .card-target {
      font-size: 0.875rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    @media (pointer: coarse) {
      .card {
        min-block-size: 44px;
        justify-content: center;
      }
    }

    @media (forced-colors: active) {
      .card {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      .card:hover,
      .card:focus-visible {
        outline: 2px solid Highlight;
        background: Highlight;
        color: HighlightText;
      }
    }
  `;

  /**
   * The EPR relationships to render.
   */
  @property({ attribute: false })
  relationships: EprRelationship[] = [];

  /**
   * Optional callback to navigate when a relationship card is activated.
   * Called with the target EPR ID. If absent, fires 'epr-navigate' event.
   */
  @property({ attribute: false })
  navigator: ((target: string) => void) | null = null;

  private get groups(): RelationshipGroup[] {
    return this.relationships.length > 0 ? buildGroups(this.relationships) : [];
  }

  override render(): unknown {
    const groups = this.groups;
    if (groups.length === 0) return nothing;

    return html`
      <section class="panel" part="panel" data-testid="epr-relationships-panel">
        ${groups.map(group => this.renderGroup(group))}
      </section>
    `;
  }

  private renderGroup(group: RelationshipGroup) {
    return html`
      <div class="group" part="group" data-testid="epr-rel-group" data-type=${group.type}>
        <h3 class="group-heading" part="group-heading">${group.label}</h3>
        <div class="group-grid" part="group-grid">
          ${group.items.map(item => this.renderCard(item))}
        </div>
      </div>
    `;
  }

  private renderCard(item: EprRelationship) {
    return html`
      <button
        class="card"
        part="card"
        type="button"
        data-testid="epr-relationship-card"
        data-target=${item.target}
        aria-label="${item.type}: ${item.target}"
        @click=${() => this.handleCardClick(item.target)}
        @keydown=${(e: KeyboardEvent) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            this.handleCardClick(item.target);
          }
        }}
      >
        <span class="card-type" part="card-type">${item.type}</span>
        <span class="card-target" part="card-target">${item.target}</span>
      </button>
    `;
  }

  private handleCardClick(target: string): void {
    if (this.navigator) {
      this.navigator(target);
    } else {
      this.dispatchEvent(
        new CustomEvent('epr-navigate', {
          detail: { target },
          bubbles: true,
          composed: true,
        })
      );
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-epr-relationships-panel': ElohimEprRelationshipsPanel;
  }
}
