/**
 * Utility functions for building metadata objects in transformers
 */

import { extractGovernanceScope } from '../parsers/markdown-parser';

/**
 * Build base metadata with source information
 * @param sourceVersion - Version of the source (default: '1.0.0')
 * @returns Base metadata object
 */
export function buildBaseMetadata(sourceVersion = '1.0.0'): Record<string, unknown> {
  return {
    source: 'elohim-import',
    sourceVersion,
  };
}

/**
 * Add provenance metadata to an existing metadata object
 * @param metadata - Metadata object to update
 * @param sourceNodeId - ID of the source node
 * @param extractionMethod - Method used for extraction (default: 'direct-import')
 */
export function addProvenanceMetadata(
  metadata: Record<string, unknown>,
  sourceNodeId: string | undefined,
  extractionMethod = 'direct-import'
): void {
  if (sourceNodeId) {
    metadata.derivedFrom = sourceNodeId;
    metadata.extractionMethod = extractionMethod;
  }
}

/**
 * Add governance scope metadata from frontmatter
 * @param metadata - Metadata object to update
 * @param frontmatter - Frontmatter object from parsed content
 */
export function addGovernanceScopeMetadata(
  metadata: Record<string, unknown>,
  frontmatter: any
): void {
  const governanceScope = extractGovernanceScope(frontmatter);
  if (governanceScope.length > 0) {
    metadata.governanceScope = governanceScope;
  }
}
