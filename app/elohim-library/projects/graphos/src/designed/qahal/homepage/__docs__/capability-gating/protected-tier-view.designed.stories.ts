import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/Protected Tier View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          "Dowell household viewed by James (the household's child). External-link sidebar section is **DOM-absent** — per the household rubric's protected-tier discipline, James does not see external hyperlinks at all. The dignity-floor protection is visible.",
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'child',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
