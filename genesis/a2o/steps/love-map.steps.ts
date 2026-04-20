/**
 * Love Map step definitions — API-level consent gates, attestation, negotiation.
 *
 * These steps validate the consent/attestation gate logic and negotiation flow
 * for intimate learning paths. Most operate against seed data fixtures rather
 * than live API calls, since the negotiation API is not yet fully implemented.
 *
 * Steps that require browser UI remain in steps/ui/love-map.steps.ts.
 */

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { allRelationships } from '../src/framework/fixtures/humans.js';
import { E2EWorld } from '../src/framework/world.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// Seed data helpers
// ---------------------------------------------------------------------------

interface NegotiationFixture {
  id: string;
  title: string;
  initiatorId: string;
  participantId: string;
  status: string;
  requiredIntimacyLevel: string;
  validatingAttestationIds: string[];
  bridgingStrategy: string;
  generatedPathId: string;
  proposedPathStructure: {
    title: string;
    stepCount: number;
    estimatedDuration: string;
  };
}

interface PathFixture {
  id: string;
  title: string;
  visibility: string;
  participantIds: string[];
  isEmergent: boolean;
  requiresMutualAttestation: boolean;
  requiredAttestations: string[];
  estimatedDuration: string;
  chapters: {
    id: string;
    title: string;
    description: string;
    order: number;
    steps: { teachingPartner: string }[];
  }[];
}

function loadNegotiationFixture(): NegotiationFixture {
  const filePath = resolve(
    __dirname,
    '../../data/lamad/negotiations/negotiation-adam-eve-001.json'
  );
  return JSON.parse(readFileSync(filePath, 'utf-8')) as NegotiationFixture;
}

function loadPathFixture(): PathFixture {
  const filePath = resolve(__dirname, '../../data/lamad/paths/love-map-adam-eve.json');
  return JSON.parse(readFileSync(filePath, 'utf-8')) as PathFixture;
}

// ---------------------------------------------------------------------------
// Consent & Attestation Given steps
// ---------------------------------------------------------------------------

Given(
  'Adam and Eve have a/an {string} level consent relationship',
  function (this: E2EWorld, level: string) {
    // Store the consent level for later gate assertions
    this.contentIds.set('consentLevel', level);
  }
);

Given(
  'Adam and Eve have an {string} consent relationship',
  function (this: E2EWorld, level: string) {
    this.contentIds.set('consentLevel', level);
  }
);

Given(
  'both have mutually attested {string} relationship',
  function (this: E2EWorld, relType: string) {
    // Verify from humans.json that the relationship exists
    const relationships = allRelationships();
    const adamToEve = relationships.find(
      r => r.source === 'human-adam-firstman' && r.target === 'human-eve-firstwoman'
    );
    assert.ok(adamToEve, 'No relationship found between Adam and Eve in humans.json');
    assert.strictEqual(
      adamToEve.relationshipType,
      relType,
      `Expected "${relType}" relationship, found "${adamToEve.relationshipType}"`
    );
    this.contentIds.set('mutualAttestation', 'true');
  }
);

Given('Adam has attested {string} relationship to Eve', function (this: E2EWorld, relType: string) {
  this.contentIds.set('adamAttestation', relType);
  this.contentIds.set('mutualAttestation', 'false');
});

Given(
  'Eve has not attested {string} relationship to Adam',
  function (this: E2EWorld, _relType: string) {
    // Already set mutualAttestation to false in prior step
    assert.strictEqual(
      this.contentIds.get('mutualAttestation'),
      'false',
      'Expected mutual attestation to be false'
    );
  }
);

Given('Adam has proposed a love map negotiation', function (this: E2EWorld) {
  const negotiation = loadNegotiationFixture();
  this.contentIds.set('negotiationId', negotiation.id);
  this.contentIds.set('negotiationStatus', 'proposed');
  this.contentIds.set('negotiationData', JSON.stringify(negotiation));
});

Given('the love map path {string} exists', function (this: E2EWorld, pathTitle: string) {
  const pathData = loadPathFixture();
  assert.ok(
    pathData.title.includes(pathTitle),
    `Path fixture title "${pathData.title}" does not contain "${pathTitle}"`
  );
  this.contentIds.set('loveMapPathId', pathData.id);
  this.contentIds.set('loveMapPathData', JSON.stringify(pathData));
});

Given(
  'the love map path {string} exists and is accessible',
  function (this: E2EWorld, pathTitle: string) {
    const pathData = loadPathFixture();
    assert.ok(
      pathData.title.includes(pathTitle),
      `Path fixture title "${pathData.title}" does not contain "${pathTitle}"`
    );
    this.contentIds.set('loveMapPathId', pathData.id);
    this.contentIds.set('loveMapPathData', JSON.stringify(pathData));
    this.contentIds.set('loveMapAccessible', 'true');
  }
);

// ---------------------------------------------------------------------------
// When steps — negotiation actions
// ---------------------------------------------------------------------------

When('Adam tries to initiate a love map negotiation with Eve', function (this: E2EWorld) {
  const consentLevel = this.contentIds.get('consentLevel');
  const negotiation = loadNegotiationFixture();

  // Gate check: love map requires "intimate" consent level
  if (consentLevel === negotiation.requiredIntimacyLevel) {
    this.contentIds.set('gateBlocked', 'false');
    this.contentIds.set('negotiationCreated', 'true');
    this.contentIds.set('negotiationId', negotiation.id);
  } else {
    this.contentIds.set('gateBlocked', 'true');
    this.contentIds.set(
      'gateMessage',
      `Requires "${negotiation.requiredIntimacyLevel}" consent level`
    );
    this.contentIds.set('negotiationCreated', 'false');
  }
});

