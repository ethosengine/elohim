const fs = require('fs');
const path = require('path');

const pathsDir = 'src/assets/lamad-data/paths';
const contentDir = 'src/assets/lamad-data/content';
const seedFile = 'src/assets/lamad-data/lamad-seed.cypher';

function escapeCypher(str) {
  if (typeof str !== 'string') return '';
  // Use placeholders instead of escape sequences because Kuzu WASM
  // mangles \n escape sequences (strips backslash, leaves just 'n')
  // Also escape double quotes to avoid Cypher parsing issues
  return str
    .replace(/\\/g, '{{BACKSLASH}}')
    .replace(/'/g, '{{QUOTE}}')
    .replace(/"/g, '{{DQUOTE}}')
    .replace(/\n/g, '{{NEWLINE}}')
    .replace(/\r/g, '');
}

// Maximum content size to store directly in ContentNode
// Larger content is stored in ContentChunk nodes and reassembled on fetch
const MAX_CONTENT_SIZE = 15000;

function jsonToCypherFull(json) {
  const id = escapeCypher(json.id);
  const contentType = escapeCypher(json.contentType || 'content');
  const title = escapeCypher(json.title || '');
  const description = escapeCypher(json.description || '');

  // Handle content - could be string or object
  let rawContent = '';
  if (typeof json.content === 'string') {
    rawContent = json.content;
  } else if (json.content && typeof json.content === 'object') {
    // For quizzes/assessments, store as JSON string
    rawContent = JSON.stringify(json.content);
  }

  // For large content, store [CHUNKED] marker - actual content is in ContentChunk nodes
  // which are loaded from content-chunks.cypher and reassembled by KuzuDataService
  let content = '';
  if (rawContent.length > MAX_CONTENT_SIZE) {
    content = '[CHUNKED]';
    console.log('    CHUNKED: Content stored in chunks (original: ' + rawContent.length + ' chars)');
  } else {
    content = escapeCypher(rawContent);
  }

  const contentFormat = escapeCypher(json.contentFormat || 'markdown');
  const tags = Array.isArray(json.tags) ? json.tags : [];
  const escapedTags = tags.map(t => escapeCypher(String(t)));
  const tagsStr = escapedTags.length > 0 ? '[' + escapedTags.map(t => '"' + t + '"').join(', ') + ']' : '[]';

  return "CREATE (:ContentNode {id: '" + id + "', contentType: '" + contentType +
    "', title: '" + title + "', description: '" + description +
    "', content: '" + content + "', contentFormat: '" + contentFormat +
    "', tags: " + tagsStr + "});";
}

// Get all resourceIds from paths
const pathFiles = fs.readdirSync(pathsDir).filter(f => f.endsWith('.json') && f !== 'index.json');
const resourceIds = new Set();

for (const file of pathFiles) {
  const json = JSON.parse(fs.readFileSync(path.join(pathsDir, file), 'utf8'));
  const steps = json.steps || [];
  for (const step of steps) {
    if (step.resourceId) {
      resourceIds.add(step.resourceId);
    }
  }
}

// Read existing seed and remove the "Additional Path Content Nodes" section if exists
let seedContent = fs.readFileSync(seedFile, 'utf8');

// Remove any previously added path content sections
const marker = '// ============================================\n// Additional Path Content Nodes';
const markerIndex = seedContent.indexOf(marker);
if (markerIndex !== -1) {
  seedContent = seedContent.substring(0, markerIndex).trimEnd();
  fs.writeFileSync(seedFile, seedContent);
  console.log('Removed previous Additional Path Content section');
}

// Also check for Path Content Nodes (older marker)
const marker2 = '// ============================================\n// Path Content Nodes (for learning paths)';
const marker2Index = seedContent.indexOf(marker2);
if (marker2Index !== -1) {
  seedContent = seedContent.substring(0, marker2Index).trimEnd();
  fs.writeFileSync(seedFile, seedContent);
  console.log('Removed previous Path Content Nodes section');
}

// Also check for Path Content Nodes with Full Content
const marker3 = '// ============================================\n// Path Content Nodes with Full Content';
const marker3Index = seedContent.indexOf(marker3);
if (marker3Index !== -1) {
  seedContent = seedContent.substring(0, marker3Index).trimEnd();
  fs.writeFileSync(seedFile, seedContent);
  console.log('Removed previous Path Content Nodes with Full Content section');
}

// Re-read after trimming
seedContent = fs.readFileSync(seedFile, 'utf8');

// Extract all ContentNode IDs from CREATE statements
const contentNodePattern = /CREATE \(:ContentNode \{id: ['"]([^'"]+)['"]/g;
const existingContentNodes = new Set();
let match;
while ((match = contentNodePattern.exec(seedContent)) !== null) {
  existingContentNodes.add(match[1]);
}

console.log('Found ' + existingContentNodes.size + ' existing ContentNodes in seed');
console.log('Found ' + resourceIds.size + ' unique resourceIds in paths');

// Find missing content nodes
const missing = [];
for (const id of resourceIds) {
  if (!existingContentNodes.has(id)) {
    missing.push(id);
  }
}

console.log('\n=== Missing ContentNodes (' + missing.length + ') ===');

const cypherStatements = [];
for (const id of missing) {
  const jsonPath = path.join(contentDir, id + '.json');
  if (fs.existsSync(jsonPath)) {
    try {
      const json = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
      const cypher = jsonToCypherFull(json);
      cypherStatements.push(cypher);
      console.log('  ADD: ' + id + ' (content: ' + (typeof json.content === 'string' ? json.content.length + ' chars' : 'object') + ')');
    } catch (err) {
      console.log('  ERROR: ' + id + ' - ' + err.message);
    }
  } else {
    console.log('  MISSING JSON: ' + id);
  }
}

if (cypherStatements.length > 0) {
  const now = new Date().toISOString();
  const newSection = '\n\n// ============================================\n// Path Content Nodes with Full Content\n// Generated: ' + now + '\n// ============================================\n\n' + cypherStatements.join('\n') + '\n';

  fs.appendFileSync(seedFile, newSection);
  console.log('\nAppended ' + cypherStatements.length + ' new ContentNodes (with full content) to seed file');
} else {
  console.log('\nNo new ContentNodes to add');
}
