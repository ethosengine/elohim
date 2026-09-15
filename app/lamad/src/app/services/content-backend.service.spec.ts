import { ApplicationRef, provideZonelessChangeDetection } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { BLOB_FETCHER, ELOHIM_CLIENT, ELOHIM_ENV } from '@elohim/service';
import { firstValueFrom } from 'rxjs';
import { vi } from 'vitest';

import { LAMAD_STORAGE_CLIENT } from '../interfaces/storage.interface';
import { ContentBackendService } from './content-backend.service';
import { ProjectionAPIService } from './projection-api.service';

describe('ContentBackendService SSR stability', () => {
  it('hydrates a cached projection whose content body is a blob address', async () => {
    const manifesto = '# Manifesto\n\nExecutive Summary\n\nLove as Technology';
    const fetchVerified = vi.fn().mockResolvedValue(new TextEncoder().encode(manifesto));
    TestBed.configureTestingModule({
      providers: [
        provideZonelessChangeDetection(),
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ELOHIM_CLIENT, useValue: {} },
        { provide: ELOHIM_ENV, useValue: { holochain: { authUrl: 'http://doorway.test' } } },
        { provide: BLOB_FETCHER, useValue: { fetchVerified } },
        { provide: LAMAD_STORAGE_CLIENT, useValue: {} },
      ],
    });

    const resultPromise = firstValueFrom(
      TestBed.inject(ProjectionAPIService).getContentNode('manifesto')
    );
    TestBed.inject(HttpTestingController)
      .expectOne('http://doorway.test/api/v1/cache/Content/manifesto')
      .flush({
        id: 'manifesto',
        contentType: 'epic',
        contentFormat: 'markdown',
        contentBody: 'sha256-deadbeef',
        blobCid: 'bafk-unused-when-content-body-addressed',
      });
    const result = await resultPromise;

    expect(fetchVerified).toHaveBeenCalledWith('sha256-deadbeef');
    expect(result?.content).toBe(manifesto);
  });

  it.each(['resolve', 'reject', 'unsubscribe'])(
    'holds Angular stability until query %s',
    async outcome => {
      let resolve!: (value: unknown[]) => void;
      let reject!: (reason: Error) => void;
      const pending = new Promise<unknown[]>((yes, no) => {
        resolve = yes;
        reject = no;
      });
      TestBed.configureTestingModule({
        providers: [
          provideZonelessChangeDetection(),
          provideHttpClient(),
          {
            provide: ELOHIM_CLIENT,
            useValue: { query: vi.fn().mockReturnValue(pending) },
          },
          { provide: BLOB_FETCHER, useValue: {} },
          { provide: LAMAD_STORAGE_CLIENT, useValue: {} },
        ],
      });
      const app = TestBed.inject(ApplicationRef);
      const service = TestBed.inject(ContentBackendService);
      let stable = false;
      const observation = app.isStable.subscribe(value => {
        stable = value;
      });
      await app.whenStable();
      const subscription = service.queryContent({ limit: 1 }).subscribe();
      expect(stable).toBe(false);
      if (outcome === 'reject') reject(new Error('fixture unavailable'));
      else if (outcome === 'resolve') resolve([]);
      else {
        subscription.unsubscribe();
        resolve([]);
      }
      await app.whenStable();
      expect(stable).toBe(true);
      subscription.unsubscribe();
      observation.unsubscribe();
    }
  );
});
