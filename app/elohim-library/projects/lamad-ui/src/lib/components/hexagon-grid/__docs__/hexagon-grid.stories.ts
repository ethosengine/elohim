// eslint-disable-next-line import/no-extraneous-dependencies
import { action } from '@storybook/addon-actions';

import { HexagonGridComponent } from '../hexagon-grid.component';
import { HexNode } from '../hexagon-grid.model';

const DARK_BG = DARK_BG;

import {
  makeAdamEveLoveMap,
  makeAstronautGoal,
  makeArithmeticTerritory,
  makeBloomShowcase,
  makeBloomStaircase,
  makeComplianceTraining,
  makeCrossJourneyOverlap,
  makeLoveMap,
  makeP2PNetwork,
  makePoliticalCompass,
  makePoliticalFaultLines,
  nodes12,
  nodes48,
  nodes120,
  nodesAllHigh,
  nodesAllUnseen,
  nodesLearnerProgress,
} from './hexagon-grid.testdata';

import type { Meta, StoryObj } from '@storybook/angular';

// =============================================================================
// Meta
// =============================================================================

const meta: Meta<HexagonGridComponent> = {
  title: 'Components/Hexagon Grid',
  component: HexagonGridComponent,
  parameters: {
    docs: {
      description: {
        component:
          'Canvas-rendered hexagon visualization with 4 layout modes (grid, dag, radial, scatter), ' +
          '4 color palettes, 5 connection styles, and category tessellation. ' +
          'Each story below explores a radically different visual concept.',
      },
    },
    backgrounds: { default: 'stone-mid' },
  },
  argTypes: {
    itemsPerRow: {
      control: { type: 'range', min: 4, max: 24, step: 1 },
      description: 'Max hexagons per row before wrapping.',
    },
    colorMode: {
      control: { type: 'select' },
      options: ['domain', 'bloom', 'person', 'network'],
      description:
        'Color palette: domain (4-tier), bloom (8-tier), person (love map), network (P2P).',
    },
    layoutMode: {
      control: { type: 'select' },
      options: ['grid', 'dag', 'radial', 'scatter', 'tree'],
      description:
        'Layout algorithm: grid (honeycomb), dag (skill tree), radial (center-out), scatter (compass), tree (organic diagonal).',
    },
    showFreshness: {
      control: 'boolean',
      description: 'Apply freshness decay overlay.',
    },
    showEdges: {
      control: 'boolean',
      description: 'Draw connection edges between nodes.',
    },
    showLockState: {
      control: 'boolean',
      description: 'Show locked/available/mastered visual state.',
    },
    nodes: { control: false },
    edges: { control: false },
    nodeClick: {
      action: 'nodeClick',
      description: 'Emitted when a hex is clicked.',
    },
  },
  args: {
    nodes: nodes48,
    itemsPerRow: 12,
    colorMode: 'domain',
    layoutMode: 'grid',
    showFreshness: false,
    showEdges: false,
    showLockState: false,
    edges: [],
    nodeClick: action('nodeClick') as (node: HexNode) => void,
  },
};

export default meta;
type Story = StoryObj<HexagonGridComponent>;

// =============================================================================
// Legacy Stories (backward compatible, grid layout)
// =============================================================================

export const Default: Story = {};

export const Sparse: Story = {
  args: { nodes: nodes12, itemsPerRow: 6 },
};

export const Dense: Story = {
  args: { nodes: nodes120, itemsPerRow: 16 },
};

export const FullyMastered: Story = {
  name: 'All High Affinity',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: { nodes: nodesAllHigh, itemsPerRow: 10 },
};

export const NewLearner: Story = {
  name: 'All Unseen',
  args: { nodes: nodesAllUnseen, itemsPerRow: 10 },
};

export const LearnerProgress: Story = {
  parameters: { backgrounds: { default: DARK_BG } },
  args: { nodes: nodesLearnerProgress, itemsPerRow: 10 },
};

