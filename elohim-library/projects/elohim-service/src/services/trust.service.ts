/**
 * Trust Service
 *
 * Manages trust scoring and attestation-based content enrichment.
 * Ported from add_trust_fields.py
 */

import * as fs from 'fs';
import * as path from 'path';

/**
 * Reach levels for content visibility
 */
export type ReachLevel = 'private' | 'invited' | 'local' | 'community' | 'federated' | 'commons';

/**
 * Reach level ordering (higher = more visible)
 */
const REACH_ORDER: Record<ReachLevel, number> = {
  private: 0,
  invited: 1,
  local: 2,
  community: 3,
  federated: 4,
  commons: 5,
};

/**
 * Attestation types and their weight for trust scoring
 */
const ATTESTATION_WEIGHTS: Record<string, number> = {
  'author-verified': 0.1,
  'steward-approved': 0.3,
  'community-endorsed': 0.2,
  'peer-reviewed': 0.4,
  'governance-ratified': 0.5,
  'curriculum-canonical': 0.5,
  'safety-reviewed': 0.2,
  'accuracy-verified': 0.3,
  'accessibility-checked': 0.1,
  'license-cleared': 0.2,
};

/**
 * Default trust values for content without attestations
 */
const DEFAULT_AUTHOR = 'system';
const DEFAULT_REACH: ReachLevel = 'commons';
const DEFAULT_TRUST_SCORE = 0.8;

/**
 * Attestation structure
 */
export interface Attestation {
  id: string;
  contentId: string;
  attestationType: string;
  status: 'active' | 'revoked' | 'expired';
  reachGranted?: ReachLevel;
  attesterId?: string;
  createdAt?: string;
  expiresAt?: string;
}

/**
 * Attestations index file structure
 */
export interface AttestationsIndex {
  attestations: Attestation[];
  generatedAt?: string;
}

/**
 * Trust fields to add to content nodes
 */
export interface TrustFields {
  authorId: string;
  reach: ReachLevel;
  trustScore: number;
  activeAttestationIds: string[];
  trustComputedAt: string;
}

/**
 * Load attestations from index file
 */
export function loadAttestations(attestationsPath: string): Map<string, Attestation[]> {
  const byContent = new Map<string, Attestation[]>();

  if (!fs.existsSync(attestationsPath)) {
    console.warn(`Attestations file not found: ${attestationsPath}`);
    return byContent;
  }

  try {
    const data = JSON.parse(fs.readFileSync(attestationsPath, 'utf-8')) as AttestationsIndex;

    for (const att of data.attestations || []) {
      const contentId = att.contentId;
      if (contentId) {
        const existing = byContent.get(contentId) || [];
        existing.push(att);
        byContent.set(contentId, existing);
      }
    }
  } catch (err) {
    console.error(`Failed to parse attestations file '${attestationsPath}': ${err}`);
  }

  return byContent;
}

/**
 * Calculate trust score from attestations
 */
export function calculateTrustScore(attestations: Attestation[]): number {
  if (!attestations || attestations.length === 0) {
    return DEFAULT_TRUST_SCORE;
  }

  let totalWeight = 0;

  for (const att of attestations) {
    if (att.status === 'active') {
      const weight = ATTESTATION_WEIGHTS[att.attestationType] || 0.1;
      totalWeight += weight;
    }
  }

  // Normalize to 0-1 range (max possible ~1.8 if all attestations present)
  return Math.min(1, totalWeight / 1.5);
}

/**
 * Determine effective reach level from attestations
 */
export function getEffectiveReach(attestations: Attestation[]): ReachLevel {
  if (!attestations || attestations.length === 0) {
    return DEFAULT_REACH;
  }

  let highestReach: ReachLevel = 'private';
  let highestLevel = 0;

  for (const att of attestations) {
    if (att.status === 'active' && att.reachGranted) {
      const level = REACH_ORDER[att.reachGranted] || 0;
      if (level > highestLevel) {
        highestLevel = level;
        highestReach = att.reachGranted;
      }
    }
  }

  return highestReach;
}

/**
 * Generate trust fields for a content node
 */
