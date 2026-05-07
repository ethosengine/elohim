/**
 * Spatial Map UI step definitions — Playwright assertions for the /map surface.
 *
 * Iter-1 of /deliver geospatial-cybersyn-stack. Minimum-proof shape:
 * verify the SpatialMapComponent mounts and the MapLibre WebGL canvas paints.
 */

import { strict as assert } from 'node:assert';

import { Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { E2EWorld } from '../../src/framework/world.js';

function getPlaywrightDevice(world: E2EWorld): PlaywrightDevice {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  throw new Error('No Playwright device registered. Background must log a human in.');
}

Then('the spatial map component should be mounted', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const host = device.page.locator('app-spatial-map');
  await host.waitFor({ state: 'attached', timeout: 10_000 });
  const wrapper = device.page.locator('app-spatial-map .map-wrapper');
  await wrapper.waitFor({ state: 'visible', timeout: 10_000 });
});

Then('the MapLibre canvas should be rendered', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  // .maplibregl-canvas is only assigned after WebGL context init succeeds.
  // If WebGL fails, MapLibre creates the canvas DOM element but doesn't
  // tag it with this class. So waitFor(visible) on this selector is the
  // WebGL-success proof — it will time out if the GL context didn't init.
  const canvas = device.page.locator('.maplibregl-canvas');
  await canvas.waitFor({ state: 'visible', timeout: 15_000 });
  assert.ok(await canvas.isVisible(), 'MapLibre canvas not visible after waitFor');
});
