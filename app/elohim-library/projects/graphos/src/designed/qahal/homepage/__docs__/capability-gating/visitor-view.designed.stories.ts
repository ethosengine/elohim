import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/Visitor View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          "Dowell household viewed by a visitor. External-link section is filtered via the co-steward (per the rubric's visitor visibility policy).",
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'visitor',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
