import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/User Toggles/Simple User View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Dowell household homepage with **powerUserVisible: false** — the imagodei
setting "Power-user view" disabled (default for most operators).

The 4 power-user panels (standing-inspector, shefa-resources, attestations,
graph-discovery) are DOM-absent. The sidebar's power-user expandable section
is not rendered at all — per the UX spec discipline that this toggle is an
imagodei preference, not a homepage UX gesture.

Compare with the Power User View story to see the same scene with the
toggle enabled.
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
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
