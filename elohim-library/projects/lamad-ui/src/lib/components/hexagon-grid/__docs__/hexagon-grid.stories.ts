import type { Meta, StoryObj } from '@storybook/angular';
import { action } from '@storybook/addon-actions';

import { HexagonGridComponent, HexNode } from '../hexagon-grid.component';
import {
  nodes12,
  nodes48,
  nodes120,
  nodesAllHigh,
  nodesAllUnseen,
  nodesLearnerProgress,
} from './hexagon-grid.testdata';

const meta: Meta<HexagonGridComponent> = {
  title: 'Components/Hexagon Grid',
  component: HexagonGridComponent,
  parameters: {
    docs: {
      description: {
        component:
          'Canvas-rendered honeycomb grid visualizing learner affinity across a curriculum graph. ' +
          'Each hex represents a concept node, colored by mastery level. ' +
          'Hover reveals title and affinity percentage; click emits the node.',
      },
    },
    backgrounds: { default: 'stone-mid' },
  },
  argTypes: {
    itemsPerRow: {
      control: { type: 'range', min: 4, max: 24, step: 1 },
      description: 'Max hexagons per row before wrapping to the next row.',
    },
    nodes: {
      control: false,
      description: 'Array of HexNode objects to render. Each node requires id, title, affinity (0–1), and affinityLevel.',
    },
    nodeClick: {
      action: 'nodeClick',
      description: 'Emitted when a hex is clicked. Payload is the HexNode.',
    },
  },
  args: {
    nodes: nodes48,
    itemsPerRow: 12,
    nodeClick: action('nodeClick') as (node: HexNode) => void,
  },
};

export default meta;
type Story = StoryObj<HexagonGridComponent>;

/** Default 48-node grid with mixed affinity levels. */
export const Default: Story = {};

/** Sparse grid — 12 nodes, good for early-stage content areas. */
export const Sparse: Story = {
  args: {
    nodes: nodes12,
    itemsPerRow: 6,
  },
};

/** Dense grid — 120 nodes for large curriculum sections. */
export const Dense: Story = {
  args: {
    nodes: nodes120,
    itemsPerRow: 16,
  },
};

/** All high affinity — fully mastered curriculum. */
export const FullyMastered: Story = {
  name: 'All High Affinity',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story: 'All nodes at high affinity — the target end-state for any content area. Nodes glow green.',
      },
    },
  },
  args: {
    nodes: nodesAllHigh,
    itemsPerRow: 10,
  },
};

/** New learner — all nodes unseen. */
export const NewLearner: Story = {
  name: 'All Unseen',
  parameters: {
    docs: {
      description: {
        story: 'Fresh learner state. Every node is unvisited (slate gray).',
      },
    },
  },
  args: {
    nodes: nodesAllUnseen,
    itemsPerRow: 10,
  },
};

/**
 * Gradient from unseen (bottom-left) to high affinity (top-right),
 * illustrating a learner mid-journey through a 60-concept arc.
 */
export const LearnerProgress: Story = {
  parameters: {
    backgrounds: { default: 'stone-dark' },
    docs: {
      description: {
        story: 'Gradient from unseen → low → medium → high, representing a learner mid-journey.',
      },
    },
  },
  args: {
    nodes: nodesLearnerProgress,
    itemsPerRow: 10,
  },
};

/** Narrow viewport — how the grid reflows on mobile. */
export const MobileLayout: Story = {
  parameters: {
    viewport: { defaultViewport: 'mobile' },
    docs: {
      description: {
        story: 'Grid on a 390px viewport. itemsPerRow clamps to fit the available width.',
      },
    },
  },
  args: {
    nodes: nodes48,
    itemsPerRow: 8,
  },
};
