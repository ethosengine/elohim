/**
 * Path Metadata Parser
 *
 * Extracts semantic metadata from file system paths.
 *
 * The source content follows a hierarchical structure:
 * data/content/{domain}/{epic}/{userType?}/{category}/filename
 *
 * Examples:
 * - data/content/elohim-protocol/governance/epic.md
 *   → epic narrative for governance domain
 *
 * - data/content/elohim-protocol/governance/policy_maker/README.md
 *   → archetype definition for policy_maker role
 *
 * - data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature
 *   → behavioral scenario for policy_maker
 *
 * - data/content/elohim-protocol/governance/resources/books/climate_justice.md
 *   → book resource related to governance
 */

import * as path from 'path';
import {
  PathMetadata,
  PathParserOptions,
  ContentDomain,
  EpicCategory,
  ContentCategory,
  ResourceType
} from '../models/path-metadata.model';

/**
 * Known epic categories in the elohim-protocol domain
 */
const EPIC_CATEGORIES: Set<string> = new Set([
  'governance',
  'autonomous_entity',
  'public_observer',
  'social_medium',
  'value_scanner',
  'economic_coordination',
  'lamad'
]);

/**
 * Resource directory names that indicate resource content
 */
const RESOURCE_DIRECTORIES: Set<string> = new Set([
  'books',
  'video',
  'videos',
  'audio',
  'organizations',
  'articles',
  'documents',
  'tools',
  'resources'
]);

/**
 * Map directory names to resource types
 */
const RESOURCE_TYPE_MAP: Record<string, ResourceType> = {
  'books': 'book',
  'book': 'book',
  'video': 'video',
  'videos': 'video',
  'audio': 'audio',
  'organizations': 'organization',
  'organisation': 'organization',
  'articles': 'article',
  'article': 'article',
  'documents': 'document',
  'document': 'document',
  'tools': 'tool',
  'tool': 'tool'
};

/**
 * Parse a file path to extract metadata
 */
export function parsePathMetadata(
  filePath: string,
  options: PathParserOptions
): PathMetadata {
  const { contentRoot, normalizeIds = true, idPrefix = '' } = options;

  // Normalize paths
  const normalizedPath = path.normalize(filePath);
  const normalizedRoot = path.normalize(contentRoot);

  // Get relative path from content root
  let relativePath = path.relative(normalizedRoot, normalizedPath);

  // Handle case where file is not under content root
  if (relativePath.startsWith('..')) {
    relativePath = normalizedPath;
  }

  // Split path into parts
  const parts = relativePath.split(path.sep).filter(p => p.length > 0);

  // Extract file info
  const fileName = parts[parts.length - 1] || '';
  const extension = path.extname(fileName).toLowerCase();
  const baseName = path.basename(fileName, extension);

  // Initialize metadata
  const metadata: PathMetadata = {
    fullPath: normalizedPath,
    relativePath,
    domain: 'other',
    epic: 'other',
    userType: undefined,
    contentCategory: 'other',
    resourceType: undefined,
    baseName,
    extension,
    isArchetypeDefinition: false,
    isEpicNarrative: false,
    isScenario: false,
    isResource: false,
    suggestedId: ''
  };

  // Parse path parts
  if (parts.length >= 1) {
    // First part is domain (elohim-protocol, fct, ethosengine)
    metadata.domain = parts[0] as ContentDomain;
  }

  if (parts.length >= 2) {
    // Second part is epic (governance, autonomous_entity, etc.)
    const epicPart = parts[1];
    metadata.epic = EPIC_CATEGORIES.has(epicPart)
      ? epicPart as EpicCategory
      : epicPart;
  }

  // Determine content category and extract additional metadata
  determineContentCategory(parts, metadata);

  // Generate suggested ID
  metadata.suggestedId = generateSuggestedId(metadata, normalizeIds, idPrefix);

  return metadata;
}

/**
 * Determine content category from path parts
 */
