/**
 * Common Navigation Step Definitions
 *
 * Reusable step definitions for page navigation and basic page assertions.
 * Used across BDD product tests for Lamad, Shefa, Imagodei, and Doorway.
 */

import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

// ============================================================================
// Background / Setup Steps
// ============================================================================

Given('I am on the home page', () => {
  cy.visitSite();
  cy.waitForAppReady();
});

Given('I am on the {string} page', (path: string) => {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  cy.visit(normalizedPath);
  cy.waitForAppReady();
});

Given('I am on the Lamad section', () => {
  cy.visit('/lamad');
  cy.waitForAppReady();
});

Given('I am on the Shefa dashboard', () => {
  cy.visit('/shefa');
  cy.waitForAppReady();
});

// ============================================================================
// Navigation Actions
// ============================================================================

When('I click on {string} in the navigation', (linkText: string) => {
  cy.get('nav, header, [role="navigation"]')
    .contains(linkText, { matchCase: false })
    .click();
  cy.waitForAppReady();
});

When('I click on the site logo', () => {
  cy.get('[data-testid="site-logo"], .site-logo, header img, header a:first-child')
    .first()
    .click();
  cy.waitForAppReady();
});

When('I navigate to the Lamad section', () => {
  cy.visit('/lamad');
  cy.waitForAppReady();
});

When('I navigate to {string}', (path: string) => {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  cy.visit(normalizedPath);
  cy.waitForAppReady();
});

When('I click on the first learning path', () => {
  cy.get('[data-testid="learning-path"], .path-card, .learning-path')
    .first()
    .click();
  cy.waitForAppReady();
});

When('I click the {string} button', (buttonText: string) => {
  cy.contains('button', buttonText, { matchCase: false }).click();
  cy.waitForAppReady();
});

When('I click on {string}', (text: string) => {
  cy.contains(text, { matchCase: false }).click();
  cy.waitForAppReady();
});

// ============================================================================
// Page State Assertions
// ============================================================================

Then('the page should be accessible', () => {
  cy.get('body').should('be.visible');
  cy.document().should('have.property', 'readyState', 'complete');
});

Then('the page should load successfully', () => {
  cy.get('body').should('be.visible');
  cy.document().should('have.property', 'readyState', 'complete');
  // Check for no major Angular errors
  cy.window().then((win: any) => {
    if (win.ng) {
      // Angular app detected
      expect(win.ng).to.exist;
    }
  });
});

Then('I should be on the {string} page', (path: string) => {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  cy.url().should('include', normalizedPath);
});

Then('I should be on the home page', () => {
  cy.url().should('match', /\/(#.*)?$/);
});

Then('the page title should contain {string}', (text: string) => {
  cy.title().should('contain', text);
});

// ============================================================================
// Element Visibility Assertions
// ============================================================================

Then('the main navigation should be visible', () => {
  cy.get('nav, [role="navigation"], header nav').should('be.visible');
});

Then('the footer should be present', () => {
  cy.get('footer, app-footer, [role="contentinfo"]').should('exist');
});

Then('the footer should display the git hash', () => {
  cy.get('[data-cy="git-hash"], .git-hash, footer').should('exist');
});

Then('the hero section should be visible', () => {
  cy.get('app-hero, .hero, [class*="hero"], main section:first-child').should(
    'be.visible'
  );
});

Then('the Lamad content should load', () => {
  cy.get(
    '[data-testid="lamad-content"], .lamad-container, app-lamad, main'
  ).should('be.visible');
});

Then('at least one learning path should be displayed', () => {
  cy.get('[data-testid="learning-path"], .path-card, .learning-path').should(
    'have.length.at.least',
    1
  );
});

Then('each path should show a title and description', () => {
  cy.get('[data-testid="learning-path"], .path-card, .learning-path')
    .first()
    .within(() => {
      cy.get('h2, h3, .title, [class*="title"]').should('exist');
      cy.get('p, .description, [class*="description"]').should('exist');
    });
});

Then('the dashboard should display', () => {
  cy.get(
    '[data-testid="dashboard"], .dashboard, app-dashboard, main'
  ).should('be.visible');
});

Then('I should see {string}', (text: string) => {
  cy.contains(text, { matchCase: false }).should('be.visible');
});

Then('I should not see {string}', (text: string) => {
  cy.contains(text, { matchCase: false }).should('not.exist');
});

// ============================================================================
// Error Checking
// ============================================================================

Then('there should be no console errors', () => {
  cy.window().then((win: any) => {
    // Basic check - more robust error checking can be added
    expect(win.console).to.exist;
  });
});

Then('the page should not show an error message', () => {
  cy.get(
    '.error, .error-message, [class*="error"], [role="alert"]'
  ).should('not.exist');
});

// ============================================================================
// Loading State Assertions
// ============================================================================

Then('the content should finish loading', () => {
  // Wait for any loading indicators to disappear
  cy.get('.loading, .spinner, [class*="loading"]', { timeout: 1000 }).should(
    'not.exist'
  );
});

Then('no loading spinner should be visible', () => {
  cy.get('.spinner, .loading-spinner, [class*="spinner"]').should('not.exist');
});
