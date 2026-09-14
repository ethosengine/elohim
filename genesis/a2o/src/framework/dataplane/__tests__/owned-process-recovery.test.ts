/* eslint-disable @typescript-eslint/require-await -- deterministic async fakes */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { awaitOwnedProcessRecovery } from '../owned-process-recovery.js';

void describe('awaitOwnedProcessRecovery', () => {
  void it('keeps checking an already-resumed process until its faulted path recovers', async () => {
    let now = 0;
    let pathChecks = 0;
    await awaitOwnedProcessRecovery(
      {
        identityMatches: async () => true,
        processRunning: async () => true,
        faultedPathHealthy: async () => {
          pathChecks += 1;
          return pathChecks === 3;
        },
      },
      10,
      1,
      {
        now: () => now,
        sleep: async milliseconds => {
          now += milliseconds;
        },
      }
    );
    assert.equal(pathChecks, 3);
  });

  void it('checks the immutable faulted origin after a visitor moves to the sibling', async () => {
    const state = {
      faultedOrigin: 'http://faulted.example',
      visitorOrigin: 'http://faulted.example',
    };
    state.visitorOrigin = 'http://sibling.example';
    let checkedOrigin = '';
    await awaitOwnedProcessRecovery(
      {
        identityMatches: async () => true,
        processRunning: async () => true,
        faultedPathHealthy: async () => {
          checkedOrigin = state.faultedOrigin;
          return true;
        },
      },
      10,
      1
    );
    assert.equal(state.visitorOrigin, 'http://sibling.example');
    assert.equal(checkedOrigin, state.faultedOrigin);
  });
});
