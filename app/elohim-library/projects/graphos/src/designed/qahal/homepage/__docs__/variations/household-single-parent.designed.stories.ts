import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { householdSingleParent } from '../../../../../default/qahal/fixtures/variations/household-single-parent';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Household Single Parent',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — one steward (Jessica removed from member list). The substrate has noted the load and is gently surfacing aid offers from the life-group; the household participates honestly at the floor of what it can hold.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(householdSingleParent, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
