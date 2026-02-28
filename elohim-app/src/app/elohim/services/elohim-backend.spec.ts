import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import { ElohimBackendCatalog, MockBackend } from './elohim-backend';

import type {
  ElohimAgent,
  ElohimRequest,
  ElohimResponse,
} from '../models/elohim-agent.model';
import type { ElohimBackend } from './elohim-backend';
import { type Mock, vi } from 'vitest';

describe('MockBackend', () => {
  let backend: MockBackend;

  const mockElohim: ElohimAgent = {
    id: 'elohim-1',
    displayName: 'Wisdom Guardian',
    layer: 'family',
    bio: 'Protects content integrity',
    attestations: [],
    capabilities: ['content-safety-review', 'attestation-recommendation'],
    visibility: 'public',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  const buildRequest = (
    overrides: Partial<ElohimRequest> = {},
  ): ElohimRequest => ({
    requestId: 'req-test-1',
    targetElohimId: 'elohim-1',
    capability: 'content-safety-review',
    params: {
      type: 'content-review',
      contentId: 'content-123',
      reviewType: 'safety',
    },
    requesterId: 'user-123',
    priority: 'normal',
    requestedAt: new Date().toISOString(),
    ...overrides,
  });

  beforeEach(() => {
    backend = new MockBackend();
  });

  it('should have correct id, type, and name', () => {
    expect(backend.id).toBe('mock');
    expect(backend.type).toBe('mock');
    expect(backend.name).toBe('Simulation (Mock)');
  });

  it('should always be available', async () => {
    const available = await backend.isAvailable();
    expect(available).toBe(true);
  });

  it('should return response after processing delay', fakeAsync(() => {
    const request = buildRequest();
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });

    expect(response).toBeUndefined();
    tick(2000);
    expect(response).toBeDefined();
    expect(response!.requestId).toBe('req-test-1');
    expect(response!.elohimId).toBe('elohim-1');
  }));

  it('should return fulfilled status with constitutional reasoning', fakeAsync(() => {
    const request = buildRequest();
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.status).toBe('fulfilled');
    expect(response!.constitutionalReasoning).toBeDefined();
    expect(response!.constitutionalReasoning.primaryPrinciple).toBeTruthy();
    expect(response!.constitutionalReasoning.confidence).toBeGreaterThan(0);
  }));

  it('should include computation cost', fakeAsync(() => {
    const request = buildRequest();
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.cost).toBeDefined();
    expect(response!.cost!.tokensProcessed).toBeGreaterThan(0);
    expect(response!.cost!.timeMs).toBeGreaterThan(0);
    expect(response!.cost!.constitutionalChecks).toBeGreaterThan(0);
  }));

  it('should handle content-safety-review capability', fakeAsync(() => {
    const request = buildRequest({ capability: 'content-safety-review' });
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.status).toBe('fulfilled');
    expect(response!.payload).toBeDefined();
    const payload = response!.payload as { type: string; contentId: string; approved: boolean };
    expect(payload.type).toBe('content-review');
    expect(payload.contentId).toBe('content-123');
    expect(typeof payload.approved).toBe('boolean');
  }));

  it('should handle attestation-recommendation capability', fakeAsync(() => {
    const request = buildRequest({
      capability: 'attestation-recommendation',
      params: {
        type: 'attestation-recommendation',
        contentId: 'content-456',
        requestedAttestationType: 'fact-check',
      },
    });
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.status).toBe('fulfilled');
    expect(response!.payload).toBeDefined();
    const payload = response!.payload as { type: string; recommend: string };
    expect(payload.type).toBe('attestation-recommendation');
    expect(['grant', 'deny', 'defer']).toContain(payload.recommend);
  }));

  it('should generate default reasoning for unknown capabilities', fakeAsync(() => {
    const request = buildRequest({ capability: 'spiral-detection' });
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(3000);

    expect(response!.status).toBe('fulfilled');
    expect(response!.constitutionalReasoning.interpretation).toContain('spiral-detection');
  }));

  it('should handle path-recommendation capability', fakeAsync(() => {
    const request = buildRequest({
      capability: 'path-recommendation',
      params: {
        type: 'path-analysis',
        pathId: 'know-thyself',
        analysisType: 'learning-objectives',
      },
    });
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.status).toBe('fulfilled');
    expect(response!.payload).toBeDefined();
    const payload = response!.payload as { type: string; pathId: string; overallAssessment: string };
    expect(payload.type).toBe('path-analysis');
    expect(payload.pathId).toBe('know-thyself');
    expect(payload.overallAssessment).toBeTruthy();
  }));

  it('should use longer processing time for knowledge-map-synthesis', fakeAsync(() => {
    const request = buildRequest({ capability: 'knowledge-map-synthesis' });
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });

    tick(2500);
    expect(response).toBeUndefined();

    tick(1000);
    expect(response).toBeDefined();
  }));

  it('should include respondedAt timestamp', fakeAsync(() => {
    const request = buildRequest();
    let response: ElohimResponse | undefined;

    backend.invoke(request, mockElohim).subscribe(r => {
      response = r;
    });
    tick(2000);

    expect(response!.respondedAt).toBeTruthy();
    expect(() => new Date(response!.respondedAt)).not.toThrow();
  }));
});

