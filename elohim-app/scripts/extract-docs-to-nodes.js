#!/usr/bin/env node

/**
 * Extract Documentation to Content Nodes
 *
 * Scans the /docs directory and generates a structured JSON output
 * that can be imported into the Lamad learning platform node graph.
 *
 * Output: A JSON file containing all ContentNodes with relationships,
 * metadata, and taxonomy information for systematic import.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

// Configuration
const DOCS_ROOT = path.resolve(__dirname, '../../docs');
const OUTPUT_FILE = path.resolve(__dirname, '../src/app/lamad/data/content-nodes.json');

// Content type mappings based on directory structure
const EPIC_NAMES = {
  'autonomous_entity': 'Autonomous Entity',
  'governance': 'Governance',
  'governance_layers': 'Governance Layers',
  'public_observer': 'Public Observer',
  'social_medium': 'Social Medium',
  'value_scanner': 'Value Scanner'
};

// Node type hierarchy
const NODE_TYPES = {
  EPIC: 'epic',
  PERSONA: 'persona',
  SCENARIO: 'scenario',
  ARTICLE: 'article',
  BOOK: 'book',
  VIDEO: 'video',
  AUDIO: 'audio',
  DOCUMENT: 'document',
  ORGANIZATION: 'organization',
  ROOT_DOC: 'root-document'
};

// Special directories that contain reference materials
const REFERENCE_TYPES = {
  'organizations': NODE_TYPES.ORGANIZATION,
  'books': NODE_TYPES.BOOK,
  'video': NODE_TYPES.VIDEO,
  'audio': NODE_TYPES.AUDIO,
  'articles': NODE_TYPES.ARTICLE,
  'documents': NODE_TYPES.DOCUMENT
};

/**
 * Generate a deterministic ID from a file path
 */
function generateId(filePath) {
  const relativePath = path.relative(DOCS_ROOT, filePath);
  const hash = crypto.createHash('md5').update(relativePath).digest('hex').substring(0, 12);
  const slugified = relativePath
    .replace(/\.(md|feature)$/, '')
    .replace(/[^a-z0-9]+/gi, '-')
    .toLowerCase()
    .replace(/^-|-$/g, '');
  return `${slugified}-${hash}`;
}

/**
 * Extract title from markdown or feature file
 */
