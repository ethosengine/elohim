import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../../environments/environment';

import { MyClusterComponent } from './my-cluster.component';

const flushAsync = () => new Promise<void>(resolve => setTimeout(resolve, 0));

describe('MyClusterComponent', () => {
  let fixture: ComponentFixture<MyClusterComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    // Pin REST transport — service-level specs cover GraphQL separately.
    environment.features = { useGraphqlTopology: false };
    await TestBed.configureTestingModule({
      imports: [MyClusterComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: AgentService, useValue: { getCurrentAgentId: () => 'agent_M' } },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(MyClusterComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
    TestBed.resetTestingModule();
  });

  it('renders device tiles for each cluster device on load', async () => {
    fixture.detectChanges();
    const req = http.expectOne('/api/v1/cluster');
    req.flush({
      agentCid: 'agent_M',
      devices: [
        {
          peerId: 'P1',
          archetype: 'desktop',
          online: true,
          freshness: { state: 'live' },
          displayName: 'matthew',
          hostingCount: 1247,
        },
        {
          peerId: 'P2',
          archetype: 'node',
          online: true,
          freshness: { state: 'live' },
          displayName: 'home',
          hostingCount: 800,
        },
      ],
      totals: {
        storageUsedBytes: 25_000_000_000,
        storageTotalBytes: 298_000_000_000,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const tiles = fixture.nativeElement.querySelectorAll('[data-testid="device-tile"]');
    expect(tiles.length).toBe(2);
  });

  it('renders a clickable capacity bar with used/free segments sized from totals', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/cluster').flush({
      agentCid: 'agent_M',
      devices: [{ peerId: 'P1', archetype: 'node', online: true, freshness: { state: 'live' } }],
      totals: {
        // 25 GB used of 100 GB total => 25% used / 75% free.
        storageUsedBytes: 25_000_000_000,
        storageTotalBytes: 100_000_000_000,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const bar = fixture.nativeElement.querySelector('[data-testid="my-cluster-capacity-bar"]');
    expect(bar).toBeTruthy();
    expect(bar.getAttribute('role')).toBe('button');
    expect(bar.getAttribute('tabindex')).toBe('0');
    const used = bar.querySelector('.capacity-bar__segment--used') as HTMLElement;
    const free = bar.querySelector('.capacity-bar__segment--free') as HTMLElement;
    expect(used.style.width).toBe('25%');
    expect(free.style.width).toBe('75%');
  });

  it('guards against a zero total — used segment is 0% (no NaN)', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/cluster').flush({
      agentCid: 'agent_M',
      devices: [{ peerId: 'P1', archetype: 'node', online: true, freshness: { state: 'live' } }],
      totals: {
        storageUsedBytes: 0,
        storageTotalBytes: 0,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const bar = fixture.nativeElement.querySelector('[data-testid="my-cluster-capacity-bar"]');
    const used = bar.querySelector('.capacity-bar__segment--used') as HTMLElement;
    const free = bar.querySelector('.capacity-bar__segment--free') as HTMLElement;
    expect(used.style.width).toBe('0%');
    expect(free.style.width).toBe('100%');
  });

  it('toggles the totals detail panel when the capacity bar is clicked', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/cluster').flush({
      agentCid: 'agent_M',
      devices: [{ peerId: 'P1', archetype: 'node', online: true, freshness: { state: 'live' } }],
      totals: {
        storageUsedBytes: 1,
        storageTotalBytes: 2,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const bar = fixture.nativeElement.querySelector('[data-testid="my-cluster-capacity-bar"]');
    // Collapsed by default.
    expect(fixture.nativeElement.querySelector('[data-testid="my-cluster-totals"]')).toBeNull();
    expect(bar.getAttribute('aria-expanded')).toBe('false');
    bar.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="my-cluster-totals"]')).toBeTruthy();
    expect(bar.getAttribute('aria-expanded')).toBe('true');
    bar.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="my-cluster-totals"]')).toBeNull();
  });

  it('toggles the totals detail panel on Enter keydown', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/cluster').flush({
      agentCid: 'agent_M',
      devices: [{ peerId: 'P1', archetype: 'node', online: true, freshness: { state: 'live' } }],
      totals: {
        storageUsedBytes: 1,
        storageTotalBytes: 2,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const bar = fixture.nativeElement.querySelector('[data-testid="my-cluster-capacity-bar"]');
    bar.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="my-cluster-totals"]')).toBeTruthy();
  });

  it('marks offline devices with the offline class', async () => {
    fixture.detectChanges();
    const req = http.expectOne('/api/v1/cluster');
    req.flush({
      agentCid: 'agent_M',
      devices: [
        {
          peerId: 'P1',
          archetype: 'mobile',
          online: false,
          freshness: { state: 'offline', staleSinceMs: 240000 },
          displayName: 'phone',
        },
      ],
      totals: {
        storageUsedBytes: 0,
        storageTotalBytes: 0,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    expect(tile?.classList.contains('offline')).toBe(true);
  });
});
