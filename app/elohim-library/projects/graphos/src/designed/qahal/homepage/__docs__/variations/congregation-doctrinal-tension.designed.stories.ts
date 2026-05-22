import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { congregationDoctrinalTension } from '../../../../../default/qahal/fixtures/variations/congregation-doctrinal-tension';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Congregation Doctrinal Tension',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — a teaching from a sister congregation has surfaced concern among the elders. The dispute-mediation curated EPR is available in the sidebar; the co-steward observation acknowledges the tension without prescribing.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(congregationDoctrinalTension, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
