# Elohim Protocol Pipeline Guide

This document explains the **separate Jenkins pipeline** for Elohim Protocol BDD specification tests.

## Overview

The Elohim Protocol pipeline (`Jenkinsfile.protocol`) is **completely independent** from the main elohim-app pipeline (`Jenkinsfile`). It validates and tests the protocol specifications defined in `/docs` using BDD (Behavior-Driven Development) tests with intentionally failing stubs.

### Purpose

- **Validate** all .feature files for correct Gherkin syntax
- **Test** protocol specifications with stub implementations
- **Generate** HTML/JSON reports showing what needs to be implemented
- **Export** .feature files for external developers to implement

### Key Concept

All tests intentionally fail with "NOT IMPLEMENTED" errors. This is the correct behavior. The failing tests demonstrate what needs to be built to create a complete Elohim Protocol implementation.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Jenkins CI/CD                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────────────┐  ┌──────────────────────┐   │
│  │   Main Pipeline      │  │  Protocol Pipeline    │   │
│  │   (Jenkinsfile)      │  │  (Jenkinsfile.protocol)│  │
│  ├──────────────────────┤  ├──────────────────────┤   │
│  │ • Build elohim-app   │  │ • Validate .features │   │
│  │ • Run unit tests     │  │ • Run BDD stubs      │   │
│  │ • E2E tests          │  │ • Generate reports   │   │
│  │ • Deploy to envs     │  │ • Export features    │   │
│  │ • Push to Harbor     │  │                      │   │
│  └──────────────────────┘  └──────────────────────┘   │
│          ↓                          ↓                  │
│    Production App          Protocol Specs             │
└─────────────────────────────────────────────────────────┘
```

## Pipeline Stages

### 1. **Checkout**
- Clones the repository
- Counts .feature files in `/docs`

### 2. **Install Protocol Test Dependencies**
- Installs npm packages in `protocol-tests/`
- Uses Node 20 Alpine container

### 3. **Validate Feature Files**
- Checks all 252 .feature files for Gherkin syntax errors
- Reports any invalid files

### 4. **Run Protocol BDD Tests** (5 parallel stages)
- `test:public-observer` - Public Observer protocol
- `test:governance` - Governance protocol
- `test:value-scanner` - Value Scanner protocol
- `test:social-medium` - Social Medium protocol
- `test:autonomous-entity` - Autonomous Entity protocol

Each test intentionally fails with stub implementations.

### 5. **Generate Unified Report**
- Creates comprehensive HTML report
- Generates JSON reports for CI/CD integration

### 6. **Export Feature Files**
- Packages all .feature files
- Creates README and manifest
- Prepares distribution package

### 7. **Publish Reports**
- Publishes Cucumber reports to Jenkins
- Archives HTML/JSON reports as artifacts
- Archives exported feature packages

## Jenkins Configuration

### Create the Protocol Pipeline Job

1. **In Jenkins**, create a new Pipeline job:
   - Name: `Elohim-Protocol-Specifications`
   - Type: Pipeline

2. **Configure Source Control**:
   - Repository: `https://github.com/ethosengine/elohim`
   - Script Path: `Jenkinsfile.protocol`

3. **Configure Triggers**:
   - Poll SCM: `H/15 * * * *` (every 15 minutes)
   - Or use webhooks for immediate builds

4. **Add Build Parameters** (optional):
   - `PROTOCOL_VERSION` - Version tag for the protocol

### Required Jenkins Plugins

- **Kubernetes Plugin** - For pod-based agents
- **Cucumber Reports Plugin** - For BDD reports
- **Pipeline** - For Jenkinsfile support
- **Git Plugin** - For SCM integration

### Environment Requirements

- **Kubernetes Cluster** with `node-type: edge` nodes
- **Node.js 20+** Docker image
- **Memory**: 512Mi request, 2Gi limit
- **CPU**: 500m request, 2000m limit

## Local Testing

### Prerequisites

```bash
cd protocol-tests
npm install
```

### Run Validation

```bash
npm run validate-features
```

**Expected Output**:
```
=== Validation Summary ===
Total Features: 252
Valid: 245
Invalid: 7
Total Scenarios: 2839
```

### Run Tests

```bash
# All tests
npm test

# Single domain
npm run test:public-observer
```

**Expected Result**: All tests fail with "NOT IMPLEMENTED" errors. This is correct!

### Generate Reports

```bash
npm run report:generate
```

Open `reports/html/protocol-report.html` in a browser.

### Export Features

```bash
npm run export:features
```

Creates `exports/` directory with:
- All 252 .feature files
- README.md with implementation guide
- manifest.json with metadata

## Directory Structure

```
elohim/
├── Jenkinsfile                  # Main app pipeline
├── Jenkinsfile.protocol         # Protocol pipeline (NEW)
├── PROTOCOL_PIPELINE.md         # This document (NEW)
├── docs/                        # Protocol specifications (252 .feature files)
│   ├── public_observer/
│   ├── governance/
│   ├── value_scanner/
│   ├── social_medium/
│   └── autonomous_entity/
└── protocol-tests/              # BDD test suite (NEW)
    ├── README.md
    ├── QUICKSTART.md
    ├── package.json
    ├── jest.config.js
    ├── step-definitions/        # Stub implementations
    │   ├── public-observer.test.js
    │   ├── governance.test.js
    │   ├── value-scanner.test.js
    │   ├── social-medium.test.js
    │   └── autonomous-entity.test.js
    ├── scripts/                 # Utility scripts
    │   ├── validate-features.js
    │   ├── generate-report.js
    │   └── export-features.js
    ├── reports/                 # Generated (gitignored)
    └── exports/                 # Generated (gitignored)
```

