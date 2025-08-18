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