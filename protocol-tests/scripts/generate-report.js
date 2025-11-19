#!/usr/bin/env node

/**
 * Generates a unified HTML report for all protocol specification tests
 */

const fs = require('fs');
const path = require('path');
const reporter = require('cucumber-html-reporter');

const reportsDir = path.join(__dirname, '../reports');
const jsonDir = path.join(reportsDir, 'json');
const htmlDir = path.join(reportsDir, 'html');

// Ensure directories exist
if (!fs.existsSync(htmlDir)) {
  fs.mkdirSync(htmlDir, { recursive: true });
}

// Find all JSON report files
const jsonFiles = fs.existsSync(jsonDir)
  ? fs.readdirSync(jsonDir).filter((f) => f.endsWith('.json'))
  : [];

if (jsonFiles.length === 0) {
  console.log('No JSON report files found. Generating placeholder report...');

  // Create a placeholder JSON report
  const placeholderReport = {
    description: 'Elohim Protocol Specification Tests',
    elements: [
      {
        id: 'placeholder',
        keyword: 'Feature',
        name: 'Protocol Specification Placeholder',
        description:
          'This is a placeholder report. Run the protocol tests to see actual results.',
        line: 1,
        tags: [],
        uri: 'placeholder.feature',
      },
    ],
    id: 'elohim-protocol',
    keyword: 'Feature',
    line: 1,
    name: 'Elohim Protocol Specifications',
    tags: [],
    uri: 'elohim-protocol.feature',
  };

  if (!fs.existsSync(jsonDir)) {
    fs.mkdirSync(jsonDir, { recursive: true });
  }

  fs.writeFileSync(
    path.join(jsonDir, 'placeholder.json'),
    JSON.stringify([placeholderReport], null, 2)
  );
}

// Generate HTML report
const options = {
  theme: 'bootstrap',
  jsonDir: jsonDir,
  output: path.join(htmlDir, 'protocol-report.html'),
  reportSuiteAsScenarios: true,
  scenarioTimestamp: true,
  launchReport: false,
  metadata: {
    'Protocol Version': process.env.PROTOCOL_VERSION || 'dev',
    'Test Environment': 'CI/CD Pipeline',
    Platform: process.platform,
    'Node Version': process.version,
    'Report Generated': new Date().toISOString(),
  },
  name: 'Elohim Protocol Specification Report',
  brandTitle: 'Elohim Protocol BDD Specifications',
  screenshotsDirectory: path.join(reportsDir, 'screenshots'),
  storeScreenshots: false,
};

try {
  reporter.generate(options);
  console.log('✓ Protocol specification report generated successfully!');
  console.log(`  Location: ${options.output}`);
} catch (error) {
  console.error('✗ Failed to generate report:', error.message);
  process.exit(1);
}
