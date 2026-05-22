/**
 * Library B canonical pattern story — Dowell Household Tuesday-morning.
 *
 * Renders the storyteller §4.1 named moment: sick James, Sheila's soup,
 * Gertrude's check-in, "the household is steady."
 *
 * Default rendering: viewerTier='steward' (Matthew), lens='standard',
 * powerUserVisible=false, activePanel='stream'.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Dowell Household',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Tuesday-morning Dowell household scene from canonical narrative §4.1.

James is sick. Sheila sent chicken-and-rice soup the night before. Gertrude
checked in from half a continent away. Three pending acknowledgments wait
in the stream. The co-steward writes a quiet observation: "the household is steady."

Rendered through Matthew's steward lens at the standard capability profile —
the household's adult-pilot default view.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const TuesdayMorning: Story = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
