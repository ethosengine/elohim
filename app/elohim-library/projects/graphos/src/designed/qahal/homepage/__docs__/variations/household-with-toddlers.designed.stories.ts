import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { householdWithToddlers } from '../../../../../default/qahal/fixtures/variations/household-with-toddlers';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Household With Toddlers',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — the Dowell household viewed at a season when care is mostly under-three care. Stream activity is dominated by repetitive small-care events; member-ring shows non-pilot capability tiers (toddler is child-tier).',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(householdWithToddlers, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
