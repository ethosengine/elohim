import { strict as assert } from 'node:assert';
import { test } from 'node:test';

import { E2EWorld } from '../../world.js';

function world(): E2EWorld {
  return new E2EWorld({
    attach: async () => await Promise.resolve(),
    log: () => undefined,
    link: () => undefined,
    parameters: {},
  });
}

void test('ordinary cleanup remains best effort and all callbacks run in LIFO order', async () => {
  const subject = world();
  const calls: string[] = [];
  subject.onCleanup(async () => {
    await Promise.resolve();
    calls.push('first');
  });
  subject.onCleanup(async () => {
    await Promise.resolve();
    calls.push('second');
    throw new Error('best effort');
  });
  await subject.runCleanup();
  assert.deepEqual(calls, ['second', 'first']);
});

void test('required cleanup fails after every callback was attempted', async () => {
  const subject = world();
  const calls: string[] = [];
  subject.onCleanup(async () => {
    await Promise.resolve();
    calls.push('last');
  });
  subject.onCleanup(
    async () => {
      await Promise.resolve();
      calls.push('required');
      throw new Error('owned process survived');
    },
    { required: true }
  );
  await assert.rejects(subject.runCleanup(), /required scenario cleanup failed/);
  assert.deepEqual(calls, ['required', 'last']);
});
