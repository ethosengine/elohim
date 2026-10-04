import * as Automerge from '@automerge/automerge';
import { describe, expect, it } from 'vitest';

import { AutomergeSync } from '../src/sync.js';

import type { StorageClient } from '../src/client.js';

function toBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('base64');
}

/**
 * A stand-in for storage that answers in the wire shape the /sync/v1 handlers send:
 * camelCase, with the heads under `newHeads`.
 */
function fakeStorage(serverDoc: Automerge.Doc<{ title?: string }>) {
  let doc = serverDoc;
  const applied: Uint8Array[][] = [];
  const client = {
    decodeBase64(b64: string): Uint8Array {
      return new Uint8Array(Buffer.from(b64, 'base64'));
    },
    async getChangesSince(_docId: string, have: string[]) {
      const changes =
        have.length === 0
          ? Automerge.getAllChanges(doc)
          : Automerge.getChanges(Automerge.view(doc, have), doc);
      return {
        hAppId: 'elohim',
        docId: 'node:a',
        changes: changes.map(toBase64),
        newHeads: Automerge.getHeads(doc),
      };
    },
    async applyChanges(_docId: string, changes: Uint8Array[]) {
      applied.push(changes);
      [doc] = Automerge.applyChanges(doc, changes);
      return { hAppId: 'elohim', docId: 'node:a', newHeads: Automerge.getHeads(doc) };
    },
  } as unknown as StorageClient;
  return { client, applied };
}

describe('AutomergeSync against the server wire shape', () => {
  it('remembers the heads the server reports, so a sync with nothing new sends nothing', async () => {
    const server = Automerge.change(Automerge.init<{ title?: string }>(), d => {
      d.title = 'published';
    });
    const { client, applied } = fakeStorage(server);
    const sync = new AutomergeSync(client);

    const loaded = await sync.load<{ title?: string }>('node:a');
    expect(loaded.title).toBe('published');

    await sync.sync('node:a', loaded);

    const bytesSent = applied.flat().reduce((n, c) => n + c.length, 0);
    expect(bytesSent).toBe(0);
  });

  it('sends only the local change after a load, not the whole document', async () => {
    const server = Automerge.change(Automerge.init<{ title?: string }>(), d => {
      d.title = 'published';
    });
    const { client, applied } = fakeStorage(server);
    const sync = new AutomergeSync(client);

    const loaded = await sync.load<{ title?: string }>('node:a');
    const edited = Automerge.change(loaded, d => {
      d.title = 'edited';
    });
    const wholeDoc = Automerge.save(edited).length;

    const heads = await sync.save('node:a', edited);

    expect(heads).toEqual(Automerge.getHeads(edited));
    const bytesSent = applied.flat().reduce((n, c) => n + c.length, 0);
    expect(bytesSent).toBeGreaterThan(0);
    expect(bytesSent).toBeLessThan(wholeDoc);
  });
});
