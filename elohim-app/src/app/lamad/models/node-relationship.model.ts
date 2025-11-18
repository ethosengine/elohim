/**
 * Represents relationships between nodes in the documentation graph
 */

export enum RelationshipType {
  /** An epic describes/specifies a feature */
  DESCRIBES = 'describes',

  /** A feature implements an epic */
  IMPLEMENTS = 'implements',

  /** A scenario belongs to a feature */
  BELONGS_TO = 'belongs_to',

  /** A scenario validates an epic's claims */
  VALIDATES = 'validates',

  /** General relation between nodes */
  RELATES_TO = 'relates_to',

  /** One epic references another */
  REFERENCES = 'references',

  /** Temporal or logical dependency */
  DEPENDS_ON = 'depends_on'
}

export interface NodeRelationship {
  /** Unique ID for this relationship */
  id: string;

  /** Type of relationship */
  type: RelationshipType;

  /** Source node ID */
  sourceId: string;

  /** Target node ID */
  targetId: string;

  /** Relationship strength/weight (0-1) */
  weight?: number;

  /** Description of the relationship */
  description?: string;

  /** Additional metadata */
  metadata?: Record<string, any>;

  /** Whether this relationship is bidirectional */
  bidirectional: boolean;
}

/**
 * Helper to create bidirectional relationships
 */
export function createBidirectionalRelationship(
  type: RelationshipType,
  nodeAId: string,
  nodeBId: string,
  description?: string
): NodeRelationship[] {
  const baseId = `${nodeAId}_${nodeBId}_${type}`;

  return [
    {
      id: `${baseId}_forward`,
      type,
      sourceId: nodeAId,
      targetId: nodeBId,
      bidirectional: true,
      description
    },
    {
      id: `${baseId}_reverse`,
      type: getReverseRelationshipType(type),
      sourceId: nodeBId,
      targetId: nodeAId,
      bidirectional: true,
      description
    }
  ];
}

/**
 * Get the reverse/inverse relationship type
 */
function getReverseRelationshipType(type: RelationshipType): RelationshipType {
  const reverseMap: Record<RelationshipType, RelationshipType> = {
    [RelationshipType.DESCRIBES]: RelationshipType.IMPLEMENTS,
    [RelationshipType.IMPLEMENTS]: RelationshipType.DESCRIBES,
    [RelationshipType.BELONGS_TO]: RelationshipType.RELATES_TO,
    [RelationshipType.VALIDATES]: RelationshipType.RELATES_TO,
    [RelationshipType.RELATES_TO]: RelationshipType.RELATES_TO,
    [RelationshipType.REFERENCES]: RelationshipType.REFERENCES,
    [RelationshipType.DEPENDS_ON]: RelationshipType.RELATES_TO
  };

  return reverseMap[type];
}
