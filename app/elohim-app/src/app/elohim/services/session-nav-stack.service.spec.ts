import { TestBed } from '@angular/core/testing';

import { SessionNavStackService } from './session-nav-stack.service';

describe('SessionNavStackService', () => {
  let svc: SessionNavStackService;

  beforeEach(() => {
    sessionStorage.clear();
    TestBed.configureTestingModule({});
    svc = TestBed.inject(SessionNavStackService);
  });

  afterEach(() => sessionStorage.clear());

  it('starts empty', () => {
    expect(svc.length()).toBe(0);
    expect(svc.previous()).toBeNull();
  });

  it('records protocol routes in order', () => {
    svc.record({ url: '/', cid: 'home', label: 'Home' });
    svc.record({ url: '/resource/abc', cid: 'abc', label: 'Item' });
    expect(svc.length()).toBe(2);
    expect(svc.previous()?.cid).toBe('home');
  });

  it('does not record consecutive duplicates', () => {
    svc.record({ url: '/resource/abc', cid: 'abc' });
    svc.record({ url: '/resource/abc', cid: 'abc' });
    expect(svc.length()).toBe(1);
  });

  it('persists across instances via sessionStorage', () => {
    svc.record({ url: '/resource/abc', cid: 'abc' });
    // Bypass DI to get a genuinely fresh instance reading from sessionStorage
    const fresh = new SessionNavStackService();
    expect(fresh.length()).toBe(1);
    expect(fresh.previous()?.cid).toBe('abc');
  });

  it('pops the top entry', () => {
    svc.record({ url: '/resource/abc', cid: 'abc' });
    svc.record({ url: '/resource/def', cid: 'def' });
    const popped = svc.pop();
    expect(popped?.cid).toBe('def');
    expect(svc.length()).toBe(1);
  });

  it('exposes the full stack via entries()', () => {
    svc.record({ url: '/', cid: 'home' });
    svc.record({ url: '/resource/abc', cid: 'abc' });
    expect(svc.entries()).toHaveLength(2);
  });
});
