/**
 * Hexagon Grid — Data Model
 *
 * Types for the multi-mode hexagon visualization component.
 * Supports four knowledge map modes:
 *
 *   domain  — Territory mastery heat map (Khan Academy "World of Math")
 *   person  — Love Map overlap (Gottman-inspired, red/blue/purple)
 *   network — P2P agent topology (humans in relationship space)
 *   bloom   — Bloom's taxonomy 8-tier with gate glow
 *
 * All new fields on HexNode are optional for backward compatibility.
 * Existing consumers passing { id, title, affinity, affinityLevel }
 * continue to work unchanged.
 */

// =============================================================================
// Mastery & Affinity
// =============================================================================

/**
 * Bloom's Taxonomy levels — mirrors app-level MasteryLevel
 * but defined locally so lamad-ui stays self-contained (no @app/* imports).
 */
export type HexMasteryLevel =
  | 'not_started'
  | 'seen'
  | 'remember'
  | 'understand'
  | 'apply' // ← Attestation gate (participation privileges unlock here)
  | 'analyze'
  | 'evaluate'
  | 'create';

/** Simplified 4-tier affinity for backward compatibility. */
export type AffinityLevel = 'unseen' | 'low' | 'medium' | 'high';

/** Ownership in a love map / overlap visualization. */
export type HexOwner = 'self' | 'other' | 'shared';

// =============================================================================
// Node
// =============================================================================

export interface HexNode {
  id: string;
  title: string;
  affinity: number; // 0–1 scalar
  affinityLevel: AffinityLevel; // 4-tier bucket (existing)

  // Canvas-computed position (set by layout algorithms)
  x?: number;
  y?: number;

  // --- Phase 1: Rich mastery ---

  /** Bloom's 8-level mastery. When set, overrides affinityLevel for coloring. */
  masteryLevel?: HexMasteryLevel;

  /** 0–1 freshness decay. 1 = fresh, 0 = completely stale. Affects opacity. */
  freshness?: number;

  /** Category label for grouping (e.g., "Fractions", "Workplace Safety"). */
  category?: string;

  /** Subtitle shown in tooltip below title. */
  subtitle?: string;

  // --- Phase 1: Love map / overlap ---

  /** Who "owns" this knowledge node in a love map. */
  owner?: HexOwner;

  /** 0–1 overlap intensity for shared nodes (purple saturation). */
  overlapScore?: number;

  /**
   * Source journey ID(s) — which journey(s) this node was encountered in.
   * Used for cross-journey overlap visualization.
   */
  sourceJourneyIds?: string[];

  /** Label for the source (e.g., "Adam", "Arithmetic", "Team Alpha"). */
  sourceLabel?: string;

  // --- Phase 1: Grouping ---

  /** Arbitrary group key for network/cluster coloring. */
  group?: string;

  /** Political compass position (for compass overlay). */
  compassX?: number; // -1 (left) to +1 (right)
  compassY?: number; // -1 (libertarian) to +1 (authoritarian)

  // --- Phase 2: Animation & interaction state ---

  isFocused?: boolean;
  isConnected?: boolean;
  opacity?: number;
  targetX?: number;
  targetY?: number;
  scale?: number;
  targetScale?: number;
  targetOpacity?: number;
  originalX?: number;
  originalY?: number;
  lockState?: HexLockState;
}

// =============================================================================
// Edge / Connection
// =============================================================================

/** Visual style categories for edges between hexagons. */
export type HexEdgeType =
  | 'contains' // hierarchical parent-child
  | 'depends_on' // prerequisite
  | 'follows' // sequence / suggested next
  | 'relates_to' // thematic association
  | 'requires' // hard prerequisite
  | 'shared_concept' // cross-journey overlap link
  | 'social' // interpersonal bond
  | 'teaches' // who-taught-whom in love maps
  | 'opposes' // ideological tension (political compass)
  | 'custom';

export interface HexEdge {
  id: string;
  sourceId: string;
  targetId: string;
  edgeType: HexEdgeType;

  /** 0–1 maps to thickness / opacity. Default 0.7. */
  confidence?: number;

  /** Show arrowhead at target end? */
  directed?: boolean;

  /** Optional label drawn at midpoint. */
  label?: string;

  /** Override the palette-derived edge color. */
  color?: string;
}

// =============================================================================
// Graph Data Container
// =============================================================================

/** Bundles nodes + edges for the component. */
export interface HexGraphData {
  nodes: HexNode[];
  edges: HexEdge[];
}

// =============================================================================
// Color & Layout Configuration
// =============================================================================

/** Which color palette to use. */
export type HexColorMode = 'domain' | 'bloom' | 'person' | 'network';

/** Which layout algorithm to use. */
export type HexLayoutMode = 'grid' | 'dag' | 'radial' | 'scatter' | 'tree';

/** Computed lock state for skill-tree visualization. */
export type HexLockState = 'locked' | 'available' | 'started' | 'mastered';

/** Focus state tracking for expand/collapse interaction. */
export interface HexFocusState {
  focusedNodeId: string | null;
  connectedNodeIds: Set<string>;
  activeEdgeIds: Set<string>;
  prerequisiteChainIds: Set<string>;
}

// =============================================================================
// Bloom Level Metadata (for palettes)
// =============================================================================

/** Numeric order for Bloom's levels (for gradient calculations). */
export const BLOOM_ORDER: Record<HexMasteryLevel, number> = {
  not_started: 0,
  seen: 1,
  remember: 2,
  understand: 3,
  apply: 4,
  analyze: 5,
  evaluate: 6,
  create: 7,
};
