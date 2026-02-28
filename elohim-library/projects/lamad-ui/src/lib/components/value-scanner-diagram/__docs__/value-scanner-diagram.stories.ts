import type { Meta, StoryObj } from '@storybook/angular';

import { ValueScannerDiagramComponent } from '../value-scanner-diagram.component';

const meta: Meta<ValueScannerDiagramComponent> = {
  title: 'Components/Value Scanner Diagram',
  component: ValueScannerDiagramComponent,
  parameters: {
    docs: {
      description: {
        component:
          'Animated timeline showing the **Value Scanner Protocol** — how a simple purchase becomes a ' +
          'multi-dimensional bundle of economic, social, and emotional value.\n\n' +
          'The diagram auto-advances through four steps every 2.5 seconds:\n\n' +
          '1. **Scan** — Tommy scans strawberries\n' +
          '2. **Negotiate** — Agents align budget vs. care intent\n' +
          '3. **Bundle** — Economic + social + supply-chain data merged\n' +
          '4. **Story** — Care token earned; value made visible\n\n' +
          'The transaction bundle card below the timeline reveals Care Value and Community provenance as the step advances.',
      },
    },
    backgrounds: { default: 'stone-light' },
    layout: 'centered',
  },
};

export default meta;
type Story = StoryObj<ValueScannerDiagramComponent>;

/**
 * Default auto-advancing timeline — watch all four steps cycle automatically.
 */
export const Default: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Auto-advancing through all four protocol steps. The transaction bundle reveals Care and Community fields at step 3.',
      },
    },
  },
};

/** Stone-mid background — as used in the protocol explainer section. */
export const OnMidBackground: Story = {
  parameters: {
    backgrounds: { default: 'stone-mid' },
    docs: {
      description: {
        story: 'Value Scanner on a slightly darker stone surface, as used in alternating explainer sections.',
      },
    },
  },
};

/** Full-width on a laptop viewport — timeline labels fully visible. */
export const LaptopLayout: Story = {
  parameters: {
    viewport: { defaultViewport: 'laptop' },
    docs: {
      description: {
        story: 'On wider viewports the timeline stretches to fill the container, reducing label truncation.',
      },
    },
  },
};

/** Narrow — step labels collapse gracefully. */
export const MobileLayout: Story = {
  parameters: {
    viewport: { defaultViewport: 'mobile' },
    docs: {
      description: {
        story: 'On mobile, the step icons stack and label text wraps. The bundle card remains readable.',
      },
    },
  },
};
