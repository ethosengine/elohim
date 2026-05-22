/**
 * The single composer for all Library B Qahal-homepage stories.
 *
 * `renderQahalHomepage(scene, opts)` returns a Lit `TemplateResult` carrying
 * the four-column chrome assembly:
 *   1. <elohim-qahal-collective-switcher> — the far-left icon strip
 *   2. <elohim-qahal-sidebar> — the per-Qahal resource list
 *   3. <elohim-qahal-main-viewer> — the active panel
 *   4. <elohim-qahal-context-column> — persistent right-nav context
 *
 * Branching behavior:
 *   - External-link section gates on opts.viewerTier × scene.rubric.externalLinkVisibility
 *   - Power-user-expandable section gates on opts.powerUserVisible (DOM-absent when off)
 *   - Main-viewer mounts the element matching opts.activePanel (default 'stream')
 *   - Context column persists co-steward + condensed rules + condensed graph-discovery
 *
 * Per app/elohim-library/CLAUDE.md, the composer NEVER modifies any element's
 * internals — only assembles them with the public prop/slot surface.
 *
 * Prop-name notes (verified against element source 2026-05-22):
 *   - elohim-qahal-co-steward-panel uses `primary-observation` attribute
 *     + `.pendingAcknowledgments` property (NO `compact` attribute exists)
 *   - elohim-qahal-external-link-list uses `.links` + `.visibilityMap`
 *     + `viewer-tier` attribute (NO `filter-mode` attribute)
 *   - elohim-qahal-external-link-list ExternalLink shape: { id, url, label }
 *     (no `title`, no `visibilityRequirement`)
 *   - elohim-qahal-member-ring-panel uses `.reach: number` + `.tiers: MemberTier[]`
 *     (no `.members`); the composer derives reach/tiers from Scene.members
 */

import { html, nothing, type TemplateResult } from 'lit';
import type { CapabilityTier } from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import type { Scene, RenderOpts, ActivePanel } from './types';

// ---------------------------------------------------------------------------
// Element registration — load via barrel imports so Storybook + tests get all
// custom elements defined before the composer renders them.
// ---------------------------------------------------------------------------

