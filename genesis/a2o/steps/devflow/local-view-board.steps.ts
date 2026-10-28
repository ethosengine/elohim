import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

let receipt: Record<string, unknown>;
Given('the production Avodah build and the native local-view node', async () => {
  await access(resolve('../../app/elohim-app/dist/elohim-app/browser/index.csr.html'));
});
When(
  'I exercise the private {word} collection through this offline journey:',
  { timeout: 600000 },
  async (collection: string, _journey: string) => {
    const { runLocalViewBrowser } = await import('../../scripts/local-view-browser.mjs');
    const directory = await runLocalViewBrowser(collection);
    receipt = JSON.parse(await readFile(`${directory}/receipt.json`, 'utf8')) as Record<
      string,
      unknown
    >;
  }
);
Then(
  'the board read before forgetting had {int} subjects and the chart retains two observations from exactly two captures',
  (subjects: number) => {
    assert.equal(receipt.subjects, subjects);
    assert.equal(receipt.observations, 2);
    assert.equal(receipt.captures, 2);
  }
);
Then(
  'before forgetting the retained source bytes and observation time are unchanged and the reading is visibly offline',
  () => {
    for (const key of [
      'coldRestart',
      'servingProcessesStopped',
      'networkDisabled',
      'observationTimePreserved',
      'exactSourcePreserved',
    ])
      assert.equal(receipt[key], true, key);
  }
);
Then('interrupted, exhausted, corrupt and failed replacements preserve the previous usable copy', () => {
  for (const key of [
    'interruptedSavePreserved',
    'storageExhaustionPreserved',
    'corruptReplacementPreserved',
    'failedRefreshPreserved',
  ])
    assert.equal(receipt[key], true, key);
});
Then(
  'viewing and reopening leave captured source files unchanged and create no workflow acts',
  () => {
    assert.equal(receipt.sourceFilesUnchanged, true);
    assert.equal(receipt.workflowActsUnchanged, true);
  }
);

Then('forgetting removes the explicitly retained copy', () => {
  assert.equal(receipt.forgotten, true);
});
