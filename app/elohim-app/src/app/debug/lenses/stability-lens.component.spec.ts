import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { signal } from '@angular/core';
import { DebugContextService } from '../../elohim/services/debug-context.service';
import { StabilityLensComponent } from './stability-lens.component';

describe('StabilityLensComponent', () => {
  function setup(mode: 'doorway' | 'tauri') {
    const ctx = {
      mode: signal(mode),
      isTauri: signal(mode === 'tauri'),
      isDirectStorage: signal(mode !== 'doorway'),
      storageBaseUrl: signal(mode === 'doorway' ? '' : 'http://localhost:8090'),
      environmentName: 'test',
    };
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: DebugContextService, useValue: ctx },
      ],
    });
    const fixture = TestBed.createComponent(StabilityLensComponent);
    const httpMock = TestBed.inject(HttpTestingController);
    return { fixture, httpMock };
  }

  it('doorway: marks autoPreset PENDING and projector REAL', async () => {
    const { fixture, httpMock } = setup('doorway');
    fixture.detectChanges(); // ngOnInit fires the fetch
    httpMock
      .expectOne(r => r.url.endsWith('/admin/self-healing'))
      .flush({
        autoPreset: null,
        admission: null,
        upstreams: [],
        projector: { lagSeconds: 3, caughtUp: true, divergentAnchor: 0 },
        peers: [],
        render: { total: 5, degenerateRate: 0 },
        warmup: { inProgress: false, attempts: 0, completed: true, lastError: null },
        conductor: { connected: true, connectedWorkers: 1, totalWorkers: 1 },
      });
    await fixture.whenStable();
    expect(fixture.componentInstance.blocks().autoPreset.state).toBe('pending');
    expect(fixture.componentInstance.blocks().projector.state).toBe('real');
    expect(fixture.componentInstance.blocks().render.state).toBe('real');
  });

  it('tauri: projector REAL from storage, doorway-role blocks N/A', async () => {
    const { fixture, httpMock } = setup('tauri');
    fixture.detectChanges();
    httpMock
      .expectOne(r => r.url.endsWith('/api/v1/status/projector'))
      .flush({
        cursors: [],
        lag: [{ pillar: 'lamad', kind: 'content', lagSeconds: 7 }],
      });
    httpMock
      .expectOne(r => r.url.endsWith('/p2p/status'))
      .flush({
        projectionReconcile: { caughtUp: false, divergentAnchor: 2 },
      });
    await fixture.whenStable();
    const b = fixture.componentInstance.blocks();
    expect(b.projector.state).toBe('real');
    expect(b.projector.value?.lagSeconds).toBe(7);
    expect(b.projector.value?.caughtUp).toBe(false);
    expect(b.render.state).toBe('na');
    expect(b.peers.state).toBe('na');
  });
});
