/**
 * Path Metadata Model
 *
 * Metadata extracted from file system paths.
 * The source content follows a hierarchical structure:
 *
 * data/content/{domain}/{epic}/{userType?}/{category}/filename
 *
 * Example:
 * data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature
 * → domain: elohim-protocol
 * → epic: governance
 * → userType: policy_maker
 * → contentCategory: scenario
 */

/**
 * Content domains in the repository
 */
export type ContentDomain =
  | 'elohim-protocol'  // Main protocol content
  | 'fct'              // Foundations for Christian Technology
  | 'ethosengine';     // EthosEngine content

/**
 * Epic/pillar categories within elohim-protocol
 */
export type EpicCategory =
  | 'governance'
  | 'autonomous_entity'
  | 'public_observer'
  | 'social_medium'
  | 'value_scanner'
  | 'economic_coordination'
  | 'lamad'
  | 'other';

/**
 * Content categories determined by file location/type
 */
export type ContentCategory =
  | 'epic'           // Domain-level narrative (epic.md)
  | 'archetype'      // Role/persona definition (README.md in role dir)
  | 'scenario'       // Behavioral spec (.feature file)
  | 'resource'       // Supporting resource (books, videos, orgs)
  | 'concept'        // Abstract concept
  | 'documentation'  // General documentation
  | 'other';

/**
 * Resource subtypes (when contentCategory is 'resource')
 */
export type ResourceType =
  | 'book'
  | 'video'
  | 'audio'
  | 'organization'
  | 'article'
  | 'document'
  | 'tool';

/**
 * Metadata extracted from file path
 */
export interface PathMetadata {
  /** Full original file path */
  fullPath: string;

  /** Relative path from content root */
  relativePath: string;

  /** Content domain (elohim-protocol, fct, ethosengine) */
  domain: ContentDomain | string;

  /** Epic/pillar (governance, autonomous_entity, etc.) */
  epic: EpicCategory | string;

  /** User type/archetype if in a role directory */
  userType?: string;

  /** Determined content category */
  contentCategory: ContentCategory;

  /** Resource type if this is a resource */
  resourceType?: ResourceType;

  /** File name without extension */
  baseName: string;

  /** File extension */
  extension: string;

  /** Whether this is a README.md (archetype definition) */
  isArchetypeDefinition: boolean;

  /** Whether this is an epic.md (domain narrative) */
  isEpicNarrative: boolean;

  /** Whether this is in a scenarios directory */
  isScenario: boolean;

  /** Whether this is in a resources directory (books, videos, etc.) */
  isResource: boolean;

  /** Suggested node ID based on path */
  suggestedId: string;
}

/**
 * Options for path parsing
 */
export interface PathParserOptions {
  /** Base path to content directory */
  contentRoot: string;

  /** Whether to normalize IDs to lowercase */
  normalizeIds?: boolean;

  /** ID prefix (e.g., 'source-' for source nodes) */
  idPrefix?: string;
}