## Workflow

### Developer Workflow

1. **Edit** protocol specifications in `/docs/*.feature`
2. **Commit** and push changes
3. **Jenkins** automatically runs protocol pipeline
4. **Review** reports showing failing tests
5. **Implement** step definitions to make tests pass
6. **Export** features for external implementers

### External Implementer Workflow

1. **Download** exported features from Jenkins artifacts
2. **Read** feature files to understand requirements
3. **Implement** functionality to satisfy scenarios
4. **Test** implementation against feature specifications
5. **Contribute** back to the protocol

## Differences from Main Pipeline

| Aspect | Main Pipeline | Protocol Pipeline |
|--------|--------------|------------------|
| **File** | `Jenkinsfile` | `Jenkinsfile.protocol` |
| **Purpose** | Build & deploy app | Validate protocol specs |
| **Source** | `/elohim-app` | `/docs` |
| **Tests** | Unit + E2E tests | BDD stub tests |
| **Output** | Docker images | Test reports + exports |
| **Deploy** | Kubernetes | N/A |
| **Success** | All tests pass | All tests fail (intentional) |
| **Trigger** | All branches | Docs/protocol changes |

## Monitoring

### Jenkins Dashboard

The protocol pipeline shows:
- **Blue** - Build successful (tests ran, reports generated)
- **Red** - Build failed (script error, not test failures)
- **Test Results** - All scenarios fail (expected)

### Key Metrics

- **Total Features**: 252
- **Valid Features**: 245 (7 have syntax errors)
- **Total Scenarios**: 2839
- **Passing Tests**: 0 (expected - all stubs)
- **Failing Tests**: 2839 (expected - shows work needed)

### Reports

1. **Cucumber Report**
   - Viewable in Jenkins UI
   - Shows all scenarios and their failure messages
   - Downloadable as JSON

2. **HTML Report**
   - Archived as Jenkins artifact
   - Opens in browser
   - Shows detailed test results

3. **Exported Features**
   - Archived as Jenkins artifact
   - Downloadable package
   - Ready for distribution

## Customization

### Add New Test Domain

1. Create `.feature` files in `/docs/new-domain/`
2. Create `protocol-tests/step-definitions/new-domain.test.js`
3. Add stage to `Jenkinsfile.protocol`:

```groovy
stage('Run Protocol BDD Tests - New Domain') {
    steps {
        container('node') {
            dir('protocol-tests') {
                sh 'npm run test:new-domain || true'
            }
        }
    }
}
```

4. Add script to `package.json`:

```json
"test:new-domain": "jest --testPathPattern=new-domain --verbose"
```

### Customize Reporting

Edit `protocol-tests/scripts/generate-report.js` to:
- Change report theme
- Add custom metadata
- Include screenshots
- Modify layout

### Change Test Framework

Replace `jest-cucumber` with:
- **Cucumber.js** - JavaScript BDD framework
- **Behave** - Python BDD framework
- **SpecFlow** - .NET BDD framework

Update `step-definitions/` accordingly.

## Troubleshooting

### "No feature files found"

**Cause**: `/docs` directory empty or missing
**Fix**: Ensure repository has `/docs` with .feature files

### "npm install fails"

**Cause**: Network issues or npm registry down
**Fix**: Use offline npm cache or private registry

### "All tests pass"

**Cause**: Stub implementations were replaced with real code
**Fix**: This is actually success! You implemented the protocol.

### "Build fails immediately"

**Cause**: Kubernetes pod can't start
**Fix**: Check node selectors, resource limits, image availability

## Performance

- **Average Build Time**: 5-8 minutes
- **Peak Memory**: ~1.5Gi
- **CPU Usage**: ~1 core
- **Artifact Size**: ~500KB (reports + exports)

## Security

- **No secrets required** - Read-only operations
- **No deployments** - Only generates reports
- **No external access** - Self-contained tests
- **Safe to run on PRs** - No side effects

## Future Enhancements

- [ ] Integration with SonarQube for quality metrics
- [ ] Automated PR comments with test results
- [ ] Trend analysis across builds
- [ ] Feature coverage metrics
- [ ] Automated export to external repositories
- [ ] Protocol versioning based on feature changes

## Resources

- **Main Documentation**: `/docs/manifesto.md`
- **Test Guide**: `/protocol-tests/README.md`
- **Quick Start**: `/protocol-tests/QUICKSTART.md`
- **GitHub Repository**: https://github.com/ethosengine/elohim

## Support

For questions about:
- **Pipeline configuration**: See Jenkins documentation
- **Protocol specifications**: See `/docs/manifesto.md`
- **Test implementation**: See `/protocol-tests/README.md`
- **Feature syntax errors**: Run `npm run validate-features`

---

**Remember**: Failing tests are the specification. When they pass, the protocol is built.
