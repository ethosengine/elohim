// Build-time dependency/step-discovery proof only: dryRun never executes hooks or steps.
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { loadConfiguration, runCucumber } from '@cucumber/cucumber/api';
import profiles from '../cucumber.mjs';

const cwd = fileURLToPath(new URL('../', import.meta.url));
process.chdir(cwd);
const feature = 'features/dataplane/peer-mesh.feature';
const { runConfiguration } = await loadConfiguration(
  {
    file: false,
    provided: {
      ...profiles().mesh,
      paths: [feature],
      format: ['summary'],
      dryRun: true,
      strict: true,
      publish: false,
    },
  },
  { cwd }
);
const scenarios = new Set();
const result = await runCucumber(runConfiguration, { cwd }, message => {
  if (message.pickle) {
    assert.equal(message.pickle.uri, feature);
    scenarios.add(message.pickle.id);
  }
});
assert.equal(scenarios.size, 3, 'the initial peer feature must discover exactly three scenarios');
assert.equal(result.success, true, 'peer stage step/dependency discovery failed');
