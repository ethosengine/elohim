import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/Contributor View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Dowell household viewed by a contributor. Same external-link surface as engaged, with additional power-user-eligible affordances available via the imagodei settings palette.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'contributor',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
