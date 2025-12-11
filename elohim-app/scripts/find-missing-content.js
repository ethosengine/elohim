const fs = require('fs');
const path = require('path');

const pathsDir = 'src/assets/lamad-data/paths';
const contentDir = 'src/assets/lamad-data/content';
const seedFile = 'src/assets/lamad-data/lamad-seed.cypher';

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

// Check which are in seed file
const seedContent = fs.readFileSync(seedFile, 'utf8');

const missing = [];
const present = [];

for (const id of resourceIds) {
  // Check for the id in single or double quotes
  if (seedContent.includes("id: '" + id + "'") || seedContent.includes('id: "' + id + '"')) {
    present.push(id);
  } else {
    missing.push(id);
  }
}

console.log('=== Present in seed (' + present.length + ') ===');
present.forEach(id => console.log('  ✓ ' + id));

console.log('\n=== Missing from seed (' + missing.length + ') ===');
missing.forEach(id => {
  const jsonPath = path.join(contentDir, id + '.json');
  const exists = fs.existsSync(jsonPath) ? 'JSON exists' : 'NO JSON';
  console.log('  ✗ ' + id + ' (' + exists + ')');
});
