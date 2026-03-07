/**
 * Compute coordination step definitions — elohim node budget, deferral, and
 * REA economic events for inference compute.
 *
 * These steps are aspirational — they return 'pending' until the underlying
 * infrastructure (admission controller, REA event store) is wired end-to-end.
 * They document the target contract between the learner experience and the
 * compute coordination layer.
 */

import { Given, When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Budget state (Given)
// ---------------------------------------------------------------------------

Given('the elohim node has inference budget remaining', async function (this: E2EWorld) {
  // Training wheels: assume budget is available in dev mode.
  // When the admission controller is wired, this step will verify the
  // /elohim/capacity endpoint reports remaining budget > 0.
  return 'pending';
});

Given('the elohim node has exhausted its inference budget', async function (this: E2EWorld) {
  // Will call the admission controller's test endpoint to artificially
  // exhaust the inference budget for this scenario.
  return 'pending';
});

// ---------------------------------------------------------------------------
// Insight receipt (When)
// ---------------------------------------------------------------------------

When('the learner receives an elohim insight', async function (this: E2EWorld) {
  // Composite step: navigate to an assessment, complete it, and wait for
  // the elohim insight section to appear. Pending until the full flow is
  // exercisable end-to-end.
  return 'pending';
});

// ---------------------------------------------------------------------------
// Deferral assertions (Then)
// ---------------------------------------------------------------------------

Then('the insight shows a capacity unavailable message', async function (this: E2EWorld) {
  // When budget is exhausted, the UI should show a graceful deferral message
  // instead of an error. Pending until the deferred response handling is
  // wired in NativeBackend.
  return 'pending';
});

// ---------------------------------------------------------------------------
// Economic event assertions (Then)
// ---------------------------------------------------------------------------

Then('a compute economic event is recorded', async function (this: E2EWorld) {
  // Verify that a ComputeEvent (REA) was persisted after the insight was
  // delivered. Will query the doorway's /admin/compute-events endpoint or
  // the elohim-node's event log.
  return 'pending';
});

Then('the event captures the tokens used and model', async function (this: E2EWorld) {
  // Verify the ComputeEvent includes tokenCount and modelId fields that
  // match what the insight panel displays.
  return 'pending';
});
