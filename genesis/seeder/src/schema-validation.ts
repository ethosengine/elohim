/**
 * Schema Validation for Seed Data
 *
 * Validates seed files offline before any Holochain zome calls.
 * This catches data issues in seconds instead of failing after 20+ minutes.
 *
 * Usage:
 *   import { validateContentFile, validateAllContent, ValidationReport } from './schema-validation';
 *
 *   // Validate single file
 *   const result = validateContentFile('/path/to/content.json');
 *
 *   // Validate all files
 *   const report = await validateAllContent('/path/to/data/lamad/content');
 */

import * as fs from 'fs';
import * as path from 'path';

// =============================================================================
// Content Schema (matches what seed files should contain)
// =============================================================================

const VALID_CONTENT_TYPES = [
  'epic', 'concept', 'lesson', 'scenario', 'assessment', 'resource',
  'reflection', 'discussion', 'exercise', 'example', 'reference', 'article',
  'role', 'quiz', 'simulation'  // Additional types used in seed files
];

// Content Format Architecture:
// - DNA stores metadata including `content_format` as a hint for clients
// - DNA does NOT validate the actual format (intentionally flexible)
// - holochain-cache-core handles content resolution and delivery
// - Clients use format to determine rendering strategy
//
// This means ANY format string is valid at the DNA level.
// Validation here is for CLIENT compatibility, not DNA rejection.

// Formats the CLIENT knows how to render
const CLIENT_SUPPORTED_FORMATS = [
  // Standard formats
  'markdown', 'html', 'video', 'audio', 'interactive', 'external',
  // Extended formats (cached blobs, not in DHT)
  'gherkin', 'perseus-quiz-json', 'html5-app', 'plaintext'
];

// In strict mode, warn about unknown formats (client can't render them)
// In loose mode, accept any format (for forward compatibility)
const STRICT_VALIDATION = process.env.STRICT_VALIDATION === 'true';
const VALID_CONTENT_FORMATS = STRICT_VALIDATION
  ? CLIENT_SUPPORTED_FORMATS
  : null;  // null = accept any format

const VALID_REACH_LEVELS = [
  'private', 'self', 'intimate', 'trusted', 'familiar', 'community', 'public', 'commons'
];

interface ContentSeedFile {
  id: string;
  contentType?: string;
  contentFormat?: string;
  title?: string;
  description?: string;
  content?: string | object;
  tags?: string[];
  reach?: string;
  relatedNodeIds?: string[];
  estimatedMinutes?: number;
  thumbnailUrl?: string;
  // Blob reference fields (for content stored in projection cache)
  blob_hash?: string;      // SHA256 hash of blob content
  blob_url?: string;       // URL to fetch blob from cache
  blob_file?: string;      // Local file path for blob
  entry_point?: string;    // Entry point for compound content (e.g., "index.html")
  fallback_url?: string;   // External fallback URL
  // Additional fields that may be present
  did?: string;
  activityPubType?: string;
  questions?: unknown[];  // For assessments
  [key: string]: unknown;
}

interface ValidationError {
  field: string;
  message: string;
  value?: unknown;
}

interface FileValidationResult {
  filePath: string;
  valid: boolean;
  errors: ValidationError[];
  warnings: string[];
}

export interface ValidationReport {
  totalFiles: number;
  validFiles: number;
  invalidFiles: number;
  errors: Map<string, ValidationError[]>;
  warnings: Map<string, string[]>;
  formatCoverage: Map<string, number>;
  typeCoverage: Map<string, number>;
}

// =============================================================================
// Validation Functions
// =============================================================================

function validateField(
  obj: ContentSeedFile,
  field: string,
  expectedType: 'string' | 'number' | 'array' | 'object',
  required: boolean = false
): ValidationError | null {
  const value = obj[field];

  if (value === undefined || value === null) {
    if (required) {
      return { field, message: `Required field '${field}' is missing` };
    }
    return null;
  }

  switch (expectedType) {
    case 'string':
      if (typeof value !== 'string') {
        return { field, message: `Field '${field}' should be a string`, value };
      }
      break;
    case 'number':
      if (typeof value !== 'number') {
        return { field, message: `Field '${field}' should be a number`, value };
      }
      break;
    case 'array':
      if (!Array.isArray(value)) {
        return { field, message: `Field '${field}' should be an array`, value };
      }
      break;
    case 'object':
      if (typeof value !== 'object' || Array.isArray(value)) {
        return { field, message: `Field '${field}' should be an object`, value };
      }
      break;
  }

  return null;
}

