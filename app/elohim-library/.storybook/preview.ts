import type { Preview } from '@storybook/web-components';

// Brand webfont substrate (@font-face only — see fonts.css header). Without
// this, every `--el-font-*` binding in Library B silently falls back to
// system-ui/Georgia and the typography half of the design spec can't execute.
import './fonts.css';
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
