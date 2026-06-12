import type { Preview } from '@storybook/web-components';

// Brand webfont substrate (@font-face only — see fonts.css header). Without
// this, every `--el-font-*` binding in Library B silently falls back to
// system-ui/Georgia and the typography half of the design spec can't execute.
import './fonts.css';
import 'elohim-core/register';
import { VIEWPORT_ARCHETYPES } from 'elohim-core';

// Viewport archetypes are a first-class matrix dimension, sibling of the
// light/dark theme cells and the a11y gate: the same named set drives the
// element viewport gate (elohim-core/src/testing/viewport.ts), the a2o
// browser steps, the graphos sheet's --viewports bands, and this selector.
const viewportOptions = Object.fromEntries(
  Object.entries(VIEWPORT_ARCHETYPES).map(([key, archetype]) => [
    key,
    {
      name: `${key} (${archetype.width}×${archetype.height} — ${archetype.represents})`,
      styles: { width: `${archetype.width}px`, height: `${archetype.height}px` },
    },
  ])
);

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
    viewport: {
      options: viewportOptions,
    },
  },
};

export default preview;