function validateEnum(
  obj: ContentSeedFile,
  field: string,
  validValues: string[],
  required: boolean = false
): ValidationError | null {
  const value = obj[field];

  if (value === undefined || value === null) {
    if (required) {
      return { field, message: `Required field '${field}' is missing` };
    }
    return null;
  }

  if (typeof value !== 'string') {
    return { field, message: `Field '${field}' should be a string`, value };
  }

  if (!validValues.includes(value)) {
    return {
      field,
      message: `Invalid value for '${field}'. Must be one of: ${validValues.join(', ')}`,
      value
    };
  }

  return null;
}

export function validateContentFile(filePath: string): FileValidationResult {
  const result: FileValidationResult = {
    filePath,
    valid: true,
    errors: [],
    warnings: []
  };

  // Check file exists
  if (!fs.existsSync(filePath)) {
    result.valid = false;
    result.errors.push({ field: 'file', message: `File not found: ${filePath}` });
    return result;
  }

  // Parse JSON
  let content: ContentSeedFile;
  try {
    const raw = fs.readFileSync(filePath, 'utf-8');
    content = JSON.parse(raw);
  } catch (e) {
    result.valid = false;
    result.errors.push({
      field: 'json',
      message: `Invalid JSON: ${e instanceof Error ? e.message : 'Unknown error'}`
    });
    return result;
  }

  // Validate required fields
  const idError = validateField(content, 'id', 'string', true);
  if (idError) {
    result.errors.push(idError);
    result.valid = false;
  }

  // Validate optional fields with correct types
  const fieldValidations: [string, 'string' | 'number' | 'array' | 'object', boolean][] = [
    ['title', 'string', false],
    ['description', 'string', false],
    ['tags', 'array', false],
    ['relatedNodeIds', 'array', false],
    ['estimatedMinutes', 'number', false],
    ['thumbnailUrl', 'string', false],
  ];

  for (const [field, type, required] of fieldValidations) {
    const error = validateField(content, field, type, required);
    if (error) {
      result.errors.push(error);
      result.valid = false;
    }
  }

  // Validate content field (can be string or object for assessments)
  if (content.content !== undefined && content.content !== null) {
    if (typeof content.content !== 'string' && typeof content.content !== 'object') {
      result.errors.push({
        field: 'content',
        message: 'Field \'content\' should be a string or object',
        value: typeof content.content
      });
      result.valid = false;
    }
  }

  // Validate enums
  const contentTypeError = validateEnum(content, 'contentType', VALID_CONTENT_TYPES, false);
  if (contentTypeError) {
    // Warn but don't fail - might be a new type
    result.warnings.push(`${contentTypeError.message} (value: ${contentTypeError.value})`);
  }

  // Content format validation:
  // - If VALID_CONTENT_FORMATS is null, accept any format (loose mode)
  // - If VALID_CONTENT_FORMATS is set, warn about unknown formats (strict mode)
  if (VALID_CONTENT_FORMATS !== null) {
    const contentFormatError = validateEnum(content, 'contentFormat', VALID_CONTENT_FORMATS, false);
    if (contentFormatError) {
      // Warn only - unknown formats may work with future client updates
      result.warnings.push(`${contentFormatError.message} (client may not render correctly)`);
    }
  }

  const reachError = validateEnum(content, 'reach', VALID_REACH_LEVELS, false);
  if (reachError) {
    result.errors.push(reachError);
    result.valid = false;
  }

  // Validate array contents
  if (Array.isArray(content.tags)) {
    for (let i = 0; i < content.tags.length; i++) {
      if (typeof content.tags[i] !== 'string') {
        result.errors.push({
          field: `tags[${i}]`,
          message: 'All tags must be strings',
          value: content.tags[i]
        });
        result.valid = false;
      }
    }
  }

  if (Array.isArray(content.relatedNodeIds)) {
    for (let i = 0; i < content.relatedNodeIds.length; i++) {
      if (typeof content.relatedNodeIds[i] !== 'string') {
        result.errors.push({
          field: `relatedNodeIds[${i}]`,
          message: 'All relatedNodeIds must be strings',
          value: content.relatedNodeIds[i]
        });
        result.valid = false;
      }
    }
  }

  // Validate blob references for blob-extractable formats
  const BLOB_FORMATS = ['html5-app', 'perseus-quiz-json'];
  if (content.contentFormat && BLOB_FORMATS.includes(content.contentFormat)) {
    const hasInlineContent = content.content !== undefined && content.content !== null;
    const hasBlobRef = content.blob_hash !== undefined && content.blob_hash !== null;

    // Warn if has neither
    if (!hasInlineContent && !hasBlobRef) {
      result.warnings.push(`${content.contentFormat} content should have either 'content' or 'blob_hash'`);
    }

    // Validate blob_hash format if present
    if (hasBlobRef) {
      const hash = content.blob_hash as string;
      if (!/^sha256-[a-f0-9]{64}$/.test(hash)) {
        result.warnings.push(`Invalid blob_hash format (expected sha256-{64 hex chars}): ${hash}`);
      }

      // Warn about missing fallback_url
      if (!content.fallback_url) {
        result.warnings.push(`No fallback_url for blob content (recommended for resilience)`);
      }
    }

    // html5-app should have entry_point when using blob reference
    if (content.contentFormat === 'html5-app' && hasBlobRef && !content.entry_point) {
      result.warnings.push(`html5-app with blob_hash should have 'entry_point' (defaulting to 'index.html')`);
    }
  }

  return result;
}

