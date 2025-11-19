#!/usr/bin/env node

/**
 * Validates all .feature files in the docs directory
 * Checks for Gherkin syntax errors and reports statistics
 */

const fs = require('fs');
const path = require('path');
const Gherkin = require('@cucumber/gherkin');
const Messages = require('@cucumber/messages');

const docsDir = path.join(__dirname, '../../docs');
const uuidFn = Messages.IdGenerator.uuid();
const builder = new Gherkin.AstBuilder(uuidFn);
const matcher = new Gherkin.GherkinClassicTokenMatcher();
const parser = new Gherkin.Parser(builder, matcher);

let totalFeatures = 0;
let validFeatures = 0;
let invalidFeatures = 0;
let totalScenarios = 0;
const errors = [];

function findFeatureFiles(dir) {
  const files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...findFeatureFiles(fullPath));
    } else if (entry.name.endsWith('.feature')) {
      files.push(fullPath);
    }
  }

  return files;
}

function validateFeature(featurePath) {
  try {
    const content = fs.readFileSync(featurePath, 'utf8');
    const gherkinDocument = parser.parse(content);

    if (gherkinDocument.feature) {
      const scenarios = gherkinDocument.feature.children.length;
      totalScenarios += scenarios;
      validFeatures++;

      const relativePath = path.relative(docsDir, featurePath);
      console.log(`✓ ${relativePath} (${scenarios} scenarios)`);
    }
  } catch (error) {
    invalidFeatures++;
    const relativePath = path.relative(docsDir, featurePath);
    console.error(`✗ ${relativePath}`);
    console.error(`  Error: ${error.message}`);
    errors.push({ file: relativePath, error: error.message });
  }
}

console.log('=== Validating Elohim Protocol Feature Files ===\n');

const featureFiles = findFeatureFiles(docsDir);
totalFeatures = featureFiles.length;

console.log(`Found ${totalFeatures} feature files\n`);

featureFiles.forEach(validateFeature);

console.log('\n=== Validation Summary ===');
console.log(`Total Features: ${totalFeatures}`);
console.log(`Valid: ${validFeatures}`);
console.log(`Invalid: ${invalidFeatures}`);
console.log(`Total Scenarios: ${totalScenarios}`);

if (errors.length > 0) {
  console.log('\n=== Errors ===');
  errors.forEach(({ file, error }) => {
    console.log(`${file}: ${error}`);
  });
  process.exit(1);
} else {
  console.log('\n✓ All feature files are valid!');
  process.exit(0);
}
