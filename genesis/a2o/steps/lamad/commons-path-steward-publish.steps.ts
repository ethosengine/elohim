import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';

import { Given, When, Then } from '@cucumber/cucumber';

import { runClosingPublication } from '../../scripts/lib/steward-closing-publication.js';

import type { ClosingPublicationFixture } from '../../scripts/lib/steward-closing-publication.js';
import type { E2EWorld } from '../../src/framework/world.js';

const runs = new WeakMap<E2EWorld, { fixture: ClosingPublicationFixture; heads?: string[] }>();
Given(
  'the original Che peer has verified credentials for one Matthew-authored commons item',
  function (this: E2EWorld) {
    const path = process.env.E2E_COMMONS_PUBLICATION_FIXTURE;
    assert.ok(
      path,
      'explicit verified one-root publication fixture required; no roots or grants are created by this step'
    );
    const fixture = JSON.parse(readFileSync(path, 'utf8')) as ClosingPublicationFixture;
    assert.ok(fixture.root && fixture.che && fixture.matthew && fixture.humanId);
    assert.notEqual(fixture.che, fixture.matthew);
    runs.set(this, { fixture });
  }
);
When(
  'Che publishes an update with exact controller witnesses, waits for election and doorway delivery, then publishes a second update',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const run = runs.get(this);
    assert.ok(run, 'verified publication prerequisite required');
    run.heads = await runClosingPublication(run.fixture);
  }
);
Then(
  'Adam elects each Che update and alpha and apex serve its exact head and body within 75 seconds of that update starting',
  function (this: E2EWorld) {
    const heads = runs.get(this)?.heads;
    assert.equal(heads?.length, 2);
    assert.notEqual(heads?.[0], heads?.[1]);
  }
);