export const MobileLayout: Story = {
  parameters: { viewport: { defaultViewport: 'mobile' } },
  args: { nodes: nodes48, itemsPerRow: 8 },
};

const staircaseData = makeBloomStaircase();

/**
 * A vertical staircase. Five category bands (Number Sense → Geometry) stack
 * as a skill tree. A horizontal gate line divides "passive knowledge" (remember,
 * understand) from "active capability" (apply+). Lock icons on unreached concepts,
 * green pulse on available ones. Reads like a curriculum progression map.
 */
export const TheStaircase: Story = {
  name: "The Staircase — Bloom's Taxonomy",
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Curriculum progression as a skill tree. Gate line divides passive knowledge (remember/understand) ' +
          'from active capability (apply+). Locked concepts show padlock icons. Available concepts pulse green. ' +
          'The shape communicates "where you are and what\'s next."',
      },
    },
  },
  args: {
    nodes: staircaseData.nodes,
    edges: staircaseData.edges,
    colorMode: 'bloom',
    layoutMode: 'dag',
    showEdges: true,
    showLockState: true,
    itemsPerRow: 10,
  },
};

export const BloomProgression: Story = {
  name: "Bloom's Taxonomy — Color Reference",
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: makeBloomShowcase(),
    itemsPerRow: 6,
    colorMode: 'bloom',
  },
};

// =============================================================================
// "The Waterfall" — Arithmetic Skill Tree (DAG)
// =============================================================================

const arithmeticData = makeArithmeticTerritory();

/**
 * A vertical cascade. Five category bands stack top→bottom like geological
 * strata. Tight hex chains between layers. Foundation concepts at top (bright),
 * advanced concepts at bottom (dark/locked). Reads like a cliff face.
 */
export const TheWaterfall: Story = {
  name: 'The Waterfall — Arithmetic Skill Tree',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Vertical prerequisite cascade. Counting→Addition→Multiplication→Fractions flows top→bottom. ' +
          'Tight bridge connections chain layers. Category bands form geological strata. ' +
          'The top glows bright (mastered), the bottom is dark frontier.',
      },
    },
  },
  args: {
    nodes: arithmeticData.nodes,
    edges: arithmeticData.edges,
    colorMode: 'bloom',
    layoutMode: 'dag',
    showEdges: true,
    itemsPerRow: 13,
  },
};

// =============================================================================
// "The Solar System" — P2P Network (Radial)
// =============================================================================

const networkData = makeP2PNetwork();

/**
 * A starburst. "You" sits at center, 1.5x size, glowing. Thick tapered spokes
 * radiate outward. Family in tight inner ring, friends next, acquaintances
 * at the edge barely visible. Each ring tessellates into arc clusters.
 */
export const TheSolarSystem: Story = {
  name: 'The Solar System — P2P Network',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Radial starburst. "You" at center (1.5x), tapered spokes to inner rings of family/friends. ' +
          'Outer rings fade. Same-group nodes form arc clusters. ' +
          'Distance from center = relationship intimacy.',
      },
    },
  },
  args: {
    nodes: networkData.nodes,
    edges: networkData.edges,
    colorMode: 'network',
    layoutMode: 'radial',
    showEdges: true,
    itemsPerRow: 6,
  },
};

// =============================================================================
// "The Fault Lines" — Political Polarization (DAG bilateral)
// =============================================================================

const faultLinesData = makePoliticalFaultLines();

/**
 * Bilateral policy tension. 6 policy domains arranged as columns. Left positions
 * (red) on one side, right positions (blue) on the other. Red tension lines
 * cross between opposed positions — making polarization visible as physical
 * distance and crossing lines. Common ground nodes (purple) bridge the center.
 */
export const TheFaultLines: Story = {
  name: 'The Fault Lines — Political Polarization',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Bilateral DAG with 6 policy domains. Civic Left (red) vs Civic Right (blue). ' +
          'Red tension lines cross between opposed positions with a subtle glow. ' +
          'Common ground (purple) bridges the center. Polarization as physical distance.',
      },
    },
  },
  args: {
    nodes: faultLinesData.nodes,
    edges: faultLinesData.edges,
    colorMode: 'person',
    layoutMode: 'grid',
    showEdges: true,
    itemsPerRow: 12,
  },
};

