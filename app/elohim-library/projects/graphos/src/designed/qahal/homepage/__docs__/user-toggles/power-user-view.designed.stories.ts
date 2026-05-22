import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/User Toggles/Power User View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Dowell household homepage with **powerUserVisible: true** — the imagodei
setting "Power-user view" enabled.

The 4 power-user panels (standing-inspector, shefa-resources, attestations,
graph-discovery) appear in the sidebar's power-user-expandable section.
The chrome and core panels are otherwise identical to the Simple User View.

This story renders both the visible-stub power-user panels and the
established 5 deep-implementation panels of the simple tier — proving the
architecture's full surface from one toggle flip.
        `.trim(),
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'steward',
      powerUserVisible: true,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
