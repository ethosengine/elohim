// Environment detection and URL utilities for Cypress tests

export const Environment = {
  // Detect current environment type
  detectEnvironment() {
    const baseUrl = Cypress.config('baseUrl') || '';
    
    if (baseUrl.includes('localhost') || baseUrl.includes('127.0.0.1')) {
      return 'local';
    } else if (baseUrl.includes('.code.ethosengine.com')) {
      return 'eclipse-che';
    } else if (baseUrl.includes('staging.elohim.host')) {
      return 'staging';
    } else if (baseUrl.includes('elohim.host')) {
      return 'production';
    }
    return 'unknown';
  },

  // Get environment-specific configuration
  getEnvironmentConfig() {
    const envType = this.detectEnvironment();
    
    const configs = {
      local: {
        name: 'Local Development',
        waitTime: 2000,
        retries: 2,
        timeout: 10000
      },
      'eclipse-che': {
        name: 'Eclipse Che Preview',
        waitTime: 5000,
        retries: 3,
        timeout: 45000
      },
      staging: {
        name: 'Staging Environment',
        waitTime: 3000,
        retries: 2,
        timeout: 30000
      },
      production: {
        name: 'Production Environment',
        waitTime: 3000,
        retries: 1,
        timeout: 30000
      },
      unknown: {
        name: 'Unknown Environment',
        waitTime: 3000,
        retries: 2,
        timeout: 30000
      }
    };
    
    return configs[envType];
  },

  // Check if we're running in Eclipse Che
  isEclipseChe() {
    return this.detectEnvironment() === 'eclipse-che';
  },

  // Check if we're running locally
  isLocal() {
    return this.detectEnvironment() === 'local';
  },

  // Check if we're running in CI/CD
  isCI() {
    // Use Cypress.env() instead of process.env for browser compatibility
    return Cypress.env('CI') || Cypress.env('JENKINS_URL') || Cypress.env('GITHUB_ACTIONS');
  },

  // Generate Eclipse Che preview URL pattern
  generatePreviewUrl(workspaceId, port = 4200) {
    const basePattern = '.code.ethosengine.com';
    return `https://${workspaceId}-${port}${basePattern}`;
  },

  // Log environment information
  logEnvironmentInfo() {
    const config = this.getEnvironmentConfig();
    const baseUrl = Cypress.config('baseUrl');
    
    cy.log('=== Environment Information ===');
    cy.log(`Environment: ${config.name}`);
    cy.log(`Base URL: ${baseUrl}`);
    cy.log(`Timeout: ${config.timeout}ms`);
    cy.log(`Retries: ${config.retries}`);
    cy.log(`Is CI: ${this.isCI()}`);
  }
};

// Add environment commands
Cypress.Commands.add('detectEnvironment', () => {
  return Environment.detectEnvironment();
});

Cypress.Commands.add('getEnvironmentConfig', () => {
  return Environment.getEnvironmentConfig();
});

Cypress.Commands.add('logEnvironmentInfo', () => {
  Environment.logEnvironmentInfo();
});