# Elohim Protocol BDD Test Suite

This directory contains the BDD (Behavior-Driven Development) test suite for the Elohim Protocol specifications. The tests are **intentionally failing** with stub implementations to demonstrate what needs to be built.

## Purpose

The Elohim Protocol is defined through executable specifications in the `/docs` directory. This test suite:

1. **Validates** that all .feature files are syntactically correct Gherkin
2. **Demonstrates** what functionality needs to be implemented
3. **Exports** feature files for external implementation
4. **Reports** on the current implementation status

## Philosophy

> **All tests currently fail. This is by design.**

Each failing test represents a component of the Elohim Protocol that needs to be implemented. When you make the tests pass by implementing real functionality, you will have built a working piece of the protocol.

## Test Domains

The protocol is organized into five main domains:

### 1. Public Observer (`public_observer/`)
- Civic transparency and observation systems
- User personas: developer, politician, journalist, citizen, teacher, etc.
- Governance layers: municipality, county/regional, provincial/state, etc.

### 2. Governance (`governance/`)
- Multi-tiered governance models
- Constitutional protocols
- Amendment and voting processes

### 3. Value Scanner (`value_scanner/`)
- Value assessment for different demographics
- Economic and social value tracking
- Resource allocation systems

### 4. Social Medium (`social_medium/`)
- Social interaction protocols
- Community engagement systems

### 5. Autonomous Entity (`autonomous_entity/`)
- Autonomous system behaviors
- AI agent protocols

## Installation

```bash
npm install
```

## Running Tests

### Run all tests
```bash
npm test
```

### Run by domain
```bash
npm run test:public-observer
npm run test:governance
npm run test:value-scanner
npm run test:social-medium
npm run test:autonomous-entity
```

### Validate feature files
```bash
npm run validate-features
```

### Generate reports
```bash
npm run report:generate
```

### Export feature files
```bash
npm run export:features
```

## Test Reports

After running tests, reports are generated in:

- `reports/html/` - HTML test reports (open `index.html` in a browser)
- `reports/json/` - JSON reports for CI/CD integration
- `reports/coverage/` - Code coverage (currently N/A for stubs)

## Exported Features

Running `npm run export:features` creates a distributable package in `exports/`:

- All .feature files organized by domain
- README.md with implementation instructions
- manifest.json with metadata

External developers can use these exported features to build protocol-compliant implementations.

## Jenkins Pipeline

A separate Jenkins pipeline (`Jenkinsfile.protocol`) runs these tests automatically:

- **Validates** all feature file syntax
- **Runs** tests for each domain
- **Generates** unified HTML/JSON reports
- **Exports** feature files as artifacts
- **Publishes** results to Jenkins

This pipeline is completely independent from the elohim-app pipeline.

## Implementation Guide

To implement a protocol component:

1. **Choose** a feature file from `/docs`
2. **Read** the scenarios to understand the expected behavior
3. **Replace** the stub step definitions in `step-definitions/` with real implementations
4. **Run** the tests: `npm test`
5. **Iterate** until all scenarios pass

### Example

For the Public Observer protocol:

```javascript
// Current stub (in step-definitions/public-observer.test.js)
given(/the Elohim Protocol is operational/, () => {
  throw new Error('NOT IMPLEMENTED: Elohim Protocol initialization');
});

// Your implementation
given(/the Elohim Protocol is operational/, () => {
  // Initialize the protocol
  protocol = new ElohimProtocol();
  expect(protocol.isOperational()).toBe(true);
});
```

## Project Structure

```
protocol-tests/
├── README.md              # This file
├── package.json           # Dependencies and scripts
├── jest.config.js         # Jest configuration
├── step-definitions/      # Test implementations (currently stubs)
│   ├── public-observer.test.js
│   ├── governance.test.js
│   ├── value-scanner.test.js
│   ├── social-medium.test.js
│   └── autonomous-entity.test.js
├── scripts/               # Utility scripts
│   ├── validate-features.js    # Validate Gherkin syntax
│   ├── generate-report.js      # Generate HTML reports
│   └── export-features.js      # Export feature files
├── reports/               # Generated test reports (gitignored)
└── exports/               # Exported feature packages (gitignored)
```

## CI/CD Integration

### Jenkins

The `Jenkinsfile.protocol` pipeline:
- Runs on every commit to `/docs` or `/protocol-tests`
- Executes all test domains
- Publishes Cucumber reports
- Archives feature exports as artifacts

### Triggering

The protocol pipeline runs independently from the main elohim-app pipeline and can be triggered:
- Automatically via SCM polling (every 15 minutes)
- Manually from Jenkins UI
- Via webhook on docs changes

## Expected Results

**Current state**: All tests fail with "NOT IMPLEMENTED" errors.

This is correct! The failures show:
- ✓ Feature files are being read correctly
- ✓ Test framework is working
- ✗ Protocol components need implementation

**Success criteria**: When you implement the protocol:
- All scenarios pass
- Reports show green status
- Exported features can be satisfied by your implementation

## Contributing

To add new protocol specifications:

1. Add .feature files to `/docs` in the appropriate domain
2. Run `npm run validate-features` to check syntax
3. Run tests to see the new failing scenarios
4. Implement step definitions to make them pass

## License

Part of the Elohim Protocol project. See main repository for license details.

## Questions?

See `/docs/manifesto.md` for the complete Elohim Protocol vision and philosophy.