export function generateTrustFields(
  contentId: string,
  attestations: Attestation[],
  existingAuthorId?: string
): TrustFields {
  const activeAttestationIds = attestations
    .filter(att => att.status === 'active')
    .map(att => att.id);

  return {
    authorId: existingAuthorId || DEFAULT_AUTHOR,
    reach: getEffectiveReach(attestations),
    trustScore: Math.round(calculateTrustScore(attestations) * 100) / 100,
    activeAttestationIds,
    trustComputedAt: new Date().toISOString(),
  };
}

/**
 * Content node with trust fields
 */
export interface ContentWithTrust {
  id: string;
  authorId?: string;
  reach?: ReachLevel;
  trustScore?: number;
  activeAttestationIds?: string[];
  trustComputedAt?: string;
  [key: string]: unknown;
}

/**
 * Enrich a content node with trust fields
 */
export function enrichWithTrust(
  content: ContentWithTrust,
  attestationsByContent: Map<string, Attestation[]>
): ContentWithTrust {
  const attestations = attestationsByContent.get(content.id) || [];
  const trustFields = generateTrustFields(content.id, attestations, content.authorId);

  return {
    ...content,
    ...trustFields,
  };
}

/**
 * Process result for trust enrichment
 */
export interface TrustEnrichmentResult {
  processed: number;
  enriched: number;
  withAttestations: number;
  errors: string[];
}

/**
 * Enrich all content files in a directory with trust fields
 */
export async function enrichContentDirectory(
  contentDir: string,
  attestationsPath: string
): Promise<TrustEnrichmentResult> {
  const result: TrustEnrichmentResult = {
    processed: 0,
    enriched: 0,
    withAttestations: 0,
    errors: [],
  };

  // Load attestations
  const attestationsByContent = loadAttestations(attestationsPath);
  console.log(`Loaded attestations for ${attestationsByContent.size} content nodes`);

  // Get all JSON files
  const files = fs.readdirSync(contentDir).filter(f => f.endsWith('.json') && f !== 'index.json');

  for (const file of files) {
    const filePath = path.join(contentDir, file);

    try {
      const content = JSON.parse(fs.readFileSync(filePath, 'utf-8')) as ContentWithTrust;
      result.processed++;

      const enriched = enrichWithTrust(content, attestationsByContent);

      if (enriched.activeAttestationIds && enriched.activeAttestationIds.length > 0) {
        result.withAttestations++;
      }

      fs.writeFileSync(filePath, JSON.stringify(enriched, null, 2), 'utf-8');
      result.enriched++;

      const hasAtt = enriched.activeAttestationIds?.length ? '✓' : '·';
      console.log(
        `  ${hasAtt} ${file.substring(0, 50).padEnd(50)} reach=${enriched.reach?.padEnd(12)} score=${enriched.trustScore}`
      );
    } catch (err) {
      result.errors.push(`${file}: ${err}`);
    }
  }

  return result;
}

/**
 * Update content index with trust summaries
 */
export function updateContentIndexWithTrust(
  indexPath: string,
  attestationsByContent: Map<string, Attestation[]>
): void {
  if (!fs.existsSync(indexPath)) {
    console.warn(`Index file not found: ${indexPath}`);
    return;
  }

  try {
    const index = JSON.parse(fs.readFileSync(indexPath, 'utf-8')) as {
      nodes?: { id: string; reach?: ReachLevel; trustScore?: number; hasAttestations?: boolean }[];
      lastUpdated?: string;
    };

    for (const node of index.nodes || []) {
      const attestations = attestationsByContent.get(node.id) || [];
      node.reach = getEffectiveReach(attestations);
      node.trustScore = Math.round(calculateTrustScore(attestations) * 100) / 100;
      node.hasAttestations = attestations.length > 0;
    }

    index.lastUpdated = new Date().toISOString();

    fs.writeFileSync(indexPath, JSON.stringify(index, null, 2), 'utf-8');
    console.log(`Updated ${indexPath}`);
  } catch (err) {
    console.error(`Failed to update content index '${indexPath}': ${err}`);
  }
}
