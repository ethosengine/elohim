import type { Preview } from '@storybook/web-components';

import 'elohim-core/register';

const preview: Preview = {
  parameters: {
    backgrounds: {
      default: 'dark',
      values: [
        { name: 'dark', value: '#0a0a0a' },
        { name: 'light', value: '#f3f4f6' },
      ],
    },
    a11y: {
      element: '#storybook-root',
      manual: false,
    },
  },
};

export default preview;
