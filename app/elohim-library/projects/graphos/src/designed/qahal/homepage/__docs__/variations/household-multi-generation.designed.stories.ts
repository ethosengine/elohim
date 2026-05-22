import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { householdMultiGeneration } from '../../../../../default/qahal/fixtures/variations/household-multi-generation';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Household Multi Generation',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — three generations under one roof. Gertrude (elder under guardianship) is present alongside James (child). Co-steward observation captures bidirectional care flowing through the household.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(householdMultiGeneration, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