// =============================================================================
// "The Glacier" — Compliance Decay at 18 months (Grid + Freshness)
// =============================================================================

const complianceCritical = makeComplianceTraining(18);

/**
 * Dense tessellated islands. Category clusters packed tight, each with a faint
 * hull. But nearly monochrome — at 18 months, most hexes are ghostly pale.
 * Only deeply "applied" modules still glow like embers in ash.
 * The absence of color IS the message: knowledge has decayed.
 */
export const TheGlacier: Story = {
  name: 'The Glacier — Compliance Decay (18 months)',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Almost monochrome. 18 months since training — most modules ghostly. ' +
          'Category clusters as tessellated islands. Only deeply "applied" knowledge ' +
          'still glows. The absence of color communicates decay.',
      },
    },
  },
  args: {
    nodes: complianceCritical.nodes,
    colorMode: 'bloom',
    layoutMode: 'grid',
    showFreshness: true,
    itemsPerRow: 10,
  },
};

// =============================================================================
// "The Bridge" — Cross-Journey Overlap (DAG)
// =============================================================================

const overlapData = makeCrossJourneyOverlap();

/**
 * Three columns. Left: Arithmetic (red, mastered). Right: Algebra (blue, frontier).
 * Center: shared bridge concepts (purple). Thick gradient arcs sweep from red
 * through purple to blue. Bridge concepts are 1.3x scale. Bilateral symmetry
 * like an architectural blueprint.
 */
export const TheBridge: Story = {
  name: 'The Bridge — Cross-Journey Overlap',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Three-column bridge: Arithmetic (red, mastered) → Shared concepts (purple, bridge) → ' +
          'Algebra (blue, frontier). Gradient-colored bridge arcs sweep between journeys. ' +
          'Territory-first insight: completing one course fills another.',
      },
    },
  },
  args: {
    nodes: overlapData.nodes,
    edges: overlapData.edges,
    colorMode: 'person',
    layoutMode: 'dag',
    showEdges: true,
    itemsPerRow: 8,
  },
};

// =============================================================================
// "The Mountain Range" — Astronaut Goal (DAG)
// =============================================================================

const astronautData = makeAstronautGoal();

/**
 * A towering 7-layer skill tree. Mathematics at summit (few lit nodes),
 * cascading down to Space Science at base (all dark/locked). The sheer
 * height communicates "this is an enormous undertaking." Cross-domain
 * prerequisite arcs bridge between column spines.
 */
export const TheMountainRange: Story = {
  name: 'The Mountain Range — Astronaut Goal',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Towering 7-layer DAG. Mathematics (lit) at top, Space Science (dark) at bottom. ' +
          'Cross-domain arcs bridge between spines. Mostly dark frontier with few lit peaks. ' +
          'Communicates aspiration and distance to a complex goal.',
      },
    },
  },
  args: {
    nodes: astronautData.nodes,
    edges: astronautData.edges,
    colorMode: 'bloom',
    layoutMode: 'dag',
    showEdges: true,
    showLockState: true,
    itemsPerRow: 9,
  },
};

// =============================================================================
// "The Love Map" — River & Sky Relationship Intimacy (Radial)
// =============================================================================

const loveMapData = makeLoveMap();

/**
 * A relationship intimacy map. "River & Sky" at center, surrounded by
 * life domain clusters: Finances, Career, Health, Parenting, Intimacy,
 * Shared Dreams. Teaches edges flow between partners through shared concepts.
 * Each domain tells a story of who contributes what to the relationship.
 */
export const TheLoveMap: Story = {
  name: 'The Love Map — River & Sky',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Relationship intimacy across 6 life domains. River (red) and Sky (blue) each ' +
          'contribute different strengths. Shared concepts (purple) at center. ' +
          'Teaches arcs show knowledge flowing between partners.',
      },
    },
  },
  args: {
    nodes: loveMapData.nodes,
    edges: loveMapData.edges,
    colorMode: 'person',
    layoutMode: 'radial',
    showEdges: true,
    itemsPerRow: 8,
  },
};

