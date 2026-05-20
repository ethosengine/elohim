import { msg, str } from '@lit/localize';
import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Hardware/deployment archetype for the device-kind variant.
 * Mirrors `DeviceArchetype` from `@elohim/storage-client` (substrate-local
 * copy to keep elohim-core independent of application-layer packages).
 */
export type ComputeTileArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Substrate-local structural equivalent of `ComputeTriptych` from
 * `@elohim/storage-client`. Structurally compatible — TypeScript's structural
 * typing means the generated type can be assigned to this interface without
 * an explicit cast.
 *
 * elohim-core does not depend on `@elohim/storage-client` to remain a
 * blank-slate substrate independent of application-layer packages.
 * Story fixtures in elohim-library import the real generated type.
 */
export interface ComputeTriptychShape {
  free: bigint | null;
  used: bigint | null;
  stewarded: bigint | null;
}

/**
 * Hub-aggregate input shape.
 *
 * No ts-rs view exists for a HubComputeAggregate yet — this is a locally
 * typed operational projection. The fields mirror `ComputeTriptych` (three
 * `bigint | null` triptych fields) plus hub-contextual chrome.
 *
 * Follow-up: rust-architect should define a `HubComputeAggregateView` so
 * this local interface can be replaced with the generated type.
 */
export interface ComputeTileHubValue {
  kind: 'hub';
  /** Opaque hub identifier — e.g. 'hub:household:matthew-jessica-james' */
  hubId: string;
  /** Optional human-readable label for the hub. */
  displayLabel?: string;
  /** Bytes available across all member devices. null when no sample. */
  free: bigint | null;
  /** Bytes used across all member devices. null when no sample. */
  used: bigint | null;
  /** Bytes stewarded (committed via rea_commitments) across member devices. null when no sample. */
  stewarded: bigint | null;
  /** Number of member devices contributing to this aggregate. */
  memberDeviceCount?: number;
}

/**
 * Per-device input shape.
 * Uses the subset of `DeviceSummary` fields needed for compute rendering.
 */
export interface ComputeTileDeviceValue {
  kind: 'device';
  /** Peer identifier — e.g. 'peer:12D3KooW…' */
  peerId: string;
  /** Human-readable device name. null when not set. */
  displayName: string | null;
  /** Hardware/deployment archetype. */
  archetype: ComputeTileArchetype;
  /** Whether the device is currently online. */
  online: boolean;
  /**
   * Compute triptych. Structurally compatible with `ComputeTriptych` from
   * `@elohim/storage-client` — assign directly without cast.
   * null when no sample yet.
   */
  compute: ComputeTriptychShape | null;
}

export type ComputeTileValue = ComputeTileHubValue | ComputeTileDeviceValue;

/**
 * Content state of the tile.
 */
export type ComputeTileState = 'idle' | 'loading' | 'error' | 'stale' | 'offline' | 'empty';

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Extract the triptych numbers from either value shape.
 */
function extractTriptych(value: ComputeTileValue): ComputeTriptychShape {
  if (value.kind === 'hub') {
    return { free: value.free, used: value.used, stewarded: value.stewarded };
  }
  return {
    free: value.compute?.free ?? null,
    used: value.compute?.used ?? null,
    stewarded: value.compute?.stewarded ?? null,
  };
}

/**
 * Format bytes using Intl.NumberFormat with `unit: 'byte'` where the runtime
 * supports it (Chromium 77+ / Firefox 78+ / Safari 14.5+), with a decimal-
 * compact fallback for older runtimes.
 *
 * Returns '—' for null/undefined inputs (no sample available).
 */
