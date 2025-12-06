/**
 * Human Network Service
 *
 * Manages human personas and relationships in the Lamad network.
 * Ported from add_human.py, add_relationship.py, import_humans.py
 */

import * as fs from 'fs';
import * as path from 'path';
import { ContentNode, ContentRelationship, ContentRelationshipType } from '../models/content-node.model';

/**
 * Human categories
 */
export type HumanCategory =
  | 'core-family'
  | 'workplace'
  | 'community'
  | 'affinity'
  | 'local-economy'
  | 'newcomer'
  | 'visitor'
  | 'red-team'
  | 'edge-case';

/**
 * Reach levels for human profiles
 */
export type ProfileReach = 'hidden' | 'network' | 'community' | 'public';

/**
 * Governance layers
 */
export type GovernanceLayer =
  | 'household'
  | 'neighborhood'
  | 'municipality'
  | 'county_regional'
  | 'state_provincial'
  | 'national'
  | 'global'
  | 'family'
  | 'workplace'
  | 'economy'
  | 'affinity'
  | 'personal'
  | 'network'
  | 'community';

/**
 * Relationship types with their typical governance layer and intimacy
 */
export const RELATIONSHIP_TYPES: Record<string, { layer: GovernanceLayer; typicalIntimacy: IntimacyLevel }> = {
  // Family layer
  spouse: { layer: 'family', typicalIntimacy: 'intimate' },
  parent: { layer: 'family', typicalIntimacy: 'intimate' },
  child: { layer: 'family', typicalIntimacy: 'intimate' },
  sibling: { layer: 'family', typicalIntimacy: 'intimate' },
  grandparent: { layer: 'family', typicalIntimacy: 'trusted' },
  grandchild: { layer: 'family', typicalIntimacy: 'trusted' },

  // Neighborhood layer
  neighbor: { layer: 'neighborhood', typicalIntimacy: 'connection' },
  local_friend: { layer: 'neighborhood', typicalIntimacy: 'trusted' },

  // Community layer
  community_member: { layer: 'community', typicalIntimacy: 'connection' },
  acquaintance: { layer: 'community', typicalIntimacy: 'recognition' },

  // Workplace layer
  coworker: { layer: 'workplace', typicalIntimacy: 'connection' },
  manager: { layer: 'workplace', typicalIntimacy: 'connection' },
  direct_report: { layer: 'workplace', typicalIntimacy: 'connection' },
  business_partner: { layer: 'economy', typicalIntimacy: 'trusted' },

  // Affinity layer
  mentor: { layer: 'affinity', typicalIntimacy: 'trusted' },
  mentee: { layer: 'affinity', typicalIntimacy: 'trusted' },
  congregation_member: { layer: 'affinity', typicalIntimacy: 'connection' },
  interest_group_member: { layer: 'affinity', typicalIntimacy: 'connection' },
  learning_partner: { layer: 'affinity', typicalIntimacy: 'connection' },

  // General
  friend: { layer: 'personal', typicalIntimacy: 'trusted' },
  network_connection: { layer: 'network', typicalIntimacy: 'connection' },
  other: { layer: 'community', typicalIntimacy: 'recognition' }
};

/**
 * Intimacy levels for relationships
 */
export type IntimacyLevel = 'intimate' | 'trusted' | 'connection' | 'recognition';

/**
 * Human location
 */
export interface HumanLocation {
  layer: GovernanceLayer;
  name: string;
  coordinates?: { lat: number; lon: number };
}

/**
 * Organization membership
 */
export interface OrganizationMembership {
  orgId: string;
  orgName: string;
  role: string;
}

/**
 * Human entry in humans.json
 */