function extractTitle(content, fileName, contentFormat) {
  if (contentFormat === 'markdown') {
    // Look for first # heading
    const match = content.match(/^#\s+(.+)$/m);
    if (match) return match[1].trim();
  } else if (contentFormat === 'gherkin') {
    // Look for Feature: line
    const match = content.match(/^Feature:\s+(.+)$/m);
    if (match) return match[1].trim();
  }

  // Fallback to filename
  return fileName
    .replace(/\.(md|feature)$/, '')
    .replace(/[_-]/g, ' ')
    .replace(/\b\w/g, l => l.toUpperCase());
}

/**
 * Extract description from content
 */
function extractDescription(content, contentFormat) {
  if (contentFormat === 'markdown') {
    // Get text after first heading, before next heading or first 200 chars
    const match = content.match(/^#\s+.+$\n\n([\s\S]+?)(?=\n#|\n\n#|$)/m);
    if (match) {
      return match[1].trim().substring(0, 300);
    }
  } else if (contentFormat === 'gherkin') {
    // Get description lines after Feature:
    const lines = content.split('\n');
    const featureIndex = lines.findIndex(line => line.trim().startsWith('Feature:'));
    if (featureIndex >= 0) {
      const descLines = [];
      for (let i = featureIndex + 1; i < lines.length; i++) {
        const line = lines[i].trim();
        if (line.startsWith('Background:') || line.startsWith('Scenario:') || line.startsWith('@')) {
          break;
        }
        if (line) descLines.push(line);
      }
      return descLines.join(' ').substring(0, 300);
    }
  }

  // Fallback to first 200 chars
  return content.substring(0, 200).trim();
}

/**
 * Extract tags from file path and content
 */
function extractTags(filePath, content) {
  const tags = [];
  const relativePath = path.relative(DOCS_ROOT, filePath);
  const pathParts = relativePath.split(path.sep);

  // Add epic as tag
  if (pathParts[0] && EPIC_NAMES[pathParts[0]]) {
    tags.push(pathParts[0]);
  }

  // Add persona/role as tag
  if (pathParts.length > 1 && !REFERENCE_TYPES[pathParts[1]]) {
    tags.push(pathParts[1]);
  }

  // Extract @tags from Gherkin files
  if (filePath.endsWith('.feature')) {
    const tagMatches = content.match(/@[\w-]+/g);
    if (tagMatches) {
      tags.push(...tagMatches.map(t => t.substring(1)));
    }
  }

  return [...new Set(tags)]; // Remove duplicates
}

/**
 * Determine content type from file path
 */
function determineContentType(filePath) {
  const relativePath = path.relative(DOCS_ROOT, filePath);
  const pathParts = relativePath.split(path.sep);

  // Check if it's a root-level document
  if (pathParts.length === 1) {
    return NODE_TYPES.ROOT_DOC;
  }

  // Check for reference type directories
  for (let i = 0; i < pathParts.length; i++) {
    if (REFERENCE_TYPES[pathParts[i]]) {
      return REFERENCE_TYPES[pathParts[i]];
    }
  }

  // Check if it's a scenario (in scenarios directory or .feature file)
  if (pathParts.includes('scenarios') || filePath.endsWith('.feature')) {
    return NODE_TYPES.SCENARIO;
  }

  // Check if it's a persona (second level in epic, not a reference type)
  if (pathParts.length >= 2 && EPIC_NAMES[pathParts[0]]) {
    return NODE_TYPES.PERSONA;
  }

  // Default to document
  return NODE_TYPES.DOCUMENT;
}

/**
 * Determine category from file path
 */
function determineCategory(filePath) {
  const relativePath = path.relative(DOCS_ROOT, filePath);
  const pathParts = relativePath.split(path.sep);

  // Primary category is the epic
  if (pathParts[0] && EPIC_NAMES[pathParts[0]]) {
    return EPIC_NAMES[pathParts[0]];
  }

  return 'General';
}

/**
 * Extract metadata from file path and content
 */
function extractMetadata(filePath, content, contentType) {
  const relativePath = path.relative(DOCS_ROOT, filePath);
  const pathParts = relativePath.split(path.sep);
  const metadata = {
    sourcePath: relativePath,
    epic: null,
    persona: null,
    referenceType: null,
    layer: null,
    layerType: null,
    priority: 0
  };

  // Determine epic
  if (pathParts[0] && EPIC_NAMES[pathParts[0]]) {
    metadata.epic = pathParts[0];
  }

  // Determine persona (if not a reference type)
  if (pathParts.length > 1 && !REFERENCE_TYPES[pathParts[1]]) {
    metadata.persona = pathParts[1];
  }

  // Reference type
  for (const [dir, type] of Object.entries(REFERENCE_TYPES)) {
    if (pathParts.includes(dir)) {
      metadata.referenceType = type;
      break;
    }
  }

  // Governance layers specific metadata
  if (pathParts[0] === 'governance_layers' && pathParts.length >= 3) {
    metadata.layerType = pathParts[1]; // 'geographic_political' or 'functional'
    metadata.layer = pathParts[2]; // e.g., 'individual', 'family', 'community'
  }

  // Priority based on content type and position
  switch (contentType) {
    case NODE_TYPES.ROOT_DOC:
      metadata.priority = 100;
      break;
    case NODE_TYPES.EPIC:
      metadata.priority = 90;
      break;
    case NODE_TYPES.PERSONA:
      metadata.priority = 70;
      break;
    case NODE_TYPES.SCENARIO:
      metadata.priority = 50;
      break;
    default:
      metadata.priority = 40;
  }

  return metadata;
}

/**
 * Create a ContentNode from a file
 */
function createContentNode(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const fileName = path.basename(filePath);
  const contentFormat = filePath.endsWith('.feature') ? 'gherkin' : 'markdown';
  const contentType = determineContentType(filePath);
  const category = determineCategory(filePath);

  const node = {
    id: generateId(filePath),
    contentType,
    title: extractTitle(content, fileName, contentFormat),
    description: extractDescription(content, contentFormat),
    content,
    contentFormat,
    tags: extractTags(filePath, content),
    sourcePath: path.relative(DOCS_ROOT, filePath),
    relatedNodeIds: [], // Will be populated in a second pass
    metadata: extractMetadata(filePath, content, contentType),
    category,
    createdAt: fs.statSync(filePath).birthtime.toISOString(),
    updatedAt: fs.statSync(filePath).mtime.toISOString()
  };

  return node;
}

/**
 * Recursively scan directory for markdown and feature files
 */
function scanDirectory(dirPath, nodes = []) {
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      scanDirectory(fullPath, nodes);
    } else if (entry.isFile() && (entry.name.endsWith('.md') || entry.name.endsWith('.feature'))) {
      try {
        const node = createContentNode(fullPath);
        nodes.push(node);
        console.log(`✓ Extracted: ${node.title} (${node.contentType})`);
      } catch (error) {
        console.error(`✗ Error processing ${fullPath}:`, error.message);
      }
    }
  }

  return nodes;
}