function formatBytes(value: bigint | null | undefined, locale: string): string {
  if (value === null || value === undefined) return '—';
  const n = Number(value);
  if (!Number.isFinite(n)) return '—';

  // Preferred path: use Intl byte unit
  try {
    // Byte formatting is not universally available — wrap in try/catch
    if (n >= 1_073_741_824) {
      return new Intl.NumberFormat(locale, {
        style: 'unit',
        unit: 'gigabyte',
        unitDisplay: 'short',
        maximumFractionDigits: 1,
      }).format(n / 1_073_741_824);
    }
    if (n >= 1_048_576) {
      return new Intl.NumberFormat(locale, {
        style: 'unit',
        unit: 'megabyte',
        unitDisplay: 'short',
        maximumFractionDigits: 1,
      }).format(n / 1_048_576);
    }
    if (n >= 1024) {
      return new Intl.NumberFormat(locale, {
        style: 'unit',
        unit: 'kilobyte',
        unitDisplay: 'short',
        maximumFractionDigits: 1,
      }).format(n / 1024);
    }
    return new Intl.NumberFormat(locale, {
      style: 'unit',
      unit: 'byte',
      unitDisplay: 'short',
    }).format(n);
  } catch {
    // Fallback for runtimes without style:'unit' support
    if (n >= 1_073_741_824) return `${(n / 1_073_741_824).toFixed(1)} GB`;
    if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)} MB`;
    if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${n} B`;
  }
}

/** Archetype display labels. Protocol vocabulary — not user-invented. */
const ARCHETYPE_LABELS: Record<ComputeTileArchetype, () => string> = {
  node: () => msg('Home server'),
  desktop: () => msg('Laptop'),
  mobile: () => msg('Phone'),
  steward: () => msg('Steward process'),
};

// ---------------------------------------------------------------------------
// Element
// ---------------------------------------------------------------------------

/**
 * `<elohim-compute-tile>` — polymorphic compute-capacity tile.
 *
 * Renders the compute triptych (free / used / stewarded bytes) for either a
 * hub aggregate or a single device, with **progressive disclosure** matching
 * the viewer's capability lens.
 *
 * | Lens     | What's shown                                                   |
 * |----------|----------------------------------------------------------------|
 * | minimal  | Qualitative summary ("You have enough." when free is healthy) |
 * | simple   | "5 GB free"                                                    |
 * | standard | "5 GB of 15 GB free"                                          |
 * | detail   | "5 GB of 15 GB · 12 GB stewarded"                             |
 * | debug    | Above + peer-id / hub-id / freshness timestamp                 |
 * | trace    | Above + source-route annotation                                |
 *
 * @element elohim-compute-tile
 *
 * @prop {ComputeTileValue | null} value - Input data: hub or device shape.
 * @prop {ComputeTileState} tileState - Content state of the tile.
 * @prop {string | null} lastSampleAt - ISO-8601 timestamp of last metric sample (debug+).
 * @prop {string | null} sourceRoute - Source-route annotation for trace lens.
 *
 * @slot label - Optional override for the contextual label (hub name / device name).
 *
 * @cssprop --elohim-compute-tile-bg - Tile background color
 * @cssprop --elohim-compute-tile-fg - Tile foreground text color
 * @cssprop --elohim-compute-tile-border - Tile border style
 * @cssprop --elohim-compute-tile-radius - Tile border-radius
 * @cssprop --elohim-compute-tile-padding - Tile padding
 * @cssprop --elohim-compute-tile-label-color - Contextual label color
 * @cssprop --elohim-compute-tile-value-color - Primary value color (free bytes)
 * @cssprop --elohim-compute-tile-secondary-color - Secondary value color (used/stewarded)
 * @cssprop --elohim-compute-tile-state-color - State indicator color (stale, offline, error)
 * @cssprop --elohim-compute-tile-gauge-track - Capacity bar track color
 * @cssprop --elohim-compute-tile-gauge-fill - Capacity bar fill color
 * @cssprop --elohim-compute-tile-meta-color - Debug/trace metadata text color
 *
 * @csspart container - The outer tile container
 * @csspart header - Label + status row
 * @csspart label - Contextual label (hub name / device name)
 * @csspart status-dot - Online/offline indicator dot
 * @csspart body - Main content area
 * @csspart value-primary - The primary free-bytes value
 * @csspart value-detail - Detail row (triptych breakdown)
 * @csspart gauge - Symbolic capacity gauge (symbolic textuality mode)
 * @csspart state-banner - Loading / error / stale / offline state banner
 * @csspart meta - Debug/trace metadata section
 *
 * @capabilityMaxLens trace
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:designed, contested:n/a, offline:designed, unauthorized:n/a
 */
