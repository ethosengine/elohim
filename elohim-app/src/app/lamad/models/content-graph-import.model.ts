/**
 * Content Graph Import Data Structure
 *
 * TypeScript interfaces for the structured content node data
 * generated from the /docs directory extraction process.
 *
 * This file defines the format for systematic import into the
 * Lamad learning platform node graph database.
 */

import { ContentNode, ContentMetadata } from './content-node.model';

/**
 * Main data structure for content graph import
 */
export interface ContentGraphImport {
  version: string;
  generatedAt: string;
  stats: ContentGraphStats;
  taxonomy: ContentTaxonomy;
  suggestedPaths: SuggestedLearningPath[];
  nodes: ContentNode[];
}

/**
 * Statistics about the content graph
 */
export interface ContentGraphStats {
  totalNodes: number;
  byType: Record<string, number>;
  byEpic: Record<string, number>;
}

/**
 * Taxonomy structure for navigation and categorization
 */
export interface ContentTaxonomy {
  epics: Record<string, EpicTaxonomy>;
  layers: Record<string, Record<string, number>>;
  contentTypes: Record<string, number>;
  personas: Record<string, PersonaTaxonomy>;
  total: number;
}

/**
 * Epic-level taxonomy information
 */
export interface EpicTaxonomy {
  name: string;
  count: number;
  personas: string[];
  contentTypes: string[];
}

/**
 * Persona-level taxonomy information
 */
export interface PersonaTaxonomy {
  count: number;
  epics: string[];
}

/**
 * Suggested learning path for users
 */
export interface SuggestedLearningPath {
  id: string;
  title: string;
  description: string;
  targetSubject: string;
  path: string[]; // Array of ContentNode IDs in sequence
  requiredAttestations?: string[]; // Optional: attestations needed to access
  estimatedDuration?: string; // Optional: estimated time to complete
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
}

/**
 * Extended metadata for imported content nodes
 */
export interface ImportedContentMetadata extends ContentMetadata {
  epic?: string;
  persona?: string;
  referenceType?: string;
  layer?: string;
  layerType?: 'geographic_political' | 'functional';
  priority?: number;
  sourcePath?: string;
}

/**
 * Node types used in the content graph
 */
export enum ContentNodeType {
  EPIC = 'epic',
  PERSONA = 'persona',
  SCENARIO = 'scenario',
  ARTICLE = 'article',
  BOOK = 'book',
  VIDEO = 'video',
  AUDIO = 'audio',
  DOCUMENT = 'document',
  ORGANIZATION = 'organization',
  ROOT_DOCUMENT = 'root-document'
}

/**
 * Epic identifiers
 */
export enum EpicIdentifier {
  AUTONOMOUS_ENTITY = 'autonomous_entity',
  GOVERNANCE = 'governance',
  GOVERNANCE_LAYERS = 'governance_layers',
  PUBLIC_OBSERVER = 'public_observer',
  SOCIAL_MEDIUM = 'social_medium',
  VALUE_SCANNER = 'value_scanner'
}

/**
 * Epic display names
 */
export const EPIC_NAMES: Record<EpicIdentifier, string> = {
  [EpicIdentifier.AUTONOMOUS_ENTITY]: 'Autonomous Entity',
  [EpicIdentifier.GOVERNANCE]: 'Governance',
  [EpicIdentifier.GOVERNANCE_LAYERS]: 'Governance Layers',
  [EpicIdentifier.PUBLIC_OBSERVER]: 'Public Observer',
  [EpicIdentifier.SOCIAL_MEDIUM]: 'Social Medium',
  [EpicIdentifier.VALUE_SCANNER]: 'Value Scanner'
};

/**
 * Governance layer types
 */
export interface GovernanceLayer {
  type: 'geographic_political' | 'functional';
  name: string;
  level: string; // e.g., 'individual', 'family', 'community', 'municipality', etc.
}

/**
 * Geographic/Political governance layers (from smallest to largest scale)
 */
export enum GeographicPoliticalLayer {
  INDIVIDUAL = 'individual',
  FAMILY = 'family',
  NEIGHBORHOOD = 'neighborhood',
  COMMUNITY = 'community',
  DISTRICT = 'district',
  MUNICIPALITY = 'municipality',
  COUNTY_REGIONAL = 'county_regional',
  PROVINCIAL_STATE = 'provincial_state',
  NATION_STATE = 'nation_state',
  CONTINENTAL = 'continental',
  GLOBAL = 'global'
}

/**
 * Functional governance layers (cross-cutting concerns)
 */
export enum FunctionalLayer {
  ECOLOGICAL_BIOREGIONAL = 'ecological_bioregional',
  EDUCATIONAL = 'educational',
  WORKPLACE_ORGANIZATIONAL = 'workplace_organizational',
  INDUSTRY_SECTOR = 'industry_sector',
  CULTURAL_LINGUISTIC = 'cultural_linguistic',
  AFFINITY_NETWORK = 'affinity_network'
}

/**
 * Helper functions for working with imported content
 */
export class ContentGraphImportHelper {
  /**
   * Load and parse content graph import JSON
   */
  static async loadFromJson(jsonPath: string): Promise<ContentGraphImport> {
    const response = await fetch(jsonPath);
    if (!response.ok) {
      throw new Error(`Failed to load content graph from ${jsonPath}`);
    }
    return response.json();
  }

