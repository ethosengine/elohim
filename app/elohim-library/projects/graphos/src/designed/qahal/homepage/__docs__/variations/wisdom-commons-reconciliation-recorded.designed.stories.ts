import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { wisdomCommonsReconciliationRecorded } from '../../../../../default/qahal/fixtures/variations/wisdom-commons-reconciliation-recorded';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/Wisdom Commons Reconciliation Recorded',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component:
          'Variation — counterpoint to the canonical concern-surface moment. The Arkansas sister congregation has written back; the teaching has been reconsidered; an REA reconciliation event is recorded.',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(wisdomCommonsReconciliationRecorded, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
