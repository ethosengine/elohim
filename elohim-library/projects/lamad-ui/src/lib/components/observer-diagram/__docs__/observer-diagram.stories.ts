import { ObserverDiagramComponent } from '../observer-diagram.component';

import type { Meta, StoryObj } from '@storybook/angular';

const meta: Meta<ObserverDiagramComponent> = {
  title: 'Components/Observer Diagram',
  component: ObserverDiagramComponent,
  parameters: {
    docs: {
      description: {
        component:
          'Animated infographic for the **Elohim Observer Protocol** — from surveillance to witness. ' +
          'A physical privacy switch controls two modes:\n\n' +
          '- **Witness mode**: The Observer sees, extracts value, then cryptographically destroys the raw data. ' +
          'Animated data packets float upward and dissolve; a "Story Preserved" badge appears.\n' +
          '- **Private mode**: No data flows. The camera lens darkens, packets stop.',
      },
    },
    backgrounds: { default: 'white' },
    layout: 'centered',
  },
};

export default meta;
type Story = StoryObj<ObserverDiagramComponent>;

/**
 * Default state — Observer starts in Witness mode.
 * Click the physical switch to toggle to Private mode.
 */
export const WitnessMode: Story = {
  parameters: {
    docs: {
      description: {
        story:
          'Starts in **Witness** mode. Data packets animate upward and are destroyed after value extraction. ' +
          'Click the switch to toggle privacy.',
      },
    },
  },
};

/**
 * Demonstrates the component in its white-background context,
 * matching the intended usage inside lamad-infographic sections.
 */
export const OnLightBackground: Story = {
  parameters: {
    backgrounds: { default: 'stone-light' },
    docs: {
      description: {
        story:
          'Observer on the standard stone-light surface — as it appears in the main protocol explainer.',
      },
    },
  },
};

/** Dark canvas — suitable for full-bleed section backgrounds. */
export const OnDarkBackground: Story = {
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          "Observer on an obsidian background. The card's white surface creates strong contrast, " +
          'suitable for dark-mode landings.',
      },
    },
  },
};
