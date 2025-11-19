# Quick Start Guide - Elohim Protocol Tests

Get started with the Elohim Protocol BDD test suite in 5 minutes.

## Prerequisites

- Node.js 18+ installed
- npm or yarn package manager
- Access to the `/docs` directory with .feature files

## Installation

```bash
cd protocol-tests
npm install
```

## Run Your First Test

```bash
# Validate all feature files
npm run validate-features

# Run a single domain's tests
npm run test:public-observer
```

## What You'll See

**Expected output:**
```
FAIL  step-definitions/public-observer.test.js
  ✕ Municipal Development Navigation for Developer Interests
    ✕ Responsible developer navigates approval process transparently
      Error: NOT IMPLEMENTED: a developer is proposing affordable housing project
```

This is correct! The test found the feature file and is showing what needs to be implemented.

## Understanding the Output

- **✕ (Red X)**: Test failed - this is expected for stubs
- **NOT IMPLEMENTED**: Shows which step needs implementation
- Each failing test = a protocol component to build

## View the HTML Report

```bash
# Generate and open the HTML report
npm run report:generate

# Open in browser
open reports/html/protocol-report.html
```

The report shows:
- All feature files tested
- Which scenarios failed (all of them, initially)
- What needs to be implemented

## Export Features for Implementation

```bash
npm run export:features
```

This creates `exports/` with:
- All .feature files organized by domain
- README.md with implementation guide
- manifest.json with metadata

Share the `exports/` directory with anyone who wants to implement the protocol.

## Next Steps

### 1. Pick a Feature to Implement

Browse `/docs` and choose a feature that interests you:

```bash
# Example: Municipal developer workflows
less ../docs/public_observer/developer_interests/scenarios/municipality.feature
```

### 2. Implement the Step Definitions

Edit `step-definitions/public-observer.test.js`:

```javascript
// Before (stub)
given(/a developer is proposing affordable housing project/, () => {
  throw new Error('NOT IMPLEMENTED: ...');
});

// After (real implementation)
given(/a developer is proposing affordable housing project/, () => {
  developer = new Developer({ project: 'affordable housing' });
  observer = new PublicObserver();
  expect(developer.project).toBeDefined();
});
```

### 3. Run Tests Again

```bash
npm run test:public-observer
```

Watch your test go from red to green!

### 4. Repeat

- Implement more step definitions
- Run tests
- See more scenarios pass
- Build the protocol piece by piece

## Common Commands

| Command | Purpose |
|---------|---------|
| `npm test` | Run all tests |
| `npm run test:public-observer` | Run Public Observer tests |
| `npm run test:governance` | Run Governance tests |
| `npm run validate-features` | Check feature file syntax |
| `npm run report:generate` | Create HTML report |
| `npm run export:features` | Package features for distribution |

## Troubleshooting

### "Cannot find module"
```bash
npm install
```

### "No feature files found"
Ensure `/docs` directory exists with .feature files:
```bash
ls -la ../docs/public_observer/
```

### Tests timeout
Increase timeout in `jest.config.js`:
```javascript
testTimeout: 30000  // 30 seconds
```

## Jenkins Pipeline

The protocol tests run automatically in Jenkins via `Jenkinsfile.protocol`:

- **Trigger**: Commits to `/docs` or `/protocol-tests`
- **Frequency**: Every 15 minutes (SCM poll)
- **Output**: HTML reports + exported features

View results in Jenkins under "Elohim Protocol" pipeline.

## Development Workflow

1. **Feature Definition** → Add/edit .feature files in `/docs`
2. **Validation** → `npm run validate-features`
3. **Test Run** → `npm run test:domain`
4. **Implementation** → Write code to make tests pass
5. **Verify** → `npm test`
6. **Export** → `npm run export:features`
7. **Share** → Distribute exports to implementers

## Need Help?

- **Documentation**: See [README.md](./README.md)
- **Examples**: Look at existing .feature files in `/docs`
- **Issues**: Check the main repository issue tracker

## Philosophy

> "The protocol is defined by its tests. When the tests pass, the protocol is implemented."

These BDD tests are not just validation - they ARE the specification. Implement the functionality that makes them pass, and you've built the Elohim Protocol.

Happy coding! 🚀
