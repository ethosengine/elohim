/**
 * Base Parser Utilities
 *
 * Shared logic for all parsers - structure extraction only.
 * No semantic extraction (tags, descriptions, etc.) - that's for transformers.
 */

import * as crypto from 'crypto';
import { PathMetadata } from '../models/path-metadata.model';
import { ParserResult, ParserError } from './parser-result';

/**
 * Calculate SHA-256 hash of content for change detection
 */
export function calculateContentHash(content: string): string {
  try {
    return crypto.createHash('sha256').update(content).digest('hex');
  } catch (error) {
    throw new ParserError(
      'Failed to calculate content hash',
      'unknown',
      error as Error
    );
  }
}

/**
 * Build base parser result with common fields
 */
export function buildParserResult(
  content: string,
  pathMeta: PathMetadata,
  frontmatter: Record<string, unknown>,
  title: string
): ParserResult {
  return {
    pathMeta,
    frontmatter,
    rawContent: content,
    title,
    contentHash: calculateContentHash(content),
    extension: pathMeta.extension
  };
}

/**
 * Split content into lines, handling different line endings
 */
export function splitLines(content: string): string[] {
  return content.split(/\r?\n/);
}

/**
 * Check if line matches a pattern
 */
export function matchLine(line: string, pattern: RegExp): RegExpExecArray | null {
  return pattern.exec(line.trim());
}
