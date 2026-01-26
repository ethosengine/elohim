#!/usr/bin/env node
/**
 * fetch-bdd-features.js
 *
 * Fetches Gherkin feature files from a running Elohim instance
 * for the BDD product test pipeline (dogfooding approach).
 *
 * Architecture:
 *   1. Query the content graph for the test path
 *   2. Get ordered list of feature node IDs
 *   3. Fetch each feature's content from blob store
 *   4. Write .feature files to output directory
 *
 * Usage:
 *   node scripts/fetch-bdd-features.js \
 *     --doorway-url https://doorway-dev.elohim.host \
 *     --test-path-id bdd-smoke-tests \
 *     --output-dir cypress/e2e/features/dynamic \
 *     --tags @smoke,@lamad
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const http = require('http');

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_CONFIG = {
  doorwayUrl: process.env.DOORWAY_HOST || 'https://doorway-dev.elohim.host',
  testPathId: 'bdd-smoke-tests',
  outputDir: 'cypress/e2e/features/dynamic',
  tags: '',
  dryRun: false,
  verbose: false,
};

// ============================================================================
// Argument Parsing
// ============================================================================

function parseArgs() {
  const args = { ...DEFAULT_CONFIG };

  for (let i = 2; i < process.argv.length; i++) {
    const arg = process.argv[i];
    const value = process.argv[i + 1];

    switch (arg) {
      case '--doorway-url':
        args.doorwayUrl = value;
        i++;
        break;
      case '--test-path-id':
        args.testPathId = value;
        i++;
        break;
      case '--output-dir':
        args.outputDir = value;
        i++;
        break;
      case '--tags':
        args.tags = value;
        i++;
        break;
      case '--dry-run':
        args.dryRun = true;
        break;
      case '--verbose':
        args.verbose = true;
        break;
      case '--help':
        showHelp();
        process.exit(0);
    }
  }

  return args;
}

function showHelp() {
  console.log(`
fetch-bdd-features.js - Fetch Gherkin features from Elohim content graph

USAGE:
  node scripts/fetch-bdd-features.js [options]

OPTIONS:
  --doorway-url <url>    Doorway API URL
                         Default: DOORWAY_HOST env or https://doorway-dev.elohim.host
  --test-path-id <id>    ID of the test path in the content graph
                         Default: bdd-smoke-tests
  --output-dir <dir>     Output directory for .feature files
                         Default: cypress/e2e/features/dynamic
  --tags <tags>          Filter by tags, comma-separated (e.g., @smoke,@lamad)
  --dry-run              List features without downloading
  --verbose              Show detailed output
  --help                 Show this help message

ENVIRONMENT VARIABLES:
  DOORWAY_HOST           Default Doorway API URL
  BDD_AUTH_TOKEN         Authentication token (if required)

EXAMPLES:
  # Fetch all features from smoke test path
  node scripts/fetch-bdd-features.js --test-path-id bdd-smoke-tests

  # Fetch only @lamad tagged features
  node scripts/fetch-bdd-features.js --tags @lamad

  # Dry run to see what would be fetched
  node scripts/fetch-bdd-features.js --dry-run --verbose
`);
}

// ============================================================================
// HTTP Helpers
// ============================================================================

function fetch(url, options = {}) {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith('https') ? https : http;

    const headers = {
      Accept: options.accept || 'application/json',
      'User-Agent': 'elohim-bdd-fetcher/1.0',
    };

    if (process.env.BDD_AUTH_TOKEN) {
      headers.Authorization = `Bearer ${process.env.BDD_AUTH_TOKEN}`;
    }

    const req = protocol.get(url, { headers }, (res) => {
      let data = '';
      res.on('data', (chunk) => (data += chunk));
      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          if (options.accept === 'text/plain') {
            resolve(data);
          } else {
            try {
              resolve(JSON.parse(data));
            } catch (e) {
              resolve(data);
            }
          }
        } else {
          reject(new Error(`HTTP ${res.statusCode}: ${data.substring(0, 200)}`));
        }
      });
    });

    req.on('error', reject);
    req.setTimeout(30000, () => {
      req.destroy();
      reject(new Error('Request timeout'));
    });
  });
}

// ============================================================================
// Content Fetching
// ============================================================================

/**
 * Fetch the test path from the content graph
 */
async function fetchTestPath(doorwayUrl, testPathId, verbose) {
  // Try to get path via the projection API
  const pathUrl = `${doorwayUrl}/api/v1/lamad_dna/content_store/get_path_with_steps`;

  if (verbose) {
    console.log(`  Fetching test path: ${pathUrl}?path_id=${testPathId}`);
  }

  try {
    const path = await fetch(`${pathUrl}?path_id=${testPathId}`);
    return path;
  } catch (error) {
    // If path API fails, try getting content by ID
    if (verbose) {
      console.log(`  Path API failed, trying content by ID...`);
    }

    const contentUrl = `${doorwayUrl}/api/v1/lamad_dna/content_store/get_content_by_id`;
    const content = await fetch(`${contentUrl}?id=${testPathId}`);

    // If it's a path-type content, extract the related nodes
    if (content && (content.content_type === 'path' || content.contentType === 'path')) {
      return {
        id: content.id,
        steps: (content.relatedNodeIds || content.related_node_ids || []).map((id) => ({
          resourceId: id,
        })),
      };
    }

    throw new Error(`Could not find test path: ${testPathId}`);
  }
}

