/**
 * Represents a user's affinity (relationship strength) to content nodes.
 *
 * Affinity is a scalar value from 0.0 to 1.0 that captures the strength
 * of the user's connection to a piece of content. The interpretation of
 * this value is domain-specific:
 * - Documentation: familiarity/understanding
 * - Learning: mastery/competence
 * - Social: engagement/interest
 *
 * This abstraction allows the model to be extended into any domain while
 * maintaining a consistent progress tracking mechanism.
 */

export interface UserAffinity {
  /** Hardcoded user ID for prototyping */
  userId: string;

  /** Map of node IDs to affinity values (0.0 to 1.0) */
  affinity: { [nodeId: string]: number };

  /** Last updated timestamp */
  lastUpdated: Date;
}

export interface AffinityStats {
  /** Total number of nodes tracked */
  totalNodes: number;

  /** Number of nodes with affinity > 0 */
  engagedNodes: number;

  /** Average affinity across all nodes */
  averageAffinity: number;

  /** Distribution of affinity values */
  distribution: {
    /** 0.0 - no affinity */
    unseen: number;
    /** 0.01 - 0.33 - low affinity */
    low: number;
    /** 0.34 - 0.66 - medium affinity */
    medium: number;
    /** 0.67 - 1.0 - high affinity */
    high: number;
  };

  /** Stats by content category */
  byCategory: Map<string, CategoryAffinityStats>;

  /** Stats by content type */
  byType: Map<string, TypeAffinityStats>;
}

export interface CategoryAffinityStats {
  category: string;
  nodeCount: number;
  averageAffinity: number;
  engagedCount: number;
}

export interface TypeAffinityStats {
  type: string;
  nodeCount: number;
  averageAffinity: number;
  engagedCount: number;
}

/**
 * Event emitted when affinity changes
 */
export interface AffinityChangeEvent {
  nodeId: string;
  oldValue: number;
  newValue: number;
  timestamp: Date;
}