// =============================================================================
// "The Unit Map" — Compliance Fresh (Grid + Categories)
// =============================================================================

const complianceFresh = makeComplianceTraining(2);

/**
 * Khan Academy-style unit rows. Horizontal bands separated by category headers
 * and horizontal rules. Hexes pack tight within each unit — forming solid colored
 * tiles. Each category has a background wash. Vivid colors (2-month fresh).
 * Reads like a syllabus — structured, organized, institutional.
 *
 * Compare with "The Glacier" (same data, 18 months) to see freshness decay.
 */
export const TheUnitMap: Story = {
  name: 'The Unit Map — Compliance (Fresh)',
  parameters: {
    backgrounds: { default: DARK_BG },
    docs: {
      description: {
        story:
          'Khan-style unit rows with category headers and tight tessellation. ' +
          'All modules vivid (2 months fresh). Each category forms a solid colored tile cluster. ' +
          'Compare with "The Glacier" to see 18-month decay on the same data.',
      },
    },
  },
  args: {
    nodes: complianceFresh.nodes,
    colorMode: 'bloom',
    layoutMode: 'grid',
    showFreshness: true,
    itemsPerRow: 10,
  },
};

// =============================================================================
// "The Unit Map — Grid (Original)" — Reference copy for comparison
// =============================================================================

export const TheUnitMapGrid: Story = {
  name: 'The Unit Map — Grid (Original)',
  parameters: {
    backgrounds: { default: DARK_BG },
    docs: {
      description: {
        story:
          'Khan-style unit rows with category headers and tight tessellation. ' +
          'Reference copy of the original grid layout for side-by-side comparison with tree variant.',
      },
    },
  },
  args: {
    nodes: complianceFresh.nodes,
    colorMode: 'bloom',
    layoutMode: 'grid',
    showFreshness: true,
    itemsPerRow: 10,
  },
};

// =============================================================================
// "The Unit Map — Tree (Organic)" — Diagonal hex-grid walk per category
// =============================================================================

/**
 * Organic tree growth. Same compliance data as TheUnitMap, but each category
 * grows from a root node at top-left, tessellating diagonally at 50% hex scale.
 * A seeded PRNG gives each category tree a unique but deterministic shape.
 */
export const TheUnitMapTree: Story = {
  name: 'The Unit Map — Tree (Organic)',
  parameters: {
    backgrounds: { default: DARK_BG },
    docs: {
      description: {
        story:
          'Organic diagonal growth. Each category grows from a root at top-left using 50%-scale hexes. ' +
          'Seeded PRNG makes each tree unique but deterministic. Compare with Grid (Original) ' +
          'to see how the same data feels as an organism vs a spreadsheet.',
      },
    },
  },
  args: {
    nodes: complianceFresh.nodes,
    colorMode: 'bloom',
    layoutMode: 'tree',
    showFreshness: true,
    itemsPerRow: 10,
  },
};

// =============================================================================
// "Layout Switcher" — Interactive Comparison
// =============================================================================

/**
 * Same Astronaut Goal data — switch layoutMode via Storybook controls to see
 * how the same data feels in grid vs dag vs radial vs scatter.
 */
export const LayoutSwitcher: Story = {
  name: 'Layout Switcher (use controls)',
  parameters: {
    backgrounds: { default: 'obsidian' },
    docs: {
      description: {
        story:
          'Switch layoutMode via controls: grid (flat honeycomb), dag (skill tree), ' +
          'radial (center-out), scatter (compass). Same astronaut data, different spatial arrangements.',
      },
    },
  },
  args: {
    nodes: astronautData.nodes,
    edges: astronautData.edges,
    colorMode: 'bloom',
    layoutMode: 'dag',
    showEdges: true,
    itemsPerRow: 9,
  },
};

