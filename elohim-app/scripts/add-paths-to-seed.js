const fs = require('fs');
const path = require('path');

const pathsDir = 'src/assets/lamad-data/paths';
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

// Get all path JSON files
const pathFiles = fs.readdirSync(pathsDir)
  .filter(f => f.endsWith('.json') && f !== 'index.json');

const pathStatements = [];
const stepStatements = [];
const pathHasStepRels = [];
const stepUsesContentRels = [];

for (const file of pathFiles) {
  const filePath = path.join(pathsDir, file);
  const json = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // Create LearningPath node
  const pathId = escapeCypher(json.id);
  const version = escapeCypher(json.version || '1.0.0');
  const title = escapeCypher(json.title || '');
  const description = escapeCypher(json.description || '');
  const purpose = escapeCypher(json.purpose || '');
  const createdBy = escapeCypher(json.createdBy || 'system');
  const difficulty = escapeCypher(json.difficulty || 'beginner');
  const estimatedDuration = escapeCypher(json.estimatedDuration || '');
  const visibility = escapeCypher(json.visibility || 'public');
  const pathType = escapeCypher(json.pathType || 'learning');
  const tags = Array.isArray(json.tags) ? json.tags.map(t => escapeCypher(String(t))) : [];
  const tagsStr = tags.length > 0 ? '[' + tags.map(t => '"' + t + '"').join(', ') + ']' : '[]';
  const thumbnailUrl = escapeCypher(json.thumbnailUrl || '');
  const thumbnailAlt = escapeCypher(json.thumbnailAlt || '');

  pathStatements.push(
    "CREATE (:LearningPath {id: '" + pathId + "', version: '" + version + "', title: '" + title +
    "', description: '" + description + "', purpose: '" + purpose + "', createdBy: '" + createdBy +
    "', difficulty: '" + difficulty + "', estimatedDuration: '" + estimatedDuration +
    "', visibility: '" + visibility + "', pathType: '" + pathType + "', tags: " + tagsStr +
    ", thumbnailUrl: '" + thumbnailUrl + "', thumbnailAlt: '" + thumbnailAlt + "'});"
  );

  // Create PathStep nodes and relationships
  const steps = json.steps || [];
  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    const stepId = pathId + '-step-' + i;
    const orderIndex = step.order !== undefined ? step.order : i;
    const stepType = escapeCypher(step.stepType || 'content');
    const resourceId = escapeCypher(step.resourceId || '');
    const stepTitle = escapeCypher(step.stepTitle || '');
    const stepNarrative = escapeCypher(step.stepNarrative || '');
    const isOptional = step.optional === true;
    const attestationRequired = escapeCypher(step.attestationRequired || '');
    const attestationGranted = escapeCypher(step.attestationGranted || '');
    const estimatedTime = escapeCypher(step.estimatedTime || '');

    stepStatements.push(
      "CREATE (:PathStep {id: '" + stepId + "', pathId: '" + pathId + "', orderIndex: " + orderIndex +
      ", stepType: '" + stepType + "', resourceId: '" + resourceId + "', stepTitle: '" + stepTitle +
      "', stepNarrative: '" + stepNarrative + "', isOptional: " + isOptional +
      ", attestationRequired: '" + attestationRequired + "', attestationGranted: '" + attestationGranted +
      "', estimatedTime: '" + estimatedTime + "'});"
    );

    // PATH_HAS_STEP relationship
    pathHasStepRels.push(
      "MATCH (p:LearningPath {id: '" + pathId + "'}), (s:PathStep {id: '" + stepId + "'}) CREATE (p)-[:PATH_HAS_STEP]->(s);"
    );

    // STEP_USES_CONTENT relationship
    if (resourceId) {
      stepUsesContentRels.push(
        "MATCH (s:PathStep {id: '" + stepId + "'}), (c:ContentNode {id: '" + resourceId + "'}) CREATE (s)-[:STEP_USES_CONTENT]->(c);"
      );
    }
  }

  console.log('Processed: ' + json.id + ' (' + steps.length + ' steps)');
}

// Append to seed file
const now = new Date().toISOString();
const newSection = `

// ============================================
// Learning Paths
// Generated: ${now}
// ============================================

${pathStatements.join('\n')}

// ============================================
// Path Steps
// ============================================

${stepStatements.join('\n')}

// ============================================
// PATH_HAS_STEP Relationships
// ============================================

${pathHasStepRels.join('\n')}

// ============================================
// STEP_USES_CONTENT Relationships
// ============================================

${stepUsesContentRels.join('\n')}
`;

fs.appendFileSync(seedFile, newSection);
console.log('\nAppended ' + pathStatements.length + ' paths, ' + stepStatements.length + ' steps to seed file');
