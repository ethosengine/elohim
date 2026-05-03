import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { PeerTopologyComponent } from './peer-topology.component';

const flushAsync = () => new Promise<void>(resolve => setTimeout(resolve, 0));

describe('PeerTopologyComponent', () => {
  let fixture: ComponentFixture<PeerTopologyComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PeerTopologyComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(PeerTopologyComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
  });

  it('renders one card per peer household edge', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/peer-topology').flush({
      agentCid: 'agent_M',
      edges: [
        {
          householdId: 'adam',
          online: true,
          myCidsHostedByThem: 7,
          theirCidsHostedByMe: 12,
          netDiff: 5,
        },
        {
          householdId: 'pete',
          online: true,
          myCidsHostedByThem: 4,
          theirCidsHostedByMe: 8,
          netDiff: 4,
        },
      ],
      reciprocationCount: 2,
      resilienceCliffs: [],
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const cards = fixture.nativeElement.querySelectorAll('[data-testid="peer-household-card"]');
    expect(cards.length).toBe(2);
  });

  it('shows the resilience cliff warning when cliffs are present', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/peer-topology').flush({
      agentCid: 'agent_M',
      edges: [],
      reciprocationCount: 0,
      resilienceCliffs: [{ householdId: 'adam', soleReplicaCidCount: 3 }],
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="peer-topology-cliff"]')).toBeTruthy();
  });
});
