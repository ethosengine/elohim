import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Routes } from '@angular/router';
import { Router } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { ProtocolRouteContextService } from './protocol-route-context.service';

@Component({ standalone: true, template: '' })
class StubComponent {}

const routes: Routes = [
  {
    path: '',
    component: StubComponent,
    data: { protocolContent: true, fallbackCid: 'landing' },
  },
  {
    path: 'resource/:resourceId',
    component: StubComponent,
    data: { protocolContent: true },
  },
  { path: 'admin', component: StubComponent, data: { protocolContent: false } },
  { path: 'settings', component: StubComponent },
];

describe('ProtocolRouteContextService', () => {
  let svc: ProtocolRouteContextService;
  let router: Router;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [provideRouter(routes)],
    });
    router = TestBed.inject(Router);
    svc = TestBed.inject(ProtocolRouteContextService);
  });

  it('reads fallbackCid for the root protocol route', async () => {
    await router.navigateByUrl('/');
    expect(svc.isProtocol()).toBe(true);
    expect(svc.cid()).toBe('landing');
  });

  it('reads resourceId for parameterized protocol routes', async () => {
    await router.navigateByUrl('/resource/abc');
    expect(svc.isProtocol()).toBe(true);
    expect(svc.cid()).toBe('abc');
  });

  it('reports non-protocol routes as such (explicit false)', async () => {
    await router.navigateByUrl('/admin');
    expect(svc.isProtocol()).toBe(false);
  });

  it('reports non-protocol routes as such (absent flag)', async () => {
    await router.navigateByUrl('/settings');
    expect(svc.isProtocol()).toBe(false);
    expect(svc.cid()).toBeNull();
  });

  it('cid() is null when no resourceId param and no fallbackCid', async () => {
    await router.navigateByUrl('/admin');
    expect(svc.cid()).toBeNull();
  });

  it('updates signals reactively on subsequent navigations', async () => {
    await router.navigateByUrl('/resource/first');
    expect(svc.isProtocol()).toBe(true);
    expect(svc.cid()).toBe('first');

    await router.navigateByUrl('/admin');
    expect(svc.isProtocol()).toBe(false);
    expect(svc.cid()).toBeNull();
  });
});
