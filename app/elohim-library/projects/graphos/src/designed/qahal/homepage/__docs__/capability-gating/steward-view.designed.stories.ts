import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/Steward View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Dowell household viewed by a steward (Matthew). The default canonical view — rules + co-steward + social-compute panels are editable; external-links full.',
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