/**
 * Fetch content node by ID
 */
async function fetchContentNode(doorwayUrl, nodeId, verbose) {
  const url = `${doorwayUrl}/api/v1/lamad_dna/content_store/get_content_by_id?id=${encodeURIComponent(nodeId)}`;

  if (verbose) {
    console.log(`  Fetching content: ${nodeId}`);
  }

  return await fetch(url);
}

/**
 * Fetch raw content from blob store
 */
async function fetchBlobContent(doorwayUrl, contentHash, verbose) {
  const url = `${doorwayUrl}/store/${contentHash}`;

  if (verbose) {
    console.log(`  Fetching blob: ${contentHash.substring(0, 16)}...`);
  }

  return await fetch(url, { accept: 'text/plain' });
}

/**
 * Extract Gherkin content from a content node
 */
function extractGherkinContent(node) {
  // Direct content field
  if (node.content && typeof node.content === 'string') {
    if (node.content.includes('Feature:')) {
      return node.content;
    }
  }

  // Content might be in a nested structure
  if (node.content && typeof node.content === 'object' && node.content.gherkin) {
    return node.content.gherkin;
  }

  return null;
}

/**
 * Check if a node is a Gherkin feature
 */
function isGherkinFeature(node) {
  const format = node.content_format || node.contentFormat;
  const type = node.content_type || node.contentType;

  return format === 'gherkin' || type === 'feature' || type === 'scenario';
}

/**
 * Filter nodes by tags
 */
function matchesTags(node, filterTags) {
  if (!filterTags || filterTags.length === 0) {
    return true;
  }

  const nodeTags = (node.tags || []).map((t) => t.toLowerCase().replace(/^@/, ''));

  return filterTags.some((filterTag) => {
    const normalizedFilter = filterTag.toLowerCase().replace(/^@/, '');
    return nodeTags.some((nodeTag) => nodeTag.includes(normalizedFilter));
  });
}

// ============================================================================
// File Output
// ============================================================================

/**
 * Sanitize filename
 */
