/**
 * Library B playground — interactive Qahal-homepage exploration.
 *
 * One story with Storybook argTypes controls for scene + tier + lens +
 * power-user + active-panel. Useful for stakeholder demos and the
 * recognition+distinction Checkpoint F verification.
 *
 * The 18 stable artifacts in this directory remain authoritative — this
 * playground complements them with interactive controls; it does not replace.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../_lib/render-qahal-homepage';
import type { ActivePanel, Lens } from '../../_lib/types';
import type { CapabilityTier } from '../../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';

import { dowellHouseholdTuesdayMorning } from '../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';
import { cofcCongregationSundayMorning } from '../../../../default/qahal/fixtures/canonical/cofc-congregation-sunday-morning';
import { hardinsLifeGroupTuesdayEvening } from '../../../../default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening';
import { wisdomCommonsThursdayAfternoon } from '../../../../default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon';

const SCENE_MAP = {
  'dowell-household-tuesday-morning': dowellHouseholdTuesdayMorning,
  'cofc-congregation-sunday-morning': cofcCongregationSundayMorning,
  'hardins-life-group-tuesday-evening': hardinsLifeGroupTuesdayEvening,
  'wisdom-commons-thursday-afternoon': wisdomCommonsThursdayAfternoon,
} as const;

const CAPABILITY_TIERS: CapabilityTier[] = [
  'visitor',
  'engaged',
  'contributor',
  'steward',
  'elohim-support',
  'child',
  'idd_member',
  'elder_under_guardianship',
  'legal_steward_protected',
];

const LENSES: Lens[] = ['minimal', 'simple', 'standard', 'detail', 'debug', 'trace'];

const PANELS: ActivePanel[] = [
  'stream',
  'member-ring',
  'rules',
  'co-steward',
  'social-compute',
  'standing-inspector',
  'shefa-resources',
  'attestations',
  'graph-discovery',
];

interface PlaygroundArgs {
  sceneId: keyof typeof SCENE_MAP;
  viewerTier: CapabilityTier;
  powerUserVisible: boolean;
  lens: Lens;
  activePanel: ActivePanel;
}

const meta: Meta<PlaygroundArgs> = {
  title: 'Designed/Qahal/Homepage/Playground',
  decorators: [qahalLightDecorator],
  argTypes: {
    sceneId: {
      control: 'select',
      options: Object.keys(SCENE_MAP),
      description: 'Which canonical scene to render',
    },
    viewerTier: {
      control: 'select',
      options: CAPABILITY_TIERS,
      description: 'Capability tier of the viewer',
    },
    powerUserVisible: {
      control: 'boolean',
      description: 'Imagodei setting: power-user view enabled',
    },
    lens: { control: 'select', options: LENSES },
    activePanel: { control: 'select', options: PANELS },
  },
  args: {
    sceneId: 'dowell-household-tuesday-morning',
    viewerTier: 'steward',
    powerUserVisible: false,
    lens: 'standard',
    activePanel: 'stream',
  },
  parameters: {
    docs: {
      description: {
        component: `
Interactive playground for the Qahal homepage architecture.

Use the controls panel to switch scenes, capability tiers, lenses, and
active panels. The Checkpoint F recognition+distinction test:
- Open Dowell + viewerTier=steward — observe James/Sheila/Gertrude
- Then drop viewerTier to 'child' — observe the external-link section vanish
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj<PlaygroundArgs>;

export const Interactive: Story = {
  render: (args) =>
    renderQahalHomepage(SCENE_MAP[args.sceneId], {
      viewerTier: args.viewerTier,
      powerUserVisible: args.powerUserVisible,
      lens: args.lens,
      activePanel: args.activePanel,
    }),
};
