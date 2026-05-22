import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/Engaged View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Dowell household viewed by an engaged member. Full external-link visibility; power-user panels off.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'engaged',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