export interface Human {
  id: string;
  displayName: string;
  bio: string;
  category: HumanCategory;
  profileReach: ProfileReach;
  location?: HumanLocation;
  affinities?: string[];
  organizations?: OrganizationMembership[];
  communities?: string[];
  isMinor?: boolean;
  guardianIds?: string[];
  isPseudonymous?: boolean;
  notes?: string;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Relationship entry in humans.json
 */
export interface HumanRelationship {
  sourceId: string;
  targetId: string;
  relationshipType: string;
  intimacy: IntimacyLevel;
  contextOrgId?: string;
  layer: GovernanceLayer;
  createdAt?: string;
}

/**
 * Humans data file structure
 */
export interface HumansData {
  humans: Human[];
  relationships: HumanRelationship[];
  generatedAt?: string;
}

/**
 * Create a new human entry
 */
export function createHuman(params: {
  id: string;
  displayName: string;
  bio: string;
  category: HumanCategory;
  profileReach?: ProfileReach;
  location?: { layer: GovernanceLayer; name: string };
  affinities?: string[];
  organizations?: Array<{ orgId: string; orgName: string; role: string }>;
  communities?: string[];
  isMinor?: boolean;
  guardianIds?: string[];
  isPseudonymous?: boolean;
  notes?: string;
}): Human {
  const now = new Date().toISOString();

  const human: Human = {
    id: params.id.startsWith('human-') ? params.id : `human-${params.id}`,
    displayName: params.displayName,
    bio: params.bio,
    category: params.category,
    profileReach: params.profileReach || 'community',
    createdAt: now,
    updatedAt: now
  };

  if (params.location) {
    human.location = {
      layer: params.location.layer,
      name: params.location.name
    };
  }

  if (params.affinities && params.affinities.length > 0) {
    human.affinities = params.affinities;
  }

  if (params.organizations && params.organizations.length > 0) {
    human.organizations = params.organizations;
  }

  if (params.communities && params.communities.length > 0) {
    human.communities = params.communities;
  }

  if (params.isMinor) {
    human.isMinor = true;
    human.guardianIds = params.guardianIds || [];
  }

  if (params.isPseudonymous) {
    human.isPseudonymous = true;
  }

  if (params.notes) {
    human.notes = params.notes;
  }

  return human;
}

/**
 * Normalize human ID to include prefix
 */
export function normalizeHumanId(id: string): string {
  return id.startsWith('human-') ? id : `human-${id}`;
}

/**
 * Create a relationship between humans
 */
export function createRelationship(params: {
  sourceId: string;
  targetId: string;
  relationshipType: string;
  intimacy?: IntimacyLevel;
  contextOrgId?: string;
}): HumanRelationship {
  const relType = RELATIONSHIP_TYPES[params.relationshipType] || RELATIONSHIP_TYPES.other;

  return {
    sourceId: normalizeHumanId(params.sourceId),
    targetId: normalizeHumanId(params.targetId),
    relationshipType: params.relationshipType,
    intimacy: params.intimacy || relType.typicalIntimacy,
    layer: relType.layer,
    contextOrgId: params.contextOrgId,
    createdAt: new Date().toISOString()
  };
}

/**
 * Load humans data from file
 */
export function loadHumansData(filePath: string): HumansData {
  if (!fs.existsSync(filePath)) {
    return { humans: [], relationships: [] };
  }

  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf-8')) as HumansData;
  } catch {
    return { humans: [], relationships: [] };
  }
}

/**
 * Save humans data to file
 */
export function saveHumansData(filePath: string, data: HumansData): void {
  data.generatedAt = new Date().toISOString();
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}

/**
 * Add a human to the data file
 */
export function addHumanToFile(filePath: string, human: Human): void {
  const data = loadHumansData(filePath);

  // Check for duplicates
  if (data.humans.some(h => h.id === human.id)) {
    throw new Error(`Human with ID ${human.id} already exists`);
  }

  data.humans.push(human);
  saveHumansData(filePath, data);
}

/**
 * Add a relationship to the data file
 */
export function addRelationshipToFile(filePath: string, relationship: HumanRelationship): void {
  const data = loadHumansData(filePath);

  // Validate that both humans exist
  const humanIds = new Set(data.humans.map(h => h.id));
  if (!humanIds.has(relationship.sourceId)) {
    throw new Error(`Source human not found: ${relationship.sourceId}`);
  }
  if (!humanIds.has(relationship.targetId)) {
    throw new Error(`Target human not found: ${relationship.targetId}`);
  }

  // Check for duplicate relationship
  const exists = data.relationships.some(
    r => r.sourceId === relationship.sourceId &&
         r.targetId === relationship.targetId &&
         r.relationshipType === relationship.relationshipType
  );
  if (exists) {
    throw new Error(`Relationship already exists`);
  }

  data.relationships.push(relationship);
  saveHumansData(filePath, data);
}

