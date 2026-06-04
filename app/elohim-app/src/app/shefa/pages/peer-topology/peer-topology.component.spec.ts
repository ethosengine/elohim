import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../../environments/environment';

import { PeerTopologyComponent } from './peer-topology.component';

const flushAsync = () => new Promise<void>(resolve => setTimeout(resolve, 0));

describe('PeerTopologyComponent', () => {
  let fixture: ComponentFixture<PeerTopologyComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    // Pin REST transport — service-level specs cover GraphQL separately.
    environment.features = { useGraphqlTopology: false };
    await TestBed.configureTestingModule({
      imports: [PeerTopologyComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: AgentService, useValue: { getCurrentAgentId: () => 'agent_M' } },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(PeerTopologyComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
    TestBed.resetTestingModule();
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

  it('toggles a per-household detail panel when its row is activated', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/peer-topology').flush({
      agentCid: 'agent_M',
      edges: [
        {
          householdId: 'adam',
          displayName: 'Adam',
          online: true,
          lastSyncSec: 42,
          myCidsHostedByThem: 7,
          theirCidsHostedByMe: 12,
          netDiff: 5,
          isCriticalForMe: true,
        },
        {
          householdId: 'pete',
          online: true,
          lastSyncSec: 9,
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
    const rows = fixture.nativeElement.querySelectorAll('[data-testid="peer-household-expand"]');
    expect(rows.length).toBe(2);
    expect(rows[0].getAttribute('role')).toBe('button');
    expect(rows[0].getAttribute('tabindex')).toBe('0');
    expect(rows[0].getAttribute('aria-expanded')).toBe('false');
    // No panel until activated.
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-household-detail-panel"]')
    ).toBeNull();
    rows[0].click();
    fixture.detectChanges();
    expect(rows[0].getAttribute('aria-expanded')).toBe('true');
    const panel = fixture.nativeElement.querySelector(
      '[data-testid="peer-household-detail-panel"]'
    );
    expect(panel).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-detail-last-sync"]')?.textContent
    ).toContain('42');
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-detail-critical-for-me"]')
        ?.textContent
    ).toContain('yes');
    // Independent rows: the second row stays collapsed.
    expect(rows[1].getAttribute('aria-expanded')).toBe('false');
    rows[0].click();
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-household-detail-panel"]')
    ).toBeNull();
  });

  it('expands a household detail panel on Enter keydown', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/peer-topology').flush({
      agentCid: 'agent_M',
      edges: [
        {
          householdId: 'adam',
          online: true,
          myCidsHostedByThem: 7,
          theirCidsHostedByMe: 12,
        },
      ],
      reciprocationCount: 1,
      resilienceCliffs: [],
      freshness: { state: 'live' },
    });
    await flushAsync();
    fixture.detectChanges();
    const row = fixture.nativeElement.querySelector('[data-testid="peer-household-expand"]');
    row.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-household-detail-panel"]')
    ).toBeTruthy();
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
