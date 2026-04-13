/**
 * Device archetype step definitions — load hardware profiles from devices.json
 * and verify that protocol operations respect device constraints.
 *
 * These steps are pure data assertions against the device fixture catalogue.
 * They do not spin up real devices — they verify that the protocol's documented
 * constraints match the archetype definitions.
 */

import { Given, Then, DataTable } from '@cucumber/cucumber';
import { expect } from 'chai';

import { DeviceArchetype, getDevice, getAllDevices, getDevicesByLevel } from '../src/framework/fixtures/devices.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// World augmentation — store currentDevice on the world instance directly.
// Cucumber worlds are plain class instances; TypeScript augmentation via
// declaration merging works at compile time but the runtime state lives on
// `this` in the step function (typed as E2EWorld & { currentDevice? }).
// ---------------------------------------------------------------------------

type WorldWithDevice = E2EWorld & { currentDevice?: DeviceArchetype; allDevices?: DeviceArchetype[] };

// ---------------------------------------------------------------------------
// Given — load device(s)
// ---------------------------------------------------------------------------

/**
 * Load a named device archetype from the portfolio and store it on the world.
 *
 * Example:
 *   Given device "2019 Android Phone" from the device portfolio
 */
Given(
  'device {string} from the device portfolio',
  function (this: WorldWithDevice, deviceName: string) {
    this.currentDevice = getDevice(deviceName);
  }
);

/**
 * Load all devices into the world for portfolio-level assertions.
 *
 * Example:
 *   Given the device portfolio is loaded
 */
Given('the device portfolio is loaded', function (this: WorldWithDevice) {
  this.allDevices = getAllDevices();
});

// ---------------------------------------------------------------------------
// Then — capability assertions
// ---------------------------------------------------------------------------

Then(
  'the device memory should be {float} GB',
  function (this: WorldWithDevice, expectedGb: number) {
    const device = requireDevice(this);
    expect(device.memoryGb).to.equal(expectedGb);
  }
);

Then(
  'the device capability level should be {int}',
  function (this: WorldWithDevice, expectedLevel: number) {
    const device = requireDevice(this);
    expect(device.capabilityLevel).to.equal(expectedLevel);
  }
);

Then('the device can steward content', function (this: WorldWithDevice) {
  const device = requireDevice(this);
  expect(device.canSteward, `Expected ${device.displayName} to have canSteward=true`).to.be.true;
});

Then('the device cannot steward content', function (this: WorldWithDevice) {
  const device = requireDevice(this);
  expect(device.canSteward, `Expected ${device.displayName} to have canSteward=false`).to.be.false;
});

Then('the device should be always-on', function (this: WorldWithDevice) {
  const device = requireDevice(this);
  expect(device.alwaysOn, `Expected ${device.displayName} to be always-on`).to.be.true;
});

Then('the device should not be always-on', function (this: WorldWithDevice) {
  const device = requireDevice(this);
  expect(device.alwaysOn, `Expected ${device.displayName} to not be always-on`).to.be.false;
});

Then(
  'the device NAT type should be {string}',
  function (this: WorldWithDevice, expectedNat: string) {
    const device = requireDevice(this);
    expect(device.natType).to.equal(expectedNat);
  }
);

Then(
  'the device degradation mode should be {string}',
  function (this: WorldWithDevice, expectedMode: string) {
    const device = requireDevice(this);
    expect(device.degradationMode).to.equal(expectedMode);
  }
);

Then(
  'the device serviceability should be {string}',
  function (this: WorldWithDevice, expectedServiceability: string) {
    const device = requireDevice(this);
    expect(device.serviceability).to.equal(expectedServiceability);
  }
);

// ---------------------------------------------------------------------------
// Then — health surfaces (DataTable)
// ---------------------------------------------------------------------------

/**
 * Verify the device's health surfaces include all rows in the DataTable.
 *
 * Example:
 *   Then the device should report health surfaces:
 *     | surface    |
 *     | smart      |
 *     | thermal    |
 */
Then(
  'the device should report health surfaces:',
  function (this: WorldWithDevice, table: DataTable) {
    const device = requireDevice(this);
    const expectedSurfaces: string[] = table.rows().map((row: string[]) => row[0]);
    for (const surface of expectedSurfaces) {
      expect(
        device.healthSurfaces,
        `Expected ${device.displayName} to report health surface "${surface}"`
      ).to.include(surface);
    }
  }
);

// ---------------------------------------------------------------------------
// Then — attestation capabilities (DataTable)
// ---------------------------------------------------------------------------

/**
 * Verify the device's attestation capabilities include all rows in the DataTable.
 *
 * Example:
 *   Then the device should support attestation:
 *     | capability           |
 *     | hardware-key-signing |
 */
Then(
  'the device should support attestation:',
  function (this: WorldWithDevice, table: DataTable) {
    const device = requireDevice(this);
    const expectedCaps: string[] = table.rows().map((row: string[]) => row[0]);
    for (const cap of expectedCaps) {
      expect(
        device.attestationCapabilities,
        `Expected ${device.displayName} to support attestation capability "${cap}"`
      ).to.include(cap);
    }
  }
);

// ---------------------------------------------------------------------------
// Then — portfolio coverage
// ---------------------------------------------------------------------------

/**
 * Assert the portfolio has at least N devices at a given capability level.
 *
 * Example:
 *   Then there should be at least 1 device at capability level 0
 */
Then(
  'there should be at least {int} device at capability level {int}',
  function (this: WorldWithDevice, minCount: number, level: number) {
    const devices = getDevicesByLevel(level);
    expect(
      devices.length,
      `Expected at least ${minCount} device(s) at capability level ${level}, found ${devices.length}`
    ).to.be.at.least(minCount);
  }
);

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function requireDevice(world: WorldWithDevice): DeviceArchetype {
  if (!world.currentDevice) {
    throw new Error(
      'No current device set. Use a "Given device {string} from the device portfolio" step first.'
    );
  }
  return world.currentDevice;
}