function sanitizeFilename(name) {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Categorize feature for subdirectory
 */
function categorizeFeature(node) {
  const tags = (node.tags || []).map((t) => t.toLowerCase());

  // Check for explicit category tags
  for (const tag of tags) {
    if (tag.startsWith('@category:') || tag.startsWith('category:')) {
      return tag.replace(/^@?category:/, '');
    }
    if (tag.startsWith('@epic:') || tag.startsWith('epic:')) {
      return tag.replace(/^@?epic:/, '');
    }
  }

  // Infer from product tags
  if (tags.some((t) => t.includes('lamad'))) return 'lamad';
  if (tags.some((t) => t.includes('shefa'))) return 'shefa';
  if (tags.some((t) => t.includes('imagodei'))) return 'imagodei';
  if (tags.some((t) => t.includes('doorway'))) return 'doorway';
  if (tags.some((t) => t.includes('navigation'))) return 'navigation';
  if (tags.some((t) => t.includes('smoke'))) return 'smoke';

  return 'general';
}

/**
 * Write feature file to disk
 */
function writeFeatureFile(outputDir, node, content) {
  const category = categorizeFeature(node);
  const categoryDir = path.join(outputDir, category);

  fs.mkdirSync(categoryDir, { recursive: true });

  const nodeId = node.id || node.title || 'unnamed';
  const filename = `${sanitizeFilename(nodeId)}.feature`;
  const filepath = path.join(categoryDir, filename);

  fs.writeFileSync(filepath, content, 'utf-8');

  return { category, filename, filepath };
}

// ============================================================================
// Main
// ============================================================================

async function main() {
  const args = parseArgs();

  console.log('═'.repeat(60));
  console.log('BDD Feature Fetcher');
  console.log('═'.repeat(60));
  console.log(`Doorway URL: ${args.doorwayUrl}`);
  console.log(`Test Path: ${args.testPathId}`);
  console.log(`Output: ${args.outputDir}`);
  console.log(`Tags: ${args.tags || '(all)'}`);
  console.log(`Dry Run: ${args.dryRun}`);
  console.log('═'.repeat(60));

  // Parse filter tags
  const filterTags = args.tags
    ? args.tags.split(',').map((t) => t.trim())
    : [];

  // Step 1: Fetch the test path
  console.log('\n📋 Fetching test path...');
  let testPath;
  try {
    testPath = await fetchTestPath(args.doorwayUrl, args.testPathId, args.verbose);
    console.log(`  ✓ Found test path with ${testPath.steps?.length || 0} steps`);
  } catch (error) {
    console.error(`  ✗ Failed to fetch test path: ${error.message}`);
    process.exit(1);
  }

  // Step 2: Get feature node IDs from the path
  const featureIds = [];

  // Handle different path structures
  if (testPath.steps) {
    for (const step of testPath.steps) {
      if (step.resourceId) {
        featureIds.push(step.resourceId);
      }
    }
  }

  // Handle chapters/modules/sections structure
  if (testPath.chapters) {
    for (const chapter of testPath.chapters) {
      if (chapter.modules) {
        for (const module of chapter.modules) {
          if (module.sections) {
            for (const section of module.sections) {
              if (section.conceptIds) {
                featureIds.push(...section.conceptIds);
              }
            }
          }
        }
      }
      if (chapter.steps) {
        for (const step of chapter.steps) {
          if (step.resourceId) {
            featureIds.push(step.resourceId);
          }
        }
      }
    }
  }

  if (featureIds.length === 0) {
    console.log('  ⚠️ No feature IDs found in test path');
    process.exit(0);
  }

  console.log(`  Found ${featureIds.length} potential feature IDs`);

  // Step 3: Fetch each feature and filter
  console.log('\n📥 Fetching features...');
  const features = [];

  for (const featureId of featureIds) {
    try {
      const node = await fetchContentNode(args.doorwayUrl, featureId, args.verbose);

      if (!isGherkinFeature(node)) {
        if (args.verbose) {
          console.log(`  ⏭️ Skipping non-Gherkin: ${featureId}`);
        }
        continue;
      }

      if (!matchesTags(node, filterTags)) {
        if (args.verbose) {
          console.log(`  ⏭️ Skipping (tag mismatch): ${featureId}`);
        }
        continue;
      }

      // Try to get Gherkin content
      let gherkinContent = extractGherkinContent(node);

      // If content has a hash, try blob store
      if (!gherkinContent && (node.content_hash || node.contentHash)) {
        const hash = node.content_hash || node.contentHash;
        try {
          gherkinContent = await fetchBlobContent(args.doorwayUrl, hash, args.verbose);
        } catch (e) {
          console.log(`  ⚠️ Could not fetch blob for ${featureId}: ${e.message}`);
        }
      }

      if (!gherkinContent || !gherkinContent.includes('Feature:')) {
        if (args.verbose) {
          console.log(`  ⚠️ No valid Gherkin content for: ${featureId}`);
        }
        continue;
      }

      features.push({ node, content: gherkinContent });
      console.log(`  ✓ ${featureId} (${node.title || 'untitled'})`);
    } catch (error) {
      console.log(`  ✗ Failed to fetch ${featureId}: ${error.message}`);
    }
  }

  console.log(`\n📊 Found ${features.length} valid features`);

  if (features.length === 0) {
    console.log('  No features to write');
    process.exit(0);
  }

  if (args.dryRun) {
    console.log('\n🔍 Dry run - features that would be written:');
    for (const { node } of features) {
      const category = categorizeFeature(node);
      console.log(`  - ${category}/${sanitizeFilename(node.id || node.title)}.feature`);
    }
    process.exit(0);
  }

  // Step 4: Write feature files
  console.log('\n📝 Writing feature files...');

  // Ensure output directory exists
  fs.mkdirSync(args.outputDir, { recursive: true });

  // Clear existing dynamic features
  const existingFiles = fs
    .readdirSync(args.outputDir, { recursive: true })
    .filter((f) => f.toString().endsWith('.feature'));

  if (existingFiles.length > 0) {
    for (const file of existingFiles) {
      const fullPath = path.join(args.outputDir, file.toString());
      if (fs.existsSync(fullPath) && fs.statSync(fullPath).isFile()) {
        fs.unlinkSync(fullPath);
      }
    }
    console.log(`  Cleared ${existingFiles.length} existing feature files`);
  }

  // Write new features
  let successCount = 0;
  for (const { node, content } of features) {
    try {
      const result = writeFeatureFile(args.outputDir, node, content);
      console.log(`  ✓ ${result.category}/${result.filename}`);
      successCount++;
    } catch (error) {
      console.log(`  ✗ Failed to write ${node.id}: ${error.message}`);
    }
  }

  // Create manifest
  const manifest = {
    fetchedAt: new Date().toISOString(),
    doorwayUrl: args.doorwayUrl,
    testPathId: args.testPathId,
    tags: args.tags || null,
    featureCount: successCount,
    features: features.map(({ node }) => ({
      id: node.id,
      title: node.title,
      category: categorizeFeature(node),
    })),
  };

  fs.writeFileSync(
    path.join(args.outputDir, 'manifest.json'),
    JSON.stringify(manifest, null, 2)
  );

  console.log('\n═'.repeat(60));
  console.log(`✅ Completed: ${successCount} features written`);
  console.log('═'.repeat(60));
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
