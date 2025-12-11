const fs = require('fs');
const path = require('path');

// The 8 content IDs referenced by elohim-protocol path
const pathContentIds = [
  'manifesto',
  'quiz-who-are-you',
  'governance-epic',
  'autonomous-entity-epic',
  'economic-coordination-epic',
  'public-observer-epic',
  'social-medium-epic',
  'value-scanner-epic'
];

const contentDir = 'src/assets/lamad-data/content';
const seedFile = 'src/assets/lamad-data/lamad-seed.cypher';

function escapeCypher(str) {
  if (typeof str !== 'string') return '';
  // Escape single quotes and backslashes for Cypher string literals
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

// Check which IDs are already in the seed file
const existingSeed = fs.readFileSync(seedFile, 'utf8');
const cypherStatements = [];

for (const contentId of pathContentIds) {
  // Check if this ID already exists in seed
  if (existingSeed.includes("id: '" + contentId + "'") || existingSeed.includes('id: "' + contentId + '"')) {
    console.log('SKIP: ' + contentId + ' (already in seed)');
    continue;
  }

  const jsonPath = path.join(contentDir, contentId + '.json');
  if (!fs.existsSync(jsonPath)) {
    console.log('MISSING: ' + jsonPath);
    continue;
  }

  try {
    const json = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
    const cypher = jsonToCypher(json);
    cypherStatements.push(cypher);
    console.log('ADD: ' + contentId);
  } catch (err) {
    console.log('ERROR: ' + contentId + ' - ' + err.message);
  }
}

if (cypherStatements.length > 0) {
  // Append to seed file
  const now = new Date().toISOString();
  const newSection = '\n\n// ============================================\n// Path Content Nodes (for learning paths)\n// Generated: ' + now + '\n// ============================================\n\n' + cypherStatements.join('\n') + '\n';

  fs.appendFileSync(seedFile, newSection);
  console.log('\nAppended ' + cypherStatements.length + ' new content nodes to seed file');
} else {
  console.log('\nNo new content nodes to add');
}
