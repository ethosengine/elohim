/**
 * Parser Result Types
 *
 * Standardizes the contract between parsers and transformers.
 * Parsers extract structure; transformers extract meaning.
 */

import { PathMetadata } from '../models/path-metadata.model';

/**
 * Base result returned by all parsers
 */
export interface ParserResult {
  /** Path metadata */
  pathMeta: PathMetadata;

  /** YAML frontmatter (if any) */
  frontmatter: Record<string, unknown>;

  /** Raw content string */
  rawContent: string;

  /** Extracted title (or empty string if none found) */
  title: string;

  /** File hash for change detection */
  contentHash: string;

  /** File extension for format detection */
  extension: string;
}

/**
 * Result from markdown parser
 */
export interface MarkdownParserResult extends ParserResult {
  /** Parsed sections with hierarchy */
  sections: ParsedSection[];
}

/**
 * Result from Gherkin parser
 */
export interface GherkinParserResult extends ParserResult {
  /** Parsed scenarios */
  scenarios: ParsedScenario[];

  /** Feature-level tags */
  featureTags: GherkinTag[];
}

/**
 * Markdown section with hierarchy
 */
export interface ParsedSection {
  level: number;
  title: string;
  anchor: string;
  content: string;
  children: ParsedSection[];
}

/**
 * Gherkin scenario
 */
export interface ParsedScenario {
  title: string;
  type: 'scenario' | 'scenario_outline';
  tags: string[];
  steps: ParsedStep[];
}

/**
 * Gherkin step
 */
export interface ParsedStep {
  keyword: string;
  text: string;
}

/**
 * Gherkin tag with optional value
 */
export interface GherkinTag {
  name: string;
  value?: string;
}

/**
 * Parser error with context
 */
export class ParserError extends Error {
  constructor(
    message: string,
    public readonly filePath: string,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'ParserError';
  }
}

/**
 * Validate that parser result has required fields
 */
export function validateParserResult(result: ParserResult): void {
  if (!result.pathMeta) {
    throw new ParserError('Missing pathMeta', 'unknown');
  }
  if (!result.rawContent) {
    throw new ParserError('Missing rawContent', result.pathMeta.fullPath);
  }
  if (!result.contentHash) {
    throw new ParserError('Missing contentHash', result.pathMeta.fullPath);
  }
}
