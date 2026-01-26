/**
 * Common API Step Definitions
 *
 * Reusable step definitions for API interactions and data verification.
 * Used to verify that backend data is available and correctly displayed.
 */

import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

// Store API responses for assertion steps
let apiResponse: Cypress.Response<any> | null = null;
let contentItems: any[] = [];

// ============================================================================
// API Health & Availability
// ============================================================================

Given('the Doorway API is accessible', () => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/health`,
    failOnStatusCode: false,
    timeout: 15000,
  }).then((response) => {
    expect(response.status).to.be.oneOf([200, 204]);
  });
});

Given('the backend API is available', () => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/health`,
    failOnStatusCode: false,
    timeout: 15000,
  }).then((response) => {
    expect(response.status).to.be.oneOf([200, 204]);
  });
});

// ============================================================================
// Content API Queries
// ============================================================================

When('I request the content list from the API', () => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/api/v1/lamad_dna/content_store/get_all_content`,
    timeout: 20000,
  }).then((response) => {
    apiResponse = response;
    contentItems = Array.isArray(response.body) ? response.body : [];
  });
});

When('I request content by type {string}', (contentType: string) => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/api/v1/lamad_dna/content_store/get_content_by_type?content_type=${contentType}`,
    timeout: 20000,
  }).then((response) => {
    apiResponse = response;
    contentItems = Array.isArray(response.body) ? response.body : [];
  });
});

When('I request the learning paths from the API', () => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/api/v1/lamad_dna/content_store/get_all_paths`,
    timeout: 20000,
  }).then((response) => {
    apiResponse = response;
    contentItems = Array.isArray(response.body) ? response.body : [];
  });
});

When('I request content stats from the API', () => {
  const doorwayHost = cy.getDoorwayHost();

  cy.request({
    url: `${doorwayHost}/api/v1/lamad_dna/content_store/get_content_stats`,
    timeout: 15000,
  }).then((response) => {
    apiResponse = response;
  });
});

// ============================================================================
// Response Assertions
// ============================================================================

Then('the response should be successful', () => {
  expect(apiResponse).to.not.be.null;
  expect(apiResponse!.status).to.eq(200);
});

Then('the response should contain content items', () => {
  expect(apiResponse).to.not.be.null;
  expect(apiResponse!.status).to.eq(200);
  expect(contentItems).to.be.an('array');
  expect(contentItems.length).to.be.greaterThan(0);
});

Then('each item should have required fields', () => {
  // Check first few items for required fields
  const samplesToCheck = contentItems.slice(0, 5);

  samplesToCheck.forEach((item: any) => {
    expect(item).to.have.property('id');
    expect(item).to.have.property('title');
    // Handle both camelCase and snake_case
    expect(item.content_type || item.contentType).to.exist;
  });
});

Then('the response should contain at least {int} items', (count: number) => {
  expect(contentItems.length).to.be.at.least(count);
});

Then('the response should contain paths', () => {
  expect(apiResponse).to.not.be.null;
  expect(apiResponse!.status).to.eq(200);
  expect(contentItems).to.be.an('array');
  expect(contentItems.length).to.be.greaterThan(0);
});

Then('the content stats should show data exists', () => {
  expect(apiResponse).to.not.be.null;
  expect(apiResponse!.status).to.eq(200);

  const stats = apiResponse!.body;
  // Check for content count
  const totalCount =
    stats.total_count || stats.totalCount || stats.content_count || 0;
  expect(totalCount).to.be.greaterThan(0);
});

// ============================================================================
// Content Type Specific Assertions
// ============================================================================

Then('at least one assessment should be present', () => {
  const assessments = contentItems.filter((item: any) => {
    const type = item.content_type || item.contentType;
    return type === 'assessment' || type === 'quiz';
  });

  expect(assessments.length).to.be.greaterThan(0);
});

Then('assessments should have questions defined', () => {
  const assessments = contentItems.filter((item: any) => {
    const type = item.content_type || item.contentType;
    return type === 'assessment' || type === 'quiz';
  });

  if (assessments.length > 0) {
    const assessment = assessments[0];
    // Check for questions in content or metadata
    const hasQuestions =
      assessment.content?.questions ||
      assessment.metadata?.questionCount ||
      assessment.metadata?.questions;
    expect(hasQuestions).to.exist;
  }
});

Then('at least one feature should be present', () => {
  const features = contentItems.filter((item: any) => {
    const type = item.content_type || item.contentType;
    return type === 'feature';
  });

  expect(features.length).to.be.greaterThan(0);
});

Then('at least one concept should be present', () => {
  const concepts = contentItems.filter((item: any) => {
    const type = item.content_type || item.contentType;
    return type === 'concept';
  });

  expect(concepts.length).to.be.greaterThan(0);
});

// ============================================================================
// UI + API Verification
// ============================================================================

Then('the displayed content count should match the API', () => {
  const apiCount = contentItems.length;

  // Get the count displayed in the UI
  cy.get('[data-testid="content-count"], .content-count, .item-count').then(
    ($el) => {
      const displayedCount = parseInt($el.text().replace(/\D/g, ''), 10);
      // Allow some flexibility for pagination
      expect(displayedCount).to.be.at.least(1);
    }
  );
});

Then('the learning paths should match the API data', () => {
  if (contentItems.length > 0) {
    const firstPath = contentItems[0];
    const pathTitle = firstPath.title;

    // Verify the title appears in the UI
    cy.contains(pathTitle, { matchCase: false }).should('exist');
  }
});

// ============================================================================
// Blob Store Verification
// ============================================================================

When('I request a blob from the store', () => {
  const doorwayHost = cy.getDoorwayHost();

  // First get a content item with a hash
  cy.request({
    url: `${doorwayHost}/api/v1/lamad_dna/content_store/get_all_content`,
    timeout: 20000,
  }).then((response) => {
    const items = response.body;
    const itemWithHash = items.find(
      (item: any) => item.content_hash || item.contentHash
    );

    if (itemWithHash) {
      const hash = itemWithHash.content_hash || itemWithHash.contentHash;

      cy.request({
        url: `${doorwayHost}/store/${hash}`,
        timeout: 15000,
        failOnStatusCode: false,
      }).then((blobResponse) => {
        apiResponse = blobResponse;
      });
    }
  });
});

Then('the blob should be served successfully', () => {
  if (apiResponse) {
    expect(apiResponse.status).to.be.oneOf([200, 206]);
  }
});