function determineContentCategory(parts: string[], metadata: PathMetadata): void {
  const fileName = parts[parts.length - 1] || '';
  const baseName = metadata.baseName.toLowerCase();

  // Check for epic narrative (epic.md at domain level)
  if (baseName === 'epic' && metadata.extension === '.md') {
    metadata.contentCategory = 'epic';
    metadata.isEpicNarrative = true;
    return;
  }

  // Check for scenarios directory
  const hasScenarioDir = parts.some(p => p === 'scenarios');
  if (hasScenarioDir || metadata.extension === '.feature') {
    metadata.contentCategory = 'scenario';
    metadata.isScenario = true;

    // Extract user type from path (part before 'scenarios')
    const scenarioIndex = parts.indexOf('scenarios');
    if (scenarioIndex > 2) {
      metadata.userType = parts[scenarioIndex - 1];
    }
    return;
  }

  // Check for resources
  for (const part of parts) {
    if (RESOURCE_DIRECTORIES.has(part)) {
      metadata.contentCategory = 'resource';
      metadata.isResource = true;
      metadata.resourceType = RESOURCE_TYPE_MAP[part];

      // Extract user type if present before resource dir
      const resourceIndex = parts.indexOf(part);
      if (resourceIndex > 2) {
        const potentialUserType = parts[resourceIndex - 1];
        if (!RESOURCE_DIRECTORIES.has(potentialUserType) && potentialUserType !== 'resources') {
          metadata.userType = potentialUserType;
        }
      }
      return;
    }
  }

  // Check for README.md (archetype definition)
  if (baseName === 'readme' && metadata.extension === '.md') {
    // README in a subdirectory under epic = archetype definition
    if (parts.length > 3) {
      metadata.contentCategory = 'archetype';
      metadata.isArchetypeDefinition = true;
      // User type is the directory containing README
      metadata.userType = parts[parts.length - 2];
      return;
    }
  }

  // Check for concept files
  if (baseName.startsWith('concept-') || baseName.includes('concept')) {
    metadata.contentCategory = 'concept';
    return;
  }

  // Default: documentation or other based on extension
  if (metadata.extension === '.md') {
    metadata.contentCategory = 'documentation';

    // Try to extract user type from path
    if (parts.length > 3) {
      const potentialUserType = parts[2];
      if (!EPIC_CATEGORIES.has(potentialUserType) && !RESOURCE_DIRECTORIES.has(potentialUserType)) {
        metadata.userType = potentialUserType;
      }
    }
  }
}

/**
 * Generate a suggested node ID from metadata
 */
function generateSuggestedId(
  metadata: PathMetadata,
  normalize: boolean,
  prefix: string
): string {
  const parts: string[] = [];

  // Add prefix if provided
  if (prefix) {
    parts.push(prefix.replace(/-$/, ''));
  }

  // Add content category as type prefix
  parts.push(metadata.contentCategory);

  // Add domain (shortened)
  if (metadata.domain === 'elohim-protocol') {
    // Skip domain prefix for main protocol content
  } else if (metadata.domain) {
    parts.push(metadata.domain);
  }

  // Add epic
  if (metadata.epic && metadata.epic !== 'other') {
    parts.push(metadata.epic);
  }

  // Add user type if present
  if (metadata.userType) {
    parts.push(metadata.userType);
  }

  // Add base name (unless it's generic like README or epic)
  const genericNames = new Set(['readme', 'epic', 'index']);
  if (!genericNames.has(metadata.baseName.toLowerCase())) {
    parts.push(metadata.baseName);
  }

  // Join and normalize
  let id = parts.join('-');

  if (normalize) {
    id = id
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '');
  }

  return id;
}

/**
 * Check if a file should be processed based on extension
 */
export function isProcessableFile(filePath: string): boolean {
  const ext = path.extname(filePath).toLowerCase();
  return ext === '.md' || ext === '.feature';
}

/**
 * Get all processable files from a directory listing
 */
export function filterProcessableFiles(files: string[]): string[] {
  return files.filter(isProcessableFile);
}

/**
 * Determine if a path is a source file or derived content
 */
export function isSourceContent(metadata: PathMetadata): boolean {
  // Source content comes from the data/content directory
  return metadata.relativePath.startsWith('data/content') ||
         metadata.relativePath.startsWith('data\\content') ||
         !metadata.relativePath.includes('assets');
}
