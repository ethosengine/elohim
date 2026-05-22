import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { congregationNewlyFormed } from '../../../../../default/qahal/fixtures/variations/congregation-newly-formed';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Congregation Newly Formed',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — the congregation in its first season. Stream is sparse (truncated to first 3 events); cohesion thresholds are not yet reached; the substrate watches and waits.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(congregationNewlyFormed, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