export class ElohimComputeTile extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      contain: layout style;
    }

    :host([hidden]) {
      display: none;
    }

    .container {
      background: var(--elohim-compute-tile-bg, Canvas);
      color: var(--elohim-compute-tile-fg, CanvasText);
      border: var(--elohim-compute-tile-border, 1px solid currentColor);
      border-radius: var(--elohim-compute-tile-radius, 0.375rem);
      padding: var(--elohim-compute-tile-padding, 0.75rem 1rem);
      font: inherit;
      min-inline-size: 0;
      word-break: break-word;
    }

    /* Header row */
    .header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-block-end: 0.5rem;
    }

    .status-dot {
      display: inline-block;
      inline-size: 0.5rem;
      block-size: 0.5rem;
      border-radius: 50%;
      background: currentColor;
      flex-shrink: 0;
    }

    .status-dot[data-online='true'] {
      color: var(--elohim-compute-tile-state-color, LinkText);
    }

    .status-dot[data-online='false'] {
      opacity: 0.4;
    }

    .label {
      font-weight: 500;
      color: var(--elohim-compute-tile-label-color, CanvasText);
      flex: 1;
    }

    /* Body */
    .body {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .value-primary {
      font-size: 1.25em;
      font-weight: 600;
      color: var(--elohim-compute-tile-value-color, CanvasText);
    }

    .value-detail {
      font-size: 0.875em;
      color: var(--elohim-compute-tile-secondary-color, CanvasText);
    }

    /* Symbolic gauge */
    .gauge {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      block-size: 0.5rem;
      border-radius: 0.25rem;
      overflow: hidden;
      background: var(--elohim-compute-tile-gauge-track, ButtonFace);
      margin-block: 0.375rem;
    }

    .gauge-fill {
      block-size: 100%;
      border-radius: inherit;
      background: var(--elohim-compute-tile-gauge-fill, CanvasText);
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      .gauge-fill {
        transition: inline-size 200ms ease;
      }
    }

    /* State banner */
    .state-banner {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding-block: 0.25rem;
      font-size: 0.875em;
      color: var(--elohim-compute-tile-state-color, CanvasText);
    }

    /* Debug / trace meta */
    .meta {
      margin-block-start: 0.75rem;
      padding-block-start: 0.5rem;
      border-block-start: 1px solid currentColor;
      font-size: 0.75em;
      font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', monospace;
      color: var(--elohim-compute-tile-meta-color, CanvasText);
      word-break: break-all;
    }

    .meta dt {
      font-weight: 600;
      display: inline;
    }

    .meta dt::after {
      content: ': ';
    }

    .meta dd {
      display: inline;
      margin-inline-start: 0;
    }

    .meta dd::after {
      content: '';
      display: block;
    }

    /* Forced-colors mode */
    @media (forced-colors: active) {
      .container {
        background: Canvas;
        color: CanvasText;
        border: 1px solid CanvasText;
      }

      .value-primary {
        color: CanvasText;
      }

      .value-detail {
        color: CanvasText;
      }

      .status-dot[data-online='true'] {
        color: Highlight;
      }

      .label {
        color: CanvasText;
      }

      .gauge {
        background: ButtonFace;
      }

      .gauge-fill {
        background: ButtonText;
      }

      .meta {
        color: CanvasText;
        border-block-start-color: CanvasText;
      }
    }

    /* Reduced-transparency: elements that use opacity for visual layering restore it
       only when transparency is acceptable. Default (no media query) is full opacity
       so forced-colors and high-contrast environments always get full legibility. */
    @media (prefers-reduced-transparency: no-preference) {
      .value-detail {
        opacity: 0.8;
      }

      .state-banner {
        opacity: 0.7;
      }
    }

    /* Touch target floor for the host (when interactive / receives focus) */
    :host(:focus-visible) .container {
      outline: 2px solid var(--elohim-compute-tile-fg, CanvasText);
      outline-offset: 2px;
    }
  `;

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  @property({ attribute: false })
  value: ComputeTileValue | null = null;

  @property({ attribute: 'tile-state', reflect: true })
  tileState: ComputeTileState = 'idle';

  @property({ attribute: 'last-sample-at' })
  lastSampleAt: string | null = null;

  @property({ attribute: 'source-route' })
  sourceRoute: string | null = null;

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  override render() {
    const lens = this.profile.lens;
    const locale = this.profile.locale === 'auto' ? navigator.language : this.profile.locale;
    const isSymbolic = this.profile.textuality === 'symbolic';

    // State banners (loading / error / offline / stale / empty)
    if (this.tileState === 'loading') return this._renderState('loading');
    if (this.tileState === 'error') return this._renderState('error');
    if (this.tileState === 'offline') return this._renderState('offline');
    if (this.tileState === 'stale') return this._renderState('stale');
    if (this.tileState === 'empty' || this.value === null) return this._renderState('empty');

    const { free, used, stewarded } = extractTriptych(this.value);
    const total = free !== null && used !== null ? free + used : null;

    const label = this._contextLabel();
    const isOnline = this.value.kind === 'device' ? this.value.online : true;

    return html`
      <div
        class="container"
        part="container"
        role="group"
        aria-label=${this._ariaLabel(free, total)}
      >
        <div class="header" part="header">
          <span
            class="status-dot"
            part="status-dot"
            data-online=${isOnline ? 'true' : 'false'}
            aria-hidden="true"
          ></span>
          <span class="label" part="label">
            <slot name="label">${label}</slot>
          </span>
        </div>

        <div class="body" part="body">
          ${this._renderBody(lens, isSymbolic, free, used, stewarded, total, locale)}
        </div>

        ${lens === 'debug' || lens === 'trace' ? this._renderMeta(lens) : nothing}
      </div>
    `;
  }

  // ---------------------------------------------------------------------------
  // Private render helpers
  // ---------------------------------------------------------------------------

  private _qualitativeSummary(free: bigint | null): unknown {
    if (free === null) return msg('No sample available.');
    if (free > 1_073_741_824n) return msg('You have enough.');
    return msg('Running low.');
  }

  private _freeOfTotalText(freeStr: string, totalStr: string, total: bigint | null): unknown {
    if (total === null) return msg(str`${freeStr} free`);
    return msg(str`${freeStr} of ${totalStr} free`);
  }

  private _renderBody(
    lens: string,
    isSymbolic: boolean,
    free: bigint | null,
    used: bigint | null,
    stewarded: bigint | null,
    total: bigint | null,
    locale: string
  ) {
    const freeStr = formatBytes(free, locale);
    const totalStr = formatBytes(total, locale);
    const gauge = isSymbolic ? this._renderGauge(free, total) : nothing;

    if (lens === 'minimal') {
      return html`
        <span class="value-primary" part="value-primary">${this._qualitativeSummary(free)}</span>
      `;
    }

    if (lens === 'simple') {
      return html`
        <span class="value-primary" part="value-primary">
          ${gauge} ${msg(str`${freeStr} free`)}
        </span>
      `;
    }

    if (lens === 'standard') {
      return html`
        ${gauge}
        <span class="value-primary" part="value-primary">
          ${this._freeOfTotalText(freeStr, totalStr, total)}
        </span>
      `;
    }

    // detail / debug / trace — full triptych
    const stewardedStr = formatBytes(stewarded, locale);
    const selfStr = used === null ? '—' : formatBytes(used, locale);
    return html`
      ${gauge}
      <span class="value-primary" part="value-primary">
        ${this._freeOfTotalText(freeStr, totalStr, total)}
      </span>
      <span class="value-detail" part="value-detail">
        ${msg(str`${stewardedStr} stewarded · ${selfStr} used`)}
      </span>
    `;
  }

  private _renderGauge(free: bigint | null, total: bigint | null) {
    if (free === null || total === null || total === 0n) {
      return html`
        <div class="gauge" part="gauge" aria-hidden="true">
          <div class="gauge-fill" style="inline-size: 0%;"></div>
        </div>
      `;
    }
    const pct = Math.min(100, Math.round(Number((free * 100n) / total)));
    return html`
      <div class="gauge" part="gauge" aria-hidden="true">
        <div class="gauge-fill" style="inline-size: ${pct}%;"></div>
      </div>
    `;
  }

  // eslint-disable-next-line sonarjs/function-return-type -- html template and nothing() are both valid Lit renderable; explicit union type loses readability
  private _renderMetaIdLine() {
    if (this.value?.kind === 'hub') {
      return html`
        <dt>${msg('Hub')}</dt>
        <dd>${this.value.hubId}</dd>
      `;
    }
    if (this.value?.kind === 'device') {
      return html`
        <dt>${msg('Peer')}</dt>
        <dd>${this.value.peerId}</dd>
      `;
    }
    return nothing;
  }

  private _renderMeta(lens: string) {
    const idLine = this._renderMetaIdLine();

    const memberLine =
      this.value?.kind === 'hub' && this.value.memberDeviceCount !== undefined
        ? html`
            <dt>${msg('Members')}</dt>
            <dd>${this.value.memberDeviceCount}</dd>
          `
        : nothing;

    const sampleLine = this.lastSampleAt
      ? html`
          <dt>${msg('Sampled')}</dt>
          <dd>${this.lastSampleAt}</dd>
        `
      : nothing;

    const routeLine =
      lens === 'trace' && this.sourceRoute
        ? html`
            <dt>${msg('Route')}</dt>
            <dd>${this.sourceRoute}</dd>
          `
        : nothing;

    return html`
      <dl class="meta" part="meta">${idLine} ${memberLine} ${sampleLine} ${routeLine}</dl>
    `;
  }

  private _renderState(state: ComputeTileState) {
    let message: unknown;
    switch (state) {
      case 'loading':
        message = msg('Loading compute data…');
        break;
      case 'error':
        message = msg('Unable to load compute data.');
        break;
      case 'offline':
        message = msg('Device is offline.');
        break;
      case 'stale':
        message = msg('Compute data may be stale.');
        break;
      case 'empty':
      default:
        message = msg('No compute data available.');
        break;
    }
    return html`
      <div class="container" part="container" role="status" aria-live="polite">
        <span class="state-banner" part="state-banner">${message}</span>
      </div>
    `;
  }

  private _contextLabel(): string {
    if (!this.value) return '';
    if (this.value.kind === 'hub') {
      return this.value.displayLabel ?? this.value.hubId;
    }
    const archetypeLabel = ARCHETYPE_LABELS[this.value.archetype]?.() ?? this.value.archetype;
    return this.value.displayName
      ? `${archetypeLabel} (${this.value.displayName})`
      : archetypeLabel;
  }

  private _ariaLabel(free: bigint | null, total: bigint | null): string {
    const locale = this.profile.locale === 'auto' ? navigator.language : this.profile.locale;
    const label = this._contextLabel();
    const freeStr = formatBytes(free, locale);
    const totalStr = formatBytes(total, locale);
    if (total !== null) {
      return msg(str`Compute tile for ${label}: ${freeStr} of ${totalStr} free`);
    }
    return msg(str`Compute tile for ${label}: ${freeStr} free`);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-compute-tile': ElohimComputeTile;
  }
}
