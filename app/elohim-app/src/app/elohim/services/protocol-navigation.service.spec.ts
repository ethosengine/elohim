import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { of } from 'rxjs';

import { EprNavContextService, type EprNavContextView } from './epr-nav-context.service';
import { ProtocolNavigationService } from './protocol-navigation.service';
import { SessionNavStackService } from './session-nav-stack.service';

function makeContext(overrides: Partial<EprNavContextView> = {}): EprNavContextView {
  return {
    cid: 'x',
    prev: { cid: 'epr-prev', label: 'Prev (EPR)' },
    next: { cid: 'epr-next', label: 'Next (EPR)' },
    partOf: [],
    related: [],
    derivedFrom: ['partOf:Path'],
    ...overrides,
  };
}

describe('ProtocolNavigationService', () => {
  let svc: ProtocolNavigationService;
  let session: SessionNavStackService;
  let eprFetch: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    sessionStorage.clear();
    eprFetch = vi.fn(() => of(makeContext()));
    TestBed.configureTestingModule({
      providers: [{ provide: EprNavContextService, useValue: { fetch: eprFetch } }],
    });
    svc = TestBed.inject(ProtocolNavigationService);
    session = TestBed.inject(SessionNavStackService);
  });

  afterEach(() => sessionStorage.clear());

  it('uses EPR prev as back when session is empty (cold hit)', async () => {
    await svc.activate('x', '/resource/x');
    expect(svc.back()?.cid).toBe('epr-prev');
    expect(svc.forward()?.cid).toBe('epr-next');
  });

  it('uses session top as back when session has prior protocol routes', async () => {
    session.record({ url: '/resource/y', cid: 'y', label: 'Y' });
    await svc.activate('x', '/resource/x');
    expect(svc.back()?.cid).toBe('y');
    expect(svc.back()?.label).toBe('Y');
  });

  it('uses EPR next as forward regardless of session state', async () => {
    session.record({ url: '/resource/y', cid: 'y' });
    await svc.activate('x', '/resource/x');
    expect(svc.forward()?.cid).toBe('epr-next');
  });

  it('records the current activation in the session stack', async () => {
    await svc.activate('x', '/resource/x');
    expect(session.entries()).toHaveLength(1);
    expect(session.entries()[0]?.cid).toBe('x');
  });

  it('returns null for back and forward when EPR returns null and session is empty', async () => {
    eprFetch.mockReturnValue(of(null));
    await svc.activate('x', '/resource/x');
    expect(svc.back()).toBeNull();
    expect(svc.forward()).toBeNull();
  });

  it('hides forward when EPR has no next', async () => {
    eprFetch.mockReturnValue(of(makeContext({ next: null })));
    await svc.activate('x', '/resource/x');
    expect(svc.forward()).toBeNull();
  });

  it('exposes the raw context for richer surfaces', async () => {
    await svc.activate('x', '/resource/x');
    expect(svc.context()?.derivedFrom).toContain('partOf:Path');
  });
});
