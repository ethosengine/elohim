/**
 * Steps for features/dataplane/hub-carries-edit.feature — the runnable seam of the
 * local-first shared-sheet vision (features/federation/local-first-shared-sheet.feature):
 * an edit made through a doorway while its author's own peer is shut reaches the always-on
 * hub promptly, and reaches the author's peer when it comes back, with nothing written to it.
 */
import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

import {
  pollForGaugeCapturingError,
  probeSyncDocHeads,
  resolveStorageUrl,
} from '../../src/framework/dataplane/surfaces.js';
import { destructiveAllowed } from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';
import { meshControl } from './epr-app-deliverability.helpers.js';

const shutPeers = new WeakMap<E2EWorld, Set<string>>();

function storageUrl(peer: string): string {
  const url = resolveStorageUrl(peer);
  assert.ok(url, `E2E_STORAGE_${peer.toUpperCase()} is not set`);
  return url;
}

async function refusesConnections(peer: string): Promise<boolean> {
  return fetch(`${storageUrl(peer)}/health`, { signal: AbortSignal.timeout(2_000) }).then(
    () => false,
    () => true
  );
}

Given(
  'household peer {string} is stopped',
  { timeout: 180_000 },
  async function (this: E2EWorld, peer: string) {
    assert.ok(
      destructiveAllowed(),
      'NEEDS_OWNED_SUBSTRATE: A2O_ALLOW_DESTRUCTIVE=1 is required before storage-stop'
    );
    const shut = shutPeers.get(this) ?? new Set<string>();
    shutPeers.set(this, shut);
    shut.add(peer);
    this.onCleanup(
      async () => {
        if (shut.has(peer)) await meshControl('storage-restart', peer);
      },
      { required: true }
    );
    await meshControl('storage-stop', peer);
    assert.ok(await refusesConnections(peer), `${peer} still answers /health after storage-stop`);
  }
);

When(
  'household peer {string} comes back',
  { timeout: 300_000 },
  async function (this: E2EWorld, peer: string) {
    await meshControl('storage-restart', peer);
    shutPeers.get(this)?.delete(peer);
    assert.equal(await refusesConnections(peer), false, `${peer} did not come back`);
  }
);

Then(
  "household peer {string} serves the new document at peer {string}'s exact Automerge heads within {int} seconds",
  { timeout: 300_000 },
  async function (this: E2EWorld, peer: string, author: string, seconds: number) {
    const contentId = this.contentIds.get('lastContentId');
    assert.ok(contentId, 'the new content id was not captured');
    const docId = `node:${contentId}`;

    let authorHeads: string[] = [];
    const authored = await pollForGaugeCapturingError(
      async () => {
        authorHeads = (await probeSyncDocHeads(storageUrl(author), 'elohim', docId)).body.heads;
        return authorHeads.length > 0 ? true : undefined;
      },
      { intervalMs: 500, timeoutMs: 15_000 }
    );
    assert.equal(authored.value, true, `${docId} did not project on ${author} within 15 seconds`);

    // The window opens once the author holds the change, so it measures only the hop to `peer`.
    const started = Date.now();
    let peerHeads: string[] = [];
    const converged = await pollForGaugeCapturingError(
      async () => {
        peerHeads = (await probeSyncDocHeads(storageUrl(peer), 'elohim', docId)).body.heads;
        const same =
          peerHeads.length === authorHeads.length &&
          [...peerHeads].sort().every((head, i) => head === [...authorHeads].sort()[i]);
        return same ? true : undefined;
      },
      { intervalMs: 250, timeoutMs: seconds * 1_000 }
    );
    this.attach(
      JSON.stringify(
        { measure: 'hub-carries-edit', peer, author, docId, elapsedMs: Date.now() - started },
        null,
        2
      ),
      'application/json'
    );
    assert.equal(
      converged.value,
      true,
      `${peer} did not reach ${author}'s heads [${authorHeads.join(', ')}] within ${seconds}s; ` +
        `it holds [${peerHeads.join(', ')}]` +
        (converged.lastError ? ` (last error: ${converged.lastError})` : '')
    );
  }
);
