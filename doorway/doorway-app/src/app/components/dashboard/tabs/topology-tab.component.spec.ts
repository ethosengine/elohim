import { provideHttpClient } from '@angular/common/http';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { TopologyTabComponent } from './topology-tab.component';

const flushAsync = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe('TopologyTabComponent', () => {
  let fixture: ComponentFixture<TopologyTabComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TopologyTabComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(TopologyTabComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
  });

  it('renders the dashboard view from /admin/dashboard/topology', async () => {
    fixture.detectChanges();
    http.expectOne('/admin/dashboard/topology').flush({
      doorwayHostname: 'matthew.elohim.host',
      storageStewards: [],
      federationPeers: [],
      projectionCoverage: {
        projectedCidCount: 4318,
        knownCidCount: 5672,
        cacheHitRate24h: 0.87,
        projectionLagMsAvg: 340,
      },
      publicSurface: {
        dnsResolves: true,
        tlsValid: true,
        publicReachable: true,
      },
    });
    await flushAsync();
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="topology-hostname"]')?.textContent,
    ).toContain('matthew.elohim.host');
    expect(
      fixture.nativeElement.querySelector('[data-testid="projection-coverage"]')?.textContent,
    ).toMatch(/87/);
  });

  it('shows reachability state from public surface', async () => {
    fixture.detectChanges();
    http.expectOne('/admin/dashboard/topology').flush({
      doorwayHostname: 'pete.elohim.host',
      storageStewards: [],
      federationPeers: [],
      projectionCoverage: {
        projectedCidCount: 0,
        knownCidCount: 0,
        cacheHitRate24h: 0,
        projectionLagMsAvg: 0,
      },
      publicSurface: { dnsResolves: false, tlsValid: false, publicReachable: false },
    });
    await flushAsync();
    fixture.detectChanges();
    const reachable = fixture.nativeElement.querySelector('[data-testid="surface-reachable"]');
    expect(reachable?.textContent).toMatch(/unreachable/);
  });
});
