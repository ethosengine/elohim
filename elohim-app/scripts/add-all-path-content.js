const fs = require('fs');
const path = require('path');

const pathsDir = 'src/assets/lamad-data/paths';
const contentDir = 'src/assets/lamad-data/content';
const seedFile = 'src/assets/lamad-data/lamad-seed.cypher';

function escapeCypher(str) {
  if (typeof str !== 'string') return '';
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\n');
}

function jsonToCypher(json) {
  const id = escapeCypher(json.id);
  const contentType = escapeCypher(json.contentType || 'content');
  const title = escapeCypher(json.title || '');
  const description = escapeCypher(json.description || '');
  const tags = Array.isArray(json.tags) ? json.tags : [];
  const escapedTags = tags.map(t => escapeCypher(String(t)));
  const tagsStr = escapedTags.length > 0 ? '[' + escapedTags.map(t => '"' + t + '"').join(', ') + ']' : '[]';

  return "CREATE (:ContentNode {id: '" + id + "', contentType: '" + contentType + "', title: '" + title + "', description: '" + description + "', tags: " + tagsStr + "});";
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

// Check which ContentNodes actually exist in seed file (more specific check)
const seedContent = fs.readFileSync(seedFile, 'utf8');

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
      const cypher = jsonToCypher(json);
      cypherStatements.push(cypher);
      console.log('  ADD: ' + id);
    } catch (err) {
      console.log('  ERROR: ' + id + ' - ' + err.message);
    }
  } else {
    console.log('  MISSING JSON: ' + id);
  }
}

if (cypherStatements.length > 0) {
  const now = new Date().toISOString();
  const newSection = '\n\n// ============================================\n// Additional Path Content Nodes\n// Generated: ' + now + '\n// ============================================\n\n' + cypherStatements.join('\n') + '\n';

  fs.appendFileSync(seedFile, newSection);
  console.log('\nAppended ' + cypherStatements.length + ' new ContentNodes to seed file');
} else {
  console.log('\nNo new ContentNodes to add');
}
