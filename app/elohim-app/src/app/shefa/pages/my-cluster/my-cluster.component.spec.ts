import { provideHttpClient } from '@angular/common/http';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { MyClusterComponent } from './my-cluster.component';

const flushAsync = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe('MyClusterComponent', () => {
  let fixture: ComponentFixture<MyClusterComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [MyClusterComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(MyClusterComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
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