export async function validateAllContent(contentDir: string): Promise<ValidationReport> {
  const report: ValidationReport = {
    totalFiles: 0,
    validFiles: 0,
    invalidFiles: 0,
    errors: new Map(),
    warnings: new Map(),
    formatCoverage: new Map(),
    typeCoverage: new Map()
  };

  // Find all JSON files
  const files = fs.readdirSync(contentDir)
    .filter(f => f.endsWith('.json') && f !== 'index.json')
    .map(f => path.join(contentDir, f));

  report.totalFiles = files.length;

  for (const filePath of files) {
    const result = validateContentFile(filePath);
    const fileName = path.basename(filePath);

    if (result.valid) {
      report.validFiles++;
    } else {
      report.invalidFiles++;
      report.errors.set(fileName, result.errors);
    }

    if (result.warnings.length > 0) {
      report.warnings.set(fileName, result.warnings);
    }

    // Track format and type coverage
    try {
      const content = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
      if (content.contentFormat) {
        const count = report.formatCoverage.get(content.contentFormat) || 0;
        report.formatCoverage.set(content.contentFormat, count + 1);
      }
      if (content.contentType) {
        const count = report.typeCoverage.get(content.contentType) || 0;
        report.typeCoverage.set(content.contentType, count + 1);
      }
    } catch {
      // Already captured as parse error
    }
  }

  return report;
}

export function printValidationReport(report: ValidationReport): void {
  console.log('\n' + '='.repeat(70));
  console.log('SEED DATA VALIDATION REPORT');
  console.log('='.repeat(70));

  console.log(`\nTotal files:   ${report.totalFiles}`);
  console.log(`Valid files:   ${report.validFiles} ✅`);
  console.log(`Invalid files: ${report.invalidFiles} ❌`);

  if (report.errors.size > 0) {
    console.log('\n--- ERRORS ---');
    for (const [file, errors] of report.errors) {
      console.log(`\n${file}:`);
      for (const error of errors) {
        console.log(`  ❌ ${error.field}: ${error.message}`);
        if (error.value !== undefined) {
          console.log(`     Value: ${JSON.stringify(error.value)}`);
        }
      }
    }
  }

  if (report.warnings.size > 0) {
    console.log('\n--- WARNINGS ---');
    for (const [file, warnings] of report.warnings) {
      console.log(`\n${file}:`);
      for (const warning of warnings) {
        console.log(`  ⚠️  ${warning}`);
      }
    }
  }

  console.log('\n--- FORMAT COVERAGE ---');
  for (const [format, count] of report.formatCoverage) {
    console.log(`  ${format}: ${count} files`);
  }

  console.log('\n--- TYPE COVERAGE ---');
  for (const [type, count] of report.typeCoverage) {
    console.log(`  ${type}: ${count} files`);
  }

  console.log('\n' + '='.repeat(70));
}

// CLI entry point for standalone validation
if (import.meta.url === `file://${process.argv[1]}`) {
  const contentDir = process.argv[2] || '../data/lamad/content';

  console.log(`Validating seed files in: ${contentDir}`);

  validateAllContent(contentDir).then(report => {
    printValidationReport(report);
    process.exit(report.invalidFiles > 0 ? 1 : 0);
  });
}
