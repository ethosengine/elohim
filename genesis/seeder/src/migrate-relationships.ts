#!/usr/bin/env npx tsx
/**
 * Relationship Migration Script
 *
 * Migrates existing content data to the new relationship-focused tables:
 * - Extracts relationships from content.relatedNodeIds[]
 * - Infers CONTAINS relationships from path structure
 * - Creates ContributorPresence records from author metadata
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 npx tsx src/migrate-relationships.ts
 *
 * Options:
 *   --dry-run     Preview changes without writing (default)
 *   --commit      Actually write the migrations
 *   --skip-paths  Skip path structure inference
 *   --verbose     Show detailed progress
 */

import * as fs from 'fs';
import * as path from 'path';

// Configuration
const STORAGE_URL = process.env.STORAGE_URL || 'http://localhost:8090';
const DATA_DIR = process.env.DATA_DIR || path.join(__dirname, '../../data/lamad');
const DRY_RUN = !process.argv.includes('--commit');
const SKIP_PATHS = process.argv.includes('--skip-paths');
const VERBOSE = process.argv.includes('--verbose');

// Counters for reporting
let relationshipsCreated = 0;
let presencesCreated = 0;
let pathRelationshipsCreated = 0;
let errors: string[] = [];

/**
 * Content node as stored in JSON files
 */
interface ContentNode {
  id: string;
  title?: string;
  contentType?: string;
  author?: string | { name: string; id?: string };
  contributors?: Array<string | { name: string; id?: string }>;
  relatedNodeIds?: string[];
  metadata?: Record<string, unknown>;
}

/**
 * Path structure
 */
interface LearningPath {
  id: string;
  title?: string;
  chapters?: PathChapter[];
  steps?: PathStep[];
}

interface PathChapter {
  id: string;
  title?: string;
  modules?: PathModule[];
  conceptIds?: string[];
}

interface PathModule {
  id: string;
  title?: string;
  sections?: PathSection[];
  conceptIds?: string[];
}

interface PathSection {
  id: string;
  title?: string;
  conceptIds?: string[];
}

interface PathStep {
  resourceId: string;
  order: number;
}

/**
 * Relationship type mappings based on content relationships
 */
const RELATIONSHIP_TYPE_MAP: Record<string, string> = {
  'requires': 'REQUIRES',
  'extends': 'EXTENDS',
  'references': 'REFERENCES',
  'relates': 'RELATES_TO',
  'contains': 'CONTAINS',
  'default': 'RELATES_TO',
};

/**
 * Main entry point
 */
async function main() {
  console.log('='.repeat(60));
  console.log('Relationship Migration Script');
  console.log('='.repeat(60));
  console.log(`Storage URL: ${STORAGE_URL}`);
  console.log(`Data Directory: ${DATA_DIR}`);
  console.log(`Mode: ${DRY_RUN ? 'DRY RUN (preview only)' : 'COMMIT (writing changes)'}`);
  console.log('');

  try {
    // Check storage server health
    await checkStorageHealth();

    // Phase 1: Migrate content relationships
    console.log('\n--- Phase 1: Content Relationships ---');
    await migrateContentRelationships();

    // Phase 2: Infer path structure relationships
    if (!SKIP_PATHS) {
      console.log('\n--- Phase 2: Path Structure Relationships ---');
      await migratePathRelationships();
    }

    // Phase 3: Create contributor presences
    console.log('\n--- Phase 3: Contributor Presences ---');
    await createContributorPresences();

    // Summary
    printSummary();

  } catch (err) {
    console.error('\nFATAL ERROR:', err);
    process.exit(1);
  }
}

/**
 * Check if storage server is available
 */
async function checkStorageHealth() {
  try {
    const response = await fetch(`${STORAGE_URL}/health`);
    if (!response.ok) {
      throw new Error(`Health check failed: ${response.status}`);
    }
    console.log('Storage server health: OK');
  } catch (err) {
    throw new Error(`Cannot connect to storage server at ${STORAGE_URL}: ${err}`);
  }
}

/**
 * Phase 1: Migrate relationships from content.relatedNodeIds[]
 */
async function migrateContentRelationships() {
  const contentDir = path.join(DATA_DIR, 'content');

  if (!fs.existsSync(contentDir)) {
    console.log(`Content directory not found: ${contentDir}`);
    return;
  }

  const files = fs.readdirSync(contentDir).filter(f => f.endsWith('.json'));
  console.log(`Found ${files.length} content files`);

  for (const file of files) {
    try {
      const filePath = path.join(contentDir, file);
      const content: ContentNode = JSON.parse(fs.readFileSync(filePath, 'utf-8'));

      if (!content.relatedNodeIds || content.relatedNodeIds.length === 0) {
        continue;
      }

      if (VERBOSE) {
        console.log(`  Processing ${content.id}: ${content.relatedNodeIds.length} relationships`);
      }

      for (const targetId of content.relatedNodeIds) {
        await createRelationship({
          sourceId: content.id,
          targetId,
          relationshipType: 'RELATES_TO',
          confidence: 1.0,
          inferenceSource: 'author',
        });
        relationshipsCreated++;
      }
    } catch (err) {
      errors.push(`Error processing ${file}: ${err}`);
    }
  }
}