/**
 * Transform human to ContentNode
 */
export function humanToContentNode(human: Human): ContentNode {
  const now = new Date().toISOString();

  return {
    id: human.id,
    contentType: 'role',
    title: human.displayName,
    description: human.bio,
    content: human.bio,
    contentFormat: 'plaintext',
    tags: [
      'human',
      human.category,
      ...(human.affinities || []),
      ...(human.organizations?.map(o => o.orgId) || [])
    ],
    relatedNodeIds: [
      ...(human.guardianIds || []),
      ...(human.communities || [])
    ],
    metadata: {
      category: human.category,
      profileReach: human.profileReach,
      location: human.location,
      isMinor: human.isMinor,
      isPseudonymous: human.isPseudonymous
    },
    createdAt: human.createdAt || now,
    updatedAt: human.updatedAt || now
  };
}

/**
 * Transform relationship to ContentRelationship
 */
export function humanRelationshipToContentRelationship(
  rel: HumanRelationship
): ContentRelationship {
  return {
    id: `rel-${rel.sourceId}-${rel.targetId}-${rel.relationshipType}`,
    sourceNodeId: rel.sourceId,
    targetNodeId: rel.targetId,
    relationshipType: ContentRelationshipType.RELATES_TO,
    confidence: 1.0,
    inferenceSource: 'explicit'
  };
}

/**
 * Import result
 */
export interface HumanImportResult {
  humansImported: number;
  relationshipsImported: number;
  errors: string[];
}

/**
 * Import humans data to Lamad content nodes
 */
export async function importHumansToLamad(
  humansFilePath: string,
  outputDir: string
): Promise<HumanImportResult> {
  const result: HumanImportResult = {
    humansImported: 0,
    relationshipsImported: 0,
    errors: []
  };

  const data = loadHumansData(humansFilePath);
  const contentDir = path.join(outputDir, 'content');
  const graphDir = path.join(outputDir, 'graph');

  // Ensure directories exist
  if (!fs.existsSync(contentDir)) {
    fs.mkdirSync(contentDir, { recursive: true });
  }
  if (!fs.existsSync(graphDir)) {
    fs.mkdirSync(graphDir, { recursive: true });
  }

  // Transform and write humans
  for (const human of data.humans) {
    try {
      const node = humanToContentNode(human);
      const filePath = path.join(contentDir, `${node.id}.json`);
      fs.writeFileSync(filePath, JSON.stringify(node, null, 2), 'utf-8');
      result.humansImported++;
    } catch (err) {
      result.errors.push(`Human ${human.id}: ${err}`);
    }
  }

  // Transform relationships
  const relationships: ContentRelationship[] = [];
  for (const rel of data.relationships) {
    try {
      relationships.push(humanRelationshipToContentRelationship(rel));
      result.relationshipsImported++;
    } catch (err) {
      result.errors.push(`Relationship ${rel.sourceId} → ${rel.targetId}: ${err}`);
    }
  }

  // Load existing relationships and merge
  const relPath = path.join(graphDir, 'relationships.json');
  let existingRels: ContentRelationship[] = [];
  if (fs.existsSync(relPath)) {
    try {
      existingRels = JSON.parse(fs.readFileSync(relPath, 'utf-8'));
    } catch {
      existingRels = [];
    }
  }

  // Merge (avoiding duplicates)
  const relIds = new Set(existingRels.map(r => r.id));
  for (const rel of relationships) {
    if (!relIds.has(rel.id)) {
      existingRels.push(rel);
    }
  }

  fs.writeFileSync(relPath, JSON.stringify(existingRels, null, 2), 'utf-8');

  return result;
}

/**
 * List available relationship types
 */
export function listRelationshipTypes(): Array<{ type: string; layer: GovernanceLayer; intimacy: IntimacyLevel }> {
  return Object.entries(RELATIONSHIP_TYPES).map(([type, info]) => ({
    type,
    layer: info.layer,
    intimacy: info.typicalIntimacy
  }));
}

/**
 * List available human categories
 */
export function listHumanCategories(): HumanCategory[] {
  return [
    'core-family',
    'workplace',
    'community',
    'affinity',
    'local-economy',
    'newcomer',
    'visitor',
    'red-team',
    'edge-case'
  ];
}
