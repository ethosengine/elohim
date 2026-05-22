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

describe('renderQahalHomepage — external-link capability gating', () => {
  // The Dowell rubric (DOWELL_HOUSEHOLD_RUBRIC.externalLinkVisibility) maps:
  //   visitor → filtered_via_co_steward
  //   engaged / contributor / steward → full
  //   child / legal_steward_protected → hidden
  //   idd_member / elder_under_guardianship → filtered_via_co_steward
  // (elohim-support is not in the rubric → composer falls back to 'full')

  it.each(['engaged', 'contributor', 'steward'] as const)(
    'shows external-link section in full mode for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      const list = host.querySelector('elohim-qahal-external-link-list') as HTMLElement & {
        links?: Array<{ id: string; url: string; label: string }>;
      };
      expect(list).toBeTruthy();
      // 'full' visibility passes all scene.externalLinks (length 1 in baseScene)
      expect(list?.links?.length).toBe(baseScene.externalLinks.length);
    }
  );

  it('shows external-link section in filtered mode for visitor (filters per visibilityRequirement)', () => {
    const host = renderToHost(baseScene, { ...baseOpts, viewerTier: 'visitor' });
    const list = host.querySelector('elohim-qahal-external-link-list') as HTMLElement & {
      links?: Array<{ id: string; url: string; label: string }>;
    };
    expect(list).toBeTruthy();
    // 'filtered_via_co_steward' filters to links whose visibilityRequirement includes 'visitor'.
    // The baseScene's single link has visibilityRequirement = ['engaged','contributor','steward'],
    // so 'visitor' is NOT in that set → 0 links survive the filter.
    expect(list?.links?.length).toBe(0);
  });

  it('forwards viewer-tier attribute to the external-link-list element', () => {
    const host = renderToHost(baseScene, { ...baseOpts, viewerTier: 'visitor' });
    const list = host.querySelector('elohim-qahal-external-link-list');
    expect(list?.getAttribute('viewer-tier')).toBe('visitor');
  });

  it.each(['child', 'legal_steward_protected'] as const)(
    'OMITS external-link section entirely from DOM for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      expect(host.querySelector('elohim-qahal-external-link-list')).toBeNull();
    }
  );

  it.each(['idd_member', 'elder_under_guardianship'] as const)(
    'shows external-link section in filtered mode for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      const list = host.querySelector('elohim-qahal-external-link-list') as HTMLElement & {
        links?: Array<{ id: string; url: string; label: string }>;
      };
      expect(list).toBeTruthy();
      // Same filter applies: baseScene's single link doesn't include these tiers in
      // visibilityRequirement → 0 links survive.
      expect(list?.links?.length).toBe(0);
    }
  );

  it('falls back to full visibility for tiers absent from the rubric (elohim-support)', () => {
    const host = renderToHost(baseScene, { ...baseOpts, viewerTier: 'elohim-support' });
    const list = host.querySelector('elohim-qahal-external-link-list') as HTMLElement & {
      links?: Array<{ id: string; url: string; label: string }>;
    };
    expect(list).toBeTruthy();
    expect(list?.links?.length).toBe(baseScene.externalLinks.length);
  });

  it('maps Scene ExternalLink.title to element ExternalLink.label', () => {
    const host = renderToHost(baseScene, { ...baseOpts, viewerTier: 'steward' });
    const list = host.querySelector('elohim-qahal-external-link-list') as HTMLElement & {
      links?: Array<{ id: string; url: string; label: string }>;
    };
    expect(list?.links?.[0]?.label).toBe('Family Google Doc');
    expect(list?.links?.[0]?.url).toBe('https://docs.google.com/example');
  });
});

describe('renderQahalHomepage — power-user gating', () => {
  it('OMITS power-user-expandable when powerUserVisible=false', () => {
    const host = renderToHost(baseScene, { ...baseOpts, powerUserVisible: false });
    expect(host.querySelector('elohim-qahal-power-user-expandable')).toBeNull();
  });

  it('mounts power-user-expandable when powerUserVisible=true', () => {
    const host = renderToHost(baseScene, { ...baseOpts, powerUserVisible: true });
    expect(host.querySelector('elohim-qahal-power-user-expandable')).toBeTruthy();
  });
});
