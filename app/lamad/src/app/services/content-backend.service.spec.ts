import { ApplicationRef, provideZonelessChangeDetection } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { TestBed } from '@angular/core/testing';
import { BLOB_FETCHER, ELOHIM_CLIENT } from '@elohim/service';
import { vi } from 'vitest';

import { LAMAD_STORAGE_CLIENT } from '../interfaces/storage.interface';
import { ContentBackendService } from './content-backend.service';

describe('ContentBackendService SSR stability', () => {
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
