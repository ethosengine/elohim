/**
 * Standards Service
 *
 * Generates standards-compliant metadata fields for ContentNodes:
 * - W3C Decentralized Identifiers (DID)
 * - ActivityPub type mapping
 * - Open Graph Protocol metadata
 * - JSON-LD / Schema.org linked data
 * - Git-based timestamps
 */

import { execSync } from 'child_process';
import * as fs from 'fs';

import { ContentNode } from '../models/content-node.model';

/**
 * ActivityPub type mapping from ContentType
 */
const ACTIVITYPUB_TYPE_MAP: Record<string, string> = {
  epic: 'Article',
  feature: 'Article',
  scenario: 'Note',
  video: 'Video',
  book: 'Document',
  'book-chapter': 'Document',
  'bible-verse': 'Note',
  'course-module': 'Article',
  simulation: 'Application',
  assessment: 'Question',
  concept: 'Page',
  organization: 'Organization',
  podcast: 'AudioObject',
  article: 'Article',
  source: 'Document',
  role: 'Page',
  reference: 'Document',
  example: 'Note',
};

/**
 * Schema.org type mapping from ContentType
 */
const SCHEMA_TYPE_MAP: Record<string, string> = {
  epic: 'Article',
  feature: 'Article',
  video: 'VideoObject',
  book: 'Book',
  'book-chapter': 'Chapter',
  organization: 'Organization',
  assessment: 'Quiz',
  'course-module': 'LearningResource',
  'bible-verse': 'CreativeWork',
  podcast: 'PodcastEpisode',
  article: 'Article',
  source: 'CreativeWork',
  scenario: 'HowTo',
  role: 'JobPosting',
  concept: 'DefinedTerm',
  reference: 'Article',
  example: 'CreativeWork',
};

/**
 * Generate W3C Decentralized Identifier from source path
 */
export function generateDid(sourcePath: string, nodeType = 'content'): string {
  // Clean and normalize the path
  let pathPart = sourcePath
    .replace(/\.md$/, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, ':')
    .replace(/_/g, '-')
    .toLowerCase();

  // Remove leading/trailing separators and collapse multiple dashes
  pathPart = pathPart.replace(/-+/g, '-').replace(/^[:-]+|[:-]+$/g, '');

  return `did:web:elohim.host:${nodeType}:${pathPart}`;
}

/**
 * Infer ActivityPub type from ContentType
 */
export function inferActivityPubType(contentType: string): string {
  return ACTIVITYPUB_TYPE_MAP[contentType] || 'Page';
}

/**
 * Get timestamps from git history for a file
 */
export function getGitTimestamps(filePath: string): { created: string; modified: string } {
  const now = new Date().toISOString();

  try {
    // Get first commit (creation date)
    const createdRaw = execSync(`git log --diff-filter=A --format=%aI -- "${filePath}"`, {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'ignore'],
    })
      .trim()
      .split('\n')[0];

    // Get last commit (modification date)
    const modifiedRaw = execSync(`git log -1 --format=%aI -- "${filePath}"`, {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'ignore'],
    }).trim();

    return {
      created: createdRaw || now,
      modified: modifiedRaw || now,
    };
  } catch {
    // Fallback to file system timestamps
    try {
      const stats = fs.statSync(filePath);
      const mtime = stats.mtime.toISOString();
      return { created: mtime, modified: mtime };
    } catch {
      return { created: now, modified: now };
    }
  }
}

/**
 * Open Graph metadata for social sharing
 */
export interface OpenGraphMetadata {
  ogTitle: string;
  ogDescription: string;
  ogType: string;
  ogUrl: string;
  ogSiteName: string;
  ogImage?: string;
  ogImageAlt?: string;
  articlePublishedTime?: string;
  articleModifiedTime?: string;
  articleSection?: string;
}

/**
 * Generate Open Graph metadata for a content node
 */