/**
 * Build relationships between nodes
 */
function buildRelationships(nodes) {
  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  const pathMap = new Map(nodes.map(n => [n.sourcePath, n]));

  for (const node of nodes) {
    const pathParts = node.sourcePath.split(path.sep);
    const relatedIds = new Set();

    // Relate to parent directory's README if exists
    if (pathParts.length > 1) {
      const parentPath = pathParts.slice(0, -1).join(path.sep);
      const parentReadme = pathMap.get(path.join(parentPath, 'README.md'));
      if (parentReadme && parentReadme.id !== node.id) {
        relatedIds.add(parentReadme.id);
      }
    }

    // Relate to epic README
    if (node.metadata.epic) {
      const epicReadme = pathMap.get(path.join(node.metadata.epic, 'README.md'));
      if (epicReadme && epicReadme.id !== node.id) {
        relatedIds.add(epicReadme.id);
      }
    }

    // Relate to persona README
    if (node.metadata.epic && node.metadata.persona) {
      const personaReadme = pathMap.get(path.join(node.metadata.epic, node.metadata.persona, 'README.md'));
      if (personaReadme && personaReadme.id !== node.id) {
        relatedIds.add(personaReadme.id);
      }
    }

    // Relate scenarios within the same persona
    if (node.contentType === NODE_TYPES.SCENARIO && node.metadata.epic && node.metadata.persona) {
      for (const otherNode of nodes) {
        if (otherNode.contentType === NODE_TYPES.SCENARIO &&
            otherNode.metadata.epic === node.metadata.epic &&
            otherNode.metadata.persona === node.metadata.persona &&
            otherNode.id !== node.id) {
          relatedIds.add(otherNode.id);
        }
      }
    }

    // Relate nodes with same tags
    for (const tag of node.tags.slice(0, 3)) { // Limit to first 3 tags to avoid over-connection
      for (const otherNode of nodes) {
        if (otherNode.tags.includes(tag) && otherNode.id !== node.id && relatedIds.size < 10) {
          relatedIds.add(otherNode.id);
        }
      }
    }

    node.relatedNodeIds = Array.from(relatedIds);
  }

  return nodes;
}

/**
 * Generate taxonomy structure for navigation
 */
function generateTaxonomy(nodes) {
  const taxonomy = {
    epics: {},
    layers: {},
    contentTypes: {},
    personas: {},
    total: nodes.length
  };

  // Group by epic
  for (const [key, name] of Object.entries(EPIC_NAMES)) {
    const epicNodes = nodes.filter(n => n.metadata.epic === key);
    taxonomy.epics[key] = {
      name,
      count: epicNodes.length,
      personas: [...new Set(epicNodes.map(n => n.metadata.persona).filter(Boolean))],
      contentTypes: [...new Set(epicNodes.map(n => n.contentType))]
    };
  }

  // Group by governance layer
  const layerNodes = nodes.filter(n => n.metadata.layer);
  for (const node of layerNodes) {
    const { layerType, layer } = node.metadata;
    if (!taxonomy.layers[layerType]) {
      taxonomy.layers[layerType] = {};
    }
    if (!taxonomy.layers[layerType][layer]) {
      taxonomy.layers[layerType][layer] = 0;
    }
    taxonomy.layers[layerType][layer]++;
  }

  // Group by content type
  for (const type of Object.values(NODE_TYPES)) {
    taxonomy.contentTypes[type] = nodes.filter(n => n.contentType === type).length;
  }

  // Group by persona
  for (const node of nodes) {
    if (node.metadata.persona) {
      if (!taxonomy.personas[node.metadata.persona]) {
        taxonomy.personas[node.metadata.persona] = {
          count: 0,
          epics: new Set()
        };
      }
      taxonomy.personas[node.metadata.persona].count++;
      if (node.metadata.epic) {
        taxonomy.personas[node.metadata.persona].epics.add(node.metadata.epic);
      }
    }
  }

  // Convert Sets to Arrays for JSON serialization
  for (const persona in taxonomy.personas) {
    taxonomy.personas[persona].epics = Array.from(taxonomy.personas[persona].epics);
  }

  return taxonomy;
}

