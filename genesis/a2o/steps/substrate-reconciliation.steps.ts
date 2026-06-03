/**
 * Substrate-reconciliation step definitions.
 *
 * Guards the contract that the CI/test bench reconciles its scope to the
 * available compute as the hardware substrate flexes (a remote peer flapping
 * on and off). These steps are pure-logic: they read the substrate signal this
 * run was given (ELOHIM_REMOTE_COMPUTE_STATUS, via the fixture layer) and assert the
 * invariant that holds whether the remote pool is up or down — the household is
 * always reachable, and only remote-only people are ever scaled out. No global
 * state is mutated, so the steps are safe inside a shared cucumber run.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  isHumanDeployed,
  isRemoteComputeAvailable,
  autoSkippedHumans,
  isRemoteOnlyPersona,
} from '../src/framework/fixtures/humans.js';

/** On-prem household personas — always have an off-shem fallback pool. */
const HOUSEHOLD = ['Matthew', 'Jessica', 'James'];

Given('the substrate signal has been read for this run', function () {
  // The signal is process.env.ELOHIM_REMOTE_COMPUTE_STATUS, read lazily by the fixture
  // layer. Nothing to set up — this step makes the run's interpretation
  // explicit in the log so a reduced-scope run reads as deliberate.
  // eslint-disable-next-line no-console
  console.log(
    `  substrate: remote pool ${isRemoteComputeAvailable() ? 'available' : 'unavailable'} ` +
      `(ELOHIM_REMOTE_COMPUTE_STATUS=${process.env['ELOHIM_REMOTE_COMPUTE_STATUS'] ?? 'unknown'})`
  );
});

Then('household person {string} is reachable for testing', function (name: string) {
  assert.equal(
    isHumanDeployed(name),
    true,
    `${name} is a household person and must remain reachable regardless of the remote ` +
      `pool, but isHumanDeployed() returned false. Check ${name}'s nodeTypes in ` +
      `deployments.json — it must include an on-prem pool (operations/edge/performance), ` +
      `not be remote-only.`
  );
});

Then(
  'remote-only person {string} is reachable only when the remote peer is available',
  function (name: string) {
    assert.equal(
      isRemoteOnlyPersona(name),
      true,
      `${name} is expected to be a remote-only persona (nodeTypes === ['remote']) in ` +
        `deployments.json, but is not. Update the scenario or the topology.`
    );
    assert.equal(
      isHumanDeployed(name),
      isRemoteComputeAvailable(),
      `${name} is remote-only, so reachability must track the remote pool: ` +
        `isHumanDeployed()=${isHumanDeployed(name)} while isRemoteComputeAvailable()=${isRemoteComputeAvailable()}.`
    );
  }
);

When('the household and a remote-only person are checked for reachability', function () {
  // Touch each so this run's auto-skip ledger reflects a representative probe
  // (the ledger also accumulates from other scenarios — the invariant below
  // holds either way).
  for (const name of [...HOUSEHOLD, 'Gertrude']) {
    isHumanDeployed(name);
  }
});

Then('no household person has been scaled out this run', function () {
  const skipped = autoSkippedHumans(); // lowercased names
  for (const name of HOUSEHOLD) {
    assert.equal(
      skipped.includes(name.toLowerCase()),
      false,
      `${name} (household) was scaled out this run — the run must only ever scale out ` +
        `remote-only people. Scaled-out set: [${skipped.join(', ')}].`
    );
  }
});

Then('every person scaled out this run is a remote-only person', function () {
  const skipped = autoSkippedHumans();
  for (const name of skipped) {
    assert.equal(
      isRemoteOnlyPersona(name),
      true,
      `"${name}" was scaled out this run but is not a remote-only persona — only personas ` +
        `whose nodeTypes are all "remote" may be auto-skipped. Check deployments.json.`
    );
  }
});
