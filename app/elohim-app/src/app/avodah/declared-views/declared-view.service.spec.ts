import { TestBed } from '@angular/core/testing';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import { describe, expect, it, vi } from 'vitest';

import { DeclaredViewService, PROJECTION_FEED } from './declared-view.service';
import { OfflineViewRepository } from './offline-view.repository';

const address = async (text: string) =>
  CID.createV1(
    0x55,
    await sha256.digest(new Uint8Array(new TextEncoder().encode(text)))
  ).toString();
describe('replaceable projection feed', () => {
  it('captures once on open and manual refresh, reads history after failed capture, and never captures on selection', async () => {
    const source = JSON.stringify({
      presentation: { titleField: '$identity', statusField: '$identity' },
    });
    const sourceCid = await address(source);
    const projection = {
      version: 1,
      purpose: 'replacement feed',
      declaration: 'view.json',
      declaration_cid: sourceCid,
      method: 'test',
      reader_versions: {},
      subjects: [],
      sources: { 'view.json': sourceCid },
      input_versions: { 'view.json': sourceCid },
      governance: {},
      diagnostics: [],
    };
    const envelope = JSON.stringify({ version: 1, projection });
    const snapshotCid = await address(envelope);
    const bundle = { snapshotCid, envelope, objects: { [sourceCid]: btoa(source) } };
    const observation = {
      sequence: 1,
      observedAt: '2026-10-13T10:00:00Z',
      snapshotCid,
      declarationCid: sourceCid,
    };
    const feed = {
      open: vi.fn(),
      capture: vi.fn(),
      history: vi.fn().mockResolvedValue([observation]),
      snapshot: vi.fn().mockResolvedValue(bundle),
    };
    TestBed.configureTestingModule({
      providers: [
        DeclaredViewService,
        { provide: PROJECTION_FEED, useValue: feed },
        {
          provide: OfflineViewRepository,
          useValue: { load: vi.fn().mockResolvedValue(undefined) },
        },
      ],
    });
    const view = TestBed.inject(DeclaredViewService);
    await view.open('test');
    await view.open('test');
    expect(feed.capture).toHaveBeenCalledTimes(1);
    expect(view.readings()[0].projection.purpose).toBe('replacement feed');
    view.selected.set(0);
    expect(feed.capture).toHaveBeenCalledTimes(1);
    feed.capture.mockRejectedValueOnce(new Error('malformed current declaration'));
    await view.refresh();
    expect(feed.capture).toHaveBeenCalledTimes(2);
    expect(feed.history).toHaveBeenCalledTimes(2);
    expect(view.readings()).toHaveLength(1);
    expect(view.error()).toContain('malformed');
  });
});

describe('offline retention ordering', () => {
  it('Forget during an in-flight save leaves no retained copy once the save finishes', async () => {
    const source = JSON.stringify({
      presentation: { titleField: '$identity', statusField: '$identity' },
    });
    const sourceCid = await address(source);
    const projection = {
      version: 1,
      purpose: 'retention',
      declaration: 'view.json',
      declaration_cid: sourceCid,
      method: 'test',
      reader_versions: {},
      subjects: [],
      sources: { 'view.json': sourceCid },
      input_versions: { 'view.json': sourceCid },
      governance: {},
      diagnostics: [],
    };
    const envelope = JSON.stringify({ version: 1, projection });
    const snapshotCid = await address(envelope);
    const bundle = { snapshotCid, envelope, objects: { [sourceCid]: btoa(source) } };
    const observation = {
      sequence: 1,
      observedAt: '2026-10-13T10:00:00Z',
      snapshotCid,
      declarationCid: sourceCid,
    };
    const feed = {
      open: vi.fn(),
      capture: vi.fn(),
      history: vi.fn().mockResolvedValue([observation]),
      snapshot: vi.fn().mockResolvedValue(bundle),
    };
    let store: unknown;
    let finishSave: () => void = () => undefined;
    const saveGate = new Promise<void>(resolve => {
      finishSave = resolve;
    });
    const repository = {
      load: vi.fn().mockResolvedValue(undefined),
      pinBuild: vi.fn().mockResolvedValue('cache-1'),
      save: vi.fn(async (copy: unknown) => {
        await saveGate;
        store = copy;
      }),
      forget: vi.fn(async () => {
        store = undefined;
      }),
    };
    TestBed.configureTestingModule({
      providers: [
        DeclaredViewService,
        { provide: PROJECTION_FEED, useValue: feed },
        { provide: OfflineViewRepository, useValue: repository },
      ],
    });
    const view = TestBed.inject(DeclaredViewService);
    await view.open('test');
    const keeping = view.keepOffline();
    await vi.waitFor(() => expect(repository.save).toHaveBeenCalled());
    const forgetting = view.forget();
    finishSave();
    await Promise.all([keeping, forgetting]);
    expect(store).toBeUndefined();
    expect(view.retained()).toBe(false);
  });
});