/**
 * Phase 2: Infer CONTAINS relationships from path structure
 */
async function migratePathRelationships() {
  const pathsDir = path.join(DATA_DIR, 'paths');

  if (!fs.existsSync(pathsDir)) {
    console.log(`Paths directory not found: ${pathsDir}`);
    return;
  }

  const files = fs.readdirSync(pathsDir).filter(f => f.endsWith('.json') && f !== 'index.json');
  console.log(`Found ${files.length} path files`);

  for (const file of files) {
    try {
      const filePath = path.join(pathsDir, file);
      const pathData: LearningPath = JSON.parse(fs.readFileSync(filePath, 'utf-8'));

      if (VERBOSE) {
        console.log(`  Processing path: ${pathData.id}`);
      }

      // Process chapters → modules → sections hierarchy
      if (pathData.chapters) {
        for (const chapter of pathData.chapters) {
          // Path → Chapter (CONTAINS)
          await createRelationship({
            sourceId: pathData.id,
            targetId: chapter.id,
            relationshipType: 'CONTAINS',
            confidence: 1.0,
            inferenceSource: 'structural',
          });
          pathRelationshipsCreated++;

          // Process modules
          if (chapter.modules) {
            for (const module of chapter.modules) {
              // Chapter → Module (CONTAINS)
              await createRelationship({
                sourceId: chapter.id,
                targetId: module.id,
                relationshipType: 'CONTAINS',
                confidence: 1.0,
                inferenceSource: 'structural',
              });
              pathRelationshipsCreated++;

              // Process sections
              if (module.sections) {
                for (const section of module.sections) {
                  // Module → Section (CONTAINS)
                  await createRelationship({
                    sourceId: module.id,
                    targetId: section.id,
                    relationshipType: 'CONTAINS',
                    confidence: 1.0,
                    inferenceSource: 'structural',
                  });
                  pathRelationshipsCreated++;

                  // Section → Concepts (CONTAINS)
                  if (section.conceptIds) {
                    for (const conceptId of section.conceptIds) {
                      await createRelationship({
                        sourceId: section.id,
                        targetId: conceptId,
                        relationshipType: 'CONTAINS',
                        confidence: 1.0,
                        inferenceSource: 'structural',
                      });
                      pathRelationshipsCreated++;
                    }
                  }
                }
              }

              // Module's direct conceptIds
              if (module.conceptIds) {
                for (const conceptId of module.conceptIds) {
                  await createRelationship({
                    sourceId: module.id,
                    targetId: conceptId,
                    relationshipType: 'CONTAINS',
                    confidence: 1.0,
                    inferenceSource: 'structural',
                  });
                  pathRelationshipsCreated++;
                }
              }
            }
          }

          // Chapter's direct conceptIds
          if (chapter.conceptIds) {
            for (const conceptId of chapter.conceptIds) {
              await createRelationship({
                sourceId: chapter.id,
                targetId: conceptId,
                relationshipType: 'CONTAINS',
                confidence: 1.0,
                inferenceSource: 'structural',
              });
              pathRelationshipsCreated++;
            }
          }
        }
      }

      // Process flat steps (legacy format)
      if (pathData.steps) {
        for (let i = 0; i < pathData.steps.length; i++) {
          const step = pathData.steps[i];
          // Path → Step content (CONTAINS)
          await createRelationship({
            sourceId: pathData.id,
            targetId: step.resourceId,
            relationshipType: 'CONTAINS',
            confidence: 1.0,
            inferenceSource: 'structural',
            metadata: { order: step.order }
          });
          pathRelationshipsCreated++;

          // Step → Next Step (NEXT)
          if (i < pathData.steps.length - 1) {
            await createRelationship({
              sourceId: step.resourceId,
              targetId: pathData.steps[i + 1].resourceId,
              relationshipType: 'NEXT',
              confidence: 1.0,
              inferenceSource: 'structural',
            });
            pathRelationshipsCreated++;
          }
        }
      }

    } catch (err) {
      errors.push(`Error processing path ${file}: ${err}`);
    }
  }
}

/**
 * Phase 3: Create contributor presences from author metadata
 */