export function generateOpenGraphMetadata(
  title: string,
  description: string,
  nodeId: string,
  contentType: string,
  metadata: Record<string, unknown> = {},
  timestamps: { created: string; modified: string } = { created: '', modified: '' }
): OpenGraphMetadata {
  const articleTypes = new Set([
    'epic',
    'feature',
    'scenario',
    'concept',
    'course-module',
    'article',
  ]);

  const og: OpenGraphMetadata = {
    ogTitle: title,
    ogDescription: (description || title).substring(0, 200),
    ogType: articleTypes.has(contentType) ? 'article' : 'website',
    ogUrl: `https://elohim-protocol.org/content/${nodeId}`,
    ogSiteName: 'Elohim Protocol - Lamad Learning Platform',
  };

  // Add timestamps for articles
  if (articleTypes.has(contentType)) {
    og.articlePublishedTime = timestamps.created;
    og.articleModifiedTime = timestamps.modified;
    if (metadata.epic) {
      og.articleSection = String(metadata.epic);
    }
  }

  // Default image
  og.ogImage = `https://elohim-protocol.org/assets/images/og-defaults/${contentType}.jpg`;
  og.ogImageAlt = `${title} - Elohim Protocol`;

  return og;
}

/**
 * JSON-LD linked data structure
 */
export interface LinkedData {
  '@context': string;
  '@type': string;
  '@id': string;
  identifier: string;
  name: string;
  description: string;
  dateCreated?: string;
  dateModified?: string;
  publisher: {
    '@type': string;
    '@id': string;
    name: string;
  };
  author?: {
    '@type': string;
    name: string;
  };
  isPartOf?: {
    '@type': string;
    name: string;
  };
}

/**
 * Generate JSON-LD for semantic web compliance
 */
export function generateLinkedData(
  nodeId: string,
  did: string,
  contentType: string,
  title: string,
  description: string,
  timestamps: { created: string; modified: string },
  metadata: Record<string, unknown> = {}
): LinkedData {
  const schemaType = SCHEMA_TYPE_MAP[contentType] || 'CreativeWork';

  const linkedData: LinkedData = {
    '@context': 'https://schema.org/',
    '@type': schemaType,
    '@id': `https://elohim-protocol.org/content/${nodeId}`,
    identifier: did,
    name: title,
    description: description || title,
    dateCreated: timestamps.created,
    dateModified: timestamps.modified,
    publisher: {
      '@type': 'Organization',
      '@id': 'https://elohim-protocol.org',
      name: 'Elohim Protocol',
    },
  };

  // Add author if available
  if (metadata.author) {
    linkedData.author = {
      '@type': 'Person',
      name: String(metadata.author),
    };
  }

  // Add epic as isPartOf
  if (metadata.epic) {
    linkedData.isPartOf = {
      '@type': 'CreativeWorkSeries',
      name: String(metadata.epic),
    };
  }

  return linkedData;
}

/**
 * Standards fields to add to a ContentNode
 */
export interface StandardsFields {
  did: string;
  activityPubType: string;
  openGraphMetadata: OpenGraphMetadata;
  linkedData: LinkedData;
}

/**
 * Generate all standards fields for a ContentNode
 */
export function generateStandardsFields(node: ContentNode, sourcePath?: string): StandardsFields {
  const effectivePath = sourcePath || node.sourcePath || node.id;
  const did = generateDid(effectivePath, 'content');
  const activityPubType = inferActivityPubType(node.contentType);

  // Get timestamps
  let timestamps = { created: node.createdAt || '', modified: node.updatedAt || '' };
  if (sourcePath && fs.existsSync(sourcePath)) {
    timestamps = getGitTimestamps(sourcePath);
  }

  const openGraphMetadata = generateOpenGraphMetadata(
    node.title,
    node.description || '',
    node.id,
    node.contentType,
    node.metadata || {},
    timestamps
  );

  const linkedData = generateLinkedData(
    node.id,
    did,
    node.contentType,
    node.title,
    node.description || '',
    timestamps,
    node.metadata || {}
  );

  return {
    did,
    activityPubType,
    openGraphMetadata,
    linkedData,
  };
}

