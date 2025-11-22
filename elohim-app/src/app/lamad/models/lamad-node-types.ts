/**
 * Lamad-specific node types and metadata interfaces.
 * These map to the 'contentType' field in the generic ContentNode model.
 * 
 * Note: "Elohim" refers to the real-time agents in the system. 
 * These types represent the static content nodes within the Lamad learning graph.
 */

export enum LamadNodeType {
  // Core documentation types
  EPIC = 'epic',
  FEATURE = 'feature', // userstories
  SCENARIO = 'scenario', // specific scenarios within features
  
  // Resource types found in docs/
  AUDIO = 'audio',
  DOCUMENT = 'document', // 'docs' in prompt
  VIDEO = 'video',
  ARTICLE = 'article',
  BOOK = 'book',
  PAGE = 'page', // html content
  
  // Structural types
  ORGANIZATION = 'organization',
  USER_TYPE = 'user_type', // Archetypes like community_investor
}

/**
 * Base metadata interface that all Lamad content nodes share
 */
export interface LamadBaseMetadata {
  gemId?: string;
  primaryEpic?: string;
  relatedEpics?: string[];
}

/**
 * Metadata for Organization nodes
 * Source: docs/governance/organizations/...
 */
export interface OrganizationMetadata extends LamadBaseMetadata {
  orgId: string;
  url: string;
  name: string;
  
  // Specific organization fields
  epicRelationships?: {
    [epicName: string]: {
      inspiration: string;
      parallelWork: string[];
    }
  };
  
  demonstratesPrinciples?: string[];
  inspiresUsers?: string[];
  operatesAtLayers?: string[];
  edgeTypes?: string[];
}

/**
 * Metadata for User Type (Archetype) nodes
 * Source: docs/autonomous_entity/community_investor/README.md
 */
export interface UserTypeMetadata extends LamadBaseMetadata {
  userType: string;
  archetypeName: string;
  epicDomain: string;
  governanceScope: string[];
  relatedUsers: string[];
}

/**
 * Metadata for Resource nodes (Book, Video, Audio, Article, Document)
 */
export interface ResourceMetadata extends LamadBaseMetadata {
  orgId?: string;
  name: string;
  url?: string;
  publisher?: string;
  
  demonstratesPrinciples?: string[];
  inspiresUsers?: string[];
  operatesAtLayers?: string[];
}

/**
 * Metadata for Feature/User Story nodes
 * Source: .feature files tags
 */
export interface FeatureMetadata extends LamadBaseMetadata {
  userType?: string;
  governanceLayer?: string;
  relatedUsers?: string[];
  relatedLayers?: string[];
  elohimAgents?: string[];
}