async function createContributorPresences() {
  const contentDir = path.join(DATA_DIR, 'content');

  if (!fs.existsSync(contentDir)) {
    console.log(`Content directory not found: ${contentDir}`);
    return;
  }

  // Collect unique authors and their content
  const authorContent = new Map<string, string[]>();

  const files = fs.readdirSync(contentDir).filter(f => f.endsWith('.json'));

  for (const file of files) {
    try {
      const filePath = path.join(contentDir, file);
      const content: ContentNode = JSON.parse(fs.readFileSync(filePath, 'utf-8'));

      // Extract author name
      let authorName: string | null = null;
      if (content.author) {
        authorName = typeof content.author === 'string'
          ? content.author
          : content.author.name;
      }

      if (authorName) {
        const existing = authorContent.get(authorName) || [];
        existing.push(content.id);
        authorContent.set(authorName, existing);
      }

      // Also process contributors
      if (content.contributors) {
        for (const contributor of content.contributors) {
          const name = typeof contributor === 'string'
            ? contributor
            : contributor.name;
          if (name) {
            const existing = authorContent.get(name) || [];
            if (!existing.includes(content.id)) {
              existing.push(content.id);
            }
            authorContent.set(name, existing);
          }
        }
      }
    } catch (err) {
      errors.push(`Error extracting authors from ${file}: ${err}`);
    }
  }

  console.log(`Found ${authorContent.size} unique contributors`);

  // Create presence records
  for (const [authorName, contentIds] of authorContent) {
    if (VERBOSE) {
      console.log(`  Creating presence for "${authorName}" with ${contentIds.length} content items`);
    }

    await createPresence({
      displayName: authorName,
      establishingContentIds: contentIds.slice(0, 10), // Limit to first 10 for establishing
    });
    presencesCreated++;
  }
}

/**
 * Create a relationship record
 */
async function createRelationship(input: {
  sourceId: string;
  targetId: string;
  relationshipType: string;
  confidence: number;
  inferenceSource: string;
  createInverse?: boolean;
  metadata?: Record<string, unknown>;
}) {
  if (DRY_RUN) {
    if (VERBOSE) {
      console.log(`    [DRY RUN] Would create: ${input.sourceId} --[${input.relationshipType}]--> ${input.targetId}`);
    }
    return;
  }

  try {
    const response = await fetch(`${STORAGE_URL}/db/relationships?app_id=lamad`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        source_id: input.sourceId,
        target_id: input.targetId,
        relationship_type: input.relationshipType,
        confidence: input.confidence,
        inference_source: input.inferenceSource,
        create_inverse: input.createInverse || false,
        metadata_json: input.metadata ? JSON.stringify(input.metadata) : null,
      }),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`HTTP ${response.status}: ${text}`);
    }
  } catch (err) {
    errors.push(`Failed to create relationship ${input.sourceId} -> ${input.targetId}: ${err}`);
  }
}

/**
 * Create a contributor presence record
 */
async function createPresence(input: {
  displayName: string;
  establishingContentIds: string[];
}) {
  if (DRY_RUN) {
    if (VERBOSE) {
      console.log(`    [DRY RUN] Would create presence: "${input.displayName}"`);
    }
    return;
  }

  try {
    const response = await fetch(`${STORAGE_URL}/db/presences?app_id=lamad`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        display_name: input.displayName,
        establishing_content_ids_json: JSON.stringify(input.establishingContentIds),
      }),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`HTTP ${response.status}: ${text}`);
    }
  } catch (err) {
    errors.push(`Failed to create presence "${input.displayName}": ${err}`);
  }
}

/**
 * Print migration summary
 */
function printSummary() {
  console.log('\n' + '='.repeat(60));
  console.log('Migration Summary');
  console.log('='.repeat(60));
  console.log(`Mode: ${DRY_RUN ? 'DRY RUN (no changes written)' : 'COMMITTED'}`);
  console.log('');
  console.log('Records processed:');
  console.log(`  Content relationships: ${relationshipsCreated}`);
  console.log(`  Path structure relationships: ${pathRelationshipsCreated}`);
  console.log(`  Contributor presences: ${presencesCreated}`);
  console.log(`  Total relationships: ${relationshipsCreated + pathRelationshipsCreated}`);
  console.log('');

  if (errors.length > 0) {
    console.log(`Errors encountered: ${errors.length}`);
    for (const err of errors.slice(0, 10)) {
      console.log(`  - ${err}`);
    }
    if (errors.length > 10) {
      console.log(`  ... and ${errors.length - 10} more errors`);
    }
  } else {
    console.log('No errors encountered.');
  }

  console.log('');
  if (DRY_RUN) {
    console.log('To apply these changes, run with --commit flag:');
    console.log('  STORAGE_URL=http://localhost:8090 npx tsx src/migrate-relationships.ts --commit');
  }
}

// Run the migration
main().catch(err => {
  console.error('Unhandled error:', err);
  process.exit(1);
});
