import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../../environments/environment';

import { ReciprocityLedgerComponent } from './reciprocity-ledger.component';

const flushAsync = () => new Promise<void>(resolve => setTimeout(resolve, 0));

describe('ReciprocityLedgerComponent', () => {
  let fixture: ComponentFixture<ReciprocityLedgerComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    // Pin REST transport for component rendering tests — service-level specs
    // (reciprocity.service.spec.ts) cover GraphQL transport separately.
    environment.features = { useGraphqlTopology: false };
    await TestBed.configureTestingModule({
      imports: [ReciprocityLedgerComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: AgentService, useValue: { getCurrentAgentId: () => 'agent_M' } },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(ReciprocityLedgerComponent);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    fixture.destroy();
    http.verify();
    TestBed.resetTestingModule();
  });

  it('renders inflow and outflow rows', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/reciprocity').flush({
      agentCid: 'agent_M',
      inflow: [
        {
          counterpartyHouseholdId: 'adam',
          displayName: 'Adam',
          committedBytes: 5_000_000_000,
          deliveredBytes: 4_500_000_000,
          honoredFraction: 0.9,
        },
      ],
      outflow: [
        {
          counterpartyHouseholdId: 'pete',
          displayName: 'Pete',
          committedBytes: 3_000_000_000,
          deliveredBytes: 3_100_000_000,
          honoredFraction: 1.03,
        },
      ],
      netHostedBytes: 1_400_000_000,
      capacityAvailableBytes: 50_000_000_000,
    });
    await flushAsync();
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelectorAll('[data-testid="reciprocity-inflow-row"]').length
    ).toBe(1);
    expect(
      fixture.nativeElement.querySelectorAll('[data-testid="reciprocity-outflow-row"]').length
    ).toBe(1);
  });

  it('renders capacity available', async () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/reciprocity').flush({
      agentCid: 'agent_M',
      inflow: [],
      outflow: [],
      netHostedBytes: 0,
      capacityAvailableBytes: 100_000_000_000,
    });
    await flushAsync();
    fixture.detectChanges();
    const cap = fixture.nativeElement.querySelector('[data-testid="reciprocity-capacity"]');
    expect(cap?.textContent).toContain('GB');
  });
});
