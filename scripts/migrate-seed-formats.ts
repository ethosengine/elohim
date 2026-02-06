#!/usr/bin/env npx tsx
/**
 * Seed Data Migration Script
 *
 * Migrates invalid contentFormat values to valid ContentFormat types.
 * Also fixes LearningPath schema issues.
 *
 * Run with: npx tsx scripts/migrate-seed-formats.ts [--dry-run]
 */

import * as fs from 'fs';
import * as path from 'path';

const SEED_DIR = path.join(__dirname, '..', 'data', 'lamad');

// Valid ContentFormat values from content-node.model.ts
type ContentFormat =
  | 'markdown'
  | 'html5-app'
  | 'video-embed'
  | 'video-file'
  | 'audio-file'
  | 'quiz-json'
  | 'external-link'
  | 'epub'
  | 'gherkin'
  | 'html'
  | 'plaintext'
  | 'assessment-json';

// Migration mapping for invalid formats
const FORMAT_MIGRATIONS: Record<string, ContentFormat> = {
  'text': 'plaintext',
  'bible-json': 'markdown',
  'book': 'markdown',
  'contributor-json': 'markdown',
  'human-json': 'markdown',
  'structured-json': 'markdown',
  'activity-json': 'markdown',
  'documentary': 'video-embed',
  'video': 'video-embed',
  'audio': 'audio-file',
  'organization-json': 'markdown',
};

// Valid difficulty values for LearningPath
const VALID_DIFFICULTIES = ['beginner', 'intermediate', 'advanced'];

interface MigrationStats {
  filesProcessed: number;
  filesModified: number;
  formatMigrations: Record<string, number>;
  pathFixes: number;
  errors: string[];
}

const stats: MigrationStats = {
  filesProcessed: 0,
  filesModified: 0,
  formatMigrations: {},
  pathFixes: 0,
  errors: [],
};

/**
 * Convert bible-json content object to markdown string
 */
function convertBibleJsonToMarkdown(content: any): string {
  if (typeof content === 'string') return content;

  const { reference, book, chapter, verseStart, verseEnd, contextInModule, translation } = content;

  let markdown = `## ${reference || `${book} ${chapter}:${verseStart}${verseEnd ? `-${verseEnd}` : ''}`}\n\n`;

  if (contextInModule) {
    // Clean up escaped characters
    const cleanContext = contextInModule.replace(/\\-/g, '-').replace(/\\"/g, '"');
    markdown += `> ${cleanContext}\n\n`;
  }

  if (translation) {
    markdown += `*${translation} Translation*\n`;
  }

  return markdown;
}

/**
 * Convert structured JSON content to markdown
 */
function convertStructuredToMarkdown(content: any, title?: string): string {
  if (typeof content === 'string') return content;

  let markdown = '';

  // Handle common patterns
  if (content.name || content.displayName) {
    markdown += `# ${content.name || content.displayName}\n\n`;
  }

  if (content.description || content.bio) {
    markdown += `${content.description || content.bio}\n\n`;
  }

  // Convert remaining fields to a readable format
  const excludeKeys = ['name', 'displayName', 'description', 'bio', 'id'];
  for (const [key, value] of Object.entries(content)) {
    if (excludeKeys.includes(key)) continue;
    if (value === null || value === undefined) continue;

    const label = key.replace(/([A-Z])/g, ' $1').replace(/^./, s => s.toUpperCase());

    if (Array.isArray(value)) {
      markdown += `**${label}:** ${value.join(', ')}\n\n`;
    } else if (typeof value === 'object') {
      markdown += `**${label}:** ${JSON.stringify(value, null, 2)}\n\n`;
    } else {
      markdown += `**${label}:** ${value}\n\n`;
    }
  }

  return markdown || JSON.stringify(content, null, 2);
}

/**
 * Migrate content format and transform content if needed
 */
function migrateContentFormat(data: any): boolean {
  const oldFormat = data.contentFormat;

  if (!oldFormat || !FORMAT_MIGRATIONS[oldFormat]) {
    return false; // Already valid or unknown
  }

  const newFormat = FORMAT_MIGRATIONS[oldFormat];
  data.contentFormat = newFormat;

  // Track migration
  stats.formatMigrations[oldFormat] = (stats.formatMigrations[oldFormat] || 0) + 1;

  // Transform content based on old format
  if (oldFormat === 'bible-json' && typeof data.content === 'object') {
    data.content = convertBibleJsonToMarkdown(data.content);
  } else if (['contributor-json', 'human-json', 'organization-json', 'activity-json', 'structured-json'].includes(oldFormat)) {
    if (typeof data.content === 'object') {
      data.content = convertStructuredToMarkdown(data.content, data.title);
    }
  }

  return true;
}

/**
 * Fix LearningPath schema issues
 */
function fixPathSchema(data: any): boolean {
  let modified = false;

  // Fix invalid difficulty
  if (data.difficulty && !VALID_DIFFICULTIES.includes(data.difficulty)) {
    const oldDifficulty = data.difficulty;

    // Map common invalid values
    if (oldDifficulty.includes('beginner') && oldDifficulty.includes('intermediate')) {
      data.difficulty = 'intermediate'; // beginner-to-intermediate -> intermediate
    } else if (oldDifficulty.includes('intermediate') && oldDifficulty.includes('advanced')) {
      data.difficulty = 'advanced';
    } else if (oldDifficulty.includes('beginner')) {
      data.difficulty = 'beginner';
    } else {
      data.difficulty = 'intermediate'; // default
    }

    console.log(`  Fixed difficulty: "${oldDifficulty}" -> "${data.difficulty}"`);
    modified = true;
    stats.pathFixes++;
  }

  // Move non-schema fields to metadata
  const nonSchemaFields = ['audienceArchetype', 'contributorPresences'];
  for (const field of nonSchemaFields) {
    if (data[field] !== undefined) {
      data.metadata = data.metadata || {};
      data.metadata[field] = data[field];
      delete data[field];
      console.log(`  Moved ${field} to metadata`);
      modified = true;
      stats.pathFixes++;
    }
  }

  return modified;
}

/**
 * Process a single JSON file
 */
function processFile(filePath: string, dryRun: boolean): void {
  stats.filesProcessed++;

  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const data = JSON.parse(content);

    let modified = false;

    // Check if it's a content node (has contentFormat)
    if (data.contentFormat) {
      modified = migrateContentFormat(data) || modified;
    }

    // Check if it's a learning path (has steps or chapters)
    if (data.steps || data.chapters) {
      modified = fixPathSchema(data) || modified;
    }

    if (modified) {
      stats.filesModified++;

      // Update timestamp
      data.updatedAt = new Date().toISOString();

      if (!dryRun) {
        fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
        console.log(`✓ Updated: ${path.relative(SEED_DIR, filePath)}`);
      } else {
        console.log(`[DRY RUN] Would update: ${path.relative(SEED_DIR, filePath)}`);
      }
    }
  } catch (error: any) {
    stats.errors.push(`${filePath}: ${error.message}`);
  }
}

