import type { Decorator, Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { cofcCongregationSundayMorning } from '../../../../../default/qahal/fixtures/canonical/cofc-congregation-sunday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/CofC Congregation',
  decorators: [qahalLightDecorator as unknown as Decorator],
  parameters: {
    docs: {
      description: {
        component: `
The Sunday-morning CofC congregation scene from canonical narrative §4.2.

230 members. Four elders — Brother Cal, Thompson, Davis, Rhodes — in plural
stewardship. Romans 12 sermon series in its fourth week. The youth retreat
needs two drivers. Three prayer requests. The co-steward notes the
congregation's reach into the neighborhood is rising slightly; three
life-groups are nearing the cohesion threshold.

Rendered through Brother Cal's steward lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;

export const SundayMorning: StoryObj = {
  render: () =>
    renderQahalHomepage(cofcCongregationSundayMorning, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