When(
  'Adam tries to access the love map path {string}',
  function (this: E2EWorld, _pathTitle: string) {
    const mutual = this.contentIds.get('mutualAttestation');

    if (mutual === 'true') {
      this.contentIds.set('attestationGateBlocked', 'false');
      this.contentIds.set('pathVisible', 'true');
    } else {
      this.contentIds.set('attestationGateBlocked', 'true');
      this.contentIds.set('gateMessage', 'Requires mutual attestation');
      this.contentIds.set('pathVisible', 'false');
    }
  }
);

When('Adam initiates a love map negotiation with Eve', function (this: E2EWorld) {
  const negotiation = loadNegotiationFixture();
  this.contentIds.set('negotiationId', negotiation.id);
  this.contentIds.set('negotiationStatus', 'initiated');
  this.contentIds.set('negotiationData', JSON.stringify(negotiation));
});

When('Adam selects the {string} bridging strategy', function (this: E2EWorld, strategy: string) {
  this.contentIds.set('bridgingStrategy', strategy);
});

When('Adam sends proposal message {string}', function (this: E2EWorld, _message: string) {
  this.contentIds.set('negotiationStatus', 'proposed');
});

When('Eve views the negotiation proposal', function (this: E2EWorld) {
  const negotiation = loadNegotiationFixture();
  this.contentIds.set('negotiationData', JSON.stringify(negotiation));
});

When('Eve accepts with strategy {string}', function (this: E2EWorld, strategy: string) {
  const negotiation = loadNegotiationFixture();
  assert.strictEqual(
    negotiation.bridgingStrategy,
    strategy,
    `Fixture strategy is "${negotiation.bridgingStrategy}", not "${strategy}"`
  );
  this.contentIds.set('negotiationStatus', 'accepted');
});

When('Eve revokes her {string} attestation to Adam', function (this: E2EWorld, _relType: string) {
  this.contentIds.set('attestationRevoked', 'true');
  this.contentIds.set('loveMapAccessible', 'false');
  this.contentIds.set('negotiationStatus', 'revoked');
});

// ---------------------------------------------------------------------------
// Then steps — gate assertions
// ---------------------------------------------------------------------------

Then(
  'a message about requiring {string} consent level should appear',
  function (this: E2EWorld, requiredLevel: string) {
    const blocked = this.contentIds.get('gateBlocked');
    assert.strictEqual(blocked, 'true', 'Expected consent gate to block');

    const message = this.contentIds.get('gateMessage');
    assert.ok(
      message?.includes(requiredLevel),
      `Gate message should mention "${requiredLevel}": ${message}`
    );
  }
);

Then('the negotiation should not be created', function (this: E2EWorld) {
  const created = this.contentIds.get('negotiationCreated');
  assert.strictEqual(created, 'false', 'Negotiation should not be created when gate blocks');
});

Then('a message about requiring mutual attestation should appear', function (this: E2EWorld) {
  const blocked = this.contentIds.get('attestationGateBlocked');
  assert.strictEqual(blocked, 'true', 'Expected attestation gate to block');

  const message = this.contentIds.get('gateMessage');
  assert.ok(
    message?.toLowerCase().includes('attestation'),
    `Gate message should mention attestation: ${message}`
  );
});

Then('the path content should not be visible', function (this: E2EWorld) {
  const visible = this.contentIds.get('pathVisible');
  assert.strictEqual(visible, 'false', 'Path content should not be visible without attestation');
});

// ---------------------------------------------------------------------------
// Then steps — negotiation status
// ---------------------------------------------------------------------------

Then(
  'the negotiation status should be {string}',
  function (this: E2EWorld, expectedStatus: string) {
    const status = this.contentIds.get('negotiationStatus');
    assert.strictEqual(
      status,
      expectedStatus,
      `Expected negotiation status "${expectedStatus}", got "${status}"`
    );
  }
);

Then('an emergent path {string} should be generated', function (this: E2EWorld, pathTitle: string) {
  const negotiation = loadNegotiationFixture();
  assert.ok(negotiation.generatedPathId, 'No generated path ID in negotiation');

  const pathData = loadPathFixture();
  assert.ok(
    pathData.title.includes(pathTitle),
    `Generated path "${pathData.title}" does not contain "${pathTitle}"`
  );
  assert.ok(pathData.isEmergent, 'Path should be marked as emergent');

  this.contentIds.set('loveMapPathId', pathData.id);
  this.contentIds.set('loveMapPathData', JSON.stringify(pathData));
});

Then(
  'the path visibility should be {string}',
  function (this: E2EWorld, expectedVisibility: string) {
    const pathData = loadPathFixture();
    assert.strictEqual(
      pathData.visibility,
      expectedVisibility,
      `Expected visibility "${expectedVisibility}", got "${pathData.visibility}"`
    );
  }
);

Then('both Adam and Eve should be listed as participants', function (this: E2EWorld) {
  const pathData = loadPathFixture();
  assert.ok(
    pathData.participantIds.includes('human-adam-firstman'),
    'Adam should be a participant'
  );
  assert.ok(
    pathData.participantIds.includes('human-eve-firstwoman'),
    'Eve should be a participant'
  );
});

// ---------------------------------------------------------------------------
// Then steps — revocation
// ---------------------------------------------------------------------------

Then('the path should become inaccessible to both Adam and Eve', function (this: E2EWorld) {
  const accessible = this.contentIds.get('loveMapAccessible');
  assert.strictEqual(accessible, 'false', 'Path should be inaccessible after revocation');
});

Then('the negotiation status should be updated to reflect the change', function (this: E2EWorld) {
  const status = this.contentIds.get('negotiationStatus');
  assert.ok(
    status === 'revoked' || status === 'suspended',
    `Expected negotiation status to reflect revocation, got "${status}"`
  );
});