  /**
   * Get all nodes for a specific epic
   */
  static getNodesForEpic(data: ContentGraphImport, epic: EpicIdentifier): ContentNode[] {
    return data.nodes.filter(node => {
      const metadata = node.metadata as ImportedContentMetadata;
      return metadata.epic === epic;
    });
  }

  /**
   * Get all nodes for a specific persona
   */
  static getNodesForPersona(data: ContentGraphImport, epic: EpicIdentifier, persona: string): ContentNode[] {
    return data.nodes.filter(node => {
      const metadata = node.metadata as ImportedContentMetadata;
      return metadata.epic === epic && metadata.persona === persona;
    });
  }

  /**
   * Get all nodes of a specific type
   */
  static getNodesByType(data: ContentGraphImport, type: ContentNodeType): ContentNode[] {
    return data.nodes.filter(node => node.contentType === type);
  }

  /**
   * Get root-level documents (manifesto, epics, etc.)
   */
  static getRootDocuments(data: ContentGraphImport): ContentNode[] {
    return this.getNodesByType(data, ContentNodeType.ROOT_DOCUMENT);
  }

  /**
   * Get all scenarios for a persona
   */
  static getScenarios(data: ContentGraphImport, epic?: EpicIdentifier, persona?: string): ContentNode[] {
    const scenarios = this.getNodesByType(data, ContentNodeType.SCENARIO);

    if (!epic && !persona) {
      return scenarios;
    }

    return scenarios.filter(node => {
      const metadata = node.metadata as ImportedContentMetadata;
      if (epic && metadata.epic !== epic) return false;
      if (persona && metadata.persona !== persona) return false;
      return true;
    });
  }

  /**
   * Get reference materials (books, articles, videos, organizations, etc.)
   */
  static getReferenceMaterials(data: ContentGraphImport): ContentNode[] {
    const referenceTypes = [
      ContentNodeType.ORGANIZATION,
      ContentNodeType.BOOK,
      ContentNodeType.VIDEO,
      ContentNodeType.AUDIO,
      ContentNodeType.ARTICLE,
      ContentNodeType.DOCUMENT
    ];

    return data.nodes.filter(node =>
      referenceTypes.includes(node.contentType as ContentNodeType)
    );
  }

  /**
   * Get nodes by governance layer
   */
  static getNodesByLayer(
    data: ContentGraphImport,
    layerType: 'geographic_political' | 'functional',
    layer: string
  ): ContentNode[] {
    return data.nodes.filter(node => {
      const metadata = node.metadata as ImportedContentMetadata;
      return metadata.layerType === layerType && metadata.layer === layer;
    });
  }

  /**
   * Get suggested path by ID
   */
  static getSuggestedPath(data: ContentGraphImport, pathId: string): SuggestedLearningPath | undefined {
    return data.suggestedPaths.find(path => path.id === pathId);
  }

  /**
   * Get nodes in a suggested path
   */
  static getNodesInPath(data: ContentGraphImport, pathId: string): ContentNode[] {
    const path = this.getSuggestedPath(data, pathId);
    if (!path) return [];

    const nodeMap = new Map(data.nodes.map(n => [n.id, n]));
    return path.path.map(id => nodeMap.get(id)).filter(Boolean) as ContentNode[];
  }

  /**
   * Search nodes by text (title, description, tags)
   */
  static searchNodes(data: ContentGraphImport, query: string): ContentNode[] {
    const lowerQuery = query.toLowerCase();
    return data.nodes.filter(node =>
      node.title.toLowerCase().includes(lowerQuery) ||
      node.description.toLowerCase().includes(lowerQuery) ||
      node.tags.some(tag => tag.toLowerCase().includes(lowerQuery))
    );
  }

  /**
   * Get related nodes for a given node ID
   */
  static getRelatedNodes(data: ContentGraphImport, nodeId: string): ContentNode[] {
    const node = data.nodes.find(n => n.id === nodeId);
    if (!node) return [];

    const nodeMap = new Map(data.nodes.map(n => [n.id, n]));
    return node.relatedNodeIds
      .map(id => nodeMap.get(id))
      .filter(Boolean) as ContentNode[];
  }

  /**
   * Sort nodes by priority (highest first)
   */
  static sortByPriority(nodes: ContentNode[]): ContentNode[] {
    return [...nodes].sort((a, b) => {
      const priorityA = (a.metadata as ImportedContentMetadata).priority || 0;
      const priorityB = (b.metadata as ImportedContentMetadata).priority || 0;
      return priorityB - priorityA;
    });
  }

  /**
   * Group nodes by category
   */
  static groupByCategory(data: ContentGraphImport): Record<string, ContentNode[]> {
    const groups: Record<string, ContentNode[]> = {};

    for (const node of data.nodes) {
      const category = node.category || 'General';
      if (!groups[category]) {
        groups[category] = [];
      }
      groups[category].push(node);
    }

    return groups;
  }

  /**
   * Get statistics for a filtered set of nodes
   */
  static getFilteredStats(nodes: ContentNode[]): {
    total: number;
    byType: Record<string, number>;
    byCategory: Record<string, number>;
  } {
    const stats = {
      total: nodes.length,
      byType: {} as Record<string, number>,
      byCategory: {} as Record<string, number>
    };

    for (const node of nodes) {
      stats.byType[node.contentType] = (stats.byType[node.contentType] || 0) + 1;
      const category = node.category || 'General';
      stats.byCategory[category] = (stats.byCategory[category] || 0) + 1;
    }

    return stats;
  }
}
