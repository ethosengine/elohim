const { defineConfig } = require('cypress');
const createBundler = require('@bahmutov/cypress-esbuild-preprocessor');
const { addCucumberPreprocessorPlugin } = require('@badeball/cypress-cucumber-preprocessor');
const { createEsbuildPlugin } = require('@badeball/cypress-cucumber-preprocessor/esbuild');

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

// Environment-specific timeout configurations
function getTimeoutConfig(baseUrl) {
  const isLocal = baseUrl.includes('localhost') || baseUrl.includes('127.0.0.1');
  const isEclipseChe = baseUrl.includes('.code.ethosengine.com');
  
  if (isLocal) {
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
    video: false,
    screenshotOnRunFailure: true,
    viewportWidth: 1280,
    viewportHeight: 720,
    ...timeoutConfig,
    
    async setupNodeEvents(on, config) {
      await addCucumberPreprocessorPlugin(on, config);
      
      on('file:preprocessor',
        createBundler({
          plugins: [createEsbuildPlugin(config)],
        })
      );

      // Log the resolved configuration for debugging
      console.log(`Cypress E2E Configuration:`);
      console.log(`  Base URL: ${config.baseUrl}`);
      console.log(`  Environment: ${config.baseUrl.includes('localhost') ? 'Local Development' : 
                                   config.baseUrl.includes('.code.ethosengine.com') ? 'Eclipse Che' : 'Staging/CI'}`);
      console.log(`  Command Timeout: ${config.defaultCommandTimeout}ms`);

      return config;
    },
  },
});