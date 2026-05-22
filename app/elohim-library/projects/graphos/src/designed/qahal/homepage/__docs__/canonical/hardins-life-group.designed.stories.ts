import type { Decorator, Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { hardinsLifeGroupTuesdayEvening } from '../../../../../default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Hardins Life-Group',
  decorators: [qahalLightDecorator as unknown as Decorator],
  parameters: {
    docs: {
      description: {
        component: `
The Tuesday-evening life-group scene from canonical narrative §4.3.

Six families. Three years of fellowship. Romans 12 verse 1 discussion.
Sarah's father is in hospital. The substrate has noticed John Hardin
hosted twenty-three of the last twenty-four Tuesdays — a gentle prompt
surfaces. The Lees offer to host next.

Rendered through John Hardin's steward lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;

export const TuesdayEvening: StoryObj = {
  render: () =>
    renderQahalHomepage(hardinsLifeGroupTuesdayEvening, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