// =============================================================================
// Palette Comparison (from Phase 1, kept)
// =============================================================================

export const PaletteComparison: Story = {
  name: 'Palette Comparison (use controls)',
  args: {
    nodes: nodes48.map((n, i) => ({
      ...n,
      masteryLevel: (
        [
          'not_started',
          'seen',
          'remember',
          'understand',
          'apply',
          'analyze',
          'evaluate',
          'create',
        ] as const
      )[i % 8],
      owner: (['self', 'other', 'shared'] as const)[i % 3],
      overlapScore: i % 3 === 2 ? 0.5 + (i / 48) * 0.5 : undefined,
      group: ['alpha', 'beta', 'gamma', 'delta'][i % 4],
    })),
    itemsPerRow: 12,
    colorMode: 'bloom',
  },
};

// =============================================================================
// Phase 1 stories kept for reference/backward compatibility
// =============================================================================

const complianceStale = makeComplianceTraining(8);

export const ComplianceFresh: Story = {
  name: 'Compliance — Fresh (2 months)',
  parameters: { backgrounds: { default: DARK_BG } },
  args: { nodes: complianceFresh.nodes, itemsPerRow: 10, colorMode: 'bloom', showFreshness: true },
};

export const ComplianceStale: Story = {
  name: 'Compliance — Stale (8 months)',
  parameters: { backgrounds: { default: DARK_BG } },
  args: { nodes: complianceStale.nodes, itemsPerRow: 10, colorMode: 'bloom', showFreshness: true },
};

export const ComplianceCritical: Story = {
  name: 'Compliance — Critical (18 months)',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: complianceCritical.nodes,
    itemsPerRow: 10,
    colorMode: 'bloom',
    showFreshness: true,
  },
};

export const ArithmeticTerritory: Story = {
  name: 'Arithmetic — Territory Heat Map',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: { nodes: arithmeticData.nodes, itemsPerRow: 13, colorMode: 'bloom' },
};

export const ArithmeticWithPrerequisites: Story = {
  name: 'Arithmetic — With Prerequisites',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: arithmeticData.nodes,
    edges: arithmeticData.edges,
    itemsPerRow: 13,
    colorMode: 'bloom',
    showEdges: true,
  },
};

const adamEveData = makeAdamEveLoveMap();

export const LoveMapAdamEve: Story = {
  name: 'Love Map — Adam & Eve',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: { nodes: adamEveData.nodes, itemsPerRow: 8, colorMode: 'person' },
};

export const LoveMapAdamEveWithEdges: Story = {
  name: 'Love Map — Adam & Eve (Teaching Edges)',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: adamEveData.nodes,
    edges: adamEveData.edges,
    itemsPerRow: 8,
    colorMode: 'person',
    showEdges: true,
  },
};

const politicalData = makePoliticalCompass('Libertarian Left', 'Libertarian Right');

export const PoliticalCompassScatter: Story = {
  name: 'Political Compass — Scatter (Reference)',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: politicalData.nodes,
    edges: politicalData.edges,
    itemsPerRow: 9,
    colorMode: 'person',
    layoutMode: 'scatter',
    showEdges: true,
  },
};

export const CrossJourneyOverlap: Story = {
  name: 'Cross-Journey — Arithmetic → Algebra',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: overlapData.nodes,
    edges: overlapData.edges,
    itemsPerRow: 8,
    colorMode: 'person',
    showEdges: true,
  },
};

export const AstronautGoal: Story = {
  name: 'Attestation Goal — Astronaut',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: astronautData.nodes,
    edges: astronautData.edges,
    itemsPerRow: 9,
    colorMode: 'bloom',
    showEdges: true,
  },
};

export const P2PNetwork: Story = {
  name: 'P2P Network — Relationships',
  parameters: { backgrounds: { default: 'obsidian' } },
  args: {
    nodes: networkData.nodes,
    edges: networkData.edges,
    itemsPerRow: 6,
    colorMode: 'network',
    showEdges: true,
  },
};
