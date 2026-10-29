import assert from 'node:assert/strict';

import { Given, Then, When } from '@cucumber/cucumber';

import {
  availableOffline,
  UnsupportedLifecycle,
  UnsupportedCapabilityError,
  type AppResidencyView,
} from '@elohim/storage-client';

let adapter: UnsupportedLifecycle;
let failure: unknown;
let residency: AppResidencyView;
let offline: boolean | undefined;
Given('a portable app adapter with no runtime implementation', () => {
  adapter = new UnsupportedLifecycle();
  failure = undefined;
});
When('a client asks it to open a release', async () => {
  try {
    await adapter.ensureInstance({
      release: {
        appId: 'orchard',
        releaseCid: 'fixture-release',
        target: 'browser',
        artifactCids: ['fixture-artifact'],
        requiredCapabilities: [],
      },
      operationId: 'open-1',
      grantRef: 'not-authority',
    });
  } catch (error) {
    failure = error;
  }
});
Then('the lifecycle operation explicitly reports unsupported', () => {
  assert.ok(failure instanceof UnsupportedCapabilityError);
});
Given('a release whose artifacts are verified but whose data is not', () => {
  residency = {
    releaseCid: 'fixture-release',
    pinned: true,
    running: true,
    artifactsVerified: true,
    dataVerified: null,
  };
  offline = undefined;
});
When('a client asks whether the release is available offline', () => {
  offline = availableOffline(residency);
});
Then('the release is not available offline', () => {
  assert.equal(offline, false);
});
