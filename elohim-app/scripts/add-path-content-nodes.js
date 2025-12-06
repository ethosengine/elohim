/**
 * Generate ContentNode entries for LearningPaths.
 *
 * Creates ContentNode entries with ID pattern `path-${path.id}` so that
 * paths can be viewed as content (via "View as Content" link in path-overview).
 *
 * This enables the path-overview "View as Content" feature to work with Kuzu.
 */
const fs = require('fs');
const path = require('path');

const pathsDir = 'src/assets/lamad-data/paths';
const seedFile = 'src/assets/lamad-data/lamad-seed.cypher';

function escapeCypher(str) {
  if (typeof str !== 'string') return '';
  // Use placeholders instead of escape sequences because Kuzu WASM
  // mangles \n escape sequences (strips backslash, leaves just 'n')
  return str
    .replace(/\\/g, '{{BACKSLASH}}')
    .replace(/'/g, '{{QUOTE}}')
    .replace(/"/g, '{{DQUOTE}}')
    .replace(/\n/g, '{{NEWLINE}}')
    .replace(/\r/g, '');
}

// Get all path JSON files
const pathFiles = fs.readdirSync(pathsDir)
  .filter(f => f.endsWith('.json') && f !== 'index.json');

const contentStatements = [];

for (const file of pathFiles) {
  const filePath = path.join(pathsDir, file);
  const json = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // Create ContentNode with ID: path-${pathId}
  const contentId = `path-${json.id}`;
  const title = escapeCypher(json.title || '');
  const description = escapeCypher(json.description || '');
  const content = escapeCypher(json.purpose || json.description || '');
  const tags = Array.isArray(json.tags) ? json.tags.map(t => escapeCypher(String(t))) : [];
  // Add 'path' tag
  if (!tags.includes('path')) {
    tags.push('path');
  }
  const tagsStr = tags.length > 0 ? '[' + tags.map(t => '"' + t + '"').join(', ') + ']' : '[]';

  contentStatements.push(
    `CREATE (:ContentNode {id: '${escapeCypher(contentId)}', contentType: 'path', title: '${title}', description: '${description}', content: '${content}', contentFormat: 'markdown', tags: ${tagsStr}});`
  );

  console.log(`Created: ${contentId}`);
}

// Append to seed file
const now = new Date().toISOString();
const newSection = `

// ============================================
// Path ContentNodes (for "View as Content" feature)
// Generated: ${now}
// ============================================

${contentStatements.join('\n')}
`;

fs.appendFileSync(seedFile, newSection);
console.log(`\nAppended ${contentStatements.length} path ContentNode entries to seed file`);
