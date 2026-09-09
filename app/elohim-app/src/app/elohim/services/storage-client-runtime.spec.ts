import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { DirectConnectionStrategy, DoorwayConnectionStrategy } from '@elohim/service/connection';

import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import { resolveDoorwayUrl } from '../utils/runtime-doorway';

import { DoorwayCacheService } from './doorway-cache.service';
import { StorageApiService } from './storage-api.service';
import { StorageClientService } from './storage-client.service';

vi.mock('../../../environments/environment', () => ({
  environment: {
    client: { doorwayUrl: 'https://doorway.elohim.host' },
    holochain: {
      adminUrl: 'wss://doorway.elohim.host',
      appUrl: 'wss://doorway.elohim.host',
    },
  },
}));

describe('packaged app runtime doorway', () => {
  afterEach(() => {
    TestBed.resetTestingModule();
    vi.unstubAllGlobals();
  });

  it.each([
    'http://localhost:8888',
    'http://127.0.0.1:8890',
    'http://[::1]:8888',
    'https://household.example',
    'https://workspace-angular-dev.devspaces.example.com',
  ])('reads content and blobs at the serving origin %s', origin => {
    vi.stubGlobal('location', { origin });
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: CONNECTION_STRATEGY, useClass: DoorwayConnectionStrategy },
      ],
    });
    const storage = TestBed.inject(StorageClientService);
    const http = TestBed.inject(HttpTestingController);
    storage.getContent('elohim-manifesto').subscribe();
    http.expectOne(`${origin}/db/content/elohim-manifesto`).flush({ id: 'elohim-manifesto' });
    expect(storage.getBlobUrl('sha256-content')).toBe(`${origin}/blob/sha256-content`);
    expect(resolveDoorwayUrl('https://doorway.elohim.host')).toBe(origin);
    TestBed.inject(DoorwayCacheService).isHealthy().subscribe();
    http.expectOne(`${origin}/health`).flush('ok');
    TestBed.inject(DoorwayCacheService).get('Content', 'manifesto').subscribe();
    http.expectOne(`${origin}/api/v1/cache/Content/manifesto`).flush({ id: 'manifesto' });
    TestBed.inject(StorageApiService)
      .getStewardshipAllocations({ contentId: 'manifesto', activeOnly: true })
      .subscribe();
    const allocations = http.expectOne(request => request.url === `${origin}/db/allocations`);
    expect(allocations.request.params.get('contentId')).toBe('manifesto');
    allocations.flush([]);
    http.verify();
  });

  it('retains explicit direct sidecar routing on an HTTP native shell', () => {
    vi.stubGlobal('location', { origin: 'https://tauri.localhost' });
    vi.stubGlobal('__TAURI__', {});
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: CONNECTION_STRATEGY, useClass: DirectConnectionStrategy },
      ],
    });
    const storage = TestBed.inject(StorageClientService);
    const http = TestBed.inject(HttpTestingController);
    storage.getContent('private-content').subscribe();
    http.expectOne('http://localhost:8090/db/content/private-content').flush({});
    expect(resolveDoorwayUrl('https://doorway.elohim.host')).toBe('https://doorway.elohim.host');
    http.verify();
  });

  it('retains configured doorway during SSR without a browser location', () => {
    vi.stubGlobal('location', undefined);
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: CONNECTION_STRATEGY, useClass: DoorwayConnectionStrategy },
      ],
    });
    expect(TestBed.inject(StorageClientService).getStorageBaseUrl()).toBe(
      'https://doorway.elohim.host'
    );
    expect(resolveDoorwayUrl('https://doorway.elohim.host')).toBe('https://doorway.elohim.host');
  });
});