describe('ElohimBackendCatalog', () => {
  let catalog: ElohimBackendCatalog;

  const createMockBackend = (
    overrides: Partial<ElohimBackend> = {},
  ): any => {
    const spy = { isAvailable: vi.fn(), invoke: vi.fn(), id: overrides.id ?? 'test-backend',
      name: overrides.name ?? 'Test Backend',
      type: overrides.type ?? 'mock', };
    spy.isAvailable.mockReturnValue(Promise.resolve(overrides.isAvailable?.() ?? true));
    return spy;
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ElohimBackendCatalog],
    });
    catalog = TestBed.inject(ElohimBackendCatalog);
  });

  describe('register/unregister', () => {
    it('should register a backend', () => {
      const backend = createMockBackend({ id: 'test' } as unknown as Partial<ElohimBackend>);
      catalog.register(backend);
      expect(catalog.get('test')).toBe(backend);
    });

    it('should unregister a backend', () => {
      const backend = createMockBackend({ id: 'test' } as unknown as Partial<ElohimBackend>);
      catalog.register(backend);
      catalog.unregister('test');
      expect(catalog.get('test')).toBeUndefined();
    });

    it('should clear preferred when unregistering preferred backend', () => {
      const backend = createMockBackend({ id: 'test' } as unknown as Partial<ElohimBackend>);
      catalog.register(backend);
      catalog.setPreferred('test');
      catalog.unregister('test');
      expect(catalog.getPreferredId()).toBeNull();
    });

    it('should not clear preferred when unregistering different backend', () => {
      const b1 = createMockBackend({ id: 'b1' } as unknown as Partial<ElohimBackend>);
      const b2 = createMockBackend({ id: 'b2' } as unknown as Partial<ElohimBackend>);
      catalog.register(b1);
      catalog.register(b2);
      catalog.setPreferred('b1');
      catalog.unregister('b2');
      expect(catalog.getPreferredId()).toBe('b1');
    });
  });

  describe('setPreferred/getPreferredId', () => {
    it('should set and get preferred id', () => {
      catalog.setPreferred('native');
      expect(catalog.getPreferredId()).toBe('native');
    });

    it('should default to null', () => {
      expect(catalog.getPreferredId()).toBeNull();
    });
  });

  describe('get/getAll', () => {
    it('should return undefined for unregistered backend', () => {
      expect(catalog.get('nonexistent')).toBeUndefined();
    });

    it('should return all registered backends', () => {
      const b1 = createMockBackend({ id: 'b1' } as unknown as Partial<ElohimBackend>);
      const b2 = createMockBackend({ id: 'b2' } as unknown as Partial<ElohimBackend>);
      catalog.register(b1);
      catalog.register(b2);
      expect(catalog.getAll().length).toBe(2);
    });
  });

  describe('selectBackend', () => {
    it('should return preferred backend when available', async () => {
      const mock = createMockBackend({ id: 'mock' } as unknown as Partial<ElohimBackend>);
      const native = createMockBackend({
        id: 'native',
        type: 'native',
      } as unknown as Partial<ElohimBackend>);
      native.isAvailable.mockReturnValue(Promise.resolve(true));

      catalog.register(mock);
      catalog.register(native);
      catalog.setPreferred('native');

      const selected = await catalog.selectBackend();
      expect(selected.id).toBe('native');
    });

    it('should fall back to non-mock when preferred unavailable', async () => {
      const mock = createMockBackend({ id: 'mock' } as unknown as Partial<ElohimBackend>);
      const preferred = createMockBackend({
        id: 'native',
        type: 'native',
      } as unknown as Partial<ElohimBackend>);
      preferred.isAvailable.mockReturnValue(Promise.resolve(false));

      const fallback = createMockBackend({
        id: 'native-2',
        type: 'native',
      } as unknown as Partial<ElohimBackend>);
      fallback.isAvailable.mockReturnValue(Promise.resolve(true));

      catalog.register(mock);
      catalog.register(preferred);
      catalog.register(fallback);
      catalog.setPreferred('native');

      const selected = await catalog.selectBackend();
      expect(selected.id).toBe('native-2');
    });

    it('should fall back to mock when no non-mock available', async () => {
      const mock = createMockBackend({ id: 'mock' } as unknown as Partial<ElohimBackend>);
      const native = createMockBackend({
        id: 'native',
        type: 'native',
      } as unknown as Partial<ElohimBackend>);
      native.isAvailable.mockReturnValue(Promise.resolve(false));

      catalog.register(mock);
      catalog.register(native);
      catalog.setPreferred('native');

      const selected = await catalog.selectBackend();
      expect(selected.id).toBe('mock');
    });

    it('should create inline MockBackend as last resort', async () => {
      const selected = await catalog.selectBackend();
      expect(selected).toBeDefined();
      expect(selected.type).toBe('mock');
    });
  });
});