/**
 * Generate suggested learning paths
 */
function generateSuggestedPaths(nodes, taxonomy) {
  const paths = {
    'understanding-elohim-protocol': {
      id: 'understanding-elohim-protocol',
      title: 'Understanding the Elohim Protocol',
      description: 'A curated journey through the core concepts and vision of the Elohim Protocol',
      targetSubject: 'The Elohim Protocol',
      path: []
    },
    'governance-deep-dive': {
      id: 'governance-deep-dive',
      title: 'Governance Systems Deep Dive',
      description: 'Explore the multi-layered governance architecture',
      targetSubject: 'Governance Systems',
      path: []
    },
    'social-medium-foundations': {
      id: 'social-medium-foundations',
      title: 'Social Medium Foundations',
      description: 'Learn the principles of the Elohim Social Medium',
      targetSubject: 'Social Medium Design',
      path: []
    }
  };

  // Build 'Understanding Elohim Protocol' path
  const manifestoNode = nodes.find(n => n.sourcePath === 'manifesto.md');
  const socialMediumReadme = nodes.find(n => n.sourcePath === 'social_medium/README.md');
  const governanceReadme = nodes.find(n => n.sourcePath === 'governance/README.md');
  const valuesScannerReadme = nodes.find(n => n.sourcePath === 'value_scanner/README.md');

  if (manifestoNode) paths['understanding-elohim-protocol'].path.push(manifestoNode.id);
  if (socialMediumReadme) paths['understanding-elohim-protocol'].path.push(socialMediumReadme.id);
  if (governanceReadme) paths['understanding-elohim-protocol'].path.push(governanceReadme.id);
  if (valuesScannerReadme) paths['understanding-elohim-protocol'].path.push(valuesScannerReadme.id);

  // Build 'Governance Deep Dive' path
  if (governanceReadme) paths['governance-deep-dive'].path.push(governanceReadme.id);
  const govLayersNodes = nodes.filter(n => n.metadata.epic === 'governance_layers').slice(0, 10);
  paths['governance-deep-dive'].path.push(...govLayersNodes.map(n => n.id));

  // Build 'Social Medium Foundations' path
  if (socialMediumReadme) paths['social-medium-foundations'].path.push(socialMediumReadme.id);
  const socialMediumNodes = nodes.filter(n => n.metadata.epic === 'social_medium').slice(0, 10);
  paths['social-medium-foundations'].path.push(...socialMediumNodes.map(n => n.id));

  return Object.values(paths);
}

/**
 * Main execution
 */
function main() {
  console.log('🚀 Extracting documentation to content nodes...\n');
  console.log(`📂 Source: ${DOCS_ROOT}`);
  console.log(`📝 Output: ${OUTPUT_FILE}\n`);

  // Scan all files
  console.log('📖 Scanning files...\n');
  let nodes = scanDirectory(DOCS_ROOT);

  console.log(`\n✅ Extracted ${nodes.length} content nodes\n`);

  // Build relationships
  console.log('🔗 Building relationships...\n');
  nodes = buildRelationships(nodes);

  // Generate taxonomy
  console.log('📊 Generating taxonomy...\n');
  const taxonomy = generateTaxonomy(nodes);

  // Generate suggested paths
  console.log('🗺️  Generating suggested learning paths...\n');
  const suggestedPaths = generateSuggestedPaths(nodes, taxonomy);

  // Prepare output
  const output = {
    version: '1.0.0',
    generatedAt: new Date().toISOString(),
    stats: {
      totalNodes: nodes.length,
      byType: taxonomy.contentTypes,
      byEpic: Object.fromEntries(
        Object.entries(taxonomy.epics).map(([key, val]) => [key, val.count])
      )
    },
    taxonomy,
    suggestedPaths,
    nodes
  };

  // Ensure output directory exists
  const outputDir = path.dirname(OUTPUT_FILE);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // Write output
  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(output, null, 2));

  console.log('✨ Complete!\n');
  console.log('📊 Statistics:');
  console.log(`   Total nodes: ${output.stats.totalNodes}`);
  console.log(`   Epics: ${Object.keys(taxonomy.epics).length}`);
  console.log(`   Personas: ${Object.keys(taxonomy.personas).length}`);
  console.log(`   Suggested paths: ${suggestedPaths.length}`);
  console.log(`\n💾 Output written to: ${OUTPUT_FILE}`);
}

// Run if called directly
if (require.main === module) {
  main();
}

module.exports = { scanDirectory, createContentNode, buildRelationships, generateTaxonomy };
