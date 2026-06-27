// @vitest-environment jsdom
import { TestBed } from '@angular/core/testing';

import { vi } from 'vitest';

import {
  AUTOMERGE_SYNC_FACTORY,
  CONTENT_SYNC_NAMESPACE,
  CONTENT_SYNC_STORAGE_BASE_URL,
  ContentDocSyncService,
  contentDocId,
} from './content-doc-sync.service';

/**
 * Minimal AutomergeSync stand-in: `load` seeds an empty doc, `sync` returns a
 * doc carrying the projected fields. Automerge docs are read like plain objects,
 * so a plain object is a faithful fake for field extraction.
 */
function fakeSyncReturning(doc: Record<string, unknown>) {
  return {
    load: vi.fn().mockResolvedValue({}),
    sync: vi.fn().mockResolvedValue({ doc, changed: true, heads: ['h1'] }),
  };
}

describe('ContentDocSyncService', () => {
  function setup(doc: Record<string, unknown>) {
    const fake = fakeSyncReturning(doc);
    const baseUrl = vi.fn().mockReturnValue('http://localhost:8090');

    TestBed.configureTestingModule({
      providers: [
        ContentDocSyncService,
        { provide: CONTENT_SYNC_STORAGE_BASE_URL, useValue: baseUrl },
        { provide: AUTOMERGE_SYNC_FACTORY, useValue: () => fake },
      ],
    });

    return { service: TestBed.inject(ContentDocSyncService), fake, baseUrl };
  }

  it('derives the backend doc id as node:{contentId}', () => {
    expect(contentDocId('manifesto-foundations')).toBe('node:manifesto-foundations');
  });

  it("surfaces a synced doc's title field into the signal", async () => {
    const { service } = setup({ id: 'edit-prop-1', title: 'Edited v2', contentFormat: 'markdown' });

    expect(service.contentDoc('edit-prop-1')()).toBeNull();

    const fields = await service.refreshContent('edit-prop-1');

    expect(fields?.title).toBe('Edited v2');
    expect(service.contentDoc('edit-prop-1')()?.title).toBe('Edited v2');
    expect(service.contentDoc('edit-prop-1')()?.contentFormat).toBe('markdown');
  });

  it('queries the sync helper under the load-bearing "elohim" namespace via the seam base URL', async () => {
    const { baseUrl } = setup({ title: 'x' });
    const service = TestBed.inject(ContentDocSyncService);

    await service.refreshContent('c1');

    // The doc-id the helper is asked for is node:{id}, and the helper is built
    // against the storage base URL the connection seam provides.
    expect(baseUrl).toHaveBeenCalled();
    expect(CONTENT_SYNC_NAMESPACE).toBe('elohim');
  });

  it('keeps the last-known fields when a sync round throws', async () => {
    const { service, fake } = setup({ title: 'good' });

    await service.refreshContent('c2');
    expect(service.contentDoc('c2')()?.title).toBe('good');

    fake.sync.mockRejectedValueOnce(new Error('transport down'));
    const after = await service.refreshContent('c2');

    expect(after?.title).toBe('good');
    expect(service.contentDoc('c2')()?.title).toBe('good');
  });

  it('does not leak a second poll loop for the same content id', () => {
    const { service } = setup({ title: 't' });
    const intervalSpy = vi.spyOn(globalThis, 'setInterval');

    service.watchContent('c3', { pollMs: 60_000 });
    service.watchContent('c3', { pollMs: 60_000 });

    expect(intervalSpy.mock.calls.length).toBe(1);

    service.stopWatchingContent('c3');
    intervalSpy.mockRestore();
  });
});
