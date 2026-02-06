const { defineConfig } = require('cypress');
const createBundler = require('@bahmutov/cypress-esbuild-preprocessor');
const { addCucumberPreprocessorPlugin } = require('@badeball/cypress-cucumber-preprocessor');
const { createEsbuildPlugin } = require('@badeball/cypress-cucumber-preprocessor/esbuild');
const fs = require('fs');
const path = require('path');

// Environment detection and URL resolution
function getBaseUrl() {
  // Priority: Command line env > Environment variable > Default staging
  if (process.env.CYPRESS_baseUrl) {
    return process.env.CYPRESS_baseUrl;
  }

  // Check for Eclipse Che environment indicators
  if (process.env.CHE_WORKSPACE_NAME || process.env.DEVFILE_FILENAME) {
    // Eclipse Che detected - default to localhost for development
    return 'http://localhost:4200';
  }

  // Default to staging for Jenkins/CI environments
  return 'https://staging.elohim.host';
}

// Check if running in BDD pipeline mode
function isBDDPipeline() {
  return process.env.CYPRESS_ENV === 'bdd-pipeline';
}

// Environment-specific timeout configurations
function getTimeoutConfig(baseUrl) {
  const isLocal = baseUrl.includes('localhost') || baseUrl.includes('127.0.0.1');
  const isEclipseChe = baseUrl.includes('.code.ethosengine.com');
  const isBDD = isBDDPipeline();

  if (isLocal && !isBDD) {
    return {
      defaultCommandTimeout: 5000,
      requestTimeout: 8000,
      responseTimeout: 8000,
      pageLoadTimeout: 10000
    };
  } else if (isEclipseChe) {
    return {
      defaultCommandTimeout: 15000,
      requestTimeout: 15000,
      responseTimeout: 15000,
      pageLoadTimeout: 30000
    };
  } else if (isBDD) {
    // BDD pipeline: extended timeouts for CI stability
    return {
      defaultCommandTimeout: 15000,
      requestTimeout: 20000,
      responseTimeout: 20000,
      pageLoadTimeout: 30000
    };
  } else {
    // Staging/Production
    return {
      defaultCommandTimeout: 10000,
      requestTimeout: 10000,
      responseTimeout: 10000,
      pageLoadTimeout: 20000
    };
  }
}

const baseUrl = getBaseUrl();
const timeoutConfig = getTimeoutConfig(baseUrl);

module.exports = defineConfig({
  e2e: {
    baseUrl: baseUrl,
    specPattern: 'cypress/e2e/**/*.feature',
    supportFile: 'cypress/support/e2e.js',
    video: isBDDPipeline(), // Record video in BDD pipeline mode
    screenshotOnRunFailure: true,
    viewportWidth: 1280,
    viewportHeight: 720,
    retries: {
      runMode: isBDDPipeline() ? 2 : 0, // Retry failed tests in CI
      openMode: 0
    },
    ...timeoutConfig,

    async setupNodeEvents(on, config) {
      await addCucumberPreprocessorPlugin(on, config);

      on('file:preprocessor',
        createBundler({
          plugins: [createEsbuildPlugin(config)],
        })
      );

      // Register custom tasks
      on('task', {
        log(message) {
          console.log(message);
          return null;
        },
        readDynamicManifest() {
          const manifestPath = path.join(__dirname, 'cypress/e2e/features/dynamic/manifest.json');
          if (fs.existsSync(manifestPath)) {
            try {
              return JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));
            } catch (e) {
              console.log('Failed to read dynamic manifest:', e.message);
              return null;
            }
          }
          return null;
        },
        listDynamicFeatures() {
          const dynamicDir = path.join(__dirname, 'cypress/e2e/features/dynamic');
          if (!fs.existsSync(dynamicDir)) {
            return [];
          }
          try {
            const files = fs.readdirSync(dynamicDir, { recursive: true });
            return files.filter(f => f.toString().endsWith('.feature'));
          } catch (e) {
            return [];
          }
        }
      });

      // Determine environment type
      const envType = isBDDPipeline() ? 'BDD Pipeline' :
                      config.baseUrl.includes('localhost') ? 'Local Development' :
                      config.baseUrl.includes('.code.ethosengine.com') ? 'Eclipse Che' : 'Staging/CI';

      // Log the resolved configuration for debugging
      console.log(`Cypress E2E Configuration:`);
      console.log(`  Base URL: ${config.baseUrl}`);
      console.log(`  Environment: ${envType}`);
      console.log(`  Command Timeout: ${config.defaultCommandTimeout}ms`);
      console.log(`  Video Recording: ${config.video}`);
      console.log(`  Retries: ${config.retries?.runMode || 0}`);

      // Ensure cucumber reports are enabled
      console.log('Cucumber preprocessor configuration loaded');

      return config;
    },
  },
});