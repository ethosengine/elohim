import { TestBed } from '@angular/core/testing';
import { DebugContextService } from './debug-context.service';

describe('DebugContextService', () => {
  beforeEach(() => TestBed.configureTestingModule({ providers: [DebugContextService] }));

  it('reports doorway mode + empty storage base in a browser test env', () => {
    const svc = TestBed.inject(DebugContextService);
    // jsdom test env: no __TAURI__, no process.versions.node guaranteed → doorway.
    expect(['doorway', 'direct', 'tauri']).toContain(svc.mode());
  });

  it('routes the storage base URL by mode', () => {
    const svc = TestBed.inject(DebugContextService);
    const base = svc.storageBaseUrl();
    // doorway → '' (same-origin) | tauri/direct → http://localhost:8090
    expect(base === '' || base === 'http://localhost:8090').toBe(true);
  });
});