/**
 * Recursively find all JSON files in directory
 */
function findJsonFiles(dir: string): string[] {
  const files: string[] = [];

  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      files.push(...findJsonFiles(fullPath));
    } else if (entry.isFile() && entry.name.endsWith('.json') && entry.name !== 'index.json') {
      files.push(fullPath);
    }
  }

  return files;
}

/**
 * Main migration function
 */
function migrate(dryRun: boolean): void {
  console.log(`\n${'='.repeat(60)}`);
  console.log(`Seed Data Migration${dryRun ? ' (DRY RUN)' : ''}`);
  console.log(`${'='.repeat(60)}\n`);

  console.log(`Scanning: ${SEED_DIR}\n`);

  const files = findJsonFiles(SEED_DIR);
  console.log(`Found ${files.length} JSON files to process\n`);

  for (const file of files) {
    processFile(file, dryRun);
  }

  // Print summary
  console.log(`\n${'='.repeat(60)}`);
  console.log('Migration Summary');
  console.log(`${'='.repeat(60)}\n`);

  console.log(`Files processed: ${stats.filesProcessed}`);
  console.log(`Files modified: ${stats.filesModified}`);
  console.log(`Path fixes: ${stats.pathFixes}`);

  if (Object.keys(stats.formatMigrations).length > 0) {
    console.log('\nFormat migrations:');
    for (const [format, count] of Object.entries(stats.formatMigrations)) {
      console.log(`  ${format} -> ${FORMAT_MIGRATIONS[format]}: ${count}`);
    }
  }

  if (stats.errors.length > 0) {
    console.log('\nErrors:');
    for (const error of stats.errors) {
      console.log(`  ❌ ${error}`);
    }
  }

  console.log(`\n${dryRun ? 'Run without --dry-run to apply changes.' : 'Migration complete!'}\n`);
}

// Run migration
const dryRun = process.argv.includes('--dry-run');
migrate(dryRun);