import 'elohim-qahal/register';
import 'elohim-imagodei/register';

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function renderQahalHomepage(scene: Scene, opts: RenderOpts): TemplateResult {
  const activeQahalId = opts.activeQahalId ?? scene.id;
  const activePanel = opts.activePanel ?? 'stream';

  return html`
    <div
      class="qahal-homepage-chrome"
      style="display: grid; grid-template-columns: 80px 260px 1fr 280px; min-block-size: 100vh; gap: 0;"
    >
      ${renderCollectiveSwitcher(scene, activeQahalId)}
      ${renderSidebar(scene, opts)}
      ${renderMainViewer(scene, opts, activePanel)}
      ${renderContextColumn(scene, opts)}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/**
 * Resolves the list of stream-event IDs that are awaiting acknowledgment.
 *
 * Two sources contribute:
 *   1. Per-event `acknowledgmentPending` flag on individual stream events
 *      (the in-place signal authored on the event itself).
 *   2. Scene-level `pendingAcknowledgments` array (a denormalized id list
 *      that lets a scene fixture flag events without rewriting each one).
 *
 * Both the `co-steward` panel branch and the persistent right-nav context
 * column need the same resolved list, so the derivation lives here.
 */
function derivePendingIds(scene: Scene): string[] {
  return scene.streamEvents
    .filter((e) => e.acknowledgmentPending || scene.pendingAcknowledgments.includes(e.id))
    .map((e) => e.id);
}

// ---------------------------------------------------------------------------
// Column 1 — collective switcher
// ---------------------------------------------------------------------------

function renderCollectiveSwitcher(scene: Scene, activeQahalId: string): TemplateResult {
  const collectives = [
    { id: scene.id, icon: scene.qahalIcon, name: scene.qahalLabel },
    ...scene.otherQahals.map((q) => ({ id: q.id, icon: q.icon, name: q.label })),
  ];
  return html`
    <elohim-qahal-collective-switcher
      .collectives=${collectives}
      active-collective-id=${activeQahalId}
    ></elohim-qahal-collective-switcher>
  `;
}

// ---------------------------------------------------------------------------
// Column 2 — sidebar (resource list)
// ---------------------------------------------------------------------------

function renderSidebar(scene: Scene, opts: RenderOpts): TemplateResult {
  return html`
    <elohim-qahal-sidebar qahal-name=${scene.qahalLabel}>
      <elohim-qahal-protocol-panel-list slot="panels"></elohim-qahal-protocol-panel-list>
      <elohim-qahal-curated-epr-list
        slot="curated"
        .eprs=${scene.curatedEprs}
      ></elohim-qahal-curated-epr-list>
      ${renderExternalLinkSection(scene, opts)}
      ${renderPowerUserSection(scene, opts)}
    </elohim-qahal-sidebar>
  `;
}

function renderExternalLinkSection(
  scene: Scene,
  opts: RenderOpts
): TemplateResult | typeof nothing {
  const visibility = scene.rubric.externalLinkVisibility[opts.viewerTier] ?? 'full';
  if (visibility === 'hidden') return nothing;
  // The element expects ExternalLink { id, url, label }. The Scene's
  // ExternalLink carries { id, title, url, visibilityRequirement[] }, so
  // map title→label and pre-filter for the filtered_via_co_steward case.
  //
  // Responsibility split (deliberate — do not collapse without reading both
  // call sites):
  //   - The COMPOSER (here) decides WHICH links exist in the DOM. For
  //     `filtered_via_co_steward`, only links whose visibilityRequirement
  //     includes the viewer's tier are passed to the element. For `full`,
  //     all links are passed. For `hidden`, the section is DOM-absent.
  //   - The ELEMENT (elohim-qahal-external-link-list) decides HOW to
  //     present the surviving links: when viewer-tier resolves to
  //     `filtered_via_co_steward` via .visibilityMap, it wraps the list in
  //     a "filtered via co-steward" annotation so the viewer knows the set
  //     was curated. Both layers cooperate; neither is redundant.
  const sourceLinks =
    visibility === 'filtered_via_co_steward'
      ? scene.externalLinks.filter((l) => l.visibilityRequirement.includes(opts.viewerTier))
      : scene.externalLinks;
  const elementLinks = sourceLinks.map((l) => ({ id: l.id, url: l.url, label: l.title }));
  return html`
    <elohim-qahal-external-link-list
      slot="external"
      .links=${elementLinks}
      .visibilityMap=${scene.rubric.externalLinkVisibility}
      viewer-tier=${opts.viewerTier}
    ></elohim-qahal-external-link-list>
  `;
}

function renderPowerUserSection(
  _scene: Scene,
  opts: RenderOpts
): TemplateResult | typeof nothing {
  if (!opts.powerUserVisible) return nothing;
  return html`
    <elohim-qahal-power-user-expandable slot="power-user"></elohim-qahal-power-user-expandable>
  `;
}

// ---------------------------------------------------------------------------
// Column 3 — main viewer (active panel)
// ---------------------------------------------------------------------------

function renderMainViewer(
  scene: Scene,
  opts: RenderOpts,
  activePanel: ActivePanel
): TemplateResult {
  return html`
    <elohim-qahal-main-viewer active-panel-name=${activePanel}>
      ${renderActivePanel(scene, opts, activePanel)}
    </elohim-qahal-main-viewer>
  `;
}

function renderActivePanel(
  scene: Scene,
  _opts: RenderOpts,
  activePanel: ActivePanel
): TemplateResult {
  const streamEvents = scene.streamEvents.map((e) => ({
    id: e.id,
    authorId: e.authorId,
    timestamp: e.timestamp,
    content: e.content,
    rea: e.rea,
    acknowledgmentPending:
      e.acknowledgmentPending ?? scene.pendingAcknowledgments.includes(e.id),
  }));
  switch (activePanel) {
    case 'stream':
      return html`<elohim-qahal-stream-panel .events=${streamEvents}></elohim-qahal-stream-panel>`;
    case 'member-ring': {
      // The element expects { reach, tiers[] } — derive a simple "members" tier
      // from Scene.members so the panel can render. Stories that want richer
      // multi-tier breakdowns can replace this in a Scene-shaped extension later.
      const tiers = [
        {
          id: 'members',
          label: 'Members',
          count: scene.members.length,
          members: scene.members.map((m) => ({ id: m.id, name: m.name })),
        },
      ];
      return html`
        <elohim-qahal-member-ring-panel
          .reach=${scene.members.length}
          .tiers=${tiers}
        ></elohim-qahal-member-ring-panel>
      `;
    }
    case 'rules':
      return html`<elohim-qahal-rules-panel .rubric=${scene.rubric}></elohim-qahal-rules-panel>`;
    case 'co-steward':
      return html`
        <elohim-qahal-co-steward-panel
          primary-observation=${scene.coStewardObservation}
          .pendingAcknowledgments=${derivePendingIds(scene)}
        ></elohim-qahal-co-steward-panel>
      `;
    case 'social-compute':
      return html`
        <elohim-qahal-social-compute-panel
          .topology=${scene.computeTopology}
        ></elohim-qahal-social-compute-panel>
      `;
    case 'standing-inspector':
      return html`<elohim-qahal-standing-inspector-panel></elohim-qahal-standing-inspector-panel>`;
    case 'shefa-resources':
      return html`<elohim-qahal-shefa-resources-panel></elohim-qahal-shefa-resources-panel>`;
    case 'attestations':
      return html`<elohim-qahal-attestations-panel></elohim-qahal-attestations-panel>`;
    case 'graph-discovery':
      return html`<elohim-qahal-graph-discovery-panel></elohim-qahal-graph-discovery-panel>`;
  }
}

// ---------------------------------------------------------------------------
// Column 4 — context column (persistent right-nav)
// ---------------------------------------------------------------------------

function renderContextColumn(scene: Scene, _opts: RenderOpts): TemplateResult {
  const pending = derivePendingIds(scene);
  return html`
    <elohim-qahal-context-column>
      <elohim-qahal-co-steward-panel
        slot="co-steward"
        primary-observation=${scene.coStewardObservation}
        .pendingAcknowledgments=${pending}
      ></elohim-qahal-co-steward-panel>
      <elohim-qahal-rules-panel slot="rules" .rubric=${scene.rubric}></elohim-qahal-rules-panel>
      <elohim-qahal-graph-discovery-panel
        slot="discovery"
      ></elohim-qahal-graph-discovery-panel>
    </elohim-qahal-context-column>
  `;
}

// Lens forwarding to every element is handled at the decorator level via
// CSS custom properties, not as individual props. If a future Sprint surfaces
// a lens-aware element prop, this composer will pass it through opts.lens.
export type { CapabilityTier };
