# E2E Testing with Cypress + Cucumber

This directory contains end-to-end tests using Cypress and Cucumber for the Elohim application. The tests are designed to work across multiple environments: Eclipse Che development, local development, and CI/CD pipeline staging validation.

## Environment Support

The test suite automatically detects and adapts to different environments:

### 🔧 Eclipse Che Development
- **Auto-detection**: Detects Eclipse Che environment via `CHE_WORKSPACE_NAME` or `DEVFILE_FILENAME`
- **Default URL**: `http://localhost:4200` (for local dev server)
- **Custom Preview URL**: Use `CYPRESS_baseUrl` environment variable
- **Timeouts**: Extended timeouts (45s) for network latency

### 🏠 Local Development  
- **URL**: `http://localhost:4200`
- **Timeouts**: Fast timeouts (10s) for local testing
- **Usage**: Perfect for rapid test development and debugging

### 🚀 CI/CD Pipeline (Jenkins)
- **URL**: `https://staging.elohim.host`
- **Purpose**: Validates staging deployment before production
- **Timeouts**: Standard timeouts (30s) for remote testing

## Usage Commands

### Eclipse Che Development

```bash
# Test against local dev server (port 4200)
npm run e2e:local

# Open Cypress GUI for test development
npm run e2e:dev

# Test against Eclipse Che preview URL
CYPRESS_baseUrl=https://your-workspace-preview.code.ethosengine.com npm run e2e:preview

# Alternative using environment variable
export CYPRESS_baseUrl=https://your-workspace-preview.code.ethosengine.com
npm run e2e:preview
```

### Jenkins Pipeline
```bash
# Staging validation (automatically used in pipeline)
npm run e2e:staging
```

### Manual Testing
```bash
# Open Cypress Test Runner GUI
npm run cypress:open

# Run all tests headlessly
npm run cypress:run

# Run specific feature
npx cypress run --spec "cypress/e2e/staging-validation.feature"
```

## File Structure

```
cypress/
├── e2e/
│   ├── staging-validation.feature     # Cucumber feature file
│   └── step_definitions/
│       └── staging-validation.js      # Step definitions
├── support/
│   ├── e2e.js                        # Main support file
│   ├── commands.js                   # Custom Cypress commands
│   └── environment.js                # Environment detection utilities
├── cypress.config.js                 # Cypress configuration
└── README.md                         # This file
```

## Test Scenarios

### Staging Site Validation
- ✅ Site loads successfully
- ✅ Essential page elements are present
- ✅ No critical console errors
- ✅ Page title contains "Elohim"
- ✅ Navigation, hero, and footer are visible

## Environment Configuration

The test suite automatically configures timeouts and retry logic based on the detected environment:

| Environment | Timeout | Retries | Wait Time |
|-------------|---------|---------|-----------|
| Local | 10s | 2 | 2s |
| Eclipse Che | 45s | 3 | 5s |
| Staging/CI | 30s | 2 | 3s |

## Debugging

### View Environment Detection
The tests automatically log environment information:
```
=== Environment Information ===
Environment: Eclipse Che Preview
Base URL: https://workspace.code.ethosengine.com
Timeout: 45000ms
Retries: 3
Is CI: false
```

### Screenshots and Videos
- Screenshots: Captured on test failure
- Videos: Disabled by default (can be enabled in `cypress.config.js`)
- Artifacts: Automatically archived in Jenkins pipeline

## Development Workflow

1. **Start your development server**
   ```bash
   npm run start  # Local: http://localhost:4200
   # or click "Open" button in Eclipse Che for preview URL
   ```

2. **Run tests during development**
   ```bash
   npm run e2e:dev  # Opens Cypress GUI
   ```

3. **Test against preview URL in Eclipse Che**
   ```bash
   # Copy the preview URL from Eclipse Che
   CYPRESS_baseUrl=https://your-preview-url.code.ethosengine.com npm run e2e:preview
   ```

4. **Pipeline validation**
   - Tests automatically run after staging deployment
   - Must pass before production deployment proceeds

## Troubleshooting

### Common Issues

**Test timeouts in Eclipse Che**
- Preview URLs can be slower due to network routing
- Tests automatically use extended timeouts for `.code.ethosengine.com` URLs

**Environment not detected correctly**  
- Check environment variables: `CHE_WORKSPACE_NAME`, `DEVFILE_FILENAME`
- Manually set: `CYPRESS_baseUrl=your-url`

**Tests fail with "baseUrl not accessible"**
- Ensure your development server is running
- For Eclipse Che, ensure the preview URL is accessible
- Check firewall/network restrictions

### Debug Commands
```bash
# Check environment detection
npx cypress run --spec cypress/e2e/staging-validation.feature --headed

# Verbose logging
DEBUG=cypress:* npx cypress run

# Test specific environment
CYPRESS_baseUrl=http://localhost:4200 npx cypress run
```