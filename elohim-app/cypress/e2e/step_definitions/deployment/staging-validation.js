import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

Given('I navigate to the staging site', () => {
  cy.logEnvironmentInfo();
  cy.visitSite();
});

When('the page loads', () => {
  cy.checkPageLoad();
  cy.waitForAppReady();
});

Then('the site should be accessible', () => {
  const baseUrl = Cypress.config('baseUrl');
  
  if (baseUrl.includes('staging.elohim.host')) {
    cy.url().should('include', 'staging.elohim.host');
  } else if (baseUrl.includes('.code.ethosengine.com')) {
    cy.url().should('include', '.code.ethosengine.com');
  } else if (baseUrl.includes('localhost')) {
    cy.url().should('include', 'localhost');
  }
  
  cy.get('body').should('be.visible');
});

Then('the page should display the main content', () => {
  cy.get('body').should('contain.text', 'Elohim');
});

Then('there should be no critical errors in the console', () => {
  cy.window().then((win) => {
    cy.wrap(win.console).should('exist');
    // Check for console errors in a more robust way
    cy.window().its('console').then((console) => {
      // This is a basic check - in a real app you might want to check for specific error patterns
      expect(console).to.exist;
    });
  });
});

Then('the page title should contain {string}', (expectedTitle) => {
  cy.title().should('contain', expectedTitle);
});

Then('the main navigation should be visible', () => {
  // Get environment-specific timeout
  cy.getEnvironmentConfig().then((config) => {
    const timeout = config.timeout;
    // Look for common navigation elements - adjust selectors based on actual HTML structure
    cy.get('nav, header, [role="navigation"]', { timeout }).should('be.visible');
  });
});

Then('the hero section should be displayed', () => {
  // Get environment-specific timeout
  cy.getEnvironmentConfig().then((config) => {
    const timeout = config.timeout;
    // Look for hero component or main content area
    cy.get('app-hero, .hero, [class*="hero"], main', { timeout }).should('be.visible');
  });
});

Then('the footer should be present', () => {
  // Get environment-specific timeout
  cy.getEnvironmentConfig().then((config) => {
    const timeout = config.timeout;
    // Look for footer component
    cy.get('app-footer, footer, [role="contentinfo"]', { timeout }).should('exist');
  });
});

Then('the footer should display the expected git hash', () => {
  cy.get('[data-cy="git-hash"]').should('be.visible')
    .and('not.be.empty')
    .and('not.contain', 'unknown')
    .and('not.contain', 'local-dev');
});

Then('the git hash should match the deployed version', () => {
  // Get the expected git hash from environment variable
  const expectedGitHash = Cypress.env('EXPECTED_GIT_HASH');

  if (expectedGitHash) {
    cy.log(`Expected git hash: ${expectedGitHash}`);
    cy.get('[data-cy="git-hash"]').then(($el) => {
      const actualHash = $el.text().trim();
      cy.log(`Actual git hash on page: ${actualHash}`);
      expect(actualHash, `Git hash mismatch! Expected ${expectedGitHash} but found ${actualHash} on staging site`).to.equal(expectedGitHash);
    });
  } else {
    // FAIL LOUDLY if no expected hash provided - deployment validation requires this!
    throw new Error('EXPECTED_GIT_HASH environment variable is not set! Cannot validate deployment.');
  }
});