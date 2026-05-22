import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { lifeGroupNewlyFormed } from '../../../../../default/qahal/fixtures/variations/life-group-newly-formed';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Life Group Newly Formed',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — counterpoint to the canonical Hardins (three years cohesive). Three meetings in; vulnerability has not yet been offered. The substrate honors patience.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(lifeGroupNewlyFormed, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
