#!/usr/bin/env node

/**
 * Exports all .feature files from docs to a distributable format
 * Creates a packaged version that developers can use to implement the protocol
 */

const fs = require('fs');
const path = require('path');

const docsDir = path.join(__dirname, '../../docs');
const exportsDir = path.join(__dirname, '../exports');

// Clean exports directory
if (fs.existsSync(exportsDir)) {
  fs.rmSync(exportsDir, { recursive: true, force: true });
}
fs.mkdirSync(exportsDir, { recursive: true });

function findFeatureFiles(dir, baseDir = dir) {
  const files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...findFeatureFiles(fullPath, baseDir));
    } else if (entry.name.endsWith('.feature')) {
      const relativePath = path.relative(baseDir, fullPath);
      files.push({ fullPath, relativePath });
    }
  }

  return files;
}

function copyFeatureFiles() {
  const features = findFeatureFiles(docsDir);
  let copiedCount = 0;

  console.log('=== Exporting Elohim Protocol Feature Files ===\n');
  console.log(`Found ${features.length} feature files\n`);

  features.forEach(({ fullPath, relativePath }) => {
    const targetPath = path.join(exportsDir, relativePath);
    const targetDir = path.dirname(targetPath);

    // Create target directory if it doesn't exist
    if (!fs.existsSync(targetDir)) {
      fs.mkdirSync(targetDir, { recursive: true });
    }

    // Copy file
    fs.copyFileSync(fullPath, targetPath);
    copiedCount++;

    console.log(`✓ Exported: ${relativePath}`);
  });

  return { total: features.length, copied: copiedCount };
}

function generateReadme() {
  const readme = `# Elohim Protocol Feature Specifications

This directory contains the complete behavioral specifications for the Elohim Protocol.
Each .feature file uses Gherkin syntax to describe the expected behavior of protocol components.

## Purpose

These feature files serve as **executable specifications**. They define what needs to be
implemented to build a complete Elohim Protocol system.

## Current Status

⚠️ **All tests currently fail with stub implementations**

This is intentional. The failing tests show what needs to be built. When you implement
the actual functionality and make these tests pass, you will have built a working component
of the Elohim Protocol.

## Structure

- **public_observer/** - Public Observer protocol specifications
  - Civic transparency and observation features
  - Different user personas (developer, politician, journalist, citizen, etc.)

- **governance/** - Governance layer specifications
  - Multi-tiered governance models
  - Constitutional protocols and amendment processes

- **value_scanner/** - Value assessment specifications
  - Value scanning for different demographics
  - Economic and social value tracking

- **social_medium/** - Social medium specifications
  - Social interaction protocols

- **autonomous_entity/** - Autonomous entity specifications
  - Autonomous system behaviors

## How to Use

1. Choose a feature file that interests you
2. Read the scenarios to understand the expected behavior
3. Implement the step definitions to make the tests pass
4. When all scenarios pass, you've successfully implemented that component

## Testing Framework

These features can be tested with:
- Cucumber (JavaScript, Ruby, Java, etc.)
- jest-cucumber (JavaScript/Node.js)
- Behave (Python)
- SpecFlow (.NET)
- Or any other Gherkin-compatible BDD framework

## Example

\`\`\`gherkin
Feature: Developer Reviews Proposals
  Scenario: Developer reviews active proposal
    Given a developer interested in infrastructure proposals
    When they access the Public Observer system
    Then they should see active infrastructure proposals
\`\`\`

To implement this:
1. Create step definitions for each Given/When/Then
2. Implement the actual system that satisfies the behavior
3. Run tests to verify your implementation

## Protocol Version

Export Date: ${new Date().toISOString()}
Build: ${process.env.BUILD_NUMBER || 'local'}
Branch: ${process.env.BRANCH_NAME || 'development'}

## Questions?

See the main Elohim Protocol documentation at:
https://github.com/ethosengine/elohim

---

*These specifications are part of the Elohim Protocol project.*
*For more information, see docs/manifesto.md*
`;

  fs.writeFileSync(path.join(exportsDir, 'README.md'), readme);
  console.log('\n✓ Generated README.md');
}

function generateManifest() {
  const features = findFeatureFiles(docsDir);

  const manifest = {
    protocol: 'Elohim Protocol',
    version: process.env.PROTOCOL_VERSION || 'dev',
    exportDate: new Date().toISOString(),
    build: process.env.BUILD_NUMBER || 'local',
    branch: process.env.BRANCH_NAME || 'development',
    features: {
      total: features.length,
      byDomain: {}
    },
    files: []
  };

  // Organize by domain
  features.forEach(({ relativePath }) => {
    const domain = relativePath.split('/')[0];
    if (!manifest.features.byDomain[domain]) {
      manifest.features.byDomain[domain] = 0;
    }
    manifest.features.byDomain[domain]++;
    manifest.files.push(relativePath);
  });

  fs.writeFileSync(
    path.join(exportsDir, 'manifest.json'),
    JSON.stringify(manifest, null, 2)
  );
  console.log('✓ Generated manifest.json');
}

// Execute export
const results = copyFeatureFiles();
generateReadme();
generateManifest();

console.log('\n=== Export Summary ===');
console.log(`Total Features: ${results.total}`);
console.log(`Successfully Exported: ${results.copied}`);
console.log(`\nExport Location: ${exportsDir}`);
console.log('\n✓ Feature files exported successfully!');