/**
 * Enrich a ContentNode with standards fields
 */
export function enrichWithStandards(
  node: ContentNode,
  sourcePath?: string
): ContentNode & StandardsFields {
  const standards = generateStandardsFields(node, sourcePath);

  return {
    ...node,
    ...standards,
  };
}

/**
 * Validation result for standards fields
 */
export interface StandardsValidationResult {
  field: string;
  valid: boolean;
  error?: string;
}

/**
 * Validate standards fields on a node
 */
export function validateStandardsFields(
  node: Record<string, unknown>
): StandardsValidationResult[] {
  const results: StandardsValidationResult[] = [];

  // Validate DID
  const did = node.did as string | undefined;
  if (did) {
    if (did.startsWith('did:')) {
      results.push({ field: 'did', valid: true });
    } else {
      results.push({ field: 'did', valid: false, error: `Invalid DID format: ${did}` });
    }
  } else {
    results.push({ field: 'did', valid: false, error: 'Missing DID' });
  }

  // Validate ActivityPub type
  if (node.activityPubType) {
    results.push({ field: 'activityPubType', valid: true });
  } else {
    results.push({ field: 'activityPubType', valid: false, error: 'Missing activityPubType' });
  }

  // Validate JSON-LD
  const ld = node.linkedData as Record<string, unknown> | undefined;
  if (ld) {
    if (!ld['@context']) {
      results.push({ field: 'linkedData', valid: false, error: 'JSON-LD missing @context' });
    } else if (ld['@type']) {
      results.push({ field: 'linkedData', valid: true });
    } else {
      results.push({ field: 'linkedData', valid: false, error: 'JSON-LD missing @type' });
    }
  } else {
    results.push({ field: 'linkedData', valid: false, error: 'Missing linkedData' });
  }

  // Validate Open Graph
  const og = node.openGraphMetadata as Record<string, unknown> | undefined;
  if (og) {
    const required = ['ogTitle', 'ogDescription', 'ogUrl'];
    const missing = required.filter(f => !og[f]);
    if (missing.length > 0) {
      results.push({
        field: 'openGraphMetadata',
        valid: false,
        error: `Open Graph missing: ${missing.join(', ')}`,
      });
    } else {
      results.push({ field: 'openGraphMetadata', valid: true });
    }
  } else {
    results.push({ field: 'openGraphMetadata', valid: false, error: 'Missing openGraphMetadata' });
  }

  return results;
}

/**
 * Standards coverage summary
 */
export interface StandardsCoverageReport {
  total: number;
  coverage: Record<string, { count: number; total: number; percentage: number }>;
  errors: string[];
  allTargetsMet: boolean;
}

/**
 * Generate coverage report for an array of nodes
 */
export function generateCoverageReport(nodes: Record<string, unknown>[]): StandardsCoverageReport {
  const total = nodes.length;
  const coverage: Record<string, { count: number; total: number; percentage: number }> = {
    did: { count: 0, total, percentage: 0 },
    activityPubType: { count: 0, total, percentage: 0 },
    linkedData: { count: 0, total, percentage: 0 },
    openGraphMetadata: { count: 0, total, percentage: 0 },
  };
  const errors: string[] = [];

  for (const node of nodes) {
    const validations = validateStandardsFields(node);
    for (const result of validations) {
      if (result.valid) {
        coverage[result.field].count++;
      } else if (result.error) {
        const id = (node.id as string) || 'unknown';
        errors.push(`${id}: ${result.error}`);
      }
    }
  }

  // Calculate percentages
  for (const field of Object.keys(coverage)) {
    coverage[field].percentage = total > 0 ? (coverage[field].count / total) * 100 : 0;
  }

  // Check targets (from original Python script)
  const targets: Record<string, number> = {
    did: 100,
    activityPubType: 100,
    linkedData: 80,
    openGraphMetadata: 80,
  };

  const allTargetsMet = Object.entries(targets).every(
    ([field, target]) => coverage[field].percentage >= target
  );

  return { total, coverage, errors, allTargetsMet };
}
