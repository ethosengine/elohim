// Custom commands for the elohim site testing

Cypress.Commands.add('visitSite', (url) => {
  // Use provided URL, or fall back to config baseUrl, or staging default
  const targetUrl = url || Cypress.config('baseUrl') || 'https://staging.elohim.host';
  
  // Log which environment we're testing against
  const envType = targetUrl.includes('localhost') ? 'Local Development' :
                  targetUrl.includes('.code.ethosengine.com') ? 'Eclipse Che Preview' :
                  targetUrl.includes('staging.elohim.host') ? 'Staging' : 'Custom Environment';
  
  cy.log(`Testing against: ${envType} - ${targetUrl}`);
  
  // Environment-specific visit options
  const visitOptions = {
    failOnStatusCode: false,
    timeout: targetUrl.includes('.code.ethosengine.com') ? 45000 : 30000
  };
  
  cy.visit(targetUrl, visitOptions);
})

Cypress.Commands.add('checkPageLoad', () => {
  cy.get('body').should('be.visible');
  cy.document().should('have.property', 'readyState', 'complete');
})

Cypress.Commands.add('waitForAppReady', () => {
  // Wait for Angular to be ready (if applicable)
  cy.window().then((win) => {
    if (win.ng) {
      cy.log('Angular detected, waiting for stability');
      // Angular is present, wait for it to be stable
      return new Cypress.Promise((resolve) => {
        const checkStability = () => {
          if (win.ng && typeof win.ng.getTestability === 'function') {
            const testability = win.ng.getTestability(win.document.body);
            if (testability && typeof testability.whenStable === 'function') {
              testability.whenStable(resolve);
            } else {
              setTimeout(resolve, 1000);
            }
          } else {
            setTimeout(resolve, 1000);
          }
        };
        checkStability();
      });
    }
  });
  
  // Additional check for page readiness
  cy.get('body').should('be.visible');
})

// Get the Doorway API host based on current environment
Cypress.Commands.add('getDoorwayHost', () => {
  // Check for explicit doorway host in environment
  const explicitDoorway = Cypress.env('DOORWAY_HOST');
  if (explicitDoorway) {
    return explicitDoorway;
  }

  // Auto-detect from base URL
  const baseUrl = Cypress.config('baseUrl') || '';

  if (baseUrl.includes('alpha.elohim.host')) {
    return 'https://doorway-dev.elohim.host';
  } else if (baseUrl.includes('staging.elohim.host')) {
    return 'https://doorway-staging.elohim.host';
  } else if (baseUrl.includes('elohim.host') && !baseUrl.includes('doorway')) {
    return 'https://doorway.elohim.host';
  } else if (baseUrl.includes('localhost')) {
    // Local development - use local doorway or dev
    return Cypress.env('LOCAL_DOORWAY_HOST') || 'http://localhost:8888';
  }

  // Fallback: try to construct doorway URL from base URL
  return baseUrl.replace('://', '://doorway.');
})

// Check if we're in BDD pipeline mode
Cypress.Commands.add('isBDDPipeline', () => {
  return Cypress.env('ENV') === 'bdd-pipeline';
})

// Get dynamic feature manifest (if fetched)
Cypress.Commands.add('getDynamicManifest', () => {
  return cy.task('readDynamicManifest');
})

// List dynamic feature files
Cypress.Commands.add('listDynamicFeatures', () => {
  return cy.task('listDynamicFeatures');
})