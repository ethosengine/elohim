/**
 * Shared decorators for Library B Qahal-pillar pattern stories.
 *
 * Three exports — `qahalLightDecorator`, `qahalDarkDecorator`,
 * `qahalHighContrastDecorator` — each wraps a Storybook story in a styled
 * wrapper div carrying the full Elohim brand-token block (`--el-*`) plus
 * the per-element `--elohim-qahal-*-*` token bindings.
 *
 * The decorators are the single source of brand-binding truth across all
 * homepage stories. Per `app/elohim-library/CLAUDE.md`:
 *   - Stories NEVER modify primitives' CSS, JSDoc, tag names, or behavior
 *   - Binding happens at the story-decorator level only
 *
 * Brand-token reference: graphos/elohim-protocol-design-spec.md §14.
 * Pattern reference: hub-aggregation-shift.designed.stories.ts.
 */

import { html, type TemplateResult } from 'lit';

// ---------------------------------------------------------------------------
// Brand-token block — the full --el-* palette + spacing + typography stack
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-night-alt:   #1A1A2E;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft:   0 2px 8px rgba(107, 97, 87, 0.08);
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

// ---------------------------------------------------------------------------
// Per-element token bindings — light mode (the garden register)
// ---------------------------------------------------------------------------

const QAHAL_TOKENS_LIGHT = `
  /* chrome surface colors */
  --elohim-color-surface-0: var(--el-cream);
  --elohim-color-surface-1: var(--el-starlight);
  --elohim-color-surface-2: rgba(107, 97, 87, 0.08);
  --elohim-color-border:    rgba(107, 97, 87, 0.18);
  --elohim-color-focus:     var(--el-green-light);
  --elohim-color-accent:    var(--el-amber);
  --elohim-color-text:      var(--el-stone);
  --elohim-color-text-emphasis: var(--el-green-deep);

  /* qahal-pillar element token bindings */
  --elohim-qahal-collective-switcher-active-bg:  rgba(127, 176, 105, 0.18);
  --elohim-qahal-sidebar-bg:                     var(--el-cream);
  --elohim-qahal-sidebar-border:                 1px solid rgba(107, 97, 87, 0.18);
  --elohim-qahal-main-viewer-bg:                 var(--el-cream);
  --elohim-qahal-context-column-bg:              var(--el-starlight);
  --elohim-qahal-context-column-border:          1px solid rgba(107, 97, 87, 0.18);
  --elohim-qahal-stream-panel-divider:           1px solid rgba(107, 97, 87, 0.12);
  --elohim-qahal-stream-item-acknowledgment-border: 1px dashed var(--el-amber);
  --elohim-qahal-member-ring-fill:               var(--el-green-light);
  --elohim-qahal-member-ring-track:              var(--el-starlight);
  --elohim-qahal-standing-ring-fill:             var(--el-amber);
  --elohim-qahal-provenance-marker-curated-color: var(--el-green-deep);
  --elohim-qahal-provenance-marker-external-color: var(--el-stone);
  --elohim-qahal-capability-tier-chip-bg:        rgba(127, 176, 105, 0.12);
  --elohim-qahal-capability-tier-chip-fg:        var(--el-green-deep);
  --elohim-qahal-co-steward-panel-bg:            var(--el-starlight);
  --elohim-qahal-co-steward-panel-accent:        var(--el-plum);
`;

const QAHAL_TOKENS_DARK = `
  --elohim-color-surface-0: var(--el-night);
  --elohim-color-surface-1: var(--el-night-alt);
  --elohim-color-surface-2: rgba(232, 228, 217, 0.08);
  --elohim-color-border:    rgba(232, 228, 217, 0.18);
  --elohim-color-focus:     var(--el-amber);
  --elohim-color-accent:    var(--el-amber);
  --elohim-color-text:      var(--el-starlight);
  --elohim-color-text-emphasis: var(--el-amber);

  --elohim-qahal-collective-switcher-active-bg:  rgba(212, 160, 62, 0.18);
  --elohim-qahal-sidebar-bg:                     var(--el-night);
  --elohim-qahal-sidebar-border:                 1px solid rgba(232, 228, 217, 0.12);
  --elohim-qahal-main-viewer-bg:                 var(--el-night);
  --elohim-qahal-context-column-bg:              var(--el-night-alt);
  --elohim-qahal-context-column-border:          1px solid rgba(232, 228, 217, 0.12);
  --elohim-qahal-stream-panel-divider:           1px solid rgba(232, 228, 217, 0.08);
  --elohim-qahal-stream-item-acknowledgment-border: 1px dashed var(--el-amber);
  --elohim-qahal-member-ring-fill:               var(--el-green-light);
  --elohim-qahal-member-ring-track:              rgba(232, 228, 217, 0.12);
  --elohim-qahal-standing-ring-fill:             var(--el-amber);
  --elohim-qahal-provenance-marker-curated-color: var(--el-amber);
  --elohim-qahal-provenance-marker-external-color: var(--el-starlight);
  --elohim-qahal-capability-tier-chip-bg:        rgba(212, 160, 62, 0.18);
  --elohim-qahal-capability-tier-chip-fg:        var(--el-amber);
  --elohim-qahal-co-steward-panel-bg:            var(--el-night-alt);
  --elohim-qahal-co-steward-panel-accent:        var(--el-plum);
`;

const QAHAL_TOKENS_HIGH_CONTRAST = `
  --elohim-color-surface-0: var(--el-cream);
  --elohim-color-surface-1: var(--el-cream);
  --elohim-color-surface-2: rgba(15, 26, 18, 0.12);
  --elohim-color-border:    var(--el-night);
  --elohim-color-focus:     var(--el-night);
  --elohim-color-accent:    var(--el-green-deep);
  --elohim-color-text:      var(--el-night);
  --elohim-color-text-emphasis: var(--el-green-deep);

  --elohim-qahal-sidebar-bg:               var(--el-cream);
  --elohim-qahal-sidebar-border:           2px solid var(--el-night);
  --elohim-qahal-main-viewer-bg:           var(--el-cream);
  --elohim-qahal-context-column-bg:        var(--el-cream);
  --elohim-qahal-context-column-border:    2px solid var(--el-night);
  --elohim-qahal-stream-panel-divider:     1px solid var(--el-night);
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function buildWrapperStyle(themeTokens: string, background: string, color: string): string {
  return `
    ${EL_TOKENS}
    ${themeTokens}
    font-family: var(--el-font-ui);
    background: ${background};
    color: ${color};
    padding: var(--el-space-md);
    min-block-size: 100vh;
  `.replace(/\s+/g, ' ');
}

export function qahalLightDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_LIGHT, 'var(--el-cream)', 'var(--el-stone)')}">
      ${story()}
    </div>
  `;
}

export function qahalDarkDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_DARK, 'var(--el-night)', 'var(--el-starlight)')}">
      ${story()}
    </div>
  `;
}

export function qahalHighContrastDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_HIGH_CONTRAST, 'var(--el-cream)', 'var(--el-night)')}">
      ${story()}
    </div>
  `;
}
