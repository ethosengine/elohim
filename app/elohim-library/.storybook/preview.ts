import type { Preview } from '@storybook/angular';
import { applicationConfig } from '@storybook/angular';
import { provideAnimations } from '@angular/platform-browser/animations';

const withAnimations = applicationConfig({
  providers: [provideAnimations()],
});

const preview: Preview = {
  decorators: [withAnimations],

  parameters: {
    actions: { argTypesRegex: '^on[A-Z].*' },

    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },

    backgrounds: {
      default: 'stone-light',
      values: [
        { name: 'stone-light', value: '#f9f8f4' },
        { name: 'stone-mid',   value: '#e7e5e4' },
        { name: 'stone-dark',  value: '#292524' },
        { name: 'obsidian',    value: '#1c1917' },
        { name: 'white',       value: '#ffffff' },
      ],
    },

    viewport: {
      viewports: {
        mobile: { name: 'Mobile', styles: { width: '390px',  height: '844px' } },
        tablet: { name: 'Tablet', styles: { width: '768px',  height: '1024px' } },
        laptop: { name: 'Laptop', styles: { width: '1280px', height: '800px' } },
        wide:   { name: 'Wide',   styles: { width: '1440px', height: '900px' } },
      },
    },

    options: {
      storySort: {
        order: ['Introduction', 'Components', ['Hexagon Grid', 'Observer Diagram', 'Value Scanner Diagram', 'Governance Diagram'], '*'],
      },
    },
  },

  tags: ['autodocs'],
};

export default preview;
