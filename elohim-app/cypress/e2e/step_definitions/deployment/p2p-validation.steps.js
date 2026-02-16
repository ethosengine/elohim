import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

let healthResponse;

Given('the doorway health endpoint is accessible', () => {
  const baseUrl = Cypress.config('baseUrl');
  cy.request({
    url: `${baseUrl}/health`,
    timeout: 30000,
    failOnStatusCode: false,
  }).then((resp) => {
    expect(resp.status).to.eq(200);
    healthResponse = resp.body;
    cy.log(`Health endpoint responded: ${JSON.stringify(healthResponse).substring(0, 200)}`);
  });
});

When('I check the P2P status', () => {
  // P2P status is embedded in the health response
  if (typeof healthResponse === 'string') {
    healthResponse = JSON.parse(healthResponse);
  }
  cy.log(`P2P status: ${JSON.stringify(healthResponse.p2p)}`);
});

Then('connected_peers should be greater than 0', () => {
  expect(healthResponse.p2p, 'P2P status should be present in health response').to.exist;
  expect(healthResponse.p2p.connected_peers, 'Should have at least 1 connected peer').to.be
    .greaterThan(0);
  cy.log(`Connected peers: ${healthResponse.p2p.connected_peers}`);
  cy.log(`Peer ID: ${healthResponse.p2p.peer_id}`);
});

Then('sync_documents count should be available', () => {
  expect(healthResponse.p2p, 'P2P status should be present in health response').to.exist;
  expect(healthResponse.p2p.sync_documents, 'sync_documents should be a number').to.be.a(
    'number',
  );
  cy.log(`Sync documents: ${healthResponse.p2p.sync_documents}`);
});
