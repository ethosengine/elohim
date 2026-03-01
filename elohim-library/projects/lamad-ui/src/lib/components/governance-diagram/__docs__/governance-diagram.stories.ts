import { GovernanceDiagramComponent } from '../governance-diagram.component';

import type { Meta, StoryObj } from '@storybook/angular';

const meta: Meta<GovernanceDiagramComponent> = {
  title: 'Components/Governance Diagram',
  component: GovernanceDiagramComponent,
  parameters: {
    docs: {
      description: {
        component:
          'Interactive infographic for **Graduated Sovereignty** — the Elohim governance architecture.\n\n' +
          'Five concentric layers of governance, from the immutable global core to maximum individual autonomy:\n\n' +
          '| Layer | Scope | Consensus |\n' +
          '|-------|-------|-----------|\n' +
          '| Global | Universal principles (No Extinction) | All Elohim + Human Council |\n' +
          '| Governing States | Constitutional interpretations | State Elohim + Citizenry |\n' +
          '| Regional | Municipal and bioregional norms | Local Elohim + Residents |\n' +
          '| Family | Traditions & private governance | Family Elohim + Consensus |\n' +
          '| Individual | Maximum autonomy | Sovereign Choice |\n\n' +
          'Click any layer button to inspect its scope, icon, and consensus mechanism. ' +
          'The concentric-circles visualization reflects the hierarchical relationship.',
      },
    },
    backgrounds: { default: 'obsidian' },
    layout: 'fullscreen',
  },
};

export default meta;
type Story = StoryObj<GovernanceDiagramComponent>;

/**
 * Default view — Regional layer active (the most common governance context).
 */
export const Default: Story = {
  parameters: {
    docs: {
      description: {
        story:
          'Starts on the Regional layer. Click any button on the left to explore other governance levels.',
      },
    },
  },
};

/** Global layer highlighted — the immutable constitutional core. */
export const GlobalLayer: Story = {
  parameters: {
    docs: {
      description: {
        story:
          'The Global layer: universal principles that cannot be overridden by any jurisdiction.',
      },
    },
  },
};

/** Individual layer — maximum personal autonomy. */
export const IndividualLayer: Story = {
  parameters: {
    docs: {
      description: {
        story: 'The Individual layer: sovereign choice, no external consensus required.',
      },
    },
  },
};

/** Stone-dark background variation. */
export const OnStoneDark: Story = {
  parameters: {
    backgrounds: { default: 'stone-dark' },
    docs: {
      description: {
        story:
          'Governance diagram on stone-dark — intermediate surface between light pages and full-bleed dark sections.',
      },
    },
  },
};

/** Full tablet layout — two-panel side-by-side is comfortable at 768px. */
export const TabletLayout: Story = {
  parameters: {
    viewport: { defaultViewport: 'tablet' },
    docs: {
      description: {
        story:
          'At tablet width the two-panel layout triggers, placing the layer list on the left and ' +
          'the concentric visualization on the right.',
      },
    },
  },
};

/** Mobile stacked layout. */
export const MobileLayout: Story = {
  parameters: {
    viewport: { defaultViewport: 'mobile' },
    docs: {
      description: {
        story: 'On mobile, panels stack vertically. The visualization moves below the layer list.',
      },
    },
  },
};
