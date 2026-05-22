import type { Decorator, Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { wisdomCommonsThursdayAfternoon } from '../../../../../default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Wisdom Commons',
  decorators: [qahalLightDecorator as unknown as Decorator],
  parameters: {
    docs: {
      description: {
        component: `
The Thursday-afternoon wisdom commons scene from canonical narrative §4.4.

83 participating congregations. Brother Cal's congregation has been in
fellowship with the Arkansas sister congregation for thirty years. A
concern has surfaced from a recent teaching. The substrate convenes a
voluntary peer council; the council sits with the witness for two months.
The Arkansas congregation writes back: the teaching has been reconsidered.
An REA reconciliation event is recorded.

Rendered through Brother Cal's contributor lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;

export const ThursdayAfternoon: StoryObj = {
  render: () =>
    renderQahalHomepage(wisdomCommonsThursdayAfternoon, {
      viewerTier: 'contributor',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
