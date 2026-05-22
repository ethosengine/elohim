import { describe, expect, it } from 'vitest';
import { render } from 'lit';
import { DOWELL_HOUSEHOLD_RUBRIC } from '../../../default/qahal/fixtures/primitives/mock-rubrics';
import { DOWELL_FAMILY } from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import {
  DOWELL_TUESDAY_MORNING_STREAM,
} from '../../../default/qahal/fixtures/primitives/mock-care-economy-events';
import { DOWELL_HOUSEHOLD_TOPOLOGY } from '../../../default/qahal/fixtures/primitives/mock-social-compute-topology';
import type { Scene, RenderOpts } from './types';
import { renderQahalHomepage } from './render-qahal-homepage';

const baseScene: Scene = {
  id: 'dowell-household',
  qahalIcon: '🏠',
  qahalLabel: "Dowell Household",
  qahalArchetype: 'household',
  otherQahals: [
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
  ],
  rubric: DOWELL_HOUSEHOLD_RUBRIC,
  members: DOWELL_FAMILY,
  streamEvents: DOWELL_TUESDAY_MORNING_STREAM,
  computeTopology: DOWELL_HOUSEHOLD_TOPOLOGY,
  coStewardObservation: 'The household is steady.',
  curatedEprs: [
    { id: 'family-recipes', title: 'Family Recipes', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'family-doc',
      title: 'Family Google Doc',
      url: 'https://docs.google.com/example',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: [],
};

const baseOpts: RenderOpts = {
  viewerTier: 'steward',
  powerUserVisible: false,
  lens: 'standard',
};

function renderToHost(scene: Scene, opts: RenderOpts): HTMLElement {
  const host = document.createElement('div');
  render(renderQahalHomepage(scene, opts), host);
  return host;
}

describe('renderQahalHomepage — chrome assembly', () => {
  it('renders all four chrome elements', () => {
    const host = renderToHost(baseScene, baseOpts);
    expect(host.querySelector('elohim-qahal-collective-switcher')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-sidebar')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-main-viewer')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-context-column')).toBeTruthy();
  });

  it('passes scene.qahalLabel to the sidebar as qahal-name', () => {
    const host = renderToHost(baseScene, baseOpts);
    const sidebar = host.querySelector('elohim-qahal-sidebar');
    expect(sidebar?.getAttribute('qahal-name')).toBe('Dowell Household');
  });

  it('passes the scene id as the active-collective-id of the switcher', () => {
    const host = renderToHost(baseScene, baseOpts);
    const switcher = host.querySelector('elohim-qahal-collective-switcher');
    expect(switcher?.getAttribute('active-collective-id')).toBe('dowell-household');
  });

  it('honors opts.activeQahalId override on the switcher', () => {
    const host = renderToHost(baseScene, { ...baseOpts, activeQahalId: 'cofc-congregation' });
    const switcher = host.querySelector('elohim-qahal-collective-switcher');
    expect(switcher?.getAttribute('active-collective-id')).toBe('cofc-congregation');
  });
});
